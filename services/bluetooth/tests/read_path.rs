// SPDX-License-Identifier: MIT
//! T-15.1a acceptance: the adapter reads a mocked BlueZ and reports adapter
//! and device state, discovery/pairing writes are explicit, and an absent
//! daemon is a normal hidden state. No bus and no daemon are involved.

use dragonfruit_bluetooth::{
    BluetoothAdapter, BluetoothData, BluetoothOutcome, BluetoothSource, DbusBluez, MockBluetooth,
};
use dragonfruit_system_adapters::{
    Adapter, AdapterEvent, AdapterId, ConnectionState, StatusSource,
};

/// A fixture shaped exactly like the raw read the live D-Bus source builds
/// from `GetManagedObjects`.
fn fixture() -> BluetoothData {
    serde_json::from_str(include_str!("fixtures/bluez-office.json")).expect("valid fixture")
}

#[test]
fn the_fixture_renders_the_adapter_and_devices() {
    let mut adapter = BluetoothAdapter::new(MockBluetooth::present(fixture()));

    // A fresh adapter is absent until the first read; the read is explicit.
    assert!(adapter.state().is_unavailable());
    adapter.refresh();

    let snapshot = adapter.snapshot().expect("BlueZ answered");
    assert!(snapshot.present());
    assert!(snapshot.powered());
    assert!(snapshot.pairable());
    assert!(!snapshot.discovering());
    assert_eq!(snapshot.adapter().unwrap().display_name(), "Workstation");
    assert_eq!(snapshot.glyph(), "bluetooth");
    assert_eq!(snapshot.label(), "Bluetooth connected (1)");

    // Only hci0's devices, connected first.
    assert_eq!(snapshot.devices().len(), 3);
    assert_eq!(snapshot.devices()[0].address, "AA:BB:CC:DD:EE:FF");
    assert!(snapshot.devices()[0].is_connected());
    assert_eq!(snapshot.devices()[0].label(), "Sony Headset");
    assert_eq!(snapshot.devices()[0].signal_percent(), Some(76));
    assert_eq!(snapshot.connected_count(), 1);
    assert_eq!(snapshot.known_devices().len(), 2);
    assert_eq!(snapshot.nearby_devices().len(), 1);

    // A device with no RSSI has no signal fill; an un-aliased one falls back
    // to its advertised name.
    let speaker = snapshot.device_by_address("DE:AD:BE:EF:00:11").unwrap();
    assert_eq!(speaker.signal_percent(), Some(31));
    assert_eq!(speaker.label(), "Speaker");
    // The mouse fixture reports RSSI 0: unknown, not a real reading.
    let mouse = snapshot.device_by_address("11:22:33:44:55:66").unwrap();
    assert_eq!(mouse.signal_percent(), None);
    assert_eq!(mouse.label(), "Work Mouse");
    assert!(snapshot.device_by_address("66:66:66:66:66:66").is_none());

    // The slot is live and the adapter is the status source.
    let slot = adapter.state().slot(AdapterId::BLUETOOTH);
    assert_eq!(slot.id, AdapterId::BLUETOOTH);
    assert!(slot.visible && slot.enabled);
    assert_eq!(slot.error, None);
    assert_eq!(StatusSource::id(&adapter), AdapterId::BLUETOOTH);
}

#[test]
fn an_absent_bluez_hides_the_item_and_never_errors() {
    let mut adapter = BluetoothAdapter::new(MockBluetooth::absent());
    adapter.refresh();

    assert!(adapter.state().is_unavailable());
    assert!(!adapter.state().is_error());
    assert_eq!(adapter.connection(), ConnectionState::Absent);
    assert_eq!(adapter.subscriptions(), 0);

    let slot = adapter.state().slot(AdapterId::BLUETOOTH);
    assert!(!slot.visible && !slot.enabled);
    assert_eq!(slot.error, None);

    // Nothing was subscribed, so there is no spurious Disconnected event.
    assert!(adapter.drain_events().is_empty());
}

#[test]
fn a_running_daemon_with_no_controller_has_nothing_to_show() {
    // No hardware: BlueZ is up, but there is no adapter. The adapter is
    // available; `present()` is what tells the shell to hide the item.
    let mut adapter = BluetoothAdapter::new(MockBluetooth::present(BluetoothData::default()));
    adapter.refresh();

    assert!(adapter.state().is_available());
    let snapshot = adapter.snapshot().unwrap();
    assert!(!snapshot.present());
    assert_eq!(snapshot.label(), "Bluetooth unavailable");
    assert_eq!(snapshot.glyph(), "bluetooth-disabled");
}

