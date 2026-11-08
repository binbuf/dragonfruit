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

/// The shell-owned window menu actions (T230). The compositor performs them;
/// the broker only carries the rows and their live state.
pub const ACTION_WINDOW_MINIMIZE: &str = "window.minimize";
pub const ACTION_WINDOW_ACTIVATE_PREFIX: &str = "window.activate:";
pub const ACTION_WINDOW_BRING_ALL: &str = "window.bring-all-to-front";

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

/// One open window of an app, for the synthesized Window menu (T230).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowEntry {
    /// The compositor-stable window id the shell addresses.
    pub window_id: String,
    /// The window title (the app name when the client publishes none).
    pub title: String,
    /// Whether this is the focused window.
    pub focused: bool,
    /// Whether this window is minimized.
    pub minimized: bool,
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
    /// The app's windows, newest first (the shell's `windowList`).
    pub window_list: Vec<WindowEntry>,
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

/// A synthesized menu row. A disabled row carries the reason the shell can
/// show to the user; an enabled row never carries a stale reason.
fn entry(label: &str, action: &str, shortcut: Option<&str>, enabled: bool) -> Value {
    entry_with_reason(label, action, shortcut, enabled, None)
}

fn entry_with_reason(
    label: &str,
    action: &str,
    shortcut: Option<&str>,
    enabled: bool,
    disabled_reason: Option<&str>,
) -> Value {
    let mut object = serde_json::Map::new();
    object.insert("label".to_owned(), Value::String(label.to_owned()));
    if let Some(shortcut) = shortcut {
        object.insert("shortcut".to_owned(), Value::String(shortcut.to_owned()));
    }
    object.insert("action".to_owned(), Value::String(action.to_owned()));
    object.insert("enabled".to_owned(), Value::Bool(enabled));
    if !enabled {
        if let Some(reason) = disabled_reason {
            object.insert(
                "disabledReason".to_owned(),
                Value::String(reason.to_owned()),
            );
        }
    }
    Value::Object(object)
}

fn submenu_entry(label: &str, items: Vec<Value>) -> Value {
    json!({ "label": label, "type": "submenu", "submenu": items })
}

fn separator() -> Value {
    json!({ "type": "separator" })
}

/// The Services menu (T230). Linux has no macOS-style service registry, so the
/// standard row is always present but explicitly disabled with its reason;
/// an app that publishes its own fixed menu can replace it.
fn services_menu() -> Value {
    submenu_entry(
        "Services",
        vec![entry_with_reason(
            "No Services Available",
            "services.none",
            None,
            false,
            Some("No application has registered a service."),
        )],
    )
}

/// Whether a menu list already carries a top-level menu with `title`.
pub fn has_menu_title(menus: &[Value], title: &str) -> bool {
    menus.iter().any(|menu| {
        menu.get("title")
            .and_then(Value::as_str)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(title))
    })
}

/// The synthesized dynamic Window menu (T230): Minimize, the app's open
/// windows with the focused one checked, then Bring All to Front. Rows carry
/// `disabledReason` when there is nothing to act on.
pub fn synthesized_window_menu(app_name: &str, state: &WindowState) -> Vec<Value> {
    let has_visible = state.window_list.iter().any(|window| !window.minimized);
    let mut rows = vec![
        entry_with_reason(
            "Minimize",
            ACTION_WINDOW_MINIMIZE,
            Some("Super+M"),
            has_visible,
            Some("Every window is already minimized."),
        ),
        separator(),
    ];
    if state.window_list.is_empty() {
        rows.push(entry_with_reason(
            "No Windows",
            "window.none",
            None,
            false,
            Some("This app has no open windows."),
        ));
    } else {
        for window in &state.window_list {
            let label = if window.title.trim().is_empty() {
                app_name
            } else {
                window.title.as_str()
            };
            let action = format!("{ACTION_WINDOW_ACTIVATE_PREFIX}{}", window.window_id);
            let mut row = entry(label, &action, None, true);
            if let Some(object) = row.as_object_mut() {
                if window.focused {
                    object.insert("checked".to_owned(), Value::Bool(true));
                    object.insert("checkable".to_owned(), Value::Bool(true));
                }
            }
            rows.push(row);
        }
    }
    rows.push(separator());
    rows.push(entry_with_reason(
        "Bring All to Front",
        ACTION_WINDOW_BRING_ALL,
        None,
        !state.window_list.is_empty(),
        Some("This app has no open windows."),
    ));
    rows
}

