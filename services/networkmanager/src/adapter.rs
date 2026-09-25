// SPDX-License-Identifier: MIT
//! The NetworkManager adapter: one read path over the shared contract.

use dragonfruit_system_adapters::{
    Adapter, AdapterError, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::model::WifiSnapshot;
use crate::source::{ActivateOutcome, ActivateRequest, NetworkManagerSource};

/// A request from the menu bar to join a network.
///
/// The secret is held only for the duration of one [`NetworkManagerAdapter::join`]
/// call and [`Debug`](std::fmt::Debug) redacts it.
#[derive(Clone, PartialEq, Eq)]
pub struct JoinRequest {
    /// The SSID to join.
    pub ssid: String,
    /// The pre-shared key for a secured network; `None` for an open network or
    /// when a secret agent owns the credential.
    pub secret: Option<String>,
}

impl std::fmt::Debug for JoinRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JoinRequest")
            .field("ssid", &self.ssid)
            .field("secret", &self.secret.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

impl JoinRequest {
    /// A join for `ssid` with an optional secret.
    pub fn new(ssid: impl Into<String>, secret: Option<String>) -> Self {
        JoinRequest {
            ssid: ssid.into(),
            secret,
        }
    }
}

/// How much of the Wi-Fi adapter the session may use.
///
/// Reads always work. When polkit refuses a join, the adapter degrades to
/// [`WifiAccess::ReadOnly`] and records the note; the network list stays live,
/// and only the join affordance is disabled. The degradation persists across
/// refills, because a successful read does not grant the write permission back.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WifiAccess {
    /// Reads and joins are both allowed.
    #[default]
    ReadWrite,
    /// A join was refused by polkit; `note` records why.
    ReadOnly {
        /// The recorded degradation note (the denial message).
        note: String,
    },
}

impl WifiAccess {
    /// A read-only degradation carrying `note`.
    pub fn read_only(note: impl Into<String>) -> Self {
        WifiAccess::ReadOnly { note: note.into() }
    }

    /// Whether joins are disabled.
    pub fn is_read_only(&self) -> bool {
        matches!(self, WifiAccess::ReadOnly { .. })
    }

    /// The recorded degradation note, when read-only.
    pub fn note(&self) -> Option<&str> {
        match self {
            WifiAccess::ReadWrite => None,
            WifiAccess::ReadOnly { note } => Some(note),
        }
    }
}

/// What happened to a [`NetworkManagerAdapter::join`] request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinResult {
    /// NetworkManager accepted the request; activation is in progress. The
    /// daemon will push the state change, and the host will
    /// [`refresh`](NetworkManagerAdapter::refresh) on the event.
    Accepted,
    /// polkit refused the request. The adapter is now
    /// [`WifiAccess::ReadOnly`] and records the note; reads still work.
    Denied {
        /// The recorded denial note.
        note: String,
    },
    /// The daemon is absent; there is nothing to join.
    Absent,
    /// The request failed for a reason other than authorization. The read
    /// state is left untouched.
    Failed(AdapterError),
}

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
    access: WifiAccess,
}

