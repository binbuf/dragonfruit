// SPDX-License-Identifier: MIT
//! The Network advanced (VPN) half of the bridge host (T-15.15b).
//!
//! The VPN connections are reached by the `dragonfruit-networkmanager` VPN
//! adapter over the system-bus D-Bus API; the shell and the Settings app never
//! link it. This module owns the one projection from the typed
//! [`VpnSnapshot`] to the flat JSON view the two consumers draw, mirroring the
//! other bridge halves ([`crate::accessibility`], [`crate::privacy`]).
//!
//! Two writes ride the same interface: `connect` and `deactivate`, by UUID,
//! one `ActivateConnection`/`DeactivateConnection` each (ADR 0146). A polkit
//! refusal degrades the adapter to `VpnAccess::ReadOnly`; the view then carries
//! `readOnly: true` and the note, reads stay live, and only the write
//! affordances disable. A write never invents a snapshot: the daemon pushes the
//! resulting state and the host re-reads on the next refresh.
//!
//! Absence is layered, exactly as the adapter's is (ADR 0146). The view is
//! `unavailable` only when the system bus is unreachable or no
//! `org.freedesktop.NetworkManager` owns its name; the item hides then. A
//! daemon that answers with no VPN configured is `available` with
//! `present: false` (the pane's empty note and the tile's second hide rule). A
//! daemon that owns its name but cannot be read is `error`, visible and inert
//! with the message.

use dragonfruit_networkmanager::{VpnAdapter, VpnResult, VpnSnapshot, VpnSource};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_VPN: &str = "vpn";

/// The bridge host for the VPN adapter: one read path plus the two writes.
#[derive(Debug, Clone, PartialEq)]
pub struct VpnHost<S> {
    adapter: VpnAdapter<S>,
}

impl<S: VpnSource> VpnHost<S> {
    /// A host over a VPN source.
    pub fn new(source: S) -> Self {
        VpnHost {
            adapter: VpnAdapter::new(source),
        }
    }

    /// Re-read NetworkManager's VPN connections once. Called on startup and
    /// when the Settings pane or the shell opens; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The VPN view the pane and tile render.
    pub fn view(&self) -> Value {
        vpn_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Connect (activate) the VPN with `uuid`. One explicit write; the report
    /// is JSON. The daemon pushes the resulting state and the host re-reads.
    pub fn connect(&mut self, uuid: &str) -> Value {
        vpn_report(&self.adapter.connect(uuid))
    }

    /// Disconnect (deactivate) the active VPN with `uuid`. One explicit write;
    /// the report is JSON.
    pub fn deactivate(&mut self, uuid: &str) -> Value {
        vpn_report(&self.adapter.deactivate(uuid))
    }

    /// The adapter, for tests and introspection.
    pub fn adapter(&self) -> &VpnAdapter<S> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut VpnAdapter<S> {
        &mut self.adapter
    }
}

/// Build the VPN view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the connection
/// list. A daemon that answers with no VPN is still `available` with
/// `present: false`, the pane's empty note and the tile's second hide rule.
pub fn vpn_view<S: VpnSource>(adapter: &VpnAdapter<S>) -> Value {
    let read_only = adapter.is_read_only();
    let note = adapter.degradation_note().unwrap_or_default();
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_VPN, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_VPN,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_VPN, "state": "unavailable" });
    };
    let mut view = vpn_snapshot_view(snapshot);
    view["readOnly"] = json!(read_only);
    view["note"] = json!(note);
    view
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn vpn_snapshot_view(snapshot: &VpnSnapshot) -> Value {
    let connections: Vec<Value> = snapshot
        .connections
        .iter()
        .map(|connection| {
            json!({
                "id": connection.id,
                "uuid": connection.uuid,
                "kind": connection.kind.id(),
                "kindLabel": connection.kind.label(),
                "state": connection.state.id(),
                "stateLabel": connection.state.label(),
                "connected": connection.is_connected(),
                "autoconnect": connection.autoconnect,
                "label": connection.label(),
            })
        })
        .collect();
    json!({
        "kind": KIND_VPN,
        "state": "available",
        "glyph": snapshot.glyph(),
        // `Work VPN` / `Not Connected` / `2 Connected` / `No VPN`; the tile.
        "label": snapshot.label(),
        // The pane's empty note and the tile's second hide rule.
        "present": snapshot.present(),
        "connectionCount": snapshot.connection_count(),
        "connectedCount": snapshot.connected_count(),
        "activeUuid": snapshot.active_uuid(),
        "activeName": snapshot.active_name(),
        "readOnly": false,
        "note": "",
        "connections": connections,
    })
}

