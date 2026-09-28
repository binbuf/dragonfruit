// SPDX-License-Identifier: MIT
//! The live transport: BlueZ over its D-Bus API.
//!
//! BlueZ is reached over the **system bus** as `org.bluez`
//! ([07-system-integration.md]). One read is
//! `org.freedesktop.DBus.ObjectManager.GetManagedObjects` at `/`: every
//! controller is an `org.bluez.Adapter1` object and every device an
//! `org.bluez.Device1` object, so one call enumerates the whole tree. The
//! writes are single method calls or one `org.freedesktop.DBus.Properties.Set`,
//! never a loop:
//!
//! * power — set `org.bluez.Adapter1.Powered`;
//! * discovery — `StartDiscovery`/`StopDiscovery` on the adapter;
//! * pair — `Pair` on the device;
//! * connect — `Connect`/`Disconnect` on the device.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. A call that cannot
//! reach the bus at all, or a bus where `org.bluez` does not own its name, is
//! treated as absence; a name owned but unreadable is reported to the adapter,
//! which shows the item visible and inert. A polkit refusal on a write is
//! reported separately so the pane can surface it without treating the daemon
//! as broken.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use std::collections::HashMap;

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

use crate::source::{
    BluetoothAdapterData, BluetoothData, BluetoothDeviceData, BluetoothOutcome, BluetoothSource,
};

/// The well-known name `bluetoothd` owns.
pub const BLUEZ_SERVICE: &str = "org.bluez";
/// The adapter interface.
pub const ADAPTER_INTERFACE: &str = "org.bluez.Adapter1";
/// The device interface.
pub const DEVICE_INTERFACE: &str = "org.bluez.Device1";
/// The standard object-manager interface BlueZ implements at `/`.
pub const OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";
/// The standard properties interface.
pub const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";

/// `GetManagedObjects`'s return shape.
type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

#[zbus::proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.bluez",
    default_path = "/"
)]
trait ObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;
}

/// The real BlueZ transport.
#[derive(Debug, Default, Clone, Copy)]
pub struct DbusBluez;

impl DbusBluez {
    /// A source that reads the system bus on demand.
    pub const fn new() -> Self {
        DbusBluez
    }
}

impl BluetoothSource for DbusBluez {
    fn read(&mut self) -> Result<Option<BluetoothData>, AdapterError> {
        // No system bus means no BlueZ can be reached: absence, not an error,
        // and never a startup blocker.
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };

        if !name_has_owner(&connection).map_err(failed)? {
            return Ok(None);
        }

        let objects = managed_objects(&connection).map_err(failed)?;
        Ok(Some(BluetoothData::from_objects(&objects)))
    }

    fn set_powered(&mut self, powered: bool) -> BluetoothOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        let adapter = match adapter_path(&connection) {
            Ok(Some(path)) => path,
            Ok(None) => return BluetoothOutcome::Absent,
            Err(error) => return classify(error),
        };
        let result = connection.call_method(
            Some(BLUEZ_SERVICE),
            adapter.as_str(),
            Some(PROPERTIES_INTERFACE),
            "Set",
            &(ADAPTER_INTERFACE, "Powered", Value::from(powered)),
        );
        match result {
            Ok(_) => BluetoothOutcome::Accepted,
            Err(error) => classify(error),
        }
    }

    fn set_discovering(&mut self, discovering: bool) -> BluetoothOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        let adapter = match adapter_path(&connection) {
            Ok(Some(path)) => path,
            Ok(None) => return BluetoothOutcome::Absent,
            Err(error) => return classify(error),
        };
        let method = if discovering {
            "StartDiscovery"
        } else {
            "StopDiscovery"
        };
        call_on_adapter(&connection, adapter.as_str(), method)
    }

    fn pair(&mut self, address: &str) -> BluetoothOutcome {
        self.device_action(address, "Pair")
    }

    fn set_connected(&mut self, address: &str, connected: bool) -> BluetoothOutcome {
        let method = if connected { "Connect" } else { "Disconnect" };
        self.device_action(address, method)
    }
}

impl DbusBluez {
    /// Run one `Device1` method for the device at `address`.
    fn device_action(&self, address: &str, method: &str) -> BluetoothOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        let device = match device_path(&connection, address) {
            Ok(Some(path)) => path,
            Ok(None) => {
                return BluetoothOutcome::Failed(AdapterError::new(format!(
                    "BlueZ: unknown device {address}"
                )))
            }
            Err(error) => return classify(error),
        };
        call_on_device(&connection, device.as_str(), method)
    }
}

