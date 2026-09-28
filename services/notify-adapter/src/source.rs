// SPDX-License-Identifier: MIT
//! The transport seam: the raw notification-service read and its mock.
//!
//! A [`NotificationsSource`] is the only thing that talks to the host stack.
//! The host stack here is the **notification service** the desktop already
//! runs (`services/notifications`): it owns the queue, the bounded history,
//! and the Focus/DND policy, and it serves both the standard
//! `org.freedesktop.Notifications` interface and the shell-facing
//! `org.dragonfruit.Notifications1` interface at
//! `/org/freedesktop/Notifications` (ADR [0056], [0058]).
//!
//! This crate never reimplements a queue, a history, or the Focus/DND
//! admission rule. It reuses the service's `FocusMode` vocabulary and its flat
//! JSON views, and it is the **projection**: the current policy, the active
//! banners, the recorded history, and the explicit Focus writes, delivered
//! through a seam so the status host (T-15.7b) and CI's
//! [`MockNotifications`] answer identically.
//!
//! The read is three-way, exactly as the adapter contract
//! ([`dragonfruit_system_adapters`]) needs it:
//!
//! * `Ok(Some(data))` — the service answered; `data` is the live read.
//! * `Ok(None)` — the service is absent (no session bus, or no notification
//!   daemon). A normal state; the item hides.
//! * `Err(error)` — the service owns its name but could not be read; the item
//!   shows visible and inert with the message.
//!
//! [0056]: ../../../docs/design/adr/0056-notification-service-surface-and-shell-banner.md
//! [0058]: ../../../docs/design/adr/0058-focus-dnd-policy-semantics.md

use dragonfruit_notifications::{FocusMode, Urgency};
use dragonfruit_system_adapters::AdapterError;

/// One recorded notification, the flattened read of the service's
/// `Banners()` / `History()` JSON views.
///
/// Only the fields a consumer draws are carried; the full body and the action
/// list stay in the service and reach the shell through its existing banner
/// path. `suppressed` is meaningful only for a history entry (the service
/// flags a banner the Focus policy silenced) and is always `false` on an
/// active banner.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NotificationRecord {
    /// The service-assigned notification id.
    pub id: u32,
    /// The app name the `Notify` call carried.
    pub app_name: String,
    /// The one-line summary.
    pub summary: String,
    /// The urgency hint.
    pub urgency: Urgency,
    /// Whether the Focus policy suppressed this notification's banner (a
    /// history-only flag).
    pub suppressed: bool,
    /// Wall-clock creation time (milliseconds since the Unix epoch).
    pub created_at_ms: u64,
}

/// The raw result of one notification-service read: the Focus/DND policy and
/// the two notification lists.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NotificationsData {
    /// The Focus/DND mode (`off` / `focus` / `dnd`).
    pub mode: FocusMode,
    /// The per-app Focus allow list, in the service's order.
    pub allow_list: Vec<String>,
    /// How many notifications the current suppression has batched.
    pub batched_count: u32,
    /// The active banners, oldest first (the service's order).
    pub active: Vec<NotificationRecord>,
    /// The recorded history, most recent first (the service's order).
    pub history: Vec<NotificationRecord>,
}

/// The result of one Focus/DND write.
///
/// Selecting a mode or replacing the allow list is an explicit user action,
/// never a poll. A write that lands invents no snapshot: the service pushes
/// `Changed`, the host re-reads, and the snapshot stays the single source of
/// truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusOutcome {
    /// The service accepted the write.
    Applied,
    /// The service is absent; there is nothing to write.
    Absent,
    /// The write failed for any other reason.
    Failed(AdapterError),
}

impl FocusOutcome {
    /// Whether the service accepted the write.
    pub fn is_applied(&self) -> bool {
        matches!(self, FocusOutcome::Applied)
    }

    /// The failure, when the write failed.
    pub fn error(&self) -> Option<&AdapterError> {
        match self {
            FocusOutcome::Failed(error) => Some(error),
            FocusOutcome::Applied | FocusOutcome::Absent => None,
        }
    }
}

/// Reads the notification service over some transport.
pub trait NotificationsSource {
    /// One read of the service.
    fn read(&mut self) -> Result<Option<NotificationsData>, AdapterError>;

    /// Set the Focus/DND mode. One explicit write.
    ///
    /// The default is [`FocusOutcome::Absent`], so a source without a write
    /// half still satisfies the trait.
    fn set_focus_mode(&mut self, _mode: FocusMode) -> FocusOutcome {
        FocusOutcome::Absent
    }

    /// Replace the per-app Focus allow list. One explicit write.
    fn set_focus_allow_list(&mut self, _apps: Vec<String>) -> FocusOutcome {
        FocusOutcome::Absent
    }
}

