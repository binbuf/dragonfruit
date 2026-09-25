// SPDX-License-Identifier: MIT
//! The Wi-Fi snapshot the menu bar renders, decoded from one raw read.
//!
//! The model owns the NetworkManager enum mapping and the aggregation a
//! consumer should not repeat: it collapses the device list to one Wi-Fi
//! state, dedupes BSSIDs down to one entry per SSID, sorts by signal, and
//! marks the active network. T-07.5 renders [`WifiSnapshot::glyph`] /
//! [`WifiSnapshot::label`] and the [`WifiSnapshot::access_points`] list.

use crate::source::{NetworkManagerData, WifiDeviceData};

/// The aggregate Wi-Fi state, mapped from `NMDeviceState`.
///
/// The intermediate `NMDeviceState` values (Prepare/Config/NeedAuth/IpConfig/
/// IpCheck/Secondaries) all mean "working on it" and collapse to
/// [`WifiState::Connecting`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WifiState {
    /// No device has reported a state yet.
    #[default]
    Unknown,
    /// The device is not managed by NetworkManager.
    Unmanaged,
    /// The device exists but cannot be used (e.g. rfkill).
    Unavailable,
    /// The device is idle and not connected.
    Disconnected,
    /// An activation is in progress.
    Connecting,
    /// The device is activated.
    Connected,
    /// A deactivation is in progress.
    Disconnecting,
    /// The last activation failed.
    Failed,
}

impl WifiState {
    /// Map the numeric `NMDeviceState`.
    pub const fn from_u32(value: u32) -> Self {
        match value {
            10 => WifiState::Unmanaged,
            20 => WifiState::Unavailable,
            30 => WifiState::Disconnected,
            40..=90 => WifiState::Connecting,
            100 => WifiState::Connected,
            110 => WifiState::Disconnecting,
            120 => WifiState::Failed,
            _ => WifiState::Unknown,
        }
    }

    /// Whether the device is activated.
    pub const fn is_connected(self) -> bool {
        matches!(self, WifiState::Connected)
    }

    /// Whether an activation/deactivation is in flight.
    pub const fn is_transitional(self) -> bool {
        matches!(self, WifiState::Connecting | WifiState::Disconnecting)
    }

    /// A stable label for logs.
    pub const fn name(self) -> &'static str {
        match self {
            WifiState::Unknown => "unknown",
            WifiState::Unmanaged => "unmanaged",
            WifiState::Unavailable => "unavailable",
            WifiState::Disconnected => "disconnected",
            WifiState::Connecting => "connecting",
            WifiState::Connected => "connected",
            WifiState::Disconnecting => "disconnecting",
            WifiState::Failed => "failed",
        }
    }

    /// How "connected" this state is, so devices can be aggregated: a
    /// connected device wins over a connecting one, which wins over idle.
    const fn rank(self) -> u8 {
        match self {
            WifiState::Connected => 8,
            WifiState::Connecting => 7,
            WifiState::Disconnecting => 6,
            WifiState::Disconnected => 5,
            WifiState::Unavailable => 4,
            WifiState::Unmanaged => 3,
            WifiState::Failed => 2,
            WifiState::Unknown => 1,
        }
    }
}

/// `NMConnectivityState`: how far the system can actually reach the internet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Connectivity {
    /// NetworkManager has not checked yet.
    #[default]
    Unknown,
    /// Fully disconnected.
    None,
    /// Behind a captive portal.
    Portal,
    /// Reachable but limited.
    Limited,
    /// Full internet access.
    Full,
}

impl Connectivity {
    /// Map the numeric `NMConnectivityState`.
    pub const fn from_u32(value: u32) -> Self {
        match value {
            1 => Connectivity::None,
            2 => Connectivity::Portal,
            3 => Connectivity::Limited,
            4 => Connectivity::Full,
            _ => Connectivity::Unknown,
        }
    }

    /// A stable label for the menu.
    pub const fn label(self) -> &'static str {
        match self {
            Connectivity::Unknown => "Unknown",
            Connectivity::None => "No Internet",
            Connectivity::Portal => "Sign in to network",
            Connectivity::Limited => "Limited",
            Connectivity::Full => "Connected",
        }
    }
}

