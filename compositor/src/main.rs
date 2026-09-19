// SPDX-License-Identifier: MIT OR Apache-2.0
//! Dragonfruit compositor — skeleton (T-01).
//!
//! This skeleton proves the scaffolding contracts, not the product:
//!
//! * the pinned-Smithay build works from a clean clone,
//! * both backends exist from day one — `nested` (a Wayland window on the
//!   host session, the daily development workflow) and `headless` (no
//!   display, used by CI and the teardown soak test),
//! * the private Wayland socket is created, printed, and removed on exit
//!   so session teardown is verifiably clean,
//! * the lockstep IPC version is embedded and checked against
//!   `protocols/` (see the `df-ipc` crate).
//!
//! Everything users see — windows, workspaces, Spaces, decorations — is
//! built in T-02 and later tickets. Keep this binary a thin policy layer
//! over Smithay.

use std::process::ExitCode;

mod state;

use state::DfState;

use smithay::reexports::{calloop, wayland_server};

use df_ipc::LOCKSTEP_VERSION;

const EXIT_USAGE: u8 = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    /// Run as a window on the host Wayland session (daily development).
    Nested,
    /// Run without any display (CI, soak tests).
    Headless,
}

pub struct Args {
    pub backend: Backend,
    pub socket_name: Option<String>,
}

