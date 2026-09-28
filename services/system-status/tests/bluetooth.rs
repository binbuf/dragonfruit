// SPDX-License-Identifier: MIT
//! Bridge tests for the Bluetooth half of the host (T-15.1b): the view the
//! Settings pane and Control Center tile decode, plus the four explicit writes
//! applied once through the adapter.

use dragonfruit_bluetooth::{
    BluetoothAdapterData, BluetoothData, BluetoothDeviceData, MockBluetooth,
};
use dragonfruit_system_status::BluetoothHost;

fn data(powered: bool) -> BluetoothData {
    BluetoothData {
        adapters: vec![BluetoothAdapterData {
            path: "/org/bluez/hci0".to_owned(),
            address: "11:22:33:44:55:66".to_owned(),
            name: "workstation".to_owned(),
            alias: "Workstation".to_owned(),
            powered,
            discoverable: false,
            pairable: true,
            discovering: false,
        }],
        devices: vec![BluetoothDeviceData {
            path: "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF".to_owned(),
            adapter_path: "/org/bluez/hci0".to_owned(),
            address: "AA:BB:CC:DD:EE:FF".to_owned(),
            name: "Headset".to_owned(),
            alias: "WF-1000XM6".to_owned(),
            paired: true,
            connected: false,
            trusted: true,
            blocked: false,
            rssi: -60,
            icon: "audio-headset".to_owned(),
        }],
    }
}

#[test]
fn the_bluetooth_view_exposes_the_adapter_and_known_devices() {
    let mut host = BluetoothHost::new(MockBluetooth::present(data(true)));
    host.refresh();
    let view = host.view();
    assert_eq!(view["kind"], "bluetooth");
    assert_eq!(view["state"], "available");
    assert_eq!(view["present"], true);
    assert_eq!(view["powered"], true);
    assert_eq!(view["adapterName"], "Workstation");
    assert_eq!(view["knownCount"], 1);
    assert_eq!(view["knownDevices"][0]["name"], "WF-1000XM6");
    assert_eq!(view["knownDevices"][0]["connected"], false);
}

#[test]
fn the_four_writes_apply_once_and_never_invent_a_snapshot() {
    let mut host = BluetoothHost::new(MockBluetooth::present(data(false)));
    host.refresh();
    assert_eq!(host.set_powered(true)["outcome"], "accepted");
    assert_eq!(host.set_discovering(true)["outcome"], "accepted");
    assert_eq!(host.pair("AA:BB:CC:DD:EE:FF")["outcome"], "accepted");
    assert_eq!(
        host.set_connected("AA:BB:CC:DD:EE:FF", true)["outcome"],
        "accepted"
    );
    assert_eq!(host.adapter().source().writes(), 4);
    // The old snapshot stands until BlueZ pushes and the host re-reads.
    assert!(!host.adapter().snapshot().unwrap().powered());
}

#[test]
fn absence_hides_the_item_and_writes_answer_absence() {
    let mut host = BluetoothHost::new(MockBluetooth::absent());
    host.refresh();
    assert_eq!(host.view()["state"], "unavailable");
    assert_eq!(host.set_powered(true)["outcome"], "absent");
    assert_eq!(host.view()["state"], "unavailable");
}

#[test]
fn a_denied_write_keeps_the_read_state_live() {
    let mut host = BluetoothHost::new(MockBluetooth::present(data(true)));
    host.refresh();
    host.adapter_mut()
        .source_mut()
        .deny_writes("BlueZ: not authorized");
    let report = host.set_powered(false);
    assert_eq!(report["outcome"], "denied");
    assert_eq!(report["note"], "BlueZ: not authorized");
    assert_eq!(host.view()["state"], "available");
}
