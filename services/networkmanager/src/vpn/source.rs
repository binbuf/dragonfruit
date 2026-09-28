// SPDX-License-Identifier: MIT
//! The transport seam: the raw NetworkManager VPN read and its mock.
//!
//! A [`VpnSource`] is the only thing that talks to the daemon on the VPN path.
//! The real source is the D-Bus client ([`crate::DbusVpn`]); tests and CI use
//! [`MockVpn`], which serves a fixture with no bus on the machine. The adapter
//! ([`crate::VpnAdapter`]) turns one raw read into the typed snapshot and
//! drives the shared `Subscription`.
//!
//! The raw shape deliberately mirrors NetworkManager's two object families:
//! the configured connections (the `Settings.Connection` objects, each a
//! `a{sa{sv}}` settings map) and the live active connections
//! (`Connection.Active` objects, each with a state and a VPN flag). The model
//! joins them by connection path. Decoding stops here: service types stay the
//! daemon's strings, active states stay the numeric NetworkManager enums.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// The result of one NetworkManager VPN read.
///
/// This is deliberately flat and daemon-shaped: the D-Bus source fills it from
/// the settings and active-connection objects, a fixture deserialises straight
/// into it, and nothing above the adapter ever sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct VpnData {
    /// Every configured connection whose type is a VPN (`vpn` or
    /// `wireguard`); non-VPN connections are filtered out at the source.
    #[serde(default)]
    pub connections: Vec<VpnConnectionData>,
    /// Every active connection the daemon currently reports; the model keeps
    /// the ones that match a configured VPN.
    #[serde(default)]
    pub active: Vec<ActiveVpnData>,
}

/// One configured VPN connection (`org.freedesktop.NetworkManager.Settings.
/// Connection`, decoded from its `GetSettings` map plus its object path).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct VpnConnectionData {
    /// The settings object path; the model matches active connections to it.
    pub path: String,
    /// `connection.id` — the user-facing name.
    pub id: String,
    /// `connection.uuid`.
    pub uuid: String,
    /// `connection.type`: `vpn` or `wireguard`.
    pub kind: String,
    /// `vpn.service-type` (e.g. `org.freedesktop.NetworkManager.openvpn`);
    /// `None` for WireGuard, whose type carries no separate service.
    #[serde(default)]
    pub service_type: Option<String>,
    /// `connection.autoconnect`.
    #[serde(default)]
    pub autoconnect: bool,
}

/// One live active connection (`org.freedesktop.NetworkManager.Connection.
/// Active`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct ActiveVpnData {
    /// The active-connection object path.
    pub path: String,
    /// The settings connection this active connection was made from.
    pub connection: String,
    /// `Id`, the user-facing name.
    #[serde(default)]
    pub id: String,
    /// `Uuid`.
    pub uuid: String,
    /// `State` (`NMActiveConnectionState`).
    pub state: u32,
    /// `Vpn`: whether this is a VPN tunnel.
    #[serde(default)]
    pub vpn: bool,
}

/// A request to connect or disconnect one configured VPN by UUID.
///
/// The UUID is the stable identity: the daemon's object paths can change when
/// a connection is edited, and the pane only ever holds the UUID it read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnRequest {
    /// The `connection.uuid` to act on.
    pub uuid: String,
}

impl VpnRequest {
    /// A request for the connection with `uuid`.
    pub fn new(uuid: impl Into<String>) -> Self {
        VpnRequest { uuid: uuid.into() }
    }
}

/// The result of one connect/disconnect request.
///
/// A polkit denial is deliberately distinct from a general failure: it is the
/// signal the adapter degrades to read-only on. It is not a read error — the
/// adapter can still see the connection list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VpnOutcome {
    /// NetworkManager accepted the request; the state change is in progress.
    /// The daemon will push the resulting state.
    Accepted,
    /// polkit refused the request. `note` records why; the adapter degrades to
    /// read-only.
    Denied(String),
    /// The daemon is absent.
    Absent,
    /// The request failed for a reason other than authorization.
    Failed(AdapterError),
}

/// Reads NetworkManager's VPN connections, and connects/disconnects them.
///
/// The read result is a three-way answer, exactly as the adapter contract
/// needs it:
///
/// * `Ok(Some(data))` — the daemon answered; `data` is the live read.
/// * `Ok(None)` — the daemon is absent. A normal state; the slot hides.
/// * `Err(error)` — the daemon is present but the read failed; the slot shows
///   visible and inert with the message.
///
/// The source is never polled by a consumer: the host calls
/// [`VpnAdapter::refresh`](crate::VpnAdapter::refresh) when the daemon signals
/// a change.
pub trait VpnSource {
    /// One read of the daemon's VPN connections and active connections.
    fn read(&mut self) -> Result<Option<VpnData>, AdapterError>;

