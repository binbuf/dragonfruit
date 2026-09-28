// SPDX-License-Identifier: MIT
//! T-15.14a acceptance: the Accessibility adapter reports state and events
//! against a mock, and an absent AT-SPI accessibility bus is a normal hidden
//! state. No `at-spi2`, bus, or assistive technology is involved.

use dragonfruit_accessibility_adapter::{
    status_from_properties, AccessibilityAdapter, AccessibilityChange, AccessibilityData,
    AccessibilitySnapshot, AccessibilitySource, HostAccessibility, MockAccessibility,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// The workstation the fixture drives: the bridge on and a screen reader on.
fn fixture() -> AccessibilityData {
    AccessibilityData {
        enabled: true,
        screen_reader: true,
    }
}

#[test]
fn the_fixture_renders_the_bridge_and_screen_reader() {
    let mut adapter = AccessibilityAdapter::new(MockAccessibility::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("the bus answered");
    assert!(snapshot.present());
    assert!(snapshot.is_enabled());
    assert!(snapshot.is_screen_reader_enabled());
    assert_eq!(snapshot.glyph(), "accessibility");
    assert_eq!(snapshot.label(), "Screen Reader On");
    assert_eq!(snapshot.enabled_label(), "On");
    assert_eq!(snapshot.screen_reader_label(), "On");

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::ACCESSIBILITY);
    assert_eq!(slot.id, AdapterId::ACCESSIBILITY);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::ACCESSIBILITY);
}

#[test]
fn the_lifecycle_resubscribes_after_the_bus_returns() {
    let mut adapter = AccessibilityAdapter::new(MockAccessibility::present(fixture()));
    adapter.refresh();
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: false },
            AdapterEvent::Changed,
        ]
    );

    // The bus goes away: the item hides, never an error.
    adapter.source_mut().kill();
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().slot(AdapterId::ACCESSIBILITY).visible);
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
fn a_flag_move_is_an_observable_event() {
    let mut adapter = AccessibilityAdapter::new(MockAccessibility::present(AccessibilityData {
        enabled: true,
        screen_reader: false,
    }));
    adapter.refresh();
    let _ = adapter.drain_changes();
    let _ = adapter.drain_events();
    assert_eq!(adapter.snapshot().unwrap().label(), "On");

    adapter.source_mut().push(fixture());
    adapter.refresh();

    assert_eq!(
        adapter.drain_changes(),
        vec![AccessibilityChange::ScreenReader { enabled: true }]
    );
    assert_eq!(adapter.snapshot().unwrap().label(), "Screen Reader On");
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Changed]);
}

#[test]
fn turning_the_bridge_off_is_an_observable_event() {
    let mut adapter = AccessibilityAdapter::new(MockAccessibility::present(fixture()));
    adapter.refresh();
    let _ = adapter.drain_changes();

    adapter.source_mut().push(AccessibilityData::default());
    adapter.refresh();

    assert_eq!(
        adapter.drain_changes(),
        vec![
            AccessibilityChange::Enabled { enabled: false },
            AccessibilityChange::ScreenReader { enabled: false },
        ]
    );
}

#[test]
fn absence_is_a_normal_state_at_every_seam() {
    // No bus at all: hidden, absent, no error, no panic.
    let mut adapter = AccessibilityAdapter::new(MockAccessibility::absent());
    adapter.refresh();
    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_visible());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    // Already absent at the first read: there is no transition to report.
    assert!(adapter.drain_events().is_empty());
    assert!(adapter.drain_changes().is_empty());

    // The bus is present but unreadable: visible, inert, with the message.
    let mut failing = AccessibilityAdapter::new(MockAccessibility::failing("AT-SPI: timeout"));
    failing.refresh();
    assert!(failing.state().is_error());
    assert!(failing.state().is_visible());
    assert!(!failing.state().is_enabled());
    assert_eq!(
        failing.state().error().unwrap().message(),
        "AT-SPI: timeout"
    );
}

#[test]
fn an_all_off_bus_is_available_but_not_present() {
    // The bus answers but every feature is off: the adapter is available, and
    // `present()` is the second hide rule.
    let mut adapter =
        AccessibilityAdapter::new(MockAccessibility::present(AccessibilityData::default()));
    adapter.refresh();
    assert!(adapter.state().is_available());
    assert!(!adapter.snapshot().unwrap().present());
    assert_eq!(adapter.snapshot().unwrap().label(), "Off");
    assert_eq!(adapter.snapshot().unwrap().glyph(), "accessibility");
}

#[test]
fn the_status_round_trips_through_a_properties_map() {
    // `GetAll` gives an opaque string-keyed map; the adapter types it without
    // guessing and treats an omitted property as off.
    let mut map = std::collections::HashMap::new();
    map.insert("IsEnabled".to_owned(), true);
    map.insert("ScreenReaderEnabled".to_owned(), false);
    let data = status_from_properties(map);
    assert_eq!(
        data,
        AccessibilityData {
            enabled: true,
            screen_reader: false,
        }
    );

    let snapshot = AccessibilitySnapshot::from_data(&data);
    assert!(snapshot.present());
    assert!(!snapshot.is_screen_reader_enabled());
    assert_eq!(snapshot.label(), "On");
}

#[test]
fn the_mock_is_the_ci_path_and_free_to_construct() {
    let source = MockAccessibility::absent();
    assert!(!source.is_present());
    assert_eq!(source.reads(), 0);
    // The trait-object path is the same seam the live host fills.
    let mut boxed: Box<dyn AccessibilitySource> = Box::new(MockAccessibility::absent());
    assert_eq!(boxed.read(), Ok(None));
}

#[test]
fn the_live_bus_reads_when_one_is_present() {
    // The live path is exercised only where an AT-SPI accessibility bus exists
    // (the test machine); CI without one reports absence and skips.
    let mut source = HostAccessibility::new();
    match source.read() {
        Ok(None) => {
            // No bus here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = AccessibilitySnapshot::from_data(&data);
            let _ = snapshot.label();
            let _ = snapshot.changes(&snapshot);
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