#[test]
fn a_present_but_unreadable_daemon_is_visible_and_inert() {
    let mut adapter = BluetoothAdapter::new(MockBluetooth::failing("BlueZ: timeout"));

    adapter.refresh();
    assert!(adapter.state().is_error());
    assert!(adapter.state().is_visible());
    assert!(!adapter.state().is_enabled());

    let slot = adapter.state().slot(AdapterId::BLUETOOTH);
    assert!(slot.visible && !slot.enabled);
    assert_eq!(slot.error.as_deref(), Some("BlueZ: timeout"));
}

#[test]
fn a_daemon_restart_resubscribes_and_resyncs() {
    let mut adapter = BluetoothAdapter::new(MockBluetooth::present(fixture()));
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
    assert!(!adapter.state().slot(AdapterId::BLUETOOTH).visible);
    assert_eq!(adapter.drain_events(), vec![AdapterEvent::Disconnected]);

    // Unmask it: the adapter re-subscribes and re-syncs to the fixture.
    adapter.source_mut().restart();
    adapter.refresh();
    assert!(adapter.state().slot(AdapterId::BLUETOOTH).visible);
    assert_eq!(adapter.connection(), ConnectionState::Subscribed);
    assert_eq!(adapter.subscriptions(), 2);
    assert!(adapter.snapshot().unwrap().powered());
    assert_eq!(
        adapter.drain_events(),
        vec![
            AdapterEvent::Subscribed { resubscribe: true },
            AdapterEvent::Changed,
        ]
    );
}

#[test]
fn discovery_and_pairing_are_explicit_writes_that_do_not_invent_a_snapshot() {
    let mut adapter = BluetoothAdapter::new(MockBluetooth::present(fixture()));
    adapter.refresh();
    assert!(!adapter.snapshot().unwrap().discovering());

    // Each action is one write; the snapshot only changes when BlueZ pushes
    // and the host re-reads.
    assert!(adapter.set_discovering(true).is_accepted());
    assert!(adapter.pair("DE:AD:BE:EF:00:11").is_accepted());
    assert!(adapter
        .set_connected("AA:BB:CC:DD:EE:FF", false)
        .is_accepted());
    assert_eq!(adapter.source().writes(), 3);
    assert!(!adapter.snapshot().unwrap().discovering());

    // BlueZ pushes the resulting state and the host re-reads.
    let mut pushed = fixture();
    pushed.adapters[0].discovering = true;
    adapter.source_mut().push(pushed);
    adapter.refresh();
    assert!(adapter.snapshot().unwrap().discovering());
    assert_eq!(adapter.snapshot().unwrap().label(), "Bluetooth discovering");
}

#[test]
fn a_denied_write_is_reported_and_the_read_state_stays_live() {
    let mut adapter = BluetoothAdapter::new(MockBluetooth::present(fixture()));
    adapter.refresh();
    adapter.source_mut().deny_writes("BlueZ: not authorized");

    let outcome = adapter.set_powered(false);
    assert_eq!(
        outcome,
        BluetoothOutcome::Denied("BlueZ: not authorized".to_owned())
    );
    assert!(adapter.state().is_available());
    assert!(adapter.state().slot(AdapterId::BLUETOOTH).visible);
}

#[test]
fn the_adapter_reads_once_per_refresh_never_in_a_poll() {
    let mut adapter = BluetoothAdapter::new(MockBluetooth::present(fixture()));
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
fn the_live_bluez_reads_when_a_session_is_present() {
    // The live D-Bus path is exercised only where a BlueZ daemon exists (the
    // test machine); CI without one reports absence and skips.
    let mut source = DbusBluez::new();
    match source.read() {
        Ok(None) => {
            // No BlueZ here: the absence path is already covered.
        }
        Ok(Some(data)) => {
            for adapter in &data.adapters {
                assert!(!adapter.path.is_empty());
            }
            let snapshot = dragonfruit_bluetooth::BluetoothSnapshot::from_data(&data);
            if let Some(adapter) = snapshot.adapter() {
                assert!(!adapter.path.is_empty());
            }
        }
        Err(error) => panic!("live read errored: {}", error.message()),
    }
}
