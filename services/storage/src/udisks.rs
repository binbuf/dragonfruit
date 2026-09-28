// SPDX-License-Identifier: MIT
//! The live transport: UDisks2 over its D-Bus API.
//!
//! UDisks2 is reached over the **system bus** as `org.freedesktop.UDisks2`
//! ([07-system-integration.md]). One read is
//! `org.freedesktop.DBus.ObjectManager.GetManagedObjects` at
//! `/org/freedesktop/UDisks2`: every drive is an
//! `org.freedesktop.UDisks2.Drive` object, and every block device is an
//! `org.freedesktop.UDisks2.Block` object with an optional
//! `org.freedesktop.UDisks2.Filesystem`, so one call enumerates the whole
//! tree. The writes are single method calls, never a loop:
//!
//! * mount — `Mount` on the block's `Filesystem`;
//! * unmount — `Unmount` on the block's `Filesystem`;
//! * eject — `Eject` on the owning `Drive`.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. A call that cannot
//! reach the bus at all, or a bus where `org.freedesktop.UDisks2` does not own
//! its name, is treated as absence; a name owned but unreadable is reported to
//! the adapter, which shows the item visible and inert. A polkit refusal on a
//! write is reported separately so the pane can surface it without treating
//! the daemon as broken.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use std::collections::HashMap;

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::{Array, ObjectPath, OwnedObjectPath, OwnedValue, Value};

use crate::source::{
    StorageData, StorageDriveData, StorageOutcome, StorageSource, StorageVolumeData,
};

/// The well-known name `udisksd` owns.
pub const UDISKS_SERVICE: &str = "org.freedesktop.UDisks2";
/// The ObjectManager root UDisks2 serves.
pub const UDISKS_ROOT: &str = "/org/freedesktop/UDisks2";
/// The drive interface.
pub const DRIVE_INTERFACE: &str = "org.freedesktop.UDisks2.Drive";
/// The block-device interface.
pub const BLOCK_INTERFACE: &str = "org.freedesktop.UDisks2.Block";
/// The filesystem interface (present only on mountable blocks).
pub const FILESYSTEM_INTERFACE: &str = "org.freedesktop.UDisks2.Filesystem";
/// The standard object-manager interface UDisks2 implements at its root.
pub const OBJECT_MANAGER_INTERFACE: &str = "org.freedesktop.DBus.ObjectManager";

/// `GetManagedObjects`'s return shape.
type ManagedObjects = HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>;

#[zbus::proxy(
    interface = "org.freedesktop.DBus.ObjectManager",
    default_service = "org.freedesktop.UDisks2",
    default_path = "/org/freedesktop/UDisks2"
)]
trait ObjectManager {
    fn get_managed_objects(&self) -> zbus::Result<ManagedObjects>;
}

/// The real UDisks2 transport.
#[derive(Debug, Default, Clone, Copy)]
pub struct DbusUDisks;

impl DbusUDisks {
    /// A source that reads the system bus on demand.
    pub const fn new() -> Self {
        DbusUDisks
    }
}

impl StorageSource for DbusUDisks {
    fn read(&mut self) -> Result<Option<StorageData>, AdapterError> {
        // No system bus means no UDisks2 can be reached: absence, not an
        // error, and never a startup blocker.
        let connection = match Connection::system() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };

        if !name_has_owner(&connection, UDISKS_SERVICE).map_err(failed)? {
            return Ok(None);
        }

        let objects = managed_objects(&connection).map_err(failed)?;
        Ok(Some(StorageData::from_objects(&objects)))
    }

    fn mount(&mut self, volume_path: &str) -> StorageOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        call_with_options(&connection, volume_path, FILESYSTEM_INTERFACE, "Mount")
    }

    fn unmount(&mut self, volume_path: &str) -> StorageOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        call_with_options(&connection, volume_path, FILESYSTEM_INTERFACE, "Unmount")
    }

    fn eject(&mut self, drive_path: &str) -> StorageOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        call_with_options(&connection, drive_path, DRIVE_INTERFACE, "Eject")
    }
}

