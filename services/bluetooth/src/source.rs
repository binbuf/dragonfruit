// SPDX-License-Identifier: MIT
//! The transport seam: the raw BlueZ read and its mock.
//!
//! A [`BluetoothSource`] is the only thing that talks to the daemon. The real
//! source is the D-Bus client ([`crate::DbusBluez`]); tests and CI use
//! [`MockBluetooth`], which serves a fixture with no bus on the machine. The
//! adapter ([`crate::BluetoothAdapter`]) turns one raw read into the typed
//! [`BluetoothSnapshot`](crate::BluetoothSnapshot) and drives the shared
//! `Subscription`.
//!
//! BlueZ is reached over the **system bus** as `org.bluez`. One read is its
//! `org.freedesktop.DBus.ObjectManager.GetManagedObjects` at `/`: every
//! adapter (`org.bluez.Adapter1`) and every device (`org.bluez.Device1`) is a
//! managed object. The raw shape below flattens that into two lists — the
//! daemon's hierarchy stays in [`crate::bluez`], and nothing above the adapter
//! ever sees it.
//!
//! The write halves are explicit user actions, never polls: power the adapter,
//! start/stop discovery, pair a device, connect/disconnect a device. Each is a
//! single D-Bus method or property set. A successful write invents no snapshot;
//! BlueZ pushes the resulting `PropertiesChanged`, the host re-reads, and the
//! snapshot stays the single source of truth.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// The raw result of one BlueZ read, flattened from `GetManagedObjects`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct BluetoothData {
    /// Every `org.bluez.Adapter1` object (`hci0`, …).
    #[serde(default)]
    pub adapters: Vec<BluetoothAdapterData>,
    /// Every `org.bluez.Device1` object the daemon knows (paired and freshly
    /// discovered alike).
    #[serde(default)]
    pub devices: Vec<BluetoothDeviceData>,
}

/// One `org.bluez.Adapter1` object.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct BluetoothAdapterData {
    /// The object path, e.g. `/org/bluez/hci0`.
    pub path: String,
    /// `Address`: the controller's own MAC.
    #[serde(default)]
    pub address: String,
    /// `Name`: the controller's system name (often the hostname).
    #[serde(default)]
    pub name: String,
    /// `Alias`: the user-facing name BlueZ keeps.
    #[serde(default)]
    pub alias: String,
    /// `Powered`: the radio is on.
    #[serde(default)]
    pub powered: bool,
    /// `Discoverable`: the adapter answers inquiries.
    #[serde(default)]
    pub discoverable: bool,
    /// `Pairable`: the adapter accepts pairing.
    #[serde(default)]
    pub pairable: bool,
    /// `Discovering`: an inquiry is in progress.
    #[serde(default)]
    pub discovering: bool,
}

/// One `org.bluez.Device1` object.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct BluetoothDeviceData {
    /// The object path, e.g. `/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF`.
    pub path: String,
    /// `Adapter`: the object path of the adapter that owns this device.
    #[serde(default)]
    pub adapter_path: String,
    /// `Address`: the device MAC.
    #[serde(default)]
    pub address: String,
    /// `Name`: the device's advertised name (may be empty).
    #[serde(default)]
    pub name: String,
    /// `Alias`: BlueZ's best display name for the device.
    #[serde(default)]
    pub alias: String,
    /// `Paired`: the device is paired (bonded).
    #[serde(default)]
    pub paired: bool,
    /// `Connected`: a live connection to the device.
    #[serde(default)]
    pub connected: bool,
    /// `Trusted`: the device may connect without prompting.
    #[serde(default)]
    pub trusted: bool,
    /// `Blocked`: the device is on the block list.
    #[serde(default)]
    pub blocked: bool,
    /// `RSSI` in dBm, 0 when BlueZ has not measured it.
    #[serde(default)]
    pub rssi: i16,
    /// `Icon`: BlueZ's suggested icon name (e.g. `audio-headset`).
    #[serde(default)]
    pub icon: String,
}

