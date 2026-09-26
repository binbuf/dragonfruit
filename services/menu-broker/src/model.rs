// SPDX-License-Identifier: MIT
//! The pure global-menu model (T-14.2a).
//!
//! A global menu does not exist on the wire; an application has to export a
//! model ([06-global-menu.md](../../docs/design/06-global-menu.md)). The broker
//! resolves the focused application's model with a strict priority order
//! (native publication → DBusMenu → no exporter) and always carries the fixed
//! **application menu** — the macOS-style About/Settings/Hide/Hide Others/Show
//! All/Quit menu — with **live** enabled state for the hide verbs.
//!
//! The system menu (the dragonfruit mark) stays shell/session-owned and is not
//! part of this model.
//!
//! # Payloads
//!
//! The native publication contract is the design system's entry shape (ADR
//! [0041](../../docs/design/adr/0041-native-menu-model-publication-shape.md)):
//!
//! ```json
//! { "appName": "Settings",
//!   "applicationMenuItems": [ { "label": "Hide Settings", "action": "hide" } ],
//!   "menus": [ { "title": "File", "items": [ { "label": "New" } ] } ] }
//! ```
//!
//! The visibility input is a flat list of per-app window state:
//!
//! ```json
//! [ { "appId": "org.dragonfruit.Settings", "windows": 1, "minimized": false } ]
//! ```
//!
//! `minimized` is true only when **all** of the app's windows are minimized
//! (the shell's Dock projection already carries this reading). An app with at
//! least one non-minimized window is *visible*; otherwise it is *hidden*.
//!
//! No D-Bus and no I/O live here, so the whole contract is unit-testable.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::accelerators::{self, Accelerator, AcceleratorTable, Chord, Dispatch};

/// The tier an app's menu resolved through.
pub const TIER_NATIVE: &str = "native";
/// DBusMenu/AppMenu (bridge lands in T-14.4).
pub const TIER_DBUSMENU: &str = "dbusmenu";
/// No exporter: the fixed system + application menus only.
pub const TIER_NONE: &str = "none";

/// The default application menu's hide verbs, in menu order.
pub const ACTION_HIDE: &str = "hide";
pub const ACTION_HIDE_OTHERS: &str = "hide-others";
pub const ACTION_SHOW_ALL: &str = "show-all";

/// The resolution tier, in priority order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    /// Our native API: a first-party app published a model directly.
    Native,
    /// DBusMenu/AppMenu (the compatibility bridge, T-14.4).
    DbusMenu,
    /// No exporter; the fixed menus render.
    None,
}

impl Tier {
    /// The wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Native => TIER_NATIVE,
            Tier::DbusMenu => TIER_DBUSMENU,
            Tier::None => TIER_NONE,
        }
    }
}

/// A first-party app's published menu model, parsed from the design-system
/// entry shape. Rows are kept as JSON objects and passed through untouched
/// (the broker only rewrites the fixed menu's live `enabled` flags).
#[derive(Debug, Clone, PartialEq)]
pub struct PublishedModel {
    /// The application's name, used for the application menu's labels.
    pub app_name: String,
    /// The complete fixed application menu the app publishes (may be empty,
    /// in which case the broker synthesizes the standard set).
    pub application_menu_items: Vec<Value>,
    /// The app's own top-level menus, after the fixed two.
    pub menus: Vec<Value>,
}

impl PublishedModel {
    /// Parse one published model. Returns `None` when the payload is not an
    /// object or carries no `appName`; a malformed exporter must never crash
    /// the broker, so unknown and missing fields degrade to empty.
    pub fn parse(payload: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(payload).ok()?;
        let object = value.as_object()?;
        let app_name = object.get("appName").and_then(Value::as_str)?.to_owned();
        Some(PublishedModel {
            app_name,
            application_menu_items: array_or_empty(object.get("applicationMenuItems")),
            menus: array_or_empty(object.get("menus")),
        })
    }
}

fn array_or_empty(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}

/// One app's window state, as the shell's Dock projection carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowState {
    /// The application identity.
    pub app_id: String,
    /// How many windows the app has.
    pub windows: u32,
    /// True only when *all* the app's windows are minimized (hidden).
    pub minimized: bool,
}

impl WindowState {
    /// A visible app (at least one window, none minimized).
    pub fn is_visible(&self) -> bool {
        self.windows > 0 && !self.minimized
    }

