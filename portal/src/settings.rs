// SPDX-License-Identifier: MIT
//! The Settings portal's pure model and live source (T-13.1b).
//!
//! The backend serves the standard read-only
//! `org.freedesktop.impl.portal.Settings` interface so toolkits and sandboxed
//! applications can ask the desktop for its appearance (a dark-mode hint,
//! contrast, the accent colour) and for the desktop's own settings.
//!
//! The values are never owned here: `settingsd` is the single writer
//! ([01-architecture.md](../../docs/design/01-architecture.md)). This module
//! only *projects* the `org.dragonfruit.Settings1` snapshot onto the two
//! portal namespaces:
//!
//! * [`APPEARANCE_NAMESPACE`] (`org.freedesktop.appearance`) — the standard
//!   cross-toolkit keys `color-scheme`, `contrast`, and (when the accent is a
//!   concrete `#rrggbb`) `accent-color`.
//! * [`DESKTOP_NAMESPACE`] (`org.dragonfruit.desktop`) — every settingsd key
//!   under its own name, read-only, so a caller can read our whole schema.
//!
//! # Absent settingsd is normal
//!
//! [`SettingsSnapshot::new`] seeds the appearance keys with the honest
//! defaults (`color-scheme` = no preference, `contrast` = 0). A session with
//! no `settingsd` therefore still answers; when `settingsd` appears (or
//! restarts) [`spawn_settings_sync`] replaces the snapshot live and emits
//! `SettingChanged` for every key that moved. Nothing polls.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use zbus::blocking::connection;
use zbus::zvariant::{Array, OwnedValue};

/// The standard appearance namespace every toolkit queries.
pub const APPEARANCE_NAMESPACE: &str = "org.freedesktop.appearance";
/// The Dragonfruit read-only namespace exposing every settingsd key.
pub const DESKTOP_NAMESPACE: &str = "org.dragonfruit.desktop";

/// The Settings portal version (`org.freedesktop.impl.portal.Settings`).
pub const SETTINGS_VERSION: u32 = 2;

/// The standard appearance keys.
pub const COLOR_SCHEME_KEY: &str = "color-scheme";
pub const CONTRAST_KEY: &str = "contrast";
pub const ACCENT_COLOR_KEY: &str = "accent-color";

/// `color-scheme` values, per the portal spec.
pub const COLOR_SCHEME_NO_PREFERENCE: u32 = 0;
pub const COLOR_SCHEME_PREFER_DARK: u32 = 1;
pub const COLOR_SCHEME_PREFER_LIGHT: u32 = 2;

/// The settingsd keys this projection reads.
pub const SETTINGS_COLOR_SCHEME_KEY: &str = "appearance.colorScheme";
pub const SETTINGS_ACCENT_KEY: &str = "appearance.accent";

/// `settingsd`'s well-known name, object path, and interface — the contract
/// the backend reads from (frozen in
/// [ADR 0030](../../docs/design/adr/0030-settingsd-schema-and-dbus-surface.md)).
pub const SETTINGS_SERVICE_NAME: &str = "org.dragonfruit.Settings1";
pub const SETTINGS_SERVICE_PATH: &str = "/org/dragonfruit/Settings1";
pub const SETTINGS_SERVICE_INTERFACE: &str = "org.dragonfruit.Settings1";

/// One read-only projection of the desktop settings, keyed by portal
/// namespace then key. Values stay in their D-Bus wire shape so they can be
/// re-emitted unchanged.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsSnapshot {
    namespaces: BTreeMap<String, BTreeMap<String, OwnedValue>>,
}

impl SettingsSnapshot {
    /// The snapshot for a session with no `settingsd`: the appearance
    /// namespace seeded with its honest defaults, the desktop namespace
    /// empty.
    pub fn new() -> Self {
        let mut snapshot = SettingsSnapshot {
            namespaces: BTreeMap::new(),
        };
        snapshot.refresh_appearance();
        snapshot
    }

