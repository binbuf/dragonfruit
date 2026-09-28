// SPDX-License-Identifier: MIT
//! The Bluetooth half of the bridge host (T-15.1b).
//!
//! BlueZ is reached by the `dragonfruit-bluetooth` adapter; the shell and the
//! Settings app never link it. This module owns the one projection from the
//! typed [`BluetoothSnapshot`] to the flat JSON view the two consumers draw,
//! plus the four explicit writes the pane and tile raise (power, discovery,
//! pair, connect). It mirrors the Wi-Fi/audio/battery halves in
//! [`crate::StatusHost`], but lives in its own small host because BlueZ is on
//! the **system** bus and its adapter is not one of the three menu-bar
//! adapters (ADR 0117/0118).
//!
//! A write never invents a snapshot: the host calls
//! [`BluetoothAdapter::refresh`](dragonfruit_bluetooth::BluetoothAdapter::refresh)
//! after a successful action (or when BlueZ signals), and the read state stays
//! the single source of truth.

use dragonfruit_bluetooth::{
    BluetoothAdapter, BluetoothOutcome, BluetoothSnapshot, BluetoothSource,
};
use dragonfruit_system_adapters::Adapter;
use serde_json::{json, Value};

/// The `kind` discriminator the view carries, so one decode path can reject a
/// payload from an unexpected interface.
const KIND_BLUETOOTH: &str = "bluetooth";

/// The bridge host for the Bluetooth adapter: one state path and the explicit
/// writes the Settings pane and Control Center tile offer.
#[derive(Debug, Clone, PartialEq)]
pub struct BluetoothHost<B> {
    adapter: BluetoothAdapter<B>,
}

impl<B: BluetoothSource> BluetoothHost<B> {
    /// A host over a Bluetooth source.
    pub fn new(source: B) -> Self {
        BluetoothHost {
            adapter: BluetoothAdapter::new(source),
        }
    }

    /// Re-read BlueZ once. Called on startup, when BlueZ signals, and after an
    /// explicit action; never a poll.
    pub fn refresh(&mut self) {
        self.adapter.refresh();
    }

    /// The Bluetooth view the pane and tile render.
    pub fn view(&self) -> Value {
        bluetooth_view(&self.adapter)
    }

    /// The view as a JSON string (the D-Bus `State()` payload).
    pub fn state(&self) -> String {
        self.view().to_string()
    }

    /// Power the controller on or off. One explicit write.
    pub fn set_powered(&mut self, powered: bool) -> Value {
        bluetooth_report(self.adapter.set_powered(powered))
    }

    /// Start or stop an inquiry. One explicit write.
    pub fn set_discovering(&mut self, discovering: bool) -> Value {
        bluetooth_report(self.adapter.set_discovering(discovering))
    }

    /// Pair (bond) the device at `address`. One explicit write.
    pub fn pair(&mut self, address: &str) -> Value {
        bluetooth_report(self.adapter.pair(address))
    }

    /// Connect to or disconnect from the device at `address`. One explicit
    /// write.
    pub fn set_connected(&mut self, address: &str, connected: bool) -> Value {
        bluetooth_report(self.adapter.set_connected(address, connected))
    }

    /// The adapter (read-only), for tests and introspection.
    pub fn adapter(&self) -> &BluetoothAdapter<B> {
        &self.adapter
    }

    /// The adapter, mutably (mostly for tests that drive a mock source).
    pub fn adapter_mut(&mut self) -> &mut BluetoothAdapter<B> {
        &mut self.adapter
    }
}

/// Build the Bluetooth view from an adapter.
///
/// The three contract states map straight through: `unavailable` hides the
/// item, `error` shows it visible and inert, `available` carries the live
/// state. A controller BlueZ reports but that is unpowered is still `present`;
/// a daemon with no controller is `available` with `present: false`, and the
/// consumer hides the item then too — exactly the battery's second hide rule.
pub fn bluetooth_view<B: BluetoothSource>(adapter: &BluetoothAdapter<B>) -> Value {
    let state = adapter.state();
    if state.is_unavailable() {
        return json!({ "kind": KIND_BLUETOOTH, "state": "unavailable" });
    }
    if let Some(error) = state.error() {
        return json!({
            "kind": KIND_BLUETOOTH,
            "state": "error",
            "error": error.message(),
        });
    }
    let Some(snapshot) = state.snapshot() else {
        return json!({ "kind": KIND_BLUETOOTH, "state": "unavailable" });
    };
    bluetooth_snapshot_view(snapshot)
}

/// Build the view from an already-decoded snapshot (the pure half, so the
/// projection is unit-testable without an adapter lifecycle).
pub fn bluetooth_snapshot_view(snapshot: &BluetoothSnapshot) -> Value {
    let known: Vec<Value> = snapshot
        .known_devices()
        .iter()
        .map(|device| device_view(device))
        .collect();
    let nearby: Vec<Value> = snapshot
        .nearby_devices()
        .iter()
        .map(|device| device_view(device))
        .collect();
    let adapter = snapshot.adapter();
    json!({
        "kind": KIND_BLUETOOTH,
        "state": "available",
        "present": snapshot.present(),
        "glyph": snapshot.glyph(),
        "label": snapshot.label(),
        "powered": snapshot.powered(),
        "discovering": snapshot.discovering(),
        "discoverable": snapshot.discoverable(),
        "pairable": snapshot.pairable(),
        "adapterName": adapter.map(|a| a.display_name()).unwrap_or_default(),
        "connectedCount": snapshot.connected_count(),
        "knownCount": known.len(),
        "nearbyCount": nearby.len(),
        "knownDevices": known,
        "nearbyDevices": nearby,
    })
}

