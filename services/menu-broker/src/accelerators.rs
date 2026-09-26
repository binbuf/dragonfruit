// SPDX-License-Identifier: MIT
//! Focus-scoped application accelerators (T-14.2b).
//!
//! The global menu is not just a picture: the rows carry shortcuts, and those
//! shortcuts must be **dispatched** to the focused application, not merely
//! displayed ([06-global-menu.md](../../docs/design/06-global-menu.md)).
//!
//! This module is the broker's side of that contract. It parses the
//! human-readable accelerator strings first-party apps publish (`"Super+H"`),
//! keeps one table per application, and resolves a chord against the **focused**
//! application only. System shortcuts always win: the broker never claims a
//! chord that the shell has declared reserved, so the compositor keeps firing
//! workspace switching, Mission Control, and the app switcher.
//!
//! No D-Bus, no `xkb`: both the chord parser and the table are pure data, so
//! the whole focus-scoping rule is unit-testable. The compositor is the final
//! authority on key matching (it owns the seat keymap); the broker's job is the
//! chord→action mapping and the focus scope.
//!
//! Conflict resolution follows [06-global-menu.md](../../docs/design/06-global-menu.md):
//! system shortcuts beat application accelerators, and among applications only
//! the focused window's menu is live.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::model::PublishedModel;

/// One logical modifier role. Mirrors `df_ipc::keymap::ModifierRole` (the fixed
/// Cmd→Super / Option→Alt mapping) but as a compact bitmask, since the broker
/// does not depend on the compositor's input stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods(u8);

const COMMAND: u8 = 1 << 0;
const OPTION: u8 = 1 << 1;
const CONTROL: u8 = 1 << 2;
const SHIFT: u8 = 1 << 3;

impl Mods {
    /// No modifiers.
    pub const EMPTY: Mods = Mods(0);
    /// The Cmd/Super role.
    pub const COMMAND: Mods = Mods(COMMAND);
    /// The Option/Alt role.
    pub const OPTION: Mods = Mods(OPTION);
    /// The Control role.
    pub const CONTROL: Mods = Mods(CONTROL);
    /// The Shift role.
    pub const SHIFT: Mods = Mods(SHIFT);

    /// Whether no role is held.
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Combine two role sets.
    pub fn union(self, other: Mods) -> Mods {
        Mods(self.0 | other.0)
    }

    /// Whether every role in `other` is held here.
    pub fn contains(self, other: Mods) -> bool {
        self.0 & other.0 == other.0
    }

    /// Parse one modifier token, tolerating the aliases apps use in the wild
    /// (`Cmd`/`Super`/`Logo`, `Option`/`Alt`, `Ctrl`/`Control`).
    fn parse_token(token: &str) -> Option<Mods> {
        match token.to_ascii_lowercase().as_str() {
            "cmd" | "super" | "logo" | "meta" => Some(Mods::COMMAND),
            "option" | "alt" => Some(Mods::OPTION),
            "ctrl" | "control" => Some(Mods::CONTROL),
            "shift" => Some(Mods::SHIFT),
            _ => None,
        }
    }

    /// The canonical role prefix, using the physical names the compositor's
    /// parser accepts (`Super+Shift`).
    pub fn label(self) -> String {
        let mut parts = Vec::new();
        for (bit, label) in [
            (COMMAND, "Super"),
            (OPTION, "Alt"),
            (CONTROL, "Control"),
            (SHIFT, "Shift"),
        ] {
            if self.0 & bit != 0 {
                parts.push(label);
            }
        }
        parts.join("+")
    }
}

/// A normalized chord: modifier roles plus the key token as published.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chord {
    mods: Mods,
    key: String,
}

impl Chord {
    /// The modifier roles.
    pub fn mods(&self) -> Mods {
        self.mods
    }

