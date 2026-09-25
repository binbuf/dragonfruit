// SPDX-License-Identifier: MIT
//! Thin, mockable adapters over the host system services (T-07.1a).
//!
//! The menu bar's Wi-Fi, volume, and battery items read from host daemons
//! (NetworkManager, PipeWire/WirePlumber, UPower) through adapters, never
//! directly ([07-system-integration.md]). This crate owns the one contract
//! every subsystem implements, plus a mock for headless tests. The concrete
//! adapters land in T-07.2 (networking), T-07.3 (audio), and T-07.4 (power).
//!
//! # The contract
//!
//! An adapter holds the last state pushed by its daemon and exposes it as one
//! of three [`AdapterState`]s:
//!
//! * [`AdapterState::Available`] — the daemon answered; the typed snapshot is
//!   the live data the consumer renders.
//! * [`AdapterState::Unavailable`] — the daemon is absent. This is a normal
//!   state, never an error, and never blocks session startup
//!   ([07-system-integration.md] principle 4).
//! * [`AdapterState::Error`] — the daemon is present but the adapter could not
//!   read it. The slot is shown, inert, with the message.
//!
//! Nothing above an adapter knows which daemon implements it.
//!
//! # No polling above the adapter
//!
//! Consumers read [`Adapter::state`], which returns the already-pushed
//! snapshot; only the adapter itself talks to its daemon. The push path is the
//! event subscription (T-07.1b): an adapter records the lifecycle transitions
//! its daemon reports in a [`Subscription`] and exposes them as
//! [`AdapterEvent`]s through [`Adapter::drain_events`]. A consumer reacts to
//! an event by re-reading the state; it never polls the daemon, and absent
//! daemons are a normal hidden state rather than an error or a startup
//! blocker.
//!
//! # Consumers render the right slot
//!
//! A consumer projects each adapter to a [`StatusSlot`] — the shape the
//! shell's menu-bar slot renders (`shell/menubar/StatusItem.qml`) — with
//! [`AdapterState::slot`] or, for a heterogeneous set, [`status_slots`].
//! The projection is the only place that decides hidden vs. visible-inert
//! vs. interactive.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

mod mock;
mod state;
mod subscription;

pub use mock::MockAdapter;
pub use state::{AdapterError, AdapterId, AdapterState, StatusSlot};
pub use subscription::{AdapterEvent, ConnectionState, SubscribeError, Subscription};

/// One system-service adapter.
///
/// Implemented once per subsystem behind a stable internal API, with a
/// [`MockAdapter`] for tests. Nothing above the adapter names the daemon.
pub trait Adapter {
    /// The daemon-specific snapshot the adapter publishes when available.
    type Snapshot;

    /// Stable identity of the status slot this adapter feeds.
    fn id(&self) -> AdapterId;

    /// The last state pushed by the daemon. Never a blocking query of the
    /// daemon and never a poll loop above the adapter.
    fn state(&self) -> &AdapterState<Self::Snapshot>;

    /// The subscription lifecycle of this adapter's daemon. [`ConnectionState::Absent`]
    /// means the daemon is not running, in which case the state is
    /// `Unavailable` and the slot hides.
    fn connection(&self) -> ConnectionState;

    /// Take the daemon events pushed since the last call, in order.
    ///
    /// The host's event loop drives this; the menu bar never calls it and
    /// never polls the daemon. After any event, re-read [`Adapter::state`].
    fn drain_events(&mut self) -> Vec<AdapterEvent>;
}

/// Type-erased view of an adapter as a menu-bar status source, so a consumer
/// can hold adapters with different snapshot types together.
pub trait StatusSource {
    /// Stable identity of the status slot this source feeds.
    fn id(&self) -> AdapterId;

    /// The menu-bar projection of the source's current state.
    fn slot(&self) -> StatusSlot;
}

impl<A: Adapter> StatusSource for A {
    fn id(&self) -> AdapterId {
        Adapter::id(self)
    }

    fn slot(&self) -> StatusSlot {
        Adapter::state(self).slot(Adapter::id(self))
    }
}

/// The consumer projection: the ordered status slots a bar renders for a set
/// of adapters.
///
/// Hidden and errored slots are included; the consumer decides how to draw
/// them from [`StatusSlot`]. Keeping the order at the call site means the
/// caller owns the bar's left-to-right arrangement.
pub fn status_slots<'a, I>(adapters: I) -> Vec<StatusSlot>
where
    I: IntoIterator<Item = &'a dyn StatusSource>,
{
    adapters.into_iter().map(StatusSource::slot).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_source_erases_the_snapshot_type() {
        let wifi = MockAdapter::with_available(AdapterId::WIFI, 42u8);
        let battery = MockAdapter::<u8>::new(AdapterId::POWER);
        let sources: [&dyn StatusSource; 2] = [&wifi, &battery];

        let slots = status_slots(sources);
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0].id, AdapterId::WIFI);
        assert!(slots[0].visible && slots[0].enabled);
        assert_eq!(slots[1].id, AdapterId::POWER);
        assert!(!slots[1].visible);
    }
}
