// SPDX-License-Identifier: MIT
//! The shared session runner (T-02).
//!
//! One calloop event loop multiplexes Wayland clients, backend input
//! devices, D-Bus (added by later tickets), signals, and timers. Rendering
//! is driven per output: nested mode renders when `needs_redraw` is set
//! (damage-driven, FR-2); DRM mode is timer/vblank-driven per output.
//!
//! Every backend runs this runner — backend selection is a flag, not a
//! fork (FR-1).

use std::sync::Arc;

use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::{Interest, Mode, PostAction};
use smithay::reexports::wayland_server::{Display, DisplayHandle};
use smithay::wayland::socket::ListeningSocketSource;

use crate::state::{DfClientState, DfState};

/// A backend hook: sets up outputs, renderer, and input devices before the
/// loop starts, and optionally drives per-iteration rendering.
type InitHook = Box<dyn FnOnce(&mut DfState) -> Result<(), String>>;
type RenderHook = Box<dyn FnMut(&mut DfState) -> Result<(), String>>;

pub struct BackendHooks {
    /// Runs once before the loop. Create outputs/renderers here.
    pub init: InitHook,
    /// Runs every loop iteration after dispatch; drive damage-driven
    /// rendering for outputs here (nested mode).
    pub render: RenderHook,
}

