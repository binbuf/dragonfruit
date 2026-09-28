// SPDX-License-Identifier: MIT
//! The VPN snapshot the pane and tile render, decoded from one raw read.
//!
//! The model owns the NetworkManager enum mapping and the join a consumer
//! should not repeat: it joins each configured connection to the active
//! connection made from it, types the VPN flavour, and orders connected
//! connections first. Nothing here reimplements a VPN stack; the raw strings
//! and numbers come straight from the daemon ([`crate::VpnData`]).

use crate::vpn::source::{ActiveVpnData, VpnConnectionData, VpnData};

/// The kind of VPN tunnel a connection is, decoded from its NetworkManager
/// `connection.type` and `vpn.service-type`.
///
/// NetworkManager carries OpenVPN, OpenConnect, IPsec and friends under the
/// generic `vpn` type with a `service-type` discriminant, and WireGuard under
/// its own `wireguard` type. The model names the common services and falls
/// back to a generic [`VpnKind::Vpn`] rather than guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VpnKind {
    /// A generic `vpn` connection whose service is not one of the named ones.
    #[default]
    Vpn,
    /// `org.freedesktop.NetworkManager.openvpn`.
    OpenVpn,
    /// `org.freedesktop.NetworkManager.openconnect`.
    OpenConnect,
    /// IPsec (`libreswan`/`strongswan`/`openswan`).
    Ipsec,
    /// PPTP.
    Pptp,
    /// L2TP.
    L2tp,
    /// WireGuard (its own `wireguard` connection type).
    WireGuard,
}

impl VpnKind {
    /// Decode the connection type and optional service type.
    pub fn from_connection(kind: &str, service_type: Option<&str>) -> Self {
        if kind == "wireguard" {
            return VpnKind::WireGuard;
        }
        let service = service_type.unwrap_or_default();
        if service.ends_with(".openvpn") {
            VpnKind::OpenVpn
        } else if service.ends_with(".openconnect") {
            VpnKind::OpenConnect
        } else if service.ends_with(".libreswan")
            || service.ends_with(".strongswan")
            || service.ends_with(".openswan")
        {
            VpnKind::Ipsec
        } else if service.ends_with(".pptp") {
            VpnKind::Pptp
        } else if service.ends_with(".l2tp") {
            VpnKind::L2tp
        } else {
            VpnKind::Vpn
        }
    }

    /// A short label for the connection row.
    pub const fn label(self) -> &'static str {
        match self {
            VpnKind::Vpn => "VPN",
            VpnKind::OpenVpn => "OpenVPN",
            VpnKind::OpenConnect => "OpenConnect",
            VpnKind::Ipsec => "IPsec",
            VpnKind::Pptp => "PPTP",
            VpnKind::L2tp => "L2TP",
            VpnKind::WireGuard => "WireGuard",
        }
    }
}

/// The live state of one VPN connection, mapped from the active connection's
/// `NMActiveConnectionState`.
///
/// A configured connection with no active-connection object is
/// [`VpnState::Disconnected`]; the intermediate NetworkManager values
/// collapse the same way the Wi-Fi device states do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VpnState {
    /// No active connection (or it was deactivated).
    #[default]
    Disconnected,
    /// `NM_ACTIVE_CONNECTION_STATE_ACTIVATING`.
    Connecting,
    /// `NM_ACTIVE_CONNECTION_STATE_ACTIVATED`.
    Connected,
    /// `NM_ACTIVE_CONNECTION_STATE_DEACTIVATING`.
    Disconnecting,
    /// An active-connection state we do not recognise.
    Unknown,
}

impl VpnState {
    /// Map an optional `NMActiveConnectionState`.
    ///
    /// `None` (no active-connection object), the `0` (`UNKNOWN`) placeholder,
    /// and the terminal `4` (`DEACTIVATED`) all mean disconnected.
    pub const fn from_active(state: Option<u32>) -> Self {
        match state {
            None | Some(0) | Some(4) => VpnState::Disconnected,
            Some(1) => VpnState::Connecting,
            Some(2) => VpnState::Connected,
            Some(3) => VpnState::Disconnecting,
            Some(_) => VpnState::Unknown,
        }
    }

    /// Whether the tunnel is up.
    pub const fn is_connected(self) -> bool {
        matches!(self, VpnState::Connected)
    }

    /// Whether an active connection exists in any non-terminal state.
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            VpnState::Connecting | VpnState::Connected | VpnState::Disconnecting
        )
    }

    /// A stable label for the connection row.
    pub const fn label(self) -> &'static str {
        match self {
            VpnState::Disconnected => "Disconnected",
            VpnState::Connecting => "Connecting…",
            VpnState::Connected => "Connected",
            VpnState::Disconnecting => "Disconnecting…",
            VpnState::Unknown => "Unknown",
        }
    }

    /// How "up" the state is, so the list sorts connected-first: a connected
    /// tunnel wins over a connecting one, which wins over idle.
    pub(crate) const fn rank(self) -> u8 {
        match self {
            VpnState::Connected => 3,
            VpnState::Connecting | VpnState::Disconnecting => 2,
            VpnState::Unknown => 1,
            VpnState::Disconnected => 0,
        }
    }
}

