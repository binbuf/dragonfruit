// SPDX-License-Identifier: MIT
//! StatusNotifier/AppIndicator tray support (T-14.3).
//!
//! The de-facto Linux tray standard is StatusNotifierItem (SNI) plus the
//! `org.kde.StatusNotifierWatcher` registry. app-index owns the watcher: a
//! third-party app calls `RegisterStatusNotifierItem`, and the shell asks
//! app-index for the live items and their DBusMenu-backed menus.
//!
//! This module is the pure half, so it can be unit-tested without a bus:
//!
//! * [`Registration`] parses and normalizes an SNI registration string
//!   (a bus name, or a `sender:/path` pair) into a proxy destination + object
//!   path, per the StatusNotifierWatcher contract.
//! * [`TrayRegistry`] is the registered-item table and its removal rules.
//! * [`MenuNode`] is a tolerance-first projection of a `com.canonical.dbusmenu`
//!   `GetLayout` tree into the design-system menu entry shape the shell's
//!   `ContextMenu` renders. A malformed row is skipped, never fatal.
//!
//! The D-Bus calls that fill these types live in [`crate::dbus`].

use std::collections::{HashMap, HashSet};

use zbus::zvariant::OwnedValue;

/// The item object path used when a registration names a bus name rather than
/// an object path (StatusNotifierWatcher contract).
pub const ITEM_PATH: &str = "/StatusNotifierItem";

/// One registered tray item: the stable registered name the shell keys by, the
/// proxy destination and object path, and the caller's unique bus name (so the
/// watcher can drop the item when its owner disconnects).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    /// The string the watcher registers and reports. For a bus-name
    /// registration this is the name itself; for a path registration it is
    /// `sender:/path`.
    pub name: String,
    /// The bus name to send item method calls to (a unique name for a path
    /// registration, else the claimed name).
    pub destination: String,
    /// The item's object path.
    pub path: String,
    /// The unique bus name that called `RegisterStatusNotifierItem`.
    pub sender: String,
}

impl Registration {
    /// Normalize a `RegisterStatusNotifierItem(service)` call. `sender` is the
    /// caller's unique bus name. Returns `None` for an empty or malformed
    /// service. A service that starts with `/` is an object path on the
    /// caller; anything else is a bus name whose item lives at [`ITEM_PATH`].
    pub fn parse(service: &str, sender: &str) -> Option<Self> {
        let service = service.trim();
        let sender = sender.trim();
        if sender.is_empty() {
            return None;
        }
        if service.starts_with('/') {
            let path = normalize_path(service)?;
            Some(Registration {
                name: format!("{sender}:{path}"),
                destination: sender.to_owned(),
                path,
                sender: sender.to_owned(),
            })
        } else if !service.is_empty() && is_valid_bus_name(service) {
            Some(Registration {
                name: service.to_owned(),
                destination: service.to_owned(),
                path: ITEM_PATH.to_owned(),
                sender: sender.to_owned(),
            })
        } else {
            None
        }
    }
}

/// A minimal object-path sanity check (the bus would reject worse).
fn normalize_path(path: &str) -> Option<String> {
    if path.len() > 1 && path.ends_with('/') {
        return Some(path.trim_end_matches('/').to_owned());
    }
    Some(path.to_owned())
}

/// A permissive bus-name check: at least one dot and no spaces/slashes.
fn is_valid_bus_name(name: &str) -> bool {
    name.contains('.')
        && !name.contains(' ')
        && !name.contains('/')
        && name.split('.').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}

/// The registered tray items, in registration order.
#[derive(Debug, Default, Clone)]
pub struct TrayRegistry {
    items: Vec<Registration>,
}

impl TrayRegistry {
    /// A new empty registry.
    pub fn new() -> Self {
        TrayRegistry { items: Vec::new() }
    }

    /// Register (or replace) an item. Returns `true` when the set changed.
    pub fn register(&mut self, registration: Registration) -> bool {
        if let Some(existing) = self.items.iter_mut().find(|i| i.name == registration.name) {
            if *existing == registration {
                return false;
            }
            *existing = registration;
            return true;
        }
        self.items.push(registration);
        true
    }

