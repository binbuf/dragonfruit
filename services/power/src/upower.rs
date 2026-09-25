// SPDX-License-Identifier: MIT
//! The live transport: UPower over its D-Bus API.
//!
//! UPower is reached over the **system bus**
//! ([07-system-integration.md](../../../docs/design/07-system-integration.md)).
//! The read is `org.freedesktop.UPower.EnumerateDevices` plus the manager's
//! `OnBattery` and each device's `org.freedesktop.UPower.Device` properties.
//! There is no write: the battery item is read-only (power profiles are
//! deferred to T-15).
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. A call that cannot
//! reach the bus at all, or a bus where UPower does not own its name, is
//! treated as absence; a name owned but unreadable is reported to the adapter,
//! which shows the item visible and inert.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedObjectPath;

use crate::source::{PowerData, PowerDeviceData, PowerSource};

/// The well-known name `UPower` owns.
pub const UPOWER_SERVICE: &str = "org.freedesktop.UPower";

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

/// The real UPower transport.
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
        // No system bus means no UPower can be reached: absence, not an error,
        // and never a startup blocker.
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };

        if !name_has_owner(&connection).map_err(failed)? {
            return Ok(None);
        }

        let manager = UPowerProxyBlocking::new(&connection).map_err(failed)?;
        let on_battery = manager.on_battery().map_err(failed)?;
        let device_paths = manager.enumerate_devices().map_err(failed)?;

        let mut devices = Vec::with_capacity(device_paths.len());
        for device_path in device_paths {
            let device =
                DeviceProxyBlocking::new(&connection, device_path.as_str()).map_err(failed)?;
            devices.push(PowerDeviceData {
                path: device_path.as_str().to_owned(),
                kind: device.device_type().map_err(failed)?,
                present: device.is_present().map_err(failed)?,
                power_supply: device.power_supply().map_err(failed)?,
                percentage: device.percentage().map_err(failed)?,
                state: device.state().map_err(failed)?,
                battery_level: device.battery_level().map_err(failed)?,
                time_to_empty: device.time_to_empty().map_err(failed)?,
                time_to_full: device.time_to_full().map_err(failed)?,
            });
        }

        Ok(Some(PowerData {
            on_battery,
            devices,
        }))
    }
}

/// Whether `org.freedesktop.UPower` currently owns its name.
fn name_has_owner(connection: &Connection) -> zbus::Result<bool> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(UPOWER_SERVICE,),
    )?;
    reply.body().deserialize::<bool>()
}

/// One read failure on the live path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("UPower: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _source = DbusUPower::new();
    }

    #[test]
    fn a_read_failure_carries_the_daemon_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("UPower: "));
    }

    #[test]
    fn the_service_name_is_the_upower_well_known_name() {
        assert_eq!(UPOWER_SERVICE, "org.freedesktop.UPower");
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
}