/// What happened to one write (power, discovery, pair, connect).
///
/// A polkit denial is deliberately distinct from a general failure: BlueZ
/// authorizes some operations (powering the adapter, pairing) through polkit,
/// and the pane should surface the refusal without treating the daemon as
/// broken. It is never a read error — the adapter can still see the state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BluetoothOutcome {
    /// BlueZ accepted the request. The daemon pushes the resulting state; the
    /// host re-reads and the snapshot updates.
    Accepted,
    /// polkit refused the request. `note` records the daemon's message.
    Denied(String),
    /// There is no Bluetooth adapter (or the daemon is absent) to act on.
    Absent,
    /// The request failed for a reason other than authorization.
    Failed(AdapterError),
}

impl BluetoothOutcome {
    /// Whether the daemon accepted the request.
    pub fn is_accepted(&self) -> bool {
        matches!(self, BluetoothOutcome::Accepted)
    }

    /// The recorded denial note, when polkit refused.
    pub fn denial_note(&self) -> Option<&str> {
        match self {
            BluetoothOutcome::Denied(note) => Some(note),
            _ => None,
        }
    }
}

/// Reads BlueZ over some transport and drives its write methods.
///
/// The read result is a three-way answer, exactly as the adapter contract
/// needs it:
///
/// * `Ok(Some(data))` — the daemon answered; `data` is the live read.
/// * `Ok(None)` — the daemon is absent. A normal state; the slot hides.
/// * `Err(error)` — the daemon is present but the read failed; the slot shows
///   visible and inert with the message.
///
/// The source is never polled by a consumer: the host calls
/// [`BluetoothAdapter::refresh`](crate::BluetoothAdapter::refresh) when BlueZ
/// signals a change (`PropertiesChanged` / `InterfacesAdded` /
/// `InterfacesRemoved`). Each write is one explicit call.
pub trait BluetoothSource {
    /// One read of the daemon.
    fn read(&mut self) -> Result<Option<BluetoothData>, AdapterError>;

    /// Power the adapter on or off (`org.bluez.Adapter1.Powered`).
    fn set_powered(&mut self, powered: bool) -> BluetoothOutcome;

    /// Start or stop an inquiry (`StartDiscovery`/`StopDiscovery`).
    fn set_discovering(&mut self, discovering: bool) -> BluetoothOutcome;

    /// Pair (bond) the device at `address`.
    fn pair(&mut self, address: &str) -> BluetoothOutcome;

    /// Connect to or disconnect from the device at `address`.
    fn set_connected(&mut self, address: &str, connected: bool) -> BluetoothOutcome;
}

/// How the simulated daemon answers write requests.
#[derive(Debug, Clone, PartialEq, Eq)]
enum WriteBehavior {
    /// Authorized: the request is accepted (the default).
    Accept,
    /// polkit denies the request; carries the recorded note.
    Deny(String),
    /// The request fails for another reason.
    Fail(AdapterError),
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`BluetoothData`] with no bus, and
/// `kill`/`restart` exercise absence and re-subscribe the way masking the real
/// daemon would. By default every write is accepted; `deny_writes`/`fail_writes`
/// drive the refusal paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockBluetooth {
    present: bool,
    data: Option<BluetoothData>,
    failure: Option<AdapterError>,
    reads: u32,
    writes: u32,
    behavior: WriteBehavior,
}

impl MockBluetooth {
    /// A daemon that is not on the bus.
    pub fn absent() -> Self {
        MockBluetooth {
            present: false,
            data: None,
            failure: None,
            reads: 0,
            writes: 0,
            behavior: WriteBehavior::Accept,
        }
    }

    /// A present daemon that answers with `data`.
    pub fn present(data: BluetoothData) -> Self {
        MockBluetooth {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
            writes: 0,
            behavior: WriteBehavior::Accept,
        }
    }

