// SPDX-License-Identifier: GPL-3.0-or-later
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
/// Where the shipped default wallpaper is installed, relative to the prefix.
pub const WALLPAPER_DIR: &str = "share/dragonfruit/wallpapers";
/// The shipped default wallpaper's installed filename.
pub const WALLPAPER_FILE: &str = "Default.jpg";
/// The shipped default wallpaper's source, relative to the session crate.
pub const WALLPAPER_ASSET: &str = "../../assets/graphics/wallpapers/Default.jpg";
/// The bundled built-in wallpapers as (installed filename, source relative to
/// the session crate). The first entry is the shipped default.
pub const WALLPAPER_ASSETS: &[(&str, &str)] = &[
    (WALLPAPER_FILE, WALLPAPER_ASSET),
    ("Lagoon.jpg", "../../assets/graphics/wallpapers/Lagoon.jpg"),
    (
        "Midnight.jpg",
        "../../assets/graphics/wallpapers/Midnight.jpg",
    ),
];

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
        "dragonfruit-files-desktop.service",
        include_str!("../units/dragonfruit-files-desktop.service"),
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
        "dragonfruit-system-status.service",
        include_str!("../units/dragonfruit-system-status.service"),
    ),
    (
        "dragonfruit-wallpaperd.service",
        include_str!("../units/dragonfruit-wallpaperd.service"),
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

    // The shipped built-in wallpapers (T-18.1a): installed to the stable share
    // path the provider resolves. Original project assets, GPL-3.0-or-later
    // (ADR 0094, ADR 0366); never fetched/third-party images.
    let wallpaper_dir = prefix.join(WALLPAPER_DIR);
    std::fs::create_dir_all(&wallpaper_dir)?;
    for (name, source) in WALLPAPER_ASSETS {
        let wallpaper = wallpaper_dir.join(name);
        std::fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(source),
            &wallpaper,
        )?;
        written.push(wallpaper);
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
            "--sync-toolkit",
            "import-environment",
            "daemon-reload",
            "XDG_CURRENT_DESKTOP",
            "XDG_SESSION_TYPE",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "GTK_CSD",
            "DRAGONFRUIT_LAUNCH_TOKEN",
            "DRAGONFRUIT_DESKTOP_LAUNCH_TOKEN",
            "DRAGONFRUIT_DEV_RETURN",
            "DRAGONFRUIT_DEV_BIN",
            "XDG_SESSION_ID",
            "LD_LIBRARY_PATH",
            "QML_IMPORT_PATH",
            "share/dragonfruit/qml",
            "native.env",
            "DF_SESSION_START_TIMEOUT",
            "start \"$target\"",
            "stop \"$target\"",
            "xdg-desktop-portal.service",
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
    fn the_entry_stops_a_stale_target_before_the_handoff_cleanup() {
        let stale = ENTRY_SCRIPT
            .find("stopping an already-active")
            .expect("the entry stops a stale target");
        let cleanup = ENTRY_SCRIPT
            .find("Remove hand-off files")
            .expect("the entry removes hand-off files");
        assert!(
            stale < cleanup,
            "the stale target must stop before the socket cleanup"
        );

        let start_failure = ENTRY_SCRIPT
            .find("failed to start")
            .expect("the entry reports a failed start");
        assert!(
            ENTRY_SCRIPT[start_failure..].contains("stop \"$target\""),
            "a failed start stops the target"
        );
    }

    #[test]
    fn every_embedded_unit_is_named_after_its_file() {
        for (name, contents) in UNIT_FILES {
            assert!(name.ends_with(".service") || name.ends_with(".target"));
            assert!(!contents.is_empty(), "{name} is empty");
        }
    }
}
