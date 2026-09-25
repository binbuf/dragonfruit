// SPDX-License-Identifier: MIT
//! The display-manager session entry (T-12.2).
//!
//! A Wayland display manager lists sessions by scanning
//! `share/wayland-sessions/*.desktop`. Dragonfruit's entry is
//! [`SESSION_FILE`]; its `Exec` is the [`ENTRY_PROGRAM`] shell script, which
//! imports the T-12.1b session environment into the systemd user manager,
//! starts [`SESSION_TARGET`], waits for the compositor anchor to exit, and
//! tears the target and the runtime hand-off files down.
//!
//! The entry file and the script are embedded at build time from the same
//! files the acceptance tests read (`services/session/dragonfruit.desktop`,
//! `services/session/dragonfruit-session-entry`), so the installed and the
//! checked content cannot drift. [`install_into`] writes them plus the
//! shipped systemd user units under a prefix; T-32 packaging calls it with
//! its `$RPM_BUILD_ROOT`/`$DESTDIR`.
//!
//! The contract is frozen in
//! [ADR 0066](../../docs/design/adr/0066-display-manager-session-entry.md).

use std::io;
use std::path::{Path, PathBuf};

/// The user-visible session name (the `.desktop` `Name=`).
pub const SESSION_NAME: &str = "Dragonfruit";
/// The session entry filename under `share/wayland-sessions/`.
pub const SESSION_FILE: &str = "dragonfruit.desktop";
/// The installed entry script's basename (the `.desktop` `Exec`).
pub const ENTRY_PROGRAM: &str = "dragonfruit-session-entry";
/// The absolute entry command a display manager launches.
pub const ENTRY_COMMAND: &str = "/usr/bin/dragonfruit-session-entry";
/// The `DesktopNames=` value (`XDG_CURRENT_DESKTOP`).
pub const DESKTOP_NAME: &str = "dragonfruit";
/// The systemd user target the entry starts and stops.
pub const SESSION_TARGET: &str = "dragonfruit-session.target";
/// The compositor unit: the session anchor the entry waits on.
pub const COMPOSITOR_UNIT: &str = "dragonfruit-compositor.service";

/// Where a session entry is installed, relative to the prefix.
pub const WAYLAND_SESSIONS_DIR: &str = "share/wayland-sessions";
/// Where the systemd user units are installed, relative to the prefix.
pub const SYSTEMD_USER_DIR: &str = "lib/systemd/user";
/// Where executables are installed, relative to the prefix.
pub const BIN_DIR: &str = "bin";

/// The shipped entry file, embedded verbatim.
pub const DESKTOP_ENTRY: &str = include_str!("../dragonfruit.desktop");
/// The shipped entry script, embedded verbatim.
pub const ENTRY_SCRIPT: &str = include_str!("../dragonfruit-session-entry");

/// The embedded systemd user units, in the order the installer writes them.
pub const UNIT_FILES: &[(&str, &str)] = &[
    (
        "dragonfruit-session.target",
        include_str!("../units/dragonfruit-session.target"),
    ),
    (
        "dragonfruit-compositor.service",
        include_str!("../units/dragonfruit-compositor.service"),
    ),
    (
        "dragonfruit-shell.service",
        include_str!("../units/dragonfruit-shell.service"),
    ),
    (
        "dragonfruit-settingsd.service",
        include_str!("../units/dragonfruit-settingsd.service"),
    ),
    (
        "dragonfruit-menu-broker.service",
        include_str!("../units/dragonfruit-menu-broker.service"),
    ),
    (
        "dragonfruit-app-index.service",
        include_str!("../units/dragonfruit-app-index.service"),
    ),
    (
        "dragonfruit-notifications.service",
        include_str!("../units/dragonfruit-notifications.service"),
    ),
    (
        "dragonfruit-portal.service",
        include_str!("../units/dragonfruit-portal.service"),
    ),
];

/// Write the session entry, the entry script (mode 0755), and the systemd
/// user units under `prefix`. Returns every path written, in install order.
pub fn install_into(prefix: &Path) -> io::Result<Vec<PathBuf>> {
    let bin = prefix.join(BIN_DIR);
    let sessions = prefix.join(WAYLAND_SESSIONS_DIR);
    let units = prefix.join(SYSTEMD_USER_DIR);
    std::fs::create_dir_all(&bin)?;
    std::fs::create_dir_all(&sessions)?;
    std::fs::create_dir_all(&units)?;

    let mut written = Vec::new();

    let entry = bin.join(ENTRY_PROGRAM);
    std::fs::write(&entry, ENTRY_SCRIPT)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(0o755))?;
    }
    written.push(entry);

    let desktop = sessions.join(SESSION_FILE);
    std::fs::write(&desktop, DESKTOP_ENTRY)?;
    written.push(desktop);

    for (name, contents) in UNIT_FILES {
        let path = units.join(name);
        std::fs::write(&path, contents)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_desktop_entry_names_our_session_and_our_entry() {
        assert!(DESKTOP_ENTRY.contains("Type=Application"));
        assert!(DESKTOP_ENTRY.contains(&format!("Name={SESSION_NAME}")));
        assert!(DESKTOP_ENTRY.contains(&format!("Exec={ENTRY_COMMAND}")));
        assert!(DESKTOP_ENTRY.contains(&format!("DesktopNames={DESKTOP_NAME}")));
    }

    #[test]
    fn the_entry_script_runs_the_contract_sequence() {
        for needle in [
            "--print-env",
            "import-environment",
            "XDG_CURRENT_DESKTOP",
            "XDG_SESSION_TYPE",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "DRAGONFRUIT_LAUNCH_TOKEN",
            "start \"$target\"",
            "stop \"$target\"",
            "reset-failed",
            "$socket.x11-display",
            "$socket.launch-token",
        ] {
            assert!(
                ENTRY_SCRIPT.contains(needle),
                "entry script lacks {needle:?}"
            );
        }
    }

    #[test]
    fn every_embedded_unit_is_named_after_its_file() {
        for (name, contents) in UNIT_FILES {
            assert!(name.ends_with(".service") || name.ends_with(".target"));
            assert!(!contents.is_empty(), "{name} is empty");
        }
    }
}
