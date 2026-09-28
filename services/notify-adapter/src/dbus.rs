// SPDX-License-Identifier: MIT
//! The live transport: the notification service over the session bus.
//!
//! The notification service (`services/notifications`) owns the standard
//! `org.freedesktop.Notifications` name and serves the shell-facing
//! `org.dragonfruit.Notifications1` interface at
//! `/org/freedesktop/Notifications`. This source reads the three shell-facing
//! JSON views (`FocusPolicy()`, `Banners()`, `History()`) and writes the two
//! Focus methods (`SetFocusMode`, `SetFocusAllowList`). It never scrapes the
//! freedesktop interface and never reimplements the queue or the policy.
//!
//! The source constructs no connection until it is called, so building an
//! adapter is free and a session without a bus still boots. A call that cannot
//! reach the session bus at all, a bus where no notification daemon owns the
//! name, or a daemon that does not serve our shell interface, are all treated
//! as **absence** — a normal state. A service that owns its name and serves
//! the interface but returns an error or malformed JSON is reported to the
//! adapter, which shows the item visible and inert.

use dragonfruit_notifications::{FocusMode, Urgency};
use dragonfruit_system_adapters::AdapterError;
use zbus::blocking::Connection;

use crate::source::{FocusOutcome, NotificationRecord, NotificationsData, NotificationsSource};

/// The standard notification well-known name (ADR 0056).
pub const NOTIFICATIONS_SERVICE: &str = "org.freedesktop.Notifications";
/// The standard object path both interfaces are served at.
pub const NOTIFICATIONS_PATH: &str = "/org/freedesktop/Notifications";
/// The shell-facing interface the notification service serves at
/// [`NOTIFICATIONS_PATH`].
pub const SHELL_INTERFACE: &str = "org.dragonfruit.Notifications1";

/// The shell-facing notification view over D-Bus.
#[zbus::proxy(
    interface = "org.dragonfruit.Notifications1",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait ShellNotifications {
    /// The active banners as a JSON array, oldest first.
    fn banners(&self) -> zbus::Result<String>;
    /// The recorded history as a JSON array, most recent first.
    fn history(&self) -> zbus::Result<String>;
    /// The Focus/DND policy as JSON (`mode`, `allowList`, `batchedCount`).
    fn focus_policy(&self) -> zbus::Result<String>;
    /// Set the Focus/DND mode; returns whether the service accepted it.
    fn set_focus_mode(&self, mode: &str) -> zbus::Result<bool>;
    /// Replace the per-app Focus allow list.
    fn set_focus_allow_list(&self, apps: &[String]) -> zbus::Result<()>;
}

/// The real notification-service transport.
///
/// [`DbusNotifications::new`] talks to the ambient session bus; `at` pins an
/// explicit bus address so the integration test can drive a private
/// `dbus-daemon`.
#[derive(Debug, Default, Clone)]
pub struct DbusNotifications {
    address: Option<String>,
}

impl DbusNotifications {
    /// A source that reads the ambient session bus on demand.
    pub const fn new() -> Self {
        DbusNotifications { address: None }
    }

    /// A source that reads the session bus at `address`.
    pub fn at(address: impl Into<String>) -> Self {
        DbusNotifications {
            address: Some(address.into()),
        }
    }

    /// Connect to the configured bus, `None` when there is no bus to reach.
    fn connect(&self) -> Option<Connection> {
        match &self.address {
            Some(address) => zbus::blocking::connection::Builder::address(address.as_str())
                .ok()
                .and_then(|builder| builder.build().ok()),
            None => Connection::session().ok(),
        }
    }
}

impl NotificationsSource for DbusNotifications {
    fn read(&mut self) -> Result<Option<NotificationsData>, AdapterError> {
        // No session bus means there is no service to reach: absence, not an
        // error, and never a startup blocker.
        let Some(connection) = self.connect() else {
            return Ok(None);
        };

        let has_owner = name_has_owner(&connection, NOTIFICATIONS_SERVICE).map_err(failed)?;
        if !has_owner {
            return Ok(None);
        }

        let proxy = ShellNotificationsProxyBlocking::new(&connection).map_err(failed)?;
        let focus = match proxy.focus_policy() {
            Ok(json) => json,
            // A daemon owns the standard name but does not serve our shell
            // interface (e.g. another desktop's notification daemon): absence.
            Err(error) if is_absent(&error) => return Ok(None),
            Err(error) => return Err(failed(error)),
        };
        let banners = proxy.banners().map_err(failed)?;
        let history = proxy.history().map_err(failed)?;
        parse_data(&focus, &banners, &history).map(Some)
    }

