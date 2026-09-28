// SPDX-License-Identifier: MIT
//! The live transport: the AT-SPI accessibility bus over the session bus.
//!
//! Assistive technology on Linux speaks **AT-SPI**. The `at-spi2` stack runs
//! an accessibility bus whose launcher owns the well-known name
//! `org.a11y.Bus` on the **session bus**. That object publishes its status at
//! `/org/a11y/bus` on the standard interface `org.a11y.Status`:
//!
//! * `IsEnabled` — the toolkit accessibility bridge is on; and
//! * `ScreenReaderEnabled` — a screen reader is asking the toolkits for
//!   events.
//!
//! This module reuses that status and never reimplements a screen reader, a
//! toolkit, or a magnifier: the bus owns the value. One read is one
//! `org.freedesktop.DBus.Properties.GetAll` on `org.a11y.Status`.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. No session bus, or
//! a bus with no `org.a11y.Bus` owner, is absence, never an error. A bus that
//! owns its name but cannot be read is reported to the adapter, which shows
//! the item visible and inert.
//!
//! [07-system-integration.md]: ../../../docs/design/07-system-integration.md

use std::collections::HashMap;

use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;
use zbus::zvariant::OwnedValue;

use crate::source::{AccessibilityData, AccessibilitySource};

/// The well-known name the accessibility bus launcher owns.
pub const A11Y_BUS_SERVICE: &str = "org.a11y.Bus";
/// The object path the status is served at.
pub const A11Y_BUS_PATH: &str = "/org/a11y/bus";
/// The interface carrying the `IsEnabled`/`ScreenReaderEnabled` properties.
pub const A11Y_STATUS_INTERFACE: &str = "org.a11y.Status";

/// The live AT-SPI accessibility-bus source.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HostAccessibility;

impl HostAccessibility {
    /// A source reading the accessibility bus with no bus connection until
    /// `read`.
    pub fn new() -> Self {
        HostAccessibility
    }
}

impl AccessibilitySource for HostAccessibility {
    fn read(&mut self) -> Result<Option<AccessibilityData>, AdapterError> {
        // No session bus means no accessibility bus can be reached: absence,
        // not an error, and never a startup blocker.
        let connection = match Connection::session() {
            Ok(connection) => connection,
            Err(_) => return Ok(None),
        };
        if !name_has_owner(&connection, A11Y_BUS_SERVICE).map_err(failed)? {
            return Ok(None);
        }
        let properties = status_properties(&connection).map_err(failed)?;
        Ok(Some(status_from_properties(properties)))
    }
}

/// The `org.a11y.Status` properties, as `GetAll` reports them.
///
/// `GetAll` answers the D-Bus-generic `a{sv}`; each boolean is unwrapped at
/// this boundary. A property carrying an unexpected type is honestly `false`,
/// never a read failure.
fn status_properties(connection: &Connection) -> zbus::Result<HashMap<String, bool>> {
    let reply = connection.call_method(
        Some(A11Y_BUS_SERVICE),
        A11Y_BUS_PATH,
        Some("org.freedesktop.DBus.Properties"),
        "GetAll",
        &(A11Y_STATUS_INTERFACE,),
    )?;
    let raw: HashMap<String, OwnedValue> = reply.body().deserialize()?;
    Ok(raw
        .into_iter()
        .map(|(name, value)| (name, bool::try_from(value).unwrap_or(false)))
        .collect())
}

/// Build the typed status from one `GetAll` map. Pure, so a fixture map drives
/// it in tests. A property the stack omits is honestly `false`.
pub fn status_from_properties(mut properties: HashMap<String, bool>) -> AccessibilityData {
    AccessibilityData {
        enabled: properties.remove("IsEnabled").unwrap_or(false),
        screen_reader: properties.remove("ScreenReaderEnabled").unwrap_or(false),
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

/// One read failure on the live path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("AT-SPI: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_is_free_to_construct() {
        // Constructing the source must not touch the bus; only `read` does.
        let _ = HostAccessibility::new();
    }

    #[test]
    fn the_service_names_are_the_at_spi_ones() {
        assert_eq!(A11Y_BUS_SERVICE, "org.a11y.Bus");
        assert_eq!(A11Y_BUS_PATH, "/org/a11y/bus");
        assert_eq!(A11Y_STATUS_INTERFACE, "org.a11y.Status");
    }

    #[test]
    fn a_read_failure_carries_the_stack_name() {
        let error = failed(zbus::Error::Failure("timeout".into()));
        assert!(error.message().starts_with("AT-SPI: "));
    }

    #[test]
    fn a_properties_map_becomes_the_typed_status() {
        let mut properties: HashMap<String, bool> = HashMap::new();
        properties.insert("IsEnabled".to_owned(), true);
        properties.insert("ScreenReaderEnabled".to_owned(), true);
        assert_eq!(
            status_from_properties(properties),
            AccessibilityData {
                enabled: true,
                screen_reader: true,
            }
        );
    }

    #[test]
    fn a_missing_property_is_honestly_false() {
        let mut properties: HashMap<String, bool> = HashMap::new();
        properties.insert("IsEnabled".to_owned(), true);
        assert_eq!(
            status_from_properties(properties),
            AccessibilityData {
                enabled: true,
                screen_reader: false,
            }
        );
        assert_eq!(
            status_from_properties(HashMap::new()),
            AccessibilityData::default()
        );
    }
}
