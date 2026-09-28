// SPDX-License-Identifier: MIT
//! The Bluetooth snapshot the pane and status slot render, decoded from one
//! raw read.
//!
//! The model owns the aggregation a consumer should not repeat: it picks the
//! adapter, keeps only the devices that belong to it, orders them usefully,
//! and derives the glyph, label, and signal fill the UI draws. It maps no
//! BlueZ enums — BlueZ reports booleans and strings directly — so it is a
//! straight projection of the raw objects with a stable ordering.
//!
//! A Bluetooth controller that BlueZ reports but that is `Powered = false` is
//! still a present adapter: the pane shows it with an Off switch. Only *no
//! adapter at all* (no hardware, or `bluetoothd` gone) makes
//! [`BluetoothSnapshot::present`] false, which is the second hide rule beside
//! the adapter-level `Unavailable`.

use crate::source::{BluetoothAdapterData, BluetoothData, BluetoothDeviceData};

/// The Bluetooth controller the session reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothAdapter {
    /// The object path, e.g. `/org/bluez/hci0`.
    pub path: String,
    /// The controller's own MAC.
    pub address: String,
    /// The controller's system name.
    pub name: String,
    /// BlueZ's user-facing alias.
    pub alias: String,
    /// `Powered`: the radio is on.
    pub powered: bool,
    /// `Discoverable`: the controller answers inquiries.
    pub discoverable: bool,
    /// `Pairable`: the controller accepts pairing.
    pub pairable: bool,
    /// `Discovering`: an inquiry is in progress.
    pub discovering: bool,
}

impl BluetoothAdapter {
    fn from_data(data: &BluetoothAdapterData) -> Self {
        BluetoothAdapter {
            path: data.path.clone(),
            address: data.address.clone(),
            name: data.name.clone(),
            alias: data.alias.clone(),
            powered: data.powered,
            discoverable: data.discoverable,
            pairable: data.pairable,
            discovering: data.discovering,
        }
    }

    /// The name to show: the alias, else the system name, else the address.
    pub fn display_name(&self) -> &str {
        first_non_empty(&[&self.alias, &self.name, &self.address])
    }
}

/// One Bluetooth device the adapter knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BluetoothDevice {
    /// The object path BlueZ serves the device at.
    pub path: String,
    /// The owning adapter's object path.
    pub adapter_path: String,
    /// The device MAC.
    pub address: String,
    /// The device's advertised name (may be empty).
    pub name: String,
    /// BlueZ's best display name.
    pub alias: String,
    /// Whether the device is paired (bonded).
    pub paired: bool,
    /// Whether a live connection exists.
    pub connected: bool,
    /// Whether the device may connect without prompting.
    pub trusted: bool,
    /// Whether the device is on the block list.
    pub blocked: bool,
    /// The last measured RSSI in dBm; `None` when BlueZ has not measured it.
    pub rssi: Option<i16>,
    /// BlueZ's suggested icon name.
    pub icon: String,
}

impl BluetoothDevice {
    fn from_data(data: &BluetoothDeviceData) -> Self {
        BluetoothDevice {
            path: data.path.clone(),
            adapter_path: data.adapter_path.clone(),
            address: data.address.clone(),
            name: data.name.clone(),
            alias: data.alias.clone(),
            paired: data.paired,
            connected: data.connected,
            trusted: data.trusted,
            blocked: data.blocked,
            // BlueZ reports 0 when it has no measurement; keep that as unknown
            // rather than a real 0 dBm reading.
            rssi: (data.rssi != 0).then_some(data.rssi),
            icon: data.icon.clone(),
        }
    }

    /// The name to show: the alias, else the advertised name, else the address.
    pub fn label(&self) -> &str {
        first_non_empty(&[&self.alias, &self.name, &self.address])
    }

    /// Whether the device is paired or connected (a device the pane lists as
    /// known rather than merely discovered).
    pub fn is_known(&self) -> bool {
        self.paired || self.connected
    }

    /// Whether the device is connected right now.
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Whether the device is paired.
    pub fn is_paired(&self) -> bool {
        self.paired
    }

    /// The signal as a 0–100 fill, when BlueZ reports an RSSI.
    ///
    /// The dBm-to-percent curve is the conventional one: about -30 dBm (or
    /// better) is full and -100 dBm or worse is empty, clamped between.
    pub fn signal_percent(&self) -> Option<u8> {
        self.rssi.map(|rssi| {
            let clamped = rssi.clamp(-100, -30);
            // -30 → 100%, -100 → 0%.
            (((clamped + 100) as f32 / 70.0) * 100.0).round() as u8
        })
    }
}

/// The Bluetooth snapshot a pane or status slot renders.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BluetoothSnapshot {
    /// The controller BlueZ reports, when one exists.
    pub adapter: Option<BluetoothAdapter>,
    /// Every device on the controller, connected and paired first.
    pub devices: Vec<BluetoothDevice>,
}

