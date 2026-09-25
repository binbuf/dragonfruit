// SPDX-License-Identifier: MIT
//! The live transport: NetworkManager over its D-Bus API.
//!
//! NetworkManager is reached over the **system bus**, never through `libnm`
//! ([07-system-integration.md]). The read path is [`NetworkManagerSource::read`];
//! the write path is [`NetworkManagerSource::activate`], which joins a network
//! with `AddAndActivateConnection` and classifies a polkit refusal so the
//! adapter can degrade to read-only.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. A call that cannot
//! reach the bus at all is treated as absence; a name owned but unreadable (or
//! a refused join) is reported to the adapter, which decides how to render it.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use std::collections::HashMap;

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, Value};

use crate::source::{
    AccessPointData, ActivateOutcome, ActivateRequest, NetworkManagerData, NetworkManagerSource,
    WifiDeviceData,
};

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

    fn activate(&mut self, request: &ActivateRequest) -> ActivateOutcome {
        // No system bus means NetworkManager is absent: there is nothing to
        // join, and it is never a startup blocker.
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return ActivateOutcome::Absent,
        };
        match name_has_owner(&connection) {
            Ok(false) | Err(_) => return ActivateOutcome::Absent,
            Ok(true) => {}
        }

        match activate_on(&connection, request) {
            Ok(()) => ActivateOutcome::Accepted,
            Err(error) => classify_activation_error(error),
        }
    }
}

/// Perform one `AddAndActivateConnection` for `request`, resolving the Wi-Fi
/// device and (unless the caller named one) the strongest matching BSSID.
fn activate_on(connection: &Connection, request: &ActivateRequest) -> zbus::Result<()> {
    let Some(device_path) = wifi_device_path(connection)? else {
        return Err(zbus::Error::Failure(
            "no managed Wi-Fi device to activate".to_owned(),
        ));
    };
    let specific = match &request.access_point {
        Some(path) => path.clone(),
        None => ap_path_for_ssid(connection, device_path.as_str(), &request.ssid)?
            .ok_or_else(|| zbus::Error::Failure(format!("unknown network '{}'", request.ssid)))?,
    };

    let settings = activation_settings(&request.ssid, request.secret.as_deref());
    let device = ObjectPath::try_from(device_path.as_str())?;
    let specific = ObjectPath::try_from(specific.as_str())?;
    let reply = connection.call_method(
        Some(NM_SERVICE),
        "/org/freedesktop/NetworkManager",
        Some("org.freedesktop.NetworkManager"),
        "AddAndActivateConnection",
        &(settings, device, specific),
    )?;
    // The reply is `(o connection, o active_connection)`; we only need to know
    // it was accepted, not the paths.
    let _: (OwnedObjectPath, OwnedObjectPath) = reply.body().deserialize()?;
    Ok(())
}

/// The path of the first managed Wi-Fi device.
fn wifi_device_path(connection: &Connection) -> zbus::Result<Option<OwnedObjectPath>> {
    let manager = NetworkManagerProxyBlocking::new(connection)?;
    for device_path in manager.devices()? {
        let path = device_path.as_str().to_owned();
        let device = DeviceProxyBlocking::new(connection, path.as_str())?;
        if device.device_type()? == NM_DEVICE_TYPE_WIFI {
            return Ok(Some(device_path));
        }
    }
    Ok(None)
}

/// The strongest access point on `device` whose SSID matches `ssid`.
fn ap_path_for_ssid(
    connection: &Connection,
    device_path: &str,
    ssid: &str,
) -> zbus::Result<Option<String>> {
    let wireless = WirelessProxyBlocking::new(connection, device_path)?;
    let mut best: Option<(String, u8)> = None;
    for ap_path in wireless.access_points()? {
        let ap = AccessPointProxyBlocking::new(connection, ap_path.as_str())?;
        let ap_ssid = String::from_utf8_lossy(&ap.ssid()?).into_owned();
        if ap_ssid != ssid {
            continue;
        }
        let strength = ap.strength()?;
        if best.as_ref().map_or(true, |(_, best)| strength > *best) {
            best = Some((ap_path.as_str().to_owned(), strength));
        }
    }
    Ok(best.map(|(path, _)| path))
}