    /// The key token as published (case preserved: the compositor's keysym
    /// lookup is case-sensitive for letters).
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Parse a chord like `"Super+H"`, `"Ctrl+Super+F"`, or `"F11"`. Modifier
    /// tokens may appear in any order before the final key token; an unknown
    /// token in a non-final position, an empty key, or a bare modifier is
    /// rejected so a malformed exporter cannot register nonsense.
    pub fn parse(spec: &str) -> Option<Chord> {
        let spec = spec.trim();
        if spec.is_empty() {
            return None;
        }
        let mut mods = Mods::EMPTY;
        let mut tokens = spec.split('+').peekable();
        let mut key = None;
        while let Some(token) = tokens.next() {
            if tokens.peek().is_none() {
                if token.is_empty() || Mods::parse_token(token).is_some() {
                    return None;
                }
                key = Some(token.to_owned());
                break;
            }
            mods = mods.union(Mods::parse_token(token)?);
        }
        let key = key?;
        if key.is_empty() {
            return None;
        }
        Some(Chord { mods, key })
    }

    /// The canonical spelling the compositor's `parse_binding` accepts.
    pub fn canonical(&self) -> String {
        if self.mods.is_empty() {
            self.key.clone()
        } else {
            format!("{}+{}", self.mods.label(), self.key)
        }
    }
}

/// A published row's actionable accelerator: the action to route and the chord
/// that triggers it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Accelerator {
    /// The opaque action id the owning app handles (`edit.undo`).
    pub action: String,
    /// The parsed chord.
    pub chord: Chord,
}

impl Accelerator {
    /// A new accelerator from an action and a chord spec (the parsed form of
    /// the published `shortcut`).
    pub fn new(action: impl Into<String>, shortcut: &str) -> Option<Self> {
        let action = action.into();
        if action.is_empty() {
            return None;
        }
        Some(Accelerator {
            chord: Chord::parse(shortcut)?,
            action,
        })
    }

    /// The JSON view `{action, chord}` the shell consumes.
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "action": self.action,
            "chord": self.chord.canonical(),
        })
    }
}

fn push_row(row: &Value, out: &mut Vec<Accelerator>) {
    let Some(object) = row.as_object() else {
        return;
    };
    if object.get("type").and_then(Value::as_str) == Some("separator") {
        return;
    }
    if let (Some(shortcut), Some(action)) = (
        object.get("shortcut").and_then(Value::as_str),
        object.get("action").and_then(Value::as_str),
    ) {
        if let Some(accelerator) = Accelerator::new(action, shortcut) {
            out.push(accelerator);
        }
    }
    // A submenu carries accelerators too; recurse so a nested action is not
    // dropped.
    if let Some(submenu) = object.get("submenu").and_then(Value::as_array) {
        for child in submenu {
            push_row(child, out);
        }
    }
}

fn push_rows(rows: &[Value], out: &mut Vec<Accelerator>) {
    for row in rows {
        push_row(row, out);
    }
}

/// Every actionable accelerator a published model declares: the fixed
/// application menu first, then each own menu's rows, recursively through
/// submenus. Rows without both a `shortcut` and an `action` are skipped, so
/// labels, separators, and submenu headers never register a binding.
pub fn extract(model: &PublishedModel) -> Vec<Accelerator> {
    let mut out = Vec::new();
    push_rows(&model.application_menu_items, &mut out);
    for menu in &model.menus {
        if let Some(items) = menu.get("items").and_then(Value::as_array) {
            push_rows(items, &mut out);
        }
    }
    out
}

/// What a chord resolved to for the broker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dispatch {
    /// A reserved system chord: the compositor owns it; the broker stays out
    /// of the way.
    System,
    /// The focused app's action for the chord.
    Application {
        /// The owning application identity.
        app_id: String,
        /// The action to route to it.
        action: String,
    },
    /// No focused app owns the chord.
    None,
}

/// The broker's focus-scoped accelerator table.
///
/// Registrations are per app; only the focused app's entries match. Reserved
/// system chords win over every application. Replacing an app's table is a
/// single atomic step, so a re-publish can never leave a stale binding behind.
#[derive(Debug, Clone, Default)]
pub struct AcceleratorTable {
    apps: BTreeMap<String, Vec<Accelerator>>,
    focused: Option<String>,
    system: Vec<Chord>,
}

