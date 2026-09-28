// SPDX-License-Identifier: MIT
//! The Accessibility adapter: one read path over the shared contract.
//!
//! It holds the last status the AT-SPI accessibility bus published and exposes
//! the three-state contract ([`AdapterState`]). [`refresh`](Self::refresh) is
//! the one place it touches the host stack; the host calls it when the bus
//! reports a change, so nothing above the adapter polls.
//!
//! A new adapter starts [`AdapterState::Unavailable`] and
//! [`ConnectionState::Absent`] — the safe "bus absent" default, so a session
//! booted without AT-SPI renders a hidden item and never blocks.
//!
//! The adapter is read-only by design: `org.a11y.Status` is published by the
//! host stack. The user's durable accessibility preferences are `settingsd`'s
//! and magnification is compositor-owned; this adapter is the live bridge
//! projection, exactly as the input adapter is the inventory half.

use std::collections::VecDeque;

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::{AccessibilityChange, AccessibilitySnapshot};
use crate::source::AccessibilitySource;

/// The Accessibility adapter.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessibilityAdapter<S> {
    source: S,
    state: AdapterState<AccessibilitySnapshot>,
    previous: Option<AccessibilitySnapshot>,
    changes: VecDeque<AccessibilityChange>,
    subscription: Subscription,
}

impl<S: AccessibilitySource> AccessibilityAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        AccessibilityAdapter {
            source,
            state: AdapterState::Unavailable,
            previous: None,
            changes: VecDeque::new(),
            subscription: Subscription::new(),
        }
    }

    /// Read the accessibility bus once and update the state and the event
    /// stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    ///
    /// The first successful read is the baseline and reports no domain change;
    /// each later read reports an [`AccessibilityChange`] per flag that moved.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                let snapshot = AccessibilitySnapshot::from_data(&data);
                if let Some(previous) = &self.previous {
                    self.changes.extend(snapshot.changes(previous));
                }
                self.previous = Some(snapshot);
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

    /// The live snapshot, when the bus answered.
    pub fn snapshot(&self) -> Option<&AccessibilitySnapshot> {
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
    pub fn drain_changes(&mut self) -> Vec<AccessibilityChange> {
        self.changes.drain(..).collect()
    }

    /// The domain change count not yet drained.
    pub fn pending_changes(&self) -> usize {
        self.changes.len()
    }
}

impl<S: AccessibilitySource> Adapter for AccessibilityAdapter<S> {
    type Snapshot = AccessibilitySnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::ACCESSIBILITY
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
    use crate::source::{AccessibilityData, MockAccessibility};

    fn data() -> AccessibilityData {
        AccessibilityData {
            enabled: true,
            screen_reader: false,
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = AccessibilityAdapter::new(MockAccessibility::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::ACCESSIBILITY).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = AccessibilityAdapter::new(MockAccessibility::present(data()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(adapter.snapshot().unwrap().is_enabled());
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
        let mut adapter = AccessibilityAdapter::new(MockAccessibility::present(data()));
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
    fn an_all_off_bus_is_available_but_not_present() {
        let mut adapter =
            AccessibilityAdapter::new(MockAccessibility::present(AccessibilityData::default()));
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(!adapter.snapshot().unwrap().present());
        assert_eq!(adapter.snapshot().unwrap().label(), "Off");
    }

    #[test]
    fn a_read_failure_is_an_error_and_leaves_the_item_inert() {
        let mut adapter = AccessibilityAdapter::new(MockAccessibility::failing("at-spi: timeout"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        assert!(adapter.state().is_visible());
        assert!(!adapter.state().is_enabled());
        assert!(adapter.snapshot().is_none());
        assert_eq!(
            adapter.state().error().unwrap().message(),
            "at-spi: timeout"
        );
    }

    #[test]
    fn a_flag_move_is_a_domain_event() {
        let mut adapter = AccessibilityAdapter::new(MockAccessibility::present(data()));
        adapter.refresh();
        let _ = adapter.drain_changes();
        let _ = adapter.drain_events();

        adapter.source_mut().push(AccessibilityData {
            enabled: true,
            screen_reader: true,
        });
        adapter.refresh();

        assert_eq!(
            adapter.drain_changes(),
            vec![AccessibilityChange::ScreenReader { enabled: true }]
        );
        assert_eq!(adapter.pending_changes(), 0);
        assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
    }

    #[test]
    fn the_adapter_is_read_only() {
        // No write methods exist: a compile-time contract, documented here so a
        // later task that wants writes has to add them deliberately. The host
        // stack publishes `org.a11y.Status`; the durable switches are
        // settingsd's and magnification is compositor-owned.
        let _ = AccessibilityAdapter::new(MockAccessibility::present(data()));
    }
}
