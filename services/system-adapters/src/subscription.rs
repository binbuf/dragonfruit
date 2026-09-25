// SPDX-License-Identifier: MIT
//! Event subscription, restart re-subscribe, and absence (T-07.1b).
//!
//! An adapter is never asked about its daemon; the daemon pushes. Every
//! adapter shares one subscription lifecycle, owned here:
//!
//! * [`Subscription::subscribed`] records that the adapter established — or,
//!   after an absence, re-established — its subscription. A concrete adapter
//!   calls it once it has hooked its daemon's signal source (a D-Bus
//!   `PropertiesChanged` + `NameOwnerChanged` watch, a PipeWire registry).
//!   Calling it again while already subscribed is a no-op.
//! * [`Subscription::absent`] records that the daemon is gone. The adapter
//!   drops its subscription and its state becomes [`AdapterState::Unavailable`],
//!   so the slot hides. Absence is a normal state, never an error, and never
//!   blocks session startup.
//! * [`Subscription::changed`] records that the daemon pushed data.
//!
//! The lifecycle is exposed to the host as [`AdapterEvent`]s, taken from
//! `Adapter::drain_events`. A consumer re-reads `Adapter::state` after an
//! event; it never polls the daemon.
//!
//! A restart is modelled as an absence followed by a fresh subscribe:
//! `absent()` then `subscribed()` emits `Disconnected`, then
//! `Subscribed { resubscribe: true }`, and bumps the subscribe count. Nothing
//! above the adapter needs to know the daemon restarted.

use std::collections::VecDeque;

use crate::state::{AdapterError, AdapterState};

/// Whether an adapter currently holds a live subscription to its daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// The daemon is absent; no subscription is held and the slot hides.
    #[default]
    Absent,
    /// The daemon is present and the adapter holds a subscription.
    Subscribed,
}

impl ConnectionState {
    /// Whether the adapter holds a subscription.
    pub const fn is_subscribed(self) -> bool {
        matches!(self, ConnectionState::Subscribed)
    }

    /// A stable label for logs and the audit trail.
    pub const fn name(self) -> &'static str {
        match self {
            ConnectionState::Absent => "absent",
            ConnectionState::Subscribed => "subscribed",
        }
    }
}

/// A notification pushed by a daemon subscription.
///
/// Events carry no snapshot: the adapter already holds the last state and the
/// consumer re-reads it. Carrying the payload here would let the event stream
/// and the adapter state disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterEvent {
    /// A subscription was established. `resubscribe` is `true` when this
    /// re-established a subscription after the daemon had gone away (a
    /// restart).
    Subscribed { resubscribe: bool },
    /// The daemon went away; the adapter is now `Unavailable` and the slot
    /// hides.
    Disconnected,
    /// The daemon pushed data; re-read `Adapter::state`.
    Changed,
}

impl AdapterEvent {
    /// A stable label for logs and the audit trail.
    pub const fn name(&self) -> &'static str {
        match self {
            AdapterEvent::Subscribed { resubscribe: false } => "subscribed",
            AdapterEvent::Subscribed { resubscribe: true } => "resubscribed",
            AdapterEvent::Disconnected => "disconnected",
            AdapterEvent::Changed => "changed",
        }
    }
}

/// Why an attempt to establish a subscription failed.
///
/// The distinction is the point of the adapter contract: a missing daemon is
/// *absence* (hidden, not an error) while a present-but-broken daemon is an
/// *error* (visible, inert). Neither aborts session startup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscribeError {
    /// The daemon is not on the bus. Not an error.
    Absent,
    /// The daemon is present but the subscription failed.
    Failed(AdapterError),
}

impl SubscribeError {
    /// The adapter state a failed subscribe leaves behind: absence hides the
    /// slot, a failure shows it inert with the message.
    pub fn to_state<T>(&self) -> AdapterState<T> {
        match self {
            SubscribeError::Absent => AdapterState::Unavailable,
            SubscribeError::Failed(error) => AdapterState::Error(error.clone()),
        }
    }
}

/// The subscription bookkeeping and event outbox for one adapter.
///
/// The mock drives it directly; a concrete adapter drives it from its
/// daemon's name-owner/signal handling. Consumers never construct one — they
/// read the [`AdapterEvent`] stream through `Adapter::drain_events`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Subscription {
    state: ConnectionState,
    subscriptions: u32,
    events: VecDeque<AdapterEvent>,
}

impl Subscription {
    /// The bounded outbox capacity. The host drains every loop iteration, so
    /// this only bounds a pathological producer.
    pub const DEFAULT_CAPACITY: usize = 256;

    /// A fresh subscription: absent, with no events.
    pub fn new() -> Self {
        Subscription {
            state: ConnectionState::Absent,
            subscriptions: 0,
            events: VecDeque::new(),
        }
    }

    /// Record that a subscription was established (or re-established).
    ///
    /// The first call emits `Subscribed { resubscribe: false }`; a call after
    /// an [`absent`](Self::absent) emits `Subscribed { resubscribe: true }`
    /// and is counted as a restart. A redundant call while already subscribed
    /// is a no-op, so a busy event loop cannot inflate the restart count.
    pub fn subscribed(&mut self) {
        if self.state.is_subscribed() {
            return;
        }
        let resubscribe = self.subscriptions > 0;
        self.state = ConnectionState::Subscribed;
        self.subscriptions += 1;
        self.push(AdapterEvent::Subscribed { resubscribe });
    }