    /// Project settingsd's `GetAll` reply (`a{sv}`) into the portal
    /// namespaces.
    pub fn from_entries(entries: impl IntoIterator<Item = (String, OwnedValue)>) -> Self {
        let mut snapshot = SettingsSnapshot {
            namespaces: BTreeMap::new(),
        };
        let desktop = snapshot
            .namespaces
            .entry(DESKTOP_NAMESPACE.to_owned())
            .or_default();
        for (key, value) in entries {
            desktop.insert(key, value);
        }
        snapshot.refresh_appearance();
        snapshot
    }

    /// A value, if the namespace and key exist.
    pub fn read(&self, namespace: &str, key: &str) -> Option<&OwnedValue> {
        self.namespaces
            .get(namespace)
            .and_then(|keys| keys.get(key))
    }

    /// The keys of one namespace. An empty namespace means "all namespaces",
    /// matching the portal's `ReadAll` convention.
    pub fn read_all(&self, namespace: &str) -> BTreeMap<String, BTreeMap<String, OwnedValue>> {
        if namespace.is_empty() {
            return self.namespaces.clone();
        }
        self.namespaces
            .get(namespace)
            .map(|keys| {
                let mut only = BTreeMap::new();
                only.insert(namespace.to_owned(), keys.clone());
                only
            })
            .unwrap_or_default()
    }

    /// The namespaces this snapshot exposes, in stable order.
    pub fn namespaces(&self) -> impl Iterator<Item = &str> {
        self.namespaces.keys().map(String::as_str)
    }

    /// A settingsd key's value, when present in the projection.
    pub fn desktop_key(&self, key: &str) -> Option<&OwnedValue> {
        self.read(DESKTOP_NAMESPACE, key)
    }

    /// Every `(namespace, key, value)` in `other` that differs from `self`.
    /// Used to turn one settingsd `Changed` into exactly the portal
    /// `SettingChanged` signals that are real changes.
    pub fn diff(&self, other: &SettingsSnapshot) -> Vec<(String, String, OwnedValue)> {
        let mut changes = Vec::new();
        for (namespace, keys) in &other.namespaces {
            for (key, value) in keys {
                if self.read(namespace, key) != Some(value) {
                    changes.push((namespace.clone(), key.clone(), value.clone()));
                }
            }
        }
        changes
    }

    /// Recompute the derived `org.freedesktop.appearance` values from the
    /// desktop namespace. `color-scheme` and `contrast` are always present;
    /// `accent-color` is present only when `appearance.accent` is a concrete
    /// `#rrggbb`.
    fn refresh_appearance(&mut self) {
        let scheme = match self.desktop_text(SETTINGS_COLOR_SCHEME_KEY).as_deref() {
            Some("dark") => COLOR_SCHEME_PREFER_DARK,
            Some("light") => COLOR_SCHEME_PREFER_LIGHT,
            _ => COLOR_SCHEME_NO_PREFERENCE,
        };
        let accent = self
            .desktop_text(SETTINGS_ACCENT_KEY)
            .and_then(|text| parse_hex_color(&text))
            .and_then(accent_value);

        let appearance = self
            .namespaces
            .entry(APPEARANCE_NAMESPACE.to_owned())
            .or_default();
        appearance.insert(COLOR_SCHEME_KEY.to_owned(), OwnedValue::from(scheme));
        appearance.insert(CONTRAST_KEY.to_owned(), OwnedValue::from(0u32));
        match accent {
            Some(value) => {
                appearance.insert(ACCENT_COLOR_KEY.to_owned(), value);
            }
            None => {
                appearance.remove(ACCENT_COLOR_KEY);
            }
        }
    }

    /// A desktop-namespace value decoded as text.
    fn desktop_text(&self, key: &str) -> Option<String> {
        let value = self.desktop_key(key)?;
        String::try_from(value.try_clone().ok()?).ok()
    }
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        SettingsSnapshot::new()
    }
}

/// The shared, live settings projection: the D-Bus object reads it, the sync
/// thread replaces it.
pub type SettingsStore = Arc<Mutex<SettingsSnapshot>>;

/// A store seeded with the appearance defaults.
pub fn store() -> SettingsStore {
    Arc::new(Mutex::new(SettingsSnapshot::new()))
}

