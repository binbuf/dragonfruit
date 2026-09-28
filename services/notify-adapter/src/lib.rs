// SPDX-License-Identifier: MIT
//! The Notifications and Focus adapter: the Focus/DND policy and the active
//! banner / recorded history state of the notification service (T-15.7a).
//!
//! The desktop already runs a notification service
//! (`services/notifications`, `dragonfruit-notifications`) that owns the
//! queue, the bounded history, and the Focus/DND policy, and serves it over
//! the session bus (ADR [0056], [0058]). Nothing above this crate sees zbus,
//! a service type, or a JSON payload: consumers read a
//! [`NotificationsAdapter`], which holds the last state the service pushed and
//! exposes the three-state contract from `dragonfruit-system-adapters`
//! ([07-system-integration.md]): available, hidden when the service is absent,
//! or visible-and-inert on a read error.
//!
//! # The read path
//!
//! 1. [`DbusNotifications`] reads the **session bus** once per
//!    [`NotificationsAdapter::refresh`] and returns the raw
//!    [`NotificationsData`] (or absence/error).
//! 2. [`crate::model`] folds the policy, the active banners, and the history
//!    into the typed [`NotificationsSnapshot`]: the Focus mode and allow list,
//!    the suppressed batch, the notification counts, and the observed per-app
//!    list.
//! 3. The adapter drives the shared subscription lifecycle, so a service
//!    restart re-subscribes and re-syncs with no user-visible error, and it
//!    diffs each read against the previous one into a
//!    [`NotificationsChange`] stream.
//!
//! # The writes
//!
//! The adapter has two explicit Focus writes,
//! [`NotificationsAdapter::set_focus_mode`] and
//! [`NotificationsAdapter::set_focus_allow_list`]. A successful write invents
//! no snapshot: the service pushes `Changed` and the host re-reads.
//!
//! # Reuse, never reimplement
//!
//! This crate does not own a queue, a history, or the Focus admission rule. It
//! reuses the service's [`FocusMode`](dragonfruit_notifications::FocusMode)
//! vocabulary and its flat JSON views, and it is the *projection* the Settings
//! pane and Control Center tile (T-15.7b) bind to.
//!
//! # Absence
//!
//! A session with no bus, no notification daemon owning
//! `org.freedesktop.Notifications`, or a daemon that does not serve our shell
//! interface is the adapter's `Unavailable` state and hides the item. This is
//! a normal state and never blocks session startup. A service that owns its
//! name but cannot be read is `Error`: visible and inert.
//!
//! # Testing
//!
//! CI has no service on the bus, so the adapter is driven by
//! [`MockNotifications`] over the [`NotificationsSource`] seam.
//! `kill`/`restart` exercise absence and re-subscribe; `push` drives the state
//! and the change stream. The live D-Bus source is proven over a private
//! `dbus-daemon` in `tests/session_bus.rs`.
//!
//! [0056]: ../../../docs/design/adr/0056-notification-service-surface-and-shell-banner.md
//! [0058]: ../../../docs/design/adr/0058-focus-dnd-policy-semantics.md
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod adapter;
mod dbus;
mod model;
mod source;

pub use adapter::NotificationsAdapter;
pub use dbus::{DbusNotifications, NOTIFICATIONS_PATH, NOTIFICATIONS_SERVICE, SHELL_INTERFACE};
pub use model::{AppNotifications, FocusSnapshot, NotificationsChange, NotificationsSnapshot};
pub use source::{
    FocusOutcome, MockNotifications, NotificationRecord, NotificationsData, NotificationsSource,
};
// Re-export the reused policy vocabulary so a consumer binds to one import.
pub use dragonfruit_notifications::FocusMode;

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_system_adapters::Adapter;

    #[test]
    fn the_adapter_reports_the_notifications_slot() {
        let adapter = NotificationsAdapter::new(MockNotifications::absent());
        assert_eq!(
            <NotificationsAdapter<MockNotifications> as Adapter>::id(&adapter),
            dragonfruit_system_adapters::AdapterId::NOTIFICATIONS
        );
    }

    #[test]
    fn absence_is_a_normal_state() {
        let mut adapter = NotificationsAdapter::new(MockNotifications::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn the_focus_modes_are_the_service_vocabulary() {
        assert_eq!(FocusMode::parse("dnd"), Some(FocusMode::Dnd));
        assert_eq!(FocusMode::Dnd.name(), "dnd");
    }
}
