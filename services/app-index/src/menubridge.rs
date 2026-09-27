// SPDX-License-Identifier: MIT
//! The DBusMenu/AppMenu global-menu bridge (T-14.4).
//!
//! Third-party applications export a global menu over two de-facto protocols:
//! they register with `com.canonical.AppMenu.Registrar` (window id → menu
//! object path) and the menu object serves `com.canonical.dbusmenu`. This
//! module is the pure half of the bridge, so it can be unit-tested without a
//! bus:
//!
//! * [`AppMenuRegistry`] is the window→menu table and its removal rules.
//! * [`menus_from_layout`] projects a DBusMenu `GetLayout` tree (the
//!   [`crate::tray::MenuNode`] tree T-14.3 already parses) into the
//!   menu-broker's exported `menus` shape (`[{title, items}]`), preserving each
//!   row's `enabled`/`checked` state and synthesizing a routable `action` for
//!   clickable rows.
//! * [`published_model`] wraps those menus in the menu-broker's published
//!   payload so the projection can be pushed over
//!   `org.dragonfruit.MenuBroker1.PublishDbusMenu`.
//!
//! There is one DBusMenu parser in the tree ([`crate::tray::parse_layout`]) and
//! one row shape; the bridge only changes the tree's top layer (the DBusMenu
//! root's children become top-level menus) and tags rows with actions. The
//! D-Bus calls that fill these types live in [`crate::dbus`].

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::tray::{MenuNode, MenuNodeType};

/// The well-known name apps call to register a global menu (the de-facto
/// `com.canonical.AppMenu.Registrar`).
pub const REGISTRAR_NAME: &str = "com.canonical.AppMenu.Registrar";
/// The registrar's object path.
pub const REGISTRAR_PATH: &str = "/com/canonical/AppMenu/Registrar";
/// The registrar's interface name.
pub const REGISTRAR_INTERFACE: &str = "com.canonical.AppMenu.Registrar";

/// The menu-broker service the bridge pushes the projection into.
pub const MENU_BROKER_NAME: &str = "org.dragonfruit.MenuBroker1";
/// The menu-broker object path.
pub const MENU_BROKER_PATH: &str = "/org/dragonfruit/MenuBroker1";
/// The menu-broker interface.
pub const MENU_BROKER_INTERFACE: &str = "org.dragonfruit.MenuBroker1";
/// The menu-broker method that publishes a bridge-born model (tier
/// `dbusmenu`).
pub const MENU_BROKER_PUBLISH: &str = "PublishDbusMenu";

/// The action prefix a projected DBusMenu row carries. The id after the colon
/// is the DBusMenu item id, so a routed action can be turned back into an
/// `Event(id, "clicked")` with [`id_from_action`].
pub const ACTION_PREFIX: &str = "dbusmenu:";

/// One registered window → menu mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuRegistration {
    /// The window id the app registered (an X11 window id, or any stable id in
    /// our session).
    pub window_id: u32,
    /// The app identity the bridge keys the broker model by: the desktop id an
    /// app supplies, else the owner's bus name.
    pub app_id: String,
    /// The bus name the menu object is reached at.
    pub destination: String,
    /// The menu object path.
    pub path: String,
    /// The unique bus name that called `RegisterWindow` (so the registrar can
    /// drop the registration when its owner disconnects).
    pub owner: String,
}

impl MenuRegistration {
    /// Normalize a registration call. `app_id` may be empty, in which case the
    /// owner bus name is the key. Returns `None` for a bad path or owner.
    pub fn parse(
        window_id: u32,
        app_id: &str,
        destination: &str,
        path: &str,
        owner: &str,
    ) -> Option<Self> {
        let owner = owner.trim();
        if owner.is_empty() {
            return None;
        }
        let app_id = if app_id.trim().is_empty() {
            owner.to_owned()
        } else {
            app_id.trim().to_owned()
        };
        let path = normalize_path(path)?;
        let destination = if destination.trim().is_empty() {
            owner.to_owned()
        } else {
            destination.trim().to_owned()
        };
        Some(MenuRegistration {
            window_id,
            app_id,
            destination,
            path,
            owner: owner.to_owned(),
        })
    }
}

/// An object path must be absolute and non-empty.
fn normalize_path(path: &str) -> Option<String> {
    let path = path.trim();
    if !path.starts_with('/') {
        return None;
    }
    if path.len() > 1 && path.ends_with('/') {
        return Some(path.trim_end_matches('/').to_owned());
    }
    Some(path.to_owned())
}

/// The registered window→menu mappings, keyed by window id.
#[derive(Debug, Default, Clone)]
pub struct AppMenuRegistry {
    windows: BTreeMap<u32, MenuRegistration>,
}

impl AppMenuRegistry {
    /// A new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register (or replace) a window. Returns `true` when the set changed.
    pub fn register(&mut self, registration: MenuRegistration) -> bool {
        let key = registration.window_id;
        if self.windows.get(&key) == Some(&registration) {
            return false;
        }
        self.windows.insert(key, registration);
        true
    }