    fn set_focus_mode(&mut self, mode: FocusMode) -> FocusOutcome {
        let Some(connection) = self.connect() else {
            return FocusOutcome::Absent;
        };
        let has_owner = match name_has_owner(&connection, NOTIFICATIONS_SERVICE) {
            Ok(has_owner) => has_owner,
            Err(error) => return FocusOutcome::Failed(failed(error)),
        };
        if !has_owner {
            return FocusOutcome::Absent;
        }
        let proxy = match ShellNotificationsProxyBlocking::new(&connection) {
            Ok(proxy) => proxy,
            Err(error) => return FocusOutcome::Failed(failed(error)),
        };
        match proxy.set_focus_mode(mode.name()) {
            Ok(true) => FocusOutcome::Applied,
            Ok(false) => FocusOutcome::Failed(AdapterError::new(
                "notification service rejected the focus mode",
            )),
            Err(error) if is_absent(&error) => FocusOutcome::Absent,
            Err(error) => FocusOutcome::Failed(failed(error)),
        }
    }

    fn set_focus_allow_list(&mut self, apps: Vec<String>) -> FocusOutcome {
        let Some(connection) = self.connect() else {
            return FocusOutcome::Absent;
        };
        let has_owner = match name_has_owner(&connection, NOTIFICATIONS_SERVICE) {
            Ok(has_owner) => has_owner,
            Err(error) => return FocusOutcome::Failed(failed(error)),
        };
        if !has_owner {
            return FocusOutcome::Absent;
        }
        let proxy = match ShellNotificationsProxyBlocking::new(&connection) {
            Ok(proxy) => proxy,
            Err(error) => return FocusOutcome::Failed(failed(error)),
        };
        match proxy.set_focus_allow_list(&apps) {
            Ok(()) => FocusOutcome::Applied,
            Err(error) if is_absent(&error) => FocusOutcome::Absent,
            Err(error) => FocusOutcome::Failed(failed(error)),
        }
    }
}

/// Decode the three shell-facing JSON views into the raw [`NotificationsData`].
fn parse_data(
    focus_json: &str,
    banners_json: &str,
    history_json: &str,
) -> Result<NotificationsData, AdapterError> {
    let focus: serde_json::Value = parse_json(focus_json, "FocusPolicy")?;
    let mode = focus
        .get("mode")
        .and_then(|value| value.as_str())
        .and_then(FocusMode::parse)
        .unwrap_or_default();
    let allow_list = focus
        .get("allowList")
        .and_then(|value| value.as_array())
        .map(|values| string_array(values))
        .unwrap_or_default();
    let batched_count = focus
        .get("batchedCount")
        .and_then(|value| value.as_u64())
        .unwrap_or(0)
        .min(u64::from(u32::MAX)) as u32;

    let active = parse_records(parse_json(banners_json, "Banners")?);
    let history = parse_records(parse_json(history_json, "History")?);

    Ok(NotificationsData {
        mode,
        allow_list,
        batched_count,
        active,
        history,
    })
}

/// Parse one JSON view; a malformed payload is an error (the service owned the
/// name and served the interface).
fn parse_json(json: &str, method: &str) -> Result<serde_json::Value, AdapterError> {
    serde_json::from_str(json).map_err(|error| {
        AdapterError::new(format!(
            "notification service {method}(): invalid JSON: {error}"
        ))
    })
}

/// Decode a banner/history array.
fn parse_records(value: serde_json::Value) -> Vec<NotificationRecord> {
    value
        .as_array()
        .map(|records| records.iter().map(parse_record).collect())
        .unwrap_or_default()
}

/// Decode one banner/history object, tolerating a missing optional field.
fn parse_record(value: &serde_json::Value) -> NotificationRecord {
    NotificationRecord {
        id: value
            .get("id")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            .min(u64::from(u32::MAX)) as u32,
        app_name: string_of(value.get("appName")),
        summary: string_of(value.get("summary")),
        urgency: urgency_from_name(&string_of(value.get("urgency"))),
        suppressed: value
            .get("suppressed")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        created_at_ms: value
            .get("createdAt")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
    }
}