/// The JSON report for a [`VpnResult`].
pub fn vpn_report(result: &VpnResult) -> Value {
    match result {
        VpnResult::Accepted => json!({ "outcome": "accepted" }),
        VpnResult::Denied { note } => json!({ "outcome": "denied", "note": note }),
        VpnResult::Absent => json!({ "outcome": "absent" }),
        VpnResult::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_networkmanager::{ActiveVpnData, MockVpn, VpnConnectionData, VpnData};

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

    fn data() -> VpnData {
        let work = connection(
            "Work VPN",
            "aaa",
            "vpn",
            Some("org.freedesktop.NetworkManager.openvpn"),
        );
        let home = connection("Home", "bbb", "wireguard", None);
        VpnData {
            active: vec![ActiveVpnData {
                path: "/org/freedesktop/NetworkManager/ActiveConnection/1".to_owned(),
                connection: work.path.clone(),
                id: work.id.clone(),
                uuid: work.uuid.clone(),
                state: 2,
                vpn: true,
            }],
            connections: vec![work, home],
        }
    }

    #[test]
    fn an_absent_daemon_projects_a_hidden_slot() {
        let mut host = VpnHost::new(MockVpn::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "vpn");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn an_empty_store_is_available_but_not_present() {
        let mut host = VpnHost::new(MockVpn::present(VpnData::default()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], false);
        assert_eq!(view["label"], "No VPN");
        assert_eq!(view["connectionCount"], 0);
        assert_eq!(view["connections"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn the_view_carries_the_connections_and_live_state() {
        let mut host = VpnHost::new(MockVpn::present(data()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["glyph"], "vpn");
        assert_eq!(view["label"], "Work VPN");
        assert_eq!(view["present"], true);
        assert_eq!(view["connectionCount"], 2);
        assert_eq!(view["connectedCount"], 1);
        assert_eq!(view["activeUuid"], "aaa");
        assert_eq!(view["activeName"], "Work VPN");
        assert_eq!(view["readOnly"], false);
        assert_eq!(view["connections"][0]["uuid"], "aaa");
        assert_eq!(view["connections"][0]["kind"], "openvpn");
        assert_eq!(view["connections"][0]["kindLabel"], "OpenVPN");
        assert_eq!(view["connections"][0]["state"], "connected");
        assert_eq!(view["connections"][0]["stateLabel"], "Connected");
        assert_eq!(view["connections"][0]["connected"], true);
        assert_eq!(view["connections"][1]["kind"], "wireguard");
        assert_eq!(view["connections"][1]["connected"], false);
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = VpnHost::new(MockVpn::failing("NetworkManager: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "NetworkManager: timeout");
    }

    #[test]
    fn a_connect_round_trips_through_a_re_read() {
        let mut host = VpnHost::new(MockVpn::present(data()));
        host.refresh();
        assert_eq!(host.connect("bbb")["outcome"], "accepted");
        // The write itself changes nothing: the daemon pushes, the host reads.
        assert_eq!(host.view()["connectedCount"], 1);
        host.refresh();
        assert_eq!(host.view()["connectedCount"], 2);
        assert_eq!(host.view()["label"], "2 Connected");
    }

    #[test]
    fn a_disconnect_round_trips_and_reports_the_state() {
        let mut host = VpnHost::new(MockVpn::present(data()));
        host.refresh();
        assert_eq!(host.deactivate("aaa")["outcome"], "accepted");
        host.refresh();
        assert_eq!(host.view()["connectedCount"], 0);
        assert_eq!(host.view()["label"], "Not Connected");
        assert_eq!(host.view()["glyph"], "vpn-off");
    }

    #[test]
    fn a_polkit_denial_degrades_to_read_only_and_is_reported() {
        let mut host = VpnHost::new(MockVpn::present(data()));
        host.refresh();
        host.adapter_mut()
            .source_mut()
            .deny_writes("not authorized to connect");
        let report = host.connect("bbb");
        assert_eq!(report["outcome"], "denied");
        assert_eq!(report["note"], "not authorized to connect");
        let view = host.view();
        assert_eq!(view["readOnly"], true);
        assert_eq!(view["note"], "not authorized to connect");
        // Reads stay live and the item stays visible.
        assert_eq!(view["state"], "available");
    }

    #[test]
    fn an_unknown_uuid_write_is_a_failure_not_a_guess() {
        let mut host = VpnHost::new(MockVpn::present(data()));
        host.refresh();
        assert_eq!(host.connect("ghost")["outcome"], "failed");
    }

    #[test]
    fn a_write_while_absent_reports_absence_and_hides() {
        let mut host = VpnHost::new(MockVpn::absent());
        assert_eq!(host.connect("aaa")["outcome"], "absent");
        assert_eq!(host.view()["state"], "unavailable");
    }
}