/// The security an access point advertises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Security {
    /// No privacy bit and no WPA/RSN flags.
    Open,
    /// Privacy bit set with no WPA/RSN flags: a legacy WEP network.
    Wep,
    /// WPA (TKIP) only.
    Wpa,
    /// WPA2 personal (RSN, PSK/CCMP/TKIP).
    Wpa2,
    /// WPA3 personal (SAE).
    Wpa3,
    /// WPA2/WPA3 enterprise (802.1X).
    Enterprise,
}

impl Security {
    /// Decode the AP `Flags`, `WpaFlags`, and `RsnFlags`.
    pub const fn from_flags(flags: u32, wpa_flags: u32, rsn_flags: u32) -> Self {
        const PRIVACY: u32 = 0x1;
        const KEY_MGMT_802_1X: u32 = 0x200;
        const KEY_MGMT_SAE: u32 = 0x400;
        if rsn_flags & KEY_MGMT_SAE != 0 {
            return Security::Wpa3;
        }
        if rsn_flags & KEY_MGMT_802_1X != 0 {
            return Security::Enterprise;
        }
        if rsn_flags != 0 {
            return Security::Wpa2;
        }
        if wpa_flags != 0 {
            return Security::Wpa;
        }
        if flags & PRIVACY != 0 {
            return Security::Wep;
        }
        Security::Open
    }

    /// Whether joining needs a secret (or an enterprise login).
    pub const fn is_secured(self) -> bool {
        !matches!(self, Security::Open)
    }

    /// A short label for the network list.
    pub const fn label(self) -> &'static str {
        match self {
            Security::Open => "Open",
            Security::Wep => "WEP",
            Security::Wpa => "WPA",
            Security::Wpa2 => "WPA2",
            Security::Wpa3 => "WPA3",
            Security::Enterprise => "Enterprise",
        }
    }
}

/// One network the menu bar lists: one entry per SSID, strongest BSSID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    /// The SSID, decoded to text (a hidden network renders as empty).
    pub ssid: String,
    /// Signal strength, 0–100.
    pub strength: u8,
    /// Frequency in MHz.
    pub frequency_mhz: u32,
    /// The security the strongest BSSID advertises.
    pub security: Security,
    /// Whether this is the SSID the device is currently connected to.
    pub active: bool,
}

impl AccessPoint {
    /// The band the frequency falls in, for grouping/labels.
    pub const fn band(&self) -> Band {
        Band::from_frequency_mhz(self.frequency_mhz)
    }
}

/// The Wi-Fi band an access point is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// 2.4 GHz (channel 1–14).
    Ghz2,
    /// 5 GHz.
    Ghz5,
    /// 6 GHz (Wi-Fi 6E).
    Ghz6,
    /// A frequency we do not recognise.
    Unknown,
}

impl Band {
    /// Classify a frequency in MHz.
    pub const fn from_frequency_mhz(frequency_mhz: u32) -> Self {
        match frequency_mhz {
            2400..=2500 => Band::Ghz2,
            5150..=5895 => Band::Ghz5,
            5925..=7125 => Band::Ghz6,
            _ => Band::Unknown,
        }
    }

    /// A short label for the network list.
    pub const fn label(self) -> &'static str {
        match self {
            Band::Ghz2 => "2.4 GHz",
            Band::Ghz5 => "5 GHz",
            Band::Ghz6 => "6 GHz",
            Band::Unknown => "Unknown",
        }
    }
}

/// The Wi-Fi snapshot a menu bar renders: the manager state, the active
/// network, and the visible network list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WifiSnapshot {
    /// Whether the radio is enabled.
    pub enabled: bool,
    /// How far the system can reach the internet.
    pub connectivity: Connectivity,
    /// The aggregate device state.
    pub state: WifiState,
    /// One entry per visible SSID, strongest first.
    pub access_points: Vec<AccessPoint>,
    /// The SSID the device is connected to, when connected.
    pub active_ssid: Option<String>,
}

