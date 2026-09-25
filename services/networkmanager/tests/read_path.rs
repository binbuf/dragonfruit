// SPDX-License-Identifier: MIT
//! T-07.2a acceptance: the adapter reads a mocked NetworkManager and renders
//! state + access-point list, and an absent daemon hides the item. No bus and
//! no daemon are involved.

use dragonfruit_networkmanager::{
    Connectivity, MockNetworkManager, NetworkManagerAdapter, NetworkManagerData, Security,
    WifiState,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the D-Bus source returns.
fn fixture() -> NetworkManagerData {
    serde_json::from_str(include_str!("fixtures/nm-office.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_state_and_the_access_point_list() {
    let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("NetworkManager answered");
    assert_eq!(snapshot.state, WifiState::Connected);
    assert_eq!(snapshot.connectivity, Connectivity::Full);
    assert!(snapshot.enabled);
    assert_eq!(snapshot.active_ssid.as_deref(), Some("dragonfruit"));
    assert_eq!(snapshot.signal_strength(), Some(82));

    // The two "dragonfruit" BSSIDs collapse to one entry; the list is sorted
    // strongest-first; the open cafe is last.
    assert_eq!(snapshot.network_count(), 3);
    let ssids: Vec<&str> = snapshot
        .access_points
        .iter()
        .map(|ap| ap.ssid.as_str())
        .collect();
    assert_eq!(ssids, ["dragonfruit", "Neighbour 5G", "Cafe Open"]);
    assert_eq!(snapshot.access_points[0].security, Security::Wpa2);
    assert!(snapshot.access_points[0].active);
    assert_eq!(snapshot.access_points[1].strength, 47);
    assert_eq!(snapshot.access_points[2].security, Security::Open);

    // The menu-bar projection is live.
    let slot = adapter.state().slot(AdapterId::WIFI);
    assert_eq!(slot.id, AdapterId::WIFI);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::WIFI);
    assert_eq!(adapter.snapshot().unwrap().glyph(), "wifi-secure");
}

#[test]
fn an_absent_networkmanager_hides_the_item_and_never_errors() {
    let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert_eq!(adapter.subscriptions(), 0);

    let slot = adapter.state().slot(AdapterId::WIFI);
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);

    // Nothing was subscribed, so there is no spurious Disconnected event.
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn a_present_but_unreadable_daemon_is_visible_and_inert() {
    let mut adapter =
        NetworkManagerAdapter::new(MockNetworkManager::failing("NetworkManager: no reply"));

    adapter.refresh();
    assert!(adapter.state().is_error());
    assert!(adapter.state().is_visible());
    assert!(!adapter.state().is_enabled());

    let slot = adapter.state().slot(AdapterId::WIFI);
    assert!(slot.visible && !slot.enabled);
    assert_eq!(slot.error.as_deref(), Some("NetworkManager: no reply"));
}

#[test]
fn a_daemon_restart_resubscribes_and_resyncs() {
    let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::present(fixture()));
    adapter.refresh();
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // Mask the daemon: absence hides the item, not an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::WIFI).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // Unmask it: the adapter re-subscribes and re-syncs to the fixture.
    adapter.source_mut().restart();
    adapter.refresh();
    assert!(adapter.state().slot(AdapterId::WIFI).visible);
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(adapter.subscriptions(), 2);
    assert_eq!(
        adapter.snapshot().unwrap().active_ssid.as_deref(),
        Some("dragonfruit")
    );
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: true },
            AdapterEvent::Changed,
        ]
    );
}

#[test]
fn the_adapter_reads_once_per_refresh_never_in_a_poll() {
    let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::present(fixture()));
    assert_eq!(adapter.source().reads(), 0);

    adapter.refresh();
    assert_eq!(adapter.source().reads(), 1);

    // Re-reading the state is free: a consumer never touches the daemon.
    let _ = adapter.state();
    let _ = adapter.snapshot();
    assert_eq!(adapter.source().reads(), 1);

    adapter.refresh();
    assert_eq!(adapter.source().reads(), 2);
}
