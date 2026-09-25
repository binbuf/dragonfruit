// SPDX-License-Identifier: MIT
//! T-07.2b acceptance: a join against a mocked NetworkManager succeeds, and a
//! polkit denial degrades the adapter to read-only with a recorded note. No bus
//! and no daemon are involved.

use dragonfruit_networkmanager::{
    JoinRequest, JoinResult, MockNetworkManager, NetworkManagerAdapter, NetworkManagerData,
    WifiAccess, WifiState,
};
use dragonfruit_system_adapters::{Adapter, AdapterId};

/// A fixture shaped exactly like the raw read the D-Bus source returns.
fn fixture() -> NetworkManagerData {
    serde_json::from_str(include_str!("fixtures/nm-office.json")).expect("valid fixture")
}

fn connected_adapter() -> NetworkManagerAdapter<MockNetworkManager> {
    let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::present(fixture()));
    adapter.refresh();
    adapter
}

#[test]
fn a_join_against_a_mocked_networkmanager_is_accepted() {
    let mut adapter = connected_adapter();
    let result = adapter.join(&JoinRequest::new("dragonfruit", Some("secret".to_owned())));

    assert_eq!(result, JoinResult::Accepted);
    // The adapter stays read-write and still renders the live list.
    assert_eq!(adapter.access(), &WifiAccess::ReadWrite);
    assert!(!adapter.is_read_only());
    assert_eq!(adapter.degradation_note(), None);
    assert!(adapter.state().is_available());
    assert_eq!(
        adapter.snapshot().unwrap().active_ssid.as_deref(),
        Some("dragonfruit")
    );
    // Exactly one explicit write; no poll loop.
    assert_eq!(adapter.source().activations(), 1);
}

#[test]
fn a_polkit_denial_leaves_a_read_only_item_with_a_recorded_note() {
    let mut adapter = connected_adapter();
    adapter
        .source_mut()
        .deny_joins("org.freedesktop.NetworkManager.PermissionDenied: not authorized");

    let result = adapter.join(&JoinRequest::new("Neighbour 5G", Some("secret".to_owned())));

    // The denial is reported with the note the adapter recorded.
    let note = match result {
        JoinResult::Denied { note } => note,
        other => panic!("expected a denial, got {other:?}"),
    };
    assert!(note.contains("not authorized"));

    // The item is now read-only: reads keep working (visible + enabled), the
    // join affordance is off, and the note is queryable.
    assert!(adapter.is_read_only());
    assert_eq!(adapter.degradation_note(), Some(note.as_str()));
    assert!(adapter.state().is_available());
    let slot = adapter.state().slot(AdapterId::WIFI);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(
        adapter.snapshot().unwrap().state,
        WifiState::Connected,
        "the read path is unaffected by the denied write"
    );

    // Once read-only, a second join is refused locally: the daemon is not
    // asked again.
    let second = adapter.join(&JoinRequest::new("Neighbour 5G", None));
    assert!(matches!(second, JoinResult::Denied { .. }));
    assert_eq!(adapter.source().activations(), 1);
}

#[test]
fn a_denied_join_does_not_drop_the_network_list() {
    let mut adapter = connected_adapter();
    let before = adapter.snapshot().unwrap().network_count();
    adapter.source_mut().deny_joins("not authorized");

    adapter.join(&JoinRequest::new("dragonfruit", None));

    assert_eq!(adapter.snapshot().unwrap().network_count(), before);
    // A later refresh cannot re-grant the write permission.
    adapter.refresh();
    assert!(adapter.is_read_only());
    assert!(adapter.state().is_available());
}

#[test]
fn an_absent_daemon_reports_absence_and_never_errors() {
    let mut adapter = NetworkManagerAdapter::new(MockNetworkManager::absent());
    let result = adapter.join(&JoinRequest::new("dragonfruit", None));

    assert_eq!(result, JoinResult::Absent);
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert!(!adapter.state().slot(AdapterId::WIFI).visible);
}