/// The standard fixed application menu, synthesized from the app's name and
/// the live hide-verb state. This is the Tier-3 / no-publication case and the
/// fallback when a publisher carries no `applicationMenuItems`.
pub fn synthesized_application_menu(app_name: &str, verbs: HideVerbs) -> Vec<Value> {
    vec![
        entry(&format!("About {app_name}"), "about", None, true),
        entry("Settings\u{2026}", "settings", Some("Super+,"), true),
        services_menu(),
        separator(),
        entry_with_reason(
            &format!("Hide {app_name}"),
            ACTION_HIDE,
            Some("Super+H"),
            verbs.hide,
            Some("The app has no visible window."),
        ),
        entry_with_reason(
            "Hide Others",
            ACTION_HIDE_OTHERS,
            Some("Super+Alt+H"),
            verbs.hide_others,
            Some("No other app has a visible window."),
        ),
        entry_with_reason(
            "Show All",
            ACTION_SHOW_ALL,
            None,
            verbs.show_all,
            Some("No hidden app to show."),
        ),
        separator(),
        entry(&format!("Quit {app_name}"), "quit", Some("Super+Q"), true),
    ]
}

/// Insert the Services submenu into a published fixed menu when it carries
/// none. It goes after the app's Settings row (or after the first row when
/// there is no Settings row), matching the macOS app-menu order. Rows other
/// than the inserted one stay byte-for-byte as the app published them.
fn ensure_services_row(rows: Vec<Value>) -> Vec<Value> {
    if rows.iter().any(|row| {
        row.get("type").and_then(Value::as_str) == Some("submenu")
            && row
                .get("label")
                .and_then(Value::as_str)
                .is_some_and(|label| label.eq_ignore_ascii_case("Services"))
    }) {
        return rows;
    }
    let insert_at = rows
        .iter()
        .position(|row| row.get("action").and_then(Value::as_str) == Some("settings"))
        .map(|index| index + 1)
        .unwrap_or_else(|| rows.len().min(1));
    let mut rows = rows;
    rows.insert(insert_at, services_menu());
    rows
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
    let rows: Vec<Value> = published
        .iter()
        .map(|row| {
            if is_separator(row) {
                return row.clone();
            }
            match hide_flag(row, verbs) {
                Some(enabled) => {
                    let mut object = row.as_object().cloned().unwrap_or_default();
                    object.insert("enabled".to_owned(), Value::Bool(enabled));
                    if enabled {
                        object.remove("disabledReason");
                    } else if !object.contains_key("disabledReason") {
                        let reason = match string_field(row, "action").as_str() {
                            ACTION_HIDE => "The app has no visible window.",
                            ACTION_HIDE_OTHERS => "No other app has a visible window.",
                            ACTION_SHOW_ALL => "No hidden app to show.",
                            _ => "This action is unavailable.",
                        };
                        object.insert(
                            "disabledReason".to_owned(),
                            Value::String(reason.to_owned()),
                        );
                    }
                    Value::Object(object)
                }
                None => row.clone(),
            }
        })
        .collect();
    ensure_services_row(rows)
}

/// The broker's resolution state: publishers by app id, the current focus and
/// window state, and a monotonic revision.
#[derive(Debug, Clone)]
pub struct Broker {
    publishers: BTreeMap<String, PublishedModel>,
    /// The tier each publisher resolved through. Native by default; the
    /// DBusMenu bridge (T-14.4) publishes through [`Broker::publish_dbusmenu`].
    tiers: BTreeMap<String, Tier>,
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
            tiers: BTreeMap::new(),
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

    /// Register or replace an app's natively-published model (tier `native`).
    /// Returns false (and changes nothing) when the payload does not parse; a
    /// malformed exporter must never take the broker down or clobber a good
    /// model.
    pub fn publish(&mut self, app_id: &str, payload: &str) -> bool {
        self.publish_tiered(app_id, payload, Tier::Native)
    }

    /// Register or replace a model that arrived through the DBusMenu bridge
    /// (T-14.4). Same shape and tolerance as [`Broker::publish`], but the
    /// resolved tier is [`Tier::DbusMenu`].
    pub fn publish_dbusmenu(&mut self, app_id: &str, payload: &str) -> bool {
        self.publish_tiered(app_id, payload, Tier::DbusMenu)
    }