    /// Connect (activate) the configured VPN with `request.uuid`.
    fn activate(&mut self, request: &VpnRequest) -> VpnOutcome;

    /// Disconnect (deactivate) the active VPN with `request.uuid`.
    fn deactivate(&mut self, request: &VpnRequest) -> VpnOutcome;
}

/// How the simulated daemon answers a VPN write.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteBehavior {
    /// Authorized: the request is accepted (the default).
    Accept,
    /// polkit denies the write; carries the recorded note.
    Deny(String),
    /// Present but the write fails for another reason.
    Fail(AdapterError),
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`VpnData`] with no bus, and
/// `kill`/`restart` exercise absence and re-subscribe the way masking the real
/// daemon would. Unlike the Wi-Fi mock, an accepted write mutates the
/// simulated store (a connect marks its active connection activated, a
/// disconnect removes it), so the adapter's "write, then re-read the pushed
/// state" round-trip is observable headlessly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockVpn {
    present: bool,
    data: VpnData,
    failure: Option<AdapterError>,
    behavior: WriteBehavior,
    reads: u32,
    activations: u32,
    deactivations: u32,
}

impl MockVpn {
    /// A daemon that is not on the bus.
    pub fn absent() -> Self {
        MockVpn {
            present: false,
            data: VpnData::default(),
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            activations: 0,
            deactivations: 0,
        }
    }

    /// A present daemon that answers with `data`.
    pub fn present(data: VpnData) -> Self {
        MockVpn {
            present: true,
            data,
            failure: None,
            behavior: WriteBehavior::Accept,
            reads: 0,
            activations: 0,
            deactivations: 0,
        }
    }

    /// A present daemon that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockVpn {
            present: true,
            data: VpnData::default(),
            failure: Some(AdapterError::new(message)),
            behavior: WriteBehavior::Accept,
            reads: 0,
            activations: 0,
            deactivations: 0,
        }
    }

    /// Make every write come back as a polkit denial with `note`.
    pub fn deny_writes(&mut self, note: impl Into<String>) {
        self.behavior = WriteBehavior::Deny(note.into());
    }

    /// Make every write fail with `message` (not authorization).
    pub fn fail_writes(&mut self, message: impl Into<String>) {
        self.behavior = WriteBehavior::Fail(AdapterError::new(message));
    }

    /// Restore the default authorized behavior.
    pub fn allow_writes(&mut self) {
        self.behavior = WriteBehavior::Accept;
    }

    /// The daemon pushes fresh data.
    pub fn push(&mut self, data: VpnData) {
        self.present = true;
        self.failure = None;
        self.data = data;
    }

    /// The daemon goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The daemon comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated daemon is on the bus.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many connect requests the source has served.
    pub fn activations(&self) -> u32 {
        self.activations
    }

    /// How many disconnect requests the source has served.
    pub fn deactivations(&self) -> u32 {
        self.deactivations
    }

    /// The active connection for `uuid`, if the simulated store has one.
    fn active_for(&self, uuid: &str) -> Option<&ActiveVpnData> {
        self.data.active.iter().find(|active| active.uuid == uuid)
    }

    /// The write behavior for a present, readable daemon, or the absence/
    /// failure that stands in for it.
    fn write_outcome(&self) -> VpnOutcome {
        if !self.present {
            return VpnOutcome::Absent;
        }
        // A daemon that cannot answer a read cannot write either.
        if let Some(error) = &self.failure {
            return VpnOutcome::Failed(error.clone());
        }
        match &self.behavior {
            WriteBehavior::Accept => VpnOutcome::Accepted,
            WriteBehavior::Deny(note) => VpnOutcome::Denied(note.clone()),
            WriteBehavior::Fail(error) => VpnOutcome::Failed(error.clone()),
        }
    }
}