/// A fixture-backed source with a simulated service lifecycle.
///
/// The mock is the CI path: it serves [`NotificationsData`] with no
/// notification service on the bus, and `kill`/`restart` exercise absence and
/// re-subscribe the way masking the service would. Its two writes mutate the
/// simulated policy the way the service would, so a subsequent `refresh` sees
/// the change.
#[derive(Debug, Clone, PartialEq)]
pub struct MockNotifications {
    present: bool,
    data: Option<NotificationsData>,
    failure: Option<AdapterError>,
    write_failure: Option<AdapterError>,
    reads: u32,
    focus_writes: u32,
}

impl MockNotifications {
    /// The service is not running.
    pub fn absent() -> Self {
        MockNotifications {
            present: false,
            data: None,
            failure: None,
            write_failure: None,
            reads: 0,
            focus_writes: 0,
        }
    }

    /// A running service that answers with `data`.
    pub fn present(data: NotificationsData) -> Self {
        MockNotifications {
            present: true,
            data: Some(data),
            failure: None,
            write_failure: None,
            reads: 0,
            focus_writes: 0,
        }
    }

    /// A running service that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockNotifications {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            write_failure: None,
            reads: 0,
            focus_writes: 0,
        }
    }

    /// A running service whose writes fail.
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// The service pushes fresh data.
    pub fn push(&mut self, data: NotificationsData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The service goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The service comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated service is running.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many Focus writes the source has accepted.
    pub fn focus_writes(&self) -> u32 {
        self.focus_writes
    }
}

impl NotificationsSource for MockNotifications {
    fn read(&mut self) -> Result<Option<NotificationsData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn set_focus_mode(&mut self, mode: FocusMode) -> FocusOutcome {
        if !self.present {
            return FocusOutcome::Absent;
        }
        if let Some(error) = &self.write_failure {
            return FocusOutcome::Failed(error.clone());
        }
        if let Some(data) = self.data.as_mut() {
            data.mode = mode;
        }
        self.focus_writes += 1;
        FocusOutcome::Applied
    }

    fn set_focus_allow_list(&mut self, apps: Vec<String>) -> FocusOutcome {
        if !self.present {
            return FocusOutcome::Absent;
        }
        if let Some(error) = &self.write_failure {
            return FocusOutcome::Failed(error.clone());
        }
        if let Some(data) = self.data.as_mut() {
            data.allow_list = apps;
        }
        self.focus_writes += 1;
        FocusOutcome::Applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(mode: FocusMode) -> NotificationsData {
        NotificationsData {
            mode,
            allow_list: vec!["Chat".to_owned()],
            batched_count: 2,
            active: vec![NotificationRecord {
                id: 1,
                app_name: "Mail".to_owned(),
                summary: "New message".to_owned(),
                urgency: Urgency::Normal,
                suppressed: false,
                created_at_ms: 1000,
            }],
            history: vec![],
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockNotifications::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockNotifications::present(data(FocusMode::Focus));
        assert_eq!(mock.read(), Ok(Some(data(FocusMode::Focus))));
        assert!(mock.is_present());
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockNotifications::failing("notifications: timeout");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "notifications: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockNotifications::present(data(FocusMode::Off));
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data(FocusMode::Off)));
    }

    #[test]
    fn a_focus_write_mutates_the_simulated_policy() {
        let mut mock = MockNotifications::present(data(FocusMode::Off));
        assert_eq!(mock.set_focus_mode(FocusMode::Dnd), FocusOutcome::Applied);
        assert_eq!(mock.focus_writes(), 1);
        assert_eq!(mock.read().unwrap().unwrap().mode, FocusMode::Dnd);

        assert_eq!(
            mock.set_focus_allow_list(vec!["Pager".to_owned()]),
            FocusOutcome::Applied
        );
        assert_eq!(mock.focus_writes(), 2);
        assert_eq!(
            mock.read().unwrap().unwrap().allow_list,
            vec!["Pager".to_owned()]
        );
    }

    #[test]
    fn a_write_while_absent_is_absent() {
        let mut mock = MockNotifications::absent();
        assert_eq!(mock.set_focus_mode(FocusMode::Focus), FocusOutcome::Absent);
        assert_eq!(
            mock.set_focus_allow_list(vec!["Mail".to_owned()]),
            FocusOutcome::Absent
        );
        assert_eq!(mock.focus_writes(), 0);
    }

    #[test]
    fn a_failing_write_reports_the_error() {
        let mut mock = MockNotifications::present(data(FocusMode::Off)).fail_writes("rejected");
        let outcome = mock.set_focus_mode(FocusMode::Dnd);
        assert_eq!(outcome.error().map(AdapterError::message), Some("rejected"));
        assert!(!outcome.is_applied());
    }
}
