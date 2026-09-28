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
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use crate::source::{
    AccessPointData, ActivateOutcome, ActivateRequest, NetworkManagerData, NetworkManagerSource,
    WifiDeviceData,
};
use crate::vpn::source::{
    ActiveVpnData, VpnConnectionData, VpnData, VpnOutcome, VpnRequest, VpnSource,
};

/// The well-known name `NetworkManager` owns.
pub const NM_SERVICE: &str = "org.freedesktop.NetworkManager";
/// The object path of the NetworkManager settings service.
pub const VPN_SETTINGS_PATH: &str = "/org/freedesktop/NetworkManager/Settings";
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
    #[zbus(property)]
    fn active_connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
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

/// The NetworkManager settings service, `/org/freedesktop/NetworkManager/
/// Settings`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Settings",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager/Settings"
)]
trait NmSettings {
    #[zbus(property)]
    fn connections(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    fn get_connection_by_uuid(&self, uuid: &str) -> zbus::Result<OwnedObjectPath>;
}

/// One live active connection, `org.freedesktop.NetworkManager.Connection.
/// Active`.
#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
trait ActiveConnection {
    #[zbus(property)]
    fn connection(&self) -> zbus::Result<OwnedObjectPath>;
    #[zbus(property)]
    fn uuid(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn id(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn vpn(&self) -> zbus::Result<bool>;
    #[zbus(property, name = "Type")]
    fn connection_type(&self) -> zbus::Result<String>;
}

/// The real NetworkManager VPN transport.
///
/// Reads the configured VPN connections (the settings objects of type `vpn`
/// or `wireguard`) and the live active connections, both over the system bus;
/// writes are `ActivateConnection`/`DeactivateConnection` by UUID. It reuses
/// the same D-Bus client and the same absence discipline as
/// [`DbusNetworkManager`], and reimplements no VPN stack.
#[derive(Debug, Default, Clone, Copy)]
pub struct DbusVpn;

impl DbusVpn {
    /// A source that reads the system bus on demand.
    pub const fn new() -> Self {
        DbusVpn
    }
}

impl VpnSource for DbusVpn {
    fn read(&mut self) -> Result<Option<VpnData>, AdapterError> {
        // No system bus means no NetworkManager can be reached: absence, not
        // an error, and never a startup blocker.
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };
        if !name_has_owner(&connection).map_err(failed)? {
            return Ok(None);
        }
        Ok(Some(read_vpn(&connection).map_err(failed)?))
    }

    fn activate(&mut self, request: &VpnRequest) -> VpnOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        let path = match resolve_connection(&connection, &request.uuid) {
            Ok(path) => path,
            Err(outcome) => return outcome,
        };
        let connection_path = match ObjectPath::try_from(path.as_str()) {
            Ok(path) => path,
            Err(error) => {
                return VpnOutcome::Failed(AdapterError::new(format!("NetworkManager: {error}")))
            }
        };
        let root = ObjectPath::try_from("/").expect("root path is valid");
        match connection.call_method(
            Some(NM_SERVICE),
            "/org/freedesktop/NetworkManager",
            Some("org.freedesktop.NetworkManager"),
            "ActivateConnection",
            &(connection_path, root.clone(), root),
        ) {
            Ok(_) => VpnOutcome::Accepted,
            Err(error) => classify_vpn_error(error),
        }
    }

    fn deactivate(&mut self, request: &VpnRequest) -> VpnOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        let path = match active_path_for_uuid(&connection, &request.uuid) {
            Ok(Some(path)) => path,
            Ok(None) => {
                return VpnOutcome::Failed(AdapterError::new(format!(
                    "NetworkManager: VPN connection '{}' is not active",
                    request.uuid
                )))
            }
            Err(error) => {
                return VpnOutcome::Failed(AdapterError::new(format!("NetworkManager: {error}")))
            }
        };
        let active_path = match ObjectPath::try_from(path.as_str()) {
            Ok(path) => path,
            Err(error) => {
                return VpnOutcome::Failed(AdapterError::new(format!("NetworkManager: {error}")))
            }
        };
        match connection.call_method(
            Some(NM_SERVICE),
            "/org/freedesktop/NetworkManager",
            Some("org.freedesktop.NetworkManager"),
            "DeactivateConnection",
            &(active_path,),
        ) {
            Ok(_) => VpnOutcome::Accepted,
            Err(error) => classify_vpn_error(error),
        }
    }
}

