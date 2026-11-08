// SPDX-License-Identifier: MIT
//! The transport seam: the raw NetworkManager read and its mock.
//!
//! A [`NetworkManagerSource`] is the only thing that talks to the daemon. The
//! real source is the D-Bus client ([`crate::DbusNetworkManager`]); tests and
//! CI use [`MockNetworkManager`], which serves a fixture with no bus on the
//! machine. The adapter ([`crate::NetworkManagerAdapter`]) turns one raw read
//! into the typed snapshot and drives the shared `Subscription`.
//!
//! Decoding stops here: an SSID is already lossy-decoded to text and device/AP
//! states stay the numeric NetworkManager enums, which [`crate::model`] maps.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// The result of one NetworkManager read, shaped after its D-Bus objects.
///
/// This is deliberately flat and daemon-shaped: the D-Bus source fills it from
/// proxies, a fixture deserialises straight into it, and nothing above the
/// adapter ever sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct NetworkManagerData {
    /// `org.freedesktop.NetworkManager.WirelessEnabled`.
    pub wireless_enabled: bool,
    /// `org.freedesktop.NetworkManager.Connectivity` (`NMConnectivityState`).
    pub connectivity: u32,
    /// Every device NetworkManager currently manages, all types; the model
    /// keeps only the Wi-Fi ones.
    #[serde(default)]
    pub devices: Vec<WifiDeviceData>,
    /// Every configured connection (`Settings.Connection`), all types. The
    /// Settings Network pane lists these as services and known networks; the
    /// menu-bar Wi-Fi snapshot keeps only the Wi-Fi half.
    #[serde(default)]
    pub connections: Vec<ConnectionData>,
    /// Every live active connection (`Connection.Active`), all types. The
    /// model joins a configured connection to its active entry by UUID.
    #[serde(default)]
    pub active: Vec<ActiveConnectionData>,
    /// Every managed device (`Device`), all types. The Network pane uses the
    /// interface, hardware address, and IPv4 address; the Wi-Fi snapshot
    /// ignores it.
    #[serde(default)]
    pub services: Vec<ServiceData>,
}

/// One configured NetworkManager connection (`Settings.Connection`).
///
/// The shape is deliberately flat: the D-Bus source fills it from one
/// `GetSettings` map, a fixture deserialises straight into it, and nothing
/// above the adapter sees the `a{sa{sv}}` wire form.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct ConnectionData {
    /// The settings object path.
    pub path: String,
    /// `connection.id` — the user-facing name.
    pub id: String,
    /// `connection.uuid`.
    pub uuid: String,
    /// `connection.type` (`802-3-ethernet`, `802-11-wireless`, `vpn`, …).
    pub kind: String,
    /// `vpn.service-type`, when the connection is a VPN.
    #[serde(default)]
    pub service_type: Option<String>,
    /// `connection.autoconnect`.
    #[serde(default)]
    pub autoconnect: bool,
    /// `connection.autoconnect-priority`; higher wins (NM's own ordering).
    #[serde(default)]
    pub priority: i32,
    /// `connection.timestamp` — the last successful activation, in seconds.
    #[serde(default)]
    pub timestamp: u64,
    /// `connection.interface-name`, when pinned.
    #[serde(default)]
    pub interface_name: Option<String>,
    /// `802-11-wireless.ssid`, decoded to text for Wi-Fi connections.
    #[serde(default)]
    pub ssid: Option<String>,
}

/// One live NetworkManager active connection (`Connection.Active`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct ActiveConnectionData {
    /// The active-connection object path.
    pub path: String,
    /// The settings connection this active connection was made from.
    pub connection: String,
    /// `Uuid`.
    pub uuid: String,
    /// `Id`.
    #[serde(default)]
    pub id: String,
    /// `State` (`NMActiveConnectionState`).
    pub state: u32,
    /// `Vpn`.
    #[serde(default)]
    pub vpn: bool,
    /// `Type` (the connection type).
    #[serde(default)]
    pub connection_type: String,
    /// `Devices`, the device object paths this connection is using.
    #[serde(default)]
    pub devices: Vec<String>,
}

