// SPDX-License-Identifier: MIT
//! The live transport: UPower and power-profiles-daemon over D-Bus.
//!
//! Both daemons are reached over the **system bus**
//! ([07-system-integration.md](../../../docs/design/07-system-integration.md))
//! and are independent: either can be absent on its own.
//!
//! * UPower (`org.freedesktop.UPower`) — the read is
//!   `org.freedesktop.UPower.EnumerateDevices` plus the manager's `OnBattery`
//!   and each device's `org.freedesktop.UPower.Device` properties (including
//!   `Capacity` and `ChargeCycles`).
//! * power-profiles-daemon — the read is the manager's `ActiveProfile`,
//!   `Profiles`, `PerformanceInhibited`, `PerformanceDegraded`, and
//!   `ActiveProfileHolds` properties; the one write is a
//!   `Properties.Set` of `ActiveProfile`.
//!
//! power-profiles-daemon moved its well-known name from
//! `net.hadess.PowerProfiles` to `org.freedesktop.UPower.PowerProfiles` (with
//! a matching object path and interface). The source probes both and speaks to
//! whichever owns its name, so both daemon generations work.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. A call that cannot
//! reach the bus at all, or a bus where neither daemon owns its name, is
//! treated as absence; a name owned but unreadable is reported to the adapter,
//! which shows the item visible and inert.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use std::collections::HashMap;

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value};

use crate::model::PowerProfile;
use crate::source::{
    PowerData, PowerDeviceData, PowerProfileData, PowerProfilesData, PowerSource, ProfileOutcome,
};

/// The well-known name `UPower` owns.
pub const UPOWER_SERVICE: &str = "org.freedesktop.UPower";
/// The UPower device interface.
pub const UPOWER_DEVICE_INTERFACE: &str = "org.freedesktop.UPower.Device";
/// The standard properties interface.
pub const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
/// The upstream power-profiles-daemon well-known name.
pub const POWER_PROFILES_SERVICE: &str = "net.hadess.PowerProfiles";
/// The upstream power-profiles-daemon object path.
pub const POWER_PROFILES_PATH: &str = "/net/hadess/PowerProfiles";
/// The upstream power-profiles-daemon interface.
pub const POWER_PROFILES_INTERFACE: &str = "net.hadess.PowerProfiles";
/// The power-profiles-daemon name since it moved under UPower.
pub const UPOWER_POWER_PROFILES_SERVICE: &str = "org.freedesktop.UPower.PowerProfiles";
/// The object path for the UPower-named power-profiles-daemon.
pub const UPOWER_POWER_PROFILES_PATH: &str = "/org/freedesktop/UPower/PowerProfiles";
/// The interface for the UPower-named power-profiles-daemon.
pub const UPOWER_POWER_PROFILES_INTERFACE: &str = "org.freedesktop.UPower.PowerProfiles";

#[zbus::proxy(
    interface = "org.freedesktop.UPower",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower"
)]
trait UPower {
    /// Whether the system is running on battery rather than line power.
    #[zbus(property, name = "OnBattery")]
    fn on_battery(&self) -> zbus::Result<bool>;

    /// The object paths of every power device UPower knows.
    fn enumerate_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[zbus::proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower"
)]
trait Device {
    #[zbus(property, name = "Type")]
    fn device_type(&self) -> zbus::Result<u32>;
    #[zbus(property, name = "IsPresent")]
    fn is_present(&self) -> zbus::Result<bool>;
    #[zbus(property, name = "PowerSupply")]
    fn power_supply(&self) -> zbus::Result<bool>;
    #[zbus(property, name = "Percentage")]
    fn percentage(&self) -> zbus::Result<f64>;
    #[zbus(property, name = "State")]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property, name = "BatteryLevel")]
    fn battery_level(&self) -> zbus::Result<u32>;
    #[zbus(property, name = "TimeToEmpty")]
    fn time_to_empty(&self) -> zbus::Result<i64>;
    #[zbus(property, name = "TimeToFull")]
    fn time_to_full(&self) -> zbus::Result<i64>;
}

/// Which well-known name/interface/path pair answers power-profiles-daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PowerProfilesTarget {
    service: &'static str,
    path: &'static str,
    interface: &'static str,
}

impl PowerProfilesTarget {
    const ALL: [PowerProfilesTarget; 2] = [
        PowerProfilesTarget {
            service: POWER_PROFILES_SERVICE,
            path: POWER_PROFILES_PATH,
            interface: POWER_PROFILES_INTERFACE,
        },
        PowerProfilesTarget {
            service: UPOWER_POWER_PROFILES_SERVICE,
            path: UPOWER_POWER_PROFILES_PATH,
            interface: UPOWER_POWER_PROFILES_INTERFACE,
        },
    ];
}

/// The real UPower + power-profiles-daemon transport.
#[derive(Debug, Default, Clone, Copy)]
pub struct DbusUPower;

impl DbusUPower {
    /// A source that reads the system bus on demand.
    pub const fn new() -> Self {
        DbusUPower
    }
}