/// Read every configured VPN connection and every live active connection.
fn read_vpn(connection: &Connection) -> zbus::Result<VpnData> {
    let manager = NetworkManagerProxyBlocking::new(connection)?;
    let settings = NmSettingsProxyBlocking::new(connection)?;

    let mut connections = Vec::new();
    for path in settings.connections()? {
        let map = get_connection_settings(connection, path.as_str())?;
        if let Some(connection) = vpn_connection_from_settings(path.as_str(), &map) {
            connections.push(connection);
        }
    }

    let mut active = Vec::new();
    for path in manager.active_connections()? {
        let proxy = ActiveConnectionProxyBlocking::new(connection, path.as_str())?;
        let is_vpn = proxy.vpn().unwrap_or(false);
        let kind = proxy.connection_type().unwrap_or_default();
        if !is_vpn && kind != "vpn" && kind != "wireguard" {
            continue;
        }
        active.push(ActiveVpnData {
            path: path.as_str().to_owned(),
            connection: proxy.connection()?.as_str().to_owned(),
            id: proxy.id().unwrap_or_default(),
            uuid: proxy.uuid()?,
            state: proxy.state()?,
            vpn: is_vpn,
        });
    }

    Ok(VpnData {
        connections,
        active,
    })
}

/// One connection's `a{sa{sv}}` settings map from `GetSettings`.
fn get_connection_settings(
    connection: &Connection,
    path: &str,
) -> zbus::Result<HashMap<String, HashMap<String, OwnedValue>>> {
    let reply = connection.call_method(
        Some(NM_SERVICE),
        path,
        Some("org.freedesktop.NetworkManager.Settings.Connection"),
        "GetSettings",
        &(),
    )?;
    reply.body().deserialize()
}

/// Decode one connection settings map into a [`VpnConnectionData`], or `None`
/// when it is not a VPN connection (a settings object of any other type).
///
/// Pure and total: a missing or mistyped property falls back to a default
/// rather than failing the whole read, so a daemon that adds a field keeps
/// working.
pub fn vpn_connection_from_settings(
    path: &str,
    settings: &HashMap<String, HashMap<String, OwnedValue>>,
) -> Option<VpnConnectionData> {
    let connection = settings.get("connection")?;

    let kind = connection.get("type").and_then(value_string)?;
    if kind != "vpn" && kind != "wireguard" {
        return None;
    }

    let id = connection
        .get("id")
        .and_then(value_string)
        .unwrap_or_default();
    let uuid = connection
        .get("uuid")
        .and_then(value_string)
        .unwrap_or_default();
    let autoconnect = connection
        .get("autoconnect")
        .and_then(value_bool)
        .unwrap_or(false);
    let service_type = settings
        .get("vpn")
        .and_then(|vpn| vpn.get("service-type"))
        .and_then(value_string);

    Some(VpnConnectionData {
        path: path.to_owned(),
        id,
        uuid,
        kind,
        service_type,
        autoconnect,
    })
}

/// A string out of one settings value.
fn value_string(value: &OwnedValue) -> Option<String> {
    value.downcast_ref::<String>().ok()
}

/// A bool out of one settings value.
fn value_bool(value: &OwnedValue) -> Option<bool> {
    value.downcast_ref::<bool>().ok()
}

/// A connection to the system bus with NetworkManager owning its name, or the
/// [`VpnOutcome`] that stands in for it.
fn daemon_connection() -> Result<Connection, VpnOutcome> {
    let connection = Connection::system().map_err(|_| VpnOutcome::Absent)?;
    match name_has_owner(&connection) {
        Ok(true) => Ok(connection),
        Ok(false) | Err(_) => Err(VpnOutcome::Absent),
    }
}

/// Resolve `uuid` to the settings object path, or a failure saying it is
/// unknown.
fn resolve_connection(connection: &Connection, uuid: &str) -> Result<String, VpnOutcome> {
    let settings = NmSettingsProxyBlocking::new(connection).map_err(vpn_failed)?;
    match settings.get_connection_by_uuid(uuid) {
        Ok(path) => Ok(path.as_str().to_owned()),
        Err(error) => Err(VpnOutcome::Failed(AdapterError::new(format!(
            "NetworkManager: unknown VPN connection '{uuid}' ({error})"
        )))),
    }
}

/// The active-connection object path for `uuid`, if it is currently active.
fn active_path_for_uuid(connection: &Connection, uuid: &str) -> zbus::Result<Option<String>> {
    let manager = NetworkManagerProxyBlocking::new(connection)?;
    for path in manager.active_connections()? {
        let proxy = ActiveConnectionProxyBlocking::new(connection, path.as_str())?;
        if proxy.uuid()? == uuid {
            return Ok(Some(path.as_str().to_owned()));
        }
    }
    Ok(None)
}