/// One managed device (`org.freedesktop.NetworkManager.Device`), all types.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct ServiceData {
    /// The device object path.
    pub path: String,
    /// `DeviceType` (`NMDeviceType`).
    pub kind: u32,
    /// `Interface`, the kernel interface name.
    #[serde(default)]
    pub interface: String,
    /// `State` (`NMDeviceState`).
    pub state: u32,
    /// `Managed`.
    #[serde(default)]
    pub managed: bool,
    /// `ActiveConnection`, `None` when idle.
    #[serde(default)]
    pub active_connection: Option<String>,
    /// `HwAddress`.
    #[serde(default)]
    pub hw_address: Option<String>,
    /// The first IPv4 address, when the device has one.
    #[serde(default)]
    pub ip4_address: Option<String>,
}

/// One NetworkManager device (`org.freedesktop.NetworkManager.Device` plus its
/// `Device.Wireless` half).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct WifiDeviceData {
    /// `Managed`: an unmanaged device is present but not ours to report.
    pub managed: bool,
    /// `State` (`NMDeviceState`).
    pub state: u32,
    /// `Device.Wireless.ActiveAccessPoint`, `None` when disconnected.
    #[serde(default)]
    pub active_ap: Option<String>,
    /// `Device.Wireless.AccessPoints`, every BSSID the device can see.
    #[serde(default)]
    pub access_points: Vec<AccessPointData>,
}

/// One access point (`org.freedesktop.NetworkManager.AccessPoint`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct AccessPointData {
    /// The AP's object path; the active AP is matched by path, because two
    /// BSSIDs of the same SSID are still different objects.
    pub path: String,
    /// `Ssid`, decoded from its `ay` bytes.
    pub ssid: String,
    /// `Strength`, 0–100.
    pub strength: u8,
    /// `Frequency` in MHz.
    pub frequency_mhz: u32,
    /// `Flags` (bit 0 is `PRIVACY`).
    #[serde(default)]
    pub flags: u32,
    /// `WpaFlags` (`NM80211ApSecurityFlags`).
    #[serde(default)]
    pub wpa_flags: u32,
    /// `RsnFlags` (`NM80211ApSecurityFlags`).
    #[serde(default)]
    pub rsn_flags: u32,
}

/// A request to join (activate) a network.
///
/// The secret lives only for the duration of one [`NetworkManagerSource::activate`]
/// call: the adapter never caches it, and [`Debug`](std::fmt::Debug) redacts it
/// so it cannot leak through a log line.
#[derive(Clone, PartialEq, Eq)]
pub struct ActivateRequest {
    /// The SSID to join.
    pub ssid: String,
    /// The pre-shared key for a secured network; `None` for an open network or
    /// when a secret agent owns the credential.
    pub secret: Option<String>,
    /// A specific BSSID object path to target, when the caller already knows
    /// it; `None` lets the source pick the strongest matching BSSID.
    pub access_point: Option<String>,
    /// A hidden network: the SSID is not advertised, so the source creates the
    /// connection directly instead of resolving an access point.
    pub hidden: bool,
}

impl std::fmt::Debug for ActivateRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActivateRequest")
            .field("ssid", &self.ssid)
            .field("secret", &self.secret.as_ref().map(|_| "[redacted]"))
            .field("access_point", &self.access_point)
            .field("hidden", &self.hidden)
            .finish()
    }
}

/// The result of one activate/join request.
///
/// A polkit denial is deliberately distinct from a general failure: it is the
/// signal the adapter degrades to read-only on. It is not a read error — the
/// adapter can still see the network list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivateOutcome {
    /// NetworkManager accepted the request; activation is in progress. The
    /// daemon will push the resulting state change.
    Accepted,
    /// polkit refused the request. `note` records why; the adapter degrades to
    /// read-only.
    Denied(String),
    /// The daemon is absent.
    Absent,
    /// The request failed for a reason other than authorization.
    Failed(AdapterError),
}

/// Reads NetworkManager over some transport, and joins a network.
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
/// [`NetworkManagerAdapter::refresh`](crate::NetworkManagerAdapter::refresh)
/// when the daemon signals a change.
pub trait NetworkManagerSource {
    /// One read of the daemon.
    fn read(&mut self) -> Result<Option<NetworkManagerData>, AdapterError>;

