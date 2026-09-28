// SPDX-License-Identifier: MIT
//! The Lock Screen policy adapter: one read path over the shared contract.
//!
//! It holds the last snapshot the host stack published and exposes the
//! three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is the
//! one place it touches the host stack; the shell bridge calls it when the
//! lock state or the policy changes, so nothing above the adapter polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "host stack absent" default, so a
//! session booted without a compositor lock bridge renders a hidden item and
//! never blocks.
//!
//! The adapter is read-only: the compositor owns the one fail-secure lock
//! state and the session owns the idle/lock timing; the durable display
//! preferences are `settingsd`'s (T-15.8b). This adapter answers "is the
//! session locked, and what policy applies?".

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{LockPolicyChange, LockPolicySnapshot};
use crate::source::LockPolicySource;

/// The Lock Screen policy adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct LockPolicyAdapter<S> {
    source: S,
    state: AdapterState<LockPolicySnapshot>,
    previous: Option<LockPolicySnapshot>,
    changes: VecDeque<LockPolicyChange>,
    subscription: Subscription,
}

impl<S: LockPolicySource> LockPolicyAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        LockPolicyAdapter {
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
    /// each later read reports a [`LockPolicyChange`] per field that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = LockPolicySnapshot::from_data(&data);
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

    /// The live snapshot, when the host stack answered.
    pub fn snapshot(&self) -> Option<&LockPolicySnapshot> {
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
    pub fn drain_changes(&mut self) -> Vec<LockPolicyChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: LockPolicySource> Adapter for LockPolicyAdapter<S> {
    type Snapshot = LockPolicySnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::LOCK
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
    use crate::model::LockState;
    use crate::source::{LockDisplay, LockPolicyData, MockLockPolicy};

    fn data(locked: bool) -> LockPolicyData {
        LockPolicyData {
            locked,
            idle: dragonfruit_session::IdlePolicy::new(),
            display: LockDisplay::default(),
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = LockPolicyAdapter::new(MockLockPolicy::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::LOCK).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = LockPolicyAdapter::new(MockLockPolicy::present(data(true)));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(adapter.snapshot().unwrap().is_locked());
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
        let mut adapter = LockPolicyAdapter::new(MockLockPolicy::present(data(false)));
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
        let mut adapter = LockPolicyAdapter::new(MockLockPolicy::failing("lock bridge: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "lock bridge: timeout"
        );
    }

    #[test]
    fn locking_is_a_domain_event() {
        let mut adapter = LockPolicyAdapter::new(MockLockPolicy::present(data(false)));
        adapter.refresh();
        let _ = adapter.drain_changes();

        adapter.source_mut().push(data(true));
        adapter.refresh();
        assert_eq!(
            adapter.drain_changes(),
            vec![LockPolicyChange::StateChanged {
                from: LockState::Unlocked,
                to: LockState::Locked,
            }]
        );
        assert_eq!(adapter.pending_changes(), 0);
    }

    #[test]
    fn an_absent_write_free_adapter_stays_hidden() {
        let mut adapter = LockPolicyAdapter::new(MockLockPolicy::absent());
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        // The slot is hidden, so a consumer neither draws nor enables it.
        let slot = adapter.state().slot(AdapterId::LOCK);
        assert!(!slot.visible && !slot.enabled);
    }
}
