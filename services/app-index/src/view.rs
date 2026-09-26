// SPDX-License-Identifier: MIT
//! Flat JSON views of the app index (T-14.1a).
//!
//! The shell decodes these; they are intentionally flat and stringly-typed so
//! the Qt side needs no D-Bus type knowledge beyond `s`. This mirrors the
//! notification service's `view` module.

use serde_json::{json, Value};

use crate::icons::IconTheme;
use crate::index::{AppIndex, AppRecord, IdentitySource};

/// The size at which records carry a resolved `iconPath` by default. The Dock
/// and menu bar scale from here.
pub const DEFAULT_ICON_SIZE: i32 = 128;

/// One record as a JSON object; `source` and `iconPath` are included when
/// known. `iconPath` is empty when no theme provides the icon.
pub fn record_value(
    record: &AppRecord,
    source: Option<IdentitySource>,
    theme: &IconTheme,
) -> Value {
    let icon_path = if record.icon.is_empty() {
        String::new()
    } else {
        theme
            .lookup(&record.icon, DEFAULT_ICON_SIZE)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let mut object = json!({
        "valid": true,
        "desktopId": record.desktop_id,
        "name": record.name,
        "icon": record.icon,
        "iconPath": icon_path,
        "exec": record.exec,
        "terminal": record.terminal,
        "noDisplay": record.no_display,
        "categories": record.categories,
        "startupWmClass": record.startup_wm_class,
    });
    if let Some(source) = source {
        object["source"] = Value::String(source.as_str().to_owned());
    }
    object
}

/// A resolved record as a JSON string (or `""` on a miss).
pub fn record_json(record: &AppRecord, source: IdentitySource, theme: &IconTheme) -> String {
    record_value(record, Some(source), theme).to_string()
}

/// Every installed record as a JSON array of objects.
pub fn records_json(index: &AppIndex, theme: &IconTheme) -> String {
    let records: Vec<Value> = index
        .records()
        .map(|record| record_value(record, None, theme))
        .collect();
    Value::Array(records).to_string()
}

/// The recorded misses as a JSON array of strings.
pub fn misses_json(index: &AppIndex) -> String {
    let misses: Vec<Value> = index
        .misses()
        .map(|miss| Value::String(miss.to_owned()))
        .collect();
    Value::Array(misses).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::AppRecord;

    fn record() -> AppRecord {
        AppRecord {
            desktop_id: "firefox.desktop".to_owned(),
            name: "Firefox".to_owned(),
            icon: "firefox".to_owned(),
            exec: "firefox %u".to_owned(),
            ..Default::default()
        }
    }

    #[test]
    fn a_record_serializes_with_an_empty_icon_path_when_unthemed() {
        let theme = IconTheme::from_roots([], ["hicolor".to_owned()], []);
        let value: Value =
            serde_json::from_str(&record_json(&record(), IdentitySource::AppId, &theme)).unwrap();
        assert_eq!(value["desktopId"], "firefox.desktop");
        assert_eq!(value["source"], "app_id");
        assert_eq!(value["icon"], "firefox");
        assert_eq!(value["iconPath"], "");
        assert_eq!(value["valid"], true);
    }
}