impl<S: NetworkManagerSource> NetworkManagerAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        NetworkManagerAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
            access: WifiAccess::ReadWrite,
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

    /// Join (activate) a network. The one write the adapter makes.
    ///
    /// A successful join changes nothing here yet: NetworkManager reports the
    /// resulting state through its subscription, and the host
    /// [`refresh`](Self::refresh)es on that event, so the snapshot stays the
    /// single source of truth.
    ///
    /// A polkit denial is recorded as a [`WifiAccess::ReadOnly`] degradation
    /// (see [`WifiAccess`]): the network list stays live, but further joins are
    /// refused by the adapter before reaching the daemon.
    pub fn join(&mut self, request: &JoinRequest) -> JoinResult {
        if let WifiAccess::ReadOnly { note } = &self.access {
            // The session already learned it cannot write; do not hammer the
            // daemon with a request polkit will refuse again.
            return JoinResult::Denied { note: note.clone() };
        }

        let outcome = self.source.activate(&ActivateRequest {
            ssid: request.ssid.clone(),
            secret: request.secret.clone(),
            access_point: None,
        });

        match outcome {
            ActivateOutcome::Accepted => JoinResult::Accepted,
            ActivateOutcome::Denied(note) => {
                self.access = WifiAccess::read_only(note.clone());
                JoinResult::Denied { note }
            }
            ActivateOutcome::Absent => {
                self.state = AdapterState::Unavailable;
                self.subscription.absent();
                JoinResult::Absent
            }
            ActivateOutcome::Failed(error) => JoinResult::Failed(error),
        }
    }

    /// How much of the adapter the session may use (read-write or read-only).
    pub fn access(&self) -> &WifiAccess {
        &self.access
    }

    /// Whether a polkit denial has degraded the adapter to read-only.
    pub fn is_read_only(&self) -> bool {
        self.access.is_read_only()
    }

    /// The recorded read-only degradation note, if any.
    pub fn degradation_note(&self) -> Option<&str> {
        self.access.note()
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

    #[test]
    fn a_join_against_a_present_mock_is_accepted_and_stays_read_write() {
        let mut adapter =
            NetworkManagerAdapter::new(MockNetworkManager::present(Default::default()));
        adapter.refresh();

        let result = adapter.join(&JoinRequest::new("home", Some("hunter2".to_owned())));
        assert_eq!(result, JoinResult::Accepted);
        assert_eq!(adapter.access(), &WifiAccess::ReadWrite);
        assert!(!adapter.is_read_only());
        assert_eq!(adapter.degradation_note(), None);
        assert_eq!(adapter.source().activations(), 1);
    }

    #[test]
    fn a_polkit_denial_degrades_to_read_only_and_records_the_note() {
        let mut adapter =
            NetworkManagerAdapter::new(MockNetworkManager::present(Default::default()));
        adapter.refresh();
        adapter.source_mut().deny_joins("not authorized to join");

        let result = adapter.join(&JoinRequest::new("home", Some("hunter2".to_owned())));
        assert_eq!(
            result,
            JoinResult::Denied {
                note: "not authorized to join".to_owned()
            }
        );
        assert_eq!(
            adapter.access(),
            &WifiAccess::read_only("not authorized to join")
        );
        assert!(adapter.is_read_only());
        assert_eq!(adapter.degradation_note(), Some("not authorized to join"));
    }

    #[test]
    fn a_denied_join_keeps_the_read_state_live_and_blocks_further_writes() {
        let mut adapter =
            NetworkManagerAdapter::new(MockNetworkManager::present(Default::default()));
        adapter.refresh();
        let _ = adapter.drain_events();
        adapter.source_mut().deny_joins("not authorized");

        adapter.join(&JoinRequest::new("home", None));
        assert!(adapter.state().is_available());
        assert!(adapter.state().is_visible());
        assert!(adapter.state().is_enabled());
        assert!(adapter.state().slot(AdapterId::WIFI).visible);

        // Once read-only, a second join is refused locally, without touching
        // the daemon again.
        let result = adapter.join(&JoinRequest::new("home", None));
        assert!(matches!(result, JoinResult::Denied { .. }));
        assert_eq!(adapter.source().activations(), 1);
    }

    #[test]
    fn a_refresh_while_read_only_keeps_the_degradation() {
        let mut adapter =
            NetworkManagerAdapter::new(MockNetworkManager::present(Default::default()));
        adapter.refresh();
        adapter.source_mut().deny_joins("not authorized");
        adapter.join(&JoinRequest::new("home", None));

        adapter.refresh();
        assert!(adapter.state().is_available());
        assert!(adapter.is_read_only());
        assert_eq!(adapter.degradation_note(), Some("not authorized"));
    }

    #[test]
    fn a_join_failure_other_than_denial_does_not_degrade() {
        let mut adapter =
            NetworkManagerAdapter::new(MockNetworkManager::present(Default::default()));
        adapter.refresh();
        adapter.source_mut().fail_joins("unknown network");

        let result = adapter.join(&JoinRequest::new("ghost", None));
        assert_eq!(
            result,
            JoinResult::Failed(AdapterError::new("unknown network"))
        );
        assert!(!adapter.is_read_only());
        assert_eq!(adapter.access(), &WifiAccess::ReadWrite);
    }

    #[test]
    fn a_join_while_absent_reports_absence_and_hides() {
        let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::absent());
        let result = adapter.join(&JoinRequest::new("home", None));
        assert_eq!(result, JoinResult::Absent);
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::WIFI).visible);
    }

    #[test]
    fn a_debug_join_request_redacts_the_secret() {
        let request = JoinRequest::new("home", Some("hunter2".to_owned()));
        let debug = format!("{request:?}");
        assert!(debug.contains("home"));
        assert!(!debug.contains("hunter2"));
        assert!(debug.contains("[redacted]"));
    }
}
