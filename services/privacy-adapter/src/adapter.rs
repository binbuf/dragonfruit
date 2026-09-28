// SPDX-License-Identifier: MIT
//! The Privacy and Security adapter: one read path over the shared contract.
//!
//! It holds the last snapshot the portal PermissionStore published and exposes
//! the three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is
//! the one place it touches the host stack; the shell bridge calls it when the
//! store changes, so nothing above the adapter polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "store absent" default, so a session
//! booted without `xdg-desktop-portal` renders a hidden item and never blocks.
//!
//! The writes ([`set_permission`](Self::set_permission) and
//! [`delete_permission`](Self::delete_permission)) are explicit user actions
//! over the same seam. They invent no snapshot: the PermissionStore publishes
//! the resulting state and the host re-reads, so the snapshot stays the single
//! source of truth.

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{PrivacyChange, PrivacySnapshot};
use crate::source::{PrivacyOutcome, PrivacySource};

/// The Privacy and Security adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct PrivacyAdapter<S> {
    source: S,
    state: AdapterState<PrivacySnapshot>,
    previous: Option<PrivacySnapshot>,
    changes: VecDeque<PrivacyChange>,
    subscription: Subscription,
}

impl<S: PrivacySource> PrivacyAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        PrivacyAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the permission store once and update the state and the event
    /// stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain change;
    /// each later read reports a [`PrivacyChange`] per entry that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = PrivacySnapshot::from_data(&data);
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

    /// Store `permissions` for `app` on the resource `id` in `table`. One
    /// explicit write.
    pub fn set_permission(
        &mut self,
        table: &str,
        id: &str,
        app: &str,
        permissions: &[String],
    ) -> PrivacyOutcome {
        self.source.set_permission(table, id, app, permissions)
    }

    /// Remove the stored permission for `app` on the resource `id` in `table`.
    /// One explicit write.
    pub fn delete_permission(&mut self, table: &str, id: &str, app: &str) -> PrivacyOutcome {
        self.source.delete_permission(table, id, app)
    }

    /// The live snapshot, when the store answered.
    pub fn snapshot(&self) -> Option<&PrivacySnapshot> {
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
    pub fn drain_changes(&mut self) -> Vec<PrivacyChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: PrivacySource> Adapter for PrivacyAdapter<S> {
    type Snapshot = PrivacySnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::PRIVACY
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
    use crate::source::{AppPermissionData, MockPrivacy, PrivacyData, ResourceData, TableData};

    fn data() -> PrivacyData {
        PrivacyData {
            tables: vec![TableData {
                table: "devices".to_owned(),
                resources: vec![ResourceData {
                    id: "camera".to_owned(),
                    apps: vec![AppPermissionData {
                        app: "org.mozilla.firefox".to_owned(),
                        permissions: vec!["ask".to_owned()],
                    }],
                }],
            }],
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = PrivacyAdapter::new(MockPrivacy::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::PRIVACY).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = PrivacyAdapter::new(MockPrivacy::present(data()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.snapshot().unwrap().app_count(), 1);
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(adapter.source().reads(), 1);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
        // The first read is the baseline: no domain change.
        assert!(adapter.drain_changes().is_empty());
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter = PrivacyAdapter::new(MockPrivacy::present(data()));
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
        let mut adapter = PrivacyAdapter::new(MockPrivacy::failing("portal: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "portal: timeout"
        );
    }

    #[test]
    fn an_empty_store_is_available_but_not_present() {
        let mut adapter = PrivacyAdapter::new(MockPrivacy::present(PrivacyData { tables: vec![] }));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(!adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().label(), "No App Permissions");
    }

    #[test]
    fn an_app_move_is_a_domain_event() {
        let mut adapter = PrivacyAdapter::new(MockPrivacy::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let mut next = data();
        next.tables[0].resources[0].apps[0].permissions = vec!["yes".to_owned()];
        adapter.source_mut().push(next);
        adapter.refresh();

        assert_eq!(
            adapter.drain_changes(),
            vec![PrivacyChange::AppChanged {
                table: "devices".to_owned(),
                id: "camera".to_owned(),
                app: "org.mozilla.firefox".to_owned(),
            }]
        );
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn the_writes_delegate_to_the_source() {
        let mut adapter = PrivacyAdapter::new(MockPrivacy::present(data()));
        adapter.refresh();

        assert!(adapter
            .set_permission(
                "devices",
                "camera",
                "org.mozilla.firefox",
                &["yes".to_owned()]
            )
            .is_applied());
        assert!(adapter
            .delete_permission("devices", "camera", "org.mozilla.firefox")
            .is_applied());

        assert_eq!(adapter.source().permission_sets(), 1);
        assert_eq!(adapter.source().permission_deletes(), 1);
        assert_eq!(adapter.source().writes(), 2);
    }

    #[test]
    fn writes_while_absent_are_absent() {
        let mut adapter = PrivacyAdapter::new(MockPrivacy::absent());
        adapter.refresh();
        assert_eq!(
            adapter.set_permission("devices", "camera", "x", &[]),
            PrivacyOutcome::Absent
        );
        // A write never invents state: the adapter stays absent until a read.
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::PRIVACY).visible);
    }

    #[test]
    fn a_denial_is_reported_and_leaves_the_read_state_live() {
        let mut adapter =
            PrivacyAdapter::new(MockPrivacy::present(data()).deny_writes("portal: denied"));
        adapter.refresh();
        let outcome = adapter.set_permission("devices", "camera", "x", &["no".to_owned()]);
        assert_eq!(outcome.denial_note(), Some("portal: denied"));
        assert!(adapter.state().is_available());
        assert!(adapter.state().slot(AdapterId::PRIVACY).visible);
    }
}