fn parse_args(iter: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut args = Args {
        backend: Backend::Nested,
        socket_name: None,
    };
    let mut it = iter;
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            "--backend" => match it.next().as_deref() {
                Some("nested") => args.backend = Backend::Nested,
                Some("headless") => args.backend = Backend::Headless,
                other => {
                    return Err(format!(
                        "unknown backend {other:?}, expected `nested` or `headless`"
                    ))
                }
            },
            "--socket-name" => {
                let Some(name) = it.next() else {
                    return Err("--socket-name requires a value".into());
                };
                if name.len() > 100 {
                    return Err("socket name too long for a Unix socket path".into());
                }
                args.socket_name = Some(name);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    Ok(args)
}

fn print_help() {
    println!(
        "dragonfruit-compositor — Dragonfruit Wayland compositor skeleton\n\
         \n\
         USAGE:\n\
         \x20   dragonfruit-compositor [--backend nested|headless] [--socket-name NAME]\n\
         \n\
         The Wayland socket is created in $XDG_RUNTIME_DIR and removed on exit."
    );
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("dragonfruit-compositor: {err}");
            return ExitCode::from(EXIT_USAGE);
        }
    };

    // Lockstep from the first commit: compositor, shell, and protocol XMLs
    // ship as one set; cross-version mixing is rejected (docs/ipc-versioning.md).
    df_ipc::assert_lockstep_compatible(LOCKSTEP_VERSION)
        .expect("the built-in lockstep version must be compatible with itself");

    let socket_name = args
        .socket_name
        .unwrap_or_else(|| format!("dragonfruit-{}", std::process::id()));

    let result = match args.backend {
        Backend::Headless => run_headless(&socket_name),
        Backend::Nested => run_nested(&socket_name),
    };

    match result {
        Ok(()) => {
            println!("dragonfruit-compositor: clean exit");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("dragonfruit-compositor: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The shared compositor loop: bind the private socket, accept clients,
/// dispatch requests until SIGINT/SIGTERM, then verify teardown.
///
/// `pump` runs backend-specific work once per iteration (winit event
/// dispatch in nested mode, nothing in headless mode).
fn compositor_loop<F>(socket_name: &str, state: &mut DfState, mut pump: F) -> Result<(), String>
where
    F: FnMut(&mut DfState, &mut wayland_server::Display<DfState>) -> Result<(), String>,
{
    let mut display: wayland_server::Display<DfState> = wayland_server::Display::new()
        .map_err(|e| format!("failed to create Wayland display: {e}"))?;
    state.display_handle = Some(display.handle());

    // `ListeningSocket` removes the socket file and its lock file on drop,
    // and refuses to shadow a socket another process is still serving.
    let socket = wayland_server::ListeningSocket::bind(socket_name)
        .map_err(|e| format!("failed to bind socket {socket_name:?}: {e}"))?;
    let socket_path = socket_path(socket_name)?;
    println!(
        "dragonfruit-compositor: wayland socket: {}",
        socket_path.display()
    );
    println!("dragonfruit-compositor: WAYLAND_DISPLAY={socket_name}");
    state.socket = Some(socket);

    let mut event_loop: calloop::EventLoop<'static, DfState> =
        calloop::EventLoop::try_new().map_err(|e| format!("failed to init event loop: {e}"))?;

    // Quit on SIGINT/SIGTERM — clean teardown is a phase exit criterion.
    let signals = calloop::signals::Signals::new(&[
        calloop::signals::Signal::SIGTERM,
        calloop::signals::Signal::SIGINT,
    ])
    .map_err(|e| format!("failed to register signal handlers: {e}"))?;
    event_loop
        .handle()
        .insert_source(signals, |_, _, state: &mut DfState| {
            state.running = false;
        })
        .map_err(|e| format!("failed to register signal source: {e}"))?;

    while state.running {
        // A short timeout keeps the loop responsive even when no fd is
        // ready; the real event loop (T-02) will be fd-driven throughout.
        event_loop
            .dispatch(Some(std::time::Duration::from_millis(50)), state)
            .map_err(|e| format!("event loop error: {e}"))?;
        pump(state, &mut display)?;
        state.accept_clients();
        let _ = display.dispatch_clients(state);
        let _ = display.flush_clients();
    }

    // Drop the display, then the event loop, then the listening socket
    // before verifying that nothing leaked into the host session. The
    // Foundation phase exit criterion is a clean teardown: no leaked
    // sockets, no orphaned clients.
    drop(display);
    drop(event_loop);
    state.socket = None;
    if socket_path.exists() || lock_path(&socket_path).exists() {
        let _ = std::fs::remove_file(&socket_path);
        let _ = std::fs::remove_file(lock_path(&socket_path));
        return Err(format!(
            "teardown leak: socket {} survived exit",
            socket_path.display()
        ));
    }
    Ok(())
}

fn socket_path(socket_name: &str) -> Result<std::path::PathBuf, String> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| "XDG_RUNTIME_DIR is not set; refusing to create a socket".to_string())?;
    Ok(std::path::PathBuf::from(runtime_dir).join(socket_name))
}

fn lock_path(socket_path: &std::path::Path) -> std::path::PathBuf {
    socket_path.with_extension("lock")
}

fn run_headless(socket_name: &str) -> Result<(), String> {
    println!(
        "dragonfruit-compositor: starting (backend=headless, lockstep-ipc=v{LOCKSTEP_VERSION})"
    );
    let mut state = DfState::new();
    compositor_loop(socket_name, &mut state, |_state, _display| Ok(()))
}

fn run_nested(socket_name: &str) -> Result<(), String> {
    use smithay::backend::renderer::glow::GlowRenderer;
    use smithay::backend::renderer::{Color32F, Frame, Renderer};
    use smithay::backend::winit::{self, WinitEvent};
    use smithay::reexports::winit::platform::pump_events::PumpStatus;
    use smithay::utils::{Rectangle, Transform};

    println!("dragonfruit-compositor: starting (backend=nested, lockstep-ipc=v{LOCKSTEP_VERSION})");

    let (mut backend, mut winit_loop) = winit::init::<GlowRenderer>().map_err(|e| {
        format!("failed to open a nested window (is there a Wayland session?): {e}")
    })?;
    backend.window().set_title("Dragonfruit");

    let mut state = DfState::new();

    compositor_loop(socket_name, &mut state, |state, _display| {
        let status = winit_loop.dispatch_new_events(|event| match event {
            WinitEvent::CloseRequested => state.running = false,
            WinitEvent::Resized { .. } | WinitEvent::Redraw => {
                // Draw a flat dragonfruit-tinted frame so the window is
                // visibly alive even in the skeleton.
                let size = backend.window_size();
                let damage = Rectangle::from_size(size);
                if let Ok((renderer, mut framebuffer)) = backend.bind() {
                    if let Ok(mut frame) =
                        renderer.render(&mut framebuffer, size, Transform::Normal)
                    {
                        let _ = frame.clear(Color32F::new(0.13, 0.05, 0.16, 1.0), &[damage]);
                        let _ = frame.finish();
                    }
                }
                let _ = backend.submit(None);
            }
            _ => {}
        });
        if matches!(status, PumpStatus::Exit(_)) {
            state.running = false;
        }
        Ok(())
    })
}
