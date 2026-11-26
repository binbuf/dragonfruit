// SPDX-License-Identifier: GPL-3.0-or-later
//! The shared session runner (T-02).
//!
//! One calloop event loop multiplexes Wayland clients, backend input
//! devices, D-Bus (added by later tickets), signals, and timers. Rendering
//! is driven per output: nested mode renders when `needs_redraw` is set
//! (damage-driven, FR-2); DRM mode is timer/vblank-driven per output.
//!
//! Every backend runs this runner — backend selection is a flag, not a
//! fork (FR-1).

use std::sync::atomic::{AtomicU64, Ordering};
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
    // T322: every client shares the disconnect inbox so the compositor can
    // revoke a disconnected client's grab sanction.
    let disconnected_clients = state.disconnected_clients.clone();
    loop_handle
        .insert_source(socket_source, move |client_stream, _, _state| {
            let mut dh = dh.clone();
            let client_state = Arc::new(DfClientState::new(
                smithay::wayland::compositor::CompositorClientState::default(),
                None,
                disconnected_clients.clone(),
            ));
            if let Err(err) = dh.insert_client(client_stream, client_state) {
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
                // T322: `ClientData::disconnected` cannot touch compositor
                // state; drain its inbox here so a gone client's grab
                // sanction is revoked.
                state.forget_disconnected_grab_clients();
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
    // Number SIGUSR1 dumps so the T-21.1a baseline can map each snapshot to a
    // scenario without parsing timestamps out of interleaved logs.
    static SIGUSR1_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    loop_handle
        .insert_source(signals, |event, _, state| match event.signal() {
            calloop::signals::Signal::SIGUSR1 => {
                let sequence = SIGUSR1_SEQUENCE.fetch_add(1, Ordering::Relaxed) + 1;
                state.dump_stats(&format!("SIGUSR1-{sequence}"));
            }
            _ => {
                println!("dragonfruit-compositor: shutting down (signal)");
                state.running = false;
            }
        })
        .map_err(|e| format!("failed to register signal source: {e}"))?;

    // Backend-specific setup (outputs, renderer, input devices).
    (hooks.init)(&mut state)?;

    // T315: arm the idle chain's first deadline. The shipped default policy is
    // active from login; the shell replaces it with the settingsd values via
    // `df_toplevel_manager.set_idle_policy` on startup and on change.
    state.schedule_idle_timer();

    // Login starts from the middle of the primary display. The backend hook
    // has now created the outputs and the seat pointer; the global origin
    // would otherwise put the cursor in a corner (possibly on a secondary
    // output). No-op on a session with no output. The layout may change once
    // the shell applies the persisted mode/scale/transform; the state
    // re-centres again for each such change until the first user pointer
    // input (see `DfState::recenter_pointer_for_initial_layout`).
    state.center_pointer_on_primary_output();

    // Xwayland (T-06): spawned eagerly now that an output and seat exist.
    // A missing binary is non-fatal (Wayland-only session).
    if let Err(err) = crate::xwayland::start(&mut state) {
        eprintln!("dragonfruit-compositor: Xwayland setup failed: {err}");
    }

    // Main loop: fully event-driven; backends drive their own frame
    // scheduling (timers/vblank for DRM, redraw flag for nested).
    let mut render = hooks.render;
    // A single failed frame is not a session-ending event: GPUs lose and
    // regain contexts, swapchains return `TemporaryFailure`, and a window can
    // be resizing while the renderer re-binds. Retry at the frame cadence and
    // only end the session when the failure is clearly persistent (roughly
    // five seconds at 60 Hz), so systemd/display-manager recovery still has a
    // clean exit path instead of a panic or a permanently stalled loop.
    const MAX_CONSECUTIVE_RENDER_FAILURES: u32 = 300;
    let mut render_failures: u32 = 0;
    while state.running {
        // Fully fd-driven: winit (nested), udev/libinput/DRM event fds,
        // client sockets, and signals all wake the loop; nothing polls.
        //
        // One exception: for a short window after input, tick at ~60 Hz while a
        // mapped window has an outstanding frame callback, so that window is
        // not starved by an otherwise idle compositor (see
        // `DfState::frame_cadence_active`). Outside that window the loop is a
        // pure block, so a genuinely idle session still produces no frames.
        // A pending host swap already owns the frame clock: the presenter's
        // ping wakes the loop when it returns, so retrying the cadence at
        // refresh rate would only burn CPU while the host is not presenting.
        let ticking = !state.present_in_flight
            && state.frame_cadence_active()
            && state.has_pending_frame_callbacks();
        if ticking {
            state.needs_redraw = true;
        }
        // Drive the short post-input frame cadence at the output's real
        // refresh, not a hard-coded 60 Hz. On a 144 Hz panel this is what lets
        // a client that is waiting on its frame callback paint at 144 instead
        // of being capped at 60.
        let refresh_interval = state
            .space
            .outputs()
            .next()
            .and_then(|output| output.current_mode())
            .filter(|mode| mode.refresh > 0)
            .map(|mode| dragonfruit_compositor::refresh::mode_period(mode.refresh))
            .unwrap_or(dragonfruit_compositor::refresh::FALLBACK_PERIOD);
        let timeout = (ticking || render_failures > 0).then_some(refresh_interval);
        event_loop
            .dispatch(timeout, &mut state)
            .map_err(|e| format!("event loop error: {e}"))?;

        // A dead Xwayland is respawned outside its own event-source borrow
        // (FR-6: an Xwayland crash never destabilizes the session).
        crate::xwayland::maybe_restart(&mut state);

        state.space.refresh();
        state.popups.cleanup();

        // T-11 U-2 / FR-8: time each rendered frame and record the interval
        // since the last one, so `dump_stats` can prove the gesture budget.
        let frame_start = std::time::Instant::now();
        let frames_before = state.stats.frames_rendered;
        // A suspended session (T-12.5a) produces no frames until it wakes:
        // the clients, the scene, and the outputs are untouched, so recovery
        // is a repaint, not a restart.
        if !state.suspend.is_suspended() {
            match (render)(&mut state) {
                Ok(()) => render_failures = 0,
                Err(err) => {
                    render_failures = render_failures.saturating_add(1);
                    eprintln!(
                        "dragonfruit-compositor: render pass failed \
                         ({render_failures}/{MAX_CONSECUTIVE_RENDER_FAILURES}): {err}"
                    );
                    if render_failures >= MAX_CONSECUTIVE_RENDER_FAILURES {
                        return Err(format!(
                            "renderer failed {render_failures} consecutive frames: {err}"
                        ));
                    }
                    // Keep the frame clock alive so the next dispatch retries
                    // instead of blocking forever on an idle session.
                    state.needs_redraw = true;
                }
            }
        }
        if state.stats.frames_rendered != frames_before {
            if state.stats.frame_observed_by_backend {
                // The nested presenter thread completed the swap and sampled
                // the full frame duration (including the host wait) itself.
                // Recording the loop's CPU slice here would double-count and
                // hide the host-side cost.
                state.stats.frame_observed_by_backend = false;
            } else {
                let frame_duration = frame_start.elapsed();
                // T-03.1a/T-03.1b/T-04.4a/T-05.6: one entry point feeds the
                // frame trace, the latency/degrade instruments, and the
                // gesture-scoped budget trace, so the render loop has a single
                // timing seam.
                state.observe_rendered_frame(frame_duration);
            }
        }

        // T-21.2b: re-arm the animation timer from the earliest per-output
        // deadline after the render pass. A presentation re-arms immediately
        // (it knows the updated clock); this covers a tick whose output was
        // not due or a backend that produced no new frame.
        crate::input::schedule_animation_timer(&mut state);

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
