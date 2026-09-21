// SPDX-License-Identifier: MIT
//! Headless backend (T-02): no display, no renderer, no input devices.
//!
//! Used by CI and automated tests. Runs the exact same session code and
//! protocol surface as nested/DRM — only the output has no render target
//! and `linux-dmabuf` stays unadvertised (a headless run cannot promise a
//! GPU import path).

use smithay::output::Mode;

use crate::backend::{
    add_output, add_seat_capabilities, headless_physical_properties, HEADLESS_MODE_SIZE,
};
use crate::session::{run_session, BackendHooks};

pub fn run(socket_name: &str) -> Result<(), String> {
    println!(
        "dragonfruit-compositor: starting (backend=headless, lockstep-ipc=v{})",
        crate::LOCKSTEP_VERSION
    );
    // T-03 synthetic-input harness: opt-in, headless-only test plumbing.
    // The socket path is a full path so tests control naming/cleanup.
    let synthetic_path = std::env::var_os(crate::input::synthetic::ENV_SYNTHETIC_INPUT)
        .map(std::path::PathBuf::from);
    let install_path = synthetic_path.clone();
    // T-09 synthetic-output harness: the headless backend has one static
    // output, so hotplug (FR-1) needs the same opt-in test channel.
    let synthetic_output_path =
        std::env::var_os(crate::backend::synthetic_output::ENV_SYNTHETIC_OUTPUT)
            .map(std::path::PathBuf::from);
    let install_output_path = synthetic_output_path.clone();
    let result = run_session(
        socket_name,
        BackendHooks {
            init: Box::new(move |state| {
                add_output(
                    state,
                    "HEADLESS-1",
                    headless_physical_properties(),
                    Mode {
                        size: HEADLESS_MODE_SIZE.into(),
                        refresh: 60_000,
                    },
                    (0, 0),
                    1.0,
                );
                add_seat_capabilities(state);
                if let Some(path) = &install_path {
                    crate::input::synthetic::install(state, path)?;
                }
                if let Some(path) = &install_output_path {
                    crate::backend::synthetic_output::install(state, path)?;
                }
                Ok(())
            }),
            render: Box::new(|state| {
                // No render target, but count the redraw request honestly:
                // `frames_rendered` only advances when something actually
                // asked for a frame, so the FR-2 idle trace (steady state
                // with no clients must produce zero damage) can be asserted
                // on this backend without a display.
                if state.needs_redraw {
                    state.stats.frames_rendered += 1;
                    // T-11 U-3: drive the same frame path a real backend does,
                    // so a client's `wl_surface.frame` callbacks are delivered
                    // in CI (FR-2: a playing video keeps playing). No pixels
                    // are rendered or presented.
                    if let Some(output) = state.space.outputs().next().cloned() {
                        let now = state.clock.now();
                        crate::render::post_repaint_headless(state, &output, now.into());
                    }
                } else {
                    state.stats.frames_skipped_no_damage += 1;
                }
                state.needs_redraw = false;
                Ok(())
            }),
        },
    );
    // The socket node is session state, not a leak; remove it even when
    // `run_session` returns an error.
    if let Some(path) = &synthetic_path {
        let _ = std::fs::remove_file(path);
    }
    if let Some(path) = &synthetic_output_path {
        let _ = std::fs::remove_file(path);
    }
    result
}