impl VpnSource for MockVpn {
    fn read(&mut self) -> Result<Option<VpnData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone()))
    }

    fn activate(&mut self, request: &VpnRequest) -> VpnOutcome {
        self.activations += 1;
        let outcome = self.write_outcome();
        if outcome != VpnOutcome::Accepted {
            return outcome;
        }
        // The connection must exist in the store; otherwise the write fails
        // the way the daemon would reject an unknown UUID.
        let Some(connection) = self
            .data
            .connections
            .iter()
            .find(|connection| connection.uuid == request.uuid)
            .cloned()
        else {
            return VpnOutcome::Failed(AdapterError::new(format!(
                "NetworkManager: unknown VPN connection '{}'",
                request.uuid
            )));
        };
        // Simulate the daemon's push: the connection becomes activated.
        self.data
            .active
            .retain(|active| active.uuid != request.uuid);
        self.data.active.push(ActiveVpnData {
            path: format!(
                "/org/freedesktop/NetworkManager/ActiveConnection/{}",
                self.activations
            ),
            connection: connection.path,
            id: connection.id,
            uuid: connection.uuid,
            state: 2,
            vpn: true,
        });
        VpnOutcome::Accepted
    }

    fn deactivate(&mut self, request: &VpnRequest) -> VpnOutcome {
        self.deactivations += 1;
        let outcome = self.write_outcome();
        if outcome != VpnOutcome::Accepted {
            return outcome;
        }
        if self.active_for(&request.uuid).is_none() {
            return VpnOutcome::Failed(AdapterError::new(format!(
                "NetworkManager: VPN connection '{}' is not active",
                request.uuid
            )));
        }
        self.data
            .active
            .retain(|active| active.uuid != request.uuid);
        VpnOutcome::Accepted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(uuid: &str) -> VpnConnectionData {
        VpnConnectionData {
            path: format!("/org/freedesktop/NetworkManager/Settings/{uuid}"),
            id: "Work VPN".to_owned(),
            uuid: uuid.to_owned(),
            kind: "vpn".to_owned(),
            service_type: Some("org.freedesktop.NetworkManager.openvpn".to_owned()),
            autoconnect: false,
        }
    }

    fn data() -> VpnData {
        VpnData {
            connections: vec![connection("aaa")],
            active: Vec::new(),
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockVpn::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
        assert!(!mock.is_present());
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockVpn::present(data());
        assert_eq!(mock.read().unwrap(), Some(data()));
        assert!(mock.is_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockVpn::failing("NetworkManager: timeout");
        assert_eq!(
            mock.read().unwrap_err().message(),
            "NetworkManager: timeout"
        );
        assert!(mock.is_present());
    }

    #[test]
    fn the_daemon_can_be_killed_and_restarted() {
        let mut mock = MockVpn::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn an_accepted_connect_marks_the_connection_active() {
        let mut mock = MockVpn::present(data());
        assert_eq!(mock.activate(&VpnRequest::new("aaa")), VpnOutcome::Accepted);
        assert_eq!(mock.activations(), 1);
        let reread = mock.read().unwrap().unwrap();
        assert_eq!(reread.active.len(), 1);
        assert_eq!(reread.active[0].uuid, "aaa");
        assert_eq!(reread.active[0].state, 2);
        assert!(reread.active[0].vpn);
    }

    #[test]
    fn an_accepted_disconnect_removes_the_active_connection() {
        let mut mock = MockVpn::present(data());
        mock.activate(&VpnRequest::new("aaa"));
        assert_eq!(
            mock.deactivate(&VpnRequest::new("aaa")),
            VpnOutcome::Accepted
        );
        assert_eq!(mock.deactivations(), 1);
        assert!(mock.read().unwrap().unwrap().active.is_empty());
    }

    #[test]
    fn an_unknown_uuid_connect_is_a_failure() {
        let mut mock = MockVpn::present(data());
        let outcome = mock.activate(&VpnRequest::new("missing"));
        assert!(matches!(outcome, VpnOutcome::Failed(_)));
    }

    #[test]
    fn a_disconnect_of_an_inactive_connection_is_a_failure() {
        let mut mock = MockVpn::present(data());
        assert!(matches!(
            mock.deactivate(&VpnRequest::new("aaa")),
            VpnOutcome::Failed(_)
        ));
    }

    #[test]
    fn a_denying_mock_reports_the_note() {
        let mut mock = MockVpn::present(data());
        mock.deny_writes("NetworkManager: not authorized");
        assert_eq!(
            mock.activate(&VpnRequest::new("aaa")),
            VpnOutcome::Denied("NetworkManager: not authorized".to_owned())
        );
    }

    #[test]
    fn a_failing_write_is_a_failure_not_a_denial() {
        let mut mock = MockVpn::present(data());
        mock.fail_writes("NetworkManager: internal error");
        assert_eq!(
            mock.activate(&VpnRequest::new("aaa")),
            VpnOutcome::Failed(AdapterError::new("NetworkManager: internal error"))
        );
    }

    #[test]
    fn an_absent_mock_write_is_absent() {
        let mut mock = MockVpn::absent();
        assert_eq!(mock.activate(&VpnRequest::new("aaa")), VpnOutcome::Absent);
        assert_eq!(mock.deactivate(&VpnRequest::new("aaa")), VpnOutcome::Absent);
    }
}