/// Run the compositor session on the given socket until SIGINT/SIGTERM or
/// the backend decides to quit. Teardown is verified before returning.
pub fn run_session(socket_name: &str, hooks: BackendHooks) -> Result<(), String> {
    let mut event_loop: calloop::EventLoop<'static, DfState> =
        calloop::EventLoop::try_new().map_err(|e| format!("failed to init event loop: {e}"))?;
    let loop_handle = event_loop.handle();

    let display: Display<DfState> =
        Display::new().map_err(|e| format!("failed to create Wayland display: {e}"))?;
    let dh: DisplayHandle = display.handle();

    let loop_signal = event_loop.get_signal();

    let mut state = DfState::new(&dh, loop_handle.clone(), loop_signal);
    state.socket_name = socket_name.to_string();

    // Provision the private-protocol launch token (T-07 trust model). The
    // session manager (T-24) hands it to the shell out-of-band; the dev
    // tool and the conformance tests read the 0600 hand-off file.
    crate::shell::provision(&mut state);

    // Wayland socket: fd-driven accept + guaranteed removal on drop (RAII,
    // per the T-01 teardown lesson — no reliance on a drop chain at the
    // end of a fallible function).
    let socket_source = ListeningSocketSource::with_name(socket_name)
        .map_err(|e| format!("failed to bind socket {socket_name:?}: {e}"))?;
    let printed_name = socket_source.socket_name().to_string_lossy().into_owned();
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| "XDG_RUNTIME_DIR is not set; refusing to create a socket".to_string())?;
    let socket_path = std::path::PathBuf::from(&runtime_dir).join(&printed_name);
    println!(
        "dragonfruit-compositor: wayland socket: {}",
        socket_path.display()
    );
    println!("dragonfruit-compositor: WAYLAND_DISPLAY={printed_name}");
    loop_handle
        .insert_source(socket_source, move |client_stream, _, _state| {
            let mut dh = dh.clone();
            if let Err(err) = dh.insert_client(client_stream, Arc::new(DfClientState::default())) {
                eprintln!("dragonfruit-compositor: rejecting client: {err}");
            }
        })
        .map_err(|e| format!("failed to register socket source: {e}"))?;

    // Client dispatch: level-triggered on the display's own fd.
    let _display_token = loop_handle
        .insert_source(
            Generic::new(display, Interest::READ, Mode::Level),
            |_, display, state| {
                // Safety: the display lives in the source, which outlives
                // the callback (calloop Generic contract, same pattern as
                // upstream anvil).
                let display: &mut Display<DfState> = unsafe { display.get_mut() };
                let _ = display.dispatch_clients(state);
                Ok(PostAction::Continue)
            },
        )
        .map_err(|e| format!("failed to register display source: {e}"))?;

    // Quit on SIGINT/SIGTERM — clean teardown is a phase exit criterion.
    // SIGUSR1 dumps the render-path counters without interrupting the
    // session (FR-2/FR-5 observability).
    let signals = calloop::signals::Signals::new(&[
        calloop::signals::Signal::SIGTERM,
        calloop::signals::Signal::SIGINT,
        calloop::signals::Signal::SIGUSR1,
    ])
    .map_err(|e| format!("failed to register signal handlers: {e}"))?;
    loop_handle
        .insert_source(signals, |event, _, state| match event.signal() {
            calloop::signals::Signal::SIGUSR1 => state.dump_stats("SIGUSR1"),
            _ => {
                println!("dragonfruit-compositor: shutting down (signal)");
                state.running = false;
            }
        })
        .map_err(|e| format!("failed to register signal source: {e}"))?;

    // Backend-specific setup (outputs, renderer, input devices).
    (hooks.init)(&mut state)?;

    // Xwayland (T-06): spawned eagerly now that an output and seat exist.
    // A missing binary is non-fatal (Wayland-only session).
    if let Err(err) = crate::xwayland::start(&mut state) {
        eprintln!("dragonfruit-compositor: Xwayland setup failed: {err}");
    }

    // Main loop: fully event-driven; backends drive their own frame
    // scheduling (timers/vblank for DRM, redraw flag for nested).
    let mut render = hooks.render;
    while state.running {
        // Fully fd-driven: winit (nested), udev/libinput/DRM event fds,
        // client sockets, and signals all wake the loop; nothing polls.
        event_loop
            .dispatch(None, &mut state)
            .map_err(|e| format!("event loop error: {e}"))?;

        // A dead Xwayland is respawned outside its own event-source borrow
        // (FR-6: an Xwayland crash never destabilizes the session).
        crate::xwayland::maybe_restart(&mut state);

        state.space.refresh();
        state.popups.cleanup();

        (render)(&mut state)?;

        // Drain the T-03/T-04/T-05 outboxes into the private shell protocol
        // (T-07): window/workspace/focus/attention and input events are
        // delivered in scene-consistent order before the flush below.
        state.broadcast_shell_events();

        if let Err(err) = state.display_handle.flush_clients() {
            eprintln!("dragonfruit-compositor: flush failed: {err}");
        }
    }

    // Teardown: the event loop (holding the display source) drops first,
    // then the state. The listening socket was dropped with its source;
    // verify nothing leaked (phase exit criterion, kept from T-01).
    //
    // Note: `loop_handle` is the last surviving calloop handle and keeps
    // the registered sources (and with them the socket) alive, so it must
    // be dropped before the leak check.
    state.dump_stats("exit");
    // The Xwayland DISPLAY hand-off file and the X11 socket/lock are session
    // state, not leaks, but they must not survive the session (T-06).
    crate::xwayland::remove_display_file(&state.socket_name);
    if let Some(number) = state.xwayland.display_number {
        crate::xwayland::cleanup_x11_files(number);
    }
    crate::shell::remove_token_file(&state.socket_name);
    drop(event_loop);
    drop(state);
    drop(loop_handle);

    // Smithay 0.7's `X11Wm` event source captures a strong `LoopHandle` in
    // its callback, which forms an `Rc` cycle with calloop's `LoopInner`.
    // The cycle keeps the `ListeningSocketSource` alive past teardown, so
    // its RAII unlink cannot run once Xwayland has been started. Unlink
    // explicitly; the check below still verifies the unlink worked (a
    // permission or unexpected-file failure still fails the session). This
    // is a T-31 hardening/upstream follow-up (see PROGRESS.md).
    let _ = std::fs::remove_file(&socket_path);
    let _ = std::fs::remove_file(socket_path.with_extension("lock"));

    if socket_path.exists() {
        return Err(format!(
            "teardown leak: socket {} survived exit",
            socket_path.display()
        ));
    }
    Ok(())
}
