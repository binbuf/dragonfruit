// SPDX-License-Identifier: MIT
//! The live transport: NetworkManager over its D-Bus API.
//!
//! NetworkManager is reached over the **system bus**, never through `libnm`
//! ([07-system-integration.md]). Every call here is a read; the write path
//! (activate/join) is T-07.2b and deliberately absent.
//!
//! The source constructs no connection until [`read`](NetworkManagerSource::read),
//! so building an adapter is free and a session without a bus still boots. A
//! read that cannot reach the bus at all is treated as absence (`Ok(None)`);
//! a name owned but unreadable is an error (`Err`), which the consumer shows
//! visible and inert.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedObjectPath;

use crate::source::{AccessPointData, NetworkManagerData, NetworkManagerSource, WifiDeviceData};

/// The well-known name `NetworkManager` owns.
pub const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
/// `NM_DEVICE_TYPE_WIFI`.
const NM_DEVICE_TYPE_WIFI: u32 = 2;

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn connectivity(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Device {
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn managed(&self) -> zbus::Result<bool>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Wireless {
    #[zbus(property)]
    fn access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
trait AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;
    #[zbus(property)]
    fn strength(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn frequency(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn wpa_flags(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn rsn_flags(&self) -> zbus::Result<u32>;
}

/// The real NetworkManager transport.
#[derive(Debug, Default, Clone, Copy)]
pub struct DbusNetworkManager;

impl DbusNetworkManager {
    /// A source that reads the system bus on demand.
    pub const fn new() -> Self {
        DbusNetworkManager
    }
}

impl NetworkManagerSource for DbusNetworkManager {
    fn read(&mut self) -> Result<Option<NetworkManagerData>, AdapterError> {
        // No system bus means no NetworkManager can be reached: absence, not
        // an error, and never a startup blocker.
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };

        if !name_has_owner(&connection).map_err(failed)? {
            return Ok(None);
        }

        let manager = NetworkManagerProxyBlocking::new(&connection).map_err(failed)?;
        let wireless_enabled = manager.wireless_enabled().map_err(failed)?;
        let connectivity = manager.connectivity().map_err(failed)?;
        let device_paths = manager.devices().map_err(failed)?;

        let mut devices = Vec::new();
        for device_path in device_paths {
            let path = device_path.as_str();
            let device = DeviceProxyBlocking::new(&connection, path).map_err(failed)?;
            if device.device_type().map_err(failed)? != NM_DEVICE_TYPE_WIFI {
                continue;
            }
            let managed = device.managed().map_err(failed)?;
            let state = device.state().map_err(failed)?;

            let wireless = WirelessProxyBlocking::new(&connection, path).map_err(failed)?;
            let ap_paths = wireless.access_points().map_err(failed)?;
            let active = wireless.active_access_point().map_err(failed)?;
            let active_path = active.as_str().to_owned();
            // A disconnected wireless device reports the root object path.
            let active_ap = (active_path != "/").then_some(active_path);

            let mut access_points = Vec::with_capacity(ap_paths.len());
            for ap_path in ap_paths {
                let ap =
                    AccessPointProxyBlocking::new(&connection, ap_path.as_str()).map_err(failed)?;
                access_points.push(AccessPointData {
                    path: ap_path.as_str().to_owned(),
                    ssid: String::from_utf8_lossy(&ap.ssid().map_err(failed)?).into_owned(),
                    strength: ap.strength().map_err(failed)?,
                    frequency_mhz: ap.frequency().map_err(failed)?,
                    flags: ap.flags().map_err(failed)?,
                    wpa_flags: ap.wpa_flags().map_err(failed)?,
                    rsn_flags: ap.rsn_flags().map_err(failed)?,
                });
            }

            devices.push(WifiDeviceData {
                managed,
                state,
                active_ap,
                access_points,
            });
        }

        Ok(Some(NetworkManagerData {
            wireless_enabled,
            connectivity,
            devices,
        }))
    }
}

/// Whether `org.freedesktop.NetworkManager` currently owns its name.
fn name_has_owner(connection: &Connection) -> zbus::Result<bool> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(NM_SERVICE,),
    )?;
    reply.body().deserialize::<bool>()
}

/// One read failure on the live path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("NetworkManager: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _source = DbusNetworkManager::new();
    }

    #[test]
    fn a_read_failure_carries_the_daemon_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("NetworkManager: "));
    }
}