    /// Register or replace an app's published model with an explicit tier.
    /// Returns false (and changes nothing) when the payload does not parse.
    pub fn publish_tiered(&mut self, app_id: &str, payload: &str, tier: Tier) -> bool {
        if app_id.is_empty() {
            return false;
        }
        let Some(model) = PublishedModel::parse(payload) else {
            return false;
        };
        let changed =
            self.publishers.get(app_id) != Some(&model) || self.tiers.get(app_id) != Some(&tier);
        let table = accelerators::extract(&model);
        self.publishers.insert(app_id.to_owned(), model);
        self.tiers.insert(app_id.to_owned(), tier);
        self.accelerators.register(app_id, table);
        if changed {
            self.revision += 1;
        }
        changed
    }

    /// Remove an app's published model, its tier, and its accelerators.
    /// Returns whether one was present.
    pub fn withdraw(&mut self, app_id: &str) -> bool {
        let removed = self.publishers.remove(app_id).is_some();
        self.tiers.remove(app_id);
        self.accelerators.clear(app_id);
        if removed {
            self.revision += 1;
        }
        removed
    }

    /// The tier an app's model resolved through, if it published one.
    pub fn tier(&self, app_id: &str) -> Tier {
        self.tiers.get(app_id).copied().unwrap_or(Tier::None)
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

    /// The focused app's live accelerator list, including rows the broker
    /// synthesizes (the dynamic Window menu). Empty with no focus.
    pub fn resolved_accelerators_focused(&self) -> Vec<Accelerator> {
        self.visibility
            .focused()
            .map(|app_id| self.resolved_accelerators_for(app_id))
            .unwrap_or_default()
    }

    /// One app's live accelerator list from its resolved menu view.
    pub fn resolved_accelerators_for(&self, app_id: &str) -> Vec<Accelerator> {
        let resolved = self.resolve(app_id);
        let application_menu_items = resolved
            .get("applicationMenuItems")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let menus = resolved
            .get("menus")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        accelerators::extract_resolved(&application_menu_items, &menus)
    }

    /// Resolve a chord spec against the focused app, system chords first. The
    /// live resolved view wins over a stale published table, so a synthesized
    /// Window menu's `Super+M` dispatches exactly like a published row.
    pub fn resolve_accelerator(&self, spec: &str) -> Dispatch {
        let Some(chord) = Chord::parse(spec) else {
            return Dispatch::None;
        };
        match self.accelerators.resolve(&chord) {
            Dispatch::None => {}
            other => return other,
        }
        let Some(app_id) = self.visibility.focused() else {
            return Dispatch::None;
        };
        self.resolved_accelerators_for(app_id)
            .into_iter()
            .find(|accelerator| accelerator.chord == chord)
            .map(|accelerator| Dispatch::Application {
                app_id: app_id.to_owned(),
                action: accelerator.action,
            })
            .unwrap_or(Dispatch::None)
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
            let window_list = object
                .get("windowList")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(|row| {
                            let row = row.as_object()?;
                            let window_id = row
                                .get("windowId")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                .to_owned();
                            if window_id.is_empty() {
                                return None;
                            }
                            Some(WindowEntry {
                                window_id,
                                title: row
                                    .get("title")
                                    .and_then(Value::as_str)
                                    .unwrap_or_default()
                                    .to_owned(),
                                focused: row
                                    .get("focused")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                                minimized: row
                                    .get("minimized")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(false),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            apps.push(WindowState {
                app_id,
                windows: object.get("windows").and_then(Value::as_u64).unwrap_or(1) as u32,
                minimized: object
                    .get("minimized")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                window_list,
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
        let mut menus = publisher
            .map(|model| model.menus.clone())
            .unwrap_or_default();
        // T230: an app that exports no Window menu still gets the dynamic
        // window list from the live compositor projection. Apps that publish
        // their own Window menu keep it verbatim.
        if !app_id.is_empty() && !has_menu_title(&menus, "Window") {
            if let Some(state) = self.visibility.state_for(app_id) {
                menus.push(json!({
                    "title": "Window",
                    "items": synthesized_window_menu(&app_name, state),
                }));
            }
        }
        let tier = self.tier(app_id);
        // T-14.2b: the focus-scoped accelerator table the shell registers with
        // the compositor while this window is focused. It is derived from the
        // resolved view (not only the published payload) so a synthesized
        // Window menu's chord is included (T230).
        let accelerators: Vec<Value> =
            accelerators::extract_resolved(&application_menu_items, &menus)
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
                    window_list: Vec::new(),
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
        assert_eq!(app_menu.len(), 9);
        assert_eq!(app_menu[0]["label"], "About Settings");
        assert_eq!(app_menu[2]["label"], "Services");
        // Live: focused+visible => Hide and Hide Others on, Show All off.
        assert_eq!(app_menu[4]["enabled"], true);
        assert_eq!(app_menu[5]["enabled"], false);
        assert_eq!(app_menu[6]["enabled"], false);
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
        assert_eq!(app_menu.len(), 9);
        assert_eq!(app_menu[4]["label"], "Hide Firefox");
        assert_eq!(app_menu[7]["type"], "separator");
        assert_eq!(app_menu[8]["label"], "Quit Firefox");
        // No published model => a synthesized dynamic Window menu follows.
        let menus = resolved["menus"].as_array().unwrap();
        assert_eq!(menus.len(), 1);
        assert_eq!(menus[0]["title"], "Window");
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
        assert_eq!(app_menu[4]["enabled"], true); // Hide a
        assert_eq!(app_menu[5]["enabled"], true); // Hide Others
        assert_eq!(app_menu[6]["enabled"], false); // Show All

        // Focus an app whose windows are all minimized: nothing to hide, but
        // Show All lights up because an app is hidden.
        broker.set_focused("b");
        broker.set_window_states(
            r#"[ { "appId": "a", "windows": 1, "minimized": false },
                 { "appId": "b", "windows": 2, "minimized": true } ]"#,
        );
        let menu = broker.resolve("b");
        let app_menu = menu["applicationMenuItems"].as_array().unwrap();
        assert_eq!(app_menu[4]["enabled"], false); // b is hidden itself
        assert_eq!(app_menu[5]["enabled"], true); // a still visible
        assert_eq!(app_menu[6]["enabled"], true); // b is hidden

        // Hide every app: only Show All stays meaningful.
        broker.set_window_states(
            r#"[ { "appId": "a", "windows": 1, "minimized": true },
                 { "appId": "b", "windows": 2, "minimized": true } ]"#,
        );
        let menu = broker.resolve("a");
        let app_menu = menu["applicationMenuItems"].as_array().unwrap();
        assert_eq!(app_menu[4]["enabled"], false);
        assert_eq!(app_menu[5]["enabled"], false);
        assert_eq!(app_menu[6]["enabled"], true);
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
        assert_eq!(menu.len(), 9);
        assert_eq!(menu[2]["label"], "Services");
        assert_eq!(menu[4]["action"], ACTION_HIDE);
        assert_eq!(menu[4]["enabled"], true);
        assert_eq!(menu[5]["action"], ACTION_HIDE_OTHERS);
        assert_eq!(menu[5]["enabled"], false);
        assert_eq!(
            menu[5]["disabledReason"],
            "No other app has a visible window."
        );
        assert_eq!(menu[6]["action"], ACTION_SHOW_ALL);
        assert_eq!(menu[6]["enabled"], true);
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
        // The missing Services row is inserted after the first row; the
        // published rows keep their order and fields.
        assert_eq!(menu[0]["label"], "Hide Settings");
        assert_eq!(menu[0]["shortcut"], "Super+H");
        assert_eq!(menu[0]["enabled"], true);
        assert_eq!(menu[1]["label"], "Services");
        assert_eq!(menu[2]["label"], "Hide Others");
        assert_eq!(menu[2]["enabled"], false);
        assert_eq!(
            menu[2]["disabledReason"],
            "No other app has a visible window."
        );
        assert_eq!(menu[3]["enabled"], false);
        // An unrelated field survives the rewrite.
        assert_eq!(menu[3]["checked"], true);
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
    fn a_dbusmenu_publication_resolves_at_the_dbusmenu_tier() {
        let mut broker = Broker::new();
        let model = r#"{ "appName": "Konsole",
            "menus": [ { "title": "File", "items": [
                { "label": "New", "enabled": true, "action": "dbusmenu:1" },
                { "label": "Quit", "enabled": false, "action": "dbusmenu:2" } ] } ] }"#;
        assert!(broker.publish_dbusmenu("org.example.Konsole", model));
        assert_eq!(broker.tier("org.example.Konsole"), Tier::DbusMenu);
        assert_eq!(broker.tier("absent"), Tier::None);

        let resolved = broker.resolve("org.example.Konsole");
        assert_eq!(resolved["tier"], TIER_DBUSMENU);
        assert_eq!(resolved["appName"], "Konsole");
        assert_eq!(resolved["menus"][0]["title"], "File");
        assert_eq!(resolved["menus"][0]["items"][0]["enabled"], true);
        assert_eq!(resolved["menus"][0]["items"][1]["enabled"], false);
        // The fixed application menu is still synthesized.
        assert_eq!(
            resolved["applicationMenuItems"].as_array().unwrap().len(),
            9
        );

        // A native publication over the same id switches the tier back.
        assert!(broker.publish("org.example.Konsole", settings_fixture()));
        assert_eq!(broker.resolve("org.example.Konsole")["tier"], TIER_NATIVE);

        // Withdraw clears the tier too.
        assert!(broker.withdraw("org.example.Konsole"));
        assert_eq!(broker.tier("org.example.Konsole"), Tier::None);
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

    #[test]
    fn the_dynamic_window_menu_lists_windows_and_checks_the_focused_one() {
        let mut broker = Broker::new();
        broker.set_focused("firefox");
        broker.set_window_states(
            r#"[ { "appId": "firefox", "windows": 2, "minimized": false,
                  "windowList": [
                    { "windowId": "11", "title": "Docs", "focused": true, "minimized": false },
                    { "windowId": "12", "title": "News", "focused": false, "minimized": false }
                  ] } ]"#,
        );
        let resolved = broker.resolve("firefox");
        let menus = resolved["menus"].as_array().unwrap();
        let window_menu = menus
            .iter()
            .find(|menu| menu["title"] == "Window")
            .expect("a no-export app gets a Window menu");
        let rows = window_menu["items"].as_array().unwrap();
        assert_eq!(rows[0]["action"], ACTION_WINDOW_MINIMIZE);
        assert_eq!(rows[0]["enabled"], true);
        assert_eq!(rows[2]["label"], "Docs");
        assert_eq!(rows[2]["action"], "window.activate:11");
        assert_eq!(rows[2]["checked"], true);
        assert_eq!(rows[3]["label"], "News");
        assert_eq!(rows[3].get("checked"), None);
        assert_eq!(rows[5]["action"], ACTION_WINDOW_BRING_ALL);
        assert_eq!(rows[5]["enabled"], true);
    }

    #[test]
    fn a_no_export_app_dispatches_its_synthesized_accelerators() {
        let mut broker = Broker::new();
        broker.set_focused("firefox");
        broker.set_window_states(
            r#"[ { "appId": "firefox", "windows": 1, "minimized": false,
                  "windowList": [ { "windowId": "5", "title": "Docs", "focused": true } ] } ]"#,
        );
        // The fixed application menu and the synthesized Window menu both
        // carry focus-scoped chords, even though the app published nothing.
        assert!(matches!(
            broker.resolve_accelerator("Super+Q"),
            Dispatch::Application { ref action, .. } if action == "quit"
        ));
        assert!(matches!(
            broker.resolve_accelerator("Super+M"),
            Dispatch::Application { ref action, .. } if action == ACTION_WINDOW_MINIMIZE
        ));
        assert!(broker
            .resolved_accelerators_focused()
            .iter()
            .any(|accelerator| {
                accelerator.action == ACTION_WINDOW_MINIMIZE && accelerator.chord.key() == "M"
            }));
    }

    #[test]
    fn an_exported_window_menu_is_not_duplicated() {
        let mut broker = Broker::new();
        broker.publish("app", settings_fixture());
        broker.set_focused("app");
        broker.set_window_states(
            r#"[ { "appId": "app", "windows": 1, "minimized": false,
                  "windowList": [ { "windowId": "1", "title": "One", "focused": true } ] } ]"#,
        );
        let resolved = broker.resolve("app");
        let menus = resolved["menus"].as_array().unwrap();
        let count = menus
            .iter()
            .filter(|menu| menu["title"] == "Window")
            .count();
        assert_eq!(count, 1, "the app's own Window menu wins");
    }
}
