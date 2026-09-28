// SPDX-License-Identifier: MIT
//! The battery snapshot the menu bar renders, decoded from one raw read.
//!
//! The model owns the UPower enum mapping and the aggregation a consumer should
//! not repeat: it picks the system battery, clamps its percentage to 0–100,
//! and derives the glyph, label, and fill level the status item draws. T-07.5
//! renders [`PowerSnapshot::glyph`] / [`PowerSnapshot::label`] and the
//! [`PowerSnapshot::level`] fill.

use crate::source::{PowerData, PowerDeviceData, PowerProfilesData, DEVICE_TYPE_BATTERY};

/// Whether a battery is charging, discharging, or in between.
///
/// Mapped from `UPowerDeviceState`; the `Pending*` values are the transient
/// states UPower reports while it is deciding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChargeState {
    /// UPower has not reported a state.
    #[default]
    Unknown,
    /// The battery is gaining charge.
    Charging,
    /// The battery is supplying power.
    Discharging,
    /// The battery is flat.
    Empty,
    /// The battery is at full capacity while plugged in.
    FullyCharged,
    /// Plugged in, waiting to charge (e.g. a charge threshold).
    PendingCharge,
    /// Unplugged, waiting to discharge.
    PendingDischarge,
}

impl ChargeState {
    /// Map the numeric `UPowerDeviceState`.
    pub const fn from_u32(value: u32) -> Self {
        match value {
            1 => ChargeState::Charging,
            2 => ChargeState::Discharging,
            3 => ChargeState::Empty,
            4 => ChargeState::FullyCharged,
            5 => ChargeState::PendingCharge,
            6 => ChargeState::PendingDischarge,
            _ => ChargeState::Unknown,
        }
    }

    /// Whether the battery is on the way up (or held full while plugged).
    pub const fn is_charging(self) -> bool {
        matches!(self, ChargeState::Charging | ChargeState::PendingCharge)
    }

    /// Whether the system is on line power in this state.
    pub const fn is_plugged(self) -> bool {
        matches!(
            self,
            ChargeState::Charging | ChargeState::FullyCharged | ChargeState::PendingCharge
        )
    }

    /// Whether the battery is supplying power.
    pub const fn is_discharging(self) -> bool {
        matches!(
            self,
            ChargeState::Discharging | ChargeState::PendingDischarge
        )
    }

    /// A stable label for logs and the menu.
    pub const fn name(self) -> &'static str {
        match self {
            ChargeState::Unknown => "unknown",
            ChargeState::Charging => "charging",
            ChargeState::Discharging => "discharging",
            ChargeState::Empty => "empty",
            ChargeState::FullyCharged => "fully-charged",
            ChargeState::PendingCharge => "pending-charge",
            ChargeState::PendingDischarge => "pending-discharge",
        }
    }
}

/// The coarse `UPowerBatteryLevel` bucket UPower reports for a battery.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BatteryLevel {
    /// UPower reports no level.
    #[default]
    Unknown,
    /// No level information is available.
    None,
    /// Low.
    Low,
    /// Critical.
    Critical,
    /// Normal.
    Normal,
    /// High.
    High,
    /// Full.
    Full,
}

impl BatteryLevel {
    /// Map the numeric `UPowerBatteryLevel`.
    pub const fn from_u32(value: u32) -> Self {
        match value {
            1 => BatteryLevel::None,
            2 => BatteryLevel::Low,
            3 => BatteryLevel::Critical,
            4 => BatteryLevel::Normal,
            5 => BatteryLevel::High,
            6 => BatteryLevel::Full,
            _ => BatteryLevel::Unknown,
        }
    }

    /// A stable label for logs.
    pub const fn name(self) -> &'static str {
        match self {
            BatteryLevel::Unknown => "unknown",
            BatteryLevel::None => "none",
            BatteryLevel::Low => "low",
            BatteryLevel::Critical => "critical",
            BatteryLevel::Normal => "normal",
            BatteryLevel::High => "high",
            BatteryLevel::Full => "full",
        }
    }
}

