// SPDX-License-Identifier: MIT
//! The live transport: the xdg-desktop-portal PermissionStore over the session
//! bus.
//!
//! `xdg-desktop-portal` (and the `xdg-permission-store` daemon that backs it)
//! owns the record of which applications may reach the resources portals
//! mediate. It serves the standard
//! `org.freedesktop.impl.portal.PermissionStore` interface on the **session
//! bus** at `/org/freedesktop/impl/portal/PermissionStore`. This module reuses
//! that store and never reimplements a portal or a permission decision:
//!
//! * one read is `List(table)` for every curated table
//!   ([`KNOWN_TABLES`](crate::KNOWN_TABLES)) followed by `Lookup(table, id)`
//!   for each returned resource id, so one pass enumerates the whole store;
//! * a write is one call each: `SetPermission(table, create, id, app,
//!   permissions)` to store a tristate and `DeletePermission(table, id, app)`
//!   to remove one.
//!
//! The store's table/entry shape is free-form and its values are opaque; this
//! adapter only enumerates and writes them, exactly as the accounts adapter
//! only enumerates AccountsService and the printer adapter only forwards CUPS
//! tool calls.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. No session bus, or
//! a bus with no `org.freedesktop.impl.portal.PermissionStore` owner, is
//! absence, never an error. A store that owns its name but cannot be read is
//! reported to the adapter, which shows the item visible and inert. A policy
//! refusal on a write is reported separately so the pane can surface it
//! without treating the store as broken.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use std::collections::HashMap;

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedValue;

use crate::model::KNOWN_TABLES;
use crate::source::{
    AppPermissionData, PrivacyData, PrivacyOutcome, PrivacySource, ResourceData, TableData,
};

/// The well-known name the permission store owns on the session bus.
pub const PERMISSION_STORE_SERVICE: &str = "org.freedesktop.impl.portal.PermissionStore";
/// The object path the store is served at.
pub const PERMISSION_STORE_PATH: &str = "/org/freedesktop/impl/portal/PermissionStore";
/// The standard backend interface name.
pub const PERMISSION_STORE_INTERFACE: &str = "org.freedesktop.impl.portal.PermissionStore";

/// The live portal PermissionStore source.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HostPrivacy;

impl HostPrivacy {
    /// A source reading the permission store with no bus connection until
    /// `read`.
    pub fn new() -> Self {
        HostPrivacy
    }
}

impl PrivacySource for HostPrivacy {
    fn read(&mut self) -> Result<Option<PrivacyData>, AdapterError> {
        // No session bus means no portal store can be reached: absence, not an
        // error, and never a startup blocker.
        let connection = match Connection::session() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };
        if !name_has_owner(&connection, PERMISSION_STORE_SERVICE).map_err(failed)? {
            return Ok(None);
        }

        let mut tables = Vec::new();
        for known in KNOWN_TABLES {
            let ids = list(&connection, known.table).map_err(failed)?;
            let mut resources = Vec::new();
            for id in ids {
                let apps = lookup(&connection, known.table, &id).map_err(failed)?;
                resources.push(ResourceData { id, apps });
            }
            resources.sort_by(|left, right| left.id.cmp(&right.id));
            // Only tables with an entry are carried; the model fills the rest
            // as honest empty categories.
            if !resources.is_empty() {
                tables.push(TableData {
                    table: known.table.to_owned(),
                    resources,
                });
            }
        }
        Ok(Some(PrivacyData { tables }))
    }

    fn set_permission(
        &mut self,
        table: &str,
        id: &str,
        app: &str,
        permissions: &[String],
    ) -> PrivacyOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        // `create = true` so a first grant creates the table entry, exactly as
        // `xdg-desktop-portal` sets a first permission.
        match connection.call_method(
            Some(PERMISSION_STORE_SERVICE),
            PERMISSION_STORE_PATH,
            Some(PERMISSION_STORE_INTERFACE),
            "SetPermission",
            &(table, true, id, app, permissions.to_vec()),
        ) {
            Ok(_) => PrivacyOutcome::Applied,
            Err(error) => classify(error),
        }
    }

    fn delete_permission(&mut self, table: &str, id: &str, app: &str) -> PrivacyOutcome {
        let connection = match daemon_connection() {
            Ok(connection) => connection,
            Err(outcome) => return outcome,
        };
        match connection.call_method(
            Some(PERMISSION_STORE_SERVICE),
            PERMISSION_STORE_PATH,
            Some(PERMISSION_STORE_INTERFACE),
            "DeletePermission",
            &(table, id, app),
        ) {
            Ok(_) => PrivacyOutcome::Applied,
            Err(error) => classify(error),
        }
    }
}

/// The resource ids in `table`, as `List` reports them.
fn list(connection: &Connection, table: &str) -> zbus::Result<Vec<String>> {
    let reply = connection.call_method(
        Some(PERMISSION_STORE_SERVICE),
        PERMISSION_STORE_PATH,
        Some(PERMISSION_STORE_INTERFACE),
        "List",
        &(table,),
    )?;
    reply.body().deserialize::<Vec<String>>()
}