impl BluetoothSnapshot {
    /// Build the snapshot from one raw read: pick the first adapter and keep
    /// the devices that belong to it, ordered connected, paired, then name.
    pub fn from_data(data: &BluetoothData) -> Self {
        let adapter = data.adapters.first().map(BluetoothAdapter::from_data);
        let adapter_path = adapter.as_ref().map(|adapter| adapter.path.as_str());

        let mut devices: Vec<BluetoothDevice> = data
            .devices
            .iter()
            // Only the chosen adapter's devices; an adapter-less device (no
            // controller) is not something a pane can act on.
            .filter(|device| {
                adapter_path
                    .map(|path| device.adapter_path == path)
                    .unwrap_or(false)
            })
            .map(BluetoothDevice::from_data)
            .collect();
        devices.sort_by(|left, right| {
            right
                .connected
                .cmp(&left.connected)
                .then_with(|| right.paired.cmp(&left.paired))
                .then_with(|| left.label().cmp(right.label()))
                .then_with(|| left.address.cmp(&right.address))
        });

        BluetoothSnapshot { adapter, devices }
    }

    /// The controller, when BlueZ reports one.
    pub fn adapter(&self) -> Option<&BluetoothAdapter> {
        self.adapter.as_ref()
    }

    /// Whether there is a controller to report. The item hides when there is
    /// not, even if `bluetoothd` is present.
    pub fn present(&self) -> bool {
        self.adapter.is_some()
    }

    /// Whether the radio is on.
    pub fn powered(&self) -> bool {
        self.adapter.as_ref().is_some_and(|adapter| adapter.powered)
    }

    /// Whether an inquiry is in progress.
    pub fn discovering(&self) -> bool {
        self.adapter
            .as_ref()
            .is_some_and(|adapter| adapter.discovering)
    }

    /// Whether the controller answers inquiries.
    pub fn discoverable(&self) -> bool {
        self.adapter
            .as_ref()
            .is_some_and(|adapter| adapter.discoverable)
    }

    /// Whether the controller accepts pairing.
    pub fn pairable(&self) -> bool {
        self.adapter
            .as_ref()
            .is_some_and(|adapter| adapter.pairable)
    }

    /// Every device on the controller.
    pub fn devices(&self) -> &[BluetoothDevice] {
        &self.devices
    }

    /// The paired or connected devices ("known devices").
    pub fn known_devices(&self) -> Vec<&BluetoothDevice> {
        self.devices
            .iter()
            .filter(|device| device.is_known())
            .collect()
    }

    /// The discovered but not-yet-known devices ("nearby devices").
    pub fn nearby_devices(&self) -> Vec<&BluetoothDevice> {
        self.devices
            .iter()
            .filter(|device| !device.is_known())
            .collect()
    }

    /// How many devices are connected right now.
    pub fn connected_count(&self) -> usize {
        self.devices
            .iter()
            .filter(|device| device.is_connected())
            .count()
    }

    /// The device with `address`, when it is on this controller.
    pub fn device_by_address(&self, address: &str) -> Option<&BluetoothDevice> {
        self.devices.iter().find(|device| device.address == address)
    }

    /// The glyph the status slot draws.
    pub fn glyph(&self) -> &'static str {
        if self.powered() {
            "bluetooth"
        } else {
            "bluetooth-disabled"
        }
    }

    /// A one-line label for the status item / pane header.
    pub fn label(&self) -> String {
        if !self.present() {
            return "Bluetooth unavailable".to_owned();
        }
        if self.discovering() {
            return "Bluetooth discovering".to_owned();
        }
        if self.powered() {
            let connected = self.connected_count();
            if connected > 0 {
                return format!("Bluetooth connected ({connected})");
            }
            return "Bluetooth on".to_owned();
        }
        "Bluetooth off".to_owned()
    }
}

