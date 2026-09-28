// SPDX-License-Identifier: MIT
//! The VPN adapter: one read path plus two explicit writes over the shared
//! contract.

use dragonfruit_system_adapters::{
    Adapter, AdapterError, AdapterEvent, AdapterId, AdapterState, ConnectionState, Subscription,
};

use crate::vpn::model::VpnSnapshot;
use crate::vpn::source::{VpnOutcome, VpnRequest, VpnSource};

/// How much of the VPN adapter the session may use.
///
/// Reads always work. When polkit refuses a connect or disconnect, the adapter
/// degrades to [`VpnAccess::ReadOnly`] and records the note; the connection
/// list stays live, and only the write affordances are disabled. The
/// degradation persists across refills, because a successful read does not
/// grant the write permission back. This mirrors the Wi-Fi join degradation
/// (`crate::WifiAccess`, ADR 0027) for the sibling NetworkManager path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VpnAccess {
    /// Reads and writes are both allowed.
    #[default]
    ReadWrite,
    /// A write was refused by polkit; `note` records why.
    ReadOnly {
        /// The recorded degradation note (the denial message).
        note: String,
    },
}

impl VpnAccess {
    /// A read-only degradation carrying `note`.
    pub fn read_only(note: impl Into<String>) -> Self {
        VpnAccess::ReadOnly { note: note.into() }
    }

    /// Whether writes are disabled.
    pub fn is_read_only(&self) -> bool {
        matches!(self, VpnAccess::ReadOnly { .. })
    }

    /// The recorded degradation note, when read-only.
    pub fn note(&self) -> Option<&str> {
        match self {
            VpnAccess::ReadWrite => None,
            VpnAccess::ReadOnly { note } => Some(note),
        }
    }
}

/// What happened to a [`VpnAdapter::connect`] or [`VpnAdapter::deactivate`]
/// request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VpnResult {
    /// NetworkManager accepted the request; the state change is in progress.
    /// The daemon will push the resulting state, and the host will
    /// [`refresh`](VpnAdapter::refresh) on the event.
    Accepted,
    /// polkit refused the request. The adapter is now [`VpnAccess::ReadOnly`]
    /// and records the note; reads still work.
    Denied {
        /// The recorded denial note.
        note: String,
    },
    /// The daemon is absent; there is nothing to connect or disconnect.
    Absent,
    /// The request failed for a reason other than authorization. The read
    /// state is left untouched.
    Failed(AdapterError),
}

/// The VPN status adapter.
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
pub struct VpnAdapter<S> {
    source: S,
    state: AdapterState<VpnSnapshot>,
    subscription: Subscription,
    access: VpnAccess,
}

