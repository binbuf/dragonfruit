// SPDX-License-Identifier: MIT
//! T-15.13a acceptance: the Privacy and Security adapter reports state and
//! events against a mock, and an absent portal PermissionStore is a normal
//! hidden state. No `xdg-desktop-portal`, bus, or hardware is involved.

use dragonfruit_privacy_adapter::{
    AppPermissionData, MockPrivacy, PermissionState, PrivacyAdapter, PrivacyChange, PrivacyData,
    PrivacyOutcome, PrivacySnapshot, PrivacySource, ResourceData, TableData, KNOWN_TABLES,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the live permission store
/// answers with.
fn fixture() -> PrivacyData {
    serde_json::from_str(include_str!("fixtures/permission-store-workstation.json"))
        .expect("valid fixture")
}

#[test]
fn the_fixture_renders_categories_and_permissions() {
    let mut adapter = PrivacyAdapter::new(MockPrivacy::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("the store answered");
    assert!(snapshot.present());
    assert_eq!(snapshot.glyph(), "privacy");
    assert_eq!(snapshot.label(), "6 Apps");
    assert_eq!(snapshot.app_count(), 6);
    assert_eq!(snapshot.granted_count(), 3);
    assert_eq!(snapshot.denied_count(), 1);

    // Every known table is a row, in the curated order.
    assert_eq!(snapshot.categories().len(), KNOWN_TABLES.len());
    assert_eq!(snapshot.categories()[0].id(), "devices");
    assert_eq!(snapshot.categories()[0].label(), "Camera");
    assert_eq!(snapshot.categories()[1].id(), "location");
    assert_eq!(snapshot.category("usb").unwrap().summary(), "None");

    let camera = snapshot.category("devices").unwrap();
    assert_eq!(camera.summary(), "2 apps");
    assert_eq!(camera.granted_count(), 1);
    assert!(camera
        .resource("camera")
        .unwrap()
        .app("org.mozilla.firefox")
        .unwrap()
        .is_allowed());
    assert!(camera
        .resource("camera")
        .unwrap()
        .app("org.example.Snapshot")
        .unwrap()
        .is_denied());

    // A non-tristate record keeps its raw permissions and reads Unset.
    let location = snapshot.category("location").unwrap();
    let maps = &location.resource("location").unwrap().apps()[0];
    assert_eq!(maps.state, PermissionState::Unset);
    assert_eq!(maps.permissions.len(), 2);

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::PRIVACY);
    assert_eq!(slot.id, AdapterId::PRIVACY);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::PRIVACY);
}