/// The first non-empty string in `candidates`, or `""`.
fn first_non_empty<'a>(candidates: &[&'a str]) -> &'a str {
    candidates
        .iter()
        .copied()
        .find(|candidate| !candidate.is_empty())
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(powered: bool) -> BluetoothAdapterData {
        BluetoothAdapterData {
            path: "/org/bluez/hci0".to_owned(),
            address: "11:22:33:44:55:66".to_owned(),
            name: "workstation".to_owned(),
            alias: "Workstation".to_owned(),
            powered,
            discoverable: false,
            pairable: true,
            discovering: false,
        }
    }

    fn device(address: &str, _powered: bool, paired: bool, connected: bool) -> BluetoothDeviceData {
        BluetoothDeviceData {
            path: format!("/org/bluez/hci0/dev_{}", address.replace(':', "_")),
            adapter_path: "/org/bluez/hci0".to_owned(),
            address: address.to_owned(),
            name: "Headset".to_owned(),
            alias: "WH-1000".to_owned(),
            paired,
            connected,
            trusted: paired,
            blocked: false,
            rssi: -55,
            icon: "audio-headset".to_owned(),
        }
    }

    #[test]
    fn an_empty_read_has_no_adapter() {
        let snapshot = BluetoothSnapshot::from_data(&BluetoothData::default());
        assert!(!snapshot.present());
        assert_eq!(snapshot.glyph(), "bluetooth-disabled");
        assert_eq!(snapshot.label(), "Bluetooth unavailable");
        assert!(!snapshot.powered());
    }

    #[test]
    fn a_powered_adapter_reports_its_glyph_and_label() {
        let data = BluetoothData {
            adapters: vec![adapter(true)],
            devices: vec![],
        };
        let snapshot = BluetoothSnapshot::from_data(&data);
        assert!(snapshot.present());
        assert!(snapshot.powered());
        assert!(snapshot.pairable());
        assert_eq!(snapshot.glyph(), "bluetooth");
        assert_eq!(snapshot.label(), "Bluetooth on");
        assert_eq!(snapshot.adapter().unwrap().display_name(), "Workstation");
    }

    #[test]
    fn an_unpowered_adapter_is_present_but_off() {
        let data = BluetoothData {
            adapters: vec![adapter(false)],
            devices: vec![device("AA:BB:CC:DD:EE:FF", false, true, false)],
        };
        let snapshot = BluetoothSnapshot::from_data(&data);
        assert!(snapshot.present());
        assert!(!snapshot.powered());
        assert_eq!(snapshot.glyph(), "bluetooth-disabled");
        assert_eq!(snapshot.label(), "Bluetooth off");
    }

    #[test]
    fn discovering_wins_the_label() {
        let mut data = adapter(true);
        data.discovering = true;
        let snapshot = BluetoothSnapshot::from_data(&BluetoothData {
            adapters: vec![data],
            devices: vec![],
        });
        assert!(snapshot.discovering());
        assert_eq!(snapshot.label(), "Bluetooth discovering");
    }

    #[test]
    fn devices_are_filtered_to_the_chosen_adapter_and_ordered() {
        let other = BluetoothDeviceData {
            path: "/org/bluez/hci1/dev_11_11_11_11_11_11".to_owned(),
            adapter_path: "/org/bluez/hci1".to_owned(),
            address: "11:11:11:11:11:11".to_owned(),
            ..BluetoothDeviceData::default()
        };
        let data = BluetoothData {
            adapters: vec![adapter(true)],
            devices: vec![
                device("BB:BB:BB:BB:BB:BB", true, false, false),
                device("CC:CC:CC:CC:CC:CC", true, true, true),
                other,
            ],
        };
        let snapshot = BluetoothSnapshot::from_data(&data);
        assert_eq!(snapshot.devices().len(), 2);
        // Connected first, then the merely discovered one.
        assert_eq!(snapshot.devices()[0].address, "CC:CC:CC:CC:CC:CC");
        assert!(snapshot.devices()[0].is_connected());
        assert_eq!(snapshot.connected_count(), 1);
        assert_eq!(snapshot.known_devices().len(), 1);
        assert_eq!(
            snapshot
                .device_by_address("BB:BB:BB:BB:BB:BB")
                .unwrap()
                .label(),
            "WH-1000"
        );
    }

    #[test]
    fn a_device_without_an_rssi_has_no_signal_fill() {
        let mut raw = device("AA:BB:CC:DD:EE:FF", true, true, false);
        raw.rssi = 0;
        let snapshot = BluetoothSnapshot::from_data(&BluetoothData {
            adapters: vec![adapter(true)],
            devices: vec![raw],
        });
        assert_eq!(snapshot.devices()[0].rssi, None);
        assert_eq!(snapshot.devices()[0].signal_percent(), None);
    }

    #[test]
    fn the_signal_fill_clamps_the_rssi_curve() {
        let mut strong = device("AA:BB:CC:DD:EE:FF", true, true, true);
        strong.rssi = -30;
        let mut weak = device("BB:BB:BB:BB:BB:BB", true, false, false);
        weak.rssi = -120;
        let snapshot = BluetoothSnapshot::from_data(&BluetoothData {
            adapters: vec![adapter(true)],
            devices: vec![strong, weak],
        });
        let connected = snapshot.device_by_address("AA:BB:CC:DD:EE:FF").unwrap();
        assert_eq!(connected.signal_percent(), Some(100));
        let far = snapshot.device_by_address("BB:BB:BB:BB:BB:BB").unwrap();
        assert_eq!(far.signal_percent(), Some(0));
    }
}