/// The macOS "Battery Health" bucket, derived from UPower's `Capacity`.
///
/// UPower reports raw capacity as a percentage of design capacity, not a
/// health verdict. The threshold below (80%) is ours, chosen to match the
/// service-level boundary common to vendor health reports; the pane shows
/// [`BatteryHealth::name`] and the raw percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BatteryHealth {
    /// UPower reports no capacity.
    #[default]
    Unknown,
    /// The battery holds a healthy share of its design capacity.
    Normal,
    /// The battery has degraded below the service threshold.
    Service,
}

impl BatteryHealth {
    /// The health bucket for a raw capacity percentage.
    ///
    /// `None` (unknown capacity) maps to [`BatteryHealth::Unknown`].
    pub const fn from_capacity(capacity: Option<u8>) -> Self {
        match capacity {
            Some(value) if value >= 80 => BatteryHealth::Normal,
            Some(_) => BatteryHealth::Service,
            None => BatteryHealth::Unknown,
        }
    }

    /// The label the pane shows.
    pub const fn label(self) -> &'static str {
        match self {
            BatteryHealth::Unknown => "Unknown",
            BatteryHealth::Normal => "Normal",
            BatteryHealth::Service => "Service",
        }
    }

    /// A stable id for logs and tests.
    pub const fn name(self) -> &'static str {
        match self {
            BatteryHealth::Unknown => "unknown",
            BatteryHealth::Normal => "normal",
            BatteryHealth::Service => "service",
        }
    }
}

/// The system battery the menu bar reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Battery {
    /// The charge, clamped to 0–100.
    pub percentage: f32,
    /// Whether the battery is charging, discharging, or in between.
    pub state: ChargeState,
    /// The coarse level bucket UPower reports.
    pub level: BatteryLevel,
    /// `Capacity`: health as a percentage of design capacity, when known.
    pub capacity: Option<u8>,
    /// `ChargeCycles`, when the daemon reports a positive count.
    pub charge_cycles: Option<u32>,
    /// Seconds until empty, when UPower knows.
    pub time_to_empty: Option<u64>,
    /// Seconds until full, when UPower knows.
    pub time_to_full: Option<u64>,
}

impl Battery {
    /// The charge as the 0–100 integer the menu shows.
    pub fn percent(&self) -> u8 {
        self.percentage.round().clamp(0.0, 100.0) as u8
    }

    /// The charge as the 0..=1 fill a status glyph draws.
    pub fn fill(&self) -> f32 {
        (self.percentage / 100.0).clamp(0.0, 1.0)
    }

    /// The health bucket derived from the reported capacity.
    pub fn health(&self) -> BatteryHealth {
        BatteryHealth::from_capacity(self.capacity)
    }

    /// Whether the battery is on the way up.
    pub fn is_charging(&self) -> bool {
        self.state.is_charging()
    }

    /// Whether the battery is supplying power.
    pub fn is_discharging(&self) -> bool {
        self.state.is_discharging()
    }
}

/// One power profile power-profiles-daemon can apply.
///
/// The three profile ids are the daemon's stable, cross-distro vocabulary
/// (`power-saver`, `balanced`, `performance`); the pane and Control Center
/// segment map straight onto [`PowerProfile::id`]. An id the daemon reports
/// that is not one of these is dropped from the snapshot rather than guessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PowerProfile {
    /// Power saver (`power-saver`).
    PowerSaver,
    /// Balanced (`balanced`).
    Balanced,
    /// Performance (`performance`).
    Performance,
}

impl PowerProfile {
    /// Every profile, in the order the pane lists them (least to most power).
    pub const ALL: [PowerProfile; 3] = [
        PowerProfile::PowerSaver,
        PowerProfile::Balanced,
        PowerProfile::Performance,
    ];

    /// The stable id used by the daemon, the wire, and any settings value.
    pub const fn id(self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "power-saver",
            PowerProfile::Balanced => "balanced",
            PowerProfile::Performance => "performance",
        }
    }

    /// The pane/popup label.
    pub const fn label(self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "Power Saver",
            PowerProfile::Balanced => "Balanced",
            PowerProfile::Performance => "Performance",
        }
    }

    /// Parse the stable id back to a profile.
    pub fn from_id(id: &str) -> Option<Self> {
        PowerProfile::ALL
            .into_iter()
            .find(|profile| profile.id() == id)
    }

    /// The design-system glyph for a profile tile segment.
    pub const fn glyph(self) -> &'static str {
        match self {
            PowerProfile::PowerSaver => "power-saver",
            PowerProfile::Balanced => "power-balanced",
            PowerProfile::Performance => "power-performance",
        }
    }
}

