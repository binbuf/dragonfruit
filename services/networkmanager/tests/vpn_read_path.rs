// SPDX-License-Identifier: MIT
//! T-15.15a acceptance: the VPN adapter reads a mocked NetworkManager, renders
//! state + connection list, reacts to events, and hides an absent daemon. No
//! bus and no daemon are involved.

use dragonfruit_networkmanager::{MockVpn, VpnAdapter, VpnData, VpnKind, VpnResult, VpnState};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the D-Bus source returns.
fn fixture() -> VpnData {
    serde_json::from_str(include_str!("fixtures/nm-vpn.json")).expect("valid fixture")
}

fn adapter() -> VpnAdapter<MockVpn> {
    let mut adapter = VpnAdapter::new(MockVpn::present(fixture()));
    adapter.refresh();
    adapter
}

#[test]
fn the_fixture_renders_connections_and_live_state() {
    let adapter = adapter();
    let snapshot = adapter.snapshot().expect("NetworkManager answered");

    assert!(snapshot.present());
    assert_eq!(snapshot.connection_count(), 3);
    // The connected and connecting connections sort ahead of the idle one.
    let order: Vec<&str> = snapshot
        .connections
        .iter()
        .map(|connection| connection.uuid.as_str())
        .collect();
    assert_eq!(
        order,
        [
            "11112222-3333-4444-5555-666677778888",
            "abcdabcd-abcd-abcd-abcd-abcdabcdabcd",
            "99990000-aaaa-bbbb-cccc-ddddeeeeffff",
        ]
    );

    let work = &snapshot.connections[0];
    assert_eq!(work.id, "Work VPN");
    assert_eq!(work.kind, VpnKind::OpenVpn);
    assert_eq!(work.state, VpnState::Connected);
    assert!(work.is_connected());

    let branch = &snapshot.connections[1];
    assert_eq!(branch.kind, VpnKind::Ipsec);
    assert_eq!(branch.state, VpnState::Connecting);

    let home = &snapshot.connections[2];
    assert_eq!(home.kind, VpnKind::WireGuard);
    assert_eq!(home.state, VpnState::Disconnected);
    assert!(home.autoconnect);

    assert_eq!(snapshot.connected_count(), 1);
    assert_eq!(
        snapshot.active_uuid(),
        Some("11112222-3333-4444-5555-666677778888")
    );
    assert_eq!(snapshot.label(), "Work VPN");
    assert_eq!(snapshot.glyph(), "vpn");

    // The status projection is live and enabled.
    let slot = adapter.state().slot(AdapterId::VPN);
    assert_eq!(slot.id, AdapterId::VPN);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::VPN);
}

#[test]
fn an_absent_networkmanager_hides_the_item_and_never_errors() {
    let mut adapter = VpnAdapter::new(MockVpn::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert_eq!(adapter.subscriptions(), 0);

    let slot = adapter.state().slot(AdapterId::VPN);
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn a_present_but_unreadable_daemon_is_visible_and_inert() {
    let mut adapter = VpnAdapter::new(MockVpn::failing("NetworkManager: no reply"));
    adapter.refresh();

    assert!(adapter.state().is_error());
    assert!(adapter.state().is_visible());
    assert!(!adapter.state().is_enabled());

    let slot = adapter.state().slot(AdapterId::VPN);
    assert!(slot.visible && !slot.enabled);
    assert_eq!(slot.error.as_deref(), Some("NetworkManager: no reply"));
}

#[test]
fn a_daemon_restart_resubscribes_and_resyncs() {
    let mut adapter = adapter();
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
    assert!(!adapter.state().slot(AdapterId::VPN).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // Unmask it: the adapter re-subscribes and re-syncs to the fixture.
    adapter.source_mut().restart();
    adapter.refresh();
    assert!(adapter.state().slot(AdapterId::VPN).visible);
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(adapter.subscriptions(), 2);
    assert_eq!(adapter.snapshot().unwrap().connected_count(), 1);
}

#[test]
fn connect_and_disconnect_round_trip_through_the_daemons_push() {
    let mut adapter = adapter();
    let home = "99990000-aaaa-bbbb-cccc-ddddeeeeffff";
    assert_eq!(
        adapter
            .snapshot()
            .unwrap()
            .connections
            .iter()
            .find(|connection| connection.uuid == home)
            .unwrap()
            .state,
        VpnState::Disconnected
    );

    // A write invents no snapshot; the daemon pushes and the host re-reads.
    assert_eq!(adapter.connect(home), VpnResult::Accepted);
    assert_eq!(adapter.source().activations(), 1);
    assert_eq!(
        adapter
            .snapshot()
            .unwrap()
            .connections
            .iter()
            .find(|connection| connection.uuid == home)
            .unwrap()
            .state,
        VpnState::Disconnected
    );

    adapter.refresh();
    assert_eq!(
        adapter
            .snapshot()
            .unwrap()
            .connections
            .iter()
            .find(|connection| connection.uuid == home)
            .unwrap()
            .state,
        VpnState::Connected
    );

    assert_eq!(adapter.deactivate(home), VpnResult::Accepted);
    adapter.refresh();
    assert_eq!(adapter.snapshot().unwrap().connected_count(), 1);
    assert_eq!(adapter.source().deactivations(), 1);
}

#[test]
fn a_polkit_denial_degrades_to_read_only_but_reads_stay_live() {
    let mut adapter = adapter();
    adapter
        .source_mut()
        .deny_writes("org.freedesktop.NetworkManager.PermissionDenied: not authorized");

    let result = adapter.connect("99990000-aaaa-bbbb-cccc-ddddeeeeffff");
    let note = match result {
        VpnResult::Denied { note } => note,
        other => panic!("expected a denial, got {other:?}"),
    };
    assert!(note.contains("not authorized"));

    assert!(adapter.is_read_only());
    assert_eq!(adapter.degradation_note(), Some(note.as_str()));
    // Reads keep working; only the write affordances are disabled.
    assert!(adapter.state().is_available());
    assert!(adapter.state().is_visible());
    assert!(adapter.state().is_enabled());

    // A second write is refused locally, without touching the daemon again.
    assert!(matches!(
        adapter.connect("99990000-aaaa-bbbb-cccc-ddddeeeeffff"),
        VpnResult::Denied { .. }
    ));
    assert_eq!(adapter.source().activations(), 1);
}

#[test]
fn an_empty_store_is_present_false() {
    let mut adapter = VpnAdapter::new(MockVpn::present(VpnData::default()));
    adapter.refresh();
    let snapshot = adapter.snapshot().unwrap();
    assert!(!snapshot.present());
    assert_eq!(snapshot.label(), "No VPN");
    // The adapter is still available: a configured-but-empty store is not
    // absence.
    assert!(adapter.state().is_available());
}