/// One resource's application permissions, as `Lookup` reports them.
fn lookup(connection: &Connection, table: &str, id: &str) -> zbus::Result<Vec<AppPermissionData>> {
    let reply = connection.call_method(
        Some(PERMISSION_STORE_SERVICE),
        PERMISSION_STORE_PATH,
        Some(PERMISSION_STORE_INTERFACE),
        "Lookup",
        &(table, id),
    )?;
    // `Lookup` answers `(a{sas} permissions, v data)`; the adapter keeps the
    // permissions and ignores the opaque per-resource data.
    let (permissions, _data): (HashMap<String, Vec<String>>, OwnedValue) =
        reply.body().deserialize()?;
    Ok(apps_from_lookup(permissions))
}

/// Build the per-app permissions from one `Lookup` map. Pure, so a fixture map
/// drives it in tests.
pub fn apps_from_lookup(permissions: HashMap<String, Vec<String>>) -> Vec<AppPermissionData> {
    let mut apps: Vec<AppPermissionData> = permissions
        .into_iter()
        .map(|(app, permissions)| AppPermissionData { app, permissions })
        .collect();
    apps.sort_by(|left, right| left.app.cmp(&right.app));
    apps
}

/// A connection to the session bus, or the outcome that stands in for it.
///
/// No bus, or a bus with no permission-store owner, is absence — never an
/// error.
fn daemon_connection() -> Result<Connection, PrivacyOutcome> {
    let connection = Connection::session().map_err(|_| PrivacyOutcome::Absent)?;
    match name_has_owner(&connection, PERMISSION_STORE_SERVICE) {
        Ok(true) => Ok(connection),
        Ok(false) | Err(_) => Err(PrivacyOutcome::Absent),
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
/// A policy refusal is a denial the pane surfaces; everything else is a plain
/// failure.
fn classify(error: zbus::Error) -> PrivacyOutcome {
    let name = match &error {
        zbus::Error::MethodError(name, ..) => name.as_str(),
        _ => "",
    };
    if is_permission_denied(name, &error.to_string()) {
        PrivacyOutcome::Denied(format!("PermissionStore: {error}"))
    } else {
        PrivacyOutcome::Failed(AdapterError::new(format!("PermissionStore: {error}")))
    }
}

/// Whether an error name or message reports an authorization refusal.
fn is_permission_denied(name: &str, message: &str) -> bool {
    const NAMES: [&str; 3] = [
        "org.freedesktop.DBus.Error.AccessDenied",
        "org.freedesktop.PolicyKit1.Error.NotAuthorized",
        "org.freedesktop.portal.Error.NotAllowed",
    ];
    if NAMES.contains(&name) {
        return true;
    }
    let lower = message.to_lowercase();
    lower.contains("not authorized")
        || lower.contains("permission denied")
        || lower.contains("access denied")
        || lower.contains("polkit")
        || lower.contains("not allowed")
}

/// One read failure on the live path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("PermissionStore: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _ = HostPrivacy::new();
    }

    #[test]
    fn the_service_names_are_the_permission_store_ones() {
        assert_eq!(
            PERMISSION_STORE_SERVICE,
            "org.freedesktop.impl.portal.PermissionStore"
        );
        assert_eq!(
            PERMISSION_STORE_PATH,
            "/org/freedesktop/impl/portal/PermissionStore"
        );
        assert_eq!(
            PERMISSION_STORE_INTERFACE,
            "org.freedesktop.impl.portal.PermissionStore"
        );
    }

    #[test]
    fn a_read_failure_carries_the_store_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("PermissionStore: "));
    }

    #[test]
    fn access_errors_and_policy_messages_classify_as_denied() {
        assert!(is_permission_denied(
            "org.freedesktop.DBus.Error.AccessDenied",
            "whatever"
        ));
        assert!(is_permission_denied("", "polkit refused the request"));
        assert!(is_permission_denied("", "not allowed to set permission"));
        assert!(!is_permission_denied("", "no such table"));
    }

    #[test]
    fn an_access_denied_becomes_a_denial_outcome() {
        let error = zbus::Error::FDO(Box::new(zbus::fdo::Error::AccessDenied(
            "polkit refused the request".to_owned(),
        )));
        match classify(error) {
            PrivacyOutcome::Denied(note) => {
                assert!(note.starts_with("PermissionStore: "));
            }
            other => panic!("expected a denial, got {other:?}"),
        }
    }

    #[test]
    fn a_non_authorization_error_is_a_failure() {
        let error = zbus::Error::Failure("no such table".to_owned());
        assert_eq!(
            classify(error),
            PrivacyOutcome::Failed(AdapterError::new("PermissionStore: no such table"))
        );
    }

    #[test]
    fn a_lookup_map_becomes_sorted_app_permissions() {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        map.insert("org.example.Snapshot".to_owned(), vec!["no".to_owned()]);
        map.insert("org.mozilla.firefox".to_owned(), vec!["yes".to_owned()]);
        let apps = apps_from_lookup(map);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].app, "org.example.Snapshot");
        assert_eq!(apps[0].permissions, vec!["no".to_owned()]);
        assert_eq!(apps[1].app, "org.mozilla.firefox");
    }
}