/// The power-profiles-daemon half of the snapshot.
///
/// `active` is `None` when the daemon is absent (`profiles` itself is `None`
/// on [`PowerSnapshot`]) or when it reported an id this build does not know;
/// `available` is the canonical subset the machine actually supports. A
/// profile is settable only when it is in `available`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PowerProfilesSnapshot {
    /// The profile the daemon is applying, when it is one we know.
    pub active: Option<PowerProfile>,
    /// The profiles this machine supports, in canonical order.
    pub available: Vec<PowerProfile>,
    /// `PerformanceInhibited`: performance is being held back.
    pub performance_inhibited: bool,
    /// `PerformanceDegraded`: the reason performance is degraded (empty when
    /// it is not).
    pub performance_degraded: String,
    /// How many clients hold a profile right now.
    pub holds: u32,
}

impl PowerProfilesSnapshot {
    /// Build the profile snapshot from one raw read.
    pub fn from_data(data: &PowerProfilesData) -> Self {
        let available = PowerProfile::ALL
            .into_iter()
            .filter(|profile| {
                data.profiles
                    .iter()
                    .any(|entry| entry.profile == profile.id())
            })
            .collect();
        PowerProfilesSnapshot {
            active: PowerProfile::from_id(&data.active_profile),
            available,
            performance_inhibited: !data.performance_inhibited.is_empty(),
            performance_degraded: data.performance_degraded.clone(),
            holds: data.holds,
        }
    }

    /// Whether the daemon exposed any profile. A present daemon with an empty
    /// list is not useful; the pane hides the profile control then.
    pub fn present(&self) -> bool {
        !self.available.is_empty()
    }

    /// Whether `profile` is one this machine supports.
    pub fn supports(&self, profile: PowerProfile) -> bool {
        self.available.contains(&profile)
    }

    /// Whether performance is currently degraded, and why.
    pub fn degraded(&self) -> bool {
        !self.performance_degraded.is_empty()
    }

    /// The active profile's label, or a status word when there is none.
    pub fn active_label(&self) -> String {
        match self.active {
            Some(profile) => profile.label().to_owned(),
            None if self.present() => "Unknown".to_owned(),
            None => "Unavailable".to_owned(),
        }
    }

    /// The design-system glyph for the active profile.
    pub fn glyph(&self) -> &'static str {
        self.active
            .map(PowerProfile::glyph)
            .unwrap_or("power-balanced")
    }
}

/// The power snapshot a menu bar renders: the battery, the system's power
/// source, and the power-profile selection.
///
/// `on_battery` is the manager's `OnBattery` property; `battery` is `None` on
/// a machine without a present battery (a desktop, a VM), so a consumer can
/// hide the item. `profiles` is `None` when power-profiles-daemon is absent —
/// an independent, normal state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PowerSnapshot {
    /// The system battery, when UPower reports a present one.
    pub battery: Option<Battery>,
    /// `org.freedesktop.UPower.OnBattery`.
    pub on_battery: bool,
    /// The power-profiles-daemon selection, when that daemon is present.
    pub profiles: Option<PowerProfilesSnapshot>,
}

impl PowerSnapshot {
    /// Build the snapshot from one raw read: pick the first present battery
    /// and map its charge state and level, and decode the profiles half.
    pub fn from_data(data: &PowerData) -> Self {
        let battery = data
            .devices
            .iter()
            .find(|device| is_present_battery(device))
            .map(battery_from_device);
        PowerSnapshot {
            battery,
            on_battery: data.on_battery,
            profiles: data.profiles.as_ref().map(PowerProfilesSnapshot::from_data),
        }
    }

    /// The system battery, when present.
    pub fn battery(&self) -> Option<&Battery> {
        self.battery.as_ref()
    }

    /// Whether the machine has a battery to report. The item hides when it
    /// does not.
    pub fn present(&self) -> bool {
        self.battery.is_some()
    }

    /// The battery charge, or `0.0` when there is no battery.
    pub fn percentage(&self) -> f32 {
        self.battery
            .as_ref()
            .map(|battery| battery.percentage)
            .unwrap_or(0.0)
    }