impl<S: VpnSource> VpnAdapter<S> {
    /// A fresh adapter over `source`, absent until the first
    /// [`refresh`](Self::refresh).
    pub fn new(source: S) -> Self {
        VpnAdapter {
            source,
            state: AdapterState::Unavailable,
            subscription: Subscription::new(),
            access: VpnAccess::ReadWrite,
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
                self.state = AdapterState::available(VpnSnapshot::from_data(&data));
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
    pub fn snapshot(&self) -> Option<&VpnSnapshot> {
        self.state.snapshot()
    }

    /// Connect (activate) the VPN with `uuid`. One explicit write.
    ///
    /// A successful write changes nothing here yet: NetworkManager reports the
    /// resulting state through its subscription, and the host
    /// [`refresh`](Self::refresh)es on that event, so the snapshot stays the
    /// single source of truth.
    ///
    /// A polkit denial is recorded as a [`VpnAccess::ReadOnly`] degradation:
    /// the connection list stays live, but further writes are refused by the
    /// adapter before reaching the daemon.
    pub fn connect(&mut self, uuid: impl Into<String>) -> VpnResult {
        self.write(VpnRequest::new(uuid), VpnWrite::Connect)
    }

    /// Disconnect (deactivate) the active VPN with `uuid`. One explicit write.
    ///
    /// Shares the [`VpnAccess`] degradation with [`connect`](Self::connect).
    pub fn deactivate(&mut self, uuid: impl Into<String>) -> VpnResult {
        self.write(VpnRequest::new(uuid), VpnWrite::Disconnect)
    }

    fn write(&mut self, request: VpnRequest, kind: VpnWrite) -> VpnResult {
        if let VpnAccess::ReadOnly { note } = &self.access {
            // The session already learned it cannot write; do not hammer the
            // daemon with a request polkit will refuse again.
            return VpnResult::Denied { note: note.clone() };
        }

        let outcome = match kind {
            VpnWrite::Connect => self.source.activate(&request),
            VpnWrite::Disconnect => self.source.deactivate(&request),
        };

        match outcome {
            VpnOutcome::Accepted => VpnResult::Accepted,
            VpnOutcome::Denied(note) => {
                self.access = VpnAccess::read_only(note.clone());
                VpnResult::Denied { note }
            }
            VpnOutcome::Absent => {
                self.state = AdapterState::Unavailable;
                self.subscription.absent();
                VpnResult::Absent
            }
            VpnOutcome::Failed(error) => VpnResult::Failed(error),
        }
    }

    /// How much of the adapter the session may use (read-write or read-only).
    pub fn access(&self) -> &VpnAccess {
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

/// Which write a [`VpnAdapter::write`] is routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VpnWrite {
    Connect,
    Disconnect,
}

impl<S: VpnSource> Adapter for VpnAdapter<S> {
    type Snapshot = VpnSnapshot;

    fn id(&self) -> AdapterId {
        AdapterId::VPN
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
    use crate::vpn::source::{MockVpn, VpnConnectionData, VpnData};

    fn connection() -> VpnConnectionData {
        VpnConnectionData {
            path: "/org/freedesktop/NetworkManager/Settings/aaa".to_owned(),
            id: "Work".to_owned(),
            uuid: "aaa".to_owned(),
            kind: "vpn".to_owned(),
            service_type: Some("org.freedesktop.NetworkManager.openvpn".to_owned()),
            autoconnect: false,
        }
    }

    fn data() -> VpnData {
        VpnData {
            connections: vec![connection()],
            active: Vec::new(),
        }
    }

    #[test]
    fn a_new_adapter_is_absent_and_unavailable() {
        let adapter = VpnAdapter::new(MockVpn::absent());
        assert!(adapter.state().is_unavailable());
        assert_eq!(adapter.connection(), ConnectionState::Absent);
        assert_eq!(adapter.subscriptions(), 0);
        assert!(!adapter.state().slot(AdapterId::VPN).visible);
        assert_eq!(
            <VpnAdapter<MockVpn> as Adapter>::id(&adapter),
            AdapterId::VPN
        );
    }

    #[test]
    fn refresh_reads_once_and_subscribes() {
        let mut adapter = VpnAdapter::new(MockVpn::present(data()));
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
        assert!(adapter.snapshot().unwrap().present());
    }

    #[test]
    fn a_kill_hides_and_a_restart_resubscribes() {
        let mut adapter = VpnAdapter::new(MockVpn::present(data()));
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
    fn a_present_but_unreadable_daemon_is_visible_and_inert() {
        let mut adapter = VpnAdapter::new(MockVpn::failing("NetworkManager: no reply"));
        adapter.refresh();
        assert!(adapter.state().is_error());
        let slot = adapter.state().slot(AdapterId::VPN);
        assert!(slot.visible && !slot.enabled);
        assert_eq!(slot.error.as_deref(), Some("NetworkManager: no reply"));
    }

    #[test]
    fn a_connect_round_trips_through_a_re_read() {
        let mut adapter = VpnAdapter::new(MockVpn::present(data()));
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().connected_count(), 0);

        let result = adapter.connect("aaa");
        assert_eq!(result, VpnResult::Accepted);
        assert_eq!(adapter.source().activations(), 1);
        // The write itself changes nothing: the daemon pushes, the host reads.
        assert_eq!(adapter.snapshot().unwrap().connected_count(), 0);

        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().connected_count(), 1);
        assert_eq!(adapter.snapshot().unwrap().active_uuid(), Some("aaa"));
        assert_eq!(adapter.access(), &VpnAccess::ReadWrite);
    }

    #[test]
    fn a_disconnect_round_trips_through_a_re_read() {
        let mut adapter = VpnAdapter::new(MockVpn::present(data()));
        adapter.refresh();
        adapter.connect("aaa");
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().connected_count(), 1);

        assert_eq!(adapter.deactivate("aaa"), VpnResult::Accepted);
        adapter.refresh();
        assert_eq!(adapter.snapshot().unwrap().connected_count(), 0);
        assert_eq!(adapter.source().deactivations(), 1);
    }

    #[test]
    fn a_polkit_denial_degrades_to_read_only_and_records_the_note() {
        let mut adapter = VpnAdapter::new(MockVpn::present(data()));
        adapter.refresh();
        adapter
            .source_mut()
            .deny_writes("not authorized to connect");

        let result = adapter.connect("aaa");
        assert_eq!(
            result,
            VpnResult::Denied {
                note: "not authorized to connect".to_owned()
            }
        );
        assert_eq!(
            adapter.access(),
            &VpnAccess::read_only("not authorized to connect")
        );
        assert!(adapter.is_read_only());
        assert_eq!(
            adapter.degradation_note(),
            Some("not authorized to connect")
        );

        // Once read-only, a second write is refused locally, without touching
        // the daemon again, but reads stay live.
        assert!(matches!(adapter.connect("aaa"), VpnResult::Denied { .. }));
        assert_eq!(adapter.source().activations(), 1);
        assert!(adapter.state().is_available());
        assert!(adapter.state().is_visible());
        assert!(adapter.state().is_enabled());
    }

    #[test]
    fn a_write_failure_other_than_denial_does_not_degrade() {
        let mut adapter = VpnAdapter::new(MockVpn::present(data()));
        adapter.refresh();
        adapter.source_mut().fail_writes("unknown connection");

        assert_eq!(
            adapter.connect("ghost"),
            VpnResult::Failed(AdapterError::new("unknown connection"))
        );
        assert!(!adapter.is_read_only());
        assert_eq!(adapter.access(), &VpnAccess::ReadWrite);
    }

    #[test]
    fn a_write_while_absent_reports_absence_and_hides() {
        let mut adapter = VpnAdapter::new(MockVpn::absent());
        assert_eq!(adapter.connect("aaa"), VpnResult::Absent);
        assert!(adapter.state().is_unavailable());
        assert!(!adapter.state().slot(AdapterId::VPN).visible);
    }
}