/// The strings of a JSON array, dropping any non-string entry.
fn string_array(values: &[serde_json::Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect()
}

/// The string a JSON value holds, `""` for any other type or missing.
fn string_of(value: Option<&serde_json::Value>) -> String {
    value
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .unwrap_or_default()
}

/// Map the wire urgency name back to the typed hint.
fn urgency_from_name(name: &str) -> Urgency {
    match name.trim().to_ascii_lowercase().as_str() {
        "low" => Urgency::Low,
        "critical" => Urgency::Critical,
        _ => Urgency::Normal,
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

/// Whether a D-Bus error means the shell interface (or its owner) is not
/// there, which is absence rather than a read failure.
fn is_absent(error: &zbus::Error) -> bool {
    match error {
        zbus::Error::InterfaceNotFound => true,
        zbus::Error::MethodError(name, ..) => matches!(
            name.as_str(),
            "org.freedesktop.DBus.Error.ServiceUnknown"
                | "org.freedesktop.DBus.Error.NameHasNoOwner"
                | "org.freedesktop.DBus.Error.UnknownInterface"
                | "org.freedesktop.DBus.Error.UnknownMethod"
        ),
        _ => false,
    }
}

/// One read or write failure on the notification-service path.
fn failed(error: zbus::Error) -> AdapterError {
    AdapterError::new(format!("notification service: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_service_name_and_path_are_the_standard_ones() {
        assert_eq!(NOTIFICATIONS_SERVICE, "org.freedesktop.Notifications");
        assert_eq!(NOTIFICATIONS_PATH, "/org/freedesktop/Notifications");
        assert_eq!(SHELL_INTERFACE, "org.dragonfruit.Notifications1");
    }

    #[test]
    fn the_source_is_free_to_construct() {
        let _ = DbusNotifications::new();
        let _ = DbusNotifications::at("unix:path=/nonexistent");
    }

    #[test]
    fn a_missing_bus_address_is_absence() {
        let mut source = DbusNotifications::at("unix:path=/nonexistent-dragonfruit-bus");
        assert_eq!(source.read(), Ok(None));
        assert_eq!(source.set_focus_mode(FocusMode::Dnd), FocusOutcome::Absent);
    }

    #[test]
    fn the_three_views_decode_into_the_raw_data() {
        let focus = r#"{"mode":"dnd","allowList":["Pager","Chat"],"batchedCount":3}"#;
        let banners = r#"[{"id":1,"appName":"Mail","summary":"New message","urgency":"critical","createdAt":1000},{"id":2,"appName":"Chat","summary":"hi","urgency":"low","createdAt":1001}]"#;
        let history = r#"[{"id":3,"appName":"Pager","summary":"on-call","urgency":"normal","suppressed":true,"createdAt":900}]"#;
        let data = parse_data(focus, banners, history).unwrap();
        assert_eq!(data.mode, FocusMode::Dnd);
        assert_eq!(data.allow_list, vec!["Pager".to_owned(), "Chat".to_owned()]);
        assert_eq!(data.batched_count, 3);
        assert_eq!(data.active.len(), 2);
        assert_eq!(data.active[0].app_name, "Mail");
        assert_eq!(data.active[0].urgency, Urgency::Critical);
        assert_eq!(data.active[1].urgency, Urgency::Low);
        assert!(data.history[0].suppressed);
    }

    #[test]
    fn a_malformed_view_is_an_error() {
        let error = parse_data("not json", "[]", "[]").unwrap_err();
        assert!(error.message().contains("FocusPolicy"));
    }

    #[test]
    fn an_unknown_urgency_and_missing_fields_are_tolerated() {
        let data = parse_data(
            r#"{"mode":"off"}"#,
            r#"[{"id":7,"urgency":"vacation"}]"#,
            "[]",
        )
        .unwrap();
        assert_eq!(data.mode, FocusMode::Off);
        assert!(data.allow_list.is_empty());
        assert_eq!(data.active[0].urgency, Urgency::Normal);
        assert_eq!(data.active[0].app_name, "");
        assert!(!data.active[0].suppressed);
    }

    #[test]
    fn absent_dbus_error_names_are_recognized() {
        assert!(is_absent(&zbus::Error::InterfaceNotFound));
        // A generic failure is not absence.
        assert!(!is_absent(&zbus::Error::Failure("boom".to_owned())));
    }
}