    /// The battery charge as the 0–100 integer the menu shows.
    pub fn percentage_percent(&self) -> u8 {
        self.battery
            .as_ref()
            .map(Battery::percent)
            .unwrap_or_default()
    }

    /// The 0..=1 fill a status glyph draws.
    pub fn level(&self) -> f32 {
        self.battery.as_ref().map(Battery::fill).unwrap_or_default()
    }

    /// The charge state, `Unknown` when there is no battery.
    pub fn state(&self) -> ChargeState {
        self.battery
            .as_ref()
            .map(|battery| battery.state)
            .unwrap_or_default()
    }

    /// Whether the battery is on the way up.
    pub fn charging(&self) -> bool {
        self.state().is_charging()
    }

    /// Whether the system is on line power.
    pub fn plugged(&self) -> bool {
        self.state().is_plugged()
    }

    /// `org.freedesktop.UPower.OnBattery`.
    pub fn on_battery(&self) -> bool {
        self.on_battery
    }

    /// The power-profiles-daemon selection, when that daemon is present.
    pub fn profiles(&self) -> Option<&PowerProfilesSnapshot> {
        self.profiles.as_ref()
    }

    /// Whether power-profiles-daemon exposed any profile to select.
    pub fn profiles_available(&self) -> bool {
        self.profiles
            .as_ref()
            .is_some_and(|profiles| profiles.present())
    }

    /// The active power profile, when one is known.
    pub fn active_profile(&self) -> Option<PowerProfile> {
        self.profiles.as_ref().and_then(|profiles| profiles.active)
    }

    /// The active profile's label, or a status word when there is none.
    pub fn profile_label(&self) -> String {
        match &self.profiles {
            Some(profiles) => profiles.active_label(),
            None => "Unavailable".to_owned(),
        }
    }

    /// The battery health, when a battery is present.
    pub fn health(&self) -> BatteryHealth {
        self.battery
            .as_ref()
            .map(Battery::health)
            .unwrap_or_default()
    }

    /// Seconds until empty, when UPower knows.
    pub fn time_to_empty(&self) -> Option<u64> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.time_to_empty)
    }

    /// Seconds until full, when UPower knows.
    pub fn time_to_full(&self) -> Option<u64> {
        self.battery
            .as_ref()
            .and_then(|battery| battery.time_to_full)
    }

    /// The glyph the status slot draws. Charging is carried by
    /// [`charging`](Self::charging) and the label, not by a distinct glyph
    /// case; T-07.5b may add a bolt.
    pub fn glyph(&self) -> &'static str {
        "battery"
    }

    /// A one-line label for the menu bar / menu header.
    pub fn label(&self) -> String {
        let Some(battery) = &self.battery else {
            return "No battery".to_owned();
        };
        let percent = battery.percent();
        match battery.state {
            ChargeState::Charging | ChargeState::PendingCharge => {
                format!("{percent}% charging")
            }
            ChargeState::FullyCharged => format!("{percent}% charged"),
            _ => format!("{percent}%"),
        }
    }

    /// What changed between `previous` and this snapshot, in a stable order:
    /// the battery, then the system power source, then the profile selection.
    ///
    /// This is the "event" half of the adapter: the host can diff two reads
    /// to learn what moved without polling each field. A snapshot changing
    /// from present to no battery (or back) is one coarse event; while a
    /// battery is present its field moves are reported individually.
    pub fn changes(&self, previous: &PowerSnapshot) -> Vec<PowerChange> {
        let mut changes = Vec::new();
        match (&previous.battery, &self.battery) {
            (None, Some(_)) => changes.push(PowerChange::BatteryAppeared),
            (Some(_), None) => changes.push(PowerChange::BatteryRemoved),
            (Some(from), Some(to)) => {
                if from.percent() != to.percent() {
                    changes.push(PowerChange::BatteryLevelChanged {
                        from: from.percent(),
                        to: to.percent(),
                    });
                }
                if from.state != to.state {
                    changes.push(PowerChange::ChargeStateChanged {
                        from: from.state,
                        to: to.state,
                    });
                }
                if from.capacity != to.capacity {
                    changes.push(PowerChange::BatteryHealthChanged {
                        from: from.capacity,
                        to: to.capacity,
                    });
                }
            }
            (None, None) => {}
        }

        if self.on_battery != previous.on_battery {
            changes.push(PowerChange::OnBatteryChanged {
                from: previous.on_battery,
                to: self.on_battery,
            });
        }

        let from_profiles = previous.profiles.as_ref();
        let to_profiles = self.profiles.as_ref();
        let from_available = from_profiles
            .map(|profiles| profiles.available.clone())
            .unwrap_or_default();
        let to_available = to_profiles
            .map(|profiles| profiles.available.clone())
            .unwrap_or_default();
        if from_available != to_available {
            changes.push(PowerChange::AvailableProfilesChanged {
                from: from_available,
                to: to_available,
            });
        }

        let from_active = from_profiles.and_then(|profiles| profiles.active);
        let to_active = to_profiles.and_then(|profiles| profiles.active);
        if from_active != to_active {
            changes.push(PowerChange::ActiveProfileChanged {
                from: from_active,
                to: to_active,
            });
        }

        let from_degraded = from_profiles
            .map(|profiles| profiles.performance_degraded.clone())
            .unwrap_or_default();
        let to_degraded = to_profiles
            .map(|profiles| profiles.performance_degraded.clone())
            .unwrap_or_default();
        if from_degraded != to_degraded {
            changes.push(PowerChange::PerformanceDegradedChanged {
                from: from_degraded,
                to: to_degraded,
            });
        }

        let from_holds = from_profiles
            .map(|profiles| profiles.holds)
            .unwrap_or_default();
        let to_holds = to_profiles
            .map(|profiles| profiles.holds)
            .unwrap_or_default();
        if from_holds != to_holds {
            changes.push(PowerChange::ProfileHoldsChanged {
                from: from_holds,
                to: to_holds,
            });
        }

        changes
    }
}

