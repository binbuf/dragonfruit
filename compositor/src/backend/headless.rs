// SPDX-License-Identifier: MIT OR Apache-2.0
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
    run_session(
        socket_name,
        BackendHooks {
            init: Box::new(|state| {
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
                } else {
                    state.stats.frames_skipped_no_damage += 1;
                }
                state.needs_redraw = false;
                Ok(())
            }),
        },
    )
}