impl BluetoothData {
    /// Flatten `GetManagedObjects` into the adapter and device lists.
    fn from_objects(objects: &ManagedObjects) -> Self {
        let mut adapters = Vec::new();
        let mut devices = Vec::new();
        for (path, interfaces) in objects {
            if let Some(props) = interfaces.get(ADAPTER_INTERFACE) {
                adapters.push(adapter_from_props(path.as_str(), props));
            }
            if let Some(props) = interfaces.get(DEVICE_INTERFACE) {
                devices.push(device_from_props(path.as_str(), props));
            }
        }
        // Deterministic order regardless of the hash-map iteration order.
        adapters.sort_by(|left, right| left.path.cmp(&right.path));
        devices.sort_by(|left, right| left.path.cmp(&right.path));
        BluetoothData { adapters, devices }
    }
}

/// One `org.bluez.Adapter1` object from its property map.
fn adapter_from_props(path: &str, props: &HashMap<String, OwnedValue>) -> BluetoothAdapterData {
    BluetoothAdapterData {
        path: path.to_owned(),
        address: string_prop(props, "Address"),
        name: string_prop(props, "Name"),
        alias: string_prop(props, "Alias"),
        powered: bool_prop(props, "Powered"),
        discoverable: bool_prop(props, "Discoverable"),
        pairable: bool_prop(props, "Pairable"),
        discovering: bool_prop(props, "Discovering"),
    }
}

/// One `org.bluez.Device1` object from its property map.
fn device_from_props(path: &str, props: &HashMap<String, OwnedValue>) -> BluetoothDeviceData {
    BluetoothDeviceData {
        path: path.to_owned(),
        adapter_path: object_path_prop(props, "Adapter"),
        address: string_prop(props, "Address"),
        name: string_prop(props, "Name"),
        alias: string_prop(props, "Alias"),
        paired: bool_prop(props, "Paired"),
        connected: bool_prop(props, "Connected"),
        trusted: bool_prop(props, "Trusted"),
        blocked: bool_prop(props, "Blocked"),
        rssi: i16_prop(props, "RSSI").unwrap_or_default(),
        icon: string_prop(props, "Icon"),
    }
}

/// A string property, `""` when BlueZ did not report it.
fn string_prop(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<&str>().ok())
        .map(str::to_owned)
        .unwrap_or_default()
}

/// A boolean property, `false` when BlueZ did not report it.
fn bool_prop(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<bool>().ok())
        .unwrap_or_default()
}

/// An `i16` property, `None` when BlueZ did not report it.
fn i16_prop(props: &HashMap<String, OwnedValue>, key: &str) -> Option<i16> {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<i16>().ok())
}

/// An object-path property as a string, `""` when absent.
fn object_path_prop(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<ObjectPath>().ok())
        .map(|path| path.as_str().to_owned())
        .unwrap_or_default()
}

/// A connection to the system bus, or the outcome that stands in for it.
///
/// No bus, or a bus with no `org.bluez` owner, is absence — never an error.
fn daemon_connection() -> Result<Connection, BluetoothOutcome> {
    let connection = Connection::system().map_err(|_| BluetoothOutcome::Absent)?;
    match name_has_owner(&connection) {
        Ok(true) => Ok(connection),
        Ok(false) | Err(_) => Err(BluetoothOutcome::Absent),
    }
}

/// Every managed object BlueZ serves.
fn managed_objects(connection: &Connection) -> zbus::Result<ManagedObjects> {
    let manager = ObjectManagerProxyBlocking::new(connection)?;
    manager.get_managed_objects()
}

/// The path of the first `org.bluez.Adapter1` object.
fn adapter_path(connection: &Connection) -> zbus::Result<Option<OwnedObjectPath>> {
    let objects = managed_objects(connection)?;
    Ok(objects
        .into_iter()
        .find(|(_, interfaces)| interfaces.contains_key(ADAPTER_INTERFACE))
        .map(|(path, _)| path))
}

/// The path of the `org.bluez.Device1` object whose `Address` is `address`.
fn device_path(connection: &Connection, address: &str) -> zbus::Result<Option<OwnedObjectPath>> {
    let objects = managed_objects(connection)?;
    for (path, interfaces) in objects {
        if let Some(props) = interfaces.get(DEVICE_INTERFACE) {
            if string_prop(props, "Address") == address {
                return Ok(Some(path));
            }
        }
    }
    Ok(None)
}

/// Call one no-argument `Adapter1` method.
fn call_on_adapter(connection: &Connection, adapter: &str, method: &str) -> BluetoothOutcome {
    match connection.call_method(
        Some(BLUEZ_SERVICE),
        adapter,
        Some(ADAPTER_INTERFACE),
        method,
        &(),
    ) {
        Ok(_) => BluetoothOutcome::Accepted,
        Err(error) => classify(error),
    }
}

/// Call one no-argument `Device1` method.
fn call_on_device(connection: &Connection, device: &str, method: &str) -> BluetoothOutcome {
    match connection.call_method(
        Some(BLUEZ_SERVICE),
        device,
        Some(DEVICE_INTERFACE),
        method,
        &(),
    ) {
        Ok(_) => BluetoothOutcome::Accepted,
        Err(error) => classify(error),
    }
}

