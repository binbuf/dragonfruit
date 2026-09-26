// SPDX-License-Identifier: MIT
//! `dragonfruit-app-index`: the application identity service (T-14.1a).
//!
//! app-index is the single owner of application identity
//! ([01-architecture.md](../../docs/design/01-architecture.md)): it maps an
//! `xdg_toplevel.app_id` or an Xwayland `WM_CLASS` to a `.desktop` entry and
//! resolves the entry's themed icon to a file. It serves
//! `org.dragonfruit.AppIndex1` on the user session bus.
//!
//! # What lives here
//!
//! * [`index`] — the pure `.desktop` scan and identity-resolution pipeline
//!   (Wayland app id, X11 `WM_CLASS`, heuristics) plus the miss set and the
//!   install/uninstall/update diff. No D-Bus and no JSON, so it is unit-tested
//!   directly against a fixture corpus.
//! * [`registry`] — the pure launch registry: which apps are running, their
//!   window counts, and the most-recent-first recency order.
//! * [`icons`] — the freedesktop icon-theme lookup that turns an `Icon` name
//!   into a file path.
//! * [`view`] — the flat JSON shapes the shell decodes.
//! * [`watch`] — the inotify directory monitor that keeps the index live.
//! * [`dbus`] — the `org.dragonfruit.AppIndex1` interface and its `run`.
//!
//! The subscription API (coalesced change signals) is T-14.1c.

pub mod dbus;
pub mod icons;
pub mod index;
pub mod registry;
pub mod view;
pub mod watch;

pub use dbus::{AppIndex1, DBUS_NAME, DBUS_PATH, INTERFACE};
pub use icons::IconTheme;
pub use index::{
    desktop_dirs, AppIndex, AppRecord, IdentitySource, IndexEvent, IndexEventKind, ResolvedApp,
};
pub use registry::{ActivityEvent, ActivityKind, LaunchRegistry, RunningApp};
pub use view::{record_json, records_json, DEFAULT_ICON_SIZE};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_documented_dbus_name_is_valid() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME), "{DBUS_NAME}");
    }
}
