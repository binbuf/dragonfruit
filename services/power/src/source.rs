// SPDX-License-Identifier: MIT
//! The transport seam: the raw UPower / power-profiles-daemon read and its
//! mock.
//!
//! A [`PowerSource`] is the only thing that talks to the daemons. The real
//! source is the D-Bus client ([`crate::DbusUPower`]); tests and CI use
//! [`MockPower`], which serves a fixture with no bus on the machine. The
//! adapter ([`crate::PowerAdapter`]) turns one raw read into the typed
//! [`PowerSnapshot`](crate::PowerSnapshot) and drives the shared
//! `Subscription`.
//!
//! The battery half is read-only; the power-profile half has one explicit
//! write ([`PowerSource::set_active_profile`]) that selects the
//! performance/balanced/power-saver profile the daemon applies. The raw shape
//! is deliberately flat and daemon-shaped: the D-Bus source fills it from
//! proxies, a fixture deserialises straight into it, and nothing above the
//! adapter ever sees it.
//!
//! UPower and power-profiles-daemon are **independent** daemons on the system
//! bus. Either can be absent on its own: a desktop with no battery still has
//! profiles, and a laptop whose power-profiles-daemon is masked still has a
//! battery. The source reports the battery half and the profile half
//! separately, and only answers absence when neither daemon is reachable — a
//! normal state, never an error.

use dragonfruit_system_adapters::AdapterError;
use serde::Deserialize;

use crate::model::PowerProfile;

/// `UPowerDeviceType` for a battery (`UP_DEVICE_KIND_BATTERY`).
pub const DEVICE_TYPE_BATTERY: u32 = 2;

/// The result of one UPower / power-profiles-daemon read, shaped after their
/// D-Bus objects.
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
    /// The power-profiles-daemon read, or `None` when that daemon is absent.
    /// Absent profiles are a normal state; the battery half stays live.
    #[serde(default)]
    pub profiles: Option<PowerProfilesData>,
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
    /// `Capacity`: health as a percentage of design capacity (100 is new);
    /// 0 when UPower does not know.
    #[serde(default)]
    pub capacity: f64,
    /// `ChargeCycles`; 0 when UPower does not know.
    #[serde(default)]
    pub charge_cycles: i32,
    /// `TimeToEmpty` in seconds; 0 when unknown.
    #[serde(default)]
    pub time_to_empty: i64,
    /// `TimeToFull` in seconds; 0 when unknown.
    #[serde(default)]
    pub time_to_full: i64,
}

/// One entry from power-profiles-daemon's `Profiles` property (`aa{sv}`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct PowerProfileData {
    /// The profile id (`power-saver`, `balanced`, `performance`).
    #[serde(default)]
    pub profile: String,
    /// The kernel driver backing the profile.
    #[serde(default)]
    pub driver: String,
    /// The platform driver backing the profile, when any.
    #[serde(default)]
    pub platform_driver: String,
}

/// The result of one power-profiles-daemon read
/// (`net.hadess.PowerProfiles`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct PowerProfilesData {
    /// `ActiveProfile`: the profile the daemon is currently applying.
    #[serde(default)]
    pub active_profile: String,
    /// `Profiles`: every profile this machine supports.
    #[serde(default)]
    pub profiles: Vec<PowerProfileData>,
    /// `PerformanceInhibited`: non-empty when performance is being held back.
    #[serde(default)]
    pub performance_inhibited: String,
    /// `PerformanceDegraded`: non-empty when the platform cannot deliver the
    /// requested performance (e.g. `lap-detected`, `high-operating-temperature`).
    #[serde(default)]
    pub performance_degraded: String,
    /// `ActiveProfileHolds`: how many clients currently hold a profile.
    #[serde(default)]
    pub holds: u32,
}

/// The result of the one power-profile write.
///
/// Selecting a profile is an explicit user action, never a poll. A write that
/// lands does not invent a snapshot: power-profiles-daemon pushes the
/// resulting `PropertiesChanged`, the host re-reads, and the snapshot stays
/// the single source of truth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileOutcome {
    /// The daemon applied the profile.
    Applied,
    /// The daemon is absent; there is nothing to select.
    Absent,
    /// The write failed for any other reason.
    Failed(AdapterError),
}

impl ProfileOutcome {
    /// Whether the daemon applied the write.
    pub fn is_applied(&self) -> bool {
        matches!(self, ProfileOutcome::Applied)
    }

    /// The failure, when the write failed.
    pub fn error(&self) -> Option<&AdapterError> {
        match self {
            ProfileOutcome::Failed(error) => Some(error),
            ProfileOutcome::Applied | ProfileOutcome::Absent => None,
        }
    }
}

/// Reads power state over some transport.
///
/// The read result is a three-way answer, exactly as the adapter contract
/// needs it:
///
/// * `Ok(Some(data))` — at least one daemon answered; `data` is the live read.
/// * `Ok(None)` — **both** UPower and power-profiles-daemon are absent. A
///   normal state; the slot hides.
/// * `Err(error)` — a daemon that owns its name could not be read; the slot
///   shows visible and inert with the message.
///
/// The source is never polled by a consumer: the host calls
/// [`PowerAdapter::refresh`](crate::PowerAdapter::refresh) when UPower or
/// power-profiles-daemon signals a change.
pub trait PowerSource {
    /// One read of the daemons.
    fn read(&mut self) -> Result<Option<PowerData>, AdapterError>;

    /// Select the active power profile. One explicit write.
    ///
    /// The default is [`ProfileOutcome::Absent`], so a source without the
    /// profiles daemon still satisfies the trait; the battery half is
    /// unaffected.
    fn set_active_profile(&mut self, _profile: PowerProfile) -> ProfileOutcome {
        ProfileOutcome::Absent
    }
}