/// A change between two power snapshots.
#[derive(Debug, Clone, PartialEq)]
pub enum PowerChange {
    /// A battery appeared (a bay gained a cell, or UPower came back).
    BatteryAppeared,
    /// The battery went away.
    BatteryRemoved,
    /// The charge level moved.
    BatteryLevelChanged {
        /// The previous 0–100 level.
        from: u8,
        /// The new 0–100 level.
        to: u8,
    },
    /// The charge state moved (charging, discharging, …).
    ChargeStateChanged {
        /// The previous state.
        from: ChargeState,
        /// The new state.
        to: ChargeState,
    },
    /// The reported battery capacity (health) moved.
    BatteryHealthChanged {
        /// The previous capacity percentage, if any.
        from: Option<u8>,
        /// The new capacity percentage, if any.
        to: Option<u8>,
    },
    /// The system switched between battery and line power.
    OnBatteryChanged {
        /// The previous `OnBattery`.
        from: bool,
        /// The new `OnBattery`.
        to: bool,
    },
    /// The set of supported profiles changed (the daemon came or went, or the
    /// machine's capability changed).
    AvailableProfilesChanged {
        /// The previous supported profiles.
        from: Vec<PowerProfile>,
        /// The new supported profiles.
        to: Vec<PowerProfile>,
    },
    /// The active profile changed.
    ActiveProfileChanged {
        /// The previous active profile, if any.
        from: Option<PowerProfile>,
        /// The new active profile, if any.
        to: Option<PowerProfile>,
    },
    /// The performance-degradation reason changed (including cleared).
    PerformanceDegradedChanged {
        /// The previous reason (empty when not degraded).
        from: String,
        /// The new reason (empty when not degraded).
        to: String,
    },
    /// The number of clients holding a profile changed.
    ProfileHoldsChanged {
        /// The previous hold count.
        from: u32,
        /// The new hold count.
        to: u32,
    },
}

/// Whether `device` is a battery this snapshot should report.
fn is_present_battery(device: &PowerDeviceData) -> bool {
    device.kind == DEVICE_TYPE_BATTERY && device.present
}

/// Map one raw battery device, clamping its percentage and unknown times.
fn battery_from_device(device: &PowerDeviceData) -> Battery {
    Battery {
        percentage: device.percentage.clamp(0.0, 100.0) as f32,
        state: ChargeState::from_u32(device.state),
        level: BatteryLevel::from_u32(device.battery_level),
        capacity: capacity_percent(device.capacity),
        charge_cycles: positive(device.charge_cycles),
        time_to_empty: seconds(device.time_to_empty),
        time_to_full: seconds(device.time_to_full),
    }
}

