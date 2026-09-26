// SPDX-License-Identifier: MIT
//! `xdg-desktop-portal-dragonfruit`: the Dragonfruit `xdg-desktop-portal`
//! backend (T-13.1a).
//!
//! The backend is a real session-bus service discovered by the
//! `xdg-desktop-portal` frontend from the `dragonfruit.portal` descriptor and
//! `dragonfruit-portals.conf` this crate ships. It owns the standard name
//! `org.freedesktop.impl.portal.desktop.dragonfruit` at
//! `/org/freedesktop/portal/desktop` and degrades gracefully when the
//! frontend is absent: registering and serving do not depend on it
//! ([`dbus`]).
//!
//! # What lives here
//!
//! * [`model`] — the pure identity, advertised interfaces, and the
//!   frontend-presence state. No D-Bus, so it is unit-tested directly.
//! * [`data`] — the three discoverability files (`.portal`, `portals.conf`,
//!   D-Bus activation), their parsers, and [`data::install_into`] for
//!   packaging.
//! * [`dbus`] — the session-bus service and the live frontend watch.
//!
//! The first concrete portal interfaces land in T-13.1b (Settings,
//! GlobalShortcuts); FileChooser, Screenshot, and ScreenCast follow in
//! T-13.2a…T-13.4a. The registration contract is frozen in
//! [ADR 0074](../../docs/design/adr/0074-portal-backend-registration-and-frontend-degradation.md).

pub mod data;
pub mod dbus;
pub mod model;

pub use data::{
    install_into, preferred_backends, ACTIVATION_FILE, ACTIVATION_FILE_NAME, DBUS_SERVICES_DIR,
    PORTALS_CONF, PORTALS_CONF_DIR, PORTALS_CONF_NAME, PORTALS_DIR, PORTAL_FILE, PORTAL_FILE_NAME,
};
pub use dbus::{initialize, probe_frontend, serve, Backend};
pub use model::{
    BackendStatus, FrontendPresence, FrontendTracker, BACKEND_INTERFACES, BACKEND_NAME, DBUS_NAME,
    DBUS_PATH, FRONTEND_NAME, STATUS_INTERFACE,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_backend_name_follows_the_lockstep_contract() {
        assert_eq!(DBUS_NAME, "org.freedesktop.impl.portal.desktop.dragonfruit");
        assert!(DBUS_NAME.starts_with("org.freedesktop.impl.portal.desktop."));
        assert!(DBUS_NAME.ends_with(df_ipc::DESKTOP_NAME));
        assert!(df_ipc::is_valid_dbus_name(STATUS_INTERFACE));
    }
}