    /// Record that the daemon is gone. The adapter drops its subscription.
    ///
    /// Emits `Disconnected` once; a second call while already absent is a
    /// no-op. Absence is not an error.
    pub fn absent(&mut self) {
        if !self.state.is_subscribed() {
            return;
        }
        self.state = ConnectionState::Absent;
        self.push(AdapterEvent::Disconnected);
    }

    /// Record a data push from the daemon.
    pub fn changed(&mut self) {
        self.push(AdapterEvent::Changed);
    }

    /// Record the outcome of an attempted subscribe, keeping the connection
    /// state and the adapter state consistent.
    ///
    /// A failed subscribe against an absent daemon is absence; a failure
    /// against a present daemon leaves the link marked subscribed (the name is
    /// owned) while the adapter state carries the error.
    pub fn record_subscribe(&mut self, result: Result<(), SubscribeError>) {
        match result {
            Ok(()) | Err(SubscribeError::Failed(_)) => self.subscribed(),
            Err(SubscribeError::Absent) => self.absent(),
        }
    }

    /// The current connection lifecycle.
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Whether the adapter holds a subscription.
    pub fn is_subscribed(&self) -> bool {
        self.state.is_subscribed()
    }

    /// How many times a subscription has been established. The first subscribe
    /// is `1`; a daemon restarted `n` times gives `n + 1`.
    pub fn subscriptions(&self) -> u32 {
        self.subscriptions
    }

    /// Take every event pushed since the last drain, in order.
    pub fn drain_events(&mut self) -> Vec<AdapterEvent> {
        self.events.drain(..).collect()
    }

    /// Number of pending events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    fn push(&mut self, event: AdapterEvent) {
        if self.events.len() >= Self::DEFAULT_CAPACITY {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_subscription_is_absent_and_silent() {
        let subscription = Subscription::new();
        assert_eq!(subscription.state(), ConnectionState::Absent);
        assert!(!subscription.is_subscribed());
        assert_eq!(subscription.subscriptions(), 0);
        assert!(subscription.is_empty());
    }

    #[test]
    fn the_first_subscribe_is_not_a_resubscribe() {
        let mut subscription = Subscription::new();
        subscription.subscribed();
        assert!(subscription.is_subscribed());
        assert_eq!(subscription.subscriptions(), 1);
        assert_eq!(
            subscription.drain_events(),
            vec![AdapterEvent::Subscribed { resubscribe: false }]
        );
    }

    #[test]
    fn a_restart_resubscribes_and_is_counted() {
        let mut subscription = Subscription::new();
        subscription.subscribed();
        let _ = subscription.drain_events();

        subscription.absent();
        assert_eq!(subscription.state(), ConnectionState::Absent);
        subscription.subscribed();

        assert_eq!(subscription.subscriptions(), 2);
        assert_eq!(
            subscription.drain_events(),
            vec![
                AdapterEvent::Disconnected,
                AdapterEvent::Subscribed { resubscribe: true },
            ]
        );
    }

    #[test]
    fn redundant_lifecycle_calls_are_no_ops() {
        let mut subscription = Subscription::new();
        subscription.absent(); // never subscribed: no event
        subscription.subscribed();
        subscription.subscribed(); // already: no event, no extra count
        subscription.absent();
        subscription.absent();

        assert_eq!(subscription.subscriptions(), 1);
        assert_eq!(
            subscription.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Disconnected,
            ]
        );
    }

    #[test]
    fn absence_and_failure_map_to_different_states() {
        let absent: AdapterState<u8> = SubscribeError::Absent.to_state();
        assert!(absent.is_unavailable());
        assert!(!absent.is_error());

        let failed: AdapterState<u8> =
            SubscribeError::Failed(AdapterError::new("NetworkManager: timeout")).to_state();
        assert!(failed.is_error());
        assert!(failed.is_visible());
        assert!(!failed.is_enabled());
    }

    #[test]
    fn record_subscribe_tracks_presence() {
        let mut subscription = Subscription::new();
        subscription.record_subscribe(Err(SubscribeError::Absent));
        assert_eq!(subscription.state(), ConnectionState::Absent);

        subscription.record_subscribe(Ok(()));
        assert!(subscription.is_subscribed());

        subscription.record_subscribe(Err(SubscribeError::Absent));
        assert_eq!(subscription.state(), ConnectionState::Absent);

        subscription.record_subscribe(Err(SubscribeError::Failed(AdapterError::new("boom"))));
        assert!(subscription.is_subscribed());
        assert_eq!(subscription.subscriptions(), 2);
    }

    #[test]
    fn events_are_bounded_and_drain_in_order() {
        let mut subscription = Subscription::new();
        subscription.subscribed();
        for _ in 0..(Subscription::DEFAULT_CAPACITY + 5) {
            subscription.changed();
        }
        assert_eq!(subscription.len(), Subscription::DEFAULT_CAPACITY);
        let events = subscription.drain_events();
        assert_eq!(events.len(), Subscription::DEFAULT_CAPACITY);
        assert!(subscription.is_empty());
        // The oldest events were evicted.
        assert_eq!(events[0], AdapterEvent::Changed);
        assert_eq!(events[1], AdapterEvent::Changed);
    }
}