impl StorageData {
    /// Flatten `GetManagedObjects` into the drive and volume lists.
    fn from_objects(objects: &ManagedObjects) -> Self {
        let mut drives = Vec::new();
        let mut volumes = Vec::new();
        for (path, interfaces) in objects {
            if let Some(props) = interfaces.get(DRIVE_INTERFACE) {
                drives.push(drive_from_props(path.as_str(), props));
            }
            if let Some(props) = interfaces.get(BLOCK_INTERFACE) {
                let filesystem = interfaces.get(FILESYSTEM_INTERFACE);
                volumes.push(volume_from_props(path.as_str(), props, filesystem));
            }
        }
        // Deterministic order regardless of the hash-map iteration order.
        drives.sort_by(|left, right| left.path.cmp(&right.path));
        volumes.sort_by(|left, right| left.path.cmp(&right.path));
        StorageData { drives, volumes }
    }
}

/// One `org.freedesktop.UDisks2.Drive` object from its property map.
fn drive_from_props(path: &str, props: &HashMap<String, OwnedValue>) -> StorageDriveData {
    StorageDriveData {
        path: path.to_owned(),
        id: string_prop(props, "Id"),
        vendor: string_prop(props, "Vendor"),
        model: string_prop(props, "Model"),
        serial: string_prop(props, "Serial"),
        media: string_prop(props, "Media"),
        connection_bus: string_prop(props, "ConnectionBus"),
        removable: bool_prop(props, "Removable"),
        ejectable: bool_prop(props, "Ejectable"),
        optical: bool_prop(props, "Optical"),
        media_available: bool_prop(props, "MediaAvailable"),
        size: u64_prop(props, "Size").unwrap_or_default(),
    }
}

/// One `org.freedesktop.UDisks2.Block` object from its property map, joined
/// with the optional `Filesystem` interface.
fn volume_from_props(
    path: &str,
    props: &HashMap<String, OwnedValue>,
    filesystem: Option<&HashMap<String, OwnedValue>>,
) -> StorageVolumeData {
    StorageVolumeData {
        path: path.to_owned(),
        drive_path: object_path_prop(props, "Drive"),
        device: string_from_bytes(&bytes_prop(props, "Device")),
        preferred_device: string_from_bytes(&bytes_prop(props, "PreferredDevice")),
        id: string_prop(props, "Id"),
        label: string_prop(props, "IdLabel"),
        uuid: string_prop(props, "IdUUID"),
        filesystem: string_prop(props, "IdType"),
        usage: string_prop(props, "IdUsage"),
        size: u64_prop(props, "Size").unwrap_or_default(),
        read_only: bool_prop(props, "ReadOnly"),
        hint_auto: bool_prop(props, "HintAuto"),
        hint_system: bool_prop(props, "HintSystem"),
        hint_ignore: bool_prop(props, "HintIgnore"),
        mountable: filesystem.is_some(),
        mount_points: filesystem
            .map(|fs| mount_points_prop(fs, "MountPoints"))
            .unwrap_or_default(),
    }
}

/// A string property, `""` when UDisks2 did not report it.
fn string_prop(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<String>().ok())
        .unwrap_or_default()
}

/// A boolean property, `false` when UDisks2 did not report it.
fn bool_prop(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<bool>().ok())
        .unwrap_or_default()
}

/// A `u64` property, `None` when UDisks2 did not report it.
fn u64_prop(props: &HashMap<String, OwnedValue>, key: &str) -> Option<u64> {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<u64>().ok())
}

/// An object-path property as a string, `""` when absent.
fn object_path_prop(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<ObjectPath>().ok())
        .map(|path| path.as_str().to_owned())
        .unwrap_or_default()
}

/// A byte-array (`ay`) property, empty when UDisks2 did not report it.
fn bytes_prop(props: &HashMap<String, OwnedValue>, key: &str) -> Vec<u8> {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<Array>().ok())
        .and_then(|array| Vec::<u8>::try_from(array).ok())
        .unwrap_or_default()
}

/// An array-of-byte-arrays (`aay`) property as mount-point strings.
fn mount_points_prop(props: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    props
        .get(key)
        .and_then(|value| value.downcast_ref::<Array>().ok())
        .and_then(|array| Vec::<Vec<u8>>::try_from(array).ok())
        .map(|points| {
            points
                .iter()
                .map(|bytes| string_from_bytes(bytes))
                .collect()
        })
        .unwrap_or_default()
}