fn device_view(device: &dragonfruit_bluetooth::BluetoothDevice) -> Value {
    json!({
        "address": device.address,
        "name": device.label(),
        "paired": device.paired,
        "connected": device.connected,
        "trusted": device.trusted,
        "blocked": device.blocked,
        "rssi": device.rssi,
        "signal": device.signal_percent(),
        "icon": device.icon,
    })
}

/// The JSON report for a Bluetooth write (`accepted`/`denied`/`absent`/
/// `failed`), shaped like the Wi-Fi/audio reports so one shell decode path
/// reads every action.
pub fn bluetooth_report(outcome: BluetoothOutcome) -> Value {
    match outcome {
        BluetoothOutcome::Accepted => json!({ "outcome": "accepted" }),
        BluetoothOutcome::Denied(note) => json!({ "outcome": "denied", "note": note }),
        BluetoothOutcome::Absent => json!({ "outcome": "absent" }),
        BluetoothOutcome::Failed(error) => {
            json!({ "outcome": "failed", "error": error.message() })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dragonfruit_bluetooth::{
        BluetoothAdapterData, BluetoothData, BluetoothDeviceData, MockBluetooth,
    };

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
            devices: vec![
                BluetoothDeviceData {
                    path: "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF".to_owned(),
                    adapter_path: "/org/bluez/hci0".to_owned(),
                    address: "AA:BB:CC:DD:EE:FF".to_owned(),
                    name: "Headset".to_owned(),
                    alias: "WH-1000XM6".to_owned(),
                    paired: true,
                    connected: true,
                    trusted: true,
                    blocked: false,
                    rssi: -55,
                    icon: "audio-headset".to_owned(),
                },
                BluetoothDeviceData {
                    path: "/org/bluez/hci0/dev_11_22_33_44_55_66".to_owned(),
                    adapter_path: "/org/bluez/hci0".to_owned(),
                    address: "11:22:33:44:55:66".to_owned(),
                    name: "Speaker".to_owned(),
                    alias: "".to_owned(),
                    paired: false,
                    connected: false,
                    trusted: false,
                    blocked: false,
                    rssi: 0,
                    icon: "audio-card".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn an_absent_daemon_projects_a_hidden_slot() {
        let mut host = BluetoothHost::new(MockBluetooth::absent());
        host.refresh();
        let view = host.view();
        assert_eq!(view["kind"], "bluetooth");
        assert_eq!(view["state"], "unavailable");
    }

    #[test]
    fn a_powered_adapter_projects_known_and_nearby_devices() {
        let mut host = BluetoothHost::new(MockBluetooth::present(data(true)));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], true);
        assert_eq!(view["glyph"], "bluetooth");
        assert_eq!(view["powered"], true);
        assert_eq!(view["adapterName"], "Workstation");
        assert_eq!(view["knownCount"], 1);
        assert_eq!(view["nearbyCount"], 1);
        assert_eq!(view["knownDevices"][0]["name"], "WH-1000XM6");
        assert_eq!(view["knownDevices"][0]["connected"], true);
        assert_eq!(view["knownDevices"][0]["signal"], 64);
        assert_eq!(view["nearbyDevices"][0]["name"], "Speaker");
        // RSSI 0 is unknown, never a real reading.
        assert_eq!(view["nearbyDevices"][0]["rssi"], Value::Null);
    }

    #[test]
    fn a_daemon_with_no_controller_is_available_but_not_present() {
        let mut host = BluetoothHost::new(MockBluetooth::present(BluetoothData::default()));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "available");
        assert_eq!(view["present"], false);
        assert_eq!(view["label"], "Bluetooth unavailable");
    }

    #[test]
    fn a_read_failure_is_visible_and_inert() {
        let mut host = BluetoothHost::new(MockBluetooth::failing("BlueZ: timeout"));
        host.refresh();
        let view = host.view();
        assert_eq!(view["state"], "error");
        assert_eq!(view["error"], "BlueZ: timeout");
    }

    #[test]
    fn writes_apply_once_through_the_adapter() {
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
        // A write invents no snapshot; the pushed state is the truth.
        assert!(!host.adapter().snapshot().unwrap().powered());
    }

    #[test]
    fn a_denial_and_a_write_while_absent_are_reported() {
        let mut host = BluetoothHost::new(MockBluetooth::present(data(false)));
        host.refresh();
        host.adapter_mut()
            .source_mut()
            .deny_writes("not authorized");
        assert_eq!(host.set_powered(true)["outcome"], "denied");
        assert_eq!(host.set_powered(true)["note"], "not authorized");
        // The read state stays live after a denial.
        assert_eq!(host.view()["state"], "available");

        let mut absent = BluetoothHost::new(MockBluetooth::absent());
        assert_eq!(absent.set_powered(true)["outcome"], "absent");
        assert_eq!(absent.view()["state"], "unavailable");
    }
}
