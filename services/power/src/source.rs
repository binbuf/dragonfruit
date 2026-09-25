// SPDX-License-Identifier: MIT
//! The transport seam: the raw UPower read and its mock.
//!
//! A [`PowerSource`] is the only thing that talks to the daemon. The real
//! source is the D-Bus client ([`crate::DbusUPower`]); tests and CI use
//! [`MockPower`], which serves a fixture with no bus on the machine. The
//! adapter ([`crate::PowerAdapter`]) turns one raw read into the typed
//! [`PowerSnapshot`](crate::PowerSnapshot) and drives the shared
//! `Subscription`.
//!
//! The read is read-only: the menu bar's battery item never writes, so this
//! seam has no write half (power profiles are deferred to T-15). The raw
//! shape is deliberately flat and daemon-shaped: the D-Bus source fills it
//! from proxies, a fixture deserialises straight into it, and nothing above
//! the adapter ever sees it.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// `UPowerDeviceType` for a battery (`UP_DEVICE_KIND_BATTERY`).
pub const DEVICE_TYPE_BATTERY: u32 = 2;

/// The result of one UPower read, shaped after its D-Bus objects.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct PowerData {
    /// `org.freedesktop.UPower.OnBattery`: the system is running on battery
    /// rather than on line power.
    #[serde(default)]
    pub on_battery: bool,
    /// Every device `EnumerateDevices` returned, all types; the model keeps
    /// only present batteries.
    #[serde(default)]
    pub devices: Vec<PowerDeviceData>,
}

/// One UPower device (`org.freedesktop.UPower.Device`).
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct PowerDeviceData {
    /// The device's object path.
    pub path: String,
    /// `Type` (`UPowerDeviceType`); [`DEVICE_TYPE_BATTERY`] is a battery.
    #[serde(default)]
    pub kind: u32,
    /// `IsPresent`: a battery bay with no cell is not a usable battery.
    #[serde(default)]
    pub present: bool,
    /// `PowerSupply`.
    #[serde(default)]
    pub power_supply: bool,
    /// `Percentage`, 0–100.
    #[serde(default)]
    pub percentage: f64,
    /// `State` (`UPowerDeviceState`).
    #[serde(default)]
    pub state: u32,
    /// `BatteryLevel` (`UPowerBatteryLevel`).
    #[serde(default)]
    pub battery_level: u32,
    /// `TimeToEmpty` in seconds; 0 when unknown.
    #[serde(default)]
    pub time_to_empty: i64,
    /// `TimeToFull` in seconds; 0 when unknown.
    #[serde(default)]
    pub time_to_full: i64,
}

/// Reads power state over some transport.
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
/// [`PowerAdapter::refresh`](crate::PowerAdapter::refresh) when UPower signals
/// a change. There is no write half — the battery item is read-only.
pub trait PowerSource {
    /// One read of the daemon.
    fn read(&mut self) -> Result<Option<PowerData>, AdapterError>;
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`PowerData`] with no UPower, and
/// `kill`/`restart` exercise absence and re-subscribe the way masking the real
/// daemon would. Unlike the audio/network mocks there are no writes to apply —
/// power is read-only here.
#[derive(Debug, Clone, PartialEq)]
pub struct MockPower {
    present: bool,
    data: Option<PowerData>,
    failure: Option<AdapterError>,
    reads: u32,
}

impl MockPower {
    /// A daemon that is not running.
    pub fn absent() -> Self {
        MockPower {
            present: false,
            data: None,
            failure: None,
            reads: 0,
        }
    }

    /// A running daemon that answers with `data`.
    pub fn present(data: PowerData) -> Self {
        MockPower {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
        }
    }

    /// A running daemon that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockPower {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
        }
    }

    /// The daemon pushes fresh data.
    pub fn push(&mut self, data: PowerData) {
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

    /// Whether the simulated daemon is running.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }
}

impl PowerSource for MockPower {
    fn read(&mut self) -> Result<Option<PowerData>, AdapterError> {
        self.reads += 1;
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if !self.present {
            return Ok(None);
        }
        Ok(Some(self.data.clone().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> PowerData {
        PowerData {
            on_battery: true,
            devices: vec![PowerDeviceData {
                path: "/org/freedesktop/UPower/devices/battery_BAT0".to_owned(),
                kind: DEVICE_TYPE_BATTERY,
                present: true,
                power_supply: true,
                percentage: 42.0,
                state: 2,
                battery_level: 4,
                time_to_empty: 3600,
                time_to_full: 0,
            }],
        }
    }

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockPower::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_present_mock_serves_its_data() {
        let mut mock = MockPower::present(data());
        assert_eq!(mock.read(), Ok(Some(data())));
        assert!(mock.is_present());
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockPower::failing("UPower: timeout");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "UPower: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockPower::present(data());
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(data()));
    }

    #[test]
    fn push_replaces_the_served_data_and_clears_a_failure() {
        let mut mock = MockPower::failing("UPower: timeout");
        mock.push(PowerData::default());
        assert_eq!(mock.read(), Ok(Some(PowerData::default())));
    }
}
