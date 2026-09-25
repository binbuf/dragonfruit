// SPDX-License-Identifier: MIT
//! `dragonfruit-notifications`: the notification service (T-11.1a).
//!
//! The service owns the notification queue and its history
//! ([01-architecture.md](../../docs/design/01-architecture.md)) and speaks two
//! session-bus interfaces:
//!
//! * `org.freedesktop.Notifications` — the standard app-facing interface every
//!   Linux app already uses ([`dbus::FreedesktopNotifications`]).
//! * `org.dragonfruit.Notifications1` — the shell-facing interface the Qt
//!   shell reads for banners and the notification-center history
//!   ([`dbus::ShellNotifications`]).
//!
//! # What lives here
//!
//! * [`model`] — the queue, the notification value, the bounded history, and
//!   the Focus/DND policy state. No D-Bus and no JSON, so it is unit-tested
//!   directly.
//! * [`policy`] — the Focus/DND mode, the per-app allow list, and the
//!   admission rule (T-11.2a).
//! * [`view`] — the flat JSON views the shell decodes.
//! * [`dbus`] — the two interfaces, the shared queue, and the event-driven
//!   expiry thread.

pub mod dbus;
pub mod model;
pub mod policy;
pub mod view;

pub use dbus::{
    new_wake, FreedesktopNotifications, ShellNotifications, DBUS_NAME, DBUS_PATH,
    FREEDESKTOP_INTERFACE, SHELL_INTERFACE,
};
pub use model::{
    now_ms, Action, CloseReason, HistoryEntry, Notification, NotifyRequest, Queue, Urgency,
    DEFAULT_TIMEOUT_MS, HISTORY_CAPACITY,
};
pub use policy::{FocusMode, FocusPolicy};
pub use view::{banners_json, focus_policy_json, history_json};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_timeout_and_history_capacity_are_documented_constants() {
        assert_eq!(DEFAULT_TIMEOUT_MS, 5000);
        assert_eq!(HISTORY_CAPACITY, 100);
    }
}
