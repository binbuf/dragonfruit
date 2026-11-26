// SPDX-License-Identifier: GPL-3.0-only
//! Shared compositor integration-test support.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The compositor binary ready to spawn, with the developer tree's user-space
/// DRM sysroot on its loader path.
///
/// The compositor links `libseat`/`libgbm` from `~/.local/df-toolchain` on a
/// machine without the distro `-devel` packages, so a bare `cargo test` would
/// otherwise see the child exit `127` before it can create its socket. The
/// Makefile exports the same directory; merge with it instead of replacing it,
/// because a caller may already have added other loader directories. On a
/// normal host the directory is absent and the default loader path applies.
pub fn compositor_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dragonfruit-compositor"));
    if let Some(home) = std::env::var_os("HOME") {
        let lib = PathBuf::from(&home).join(".local/df-toolchain/usr/lib64");
        if lib.is_dir() {
            let mut dirs = vec![lib];
            if let Some(existing) = std::env::var_os("LD_LIBRARY_PATH") {
                dirs.extend(std::env::split_paths(&existing));
            }
            if let Ok(joined) = std::env::join_paths(dirs) {
                command.env("LD_LIBRARY_PATH", joined);
            }
        }
    }
    command
}

/// Every hand-off file a compositor session leaves in `$XDG_RUNTIME_DIR`: the
/// socket, its lock, all three launch tokens, and the Xwayland `DISPLAY` file.
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
        socket.with_extension("portal-launch-token"),
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