/// Build the NetworkManager connection settings for an open or PSK network.
///
/// The `connection` and `802-11-wireless` settings are required; a secret adds
/// the `802-11-wireless-security` setting. NetworkManager normalises the
/// missing id/uuid, so we do not invent one.
fn activation_settings(
    ssid: &str,
    secret: Option<&str>,
) -> HashMap<String, HashMap<String, Value<'static>>> {
    let mut connection_settings: HashMap<String, Value<'static>> = HashMap::new();
    connection_settings.insert("id".to_owned(), Value::from(ssid.to_owned()));
    connection_settings.insert("type".to_owned(), Value::from("802-11-wireless"));

    let mut wireless: HashMap<String, Value<'static>> = HashMap::new();
    wireless.insert("ssid".to_owned(), Value::from(ssid.as_bytes().to_vec()));
    wireless.insert("mode".to_owned(), Value::from("infrastructure"));

    let mut settings: HashMap<String, HashMap<String, Value<'static>>> = HashMap::new();
    settings.insert("connection".to_owned(), connection_settings);
    settings.insert("802-11-wireless".to_owned(), wireless);

    if let Some(secret) = secret {
        let mut security: HashMap<String, Value<'static>> = HashMap::new();
        security.insert("key-mgmt".to_owned(), Value::from("wpa-psk"));
        security.insert("psk".to_owned(), Value::from(secret.to_owned()));
        settings.insert("802-11-wireless-security".to_owned(), security);
    }

    settings
}

/// Map a D-Bus error from `AddAndActivateConnection` to the adapter outcome.
///
/// A polkit refusal (by error name or message) is a read-only degradation, not
/// a failure; everything else is a plain failure.
fn classify_activation_error(error: zbus::Error) -> ActivateOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    if is_permission_denied(name, &error.to_string()) {
        // Record the daemon's own message in the note.
        ActivateOutcome::Denied(format!("NetworkManager: {error}"))
    } else {
        ActivateOutcome::Failed(AdapterError::new(format!("NetworkManager: {error}")))
    }
}

/// Whether an error name or message reports an authorization refusal.
fn is_permission_denied(name: &str, message: &str) -> bool {
    const NAMES: [&str; 3] = [
        "org.freedesktop.NetworkManager.PermissionDenied",
        "org.freedesktop.DBus.Error.AccessDenied",
        "org.freedesktop.PolicyKit1.Error.NotAuthorized",
    ];
    if NAMES.contains(&name) {
        return true;
    }
    let lower = message.to_lowercase();
    lower.contains("not authorized")
        || lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("polkit")
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

    #[test]
    fn polkit_names_and_messages_classify_as_denied() {
        assert!(is_permission_denied(
            "org.freedesktop.NetworkManager.PermissionDenied",
            "whatever"
        ));
        assert!(is_permission_denied(
            "org.freedesktop.DBus.Error.AccessDenied",
            "whatever"
        ));
        assert!(is_permission_denied(
            "org.freedesktop.PolicyKit1.Error.NotAuthorized",
            "whatever"
        ));
        assert!(is_permission_denied("", "Not authorized to join"));
        assert!(is_permission_denied("", "polkit refused the request"));
        assert!(!is_permission_denied(
            "org.freedesktop.NetworkManager.UnknownConnection",
            "no such connection"
        ));
    }

    #[test]
    fn a_polkit_error_becomes_a_denial_outcome() {
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "polkit refused the request".to_owned(),
        )));
        let outcome = classify_activation_error(error);
        match outcome {
            ActivateOutcome::Denied(note) => {
                assert!(note.starts_with("NetworkManager: "));
            }
            other => panic!("expected a denial, got {other:?}"),
        }
    }

    #[test]
    fn a_non_authorization_error_is_a_failure() {
        let error = zbus::Error::Failure("unknown network 'ghost'".to_owned());
        assert_eq!(
            classify_activation_error(error),
            ActivateOutcome::Failed(AdapterError::new("NetworkManager: unknown network 'ghost'"))
        );
    }

    #[test]
    fn activation_settings_carry_the_ssid_and_optional_secret() {
        let open = activation_settings("cafe", None);
        assert!(open.contains_key("connection"));
        assert!(open.contains_key("802-11-wireless"));
        assert!(!open.contains_key("802-11-wireless-security"));

        let secured = activation_settings("home", Some("hunter2"));
        let security = secured
            .get("802-11-wireless-security")
            .expect("security setting present");
        assert_eq!(security.get("key-mgmt"), Some(&Value::from("wpa-psk")));
        assert_eq!(security.get("psk"), Some(&Value::from("hunter2")));
    }
}