    /// A hidden app (has windows, and every one is minimized).
    pub fn is_hidden(&self) -> bool {
        self.windows > 0 && self.minimized
    }
}

/// The focused window's app plus every app's window state: the input the fixed
/// application menu's live state is derived from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Visibility {
    focused: Option<String>,
    apps: Vec<WindowState>,
}

impl Visibility {
    /// A new, empty visibility model (nothing focused, no windows).
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from an explicit focused app and app list.
    pub fn from_parts(focused: Option<String>, apps: Vec<WindowState>) -> Self {
        Visibility { focused, apps }
    }

    /// The focused app id, if any.
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Every app's state.
    pub fn apps(&self) -> &[WindowState] {
        &self.apps
    }

    /// Set (or clear, with an empty id) the focused app.
    pub fn set_focused(&mut self, app_id: &str) -> bool {
        let next = if app_id.is_empty() {
            None
        } else {
            Some(app_id.to_owned())
        };
        if self.focused == next {
            return false;
        }
        self.focused = next;
        true
    }

    /// Replace the window-state list.
    pub fn set_apps(&mut self, apps: Vec<WindowState>) -> bool {
        if self.apps == apps {
            return false;
        }
        self.apps = apps;
        true
    }

    fn state_for(&self, app_id: &str) -> Option<&WindowState> {
        if app_id.is_empty() {
            return None;
        }
        self.apps.iter().find(|state| state.app_id == app_id)
    }

    /// Whether a specific app currently has a visible (non-minimized) window.
    pub fn app_visible(&self, app_id: &str) -> bool {
        self.state_for(app_id).is_some_and(WindowState::is_visible)
    }

    /// Whether a specific app is fully hidden.
    pub fn app_hidden(&self, app_id: &str) -> bool {
        self.state_for(app_id).is_some_and(WindowState::is_hidden)
    }

    /// Hide `<App>` is meaningful only when the focused app has a visible
    /// window to hide.
    pub fn hide_enabled(&self) -> bool {
        self.focused
            .as_deref()
            .is_some_and(|app| self.app_visible(app))
    }

    /// Hide Others is meaningful when the focused app can hide at least one
    /// other app that still has a visible window.
    pub fn hide_others_enabled(&self) -> bool {
        let Some(focused) = self.focused.as_deref() else {
            return false;
        };
        self.apps
            .iter()
            .any(|state| state.app_id != focused && state.is_visible())
    }

    /// Show All is meaningful when at least one app is fully hidden.
    pub fn show_all_enabled(&self) -> bool {
        self.apps.iter().any(WindowState::is_hidden)
    }

    /// The live enabled flags for the three hide verbs.
    pub fn hide_verbs(&self) -> HideVerbs {
        HideVerbs {
            hide: self.hide_enabled(),
            hide_others: self.hide_others_enabled(),
            show_all: self.show_all_enabled(),
        }
    }
}

/// The live enabled state of the application menu's hide verbs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HideVerbs {
    /// Hide `<App>`.
    pub hide: bool,
    /// Hide Others.
    pub hide_others: bool,
    /// Show All.
    pub show_all: bool,
}

/// The hello-world app name when nothing is focused: the desktop is Files
/// (the macOS "Finder owns the desktop" model).
pub const DESKTOP_APP_NAME: &str = "Files";

/// A reverse-DNS app id's human name: the final segment, leading character
/// capitalized (`org.dragonfruit.Settings` → `Settings`). Empty is the
/// desktop (`Files`).
pub fn default_app_name(app_id: &str) -> String {
    if app_id.is_empty() {
        return DESKTOP_APP_NAME.to_owned();
    }
    let segment = app_id.rsplit('.').next().unwrap_or(app_id);
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => app_id.to_owned(),
    }
}

fn string_field(value: &Value, key: &str) -> String {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn is_separator(value: &Value) -> bool {
    string_field(value, "type") == "separator"
}

/// A synthesized fixed-application-menu row.
fn entry(label: &str, action: &str, shortcut: Option<&str>, enabled: bool) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("label".to_owned(), Value::String(label.to_owned()));
    if let Some(shortcut) = shortcut {
        object.insert("shortcut".to_owned(), Value::String(shortcut.to_owned()));
    }
    object.insert("action".to_owned(), Value::String(action.to_owned()));
    object.insert("enabled".to_owned(), Value::Bool(enabled));
    Value::Object(object)
}

