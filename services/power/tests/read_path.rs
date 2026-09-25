// SPDX-License-Identifier: MIT
//! T-07.4 acceptance: the adapter reads a mocked UPower and renders battery
//! presence, level, and charging state, and an absent daemon hides the item.
//! No bus and no daemon are involved.

use dragonfruit_power::{
    DbusUPower, MockPower, PowerAdapter, PowerData, PowerSnapshot, PowerSource,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the live D-Bus source builds.
fn fixture() -> PowerData {
    serde_json::from_str(include_str!("fixtures/upower-laptop.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_the_battery_level_and_charge_state() {
    let mut adapter = PowerAdapter::new(MockPower::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("UPower answered");
    assert!(snapshot.present());
    assert_eq!(snapshot.percentage(), 82.0);
    assert_eq!(snapshot.percentage_percent(), 82);
    assert!(!snapshot.on_battery());
    assert!(snapshot.charging());
    assert!(snapshot.plugged());
    assert_eq!(snapshot.state().name(), "charging");
    assert_eq!(snapshot.glyph(), "battery");
    assert_eq!(snapshot.label(), "82% charging");
    assert_eq!(snapshot.time_to_full(), Some(5400));

    // The slot is live and the source is the menu-bar status source.
    let slot = adapter.state().slot(AdapterId::POWER);
    assert_eq!(slot.id, AdapterId::POWER);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::POWER);
}

#[test]
fn an_absent_upower_hides_the_item_and_never_errors() {
    let mut adapter = PowerAdapter::new(MockPower::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert_eq!(adapter.subscriptions(), 0);

    let slot = adapter.state().slot(AdapterId::POWER);
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);

    // Nothing was subscribed, so there is no spurious Disconnected event.
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn a_present_daemon_with_no_battery_has_no_item_to_show() {
    // A desktop or VM: UPower is up, but no battery is present. The adapter
    // is available; `present()` is what tells the shell to hide the item.
    let mut adapter = PowerAdapter::new(MockPower::present(PowerData::default()));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert!(!snapshot.present());
    assert_eq!(snapshot.label(), "No battery");
    assert_eq!(snapshot.level(), 0.0);
}

#[test]
fn a_present_but_unreadable_daemon_is_visible_and_inert() {
    let mut adapter = PowerAdapter::new(MockPower::failing("UPower: timeout"));

    adapter.refresh();
    assert!(adapter.state().is_error());
    assert!(adapter.state().is_visible());
    assert!(!adapter.state().is_enabled());

    let slot = adapter.state().slot(AdapterId::POWER);
    assert!(slot.visible && !slot.enabled);
    assert_eq!(slot.error.as_deref(), Some("UPower: timeout"));
}

#[test]
fn a_daemon_restart_resubscribes_and_resyncs() {
    let mut adapter = PowerAdapter::new(MockPower::present(fixture()));
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
    assert!(!adapter.state().slot(AdapterId::POWER).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // Unmask it: the adapter re-subscribes and re-syncs to the fixture.
    adapter.source_mut().restart();
    adapter.refresh();
    assert!(adapter.state().slot(AdapterId::POWER).visible);
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(adapter.subscriptions(), 2);
    assert_eq!(adapter.snapshot().unwrap().percentage_percent(), 82);
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
    let mut adapter = PowerAdapter::new(MockPower::present(fixture()));
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

#[test]
fn the_live_upower_reads_when_a_session_is_present() {
    // The live D-Bus path is exercised only where a UPower daemon exists (the
    // test machine); CI without one reports absence and skips. This is the
    // task's "VM/manual where available" check.
    let mut source = DbusUPower::new();
    match source.read() {
        Ok(None) => {
            // No UPower here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            let snapshot = PowerSnapshot::from_data(&data);
            for device in &data.devices {
                assert!(!device.path.is_empty());
            }
            if let Some(battery) = snapshot.battery() {
                assert!((0.0..=100.0).contains(&battery.percentage));
            }
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