/// A fixture-backed source with a simulated daemon lifecycle.
///
/// The mock is the CI path: it serves [`PowerData`] with no UPower or
/// power-profiles-daemon, and `kill`/`restart` exercise absence and
/// re-subscribe the way masking the real daemon would. Its one write mutates
/// the simulated profile the way power-profiles-daemon would, so a subsequent
/// `refresh` sees the change.
#[derive(Debug, Clone, PartialEq)]
pub struct MockPower {
    present: bool,
    data: Option<PowerData>,
    failure: Option<AdapterError>,
    write_failure: Option<AdapterError>,
    reads: u32,
    profile_writes: u32,
}

impl MockPower {
    /// Both daemons are not running.
    pub fn absent() -> Self {
        MockPower {
            present: false,
            data: None,
            failure: None,
            write_failure: None,
            reads: 0,
            profile_writes: 0,
        }
    }

    /// A running host stack that answers with `data`.
    pub fn present(data: PowerData) -> Self {
        MockPower {
            present: true,
            data: Some(data),
            failure: None,
            write_failure: None,
            reads: 0,
            profile_writes: 0,
        }
    }

    /// A running host stack that fails every read (e.g. it went
    /// unresponsive).
    pub fn failing(message: impl Into<String>) -> Self {
        MockPower {
            present: true,
            data: None,
            failure: Some(AdapterError::new(message)),
            write_failure: None,
            reads: 0,
            profile_writes: 0,
        }
    }

    /// A running host stack whose writes fail (the simulates a daemon that
    /// owns its name but refuses the write).
    pub fn fail_writes(mut self, message: impl Into<String>) -> Self {
        self.write_failure = Some(AdapterError::new(message));
        self
    }

    /// The host stack pushes fresh data.
    pub fn push(&mut self, data: PowerData) {
        self.present = true;
        self.failure = None;
        self.data = Some(data);
    }

    /// The host stack goes away.
    pub fn kill(&mut self) {
        self.present = false;
    }

    /// The host stack comes back (serving the last data, if any).
    pub fn restart(&mut self) {
        self.present = true;
        self.failure = None;
    }

    /// Whether the simulated host stack is running.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// How many times the source has been read. Lets a test prove there is no
    /// hidden polling above the adapter.
    pub fn reads(&self) -> u32 {
        self.reads
    }

    /// How many profile writes the source has accepted. Lets a test prove the
    /// adapter does not invent a snapshot after a write.
    pub fn profile_writes(&self) -> u32 {
        self.profile_writes
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

    fn set_active_profile(&mut self, profile: PowerProfile) -> ProfileOutcome {
        if !self.present {
            return ProfileOutcome::Absent;
        }
        if let Some(error) = &self.write_failure {
            return ProfileOutcome::Failed(error.clone());
        }
        match self.data.as_mut().and_then(|data| data.profiles.as_mut()) {
            Some(profiles) => {
                profiles.active_profile = profile.id().to_owned();
                self.profile_writes += 1;
                ProfileOutcome::Applied
            }
            // The daemon is present but exposes no profiles half.
            None => ProfileOutcome::Absent,
        }
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
                capacity: 96.0,
                charge_cycles: 112,
                time_to_empty: 3600,
                time_to_full: 0,
            }],
            profiles: Some(PowerProfilesData {
                active_profile: "balanced".to_owned(),
                profiles: vec![
                    PowerProfileData {
                        profile: "power-saver".to_owned(),
                        ..PowerProfileData::default()
                    },
                    PowerProfileData {
                        profile: "balanced".to_owned(),
                        ..PowerProfileData::default()
                    },
                    PowerProfileData {
                        profile: "performance".to_owned(),
                        ..PowerProfileData::default()
                    },
                ],
                ..PowerProfilesData::default()
            }),
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

    #[test]
    fn a_profile_write_is_absent_when_the_daemon_is_gone() {
        let mut mock = MockPower::absent();
        assert_eq!(
            mock.set_active_profile(PowerProfile::Performance),
            ProfileOutcome::Absent
        );
    }

    #[test]
    fn a_profile_write_mutates_the_simulated_active_profile() {
        let mut mock = MockPower::present(data());
        assert_eq!(
            mock.set_active_profile(PowerProfile::Performance),
            ProfileOutcome::Applied
        );
        assert_eq!(mock.profile_writes(), 1);
        let read = mock.read().unwrap().unwrap();
        assert_eq!(
            read.profiles.as_ref().unwrap().active_profile,
            "performance"
        );
    }

    #[test]
    fn a_profile_write_is_absent_without_a_profiles_half() {
        let mut mock = MockPower::present(PowerData::default());
        assert_eq!(
            mock.set_active_profile(PowerProfile::Balanced),
            ProfileOutcome::Absent
        );
        assert_eq!(mock.profile_writes(), 0);
    }

    #[test]
    fn a_failing_profile_write_reports_the_error() {
        let mut mock = MockPower::present(data()).fail_writes("PPD: no such profile");
        let outcome = mock.set_active_profile(PowerProfile::Balanced);
        assert_eq!(
            outcome.error().map(AdapterError::message),
            Some("PPD: no such profile")
        );
        assert!(!outcome.is_applied());
    }

    #[test]
    fn the_fixture_deserializes_a_profiles_half() {
        let raw: PowerData = serde_json::from_str(
            r#"{
                "on_battery": false,
                "devices": [],
                "profiles": {
                    "active_profile": "power-saver",
                    "profiles": [{ "profile": "power-saver", "driver": "amd_pstate" }]
                }
            }"#,
        )
        .expect("valid raw data");
        let profiles = raw.profiles.expect("profiles present");
        assert_eq!(profiles.active_profile, "power-saver");
        assert_eq!(profiles.profiles[0].driver, "amd_pstate");
    }
}