impl PowerSource for DbusUPower {
    fn read(&mut self) -> Result<Option<PowerData>, AdapterError> {
        // No system bus means neither daemon can be reached: absence, not an
        // error, and never a startup blocker.
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };

        let upower_present = name_has_owner(&connection, UPOWER_SERVICE).map_err(failed)?;
        let profiles_target = power_profiles_target(&connection).map_err(failed)?;
        if !upower_present && profiles_target.is_none() {
            return Ok(None);
        }

        let (on_battery, devices) = if upower_present {
            read_upower(&connection).map_err(failed)?
        } else {
            (false, Vec::new())
        };
        let profiles = match profiles_target {
            Some(target) => Some(read_power_profiles(&connection, target).map_err(failed)?),
            None => None,
        };

        Ok(Some(PowerData {
            on_battery,
            devices,
            profiles,
        }))
    }

    fn set_active_profile(&mut self, profile: PowerProfile) -> ProfileOutcome {
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return ProfileOutcome::Absent,
        };
        let target = match power_profiles_target(&connection) {
            Ok(Some(target)) => target,
            Ok(None) => return ProfileOutcome::Absent,
            Err(error) => return ProfileOutcome::Failed(power_profiles_error(error)),
        };
        let result = connection.call_method(
            Some(target.service),
            target.path,
            Some(PROPERTIES_INTERFACE),
            "Set",
            &(target.interface, "ActiveProfile", Value::from(profile.id())),
        );
        match result {
            Ok(_) => ProfileOutcome::Applied,
            Err(error) => ProfileOutcome::Failed(power_profiles_error(error)),
        }
    }
}

/// Read the whole UPower half: `OnBattery` plus every device.
fn read_upower(connection: &Connection) -> zbus::Result<(bool, Vec<PowerDeviceData>)> {
    let manager = UPowerProxyBlocking::new(connection)?;
    let on_battery = manager.on_battery()?;
    let device_paths = manager.enumerate_devices()?;

    let mut devices = Vec::with_capacity(device_paths.len());
    for device_path in device_paths {
        let device = DeviceProxyBlocking::new(connection, device_path.as_str())?;
        devices.push(PowerDeviceData {
            path: device_path.as_str().to_owned(),
            kind: device.device_type()?,
            present: device.is_present()?,
            power_supply: device.power_supply()?,
            percentage: device.percentage()?,
            state: device.state()?,
            battery_level: device.battery_level()?,
            capacity: numeric_property(
                connection,
                device_path.as_str(),
                UPOWER_DEVICE_INTERFACE,
                "Capacity",
            ),
            charge_cycles: numeric_property(
                connection,
                device_path.as_str(),
                UPOWER_DEVICE_INTERFACE,
                "ChargeCycles",
            ) as i32,
            time_to_empty: device.time_to_empty()?,
            time_to_full: device.time_to_full()?,
        });
    }

    Ok((on_battery, devices))
}

/// Read the power-profiles-daemon half through the resolved target.
fn read_power_profiles(
    connection: &Connection,
    target: PowerProfilesTarget,
) -> zbus::Result<PowerProfilesData> {
    let active_profile = string_property(connection, target, "ActiveProfile");
    let performance_inhibited = string_property(connection, target, "PerformanceInhibited");
    let performance_degraded = string_property(connection, target, "PerformanceDegraded");

    let entries: Vec<HashMap<String, OwnedValue>> = get_property(connection, target, "Profiles")?
        .try_into()
        .map_err(zbus::Error::from)?;
    let profiles = entries.iter().map(profile_from_entry).collect();

    let holds: Vec<HashMap<String, OwnedValue>> =
        get_property(connection, target, "ActiveProfileHolds")?
            .try_into()
            .map_err(zbus::Error::from)?;

    Ok(PowerProfilesData {
        active_profile,
        profiles,
        performance_inhibited,
        performance_degraded,
        holds: holds.len() as u32,
    })
}

/// One `Profiles` entry (`a{sv}`) as the flat raw shape.
fn profile_from_entry(entry: &HashMap<String, OwnedValue>) -> PowerProfileData {
    PowerProfileData {
        profile: string_of(entry.get("Profile")),
        driver: string_of(entry.get("Driver")),
        platform_driver: string_of(entry.get("PlatformDriver")),
    }
}

/// Which power-profiles-daemon name currently owns its name, if either.
fn power_profiles_target(connection: &Connection) -> zbus::Result<Option<PowerProfilesTarget>> {
    for target in PowerProfilesTarget::ALL {
        if name_has_owner(connection, target.service)? {
            return Ok(Some(target));
        }
    }
    Ok(None)
}

/// `Properties.Get(interface, name)` for one power-profiles property.
fn get_property(
    connection: &Connection,
    target: PowerProfilesTarget,
    name: &str,
) -> zbus::Result<OwnedValue> {
    let reply = connection.call_method(
        Some(target.service),
        target.path,
        Some(PROPERTIES_INTERFACE),
        "Get",
        &(target.interface, name),
    )?;
    reply.body().deserialize::<OwnedValue>()
}