    /// Drop one registered name. Returns `true` when it was present.
    pub fn remove(&mut self, name: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|i| i.name != name);
        self.items.len() != before
    }

    /// Drop every item owned by `sender` (used when the caller disconnects).
    /// Returns the names that were removed.
    pub fn remove_sender(&mut self, sender: &str) -> Vec<String> {
        let mut removed = Vec::new();
        self.items.retain(|i| {
            if i.sender == sender {
                removed.push(i.name.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Drop every item whose caller or destination is `owner` (a unique or
    /// well-known bus name that just lost its owner). Returns removed names.
    pub fn remove_owner(&mut self, owner: &str) -> Vec<String> {
        let mut removed = Vec::new();
        self.items.retain(|i| {
            if i.sender == owner || i.destination == owner {
                removed.push(i.name.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// One registration by its stable name.
    pub fn get(&self, name: &str) -> Option<&Registration> {
        self.items.iter().find(|i| i.name == name)
    }

    /// The registered names, in registration order.
    pub fn names(&self) -> Vec<String> {
        self.items.iter().map(|i| i.name.clone()).collect()
    }

    /// The registrations, in registration order.
    pub fn items(&self) -> &[Registration] {
        &self.items
    }

    /// How many items are registered.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether no item is registered.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// One item's normalized shell view plus its last projected menu.
///
/// The service owns the only copy of this data: `TrayItems` and `TrayMenu`
/// answer from it, and the background refresher replaces entries in place. A
/// slow exporter therefore delays only its own refresh, never a snapshot for
/// every other item.
#[derive(Debug, Clone, PartialEq)]
pub struct TraySnapshot {
    /// The unique bus name that registered the item when the snapshot was
    /// taken. An owner change invalidates the snapshot (and its menu).
    pub owner: String,
    /// The item's flat JSON view (`tray_item_json`).
    pub item: serde_json::Value,
    /// The item's DBusMenu object path, empty when it exports none.
    pub menu_path: String,
    /// The projected DBusMenu rows, absent until the first menu read.
    pub menu: Option<Vec<serde_json::Value>>,
    /// The cache-wide generation the current `menu` was stored at.
    pub menu_generation: u64,
}

/// The service-side normalized snapshot table. It keeps registration order so
/// the shell's bar order stays stable across refreshes.
#[derive(Debug, Default)]
pub struct TrayCache {
    order: Vec<String>,
    entries: HashMap<String, TraySnapshot>,
    generation: u64,
}

impl TrayCache {
    /// An empty cache.
    pub fn new() -> Self {
        TrayCache::default()
    }

    /// Store a refreshed item view. Returns whether anything the shell renders
    /// changed. An owner change drops the cached menu: it belongs to the old
    /// exporter and must never be served to the new one.
    pub fn set_item(
        &mut self,
        name: &str,
        owner: &str,
        item: serde_json::Value,
        menu_path: &str,
    ) -> bool {
        let owner_changed = self
            .entries
            .get(name)
            .is_some_and(|entry| entry.owner != owner);
        if owner_changed {
            self.entries.remove(name);
        }
        let menu_path = menu_path.to_owned();
        if let Some(entry) = self.entries.get_mut(name) {
            let item_changed = entry.item != item || entry.owner != owner;
            let menu_changed = entry.menu_path != menu_path;
            if menu_changed {
                entry.menu = None;
                entry.menu_path = menu_path;
            }
            entry.item = item;
            entry.owner = owner.to_owned();
            return item_changed || menu_changed;
        }
        if !self.order.iter().any(|entry| entry == name) {
            self.order.push(name.to_owned());
        }
        self.entries.insert(
            name.to_owned(),
            TraySnapshot {
                owner: owner.to_owned(),
                item,
                menu_path,
                menu: None,
                menu_generation: 0,
            },
        );
        true
    }

    /// Store a refreshed menu. Returns the new generation when the rows
    /// actually changed, `None` when the snapshot was already current or the
    /// exporter owner no longer matches.
    pub fn set_menu(
        &mut self,
        name: &str,
        owner: &str,
        rows: Vec<serde_json::Value>,
    ) -> Option<u64> {
        let entry = self.entries.get_mut(name)?;
        if entry.owner != owner {
            return None;
        }
        if entry.menu.as_ref() == Some(&rows) {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        entry.menu = Some(rows);
        entry.menu_generation = self.generation;
        Some(self.generation)
    }

    /// Drop a cached menu without removing the item (the exporter removed its
    /// `Menu` property).
    pub fn clear_menu(&mut self, name: &str) {
        if let Some(entry) = self.entries.get_mut(name) {
            entry.menu = None;
            entry.menu_path.clear();
        }
    }

    /// One item's snapshot.
    pub fn get(&self, name: &str) -> Option<&TraySnapshot> {
        self.entries.get(name)
    }

    /// One item's current menu path.
    pub fn menu_path(&self, name: &str) -> Option<&str> {
        self.entries.get(name).map(|entry| entry.menu_path.as_str())
    }

    /// One item's cached menu rows, owner, and generation.
    pub fn menu(&self, name: &str) -> Option<(String, u64, Vec<serde_json::Value>)> {
        let entry = self.entries.get(name)?;
        let rows = entry.menu.clone()?;
        Some((entry.owner.clone(), entry.menu_generation, rows))
    }

    /// The cache-wide menu generation. It advances whenever any item's menu
    /// rows change.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Remove one item.
    pub fn remove(&mut self, name: &str) -> bool {
        self.order.retain(|entry| entry != name);
        self.entries.remove(name).is_some()
    }

    /// Keep only the given registrations, preserving their order. Returns
    /// whether any entry was dropped.
    pub fn retain(&mut self, names: &[String]) -> bool {
        let live: HashSet<&str> = names.iter().map(String::as_str).collect();
        let before = self.entries.len();
        self.entries.retain(|name, _| live.contains(name.as_str()));
        self.order.retain(|name| live.contains(name.as_str()));
        self.entries.len() != before
    }

    /// The item views in registration order. Items without a snapshot yet are
    /// omitted, so a partially refreshed bar still renders what it knows.
    pub fn items_json(&self) -> String {
        let values: Vec<&serde_json::Value> = self
            .order
            .iter()
            .filter_map(|name| self.entries.get(name))
            .map(|entry| &entry.item)
            .collect();
        serde_json::Value::Array(values.into_iter().cloned().collect()).to_string()
    }

    /// How many snapshots are cached.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The DBusMenu node kind after the design-system projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuNodeType {
    /// A plain clickable row.
    Item,
    /// A horizontal rule.
    Separator,
    /// A row that opens a nested panel.
    Submenu,
}

impl MenuNodeType {
    /// The wire label used in JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            MenuNodeType::Item => "item",
            MenuNodeType::Separator => "separator",
            MenuNodeType::Submenu => "submenu",
        }
    }
}

/// One projected menu row. `id` is the DBusMenu item id, sent back verbatim
/// when the row is clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuNode {
    pub id: i32,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub node_type: MenuNodeType,
    /// `None` when the row is not a toggle.
    pub checked: Option<bool>,
    pub shortcut: String,
    pub children: Vec<MenuNode>,
}

impl MenuNode {
    /// Project the node (and its visible children) to a design-system JSON
    /// value. Rows that are not visible are omitted by the parent.
    pub fn to_json(&self) -> serde_json::Value {
        let mut object = serde_json::Map::new();
        object.insert("id".to_owned(), serde_json::json!(self.id));
        object.insert(
            "type".to_owned(),
            serde_json::Value::String(self.node_type.as_str().to_owned()),
        );
        object.insert(
            "label".to_owned(),
            serde_json::Value::String(self.label.clone()),
        );
        object.insert("enabled".to_owned(), serde_json::Value::Bool(self.enabled));
        if let Some(checked) = self.checked {
            object.insert("checked".to_owned(), serde_json::Value::Bool(checked));
            object.insert("checkable".to_owned(), serde_json::Value::Bool(true));
        }
        if !self.shortcut.is_empty() {
            object.insert(
                "shortcut".to_owned(),
                serde_json::Value::String(self.shortcut.clone()),
            );
        }
        if !self.children.is_empty() {
            object.insert(
                "submenu".to_owned(),
                serde_json::Value::Array(self.children.iter().map(MenuNode::to_json).collect()),
            );
        }
        serde_json::Value::Object(object)
    }
}

/// A layout tree as read from `com.canonical.dbusmenu.GetLayout`:
/// `(id, properties, children)`.
type RawLayout = (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>);

/// Parse one `GetLayout` reply layout value into a [`MenuNode`] tree. Returns
/// `None` when the value is not a layout tuple; malformed children are
/// skipped rather than failing the whole menu.
pub fn parse_layout(value: &OwnedValue) -> Option<MenuNode> {
    let layout: RawLayout = value.try_clone().ok()?.try_into().ok()?;
    Some(node_from_raw(layout))
}

fn node_from_raw(raw: RawLayout) -> MenuNode {
    let (id, props, children) = raw;
    let node_type = if prop_string(&props, "type").as_deref() == Some("separator") {
        MenuNodeType::Separator
    } else if !children.is_empty()
        && prop_string(&props, "children-display").as_deref() == Some("submenu")
    {
        MenuNodeType::Submenu
    } else {
        MenuNodeType::Item
    };
    let chec_ked = match prop_string(&props, "toggle-type").as_deref() {
        Some("checkmark") | Some("radio") => {
            Some(prop_i32(&props, "toggle-state").unwrap_or(0) != 0)
        }
        _ => None,
    };
    let child_nodes: Vec<MenuNode> = children
        .iter()
        .filter_map(|child| {
            child
                .try_clone()
                .ok()
                .and_then(|value| value.try_into().ok())
        })
        .map(node_from_raw)
        .filter(|node| node.visible)
        .collect();
    MenuNode {
        id,
        label: strip_mnemonics(&prop_string(&props, "label").unwrap_or_default()),
        enabled: prop_bool(&props, "enabled").unwrap_or(true),
        visible: prop_bool(&props, "visible").unwrap_or(true),
        node_type,
        checked: chec_ked,
        shortcut: shortcut_string(&props),
        children: child_nodes,
    }
}

/// The display label: DBusMenu marks a mnemonic with a single `_` and escapes
/// a literal one as `__`. Drop the mnemonic markers for the shell.
fn strip_mnemonics(label: &str) -> String {
    let mut out = String::with_capacity(label.len());
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '_' {
            if chars.peek() == Some(&'_') {
                out.push('_');
                chars.next();
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// The readable shortcut text from DBusMenu's `shortcut` property
/// (`aas`, a list of key sequences). Only the first sequence is rendered; the
/// separator characters are stripped to a `+`-joined form.
fn shortcut_string(props: &HashMap<String, OwnedValue>) -> String {
    let Some(value) = props
        .get("shortcut")
        .and_then(|value| value.try_clone().ok())
    else {
        return String::new();
    };
    let Ok(sequences) = Vec::<Vec<String>>::try_from(value) else {
        return String::new();
    };
    let Some(first) = sequences.first() else {
        return String::new();
    };
    first
        .iter()
        .map(|part| part.trim_start_matches('<').trim_end_matches('>').trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

fn prop_string(props: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    props
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| String::try_from(value).ok())
}

fn prop_bool(props: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    props
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| bool::try_from(value).ok())
}

fn prop_i32(props: &HashMap<String, OwnedValue>, key: &str) -> Option<i32> {
    props
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| i32::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zbus::zvariant::Value;

    fn item(sender: &str) -> Registration {
        Registration::parse("org.kde.StatusNotifierItem-1-1", sender).unwrap()
    }

    #[test]
    fn a_bus_name_registration_uses_the_standard_item_path() {
        let registration = Registration::parse("org.kde.StatusNotifierItem-1-1", ":1.42").unwrap();
        assert_eq!(registration.name, "org.kde.StatusNotifierItem-1-1");
        assert_eq!(registration.destination, "org.kde.StatusNotifierItem-1-1");
        assert_eq!(registration.path, ITEM_PATH);
        assert_eq!(registration.sender, ":1.42");
    }

    #[test]
    fn a_path_registration_is_namespaced_by_the_sender() {
        let registration = Registration::parse("/MenuBar", ":1.7").unwrap();
        assert_eq!(registration.name, ":1.7:/MenuBar");
        assert_eq!(registration.destination, ":1.7");
        assert_eq!(registration.path, "/MenuBar");
    }

    #[test]
    fn malformed_registrations_are_rejected() {
        assert!(Registration::parse("", ":1.1").is_none());
        assert!(Registration::parse("no-dot", ":1.1").is_none());
        assert!(Registration::parse("org.kde.Item", "").is_none());
    }

    #[test]
    fn the_registry_replaces_and_removes_by_name() {
        let mut registry = TrayRegistry::new();
        assert!(registry.register(item(":1.1")));
        assert!(!registry.register(item(":1.1")));
        assert_eq!(registry.names(), vec!["org.kde.StatusNotifierItem-1-1"]);

        let other = Registration::parse("/MenuBar", ":1.2").unwrap();
        assert!(registry.register(other));
        assert_eq!(registry.len(), 2);

        let removed = registry.remove_sender(":1.1");
        assert_eq!(removed, vec!["org.kde.StatusNotifierItem-1-1"]);
        assert_eq!(registry.names(), vec![":1.2:/MenuBar"]);
        assert!(registry.remove(":1.2:/MenuBar"));
        assert!(registry.is_empty());
    }

    fn layout(id: i32, props: &[(&str, Value<'static>)], children: Vec<OwnedValue>) -> OwnedValue {
        let map: HashMap<String, OwnedValue> = props
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    OwnedValue::try_from(v.try_clone().unwrap()).unwrap(),
                )
            })
            .collect();
        OwnedValue::try_from(Value::from((id, map, children))).unwrap()
    }

    #[test]
    fn a_layout_tree_projects_to_the_design_system_shape() {
        let child = layout(
            7,
            &[
                ("label", Value::from("Show Window")),
                ("enabled", Value::from(true)),
            ],
            vec![],
        );
        let separator = layout(8, &[("type", Value::from("separator"))], vec![]);
        let submenu = layout(
            3,
            &[
                ("label", Value::from("_Tools")),
                ("children-display", Value::from("submenu")),
            ],
            vec![child],
        );
        let root = layout(0, &[], vec![submenu, separator]);

        let node = parse_layout(&root).unwrap();
        assert_eq!(node.children.len(), 2);
        let tools = &node.children[0];
        assert_eq!(tools.label, "Tools");
        assert_eq!(tools.node_type, MenuNodeType::Submenu);
        assert_eq!(tools.children[0].label, "Show Window");
        assert_eq!(tools.children[0].node_type, MenuNodeType::Item);
        assert_eq!(node.children[1].node_type, MenuNodeType::Separator);

        let json = node.to_json();
        assert_eq!(json["children"], serde_json::Value::Null);
        assert_eq!(json["submenu"][0]["label"], "Tools");
        assert_eq!(json["submenu"][0]["type"], "submenu");
    }

    #[test]
    fn toggle_rows_carry_checked_state_and_mnemonics_are_stripped() {
        let checked = layout(
            1,
            &[
                ("label", Value::from("_Enabled")),
                ("toggle-type", Value::from("checkmark")),
                ("toggle-state", Value::from(1i32)),
            ],
            vec![],
        );
        let root = layout(0, &[], vec![checked]);
        let node = parse_layout(&root).unwrap();
        assert_eq!(node.children[0].label, "Enabled");
        assert_eq!(node.children[0].checked, Some(true));
        let json = node.to_json();
        assert_eq!(json["submenu"][0]["checked"], true);
        assert_eq!(json["submenu"][0]["checkable"], true);
    }

    #[test]
    fn a_malformed_layout_is_none_not_a_panic() {
        assert!(
            parse_layout(&OwnedValue::try_from(Value::from("not a layout")).unwrap()).is_none()
        );
    }

    fn view(name: &str, title: &str) -> serde_json::Value {
        serde_json::json!({ "name": name, "title": title })
    }

    #[test]
    fn the_cache_replaces_items_in_registration_order() {
        let mut cache = TrayCache::new();
        assert!(cache.set_item("one", "org.example.One", view("one", "One"), "/Menu"));
        assert!(cache.set_item("two", "org.example.Two", view("two", "Two"), ""));
        assert!(!cache.set_item("one", "org.example.One", view("one", "One"), "/Menu"));

        let items: serde_json::Value = serde_json::from_str(&cache.items_json()).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 2);
        assert_eq!(items[0]["name"], "one");
        assert_eq!(items[1]["name"], "two");
    }

    #[test]
    fn an_owner_change_drops_the_cached_menu() {
        let mut cache = TrayCache::new();
        cache.set_item("one", "org.example.One", view("one", "One"), "/Menu");
        assert!(cache
            .set_menu("one", "org.example.One", vec![view("row", "Row")])
            .is_some());
        assert!(cache.menu("one").is_some());

        // A new exporter owner must never inherit the old menu.
        assert!(cache.set_item("one", ":1.99", view("one", "One"), "/Menu"));
        assert!(cache.menu("one").is_none());
        let items: serde_json::Value = serde_json::from_str(&cache.items_json()).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 1);
    }

    #[test]
    fn a_menu_refresh_only_bumps_the_generation_on_a_real_change() {
        let mut cache = TrayCache::new();
        cache.set_item("one", "org.example.One", view("one", "One"), "/Menu");
        let first = cache
            .set_menu("one", "org.example.One", vec![view("row", "Row")])
            .unwrap();
        assert!(first > 0);
        assert_eq!(
            cache.set_menu("one", "org.example.One", vec![view("row", "Row")]),
            None
        );
        let second = cache
            .set_menu("one", "org.example.One", vec![view("row", "Changed")])
            .unwrap();
        assert!(second > first);
        // A stale owner cannot replace the rows.
        assert_eq!(
            cache.set_menu("one", ":1.99", vec![view("row", "Other")]),
            None
        );
    }

    #[test]
    fn retaining_the_registry_drops_snapshots_for_departed_items() {
        let mut cache = TrayCache::new();
        cache.set_item("one", "org.example.One", view("one", "One"), "");
        cache.set_item("two", "org.example.Two", view("two", "Two"), "");
        assert!(cache.retain(&["two".to_owned()]));
        assert!(cache.get("one").is_none());
        assert!(cache.get("two").is_some());
        assert!(!cache.retain(&["two".to_owned()]));
    }
}