    /// Join (activate) a network. The one write the source makes.
    ///
    /// This is an explicit user action, never a poll. A polkit denial comes
    /// back as [`ActivateOutcome::Denied`] with the recorded note; the adapter
    /// turns that into a read-only degradation. Any other failure is
    /// [`ActivateOutcome::Failed`].
    fn activate(&mut self, request: &ActivateRequest) -> ActivateOutcome;

    /// Enable or disable the radio (`WirelessEnabled`). One explicit write.
    fn set_wireless_enabled(&mut self, _enabled: bool) -> WriteOutcome {
        WriteOutcome::Unsupported
    }

    /// Delete a saved connection by UUID. One explicit write.
    fn forget_connection(&mut self, _uuid: &str) -> WriteOutcome {
        WriteOutcome::Unsupported
    }

    /// Activate a saved connection by UUID. One explicit write.
    fn activate_connection(&mut self, _uuid: &str) -> WriteOutcome {
        WriteOutcome::Unsupported
    }

    /// Deactivate the active connection made from `uuid`. One explicit write.
    fn deactivate_connection(&mut self, _uuid: &str) -> WriteOutcome {
        WriteOutcome::Unsupported
    }

    /// Change one saved connection property (autoconnect or priority). One
    /// explicit write; the source reads the full settings map, changes the one
    /// key, and writes it back through `Update`.
    fn update_connection(&mut self, _uuid: &str, _property: ConnectionProperty) -> WriteOutcome {
        WriteOutcome::Unsupported
    }

    /// Create a saved connection (the Network pane's Add service/Add VPN).
    fn add_connection(&mut self, _spec: &AddConnectionSpec) -> WriteOutcome {
        WriteOutcome::Unsupported
    }
}

/// The result of one non-join NetworkManager write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteOutcome {
    /// The daemon accepted the write; the host re-reads.
    Applied,
    /// polkit refused the write. `note` records why; the adapter degrades to
    /// read-only.
    Denied(String),
    /// The daemon is absent.
    Absent,
    /// The request failed for a reason other than authorization.
    Failed(AdapterError),
    /// The source does not implement this write (a fixture or an old daemon).
    Unsupported,
}

/// One saved-connection property the Network pane can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionProperty {
    /// `connection.autoconnect`.
    Autoconnect(bool),
    /// `connection.autoconnect-priority`; higher wins.
    Priority(i32),
}

/// The kind of connection the Add service sheet creates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddKind {
    /// A wired `802-3-ethernet` connection.
    #[default]
    Ethernet,
    /// An `802-11-wireless` connection (the SSID is the service name).
    Wifi,
    /// A `vpn` connection using `plugin`.
    Vpn,
}

impl AddKind {
    /// The stable id the pane sends over D-Bus.
    pub const fn id(self) -> &'static str {
        match self {
            AddKind::Ethernet => "ethernet",
            AddKind::Wifi => "wifi",
            AddKind::Vpn => "vpn",
        }
    }

    /// Parse the pane's stable id; an unknown id is Ethernet.
    pub fn from_id(id: &str) -> AddKind {
        match id {
            "wifi" => AddKind::Wifi,
            "vpn" => AddKind::Vpn,
            _ => AddKind::Ethernet,
        }
    }
}

/// One Add service/Add VPN request.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct AddConnectionSpec {
    /// The user-facing connection name (and Wi-Fi SSID).
    pub name: String,
    /// The connection kind.
    pub kind: AddKind,
    /// The VPN plugin service type (`org.freedesktop.NetworkManager.openvpn`)
    /// when `kind` is [`AddKind::Vpn`].
    pub plugin: Option<String>,
    /// The VPN gateway/remote, when the plugin takes one.
    pub gateway: Option<String>,
    /// The pre-shared key for a Wi-Fi connection.
    pub secret: Option<String>,
}

impl std::fmt::Debug for AddConnectionSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AddConnectionSpec")
            .field("name", &self.name)
            .field("kind", &self.kind)
            .field("plugin", &self.plugin)
            .field("gateway", &self.gateway)
            .field("secret", &self.secret.as_ref().map(|_| "[redacted]"))
            .finish()
    }
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`NetworkManagerData`] with no bus, and
/// `kill`/`restart` exercise absence and re-subscribe the way masking the real
/// daemon would.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockNetworkManager {
    present: bool,
    data: Option<NetworkManagerData>,
    failure: Option<AdapterError>,
    reads: u32,
    activation: ActivationBehavior,
    activations: u32,
    write: ActivationBehavior,
    writes: u32,
}