/// Lock a store, recovering from a poisoned mutex so a panicking D-Bus call
/// never takes the service down.
pub fn lock(store: &SettingsStore) -> MutexGuard<'_, SettingsSnapshot> {
    store
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Parse `#rrggbb` into normalized components.
fn parse_hex_color(text: &str) -> Option<(f64, f64, f64)> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let component = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16)
            .ok()
            .map(|value| f64::from(value) / 255.0)
    };
    Some((component(0..2)?, component(2..4)?, component(4..6)?))
}

/// Build the `a(ddd)` accent-color value the appearance namespace expects.
fn accent_value((red, green, blue): (f64, f64, f64)) -> Option<OwnedValue> {
    let mut array = Array::new(<(f64, f64, f64) as zbus::zvariant::Type>::SIGNATURE);
    array
        .append(zbus::zvariant::Value::Structure(
            zbus::zvariant::Structure::from((red, green, blue)),
        ))
        .ok()?;
    OwnedValue::try_from(array).ok()
}

/// Ask settingsd for its full snapshot. `None` when the daemon is absent, the
/// bus cannot be asked, or the reply does not decode: all normal states.
pub fn fetch(connection: &connection::Connection) -> Option<SettingsSnapshot> {
    let reply = connection
        .call_method(
            Some(SETTINGS_SERVICE_NAME),
            SETTINGS_SERVICE_PATH,
            Some(SETTINGS_SERVICE_INTERFACE),
            "GetAll",
            &(),
        )
        .ok()?;
    let entries: std::collections::HashMap<String, OwnedValue> = reply.body().deserialize().ok()?;
    Some(SettingsSnapshot::from_entries(entries))
}

/// Resync `store` from settingsd and emit `SettingChanged` for every portal
/// key that moved. A no-op when settingsd cannot be reached.
pub fn sync(connection: &connection::Connection, store: &SettingsStore) {
    let Some(fresh) = fetch(connection) else {
        return;
    };
    let changes = {
        let current = lock(store);
        if *current == fresh {
            return;
        }
        current.diff(&fresh)
    };
    *lock(store) = fresh;
    for (namespace, key, value) in changes {
        let _ = connection.emit_signal(
            None::<&str>,
            crate::model::DBUS_PATH,
            crate::interfaces::SETTINGS_INTERFACE,
            "SettingChanged",
            &(namespace.as_str(), key.as_str(), value),
        );
    }
}

/// Start the live sync: one initial resync, then a resync on every settingsd
/// `Changed` and on every `org.dragonfruit.Settings1` name (re)appearance.
/// The thread is detached for the life of the process; a dead bus ends it.
pub fn spawn_settings_sync(connection: connection::Connection, store: SettingsStore) {
    let _ = std::thread::Builder::new()
        .name("dragonfruit-portal-settings".to_owned())
        .spawn(move || {
            sync(&connection, &store);
            spawn_name_watch(connection.clone(), store.clone());
            changed_loop(connection, store);
        });
}

/// A second thread that resyncs whenever settingsd's name appears (startup,
/// restart, crash recovery).
fn spawn_name_watch(connection: connection::Connection, store: SettingsStore) {
    let _ = std::thread::Builder::new()
        .name("dragonfruit-portal-settings-name".to_owned())
        .spawn(move || {
            let rule = zbus::MatchRule::builder()
                .msg_type(zbus::message::Type::Signal)
                .sender("org.freedesktop.DBus")
                .expect("valid sender")
                .interface("org.freedesktop.DBus")
                .expect("valid interface")
                .member("NameOwnerChanged")
                .expect("valid member")
                .add_arg(SETTINGS_SERVICE_NAME)
                .expect("valid arg")
                .build();
            let Ok(iterator) =
                zbus::blocking::MessageIterator::for_match_rule(rule, &connection, Some(4))
            else {
                return;
            };
            for message in iterator {
                if message.is_err() {
                    break;
                }
                sync(&connection, &store);
            }
        });
}

