// SPDX-License-Identifier: MIT
//! Bridge tests for the Network advanced (VPN) half of the host (T-15.15b):
//! the view the Settings pane and Control Center tile decode, and the two
//! explicit writes. The adapter is driven by `MockVpn`, so the connect /
//! deactivate round-trip is observable with no bus and no daemon.

use dragonfruit_networkmanager::{ActiveVpnData, MockVpn, VpnConnectionData, VpnData};
use dragonfruit_system_status::VpnHost;

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
fn the_view_exposes_the_connections_and_live_state() {
    let mut host = VpnHost::new(MockVpn::present(data()));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "vpn");
    assert_eq!(view["state"], "available");
    assert_eq!(view["glyph"], "vpn");
    assert_eq!(view["label"], "Work VPN");
    assert_eq!(view["present"], true);
    assert_eq!(view["connectionCount"], 2);
    assert_eq!(view["connectedCount"], 1);
    assert_eq!(view["connections"][0]["state"], "connected");
    assert_eq!(view["connections"][1]["state"], "disconnected");
}

#[test]
fn a_daemon_with_no_vpn_is_available_but_not_present() {
    let mut host = VpnHost::new(MockVpn::present(VpnData::default()));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], false);
    assert_eq!(view["label"], "No VPN");
}

#[test]
fn absence_hides_the_item() {
    let mut host = VpnHost::new(MockVpn::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert!(host.state().contains("unavailable"));
}

#[test]
fn a_read_failure_is_a_visible_inert_error() {
    let mut host = VpnHost::new(MockVpn::failing("NetworkManager: timeout"));
    host.refresh();
    let view = host.view();
    assert_eq!(view["state"], "error");
    assert_eq!(view["error"], "NetworkManager: timeout");
}

#[test]
fn a_connect_and_disconnect_round_trip_through_the_bridge() {
    let mut host = VpnHost::new(MockVpn::present(data()));
    host.refresh();
    assert_eq!(host.connect("bbb")["outcome"], "accepted");
    // The write itself invents nothing: the host re-reads the pushed state.
    assert_eq!(host.view()["connectedCount"], 1);
    host.refresh();
    assert_eq!(host.view()["connectedCount"], 2);

    assert_eq!(host.deactivate("aaa")["outcome"], "accepted");
    host.refresh();
    assert_eq!(host.view()["connectedCount"], 1);
}

#[test]
fn a_polkit_denial_degrades_to_read_only_and_is_reported() {
    let mut host = VpnHost::new(MockVpn::present(data()));
    host.refresh();
    host.adapter_mut()
        .source_mut()
        .deny_writes("not authorized");
    let report = host.connect("bbb");
    assert_eq!(report["outcome"], "denied");
    assert_eq!(report["note"], "not authorized");
    let view = host.view();
    assert_eq!(view["readOnly"], true);
    assert_eq!(view["note"], "not authorized");
    // Reads stay live and the item stays visible.
    assert_eq!(view["state"], "available");
}

#[test]
fn a_refresh_reads_once_and_never_invents_a_snapshot() {
    let mut host = VpnHost::new(MockVpn::present(data()));
    host.refresh();
    assert_eq!(host.adapter().source().reads(), 1);
    host.refresh();
    assert_eq!(host.adapter().source().reads(), 2);
}