impl WifiSnapshot {
    /// Build the snapshot from one raw read.
    ///
    /// Unmanaged devices are ignored; the states of the remaining Wi-Fi
    /// devices are aggregated by "most connected"; BSSIDs are collapsed to one
    /// entry per SSID (the strongest wins, and the SSID is active if any BSSID
    /// of it is the active access point).
    pub fn from_data(data: &NetworkManagerData) -> Self {
        let devices: Vec<&WifiDeviceData> = data.devices.iter().filter(|d| d.managed).collect();

        let state = devices
            .iter()
            .map(|d| WifiState::from_u32(d.state))
            .max_by_key(|s| s.rank())
            .unwrap_or(WifiState::Unknown);

        let active_paths: Vec<&str> = devices
            .iter()
            .filter_map(|d| d.active_ap.as_deref())
            .collect();

        let mut access_points: Vec<AccessPoint> = Vec::new();
        for device in &devices {
            for ap in &device.access_points {
                let candidate = AccessPoint {
                    ssid: ap.ssid.clone(),
                    strength: ap.strength,
                    frequency_mhz: ap.frequency_mhz,
                    security: Security::from_flags(ap.flags, ap.wpa_flags, ap.rsn_flags),
                    active: active_paths.contains(&ap.path.as_str()),
                };
                match access_points.iter_mut().find(|e| e.ssid == candidate.ssid) {
                    // Keep the strongest BSSID of an SSID; the active flag wins
                    // even if a neighbour BSSID is momentarily stronger.
                    Some(existing) => {
                        if candidate.active {
                            existing.active = true;
                            existing.strength = candidate.strength;
                            existing.frequency_mhz = candidate.frequency_mhz;
                            existing.security = candidate.security;
                        } else if !existing.active && candidate.strength > existing.strength {
                            *existing = candidate;
                        }
                    }
                    None => access_points.push(candidate),
                }
            }
        }

        // Strongest first, then SSID for determinism.
        access_points.sort_by(|a, b| {
            b.strength
                .cmp(&a.strength)
                .then_with(|| a.ssid.cmp(&b.ssid))
        });

        let active_ssid = state
            .is_connected()
            .then(|| {
                access_points
                    .iter()
                    .find(|ap| ap.active)
                    .map(|ap| ap.ssid.clone())
            })
            .flatten();

        WifiSnapshot {
            enabled: data.wireless_enabled,
            connectivity: Connectivity::from_u32(data.connectivity),
            state,
            access_points,
            active_ssid,
        }
    }

    /// The signal strength of the active network, when connected.
    pub fn signal_strength(&self) -> Option<u8> {
        self.access_points
            .iter()
            .find(|ap| ap.active)
            .map(|ap| ap.strength)
    }