fn separator() -> Value {
    json!({ "type": "separator" })
}

/// The standard fixed application menu, synthesized from the app's name and
/// the live hide-verb state. This is the Tier-3 / no-publication case and the
/// fallback when a publisher carries no `applicationMenuItems`.
pub fn synthesized_application_menu(app_name: &str, verbs: HideVerbs) -> Vec<Value> {
    vec![
        entry(&format!("About {app_name}"), "about", None, true),
        entry("Settings\u{2026}", "settings", Some("Super+,"), true),
        separator(),
        entry(
            &format!("Hide {app_name}"),
            ACTION_HIDE,
            Some("Super+H"),
            verbs.hide,
        ),
        entry(
            "Hide Others",
            ACTION_HIDE_OTHERS,
            Some("Super+Alt+H"),
            verbs.hide_others,
        ),
        entry("Show All", ACTION_SHOW_ALL, None, verbs.show_all),
        separator(),
        entry(&format!("Quit {app_name}"), "quit", Some("Super+Q"), true),
    ]
}

/// The live enabled flag a row's action maps to, if it is one of the hide
/// verbs. The action is authoritative; the label is a tolerant fallback for
/// exporters that omit actions.
fn hide_flag(value: &Value, verbs: HideVerbs) -> Option<bool> {
    let action = string_field(value, "action");
    match action.as_str() {
        ACTION_HIDE => return Some(verbs.hide),
        ACTION_HIDE_OTHERS => return Some(verbs.hide_others),
        ACTION_SHOW_ALL => return Some(verbs.show_all),
        _ => {}
    }
    let label = string_field(value, "label");
    match label.as_str() {
        "Hide Others" => Some(verbs.hide_others),
        "Show All" => Some(verbs.show_all),
        other if other.starts_with("Hide ") => Some(verbs.hide),
        _ => None,
    }
}

/// The fixed application menu: the app's published fixed menu when it carries
/// one (with the live hide-verb flags rewritten), else the standard synthesis.
///
/// A published row that is not a hide verb is passed through byte-for-byte so
/// the app's declaration is the single source of its own labels and actions
/// (ADR 0041).
pub fn fixed_application_menu(
    app_name: &str,
    published: Option<&[Value]>,
    visibility: &Visibility,
) -> Vec<Value> {
    let verbs = visibility.hide_verbs();
    let Some(published) = published.filter(|rows| !rows.is_empty()) else {
        return synthesized_application_menu(app_name, verbs);
    };
    published
        .iter()
        .map(|row| {
            if is_separator(row) {
                return row.clone();
            }
            match hide_flag(row, verbs) {
                Some(enabled) => {
                    let mut object = row.as_object().cloned().unwrap_or_default();
                    object.insert("enabled".to_owned(), Value::Bool(enabled));
                    Value::Object(object)
                }
                None => row.clone(),
            }
        })
        .collect()
}

/// The broker's resolution state: publishers by app id, the current focus and
/// window state, and a monotonic revision.
#[derive(Debug, Clone)]
pub struct Broker {
    publishers: BTreeMap<String, PublishedModel>,
    visibility: Visibility,
    accelerators: AcceleratorTable,
    revision: u64,
}

impl Default for Broker {
    fn default() -> Self {
        Broker::new()
    }
}

impl Broker {
    /// A broker with no publishers and no focus.
    pub fn new() -> Self {
        Broker {
            publishers: BTreeMap::new(),
            visibility: Visibility::new(),
            accelerators: AcceleratorTable::new(),
            revision: 0,
        }
    }

    /// The current revision; bumped on every state change.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// How many apps have published a model.
    pub fn publisher_count(&self) -> usize {
        self.publishers.len()
    }

    /// The current visibility model.
    pub fn visibility(&self) -> &Visibility {
        &self.visibility
    }