#[test]
fn the_lifecycle_resubscribes_after_the_store_returns() {
    let mut adapter = PrivacyAdapter::new(MockPrivacy::present(fixture()));
    adapter.refresh();
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // The store goes away: the item hides, never an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::PRIVACY).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // It comes back: a re-subscribe and a re-sync, no user-visible error.
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
fn an_app_move_is_an_observable_event() {
    let mut adapter = PrivacyAdapter::new(MockPrivacy::present(fixture()));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();

    let mut next = fixture();
    // Grant the camera to the denied app and revoke the ask app.
    let camera = &mut next.tables[0].resources[0];
    camera.apps[1].permissions = vec!["yes".to_owned()];
    next.tables[2].resources[0]
        .apps
        .retain(|app| app.app != "org.example.Calendar");
    adapter.source_mut().push(next);
    adapter.refresh();

    let changes = adapter.drain_changes();
    assert!(changes.contains(&PrivacyChange::AppChanged {
        table: "devices".to_owned(),
        id: "camera".to_owned(),
        app: "org.example.Snapshot".to_owned(),
    }));
    assert!(changes.contains(&PrivacyChange::AppRemoved {
        table: "notifications".to_owned(),
        id: "notification".to_owned(),
        app: "org.example.Calendar".to_owned(),
    }));
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No store at all: hidden, absent, no error, no panic.
    let mut adapter = PrivacyAdapter::new(MockPrivacy::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());
    assert_eq!(
        adapter.set_permission("devices", "camera", "x", &[]),
        PrivacyOutcome::Absent
    );

    // The store is present but unreadable: visible, inert, with the message.
    let mut failing = PrivacyAdapter::new(MockPrivacy::failing("portal: timeout"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "portal: timeout"
    );
}

#[test]
fn the_writes_are_explicit_and_do_not_invent_a_snapshot() {
    let mut adapter = PrivacyAdapter::new(MockPrivacy::present(fixture()));
    adapter.refresh();
    let before = adapter.snapshot().unwrap().clone();
    let _ = adapter.drain_events();
    let _ = adapter.drain_changes();

    assert!(adapter
        .set_permission(
            "devices",
            "camera",
            "org.example.Snapshot",
            &["yes".to_owned()]
        )
        .is_applied());
    assert!(adapter
        .delete_permission("notifications", "notification", "org.example.Calendar")
        .is_applied());

    assert_eq!(adapter.source().permission_sets(), 1);
    assert_eq!(adapter.source().permission_deletes(), 1);
    assert_eq!(adapter.source().writes(), 2);

    // The adapter state is unchanged until the host re-reads; a write never
    // invents a snapshot.
    assert_eq!(adapter.snapshot().unwrap(), &before);
    assert!(adapter.drain_events().is_empty());

    // The store (mock) published the result; the host re-reads.
    adapter.refresh();
    let after = adapter.snapshot().unwrap();
    assert!(after
        .category("devices")
        .unwrap()
        .resource("camera")
        .unwrap()
        .app("org.example.Snapshot")
        .unwrap()
        .is_allowed());
    assert!(after
        .category("notifications")
        .unwrap()
        .resource("notification")
        .unwrap()
        .app("org.example.Calendar")
        .is_none());
}

#[test]
fn a_denied_write_is_reported_and_the_read_state_stays_live() {
    let mut adapter =
        PrivacyAdapter::new(MockPrivacy::present(fixture()).deny_writes("portal: not authorized"));
    adapter.refresh();

    let outcome = adapter.set_permission("devices", "camera", "x", &["yes".to_owned()]);
    assert_eq!(
        outcome,
        PrivacyOutcome::Denied("portal: not authorized".to_owned())
    );
    assert_eq!(outcome.denial_note(), Some("portal: not authorized"));
    assert!(adapter.state().is_available());
    assert!(adapter.state().slot(AdapterId::PRIVACY).visible);
}

#[test]
fn a_failed_write_is_not_a_denial_and_leaves_the_read_state_live() {
    let mut adapter =
        PrivacyAdapter::new(MockPrivacy::present(fixture()).fail_writes("portal: busy"));
    adapter.refresh();

    let outcome = adapter.delete_permission("devices", "camera", "org.mozilla.firefox");
    assert_eq!(outcome.denial_note(), None);
    assert!(matches!(outcome, PrivacyOutcome::Failed(_)));
    assert_eq!(
        outcome.error().map(|error| error.message()),
        Some("portal: busy")
    );
    assert!(adapter.state().is_available());
}

#[test]
fn an_empty_store_is_available_but_not_present() {
    // The store answers but records nothing: the adapter is available, and
    // `present()` is what hides the tile.
    let mut adapter = PrivacyAdapter::new(MockPrivacy::present(PrivacyData { tables: vec![] }));
    adapter.refresh();
    assert!(adapter.state().is_available());
    assert!(!adapter.snapshot().unwrap().present());
    assert_eq!(adapter.snapshot().unwrap().label(), "No App Permissions");
}

#[test]
fn the_vocabulary_round_trips_through_its_stable_ids() {
    for state in PermissionState::ALL {
        assert_eq!(PermissionState::from_id(state.id()), state);
    }
    assert_eq!(PermissionState::from_id("bogus"), PermissionState::Unset);
    assert_eq!(
        PermissionState::from_permissions(&["yes".to_owned()]),
        PermissionState::Allowed
    );
    assert_eq!(PermissionState::Allowed.permissions(), &["yes"]);
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    let source = MockPrivacy::absent();
    assert!(!source.is_present());
    assert_eq!(source.reads(), 0);
    // The trait-object path is the same seam the live host fills.
    let mut boxed: Box<dyn PrivacySource> = Box::new(MockPrivacy::absent());
    assert_eq!(boxed.read(), Ok(None));

    // A store with an entry, a table and an app the adapter did not know.
    let mut mock = MockPrivacy::present(PrivacyData {
        tables: vec![TableData {
            table: "future".to_owned(),
            resources: vec![ResourceData {
                id: "future".to_owned(),
                apps: vec![AppPermissionData {
                    app: "app".to_owned(),
                    permissions: vec!["yes".to_owned()],
                }],
            }],
        }],
    });
    let read = mock.read().unwrap().unwrap();
    let snapshot = PrivacySnapshot::from_data(&read);
    assert_eq!(snapshot.app_count(), 1);
    assert!(snapshot.changes(&snapshot).is_empty());
}

#[test]
fn the_live_store_reads_when_one_is_present() {
    // The live path is exercised only where a portal permission store exists
    // (the test machine); CI without one reports absence and skips.
    let mut source = dragonfruit_privacy_adapter::HostPrivacy::new();
    match source.read() {
        Ok(None) => {
            // No store here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = PrivacySnapshot::from_data(&data);
            // The typed projection never panics on whatever the store reports.
            let _ = snapshot.label();
            let _ = snapshot.changes(&snapshot);
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
