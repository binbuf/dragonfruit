// SPDX-License-Identifier: MIT
//! The transport seam: the raw NetworkManager read and its mock.
//!
//! A [`NetworkManagerSource`] is the only thing that talks to the daemon. The
//! real source is the D-Bus client ([`crate::DbusNetworkManager`]); tests and
//! CI use [`MockNetworkManager`], which serves a fixture with no bus on the
//! machine. The adapter ([`crate::NetworkManagerAdapter`]) turns one raw read
//! into the typed snapshot and drives the shared `Subscription`.
//!
//! Decoding stops here: an SSID is already lossy-decoded to text and device/AP
//! states stay the numeric NetworkManager enums, which [`crate::model`] maps.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

/// The result of one NetworkManager read, shaped after its D-Bus objects.
///
/// This is deliberately flat and daemon-shaped: the D-Bus source fills it from
/// proxies, a fixture deserialises straight into it, and nothing above the
/// adapter ever sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct NetworkManagerData {
    /// `org.freedesktop.NetworkManager.WirelessEnabled`.
    pub wireless_enabled: bool,
    /// `org.freedesktop.NetworkManager.Connectivity` (`NMConnectivityState`).
    pub connectivity: u32,
    /// Every device NetworkManager currently manages, all types; the model
    /// keeps only the Wi-Fi ones.
    #[serde(default)]
    pub devices: Vec<WifiDeviceData>,
}

/// One NetworkManager device (`org.freedesktop.NetworkManager.Device` plus its
/// `Device.Wireless` half).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct WifiDeviceData {
    /// `Managed`: an unmanaged device is present but not ours to report.
    pub managed: bool,
    /// `State` (`NMDeviceState`).
    pub state: u32,
    /// `Device.Wireless.ActiveAccessPoint`, `None` when disconnected.
    #[serde(default)]
    pub active_ap: Option<String>,
    /// `Device.Wireless.AccessPoints`, every BSSID the device can see.
    #[serde(default)]
    pub access_points: Vec<AccessPointData>,
}

/// One access point (`org.freedesktop.NetworkManager.AccessPoint`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct AccessPointData {
    /// The AP's object path; the active AP is matched by path, because two
    /// BSSIDs of the same SSID are still different objects.
    pub path: String,
    /// `Ssid`, decoded from its `ay` bytes.
    pub ssid: String,
    /// `Strength`, 0–100.
    pub strength: u8,
    /// `Frequency` in MHz.
    pub frequency_mhz: u32,
    /// `Flags` (bit 0 is `PRIVACY`).
    #[serde(default)]
    pub flags: u32,
    /// `WpaFlags` (`NM80211ApSecurityFlags`).
    #[serde(default)]
    pub wpa_flags: u32,
    /// `RsnFlags` (`NM80211ApSecurityFlags`).
    #[serde(default)]
    pub rsn_flags: u32,
}

/// Reads NetworkManager over some transport.
///
/// The result is a three-way answer, exactly as the adapter contract needs it:
///
/// * `Ok(Some(data))` — the daemon answered; `data` is the live read.
/// * `Ok(None)` — the daemon is absent. A normal state; the slot hides.
/// * `Err(error)` — the daemon is present but the read failed; the slot shows
///   visible and inert with the message.
///
/// The source is never polled by a consumer: the host calls
/// [`NetworkManagerAdapter::refresh`](crate::NetworkManagerAdapter::refresh)
/// when the daemon signals a change.
pub trait NetworkManagerSource {
    /// One read of the daemon.
    fn read(&mut self) -> Result<Option<NetworkManagerData>, AdapterError>;
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`NetworkManagerData`] with no bus, and
/// `kill`/`restart` exercise absence and re-subscribe the way masking the real
/// daemon would.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockNetworkManager {
    present: bool,
    data: Option<NetworkManagerData>,
    failure: Option<AdapterError>,
    reads: u32,
}

impl MockNetworkManager {
    /// A daemon that is not on the bus.
    pub fn absent() -> Self {
        MockNetworkManager {
            present: false,
            data: None,
            failure: None,
            reads: 0,
        }
    }

    /// A present daemon that answers with `data`.
    pub fn present(data: NetworkManagerData) -> Self {
        MockNetworkManager {
            present: true,
            data: Some(data),
            failure: None,
            reads: 0,
        }
    }

    /// A present daemon that fails every read (e.g. it went unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockNetworkManager {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            reads: 0,
        }
    }

    /// The daemon pushes fresh data.
    pub fn push(&mut self, data: NetworkManagerData) {
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
}

impl NetworkManagerSource for MockNetworkManager {
    fn read(&mut self) -> Result<Option<NetworkManagerData>, AdapterError> {
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

    #[test]
    fn an_absent_mock_reads_as_absent() {
        let mut mock = MockNetworkManager::absent();
        assert_eq!(mock.read(), Ok(None));
        assert_eq!(mock.reads(), 1);
    }

    #[test]
    fn a_failing_mock_is_present_but_errors() {
        let mut mock = MockNetworkManager::failing("NetworkManager: timeout");
        let error = mock.read().unwrap_err();
        assert_eq!(error.message(), "NetworkManager: timeout");
        assert!(mock.is_present());
    }

    #[test]
    fn kill_and_restart_flip_presence() {
        let mut mock = MockNetworkManager::present(NetworkManagerData::default());
        assert_eq!(mock.read().unwrap(), Some(NetworkManagerData::default()));
        mock.kill();
        assert_eq!(mock.read(), Ok(None));
        mock.restart();
        assert_eq!(mock.read().unwrap(), Some(NetworkManagerData::default()));
    }
}
