// SPDX-License-Identifier: MIT
//! The portal backend's discoverability data (T-13.1a).
//!
//! `xdg-desktop-portal` finds a backend through three installed files; they
//! are embedded here from `portal/data/` so the shipped and the tested
//! content cannot drift, and [`install_into`] writes them under a packaging
//! prefix:
//!
//! * `dragonfruit.portal` → `{DATADIR}/xdg-desktop-portal/portals/` — the
//!   backend's D-Bus name and the interfaces it implements.
//! * `dragonfruit-portals.conf` → `{DATADIR}/xdg-desktop-portal/` — the
//!   `[preferred]` selection read when `XDG_CURRENT_DESKTOP` is
//!   `dragonfruit`.
//! * `org.freedesktop.impl.portal.desktop.dragonfruit.service` →
//!   `{DATADIR}/dbus-1/services/` — the D-Bus activation entry so the
//!   frontend can start the backend on demand.
//!
//! See <https://flatpak.github.io/xdg-desktop-portal/docs/writing-a-new-backend.html>.

use std::io;
use std::path::{Path, PathBuf};

/// The `.portal` descriptor's basename.
pub const PORTAL_FILE_NAME: &str = "dragonfruit.portal";
/// The embedded `.portal` descriptor.
pub const PORTAL_FILE: &str = include_str!("../data/dragonfruit.portal");
/// The `portals.conf` basename.
pub const PORTALS_CONF_NAME: &str = "dragonfruit-portals.conf";
/// The embedded `portals.conf`.
pub const PORTALS_CONF: &str = include_str!("../data/dragonfruit-portals.conf");
/// The D-Bus activation file's basename (equal to the backend name).
pub const ACTIVATION_FILE_NAME: &str = "org.freedesktop.impl.portal.desktop.dragonfruit.service";
/// The embedded D-Bus activation file.
pub const ACTIVATION_FILE: &str =
    include_str!("../data/org.freedesktop.impl.portal.desktop.dragonfruit.service");

/// Install root for `.portal` descriptors, relative to a prefix.
pub const PORTALS_DIR: &str = "share/xdg-desktop-portal/portals";
/// Install root for `portals.conf`, relative to a prefix.
pub const PORTALS_CONF_DIR: &str = "share/xdg-desktop-portal";
/// Install root for D-Bus activation files, relative to a prefix.
pub const DBUS_SERVICES_DIR: &str = "share/dbus-1/services";

/// The `DBusName=` value in `dragonfruit.portal`, if present.
pub fn portal_file_dbus_name(text: &str) -> Option<&str> {
    value_of(text, "DBusName")
}

/// The non-empty `Interfaces=` entries in `dragonfruit.portal`, in order.
pub fn portal_file_interfaces(text: &str) -> Vec<&str> {
    value_of(text, "Interfaces")
        .unwrap_or_default()
        .split(';')
        .filter(|entry| !entry.is_empty())
        .collect()
}

/// The `UseIn=` value in `dragonfruit.portal`, if present.
pub fn portal_file_use_in(text: &str) -> Option<&str> {
    value_of(text, "UseIn")
}

/// The `default=` value in the `[preferred]` group of `portals.conf`, if
/// present.
pub fn portals_conf_default(text: &str) -> Option<&str> {
    value_of(text, "default")
}

/// The `Name=` value in the D-Bus activation file, if present.
pub fn activation_file_name(text: &str) -> Option<&str> {
    value_of(text, "Name")
}

/// The backend names listed in a `portals.conf` value (semicolon-separated).
pub fn preferred_backends(value: &str) -> Vec<&str> {
    value.split(';').filter(|entry| !entry.is_empty()).collect()
}

/// Find the first `key=value` line (ignoring comments and leading spaces) and
/// return its trimmed value.
fn value_of<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#') && !line.starts_with(';'))
        .find_map(|line| {
            let (candidate, value) = line.split_once('=')?;
            (candidate.trim() == key).then(|| value.trim())
        })
}

