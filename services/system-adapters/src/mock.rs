// SPDX-License-Identifier: MIT
//! A configurable adapter for tests, with a simulated daemon lifecycle.

use crate::state::{AdapterId, AdapterState};
use crate::subscription::{AdapterEvent, ConnectionState, Subscription};
use crate::Adapter;

/// A test adapter whose state a test sets directly and whose daemon a test
/// can kill and restart.
///
/// The mock is the mock half of FR-1: a consumer can be driven through all
/// three [`AdapterState`]s and through the subscription lifecycle with no
/// daemon on the bus. A new mock starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "daemon absent" default, so a test
/// that forgets to start the daemon exercises the degradation path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockAdapter<T> {
    id: AdapterId,
    state: AdapterState<T>,
    subscription: Subscription,
    /// The last snapshot the daemon pushed, remembered across an absence so a
    /// restart can re-sync without the snapshot type being `Clone`.
    last_available: Option<T>,
}

impl<T> MockAdapter<T> {
    /// An absent mock for `id`.
    pub fn new(id: AdapterId) -> Self {
        MockAdapter {
            id,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
            last_available: None,
        }
    }

    /// A mock whose daemon is present and has already pushed `snapshot`.
    pub fn with_available(id: AdapterId, snapshot: T) -> Self {
        let mut subscription = Subscription::new();
        subscription.subscribed();
        MockAdapter {
            id,
            state: AdapterState::Available(snapshot),
            subscription,
            last_available: None,
        }
    }

    /// The daemon pushed a new snapshot.
    pub fn set_available(&mut self, snapshot: T) {
        self.subscription.subscribed();
        self.state = AdapterState::available(snapshot);
        self.last_available = None;
        self.subscription.changed();
    }

    /// The daemon is absent (no subscription, slot hidden).
    pub fn set_unavailable(&mut self) {
        self.last_available = take_snapshot(&mut self.state);
        self.state = AdapterState::Unavailable;
        self.subscription.absent();
    }

    /// The daemon is present but could not be read (slot visible, inert).
    pub fn set_error(&mut self, message: impl Into<String>) {
        self.subscription.subscribed();
        self.last_available = take_snapshot(&mut self.state);
        self.state = AdapterState::with_error(message);
        self.subscription.changed();
    }

    /// Kill the simulated daemon: the adapter drops its subscription and the
    /// slot hides. Absence, never an error.
    pub fn kill(&mut self) {
        self.set_unavailable();
    }

    /// Restart the simulated daemon: the adapter re-subscribes and re-syncs to
    /// the last snapshot the daemon pushed, if any.
    pub fn restart(&mut self) {
        self.subscription.subscribed();
        if let Some(snapshot) = self.last_available.take() {
            self.state = AdapterState::available(snapshot);
            self.subscription.changed();
        }
    }

    /// The current connection lifecycle.
    pub fn connection(&self) -> ConnectionState {
        self.subscription.state()
    }

    /// How many times the adapter has subscribed (a restart counts again).
    pub fn subscriptions(&self) -> u32 {
        self.subscription.subscriptions()
    }

    /// Take the events pushed since the last call.
    pub fn events(&mut self) -> Vec<AdapterEvent> {
        self.subscription.drain_events()
    }
}

/// Move an available snapshot out of `state`, leaving it `Unavailable`.
fn take_snapshot<T>(state: &mut AdapterState<T>) -> Option<T> {
    match std::mem::replace(state, AdapterState::Unavailable) {
        AdapterState::Available(snapshot) => Some(snapshot),
        other => {
            *state = other;
            None
        }
    }
}

impl<T> Adapter for MockAdapter<T> {
    type Snapshot = T;

    fn id(&self) -> AdapterId {
        self.id
    }

    fn state(&self) -> &AdapterState<T> {
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
    use crate::StatusSource;

    #[test]
    fn a_new_mock_is_unavailable_and_absent() {
        let mock = MockAdapter::<u8>::new(AdapterId::WIFI);
        assert!(mock.state().is_unavailable());
        assert_eq!(mock.connection(), ConnectionState::Absent);
        assert_eq!(mock.subscriptions(), 0);
        assert!(!mock.slot().visible);
    }

    #[test]
    fn a_mock_can_be_driven_through_every_state() {
        let mut mock = MockAdapter::new(AdapterId::AUDIO);

        mock.set_available(0.5_f32);
        assert!(mock.state().is_available());
        assert_eq!(mock.state().snapshot(), Some(&0.5));
        assert!(mock.slot().visible && mock.slot().enabled);
        assert!(mock.connection().is_subscribed());

        mock.set_error("PipeWire: connection refused");
        assert!(mock.state().is_error());
        assert!(mock.slot().visible && !mock.slot().enabled);
        assert_eq!(
            mock.slot().error.as_deref(),
            Some("PipeWire: connection refused")
        );

        mock.set_unavailable();
        assert!(mock.state().is_unavailable());
        assert!(!mock.slot().visible);
        assert_eq!(mock.connection(), ConnectionState::Absent);
    }

    #[test]
    fn an_initially_available_mock_carries_its_snapshot() {
        let mock = MockAdapter::with_available(AdapterId::POWER, 0.42_f64);
        assert_eq!(mock.state().snapshot(), Some(&0.42));
        assert_eq!(Adapter::id(&mock), AdapterId::POWER);
        assert!(mock.connection().is_subscribed());
        assert_eq!(mock.subscriptions(), 1);
    }

    #[test]
    fn kill_hides_the_slot_and_restart_resubscribes() {
        let mut mock = MockAdapter::with_available(AdapterId::WIFI, 82u8);
        let _ = mock.events();

        mock.kill();
        assert_eq!(mock.connection(), ConnectionState::Absent);
        assert!(mock.state().is_unavailable());
        assert!(!mock.slot().visible);
        assert_eq!(mock.events(), vec![AdapterEvent::Disconnected]);

        mock.restart();
        assert!(mock.connection().is_subscribed());
        assert!(mock.state().is_available());
        assert!(mock.slot().visible && mock.slot().enabled);
        assert_eq!(mock.subscriptions(), 2);
        assert_eq!(
            mock.events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: true },
                AdapterEvent::Changed,
            ]
        );
    }
}