/// A string power-profiles property, `""` when absent or of another type.
fn string_property(connection: &Connection, target: PowerProfilesTarget, name: &str) -> String {
    get_property(connection, target, name)
        .ok()
        .map(|value| string_of(Some(&value)))
        .unwrap_or_default()
}

/// The string an `OwnedValue` holds, `""` for any other type or `None`.
fn string_of(value: Option<&OwnedValue>) -> String {
    value
        .and_then(|value| value.downcast_ref::<&str>().ok())
        .map(str::to_owned)
        .unwrap_or_default()
}

/// A numeric device property as `f64`, tolerant of the daemon's integer vs.
/// double spelling; `0.0` when absent or unreadable.
fn numeric_property(connection: &Connection, path: &str, interface: &str, name: &str) -> f64 {
    let reply = connection.call_method(
        Some(UPOWER_SERVICE),
        path,
        Some(PROPERTIES_INTERFACE),
        "Get",
        &(interface, name),
    );
    let value: OwnedValue = match reply.and_then(|reply| reply.body().deserialize()) {
        Ok(value) => value,
        Err(_) => return 0.0,
    };
    if let Ok(number) = value.downcast_ref::<f64>() {
        return number;
    }
    if let Ok(number) = value.downcast_ref::<u32>() {
        return f64::from(number);
    }
    if let Ok(number) = value.downcast_ref::<i32>() {
        return f64::from(number);
    }
    0.0
}

/// Whether `service` currently owns its name.
fn name_has_owner(connection: &Connection, service: &str) -> zbus::Result<bool> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(service,),
    )?;
    reply.body().deserialize::<bool>()
}

/// One read failure on the UPower path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("UPower: {error}"))
}

/// One failure on the power-profiles-daemon path.
fn power_profiles_error(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("power-profiles-daemon: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hadess() -> PowerProfilesTarget {
        PowerProfilesTarget::ALL[0]
    }

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _source = DbusUPower::new();
    }

    #[test]
    fn a_read_failure_carries_the_daemon_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("UPower: "));
        let error = power_profiles_error(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("power-profiles-daemon: "));
    }

    #[test]
    fn the_service_names_match_the_daemons() {
        assert_eq!(UPOWER_SERVICE, "org.freedesktop.UPower");
        assert_eq!(POWER_PROFILES_SERVICE, "net.hadess.PowerProfiles");
        assert_eq!(
            UPOWER_POWER_PROFILES_SERVICE,
            "org.freedesktop.UPower.PowerProfiles"
        );
        assert_eq!(UPOWER_DEVICE_INTERFACE, "org.freedesktop.UPower.Device");
    }

    #[test]
    fn the_target_roster_covers_both_daemon_generations() {
        assert_eq!(PowerProfilesTarget::ALL.len(), 2);
        assert_eq!(
            PowerProfilesTarget::ALL[0].service,
            "net.hadess.PowerProfiles"
        );
        assert_eq!(
            PowerProfilesTarget::ALL[1].service,
            "org.freedesktop.UPower.PowerProfiles"
        );
        assert_eq!(
            PowerProfilesTarget::ALL[1].path,
            "/org/freedesktop/UPower/PowerProfiles"
        );
    }

    #[test]
    fn a_profile_entry_decodes_the_capitalized_keys() {
        let mut entry: HashMap<String, OwnedValue> = HashMap::new();
        entry.insert(
            "Profile".to_owned(),
            OwnedValue::try_from(Value::from("performance")).unwrap(),
        );
        entry.insert(
            "Driver".to_owned(),
            OwnedValue::try_from(Value::from("amd_pstate")).unwrap(),
        );
        entry.insert(
            "PlatformDriver".to_owned(),
            OwnedValue::try_from(Value::from("amd_pstate_epp")).unwrap(),
        );
        let decoded = profile_from_entry(&entry);
        assert_eq!(decoded.profile, "performance");
        assert_eq!(decoded.driver, "amd_pstate");
        assert_eq!(decoded.platform_driver, "amd_pstate_epp");
    }

    #[test]
    fn string_of_reads_a_string_and_ignores_other_types() {
        let value = OwnedValue::try_from(Value::from("balanced")).unwrap();
        assert_eq!(string_of(Some(&value)), "balanced");
        let number = OwnedValue::try_from(Value::from(7u32)).unwrap();
        assert_eq!(string_of(Some(&number)), "");
        assert_eq!(string_of(None), "");
    }

    #[test]
    fn the_device_value_type_is_usable_for_paths() {
        // The proxy deserialises object paths; make sure the imported type is
        // the one `enumerate_devices` returns.
        let path = OwnedObjectPath::try_from("/org/freedesktop/UPower/devices/battery_BAT0")
            .expect("a valid object path");
        assert_eq!(
            path.as_str(),
            "/org/freedesktop/UPower/devices/battery_BAT0"
        );
    }

    #[test]
    fn the_hadess_target_is_the_first_probe() {
        assert_eq!(hadess().service, POWER_PROFILES_SERVICE);
    }
}