/// Write the three discoverability files under `prefix`, returning every path
/// written in install order. Packaging (T-16) calls this with its
/// `$RPM_BUILD_ROOT`/`$DESTDIR`.
pub fn install_into(prefix: &Path) -> io::Result<Vec<PathBuf>> {
    let portals = prefix.join(PORTALS_DIR);
    let conf = prefix.join(PORTALS_CONF_DIR);
    let dbus = prefix.join(DBUS_SERVICES_DIR);
    std::fs::create_dir_all(&portals)?;
    std::fs::create_dir_all(&conf)?;
    std::fs::create_dir_all(&dbus)?;

    let mut written = Vec::new();
    for (dir, name, contents) in [
        (&portals, PORTAL_FILE_NAME, PORTAL_FILE),
        (&conf, PORTALS_CONF_NAME, PORTALS_CONF),
        (&dbus, ACTIVATION_FILE_NAME, ACTIVATION_FILE),
    ] {
        let path = dir.join(name);
        std::fs::write(&path, contents)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_portal_descriptor_names_the_backend_and_agrees_with_the_model() {
        assert_eq!(
            portal_file_dbus_name(PORTAL_FILE),
            Some("org.freedesktop.impl.portal.desktop.dragonfruit")
        );
        assert_eq!(
            portal_file_dbus_name(PORTAL_FILE),
            Some(crate::model::DBUS_NAME)
        );
        assert_eq!(portal_file_use_in(PORTAL_FILE), Some("dragonfruit"));
        let interfaces = portal_file_interfaces(PORTAL_FILE);
        let expected: Vec<&str> = crate::model::BACKEND_INTERFACES.to_vec();
        assert_eq!(interfaces, expected, "the descriptor lists what we serve");
    }

    #[test]
    fn the_portals_conf_prefers_dragonfruit_first() {
        let value = portals_conf_default(PORTALS_CONF).expect("default= is set");
        let backends = preferred_backends(value);
        assert_eq!(backends.first().copied(), Some(crate::model::BACKEND_NAME));
        assert!(
            backends.contains(&"gtk"),
            "the generic backend is a fallback"
        );
    }

    #[test]
    fn the_activation_file_matches_the_backend_name() {
        assert_eq!(
            activation_file_name(ACTIVATION_FILE),
            Some(crate::model::DBUS_NAME)
        );
        assert_eq!(
            ACTIVATION_FILE_NAME,
            "org.freedesktop.impl.portal.desktop.dragonfruit.service"
        );
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let text = "; a comment\n\n  DBusName = org.foo.bar  \n";
        assert_eq!(value_of(text, "DBusName"), Some("org.foo.bar"));
        assert_eq!(value_of(text, "Missing"), None);
    }

    #[test]
    fn install_writes_the_three_discoverability_files() {
        let dir = std::env::temp_dir().join(format!("df-portal-install-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);

        let written = install_into(&dir).expect("install the discoverability files");
        assert_eq!(written.len(), 3);
        assert_eq!(
            written,
            vec![
                dir.join(PORTALS_DIR).join(PORTAL_FILE_NAME),
                dir.join(PORTALS_CONF_DIR).join(PORTALS_CONF_NAME),
                dir.join(DBUS_SERVICES_DIR).join(ACTIVATION_FILE_NAME),
            ]
        );
        for path in &written {
            assert!(path.is_file(), "{} was written", path.display());
        }
        assert_eq!(
            std::fs::read_to_string(dir.join(PORTALS_DIR).join(PORTAL_FILE_NAME)).unwrap(),
            PORTAL_FILE
        );
        assert_eq!(
            std::fs::read_to_string(dir.join(DBUS_SERVICES_DIR).join(ACTIVATION_FILE_NAME))
                .unwrap(),
            ACTIVATION_FILE
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
