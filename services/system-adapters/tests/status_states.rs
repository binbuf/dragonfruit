// SPDX-License-Identifier: MIT
//! T-07.1a acceptance: a consumer renders the right menu-bar slot for each
//! adapter state, driven only by the mock.

use dragonfruit_system_adapters::{
    status_slots, Adapter, AdapterId, AdapterState, MockAdapter, StatusSource,
};

/// A stand-in for a real daemon snapshot: the shell renders its icon/label
/// from the typed snapshot, not from the state.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WifiSnapshot {
    ssid: String,
    signal: u8,
}

#[test]
fn a_consumer_renders_one_slot_per_adapter_state() {
    // The same three adapters a menu bar owns: Wi-Fi available, Bluetooth
    // absent, battery erroring.
    let wifi = MockAdapter::with_available(
        AdapterId::WIFI,
        WifiSnapshot {
            ssid: "dragonfruit".to_owned(),
            signal: 82,
        },
    );
    let bluetooth = MockAdapter::<()>::new(AdapterId::BLUETOOTH);
    let battery = {
        let mut battery = MockAdapter::<u8>::new(AdapterId::POWER);
        battery.set_error("UPower: no reply to GetDisplayDevice");
        battery
    };

    let sources: [&dyn StatusSource; 3] = [&wifi, &bluetooth, &battery];
    let slots = status_slots(sources);

    // Available -> interactive, and the snapshot is readable.
    assert_eq!(slots[0].id, AdapterId::WIFI);
    assert!(slots[0].visible && slots[0].enabled);
    assert_eq!(slots[0].error, None);
    assert_eq!(
        wifi.state().snapshot(),
        Some(&WifiSnapshot {
            ssid: "dragonfruit".to_owned(),
            signal: 82,
        })
    );

    // Unavailable -> hidden (an absent daemon is not an error).
    assert_eq!(slots[1].id, AdapterId::BLUETOOTH);
    assert!(!slots[1].visible && !slots[1].enabled);
    assert_eq!(slots[1].error, None);

    // Error -> visible but inert, with the message.
    assert_eq!(slots[2].id, AdapterId::POWER);
    assert!(slots[2].visible && !slots[2].enabled);
    assert_eq!(
        slots[2].error.as_deref(),
        Some("UPower: no reply to GetDisplayDevice")
    );
}

#[test]
fn a_mock_can_move_between_states_without_a_daemon() {
    let mut adapter = MockAdapter::<u8>::new(AdapterId::AUDIO);
    assert!(adapter.state().is_unavailable());

    adapter.set_available(30);
    assert!(adapter.slot().enabled);

    adapter.set_error("WirePlumber: gone");
    assert!(adapter.state().is_error());

    adapter.set_unavailable();
    assert!(adapter.state().is_unavailable());
    assert!(matches!(adapter.state(), AdapterState::Unavailable));
}
