// SPDX-License-Identifier: MIT
//! The battery snapshot the menu bar renders, decoded from one raw read.
//!
//! The model owns the UPower enum mapping and the aggregation a consumer should
//! not repeat: it picks the system battery, clamps its percentage to 0–100,
//! and derives the glyph, label, and fill level the status item draws. T-07.5
//! renders [`PowerSnapshot::glyph`] / [`PowerSnapshot::label`] and the
//! [`PowerSnapshot::level`] fill.

use crate::source::{PowerData, PowerDeviceData, DEVICE_TYPE_BATTERY};

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

/// The system battery the menu bar reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Battery {
    /// The charge, clamped to 0–100.
    pub percentage: f32,
    /// Whether the battery is charging, discharging, or in between.
    pub state: ChargeState,
    /// The coarse level bucket UPower reports.
    pub level: BatteryLevel,
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

    /// Whether the battery is on the way up.
    pub fn is_charging(&self) -> bool {
        self.state.is_charging()
    }

    /// Whether the battery is supplying power.
    pub fn is_discharging(&self) -> bool {
        self.state.is_discharging()
    }
}

/// The power snapshot a menu bar renders: the battery and the system's power
/// source.
///
/// `on_battery` is the manager's `OnBattery` property; `battery` is `None` on
/// a machine without a present battery (a desktop, a VM), so a consumer can
/// hide the item. UPower being absent is a different, adapter-level state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PowerSnapshot {
    /// The system battery, when UPower reports a present one.
    pub battery: Option<Battery>,
    /// `org.freedesktop.UPower.OnBattery`.
    pub on_battery: bool,
}

impl PowerSnapshot {
    /// Build the snapshot from one raw read: pick the first present battery
    /// and map its charge state and level.
    pub fn from_data(data: &PowerData) -> Self {
        let battery = data
            .devices
            .iter()
            .find(|device| is_present_battery(device))
            .map(battery_from_device);
        PowerSnapshot {
            battery,
            on_battery: data.on_battery,
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
        time_to_empty: seconds(device.time_to_empty),
        time_to_full: seconds(device.time_to_full),
    }
}

/// UPower reports 0 seconds for "unknown"; keep only a real duration.
fn seconds(value: i64) -> Option<u64> {
    (value > 0).then_some(value as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn battery_device(state: u32, percentage: f64) -> PowerDeviceData {
        PowerDeviceData {
            path: "/org/freedesktop/UPower/devices/battery_BAT0".to_owned(),
            kind: DEVICE_TYPE_BATTERY,
            present: true,
            power_supply: true,
            percentage,
            state,
            battery_level: 4,
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
}