/// A D-Bus byte array as a UTF-8 string (lossy, so invalid bytes never panic).
fn string_from_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A connection to the system bus, or the outcome that stands in for it.
///
/// No bus, or a bus with no `org.freedesktop.UDisks2` owner, is absence —
/// never an error.
fn daemon_connection() -> Result<Connection, StorageOutcome> {
    let connection = Connection::system().map_err(|_| StorageOutcome::Absent)?;
    match name_has_owner(&connection, UDISKS_SERVICE) {
        Ok(true) => Ok(connection),
        Ok(false) | Err(_) => Err(StorageOutcome::Absent),
    }
}

/// Every managed object UDisks2 serves.
fn managed_objects(connection: &Connection) -> zbus::Result<ManagedObjects> {
    let manager = ObjectManagerProxyBlocking::new(connection)?;
    manager.get_managed_objects()
}

/// Call one `a{sv}`-taking method on `path`'s `interface`.
fn call_with_options(
    connection: &Connection,
    path: &str,
    interface: &str,
    method: &str,
) -> StorageOutcome {
    let options: HashMap<String, Value> = HashMap::new();
    match connection.call_method(
        Some(UDISKS_SERVICE),
        path,
        Some(interface),
        method,
        &options,
    ) {
        Ok(_) => StorageOutcome::Accepted,
        Err(error) => classify(error),
    }
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

/// Map a D-Bus error from a write to the adapter outcome.
///
/// A polkit refusal (by error name or message) is a denial the pane surfaces;
/// everything else is a plain failure — including a busy device that refuses
/// to unmount.
fn classify(error: zbus::Error) -> StorageOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    if is_permission_denied(name, &error.to_string()) {
        StorageOutcome::Denied(format!("UDisks: {error}"))
    } else {
        StorageOutcome::Failed(AdapterError::new(format!("UDisks: {error}")))
    }
}