impl AcceleratorTable {
    /// An empty table with no reserved system chords.
    pub fn new() -> Self {
        Self::default()
    }

    /// The focused app id, if any.
    pub fn focused(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// The number of apps with a registered table.
    pub fn app_count(&self) -> usize {
        self.apps.len()
    }

    /// Replace the reserved system chords (the shell's declaration). Returns
    /// whether the set changed.
    pub fn set_system(&mut self, chords: Vec<Chord>) -> bool {
        if self.system == chords {
            return false;
        }
        self.system = chords;
        true
    }

    /// The reserved system chords.
    pub fn system(&self) -> &[Chord] {
        &self.system
    }

    /// Register or replace one app's accelerators. Returns false for an empty
    /// app id or when the table is unchanged.
    pub fn register(&mut self, app_id: &str, accelerators: Vec<Accelerator>) -> bool {
        if app_id.is_empty() {
            return false;
        }
        let changed = self.apps.get(app_id) != Some(&accelerators);
        self.apps.insert(app_id.to_owned(), accelerators);
        changed
    }

    /// Drop one app's accelerators. Returns whether any were present.
    pub fn clear(&mut self, app_id: &str) -> bool {
        self.apps.remove(app_id).is_some()
    }

    /// Set the focused app. Returns whether it changed.
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

    /// One app's accelerators, in publication order.
    pub fn for_app(&self, app_id: &str) -> &[Accelerator] {
        self.apps.get(app_id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// The focused app's accelerators (empty with no focus).
    pub fn focused_accelerators(&self) -> &[Accelerator] {
        self.focused
            .as_deref()
            .map(|app| self.for_app(app))
            .unwrap_or(&[])
    }

    /// Resolve a chord: a reserved system chord wins, then the focused app's
    /// table, else nothing.
    pub fn resolve(&self, chord: &Chord) -> Dispatch {
        if self.system.contains(chord) {
            return Dispatch::System;
        }
        let Some(app_id) = self.focused.as_deref() else {
            return Dispatch::None;
        };
        self.for_app(app_id)
            .iter()
            .find(|accelerator| &accelerator.chord == chord)
            .map(|accelerator| Dispatch::Application {
                app_id: app_id.to_owned(),
                action: accelerator.action.clone(),
            })
            .unwrap_or(Dispatch::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(spec: &str) -> Chord {
        Chord::parse(spec).unwrap_or_else(|| panic!("{spec} parses"))
    }

    fn accel(action: &str, shortcut: &str) -> Accelerator {
        Accelerator::new(action, shortcut).unwrap()
    }

    #[test]
    fn chords_parse_the_common_spellings() {
        assert_eq!(chord("Super+H").canonical(), "Super+H");
        assert_eq!(chord("cmd+h").canonical(), "Super+h");
        assert_eq!(chord("Ctrl+Super+F").canonical(), "Super+Control+F");
        assert_eq!(chord("Option+Shift+Z").canonical(), "Alt+Shift+Z");
        assert_eq!(chord("F11").canonical(), "F11");
        assert_eq!(chord("Super+,").canonical(), "Super+,");
    }

    #[test]
    fn malformed_chords_are_rejected() {
        assert!(Chord::parse("").is_none());
        assert!(Chord::parse("Super").is_none());
        assert!(Chord::parse("Super+Shift").is_none());
        assert!(Chord::parse("Hyper+X").is_none());
        assert!(Chord::parse("Super+").is_none());
    }

    #[test]
    fn mods_union_and_contains() {
        let cmd_shift = Mods::COMMAND.union(Mods::SHIFT);
        assert!(cmd_shift.contains(Mods::COMMAND));
        assert!(cmd_shift.contains(Mods::SHIFT));
        assert!(!cmd_shift.contains(Mods::CONTROL));
        assert!(Mods::EMPTY.is_empty());
    }

    #[test]
    fn extraction_walks_fixed_and_own_menus_through_submenus() {
        let model = PublishedModel::parse(
            r#"{
                "appName": "Settings",
                "applicationMenuItems": [
                    { "label": "Quit Settings", "shortcut": "Super+Q", "action": "quit" },
                    { "type": "separator" },
                    { "label": "Settings", "shortcut": "Super+,", "action": "settings" }
                ],
                "menus": [
                    { "title": "Edit", "items": [
                        { "label": "Undo", "shortcut": "Super+Z", "action": "edit.undo" },
                        { "label": "Redo", "shortcut": "Super+Shift+Z", "action": "edit.redo" }
                    ] },
                    { "title": "View", "items": [
                        { "label": "Sort", "type": "submenu", "submenu": [
                            { "label": "Name", "shortcut": "Ctrl+N", "action": "sort.name" }
                        ] }
                    ] }
                ]
            }"#,
        )
        .unwrap();
        let accelerators = extract(&model);
        let pairs: Vec<(String, String)> = accelerators
            .iter()
            .map(|a| (a.action.clone(), a.chord.canonical()))
            .collect();
        assert_eq!(
            pairs,
            [
                ("quit".to_owned(), "Super+Q".to_owned()),
                ("settings".to_owned(), "Super+,".to_owned()),
                ("edit.undo".to_owned(), "Super+Z".to_owned()),
                ("edit.redo".to_owned(), "Super+Shift+Z".to_owned()),
                ("sort.name".to_owned(), "Control+N".to_owned()),
            ]
        );
    }

    #[test]
    fn rows_without_both_shortcut_and_action_do_not_register() {
        let model = PublishedModel::parse(
            r#"{ "appName": "X",
                "menus": [ { "title": "File", "items": [
                    { "label": "Displayed only", "shortcut": "Super+D" },
                    { "label": "Action only", "action": "do.thing" },
                    { "label": "Both", "shortcut": "Super+B", "action": "both" }
                ] } ] }"#,
        )
        .unwrap();
        let accelerators = extract(&model);
        assert_eq!(accelerators.len(), 1);
        assert_eq!(accelerators[0].action, "both");
    }

    #[test]
    fn only_the_focused_app_resolves() {
        let mut table = AcceleratorTable::new();
        table.register("one", vec![accel("one.quit", "Super+Q")]);
        table.register("two", vec![accel("two.quit", "Super+Q")]);
        table.set_focused("one");
        assert_eq!(
            table.resolve(&chord("Super+Q")),
            Dispatch::Application {
                app_id: "one".into(),
                action: "one.quit".into(),
            }
        );
        table.set_focused("two");
        assert_eq!(
            table.resolve(&chord("Super+Q")),
            Dispatch::Application {
                app_id: "two".into(),
                action: "two.quit".into(),
            }
        );
        table.set_focused("");
        assert_eq!(table.resolve(&chord("Super+Q")), Dispatch::None);
    }

    #[test]
    fn system_chords_beat_application_accelerators() {
        let mut table = AcceleratorTable::new();
        table.register("app", vec![accel("app.next", "Ctrl+Right")]);
        table.set_focused("app");
        assert!(table.set_system(vec![chord("Ctrl+Right")]));
        assert_eq!(table.resolve(&chord("Ctrl+Right")), Dispatch::System);
        // A different chord still reaches the app.
        assert_eq!(table.resolve(&chord("Ctrl+Left")), Dispatch::None);
    }

    #[test]
    fn re_registration_replaces_the_table() {
        let mut table = AcceleratorTable::new();
        assert!(table.register("app", vec![accel("a", "Super+A")]));
        assert!(!table.register("app", vec![accel("a", "Super+A")]));
        assert!(table.register("app", vec![accel("b", "Super+B")]));
        assert_eq!(table.for_app("app"), &[accel("b", "Super+B")]);
        assert!(table.clear("app"));
        assert!(table.for_app("app").is_empty());
        assert!(!table.clear("app"));
    }

    #[test]
    fn the_json_view_carries_action_and_canonical_chord() {
        let value = accel("edit.undo", "cmd+z").to_json();
        assert_eq!(value["action"], "edit.undo");
        assert_eq!(value["chord"], "Super+z");
    }
}