    /// A present daemon that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockBluetooth {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
            writes: 0,
            behavior: WriteBehavior::Accept,
        }
    }

    /// Make every write come back as a polkit denial with `note`.
    pub fn deny_writes(&mut self, note: impl Into<String>) {
        self.behavior = WriteBehavior::Deny(note.into());
    }

    /// Make every write fail with `message` (not authorization).
    pub fn fail_writes(&mut self, message: impl Into<String>) {
        self.behavior = WriteBehavior::Fail(AdapterError::new(message));
    }

    /// Restore the default authorized behavior.
    pub fn allow_writes(&mut self) {
        self.behavior = WriteBehavior::Accept;
    }

    /// The daemon pushes fresh data.
    pub fn push(&mut self, data: BluetoothData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The daemon goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The daemon comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated daemon is on the bus.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many write requests the source has served. Lets a test prove a write
    /// is one explicit call, never a loop.
    pub fn writes(&self) -> u32 {
        self.writes
    }

    fn write_outcome(&mut self) -> BluetoothOutcome {
        self.writes += 1;
        if !self.present {
            return BluetoothOutcome::Absent;
        }
        // A daemon that cannot answer a read cannot answer a write either.
        if let Some(error) = &self.failure {
            return BluetoothOutcome::Failed(error.clone());
        }
        match &self.behavior {
            WriteBehavior::Accept => BluetoothOutcome::Accepted,
            WriteBehavior::Deny(note) => BluetoothOutcome::Denied(note.clone()),
            WriteBehavior::Fail(error) => BluetoothOutcome::Failed(error.clone()),
        }
    }
}

impl BluetoothSource for MockBluetooth {
    fn read(&mut self) -> Result<Option<BluetoothData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }

    fn set_powered(&mut self, _powered: bool) -> BluetoothOutcome {
        self.write_outcome()
    }

    fn set_discovering(&mut self, _discovering: bool) -> BluetoothOutcome {
        self.write_outcome()
    }

    fn pair(&mut self, _address: &str) -> BluetoothOutcome {
        self.write_outcome()
    }

    fn set_connected(&mut self, _address: &str, _connected: bool) -> BluetoothOutcome {
        self.write_outcome()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> BluetoothData {
        BluetoothData {
            adapters: vec![BluetoothAdapterData {
                path: "/org/bluez/hci0".to_owned(),
                powered: true,
                ..BluetoothAdapterData::default()
            }],
            devices: vec![],
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockBluetooth::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockBluetooth::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockBluetooth::failing("BlueZ: timeout");
        assert_eq!(mock.read().unwrap_err().message(), "BlueZ: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockBluetooth::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn writes_are_accepted_by_default_and_counted() {
        let mut mock = MockBluetooth::present(data());
        assert!(mock.set_powered(true).is_accepted());
        assert!(mock.set_discovering(true).is_accepted());
        assert!(mock.pair("AA:BB:CC:DD:EE:FF").is_accepted());
        assert!(mock.set_connected("AA:BB:CC:DD:EE:FF", true).is_accepted());
        assert_eq!(mock.writes(), 4);
    }

    #[test]
    fn a_denying_mock_reports_the_note() {
        let mut mock = MockBluetooth::present(data());
        mock.deny_writes("BlueZ: not authorized");
        let outcome = mock.set_powered(true);
        assert_eq!(
            outcome,
            BluetoothOutcome::Denied("BlueZ: not authorized".to_owned())
        );
        assert_eq!(outcome.denial_note(), Some("BlueZ: not authorized"));
    }

    #[test]
    fn a_failing_write_is_a_failure_not_a_denial() {
        let mut mock = MockBluetooth::present(data());
        mock.fail_writes("BlueZ: unknown device");
        assert_eq!(
            mock.pair("AA:BB:CC:DD:EE:FF"),
            BluetoothOutcome::Failed(AdapterError::new("BlueZ: unknown device"))
        );
    }

    #[test]
    fn a_write_while_absent_is_absent() {
        let mut mock = MockBluetooth::absent();
        assert_eq!(mock.set_powered(true), BluetoothOutcome::Absent);
    }
}
