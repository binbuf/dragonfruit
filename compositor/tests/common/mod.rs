// SPDX-License-Identifier: MIT
//! Shared compositor integration-test support.

use std::path::Path;

/// Remove every hand-off file a compositor session leaves in
/// `$XDG_RUNTIME_DIR`: the socket, its lock, both launch tokens, and the
/// Xwayland `DISPLAY` file.
///
/// The test harnesses hard-kill the compositor (`SIGKILL`) so it cannot run
/// its own teardown, so without this every test run would accumulate stale
/// sockets/tokens/`DISPLAY` files in the shared runtime directory (T-17.5b
/// leak enforcement). The compositor's own clean teardown path is
/// `dragonfruit_compositor::session` and the dev-tool
/// `soak::teardown_artifacts`, which this list mirrors.
pub fn cleanup_compositor_artifacts(socket: &Path) {
    let _ = std::fs::remove_file(socket);
    let _ = std::fs::remove_file(socket.with_extension("lock"));
    let _ = std::fs::remove_file(socket.with_extension("launch-token"));
    let _ = std::fs::remove_file(socket.with_extension("desktop-launch-token"));
    let _ = std::fs::remove_file(socket.with_extension("x11-display"));
}