/// One VPN connection the pane lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnConnection {
    /// The user-facing name (`connection.id`).
    pub id: String,
    /// The stable identity (`connection.uuid`).
    pub uuid: String,
    /// The decoded tunnel flavour.
    pub kind: VpnKind,
    /// The live state.
    pub state: VpnState,
    /// Whether NetworkManager is configured to bring it up on its own.
    pub autoconnect: bool,
}

impl VpnConnection {
    /// Whether this connection is currently up.
    pub const fn is_connected(&self) -> bool {
        self.state.is_connected()
    }

    /// A short secondary label, e.g. `OpenVPN · Connected`.
    pub fn label(&self) -> String {
        format!("{} · {}", self.kind.label(), self.state.label())
    }
}

/// The VPN snapshot a pane or tile renders: every configured VPN connection
/// with its live state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VpnSnapshot {
    /// One entry per configured VPN, connected first then by name.
    pub connections: Vec<VpnConnection>,
}

impl VpnSnapshot {
    /// Build the snapshot from one raw read.
    ///
    /// Each configured connection is joined to the active connection whose
    /// settings path (or UUID, for a daemon that omits the path) matches it;
    /// no match means disconnected. The list is ordered connected-first, then
    /// by name, then UUID for determinism.
    pub fn from_data(data: &VpnData) -> Self {
        let mut connections: Vec<VpnConnection> = data
            .connections
            .iter()
            .map(|connection| VpnConnection {
                id: connection.id.clone(),
                uuid: connection.uuid.clone(),
                kind: VpnKind::from_connection(
                    &connection.kind,
                    connection.service_type.as_deref(),
                ),
                state: active_state(&data.active, connection),
                autoconnect: connection.autoconnect,
            })
            .collect();

        connections.sort_by(|left, right| {
            right
                .state
                .rank()
                .cmp(&left.state.rank())
                .then_with(|| left.id.cmp(&right.id))
                .then_with(|| left.uuid.cmp(&right.uuid))
        });

        VpnSnapshot { connections }
    }

    /// Whether any VPN is configured. A consumer that shows only an active
    /// status hides the item when this is false.
    pub fn present(&self) -> bool {
        !self.connections.is_empty()
    }

    /// The number of configured connections.
    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }

    /// The number of connections currently up.
    pub fn connected_count(&self) -> usize {
        self.connections
            .iter()
            .filter(|connection| connection.is_connected())
            .count()
    }

    /// The UUID of the first connected connection, if any.
    pub fn active_uuid(&self) -> Option<&str> {
        self.connections
            .iter()
            .find(|connection| connection.is_connected())
            .map(|connection| connection.uuid.as_str())
    }

    /// The name of the first connected connection, if any.
    pub fn active_name(&self) -> Option<&str> {
        self.connections
            .iter()
            .find(|connection| connection.is_connected())
            .map(|connection| connection.id.as_str())
    }

    /// The glyph the pane or tile draws for the current state.
    pub fn glyph(&self) -> &'static str {
        if self.connected_count() > 0 {
            "vpn"
        } else {
            "vpn-off"
        }
    }

    /// A one-line label for the tile.
    pub fn label(&self) -> String {
        if !self.present() {
            return "No VPN".to_owned();
        }
        match self.connected_count() {
            0 => "Not Connected".to_owned(),
            1 => self
                .active_name()
                .map(str::to_owned)
                .unwrap_or_else(|| "Connected".to_owned()),
            count => format!("{count} Connected"),
        }
    }
}

