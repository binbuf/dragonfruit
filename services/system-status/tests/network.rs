// SPDX-License-Identifier: MIT
//! T225 bridge-host acceptance: the combined Network view carries the
//! NetworkManager services/known networks and the firewall sub-view, and the
//! two adapters degrade independently.

use dragonfruit_audio::MockAudio;
use dragonfruit_firewall::MockFirewall;
use dragonfruit_networkmanager::{
    AccessPointData, ActiveConnectionData, ConnectionData, MockNetworkManager, NetworkManagerData,
    ServiceData, WifiDeviceData,
};
use dragonfruit_power::MockPower;
use dragonfruit_system_status::{FirewallHost, StatusHost};

fn network_data() -> NetworkManagerData {
    NetworkManagerData {
        wireless_enabled: true,
        connectivity: 4,
        devices: vec![WifiDeviceData {
            managed: true,
            state: 100,
            active_ap: Some("p1".to_owned()),
            access_points: vec![AccessPointData {
                path: "p1".to_owned(),
                ssid: "home".to_owned(),
                strength: 82,
                frequency_mhz: 5180,
                flags: 0x1,
                wpa_flags: 0,
                rsn_flags: 0x108,
            }],
        }],
        connections: vec![
            ConnectionData {
                path: "/conn/wired".to_owned(),
                id: "Wired".to_owned(),
                uuid: "wired".to_owned(),
                kind: "802-3-ethernet".to_owned(),
                priority: 10,
                autoconnect: true,
                ..ConnectionData::default()
            },
            ConnectionData {
                path: "/conn/home".to_owned(),
                id: "Home".to_owned(),
                uuid: "home".to_owned(),
                kind: "802-11-wireless".to_owned(),
                priority: 5,
                autoconnect: true,
                ssid: Some("home".to_owned()),
                ..ConnectionData::default()
            },
        ],
        active: vec![
            ActiveConnectionData {
                path: "/active/wired".to_owned(),
                connection: "/conn/wired".to_owned(),
                uuid: "wired".to_owned(),
                id: "Wired".to_owned(),
                state: 2,
                devices: vec!["/dev/eth".to_owned()],
                ..ActiveConnectionData::default()
            },
            ActiveConnectionData {
                path: "/active/home".to_owned(),
                connection: "/conn/home".to_owned(),
                uuid: "home".to_owned(),
                id: "Home".to_owned(),
                state: 2,
                devices: vec!["/dev/wlan".to_owned()],
                ..ActiveConnectionData::default()
            },
        ],
        services: vec![
            ServiceData {
                path: "/dev/eth".to_owned(),
                kind: 1,
                interface: "enp1s0".to_owned(),
                state: 100,
                managed: true,
                hw_address: Some("aa:bb:cc:dd:ee:02".to_owned()),
                ip4_address: Some("192.168.1.20".to_owned()),
                ..ServiceData::default()
            },
            ServiceData {
                path: "/dev/wlan".to_owned(),
                kind: 2,
                interface: "wlan0".to_owned(),
                state: 100,
                managed: true,
                hw_address: Some("aa:bb:cc:dd:ee:01".to_owned()),
                ip4_address: Some("192.168.1.42".to_owned()),
                ..ServiceData::default()
            },
        ],
        ..NetworkManagerData::default()
    }
}

fn host() -> (
    StatusHost<MockNetworkManager, MockAudio, MockPower>,
    FirewallHost<MockFirewall>,
) {
    let mut host = StatusHost::new(
        MockNetworkManager::present(network_data()),
        MockAudio::absent(),
        MockPower::absent(),
    );
    host.refresh_wifi();
    let mut firewall = FirewallHost::new(MockFirewall::present());
    firewall.refresh();
    (host, firewall)
}

#[test]
fn the_network_view_carries_services_known_networks_and_the_firewall() {
    let (host, firewall) = host();
    let view = host.network_view(firewall.adapter());
    assert_eq!(view["kind"], "network");
    assert_eq!(view["state"], "available");
    assert_eq!(view["serviceCount"], 2);
    assert_eq!(view["services"][0]["id"], "Wired");
    assert_eq!(view["services"][0]["address"], "192.168.1.20");
    assert_eq!(view["knownNetworkCount"], 1);
    assert_eq!(view["knownNetworks"][0]["ssid"], "home");
    assert_eq!(view["knownNetworks"][0]["connected"], true);
    assert_eq!(view["activeAddress"], "192.168.1.42");
    assert_eq!(view["firewall"]["state"], "available");
    assert_eq!(view["firewall"]["backend"], "firewalld");
}

#[test]
fn the_network_and_firewall_halves_degrade_independently() {
    let mut host = StatusHost::new(
        MockNetworkManager::absent(),
        MockAudio::absent(),
        MockPower::absent(),
    );
    host.refresh_wifi();
    let mut firewall = FirewallHost::new(MockFirewall::present());
    firewall.refresh();
    let view = host.network_view(firewall.adapter());
    assert_eq!(view["state"], "unavailable");
    assert_eq!(view["firewall"]["state"], "available");

    let mut host = StatusHost::new(
        MockNetworkManager::present(network_data()),
        MockAudio::absent(),
        MockPower::absent(),
    );
    host.refresh_wifi();
    let mut firewall = FirewallHost::new(MockFirewall::absent());
    firewall.refresh();
    let view = host.network_view(firewall.adapter());
    assert_eq!(view["state"], "available");
    assert_eq!(view["firewall"]["state"], "unavailable");
    assert_eq!(view["firewall"]["detected"], false);
}

#[test]
fn nftables_is_detected_but_its_status_is_unavailable() {
    // The nftables mock reports status_known false rather than an Off toggle.
    let mut firewall = FirewallHost::new(MockFirewall::nftables_unavailable());
    firewall.refresh();
    let view = firewall.view();
    assert_eq!(view["detected"], true);
    assert_eq!(view["backend"], "nftables");
    assert_eq!(view["statusKnown"], false);
    assert_eq!(view["enabled"], serde_json::Value::Null);
    assert_eq!(firewall.set_enabled(true)["outcome"], "unsupported");
}