    /// Register or replace an app's published model. Returns false (and
    /// changes nothing) when the payload does not parse; a malformed exporter
    /// must never take the broker down or clobber a good model.
    pub fn publish(&mut self, app_id: &str, payload: &str) -> bool {
        if app_id.is_empty() {
            return false;
        }
        let Some(model) = PublishedModel::parse(payload) else {
            return false;
        };
        let changed = self.publishers.get(app_id) != Some(&model);
        let table = accelerators::extract(&model);
        self.publishers.insert(app_id.to_owned(), model);
        self.accelerators.register(app_id, table);
        if changed {
            self.revision += 1;
        }
        changed
    }

    /// Remove an app's published model and its accelerators. Returns whether
    /// one was present.
    pub fn withdraw(&mut self, app_id: &str) -> bool {
        let removed = self.publishers.remove(app_id).is_some();
        self.accelerators.clear(app_id);
        if removed {
            self.revision += 1;
        }
        removed
    }

    /// Set the focused app. Returns whether the state changed.
    pub fn set_focused(&mut self, app_id: &str) -> bool {
        let changed = self.visibility.set_focused(app_id);
        self.accelerators.set_focused(app_id);
        if changed {
            self.revision += 1;
        }
        changed
    }

    /// Replace the reserved system chords (a JSON array of chord strings). The
    /// shell declares them so the broker never dispatches a chord the
    /// compositor owns. Malformed entries are skipped. Returns `false` only
    /// when the payload is not an array.
    pub fn set_system_accelerators(&mut self, payload: &str) -> bool {
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            return false;
        };
        let Some(array) = value.as_array() else {
            return false;
        };
        let chords: Vec<Chord> = array
            .iter()
            .filter_map(Value::as_str)
            .filter_map(Chord::parse)
            .collect();
        if self.accelerators.set_system(chords) {
            self.revision += 1;
        }
        true
    }

    /// One app's registered accelerators, in publication order.
    pub fn accelerators_for(&self, app_id: &str) -> &[Accelerator] {
        self.accelerators.for_app(app_id)
    }

    /// The focused app's registered accelerators (empty with no focus).
    pub fn focused_accelerators(&self) -> &[Accelerator] {
        self.accelerators.focused_accelerators()
    }

    /// Resolve a chord spec against the focused app, system chords first.
    pub fn resolve_accelerator(&self, spec: &str) -> Dispatch {
        match Chord::parse(spec) {
            Some(chord) => self.accelerators.resolve(&chord),
            None => Dispatch::None,
        }
    }

    /// Replace the app window-state list from a JSON array. Returns false on a
    /// payload that is not an array; individual malformed rows are skipped so
    /// a single bad entry cannot fail the whole update.
    pub fn set_window_states(&mut self, payload: &str) -> bool {
        let Ok(value) = serde_json::from_str::<Value>(payload) else {
            return false;
        };
        let Some(array) = value.as_array() else {
            return false;
        };
        let mut apps = Vec::with_capacity(array.len());
        for row in array {
            let Some(object) = row.as_object() else {
                continue;
            };
            let app_id = object
                .get("appId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            if app_id.is_empty() {
                continue;
            }
            apps.push(WindowState {
                app_id,
                windows: object.get("windows").and_then(Value::as_u64).unwrap_or(1) as u32,
                minimized: object
                    .get("minimized")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
        let changed = self.visibility.set_apps(apps);
        if changed {
            self.revision += 1;
        }
        changed
    }

    /// Resolve `app_id`'s menu: the fixed application menu with live state,
    /// the exported top-level menus, and the tier.
    pub fn resolve(&self, app_id: &str) -> Value {
        let app_id = app_id.trim();
        let publisher = if app_id.is_empty() {
            None
        } else {
            self.publishers.get(app_id)
        };
        let app_name = publisher
            .map(|model| model.app_name.clone())
            .unwrap_or_else(|| default_app_name(app_id));
        let application_menu_items = fixed_application_menu(
            &app_name,
            publisher.map(|model| model.application_menu_items.as_slice()),
            &self.visibility,
        );
        let menus = publisher
            .map(|model| model.menus.clone())
            .unwrap_or_default();
        let tier = if publisher.is_some() {
            Tier::Native
        } else {
            Tier::None
        };
        // T-14.2b: the focus-scoped accelerator table the shell registers with
        // the compositor while this window is focused.
        let accelerators: Vec<Value> = self
            .accelerators
            .for_app(app_id)
            .iter()
            .map(Accelerator::to_json)
            .collect();
        json!({
            "appId": app_id,
            "appName": app_name,
            "tier": tier.as_str(),
            "applicationMenuItems": application_menu_items,
            "menus": menus,
            "accelerators": accelerators,
        })
    }

    /// Resolve the focused app's menu (the empty desktop is Files).
    pub fn resolve_focused(&self) -> Value {
        self.resolve(self.visibility.focused().unwrap_or_default())
    }

    /// The fixed application menu for `app_id` alone, ignoring any published
    /// model (the Tier-3 projection, useful for the bar's fallback).
    pub fn fixed_menu(&self, app_id: &str) -> Value {
        let app_name = self
            .publishers
            .get(app_id)
            .map(|model| model.app_name.clone())
            .unwrap_or_else(|| default_app_name(app_id));
        json!({
            "appId": app_id,
            "appName": app_name,
            "tier": TIER_NONE,
            "applicationMenuItems": synthesized_application_menu(
                &app_name,
                self.visibility.hide_verbs()
            ),
            "menus": [],
            "accelerators": [],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_fixture() -> &'static str {
        // The exact shape of `apps/settings/SettingsMenu.qml`'s publishedModel.
        r#"{
            "appName": "Settings",
            "applicationMenuItems": [
                { "label": "About Settings", "action": "about" },
                { "label": "Settings\u2026", "shortcut": "Super+,", "action": "settings" },
                { "type": "separator" },
                { "label": "Hide Settings", "shortcut": "Super+H", "action": "hide" },
                { "label": "Hide Others", "shortcut": "Super+Alt+H", "action": "hide-others" },
                { "label": "Show All", "action": "show-all" },
                { "type": "separator" },
                { "label": "Quit Settings", "shortcut": "Super+Q", "action": "quit" }
            ],
            "menus": [
                { "title": "File", "items": [ { "label": "Close Window", "action": "close" } ] },
                { "title": "Edit", "items": [ { "label": "Undo", "action": "edit.undo" } ] }
            ]
        }"#
    }

    fn states(focused: &str, rows: &[(&str, u32, bool)]) -> Visibility {
        let mut visibility = Visibility::from_parts(
            None,
            rows.iter()
                .map(|(app_id, windows, minimized)| WindowState {
                    app_id: (*app_id).to_owned(),
                    windows: *windows,
                    minimized: *minimized,
                })
                .collect(),
        );
        visibility.set_focused(focused);
        visibility
    }

    #[test]
    fn the_dbname_matches_policy() {
        assert_eq!(
            super::super::DBUS_NAME,
            df_ipc::dbus_name("MenuBroker", 1).to_string()
        );
    }

    #[test]
    fn parse_reads_the_published_shape() {
        let model = PublishedModel::parse(settings_fixture()).expect("parses");
        assert_eq!(model.app_name, "Settings");
        assert_eq!(model.application_menu_items.len(), 8);
        assert_eq!(model.menus.len(), 2);
        assert_eq!(model.menus[0]["title"], "File");
    }

    #[test]
    fn parse_tolerates_missing_fields_and_garbage() {
        let sparse = PublishedModel::parse(r#"{ "appName": "X" }"#).unwrap();
        assert!(sparse.application_menu_items.is_empty());
        assert!(sparse.menus.is_empty());
        assert!(PublishedModel::parse("not json").is_none());
        assert!(PublishedModel::parse(r#"{ "menus": [] }"#).is_none());
        assert!(PublishedModel::parse("[1, 2, 3]").is_none());
    }

    #[test]
    fn a_fixture_export_resolves_to_the_expected_menu_model() {
        let mut broker = Broker::new();
        assert!(broker.publish("org.dragonfruit.Settings", settings_fixture()));
        // Focus the app, whose single window is visible.
        broker.set_focused("org.dragonfruit.Settings");
        broker.set_window_states(
            r#"[ { "appId": "org.dragonfruit.Settings", "windows": 1, "minimized": false } ]"#,
        );

        let resolved = broker.resolve("org.dragonfruit.Settings");
        assert_eq!(resolved["appName"], "Settings");
        assert_eq!(resolved["tier"], TIER_NATIVE);
        let app_menu = resolved["applicationMenuItems"].as_array().unwrap();
        assert_eq!(app_menu.len(), 8);
        assert_eq!(app_menu[0]["label"], "About Settings");
        // Live: focused+visible => Hide and Hide Others on, Show All off.
        assert_eq!(app_menu[3]["enabled"], true);
        assert_eq!(app_menu[4]["enabled"], false);
        assert_eq!(app_menu[5]["enabled"], false);
        // The published own menus pass through.
        let menus = resolved["menus"].as_array().unwrap();
        assert_eq!(menus[0]["title"], "File");
        assert_eq!(menus[1]["title"], "Edit");
    }

    #[test]
    fn an_unpublished_app_resolves_to_the_synthesized_fixed_menu() {
        let mut broker = Broker::new();
        broker.set_focused("firefox");
        broker.set_window_states(r#"[ { "appId": "firefox", "windows": 1, "minimized": false } ]"#);

        let resolved = broker.resolve("firefox");
        assert_eq!(resolved["appName"], "Firefox");
        assert_eq!(resolved["tier"], TIER_NONE);
        let app_menu = resolved["applicationMenuItems"].as_array().unwrap();
        assert_eq!(app_menu.len(), 8);
        assert_eq!(app_menu[3]["label"], "Hide Firefox");
        assert_eq!(app_menu[6]["type"], "separator");
        assert_eq!(app_menu[7]["label"], "Quit Firefox");
    }

    #[test]
    fn hide_verbs_follow_the_live_window_state() {
        // Two running apps, both visible: Hide + Hide Others enabled.
        let mut broker = Broker::new();
        broker.publish("a", settings_fixture());
        broker.set_focused("a");
        broker.set_window_states(
            r#"[ { "appId": "a", "windows": 1, "minimized": false },
                 { "appId": "b", "windows": 2, "minimized": false } ]"#,
        );
        let menu = broker.resolve("a");
        let app_menu = menu["applicationMenuItems"].as_array().unwrap();
        assert_eq!(app_menu[3]["enabled"], true); // Hide a
        assert_eq!(app_menu[4]["enabled"], true); // Hide Others
        assert_eq!(app_menu[5]["enabled"], false); // Show All

        // Focus an app whose windows are all minimized: nothing to hide, but
        // Show All lights up because an app is hidden.
        broker.set_focused("b");
        broker.set_window_states(
            r#"[ { "appId": "a", "windows": 1, "minimized": false },
                 { "appId": "b", "windows": 2, "minimized": true } ]"#,
        );
        let menu = broker.resolve("b");
        let app_menu = menu["applicationMenuItems"].as_array().unwrap();
        assert_eq!(app_menu[3]["enabled"], false); // b is hidden itself
        assert_eq!(app_menu[4]["enabled"], true); // a still visible
        assert_eq!(app_menu[5]["enabled"], true); // b is hidden

        // Hide every app: only Show All stays meaningful.
        broker.set_window_states(
            r#"[ { "appId": "a", "windows": 1, "minimized": true },
                 { "appId": "b", "windows": 2, "minimized": true } ]"#,
        );
        let menu = broker.resolve("a");
        let app_menu = menu["applicationMenuItems"].as_array().unwrap();
        assert_eq!(app_menu[3]["enabled"], false);
        assert_eq!(app_menu[4]["enabled"], false);
        assert_eq!(app_menu[5]["enabled"], true);
    }

    #[test]
    fn the_synthesized_menu_matches_the_hide_verbs() {
        let menu = synthesized_application_menu(
            "Files",
            HideVerbs {
                hide: true,
                hide_others: false,
                show_all: true,
            },
        );
        assert_eq!(menu.len(), 8);
        assert_eq!(menu[3]["action"], ACTION_HIDE);
        assert_eq!(menu[3]["enabled"], true);
        assert_eq!(menu[4]["action"], ACTION_HIDE_OTHERS);
        assert_eq!(menu[4]["enabled"], false);
        assert_eq!(menu[5]["action"], ACTION_SHOW_ALL);
        assert_eq!(menu[5]["enabled"], true);
    }

    #[test]
    fn a_published_fixed_menu_keeps_its_labels_and_rewrites_only_liveness() {
        let published = vec![
            json!({ "label": "Hide Settings", "shortcut": "Super+H", "action": "hide" }),
            json!({ "label": "Hide Others", "action": "hide-others" }),
            json!({ "label": "Show All", "action": "show-all", "checked": true }),
        ];
        let visibility = states(
            "org.dragonfruit.Settings",
            &[("org.dragonfruit.Settings", 1, false)],
        );
        let menu = fixed_application_menu("Settings", Some(&published), &visibility);
        assert_eq!(menu[0]["label"], "Hide Settings");
        assert_eq!(menu[0]["shortcut"], "Super+H");
        assert_eq!(menu[0]["enabled"], true);
        assert_eq!(menu[1]["enabled"], false);
        assert_eq!(menu[2]["enabled"], false);
        // An unrelated field survives the rewrite.
        assert_eq!(menu[2]["checked"], true);
    }

    #[test]
    fn default_app_name_capitalizes_the_last_segment() {
        assert_eq!(default_app_name("org.dragonfruit.Settings"), "Settings");
        assert_eq!(default_app_name("firefox"), "Firefox");
        assert_eq!(default_app_name(""), "Files");
    }

    #[test]
    fn withdraw_and_revision() {
        let mut broker = Broker::new();
        broker.publish("a", settings_fixture());
        let after_publish = broker.revision();
        assert_eq!(broker.publisher_count(), 1);
        assert!(broker.withdraw("a"));
        assert!(broker.revision() > after_publish);
        assert!(!broker.withdraw("a"));
        assert_eq!(broker.publisher_count(), 0);
    }

    #[test]
    fn a_malformed_export_is_rejected_without_clobbering() {
        let mut broker = Broker::new();
        assert!(broker.publish("a", settings_fixture()));
        assert!(!broker.publish("a", "{not json"));
        // The good model is intact.
        assert_eq!(broker.resolve("a")["appName"], "Settings");
        assert!(!broker.publish("", settings_fixture()));
    }

    #[test]
    fn a_published_model_registers_its_accelerators_in_the_resolve_view() {
        let mut broker = Broker::new();
        broker.publish("org.dragonfruit.Settings", settings_fixture());
        broker.set_focused("org.dragonfruit.Settings");
        let resolved = broker.resolve("org.dragonfruit.Settings");
        let accelerators = resolved["accelerators"].as_array().unwrap();
        let pairs: Vec<(&str, &str)> = accelerators
            .iter()
            .map(|a| (a["action"].as_str().unwrap(), a["chord"].as_str().unwrap()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("settings", "Super+,"),
                ("hide", "Super+H"),
                ("hide-others", "Super+Alt+H"),
                ("quit", "Super+Q"),
            ]
        );
    }

    #[test]
    fn dispatch_is_focus_scoped_and_system_wins() {
        let mut broker = Broker::new();
        broker.publish("org.dragonfruit.Settings", settings_fixture());
        broker.publish("other", settings_fixture());
        broker.set_focused("org.dragonfruit.Settings");
        assert!(matches!(
            broker.resolve_accelerator("Super+Q"),
            Dispatch::Application { ref app_id, ref action }
                if app_id == "org.dragonfruit.Settings" && action == "quit"
        ));
        assert!(broker.set_system_accelerators(r#"["Super+Q"]"#));
        assert_eq!(broker.resolve_accelerator("Super+Q"), Dispatch::System);
        broker.set_focused("other");
        assert!(matches!(
            broker.resolve_accelerator("Super+Q"),
            Dispatch::System
        ));
        broker.set_system_accelerators("[]");
        assert!(matches!(
            broker.resolve_accelerator("Super+,"),
            Dispatch::Application { ref app_id, .. } if app_id == "other"
        ));
        assert_eq!(broker.resolve_accelerator("Super+Nope"), Dispatch::None);
    }

    #[test]
    fn withdrawing_an_app_clears_its_accelerators() {
        let mut broker = Broker::new();
        broker.publish("a", settings_fixture());
        broker.set_focused("a");
        assert!(!broker.focused_accelerators().is_empty());
        broker.withdraw("a");
        assert!(broker.focused_accelerators().is_empty());
    }

    #[test]
    fn window_states_skip_malformed_rows() {
        let mut broker = Broker::new();
        assert!(broker.set_window_states(
            r#"[ { "appId": "a", "windows": 1 }, { "windows": 3 }, "junk",
                 { "appId": "b", "windows": 1, "minimized": true } ]"#
        ));
        assert_eq!(broker.visibility().apps().len(), 2);
        assert!(!broker.set_window_states("{}"));
    }
}
