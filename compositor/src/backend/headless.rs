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
                // No render target: mark the redraw request as served so
                // damage accounting stays honest in tests.
                state.needs_redraw = false;
                Ok(())
            }),
        },
    )
}