/// Whether `org.bluez` currently owns its name.
fn name_has_owner(connection: &Connection) -> zbus::Result<bool> {
    let reply = connection.call_method(
        Some("org.freedesktop.DBus"),
        "/org/freedesktop/DBus",
        Some("org.freedesktop.DBus"),
        "NameHasOwner",
        &(BLUEZ_SERVICE,),
    )?;
    reply.body().deserialize::<bool>()
}

/// Map a D-Bus error from a write to the adapter outcome.
///
/// A polkit refusal (by error name or message) is a denial the pane surfaces;
/// everything else is a plain failure.
fn classify(error: zbus::Error) -> BluetoothOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    if is_permission_denied(name, &error.to_string()) {
        BluetoothOutcome::Denied(format!("BlueZ: {error}"))
    } else {
        BluetoothOutcome::Failed(AdapterError::new(format!("BlueZ: {error}")))
    }
}

/// Whether an error name or message reports an authorization refusal.
fn is_permission_denied(name: &str, message: &str) -> bool {
    const NAMES: [&str; 3] = [
        "org.bluez.Error.NotAuthorized",
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

/// One read failure on the live path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("BlueZ: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _source = DbusBluez::new();
    }

    #[test]
    fn a_read_failure_carries_the_daemon_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("BlueZ: "));
    }

    #[test]
    fn the_service_name_is_the_bluez_well_known_name() {
        assert_eq!(BLUEZ_SERVICE, "org.bluez");
        assert_eq!(ADAPTER_INTERFACE, "org.bluez.Adapter1");
        assert_eq!(DEVICE_INTERFACE, "org.bluez.Device1");
    }

    #[test]
    fn polkit_names_and_messages_classify_as_denied() {
        assert!(is_permission_denied(
            "org.bluez.Error.NotAuthorized",
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
        assert!(is_permission_denied("", "polkit refused the request"));
        assert!(!is_permission_denied(
            "org.bluez.Error.Failed",
            "the device is not available"
        ));
    }

    #[test]
    fn a_polkit_error_becomes_a_denial_outcome() {
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "polkit refused the request".to_owned(),
        )));
        match classify(error) {
            BluetoothOutcome::Denied(note) => assert!(note.starts_with("BlueZ: ")),
            other => panic!("expected a denial, got {other:?}"),
        }
    }

    #[test]
    fn a_non_authorization_error_is_a_failure() {
        let error = zbus::Error::Failure("the device is not available".to_owned());
        assert_eq!(
            classify(error),
            BluetoothOutcome::Failed(AdapterError::new("BlueZ: the device is not available"))
        );
    }

    #[test]
    fn flat_props_decode_from_a_managed_object_map() {
        let mut adapter: HashMap<String, OwnedValue> = HashMap::new();
        adapter.insert(
            "Address".to_owned(),
            OwnedValue::try_from(Value::from("11:22:33:44:55:66")).unwrap(),
        );
        adapter.insert(
            "Alias".to_owned(),
            OwnedValue::try_from(Value::from("Workstation")).unwrap(),
        );
        adapter.insert("Powered".to_owned(), OwnedValue::from(true));
        adapter.insert("Pairable".to_owned(), OwnedValue::from(true));

        let mut path: HashMap<String, OwnedValue> = HashMap::new();
        path.insert(
            "Adapter".to_owned(),
            OwnedValue::try_from(Value::from(
                ObjectPath::try_from("/org/bluez/hci0").unwrap(),
            ))
            .unwrap(),
        );
        path.insert(
            "Address".to_owned(),
            OwnedValue::try_from(Value::from("AA:BB:CC:DD:EE:FF")).unwrap(),
        );
        path.insert("Paired".to_owned(), OwnedValue::from(true));
        path.insert("Connected".to_owned(), OwnedValue::from(true));
        path.insert("RSSI".to_owned(), OwnedValue::from(-55i16));

        let mut interfaces: HashMap<String, HashMap<String, OwnedValue>> = HashMap::new();
        interfaces.insert(ADAPTER_INTERFACE.to_owned(), adapter);
        let mut device_interfaces: HashMap<String, HashMap<String, OwnedValue>> = HashMap::new();
        device_interfaces.insert(DEVICE_INTERFACE.to_owned(), path);

        let mut objects: ManagedObjects = HashMap::new();
        objects.insert(
            OwnedObjectPath::try_from("/org/bluez/hci0").unwrap(),
            interfaces,
        );
        objects.insert(
            OwnedObjectPath::try_from("/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF").unwrap(),
            device_interfaces,
        );

        let data = BluetoothData::from_objects(&objects);
        assert_eq!(data.adapters.len(), 1);
        assert_eq!(data.adapters[0].alias, "Workstation");
        assert!(data.adapters[0].powered);
        assert_eq!(data.devices.len(), 1);
        assert_eq!(data.devices[0].adapter_path, "/org/bluez/hci0");
        assert!(data.devices[0].connected);
        assert_eq!(data.devices[0].rssi, -55);
    }
}