/// How the simulated daemon answers an activate/join request.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ActivationBehavior {
    /// Authorized: the request is accepted (the default).
    Accept,
    /// polkit denies the join; carries the recorded note.
    Deny(String),
    /// Present but the request fails for another reason.
    Fail(AdapterError),
}

impl MockNetworkManager {
    /// A daemon that is not on the bus.
    pub fn absent() -> Self {
        MockNetworkManager {
            present: false,
            data: None,
            failure: None,
            reads: 0,
            activation: ActivationBehavior::Accept,
            activations: 0,
            write: ActivationBehavior::Accept,
            writes: 0,
        }
    }

    /// A present daemon that answers with `data`.
    pub fn present(data: NetworkManagerData) -> Self {
        MockNetworkManager {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
            activation: ActivationBehavior::Accept,
            activations: 0,
            write: ActivationBehavior::Accept,
            writes: 0,
        }
    }

    /// A present daemon that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockNetworkManager {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
            activation: ActivationBehavior::Accept,
            activations: 0,
            write: ActivationBehavior::Accept,
            writes: 0,
        }
    }

    /// Make every non-join write come back as a polkit denial with `note`.
    pub fn deny_writes(&mut self, note: impl Into<String>) {
        self.write = ActivationBehavior::Deny(note.into());
    }

    /// Restore the default authorized behavior for non-join writes.
    pub fn allow_writes(&mut self) {
        self.write = ActivationBehavior::Accept;
    }

    /// How many non-join writes the source has served.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    /// The simulated store, when the daemon is present.
    fn store(&mut self) -> Option<&mut NetworkManagerData> {
        if !self.present || self.failure.is_some() {
            return None;
        }
        if self.data.is_none() {
            self.data = Some(NetworkManagerData::default());
        }
        self.data.as_mut()
    }

    /// The common guard for a non-join write.
    fn begin_write(&mut self) -> Option<WriteOutcome> {
        self.writes += 1;
        if !self.present {
            return Some(WriteOutcome::Absent);
        }
        if let Some(error) = &self.failure {
            return Some(WriteOutcome::Failed(error.clone()));
        }
        match &self.write {
            ActivationBehavior::Accept => None,
            ActivationBehavior::Deny(note) => Some(WriteOutcome::Denied(note.clone())),
            ActivationBehavior::Fail(error) => Some(WriteOutcome::Failed(error.clone())),
        }
    }

    /// Mark the Wi-Fi connection for `ssid` active in the simulated store.
    fn simulate_join(&mut self, ssid: &str, hidden: bool) {
        let Some(data) = self.store() else { return };
        if data.devices.is_empty() {
            data.devices.push(WifiDeviceData {
                managed: true,
                state: 30,
                active_ap: None,
                access_points: Vec::new(),
            });
        }
        let device = &mut data.devices[0];
        let ap_path = if let Some(ap) = device
            .access_points
            .iter_mut()
            .find(|ap| ap.ssid == ssid)
        {
            ap.path.clone()
        } else {
            let path = format!("/mock/ap/{ssid}");
            device.access_points.push(AccessPointData {
                path: path.clone(),
                ssid: ssid.to_owned(),
                strength: if hidden { 100 } else { 60 },
                frequency_mhz: 2412,
                flags: 0,
                wpa_flags: 0,
                rsn_flags: 0,
            });
            path
        };
        device.active_ap = Some(ap_path);
        device.state = 100;
        data.wireless_enabled = true;

        let uuid = format!("mock-{ssid}");
        let path = format!("/mock/connection/{uuid}");
        if !data.connections.iter().any(|c| c.uuid == uuid) {
            data.connections.push(ConnectionData {
                path: path.clone(),
                id: ssid.to_owned(),
                uuid: uuid.clone(),
                kind: "802-11-wireless".to_owned(),
                service_type: None,
                autoconnect: true,
                priority: 0,
                timestamp: 0,
                interface_name: None,
                ssid: Some(ssid.to_owned()),
            });
        }
        data.active.retain(|a| a.uuid != uuid);
        data.active.push(ActiveConnectionData {
            path: format!("/mock/active/{uuid}"),
            connection: path,
            uuid,
            id: ssid.to_owned(),
            state: 2,
            vpn: false,
            connection_type: "802-11-wireless".to_owned(),
            devices: vec!["/mock/device/wifi".to_owned()],
        });
    }

    /// Make every join attempt come back as a polkit denial with `note`.
    pub fn deny_joins(&mut self, note: impl Into<String>) {
        self.activation = ActivationBehavior::Deny(note.into());
    }

    /// Make every join attempt fail with `message` (not authorization).
    pub fn fail_joins(&mut self, message: impl Into<String>) {
        self.activation = ActivationBehavior::Fail(AdapterError::new(message));
    }

    /// Restore the default authorized behavior.
    pub fn allow_joins(&mut self) {
        self.activation = ActivationBehavior::Accept;
    }

    /// The daemon pushes fresh data.
    pub fn push(&mut self, data: NetworkManagerData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
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

    /// How many activate/join requests the source has served. Lets a test
    /// prove a join is one explicit write, never a loop.
    pub fn activations(&self) -> u32 {
        self.activations
    }
}