/// Map a D-Bus error from a VPN write to the adapter outcome.
///
/// A polkit refusal (by error name or message) is a read-only degradation, not
/// a failure; everything else is a plain failure.
fn classify_vpn_error(error: zbus::Error) -> VpnOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    if is_permission_denied(name, &error.to_string()) {
        VpnOutcome::Denied(format!("NetworkManager: {error}"))
    } else {
        VpnOutcome::Failed(AdapterError::new(format!("NetworkManager: {error}")))
    }
}

/// One zbus error on the VPN path as an adapter error.
fn vpn_failed(error: zbus::Error) -> VpnOutcome {
    VpnOutcome::Failed(AdapterError::new(format!("NetworkManager: {error}")))
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

    #[test]
    fn the_vpn_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _source = DbusVpn::new();
    }

    fn owned(value: Value<'static>) -> OwnedValue {
        OwnedValue::try_from(value).expect("owned value")
    }

    fn settings_map(
        kind: &str,
        id: &str,
        uuid: &str,
        service: Option<&str>,
    ) -> HashMap<String, HashMap<String, OwnedValue>> {
        let mut connection: HashMap<String, OwnedValue> = HashMap::new();
        connection.insert("type".to_owned(), owned(Value::from(kind.to_owned())));
        connection.insert("id".to_owned(), owned(Value::from(id.to_owned())));
        connection.insert("uuid".to_owned(), owned(Value::from(uuid.to_owned())));
        connection.insert("autoconnect".to_owned(), owned(Value::from(true)));
        let mut settings: HashMap<String, HashMap<String, OwnedValue>> = HashMap::new();
        settings.insert("connection".to_owned(), connection);
        if let Some(service) = service {
            let mut vpn: HashMap<String, OwnedValue> = HashMap::new();
            vpn.insert(
                "service-type".to_owned(),
                owned(Value::from(service.to_owned())),
            );
            settings.insert("vpn".to_owned(), vpn);
        }
        settings
    }

    #[test]
    fn a_vpn_settings_map_decodes_to_a_connection() {
        let settings = settings_map(
            "vpn",
            "Work VPN",
            "11112222",
            Some("org.freedesktop.NetworkManager.openvpn"),
        );
        let connection =
            vpn_connection_from_settings("/org/freedesktop/NetworkManager/Settings/1", &settings)
                .expect("a vpn connection");
        assert_eq!(
            connection.path,
            "/org/freedesktop/NetworkManager/Settings/1"
        );
        assert_eq!(connection.id, "Work VPN");
        assert_eq!(connection.uuid, "11112222");
        assert_eq!(connection.kind, "vpn");
        assert_eq!(
            connection.service_type.as_deref(),
            Some("org.freedesktop.NetworkManager.openvpn")
        );
        assert!(connection.autoconnect);
    }

    #[test]
    fn a_wireguard_settings_map_decodes_without_a_service() {
        let settings = settings_map("wireguard", "Home", "3333", None);
        let connection =
            vpn_connection_from_settings("/org/freedesktop/NetworkManager/Settings/2", &settings)
                .expect("a wireguard connection");
        assert_eq!(connection.kind, "wireguard");
        assert_eq!(connection.service_type, None);
    }

    #[test]
    fn a_non_vpn_settings_map_is_filtered_out() {
        let settings = settings_map("802-3-ethernet", "Wired", "4444", None);
        assert_eq!(
            vpn_connection_from_settings("/org/freedesktop/NetworkManager/Settings/3", &settings),
            None
        );
    }

    #[test]
    fn a_settings_map_without_a_connection_table_is_filtered_out() {
        let settings: HashMap<String, HashMap<String, OwnedValue>> = HashMap::new();
        assert_eq!(
            vpn_connection_from_settings("/org/freedesktop/NetworkManager/Settings/4", &settings),
            None
        );
    }

    #[test]
    fn a_vpn_write_polkit_error_becomes_a_denial() {
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "polkit refused the request".to_owned(),
        )));
        match classify_vpn_error(error) {
            VpnOutcome::Denied(note) => assert!(note.starts_with("NetworkManager: ")),
            other => panic!("expected a denial, got {other:?}"),
        }
    }

    #[test]
    fn a_vpn_write_other_error_is_a_failure() {
        let error = zbus::Error::Failure("unknown connection".to_owned());
        assert_eq!(
            classify_vpn_error(error),
            VpnOutcome::Failed(AdapterError::new("NetworkManager: unknown connection"))
        );
    }
}
