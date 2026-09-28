// SPDX-License-Identifier: MIT
//! The General, About, and Updates adapter: one read path over the shared
//! contract.
//!
//! It holds the last snapshot the host stack published and exposes the
//! three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is the
//! one place it touches the host stack; the shell bridge calls it when the
//! update state or the host identity changes, so nothing above the adapter
//! polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "host stack absent" default, so a
//! session booted without a distribution update provider renders a hidden item
//! and never blocks.
//!
//! The update writes ([`check`](Self::check), [`install`](Self::install),
//! [`reboot`](Self::reboot)) are explicit user actions over the same seam.
//! They invent no snapshot: the provider publishes the resulting state and the
//! host re-reads, so the snapshot stays the single source of truth.

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{SystemSnapshot, UpdateChange};
use crate::source::{SystemSource, UpdateOutcome};

/// The General, About, and Updates adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct UpdateAdapter<S> {
    source: S,
    state: AdapterState<SystemSnapshot>,
    previous: Option<SystemSnapshot>,
    changes: VecDeque<UpdateChange>,
    subscription: Subscription,
}

impl<S: SystemSource> UpdateAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        UpdateAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the host stack once and update the state and the event stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain change;
    /// each later read reports an [`UpdateChange`] per field that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = SystemSnapshot::from_data(&data);
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

    /// Ask the distribution provider to check for updates. One explicit write.
    pub fn check(&mut self) -> UpdateOutcome {
        self.source.check()
    }

    /// Ask the distribution provider to install the available updates. One
    /// explicit write.
    pub fn install(&mut self) -> UpdateOutcome {
        self.source.install()
    }

    /// Ask the distribution provider to restart the host. One explicit write.
    pub fn reboot(&mut self) -> UpdateOutcome {
        self.source.reboot()
    }

    /// The live snapshot, when the host stack answered.
    pub fn snapshot(&self) -> Option<&SystemSnapshot> {
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
    pub fn drain_changes(&mut self) -> Vec<UpdateChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: SystemSource> Adapter for UpdateAdapter<S> {
    type Snapshot = SystemSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::UPDATES
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
    use crate::source::{
        MockSystem, SystemData, SystemIdentity, UpdateData, UpdateItem, UpdatePhase, UpdateSeverity,
    };

    fn data() -> SystemData {
        SystemData {
            identity: SystemIdentity {
                host_name: "dragon".to_owned(),
                os_name: "Dragonfruit Linux".to_owned(),
                os_version: "44".to_owned(),
                ..SystemIdentity::default()
            },
            updates: Some(UpdateData {
                phase: UpdatePhase::UpToDate,
                updates: vec![],
                last_checked_ms: Some(1000),
                message: None,
            }),
        }
    }

    fn available_data() -> SystemData {
        let mut raw = data();
        let updates = raw.updates.as_mut().unwrap();
        updates.phase = UpdatePhase::Available;
        updates.updates = vec![UpdateItem {
            id: "glibc".to_owned(),
            name: "glibc".to_owned(),
            summary: "C library".to_owned(),
            current_version: "2.40".to_owned(),
            available_version: "2.41".to_owned(),
            severity: UpdateSeverity::Security,
        }];
        raw
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = UpdateAdapter::new(MockSystem::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::UPDATES).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = UpdateAdapter::new(MockSystem::present(data()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.snapshot().unwrap().identity.host_name, "dragon");
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
        let mut adapter = UpdateAdapter::new(MockSystem::present(data()));
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
        let mut adapter = UpdateAdapter::new(MockSystem::failing("update provider: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "update provider: timeout"
        );
    }

    #[test]
    fn an_update_move_is_a_domain_event() {
        let mut adapter = UpdateAdapter::new(MockSystem::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        adapter.source_mut().push(available_data());
        adapter.refresh();

        assert_eq!(
            adapter.drain_changes(),
            vec![
                UpdateChange::PhaseChanged {
                    from: UpdatePhase::UpToDate,
                    to: UpdatePhase::Available,
                },
                UpdateChange::UpdateListChanged { from: 0, to: 1 },
            ]
        );
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn an_identity_move_is_a_domain_event() {
        let mut adapter = UpdateAdapter::new(MockSystem::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();

        let mut next = data();
        next.identity.host_name = "renamed".to_owned();
        adapter.source_mut().push(next);
        adapter.refresh();

        assert_eq!(adapter.drain_changes(), vec![UpdateChange::IdentityChanged]);
    }

    #[test]
    fn the_writes_delegate_to_the_source() {
        let mut adapter = UpdateAdapter::new(MockSystem::present(available_data()));
        adapter.refresh();

        assert!(adapter.check().is_applied());
        assert!(adapter.install().is_applied());
        assert!(adapter.reboot().is_applied());
        assert_eq!(adapter.source().checks(), 1);
        assert_eq!(adapter.source().installs(), 1);
        assert_eq!(adapter.source().reboots(), 1);
    }

    #[test]
    fn writes_while_absent_are_absent_never_errors() {
        let mut adapter = UpdateAdapter::new(MockSystem::absent());
        adapter.refresh();
        assert_eq!(adapter.check(), UpdateOutcome::Absent);
        assert_eq!(adapter.install(), UpdateOutcome::Absent);
        assert_eq!(adapter.reboot(), UpdateOutcome::Absent);
    }
}