/// Whether an error name or message reports an authorization refusal.
fn is_permission_denied(name: &str, message: &str) -> bool {
    const NAMES: [&str; 4] = [
        "org.freedesktop.UDisks2.Error.NotAuthorized",
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
    AdapterError::new(format!("UDisks: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn byte_array(bytes: &[u8]) -> OwnedValue {
        OwnedValue::try_from(Value::Array(Array::from(bytes.to_vec()))).unwrap()
    }

    fn string(value: &str) -> OwnedValue {
        OwnedValue::try_from(Value::from(value)).unwrap()
    }

    fn mount_points(points: &[&str]) -> OwnedValue {
        let arrays: Vec<Value> = points
            .iter()
            .map(|point| Value::Array(Array::from(point.as_bytes().to_vec())))
            .collect();
        OwnedValue::try_from(Value::Array(Array::from(arrays))).unwrap()
    }

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _source = DbusUDisks::new();
    }

    #[test]
    fn a_read_failure_carries_the_daemon_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("UDisks: "));
    }

    #[test]
    fn the_service_name_and_root_are_the_udisks2_ones() {
        assert_eq!(UDISKS_SERVICE, "org.freedesktop.UDisks2");
        assert_eq!(UDISKS_ROOT, "/org/freedesktop/UDisks2");
        assert_eq!(DRIVE_INTERFACE, "org.freedesktop.UDisks2.Drive");
        assert_eq!(BLOCK_INTERFACE, "org.freedesktop.UDisks2.Block");
        assert_eq!(FILESYSTEM_INTERFACE, "org.freedesktop.UDisks2.Filesystem");
    }

    #[test]
    fn polkit_names_and_messages_classify_as_denied() {
        assert!(is_permission_denied(
            "org.freedesktop.UDisks2.Error.NotAuthorized",
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
            "org.freedesktop.UDisks2.Error.DeviceBusy",
            "the device is busy"
        ));
    }

    #[test]
    fn a_polkit_error_becomes_a_denial_outcome() {
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "polkit refused the request".to_owned(),
        )));
        match classify(error) {
            StorageOutcome::Denied(note) => assert!(note.starts_with("UDisks: ")),
            other => panic!("expected a denial, got {other:?}"),
        }
    }

    #[test]
    fn a_non_authorization_error_is_a_failure() {
        let error = zbus::Error::Failure("the device is busy".to_owned());
        assert_eq!(
            classify(error),
            StorageOutcome::Failed(AdapterError::new("UDisks: the device is busy"))
        );
    }

    #[test]
    fn flat_props_decode_from_a_managed_object_map() {
        let mut drive: HashMap<String, OwnedValue> = HashMap::new();
        drive.insert("Id".to_owned(), string("USB_STICK"));
        drive.insert("Model".to_owned(), string("Flash Drive"));
        drive.insert("Removable".to_owned(), OwnedValue::from(true));
        drive.insert("Ejectable".to_owned(), OwnedValue::from(true));
        drive.insert("Size".to_owned(), OwnedValue::from(16_000_000_000u64));

        let mut block: HashMap<String, OwnedValue> = HashMap::new();
        block.insert("Device".to_owned(), byte_array(b"/dev/sdb1"));
        block.insert("PreferredDevice".to_owned(), byte_array(b"/dev/sdb1"));
        block.insert(
            "Drive".to_owned(),
            OwnedValue::try_from(Value::from(
                ObjectPath::try_from("/org/freedesktop/UDisks2/drives/USB_Stick").unwrap(),
            ))
            .unwrap(),
        );
        block.insert("IdLabel".to_owned(), string("Photos"));
        block.insert("IdType".to_owned(), string("vfat"));
        block.insert("IdUsage".to_owned(), string("filesystem"));
        block.insert("HintAuto".to_owned(), OwnedValue::from(true));
        block.insert("ReadOnly".to_owned(), OwnedValue::from(false));

        let mut fs: HashMap<String, OwnedValue> = HashMap::new();
        fs.insert(
            "MountPoints".to_owned(),
            mount_points(&["/run/media/user/Photos"]),
        );

        let mut drive_interfaces: HashMap<String, HashMap<String, OwnedValue>> = HashMap::new();
        drive_interfaces.insert(DRIVE_INTERFACE.to_owned(), drive);
        let mut block_interfaces: HashMap<String, HashMap<String, OwnedValue>> = HashMap::new();
        block_interfaces.insert(BLOCK_INTERFACE.to_owned(), block);
        block_interfaces.insert(FILESYSTEM_INTERFACE.to_owned(), fs);

        let mut objects: ManagedObjects = HashMap::new();
        objects.insert(
            OwnedObjectPath::try_from("/org/freedesktop/UDisks2/drives/USB_Stick").unwrap(),
            drive_interfaces,
        );
        objects.insert(
            OwnedObjectPath::try_from("/org/freedesktop/UDisks2/block_devices/sdb1").unwrap(),
            block_interfaces,
        );

        let data = StorageData::from_objects(&objects);
        assert_eq!(data.drives.len(), 1);
        assert_eq!(data.drives[0].model, "Flash Drive");
        assert!(data.drives[0].removable);
        assert_eq!(data.drives[0].size, 16_000_000_000);
        assert_eq!(data.volumes.len(), 1);
        assert_eq!(data.volumes[0].device, "/dev/sdb1");
        assert_eq!(
            data.volumes[0].drive_path,
            "/org/freedesktop/UDisks2/drives/USB_Stick"
        );
        assert!(data.volumes[0].mountable);
        assert_eq!(data.volumes[0].mount_points, vec!["/run/media/user/Photos"]);
    }

    #[test]
    fn a_block_without_a_filesystem_is_not_mountable() {
        let mut block: HashMap<String, OwnedValue> = HashMap::new();
        block.insert("Device".to_owned(), byte_array(b"/dev/sdb"));
        let mut interfaces: HashMap<String, HashMap<String, OwnedValue>> = HashMap::new();
        interfaces.insert(BLOCK_INTERFACE.to_owned(), block);
        let mut objects: ManagedObjects = HashMap::new();
        objects.insert(
            OwnedObjectPath::try_from("/org/freedesktop/UDisks2/block_devices/sdb").unwrap(),
            interfaces,
        );

        let data = StorageData::from_objects(&objects);
        assert_eq!(data.volumes.len(), 1);
        assert!(!data.volumes[0].mountable);
        assert!(data.volumes[0].mount_points.is_empty());
    }
}