/// The settingsd `Changed` loop; every signal triggers a full resync so the
/// derived appearance keys stay correct.
fn changed_loop(connection: connection::Connection, store: SettingsStore) {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .path(SETTINGS_SERVICE_PATH)
        .expect("valid path")
        .interface(SETTINGS_SERVICE_INTERFACE)
        .expect("valid interface")
        .member("Changed")
        .expect("valid member")
        .build();
    let Ok(iterator) = zbus::blocking::MessageIterator::for_match_rule(rule, &connection, Some(8))
    else {
        return;
    };
    for message in iterator {
        if message.is_err() {
            break;
        }
        sync(&connection, &store);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> OwnedValue {
        OwnedValue::from(zbus::zvariant::Str::from(value))
    }

    #[test]
    fn absent_settingsd_still_exposes_the_appearance_defaults() {
        let snapshot = SettingsSnapshot::new();
        assert_eq!(
            snapshot.read(APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY),
            Some(&OwnedValue::from(COLOR_SCHEME_NO_PREFERENCE))
        );
        assert_eq!(
            snapshot.read(APPEARANCE_NAMESPACE, CONTRAST_KEY),
            Some(&OwnedValue::from(0u32))
        );
        assert_eq!(snapshot.read(APPEARANCE_NAMESPACE, ACCENT_COLOR_KEY), None);
        assert!(snapshot.desktop_key("appearance.colorScheme").is_none());
    }

    #[test]
    fn the_scheme_projects_onto_the_standard_tristate() {
        for (value, expected) in [
            ("dark", COLOR_SCHEME_PREFER_DARK),
            ("light", COLOR_SCHEME_PREFER_LIGHT),
            ("auto", COLOR_SCHEME_NO_PREFERENCE),
        ] {
            let snapshot = SettingsSnapshot::from_entries([(
                SETTINGS_COLOR_SCHEME_KEY.to_owned(),
                text(value),
            )]);
            assert_eq!(
                snapshot.read(APPEARANCE_NAMESPACE, COLOR_SCHEME_KEY),
                Some(&OwnedValue::from(expected)),
                "colorScheme={value}"
            );
        }
    }

    #[test]
    fn every_desktop_key_is_reachable_under_its_own_name() {
        let snapshot = SettingsSnapshot::from_entries([
            ("dock.size".to_owned(), OwnedValue::from(0.5f64)),
            ("appearance.colorScheme".to_owned(), text("dark")),
        ]);
        assert_eq!(
            snapshot.desktop_key("dock.size"),
            Some(&OwnedValue::from(0.5f64))
        );
        // ReadAll of the desktop namespace carries both keys.
        let all = snapshot.read_all(DESKTOP_NAMESPACE);
        assert_eq!(all.get(DESKTOP_NAMESPACE).unwrap().len(), 2);
    }

    #[test]
    fn a_concrete_accent_becomes_the_a_ddd_triple() {
        let snapshot =
            SettingsSnapshot::from_entries([(SETTINGS_ACCENT_KEY.to_owned(), text("#ff0000"))]);
        assert!(snapshot
            .read(APPEARANCE_NAMESPACE, ACCENT_COLOR_KEY)
            .is_some());

        // An empty or malformed accent is simply absent, never a bogus
        // colour.
        for bad in ["", "red", "#12345", "#zzzzzz"] {
            let snapshot =
                SettingsSnapshot::from_entries([(SETTINGS_ACCENT_KEY.to_owned(), text(bad))]);
            assert_eq!(
                snapshot.read(APPEARANCE_NAMESPACE, ACCENT_COLOR_KEY),
                None,
                "accent={bad:?}"
            );
        }
    }

    #[test]
    fn the_diff_reports_only_real_changes() {
        let before =
            SettingsSnapshot::from_entries([(SETTINGS_COLOR_SCHEME_KEY.to_owned(), text("auto"))]);
        let after = SettingsSnapshot::from_entries([
            (SETTINGS_COLOR_SCHEME_KEY.to_owned(), text("dark")),
            ("dock.size".to_owned(), OwnedValue::from(0.25f64)),
        ]);
        let changes = before.diff(&after);
        // color-scheme moved, a desktop key appeared, and contrast stayed 0
        // (equal, so not reported).
        assert!(changes
            .iter()
            .any(|(ns, key, _)| ns == APPEARANCE_NAMESPACE && key == COLOR_SCHEME_KEY));
        assert!(changes
            .iter()
            .any(|(ns, key, _)| ns == DESKTOP_NAMESPACE && key == "dock.size"));
        assert!(!changes
            .iter()
            .any(|(ns, key, _)| ns == APPEARANCE_NAMESPACE && key == CONTRAST_KEY));
    }
}
