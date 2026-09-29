// SPDX-License-Identifier: MIT
//! Shared compositor integration-test support.

use std::path::{Path, PathBuf};

/// Every hand-off file a compositor session leaves in `$XDG_RUNTIME_DIR`: the
/// socket, its lock, both launch tokens, and the Xwayland `DISPLAY` file.
///
/// The list mirrors the compositor's clean teardown
/// (`dragonfruit_compositor::session`) and the dev-tool
/// `soak::teardown_artifacts`, so a test can assert a clean exit leaked none
/// of them (T-16.4 suspend/resume soak, T-17.5b leak enforcement).
pub fn compositor_artifacts(socket: &Path) -> Vec<PathBuf> {
    vec![
        socket.to_path_buf(),
        socket.with_extension("lock"),
        socket.with_extension("launch-token"),
        socket.with_extension("desktop-launch-token"),
        socket.with_extension("x11-display"),
    ]
}

/// Remove every hand-off file a compositor session leaves in
/// `$XDG_RUNTIME_DIR`.
///
/// The test harnesses hard-kill the compositor (`SIGKILL`) so it cannot run
/// its own teardown, so without this every test run would accumulate stale
/// sockets/tokens/`DISPLAY` files in the shared runtime directory (T-17.5b
/// leak enforcement).
pub fn cleanup_compositor_artifacts(socket: &Path) {
    for artifact in compositor_artifacts(socket) {
        let _ = std::fs::remove_file(artifact);
    }
}