/// UPower reports 0 capacity for "unknown"; keep only a real percentage.
fn capacity_percent(value: f64) -> Option<u8> {
    (value > 0.0).then(|| value.round().clamp(0.0, 100.0) as u8)
}

/// A positive count (charge cycles), `None` at the 0/negative sentinel.
fn positive(value: i32) -> Option<u32> {
    (value > 0).then_some(value as u32)
}

/// UPower reports 0 seconds for "unknown"; keep only a real duration.
fn seconds(value: i64) -> Option<u64> {
    (value > 0).then_some(value as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::PowerProfileData;

    fn battery_device(state: u32, percentage: f64) -> PowerDeviceData {
        PowerDeviceData {
            path: "/org/freedesktop/UPower/devices/battery_BAT0".to_owned(),
            kind: DEVICE_TYPE_BATTERY,
            present: true,
            power_supply: true,
            percentage,
            state,
            battery_level: 4,
            capacity: 96.0,
            charge_cycles: 112,
            time_to_empty: 0,
            time_to_full: 0,
        }
    }

    fn line_power() -> PowerDeviceData {
        PowerDeviceData {
            path: "/org/freedesktop/UPower/devices/line_power_AC".to_owned(),
            kind: 1,
            present: true,
            power_supply: true,
            ..PowerDeviceData::default()
        }
    }

    #[test]
    fn a_charging_battery_is_mapped() {
        let data = PowerData {
            on_battery: false,
            devices: vec![line_power(), battery_device(1, 82.0)],
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert!(snapshot.present());
        assert_eq!(snapshot.percentage_percent(), 82);
        assert_eq!(snapshot.state(), ChargeState::Charging);
        assert!(snapshot.charging());
        assert!(snapshot.plugged());
        assert!(!snapshot.on_battery());
        assert_eq!(snapshot.glyph(), "battery");
        assert_eq!(snapshot.label(), "82% charging");
        assert!((snapshot.level() - 0.82).abs() < 0.001);
    }

    #[test]
    fn a_discharging_battery_reports_its_time_to_empty() {
        let mut device = battery_device(2, 41.0);
        device.time_to_empty = 7200;
        let data = PowerData {
            on_battery: true,
            devices: vec![device],
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert!(snapshot.present());
        assert!(snapshot.on_battery());
        assert!(snapshot.battery().unwrap().is_discharging());
        assert!(!snapshot.charging());
        assert_eq!(snapshot.label(), "41%");
        assert_eq!(snapshot.time_to_empty(), Some(7200));
        assert_eq!(snapshot.time_to_full(), None);
    }

    #[test]
    fn a_fully_charged_battery_has_its_own_label() {
        let data = PowerData {
            on_battery: false,
            devices: vec![battery_device(4, 100.0)],
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert!(snapshot.present());
        assert!(snapshot.plugged());
        assert!(!snapshot.charging());
        assert_eq!(snapshot.label(), "100% charged");
    }

    #[test]
    fn a_battery_slot_without_a_cell_is_not_reported() {
        let mut device = battery_device(2, 0.0);
        device.present = false;
        let data = PowerData {
            on_battery: false,
            devices: vec![device],
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert!(!snapshot.present());
        assert_eq!(snapshot.battery(), None);
        assert_eq!(snapshot.glyph(), "battery");
        assert_eq!(snapshot.label(), "No battery");
        assert_eq!(snapshot.level(), 0.0);
    }

    #[test]
    fn a_desktop_with_only_line_power_has_no_battery() {
        let data = PowerData {
            on_battery: false,
            devices: vec![line_power()],
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert!(!snapshot.present());
        assert_eq!(snapshot.state(), ChargeState::Unknown);
        assert_eq!(snapshot.label(), "No battery");
    }

    #[test]
    fn an_out_of_range_percentage_is_clamped() {
        let data = PowerData {
            on_battery: true,
            devices: vec![battery_device(2, 240.0)],
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert_eq!(snapshot.percentage(), 100.0);
        assert_eq!(snapshot.percentage_percent(), 100);
        assert_eq!(snapshot.level(), 1.0);
    }

    #[test]
    fn unknown_state_maps_to_unknown() {
        assert_eq!(ChargeState::from_u32(99), ChargeState::Unknown);
        assert_eq!(BatteryLevel::from_u32(99), BatteryLevel::Unknown);
    }

    #[test]
    fn the_label_names_every_state() {
        assert_eq!(ChargeState::PendingCharge.name(), "pending-charge");
        assert_eq!(ChargeState::PendingDischarge.name(), "pending-discharge");
        assert_eq!(BatteryLevel::Critical.name(), "critical");
        assert_eq!(BatteryLevel::Full.name(), "full");
    }

    fn profiles_data(active: &str) -> PowerProfilesData {
        PowerProfilesData {
            active_profile: active.to_owned(),
            profiles: vec![
                PowerProfileData {
                    profile: "power-saver".to_owned(),
                    driver: "amd_pstate".to_owned(),
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
        }
    }

    #[test]
    fn the_battery_reports_capacity_health_and_cycles() {
        let data = PowerData {
            devices: vec![battery_device(2, 50.0)],
            ..PowerData::default()
        };
        let battery = PowerSnapshot::from_data(&data).battery().unwrap().clone();
        assert_eq!(battery.capacity, Some(96));
        assert_eq!(battery.charge_cycles, Some(112));
        assert_eq!(battery.health(), BatteryHealth::Normal);
        assert_eq!(battery.health().label(), "Normal");

        let mut worn = battery_device(2, 50.0);
        worn.capacity = 55.0;
        worn.charge_cycles = 0;
        let worn = PowerSnapshot::from_data(&PowerData {
            devices: vec![worn],
            ..PowerData::default()
        });
        assert_eq!(worn.health(), BatteryHealth::Service);
        assert_eq!(worn.battery().unwrap().charge_cycles, None);
    }

    #[test]
    fn unknown_capacity_is_unknown_health() {
        let mut device = battery_device(2, 50.0);
        device.capacity = 0.0;
        let snapshot = PowerSnapshot::from_data(&PowerData {
            devices: vec![device],
            ..PowerData::default()
        });
        assert_eq!(snapshot.health(), BatteryHealth::Unknown);
        assert_eq!(snapshot.health().name(), "unknown");
    }

    #[test]
    fn profiles_map_to_the_canonical_available_set() {
        let data = PowerData {
            profiles: Some(profiles_data("balanced")),
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert!(snapshot.profiles_available());
        let profiles = snapshot.profiles().unwrap();
        assert_eq!(profiles.available, PowerProfile::ALL.to_vec());
        assert_eq!(snapshot.active_profile(), Some(PowerProfile::Balanced));
        assert_eq!(snapshot.profile_label(), "Balanced");
        assert!(profiles.supports(PowerProfile::Performance));
        assert!(!profiles.degraded());
        assert_eq!(profiles.glyph(), "power-balanced");
    }

    #[test]
    fn a_subset_of_profiles_is_preserved_in_canonical_order() {
        let data = PowerData {
            profiles: Some(PowerProfilesData {
                active_profile: "power-saver".to_owned(),
                profiles: vec![
                    PowerProfileData {
                        profile: "performance".to_owned(),
                        ..PowerProfileData::default()
                    },
                    PowerProfileData {
                        profile: "power-saver".to_owned(),
                        ..PowerProfileData::default()
                    },
                ],
                ..PowerProfilesData::default()
            }),
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert_eq!(
            snapshot.profiles().unwrap().available,
            vec![PowerProfile::PowerSaver, PowerProfile::Performance]
        );
        assert!(!snapshot
            .profiles()
            .unwrap()
            .supports(PowerProfile::Balanced));
    }

    #[test]
    fn an_unknown_active_profile_is_not_guessed() {
        let data = PowerData {
            profiles: Some(PowerProfilesData {
                active_profile: "turbo-experimental".to_owned(),
                profiles: vec![PowerProfileData {
                    profile: "balanced".to_owned(),
                    ..PowerProfileData::default()
                }],
                ..PowerProfilesData::default()
            }),
            ..PowerData::default()
        };
        let snapshot = PowerSnapshot::from_data(&data);
        assert_eq!(snapshot.active_profile(), None);
        assert_eq!(snapshot.profile_label(), "Unknown");
    }

    #[test]
    fn no_profiles_daemon_reports_unavailable() {
        let snapshot = PowerSnapshot::from_data(&PowerData::default());
        assert!(!snapshot.profiles_available());
        assert_eq!(snapshot.profile_label(), "Unavailable");
        assert_eq!(snapshot.profiles(), None);
    }

    #[test]
    fn performance_degradation_is_carried() {
        let mut profiles = profiles_data("performance");
        profiles.performance_inhibited = "lap-detected".to_owned();
        profiles.performance_degraded = "lap-detected".to_owned();
        profiles.holds = 2;
        let snapshot = PowerSnapshot::from_data(&PowerData {
            profiles: Some(profiles),
            ..PowerData::default()
        });
        let profiles = snapshot.profiles().unwrap();
        assert!(profiles.performance_inhibited);
        assert!(profiles.degraded());
        assert_eq!(profiles.performance_degraded, "lap-detected");
        assert_eq!(profiles.holds, 2);
    }

    #[test]
    fn battery_and_profile_moves_are_changes() {
        let previous = PowerSnapshot::from_data(&PowerData {
            devices: vec![battery_device(1, 50.0)],
            profiles: Some(profiles_data("balanced")),
            ..PowerData::default()
        });
        let mut device = battery_device(2, 40.0);
        device.capacity = 55.0;
        let next = PowerSnapshot::from_data(&PowerData {
            on_battery: true,
            devices: vec![device],
            profiles: Some(PowerProfilesData {
                active_profile: "power-saver".to_owned(),
                profiles: vec![PowerProfileData {
                    profile: "power-saver".to_owned(),
                    ..PowerProfileData::default()
                }],
                performance_degraded: "lap-detected".to_owned(),
                holds: 1,
                ..PowerProfilesData::default()
            }),
        });
        let changes = next.changes(&previous);
        assert!(changes.contains(&PowerChange::BatteryLevelChanged { from: 50, to: 40 }));
        assert!(changes.contains(&PowerChange::ChargeStateChanged {
            from: ChargeState::Charging,
            to: ChargeState::Discharging,
        }));
        assert!(changes.contains(&PowerChange::BatteryHealthChanged {
            from: Some(96),
            to: Some(55),
        }));
        assert!(changes.contains(&PowerChange::OnBatteryChanged {
            from: false,
            to: true,
        }));
        assert!(changes.contains(&PowerChange::AvailableProfilesChanged {
            from: PowerProfile::ALL.to_vec(),
            to: vec![PowerProfile::PowerSaver],
        }));
        assert!(changes.contains(&PowerChange::ActiveProfileChanged {
            from: Some(PowerProfile::Balanced),
            to: Some(PowerProfile::PowerSaver),
        }));
        assert!(changes.contains(&PowerChange::PerformanceDegradedChanged {
            from: String::new(),
            to: "lap-detected".to_owned(),
        }));
        assert!(changes.contains(&PowerChange::ProfileHoldsChanged { from: 0, to: 1 }));
    }

    #[test]
    fn battery_appearance_and_removal_are_changes() {
        let with_battery = PowerSnapshot::from_data(&PowerData {
            devices: vec![battery_device(2, 50.0)],
            ..PowerData::default()
        });
        let empty = PowerSnapshot::from_data(&PowerData::default());
        assert_eq!(
            with_battery.changes(&empty),
            vec![PowerChange::BatteryAppeared]
        );
        assert_eq!(
            empty.changes(&with_battery),
            vec![PowerChange::BatteryRemoved]
        );
    }

    #[test]
    fn an_unchanged_snapshot_has_no_changes() {
        let snapshot = PowerSnapshot::from_data(&PowerData {
            devices: vec![battery_device(2, 50.0)],
            profiles: Some(profiles_data("balanced")),
            ..PowerData::default()
        });
        assert!(snapshot.changes(&snapshot).is_empty());
    }

    #[test]
    fn profile_ids_and_glyphs_round_trip() {
        for profile in PowerProfile::ALL {
            assert_eq!(PowerProfile::from_id(profile.id()), Some(profile));
            assert!(!profile.label().is_empty());
            assert!(profile.glyph().starts_with("power-"));
        }
        assert_eq!(PowerProfile::from_id("bogus"), None);
    }
}