    /// The glyph the menu bar draws for the current state.
    pub fn glyph(&self) -> &'static str {
        if !self.enabled {
            return "wifi-disabled";
        }
        match self.state {
            WifiState::Connected => {
                if self.active_security().is_some_and(Security::is_secured) {
                    "wifi-secure"
                } else {
                    "wifi"
                }
            }
            WifiState::Connecting => "wifi-connecting",
            WifiState::Failed => "wifi-error",
            _ => "wifi-off",
        }
    }

    /// A one-line label for the menu bar / menu header.
    pub fn label(&self) -> String {
        if !self.enabled {
            return "Wi-Fi Off".to_owned();
        }
        match &self.active_ssid {
            Some(ssid) if self.state.is_connected() => {
                let strength = self
                    .signal_strength()
                    .map(|s| format!(" · {s}%"))
                    .unwrap_or_default();
                format!("{ssid}{strength}")
            }
            _ => self.connectivity.label().to_owned(),
        }
    }

    /// The security of the active network, when connected.
    pub fn active_security(&self) -> Option<Security> {
        self.access_points
            .iter()
            .find(|ap| ap.active)
            .map(|ap| ap.security)
    }

    /// The number of visible networks.
    pub fn network_count(&self) -> usize {
        self.access_points.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::AccessPointData;

    fn ap(path: &str, ssid: &str, strength: u8, rsn: u32, flags: u32) -> AccessPointData {
        AccessPointData {
            path: path.to_owned(),
            ssid: ssid.to_owned(),
            strength,
            frequency_mhz: 5180,
            flags,
            wpa_flags: 0,
            rsn_flags: rsn,
        }
    }

    fn data(state: u32, aps: Vec<AccessPointData>, active: Option<&str>) -> NetworkManagerData {
        NetworkManagerData {
            wireless_enabled: true,
            connectivity: 4,
            devices: vec![WifiDeviceData {
                managed: true,
                state,
                active_ap: active.map(str::to_owned),
                access_points: aps,
            }],
        }
    }

    #[test]
    fn device_state_maps_the_intermediate_values_to_connecting() {
        assert_eq!(WifiState::from_u32(30), WifiState::Disconnected);
        for value in [40, 50, 60, 70, 80, 90] {
            assert_eq!(WifiState::from_u32(value), WifiState::Connecting);
        }
        assert_eq!(WifiState::from_u32(100), WifiState::Connected);
        assert_eq!(WifiState::from_u32(120), WifiState::Failed);
        assert_eq!(WifiState::from_u32(999), WifiState::Unknown);
    }

    #[test]
    fn security_decodes_sae_enterprise_wpa2_wep_and_open() {
        assert_eq!(Security::from_flags(0, 0, 0), Security::Open);
        assert_eq!(Security::from_flags(0x1, 0, 0), Security::Wep);
        assert_eq!(Security::from_flags(0x1, 0x100, 0), Security::Wpa);
        assert_eq!(Security::from_flags(0x1, 0, 0x108), Security::Wpa2);
        assert_eq!(Security::from_flags(0x1, 0, 0x400), Security::Wpa3);
        assert_eq!(Security::from_flags(0x1, 0, 0x200), Security::Enterprise);
    }

    #[test]
    fn bands_classify_frequencies() {
        assert_eq!(Band::from_frequency_mhz(2412), Band::Ghz2);
        assert_eq!(Band::from_frequency_mhz(5180), Band::Ghz5);
        assert_eq!(Band::from_frequency_mhz(6115), Band::Ghz6);
        assert_eq!(Band::from_frequency_mhz(1), Band::Unknown);
    }

    #[test]
    fn unmanaged_devices_are_ignored() {
        let mut raw = data(100, vec![ap("p1", "net", 80, 0, 0)], Some("p1"));
        raw.devices.push(WifiDeviceData {
            managed: false,
            state: 120,
            active_ap: None,
            access_points: vec![ap("p2", "other", 90, 0, 0)],
        });
        let snapshot = WifiSnapshot::from_data(&raw);
        assert_eq!(snapshot.state, WifiState::Connected);
        assert_eq!(snapshot.network_count(), 1);
        assert_eq!(snapshot.active_ssid.as_deref(), Some("net"));
    }

    #[test]
    fn bssids_collapse_to_one_entry_per_ssid_keeping_the_strongest() {
        let raw = data(
            100,
            vec![
                ap("p1", "home", 30, 0, 0),
                ap("p2", "home", 70, 0, 0),
                ap("p3", "cafe", 55, 0, 0),
            ],
            Some("p2"),
        );
        let snapshot = WifiSnapshot::from_data(&raw);
        assert_eq!(snapshot.network_count(), 2);
        assert_eq!(snapshot.access_points[0].ssid, "home");
        assert_eq!(snapshot.access_points[0].strength, 70);
        assert!(snapshot.access_points[0].active);
        assert_eq!(snapshot.access_points[1].ssid, "cafe");
        assert!(!snapshot.access_points[1].active);
        assert_eq!(snapshot.signal_strength(), Some(70));
    }

    #[test]
    fn the_active_flag_survives_a_stronger_neighbour_bssid() {
        let raw = data(
            100,
            vec![
                ap("p1", "home", 90, 0, 0), // stronger, not active
                ap("p2", "home", 40, 0, 0), // active
            ],
            Some("p2"),
        );
        let snapshot = WifiSnapshot::from_data(&raw);
        assert_eq!(snapshot.network_count(), 1);
        assert!(snapshot.access_points[0].active);
        assert_eq!(snapshot.active_ssid.as_deref(), Some("home"));
        // The active BSSID's strength is the one reported for the active net.
        assert_eq!(snapshot.signal_strength(), Some(40));
    }

    #[test]
    fn connected_network_labelled_with_strength_and_glyph() {
        let raw = data(
            100,
            vec![ap("p1", "dragonfruit", 82, 0x108, 0x1)],
            Some("p1"),
        );
        let snapshot = WifiSnapshot::from_data(&raw);
        assert_eq!(snapshot.connectivity, Connectivity::Full);
        assert_eq!(snapshot.glyph(), "wifi-secure");
        assert_eq!(snapshot.label(), "dragonfruit · 82%");
        assert_eq!(snapshot.active_security(), Some(Security::Wpa2));
    }

    #[test]
    fn disabled_radio_reports_the_off_glyph() {
        let mut raw = data(30, vec![], None);
        raw.wireless_enabled = false;
        let snapshot = WifiSnapshot::from_data(&raw);
        assert_eq!(snapshot.glyph(), "wifi-disabled");
        assert_eq!(snapshot.label(), "Wi-Fi Off");
    }
}