/// The state of `connection` from the active list: match on the settings path,
/// falling back to UUID for a daemon that omits it.
fn active_state(active: &[ActiveVpnData], connection: &VpnConnectionData) -> VpnState {
    let matched = active
        .iter()
        .find(|active| active.connection == connection.path || active.uuid == connection.uuid);
    VpnState::from_active(matched.map(|active| active.state))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection(id: &str, uuid: &str, kind: &str, service: Option<&str>) -> VpnConnectionData {
        VpnConnectionData {
            path: format!("/org/freedesktop/NetworkManager/Settings/{uuid}"),
            id: id.to_owned(),
            uuid: uuid.to_owned(),
            kind: kind.to_owned(),
            service_type: service.map(str::to_owned),
            autoconnect: false,
        }
    }

    fn active(connection: &VpnConnectionData, state: u32) -> ActiveVpnData {
        ActiveVpnData {
            path: format!(
                "/org/freedesktop/NetworkManager/ActiveConnection/{}",
                connection.uuid
            ),
            connection: connection.path.clone(),
            id: connection.id.clone(),
            uuid: connection.uuid.clone(),
            state,
            vpn: true,
        }
    }

    #[test]
    fn kinds_decode_type_and_service() {
        assert_eq!(
            VpnKind::from_connection("vpn", Some("org.freedesktop.NetworkManager.openvpn")),
            VpnKind::OpenVpn
        );
        assert_eq!(
            VpnKind::from_connection("vpn", Some("org.freedesktop.NetworkManager.openconnect")),
            VpnKind::OpenConnect
        );
        assert_eq!(
            VpnKind::from_connection("vpn", Some("org.freedesktop.NetworkManager.libreswan")),
            VpnKind::Ipsec
        );
        assert_eq!(
            VpnKind::from_connection("vpn", Some("org.freedesktop.NetworkManager.strongswan")),
            VpnKind::Ipsec
        );
        assert_eq!(
            VpnKind::from_connection("vpn", Some("org.freedesktop.NetworkManager.pptp")),
            VpnKind::Pptp
        );
        assert_eq!(
            VpnKind::from_connection("wireguard", None),
            VpnKind::WireGuard
        );
        assert_eq!(VpnKind::from_connection("vpn", None), VpnKind::Vpn);
        assert_eq!(
            VpnKind::from_connection("vpn", Some("org.example.custom")),
            VpnKind::Vpn
        );
    }

    #[test]
    fn states_map_the_active_connection_enum() {
        assert_eq!(VpnState::from_active(None), VpnState::Disconnected);
        assert_eq!(VpnState::from_active(Some(0)), VpnState::Disconnected);
        assert_eq!(VpnState::from_active(Some(1)), VpnState::Connecting);
        assert_eq!(VpnState::from_active(Some(2)), VpnState::Connected);
        assert_eq!(VpnState::from_active(Some(3)), VpnState::Disconnecting);
        assert_eq!(VpnState::from_active(Some(4)), VpnState::Disconnected);
        assert_eq!(VpnState::from_active(Some(9)), VpnState::Unknown);
    }

    #[test]
    fn configured_connections_without_an_active_entry_are_disconnected() {
        let data = VpnData {
            connections: vec![connection(
                "Work",
                "aaa",
                "vpn",
                Some("org.freedesktop.NetworkManager.openvpn"),
            )],
            active: Vec::new(),
        };
        let snapshot = VpnSnapshot::from_data(&data);
        assert!(snapshot.present());
        assert_eq!(snapshot.connection_count(), 1);
        assert_eq!(snapshot.connected_count(), 0);
        assert_eq!(snapshot.connections[0].state, VpnState::Disconnected);
        assert_eq!(snapshot.connections[0].kind, VpnKind::OpenVpn);
        assert_eq!(snapshot.label(), "Not Connected");
        assert_eq!(snapshot.glyph(), "vpn-off");
    }

    #[test]
    fn an_active_connection_marks_its_configured_connection() {
        let work = connection(
            "Work",
            "aaa",
            "vpn",
            Some("org.freedesktop.NetworkManager.openvpn"),
        );
        let data = VpnData {
            active: vec![active(&work, 2)],
            connections: vec![work],
        };
        let snapshot = VpnSnapshot::from_data(&data);
        assert_eq!(snapshot.connected_count(), 1);
        assert_eq!(snapshot.active_uuid(), Some("aaa"));
        assert_eq!(snapshot.active_name(), Some("Work"));
        assert_eq!(snapshot.label(), "Work");
        assert_eq!(snapshot.glyph(), "vpn");
        assert_eq!(snapshot.connections[0].label(), "OpenVPN · Connected");
    }

    #[test]
    fn the_uuid_fallback_matches_when_the_daemon_omits_the_path() {
        let work = connection("Work", "aaa", "wireguard", None);
        let mut active = active(&work, 2);
        active.connection = String::new();
        let data = VpnData {
            connections: vec![work],
            active: vec![active],
        };
        let snapshot = VpnSnapshot::from_data(&data);
        assert_eq!(snapshot.connected_count(), 1);
        assert_eq!(snapshot.connections[0].kind, VpnKind::WireGuard);
    }

    #[test]
    fn connected_connections_sort_first() {
        let work = connection("Work", "aaa", "vpn", None);
        let home = connection("Home", "bbb", "wireguard", None);
        let data = VpnData {
            connections: vec![work.clone(), home.clone()],
            active: vec![active(&home, 2)],
        };
        let snapshot = VpnSnapshot::from_data(&data);
        assert_eq!(snapshot.connections[0].uuid, "bbb");
        assert_eq!(snapshot.connections[1].uuid, "aaa");
    }

    #[test]
    fn an_empty_store_is_not_present() {
        let snapshot = VpnSnapshot::from_data(&VpnData::default());
        assert!(!snapshot.present());
        assert_eq!(snapshot.label(), "No VPN");
    }

    #[test]
    fn several_connected_connections_report_a_count() {
        let work = connection("Work", "aaa", "vpn", None);
        let home = connection("Home", "bbb", "vpn", None);
        let data = VpnData {
            connections: vec![work.clone(), home.clone()],
            active: vec![active(&work, 2), active(&home, 2)],
        };
        let snapshot = VpnSnapshot::from_data(&data);
        assert_eq!(snapshot.connected_count(), 2);
        assert_eq!(snapshot.label(), "2 Connected");
    }
}