    /// Drop one window. Returns whether it was present.
    pub fn unregister(&mut self, window_id: u32) -> bool {
        self.windows.remove(&window_id).is_some()
    }

    /// Drop every window registered by `owner` (used when the caller
    /// disconnects). Returns the removed registrations.
    pub fn remove_owner(&mut self, owner: &str) -> Vec<MenuRegistration> {
        let mut removed = Vec::new();
        self.windows.retain(|_, registration| {
            if registration.owner == owner {
                removed.push(registration.clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// One registration by window id.
    pub fn get(&self, window_id: u32) -> Option<&MenuRegistration> {
        self.windows.get(&window_id)
    }

    /// Whether any window still maps to `app_id`.
    pub fn has_app(&self, app_id: &str) -> bool {
        self.windows
            .values()
            .any(|registration| registration.app_id == app_id)
    }

    /// The registrations, in window-id order.
    pub fn entries(&self) -> Vec<&MenuRegistration> {
        self.windows.values().collect()
    }

    /// The registered window ids.
    pub fn window_ids(&self) -> Vec<u32> {
        self.windows.keys().copied().collect()
    }

    /// How many windows are registered.
    pub fn len(&self) -> usize {
        self.windows.len()
    }

    /// Whether no window is registered.
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }
}

/// The synthesized action for one DBusMenu row.
pub fn action_for(id: i32) -> String {
    format!("{ACTION_PREFIX}{id}")
}

/// The DBusMenu item id encoded in a projected row's action, if any.
pub fn id_from_action(action: &str) -> Option<i32> {
    action.strip_prefix(ACTION_PREFIX)?.parse().ok()
}

/// Project one DBusMenu node (and its nested submenus) to a design-system row,
/// tagging a clickable row with its routable `dbusmenu:<id>` action. Other
/// fields — `enabled`, `checked`, `shortcut`, `id` — come from
/// [`MenuNode::to_json`] unchanged so the bridge and the tray share one row
/// shape.
pub fn entry_json(node: &MenuNode) -> Value {
    let mut value = node.to_json();
    let Some(object) = value.as_object_mut() else {
        return value;
    };
    if node.node_type == MenuNodeType::Item {
        object.insert("action".to_owned(), Value::String(action_for(node.id)));
    }
    if !node.children.is_empty() {
        object.insert(
            "submenu".to_owned(),
            Value::Array(
                node.children
                    .iter()
                    .filter(|child| child.visible)
                    .map(entry_json)
                    .collect(),
            ),
        );
    }
    value
}

/// Project a DBusMenu `GetLayout` root to the menu-broker's exported `menus`
/// shape: each visible, non-separator top-level child becomes one
/// `{title, items}` menu, with its children projected recursively. An empty
/// result means the exporter contributed nothing (the broker stays Tier 3).
pub fn menus_from_layout(root: &MenuNode) -> Vec<Value> {
    root.children
        .iter()
        .filter(|child| child.visible && child.node_type != MenuNodeType::Separator)
        .map(|child| {
            json!({
                "title": child.label,
                "items": child
                    .children
                    .iter()
                    .filter(|item| item.visible)
                    .map(entry_json)
                    .collect::<Vec<_>>(),
            })
        })
        .collect()
}

/// Wrap projected menus in the menu-broker's published payload. The exported
/// application menu is empty on purpose: the broker always synthesizes the
/// fixed application menu (About/Settings/Hide/…/Quit) and adds the app's own
/// menus after it.
pub fn published_model(app_name: &str, menus: Vec<Value>) -> String {
    json!({
        "appName": app_name,
        "applicationMenuItems": [],
        "menus": menus,
    })
    .to_string()
}

/// A human name for an app id when the app index has no `.desktop` record: the
/// final reverse-DNS segment, capitalized (`org.example.MockMenu` → `Menu`).
/// Empty is `Application`.
pub fn fallback_app_name(app_id: &str) -> String {
    let app_id = app_id.trim();
    if app_id.is_empty() {
        return "Application".to_owned();
    }
    let segment = app_id.rsplit('.').next().unwrap_or(app_id);
    let mut chars = segment.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => app_id.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use zbus::zvariant::{OwnedValue, Value as ZValue};

    fn registration(window_id: u32) -> MenuRegistration {
        MenuRegistration::parse(
            window_id,
            "org.example.App",
            "org.example.App",
            "/MenuBar",
            ":1.5",
        )
        .unwrap()
    }

    #[test]
    fn a_registration_falls_back_to_the_owner_for_a_blank_app_id() {
        let registration = MenuRegistration::parse(7, "", "", "/MenuBar", ":1.5").unwrap();
        assert_eq!(registration.app_id, ":1.5");
        assert_eq!(registration.destination, ":1.5");
        assert_eq!(registration.path, "/MenuBar");
    }

    #[test]
    fn malformed_registrations_are_rejected() {
        assert!(MenuRegistration::parse(1, "a", "a", "not-a-path", ":1.1").is_none());
        assert!(MenuRegistration::parse(1, "a", "a", "/MenuBar", "").is_none());
    }

    #[test]
    fn the_registry_replaces_removes_and_prunes_by_owner() {
        let mut registry = AppMenuRegistry::new();
        assert!(registry.register(registration(1)));
        assert!(!registry.register(registration(1)));
        assert!(registry.register(registration(2)));
        assert_eq!(registry.len(), 2);
        assert!(registry.has_app("org.example.App"));

        assert!(registry.unregister(1));
        assert!(!registry.unregister(1));
        assert_eq!(registry.window_ids(), vec![2]);

        assert_eq!(registry.remove_owner(":1.5").len(), 1);
        assert!(registry.is_empty());
        assert!(!registry.has_app("org.example.App"));
    }

    fn layout(id: i32, props: &[(&str, ZValue<'static>)], children: Vec<OwnedValue>) -> OwnedValue {
        let map: HashMap<String, OwnedValue> = props
            .iter()
            .map(|(k, v)| {
                (
                    k.to_string(),
                    OwnedValue::try_from(v.try_clone().unwrap()).unwrap(),
                )
            })
            .collect();
        OwnedValue::try_from(ZValue::from((id, map, children))).unwrap()
    }

    #[test]
    fn a_dbusmenu_tree_projects_to_the_broker_menus_shape() {
        // File › New (enabled) / Quit (disabled), then Edit.
        let new = layout(
            1,
            &[
                ("label", ZValue::from("New")),
                ("enabled", ZValue::from(true)),
                ("shortcut", ZValue::from(vec![vec!["<Ctrl>", "N"]])),
            ],
            vec![],
        );
        let quit = layout(
            2,
            &[
                ("label", ZValue::from("Quit")),
                ("enabled", ZValue::from(false)),
            ],
            vec![],
        );
        let file = layout(
            10,
            &[
                ("label", ZValue::from("_File")),
                ("children-display", ZValue::from("submenu")),
            ],
            vec![new, quit],
        );
        let edit = layout(
            11,
            &[
                ("label", ZValue::from("Edit")),
                ("children-display", ZValue::from("submenu")),
            ],
            vec![layout(3, &[("label", ZValue::from("Cut"))], vec![])],
        );
        let root = layout(0, &[], vec![file, edit]);

        let node = crate::tray::parse_layout(&root).unwrap();
        let menus = menus_from_layout(&node);
        assert_eq!(menus.len(), 2);
        assert_eq!(menus[0]["title"], "File");
        assert_eq!(menus[0]["items"][0]["label"], "New");
        assert_eq!(menus[0]["items"][0]["enabled"], true);
        assert_eq!(menus[0]["items"][0]["action"], "dbusmenu:1");
        assert_eq!(menus[0]["items"][1]["label"], "Quit");
        assert_eq!(menus[0]["items"][1]["enabled"], false);
        assert_eq!(menus[1]["title"], "Edit");

        let model = published_model("Example", menus);
        let value: Value = serde_json::from_str(&model).unwrap();
        assert_eq!(value["applicationMenuItems"], json!([]));
        assert_eq!(value["menus"][0]["title"], "File");
    }

    #[test]
    fn nested_submenus_keep_their_actions_and_separators_are_skipped() {
        let preferences = layout(4, &[("label", ZValue::from("Preferences"))], vec![]);
        let advanced = layout(
            5,
            &[
                ("label", ZValue::from("Advanced")),
                ("children-display", ZValue::from("submenu")),
            ],
            vec![preferences],
        );
        let tools = layout(
            7,
            &[
                ("label", ZValue::from("Tools")),
                ("children-display", ZValue::from("submenu")),
            ],
            vec![advanced],
        );
        let separator = layout(6, &[("type", ZValue::from("separator"))], vec![]);
        let root = layout(0, &[], vec![separator, tools]);

        let node = crate::tray::parse_layout(&root).unwrap();
        let menus = menus_from_layout(&node);
        assert_eq!(menus.len(), 1);
        assert_eq!(menus[0]["title"], "Tools");
        assert_eq!(menus[0]["items"][0]["type"], "submenu");
        assert_eq!(menus[0]["items"][0]["label"], "Advanced");
        assert_eq!(menus[0]["items"][0]["submenu"][0]["label"], "Preferences");
        assert_eq!(menus[0]["items"][0]["submenu"][0]["action"], "dbusmenu:4");
        assert_eq!(id_from_action("dbusmenu:4"), Some(4));
        assert_eq!(id_from_action("nope"), None);
    }

    #[test]
    fn fallback_app_name_capitalizes_the_last_segment() {
        assert_eq!(fallback_app_name("org.example.MockMenu"), "MockMenu");
        assert_eq!(fallback_app_name("firefox"), "Firefox");
        assert_eq!(fallback_app_name(""), "Application");
    }
}
