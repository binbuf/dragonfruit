// SPDX-License-Identifier: MIT
//! The Notifications and Focus adapter: one read path over the shared
//! contract.

use std::collections::VecDeque;

use dragonfruit_notifications::FocusMode;
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{NotificationsChange, NotificationsSnapshot};
use crate::source::{FocusOutcome, NotificationsSource};

/// The Notifications and Focus adapter.
///
/// It holds the last snapshot the notification service pushed and exposes the
/// three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is the
/// one place it touches the service; the host calls it when the service
/// signals `Changed`, so nothing above the adapter polls. A read is diffed
/// against the previous one into a [`NotificationsChange`] stream, and
/// [`set_focus_mode`](Self::set_focus_mode) /
/// [`set_focus_allow_list`](Self::set_focus_allow_list) are the two writes.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "service absent" default, so a
/// session booted without a notification service renders a hidden item and
/// never blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct NotificationsAdapter<S> {
    source: S,
    state: AdapterState<NotificationsSnapshot>,
    previous: Option<NotificationsSnapshot>,
    changes: VecDeque<NotificationsChange>,
    subscription: Subscription,
}

impl<S: NotificationsSource> NotificationsAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        NotificationsAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the service once and update the state and the event stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain
    /// change; each later read reports a [`NotificationsChange`] per field
    /// that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = NotificationsSnapshot::from_data(&data);
                if let Some(previous) = &self.previous {
                    self.changes.extend(snapshot.changes(previous));
                }
                self.previous = Some(snapshot.clone());
                self.state = AdapterState::available(snapshot);
                self.subscription.changed();
            }
            Ok(None) => {
                self.state = AdapterState::Unavailable;
                self.subscription.absent();
            }
            Err(error) => {
                self.subscription.subscribed();
                self.state = AdapterState::Error(error);
                self.subscription.changed();
            }
        }
    }

    /// Set the Focus/DND mode. One explicit write.
    ///
    /// A successful write invents no snapshot: the service pushes `Changed`
    /// and the host calls [`refresh`](Self::refresh), so the snapshot stays
    /// the single source of truth. The caller reacts to the [`FocusOutcome`]
    /// and [refreshes](Self::refresh) on success.
    pub fn set_focus_mode(&mut self, mode: FocusMode) -> FocusOutcome {
        let outcome = self.source.set_focus_mode(mode);
        self.after_write(outcome)
    }

    /// Replace the per-app Focus allow list. One explicit write.
    pub fn set_focus_allow_list(&mut self, apps: Vec<String>) -> FocusOutcome {
        let outcome = self.source.set_focus_allow_list(apps);
        self.after_write(outcome)
    }

    /// A write against an absent service is absence at the state level too:
    /// the item hides and the subscription drops, never an error. It re-reads
    /// so the snapshot stays truthful.
    fn after_write(&mut self, outcome: FocusOutcome) -> FocusOutcome {
        if matches!(outcome, FocusOutcome::Absent) {
            self.refresh();
        }
        outcome
    }

    /// The live snapshot, when the service answered.
    pub fn snapshot(&self) -> Option<&NotificationsSnapshot> {
        self.state.snapshot()
    }

    /// The transport this adapter reads.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// The transport, mutably (for tests and lifecycle control).
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// How many times the adapter subscribed (a restart counts again).
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }

    /// Take the domain changes pushed since the last drain, in order.
    pub fn drain_changes(&mut self) -> Vec<NotificationsChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: NotificationsSource> Adapter for NotificationsAdapter<S> {
    type Snapshot = NotificationsSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::NOTIFICATIONS
    }

    fn state(&self) -> &AdapterState<Self::Snapshot> {
        &self.state
    }

    fn connection(&self) -> ConnectionState {
        self.subscription.state()
    }

    fn drain_events(&mut self) -> Vec<AdapterEvent> {
        self.subscription.drain_events()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{MockNotifications, NotificationRecord, NotificationsData};

    fn data(mode: FocusMode) -> NotificationsData {
        NotificationsData {
            mode,
            allow_list: vec!["Pager".to_owned()],
            batched_count: 0,
            active: vec![NotificationRecord {
                id: 1,
                app_name: "Mail".to_owned(),
                summary: "New message".to_owned(),
                ..NotificationRecord::default()
            }],
            history: vec![],
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = NotificationsAdapter::new(MockNotifications::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::NOTIFICATIONS).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter =
            NotificationsAdapter::new(MockNotifications::present(data(FocusMode::Off)));
        adapter.refresh();
        assert!(adapter.state().is_available());
        let snapshot = adapter.snapshot().unwrap();
        assert_eq!(snapshot.focus.mode, FocusMode::Off);
        assert_eq!(snapshot.active_count(), 1);
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
        // The first read is the baseline: no domain change reported.
        assert!(adapter.drain_changes().is_empty());
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter =
            NotificationsAdapter::new(MockNotifications::present(data(FocusMode::Off)));
        adapter.refresh();
        let _ = adapter.drain_events();

        adapter.source_mut().kill();
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

        adapter.source_mut().restart();
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.subscriptions(), 2);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: true },
                AdapterEvent::Changed,
            ]
        );
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter =
            NotificationsAdapter::new(MockNotifications::failing("notifications: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
    }

    #[test]
    fn a_focus_mode_change_is_a_domain_event() {
        let mut adapter =
            NotificationsAdapter::new(MockNotifications::present(data(FocusMode::Off)));
        adapter.refresh();
        assert!(adapter.drain_changes().is_empty());

        adapter.source_mut().push(data(FocusMode::Dnd));
        adapter.refresh();
        assert_eq!(
            adapter.drain_changes(),
            vec![NotificationsChange::FocusModeChanged {
                from: FocusMode::Off,
                to: FocusMode::Dnd,
            }]
        );
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn a_focus_write_updates_the_snapshot_on_refresh() {
        let mut adapter =
            NotificationsAdapter::new(MockNotifications::present(data(FocusMode::Off)));
        adapter.refresh();
        let _ = adapter.drain_changes();

        assert_eq!(
            adapter.set_focus_mode(FocusMode::Dnd),
            FocusOutcome::Applied
        );
        // The write invents no snapshot until the host re-reads.
        assert_eq!(adapter.snapshot().unwrap().focus.mode, FocusMode::Off);
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().focus.mode, FocusMode::Dnd);
        assert_eq!(
            adapter.drain_changes(),
            vec![NotificationsChange::FocusModeChanged {
                from: FocusMode::Off,
                to: FocusMode::Dnd,
            }]
        );

        assert_eq!(
            adapter.set_focus_allow_list(vec!["Chat".to_owned()]),
            FocusOutcome::Applied
        );
        adapter.refresh();
        assert_eq!(
            adapter.snapshot().unwrap().focus.allow_list,
            vec!["Chat".to_owned()]
        );
    }

    #[test]
    fn an_absent_write_resyncs_to_unavailable() {
        let mut adapter = NotificationsAdapter::new(MockNotifications::absent());
        adapter.refresh();
        assert_eq!(adapter.set_focus_mode(FocusMode::Dnd), FocusOutcome::Absent);
        // The resync leaves the item hidden; absence is normal.
        assert!(adapter.state().is_unavailable());
    }
}
