// SPDX-License-Identifier: MIT
//! The NetworkManager adapter: one read path over the shared contract.

use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::WifiSnapshot;
use crate::source::NetworkManagerSource;

/// The Wi-Fi status adapter.
///
/// It holds the last snapshot the daemon pushed and exposes the three-state
/// contract ([`AdapterState`]). [`refresh`](Self::refresh) is the one place it
/// touches the daemon; the host calls it when NetworkManager signals a change,
/// so nothing above the adapter polls.
///
/// A new adapter starts [`AdapterState::Unavailable`] and
/// [`ConnectionState::Absent`] — the safe "daemon absent" default, so a
/// session booted without NetworkManager renders a hidden item and never
/// blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkManagerAdapter<S> {
    source: S,
    state: AdapterState<WifiSnapshot>,
    subscription: Subscription,
}

impl<S: NetworkManagerSource> NetworkManagerAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        NetworkManagerAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
        }
    }

    /// Read the daemon once and update the state and the event stream.
    ///
    /// The three outcomes map straight to the contract:
    ///
    /// * data → `Available`, `Subscribed`/`Changed`;
    /// * absent → `Unavailable`, `Disconnected`;
    /// * error → `Error` (visible, inert), `Subscribed`/`Changed`.
    pub fn refresh(&mut self) {
        match self.source.read() {
            Ok(Some(data)) => {
                self.subscription.subscribed();
                self.state = AdapterState::available(WifiSnapshot::from_data(&data));
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

    /// The live snapshot, when the daemon answered.
    pub fn snapshot(&self) -> Option<&WifiSnapshot> {
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
}

impl<S: NetworkManagerSource> Adapter for NetworkManagerAdapter<S> {
    type Snapshot = WifiSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::WIFI
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
    use crate::source::MockNetworkManager;

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = NetworkManagerAdapter::new(MockNetworkManager::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::WIFI).visible);
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let source = MockNetworkManager::present(Default::default());
        let mut adapter = NetworkManagerAdapter::new(source);
        adapter.refresh();
        assert!(adapter.state().is_available());
        assert_eq!(adapter.connection(), ConnectionState::Subscribed);
        assert_eq!(adapter.subscriptions(), 1);
        assert_eq!(
            adapter.drain_events(),
            vec![
                AdapterEvent::Subscribed { resubscribe: false },
                AdapterEvent::Changed,
            ]
        );
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let source = MockNetworkManager::present(Default::default());
        let mut adapter = NetworkManagerAdapter::new(source);
        adapter.refresh();
        let _ = adapter.drain_events();

        adapter.source_mut().kill();
        adapter.refresh();
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().is_error());
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
}