impl NetworkManagerSource for MockNetworkManager {
    fn read(&mut self) -> Result<Option<NetworkManagerData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn activate(&mut self, request: &ActivateRequest) -> ActivateOutcome {
        self.activations += 1;
        if !self.present {
            return ActivateOutcome::Absent;
        }
        // A daemon that cannot answer a read cannot join either.
        if let Some(error) = &self.failure {
            return ActivateOutcome::Failed(error.clone());
        }
        match &self.activation {
            ActivationBehavior::Accept => {
                self.simulate_join(&request.ssid, request.hidden);
                ActivateOutcome::Accepted
            }
            ActivationBehavior::Deny(note) => ActivateOutcome::Denied(note.clone()),
            ActivationBehavior::Fail(error) => ActivateOutcome::Failed(error.clone()),
        }
    }

    fn set_wireless_enabled(&mut self, enabled: bool) -> WriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        if let Some(data) = self.store() {
            data.wireless_enabled = enabled;
            if !enabled {
                for device in &mut data.devices {
                    device.active_ap = None;
                    device.state = 30;
                }
                data.active.retain(|active| active.vpn);
            }
        }
        WriteOutcome::Applied
    }

    fn forget_connection(&mut self, uuid: &str) -> WriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        let Some(data) = self.store() else {
            return WriteOutcome::Absent;
        };
        let before = data.connections.len();
        data.connections.retain(|connection| connection.uuid != uuid);
        data.active.retain(|active| active.uuid != uuid);
        if data.connections.len() == before {
            return WriteOutcome::Failed(AdapterError::new(format!(
                "NetworkManager: unknown connection '{uuid}'"
            )));
        }
        WriteOutcome::Applied
    }

    fn activate_connection(&mut self, uuid: &str) -> WriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        let Some(data) = self.store() else {
            return WriteOutcome::Absent;
        };
        let Some(connection) = data.connections.iter().find(|c| c.uuid == uuid).cloned() else {
            return WriteOutcome::Failed(AdapterError::new(format!(
                "NetworkManager: unknown connection '{uuid}'"
            )));
        };
        data.active.retain(|active| active.uuid != uuid);
        data.active.push(ActiveConnectionData {
            path: format!("/mock/active/{uuid}"),
            connection: connection.path.clone(),
            uuid: connection.uuid.clone(),
            id: connection.id.clone(),
            state: 2,
            vpn: connection.kind == "vpn" || connection.kind == "wireguard",
            connection_type: connection.kind.clone(),
            devices: Vec::new(),
        });
        if connection.kind == "802-11-wireless" {
            if let Some(ssid) = &connection.ssid {
                let ssid = ssid.clone();
                let data = self.store().expect("store present");
                if let Some(device) = data.devices.first_mut() {
                    if let Some(ap) = device.access_points.iter().find(|ap| ap.ssid == ssid) {
                        device.active_ap = Some(ap.path.clone());
                        device.state = 100;
                    }
                }
            }
        }
        WriteOutcome::Applied
    }

    fn deactivate_connection(&mut self, uuid: &str) -> WriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        let Some(data) = self.store() else {
            return WriteOutcome::Absent;
        };
        let before = data.active.len();
        data.active.retain(|active| active.uuid != uuid);
        if data.active.len() == before {
            return WriteOutcome::Failed(AdapterError::new(format!(
                "NetworkManager: connection '{uuid}' is not active"
            )));
        }
        for device in &mut data.devices {
            device.active_ap = None;
            device.state = 30;
        }
        WriteOutcome::Applied
    }

    fn update_connection(&mut self, uuid: &str, property: ConnectionProperty) -> WriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        let Some(data) = self.store() else {
            return WriteOutcome::Absent;
        };
        let Some(connection) = data.connections.iter_mut().find(|c| c.uuid == uuid) else {
            return WriteOutcome::Failed(AdapterError::new(format!(
                "NetworkManager: unknown connection '{uuid}'"
            )));
        };
        match property {
            ConnectionProperty::Autoconnect(enabled) => connection.autoconnect = enabled,
            ConnectionProperty::Priority(priority) => connection.priority = priority,
        }
        WriteOutcome::Applied
    }

    fn add_connection(&mut self, spec: &AddConnectionSpec) -> WriteOutcome {
        if let Some(outcome) = self.begin_write() {
            return outcome;
        }
        let Some(data) = self.store() else {
            return WriteOutcome::Absent;
        };
        let uuid = format!("mock-added-{}", data.connections.len() + 1);
        let kind = match spec.kind {
            AddKind::Ethernet => "802-3-ethernet",
            AddKind::Wifi => "802-11-wireless",
            AddKind::Vpn => "vpn",
        };
        data.connections.push(ConnectionData {
            path: format!("/mock/connection/{uuid}"),
            id: spec.name.clone(),
            uuid,
            kind: kind.to_owned(),
            service_type: spec.plugin.clone(),
            autoconnect: true,
            priority: 0,
            timestamp: 0,
            interface_name: None,
            ssid: (spec.kind == AddKind::Wifi).then(|| spec.name.clone()),
        });
        WriteOutcome::Applied
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockNetworkManager::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockNetworkManager::failing("NetworkManager: timeout");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "NetworkManager: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockNetworkManager::present(NetworkManagerData::default());
        assert_eq!(mock.read().unwrap(), Some(NetworkManagerData::default()));
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(NetworkManagerData::default()));
    }

    fn request(ssid: &str) -> ActivateRequest {
        ActivateRequest {
            ssid: ssid.to_owned(),
            secret: Some("hunter2".to_owned()),
            access_point: None,
            hidden: false,
        }
    }

    #[test]
    fn the_mock_accepts_joins_by_default() {
        let mut mock = MockNetworkManager::present(NetworkManagerData::default());
        assert_eq!(mock.activate(&request("home")), ActivateOutcome::Accepted);
        assert_eq!(mock.activations(), 1);
    }

    #[test]
    fn a_denying_mock_reports_the_note() {
        let mut mock = MockNetworkManager::present(NetworkManagerData::default());
        mock.deny_joins("NetworkManager: not authorized to join");
        assert_eq!(
            mock.activate(&request("home")),
            ActivateOutcome::Denied("NetworkManager: not authorized to join".to_owned())
        );
    }

    #[test]
    fn a_failing_join_is_a_failure_not_a_denial() {
        let mut mock = MockNetworkManager::present(NetworkManagerData::default());
        mock.fail_joins("NetworkManager: unknown network");
        let outcome = mock.activate(&request("home"));
        assert_eq!(
            outcome,
            ActivateOutcome::Failed(AdapterError::new("NetworkManager: unknown network"))
        );
    }

    #[test]
    fn an_absent_mock_join_is_absent() {
        let mut mock = MockNetworkManager::absent();
        assert_eq!(mock.activate(&request("home")), ActivateOutcome::Absent);
    }

    #[test]
    fn a_debug_activated_request_redacts_the_secret() {
        let debug = format!("{:?}", request("home"));
        assert!(debug.contains("home"));
        assert!(!debug.contains("hunter2"));
        assert!(debug.contains("[redacted]"));
    }
}
