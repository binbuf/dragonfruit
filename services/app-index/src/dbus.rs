// SPDX-License-Identifier: MIT
//! The session-bus surface (T-14.1a/T-14.1b).
//!
//! The service owns one well-known name, `org.dragonfruit.AppIndex1`, and
//! serves the interface of the same name at
//! `/org/dragonfruit/AppIndex1`. The shell (Dock, menu bar, app switcher) is
//! the consumer: it never scans `.desktop` directories itself.
//!
//! Every query returns a flat JSON string so the Qt side decodes with
//! `QJsonDocument` and needs no `a{sv}` type knowledge. A miss is the empty
//! string (or `[]` for the list calls), never an error.
//!
//! T-14.1b adds the maintenance and activity surface on top of T-14.1a's
//! identity queries:
//!
//! * `Refresh` re-scans the `.desktop` directories and diffs against the
//!   current corpus; the service also watches them with inotify and calls the
//!   same path, so installs/uninstalls/updates reach the index without
//!   polling. `IndexEvents` drains the retained diff.
//! * `WindowOpened`/`WindowClosed`/`NoteActivity` feed the launch registry and
//!   the recency order; `Running`/`Recent` read them back, and
//!   `ActivityEvents` drains the log.
//!
//! T-14.1c adds the subscription surface: `Subscribe`/`Unsubscribe` register a
//! consumer (by its unique bus name) for the change categories it cares about,
//! and a coalescer thread delivers one directed `Changed` signal per burst
//! (`subscription`). Consumers stop re-querying; no polling remains.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use zbus::blocking::connection;
use zbus::interface;
use zbus::message::Header;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedValue, Value};
use zbus::Connection;

use crate::icons::IconTheme;
use crate::index::AppIndex;
use crate::registry::{self, LaunchRegistry};
use crate::subscription::{ChangeKind, ChangeNotice, Interests, Subscriptions};
use crate::tray::{self, Registration, TrayRegistry};
use crate::view;
use crate::watch;

/// The well-known name on the user session bus.
pub const DBUS_NAME: &str = "org.dragonfruit.AppIndex1";
/// The object path the interface is served at.
pub const DBUS_PATH: &str = "/org/dragonfruit/AppIndex1";
/// The interface name.
pub const INTERFACE: &str = "org.dragonfruit.AppIndex1";

/// The StatusNotifierWatcher well-known name app-index also owns (T-14.3).
pub const WATCHER_NAME: &str = "org.kde.StatusNotifierWatcher";
/// The watcher's object path.
pub const WATCHER_PATH: &str = "/StatusNotifierWatcher";
/// The watcher's interface name.
pub const WATCHER_INTERFACE: &str = "org.kde.StatusNotifierWatcher";
/// The StatusNotifierItem interface tray apps serve.
pub const ITEM_INTERFACE: &str = "org.kde.StatusNotifierItem";
/// The DBusMenu interface tray menu objects serve.
pub const DBUSMENU_INTERFACE: &str = "com.canonical.dbusmenu";

/// The wake handle for the coalescer thread: a flag plus a condvar so a new
/// change re-arms the sleep immediately instead of waiting out the old window.
pub type Wake = Arc<(Mutex<bool>, Condvar)>;

/// A fresh wake handle.
pub fn new_wake() -> Wake {
    Arc::new((Mutex::new(false), Condvar::new()))
}

/// Wake the coalescer thread so it recomputes its sleep.
pub fn wake_subscriptions(wake: &Wake) {
    let (flag, condvar) = &**wake;
    if let Ok(mut woken) = flag.lock() {
        *woken = true;
        condvar.notify_all();
    }
}

/// The served object: one shared index and launch registry behind mutexes,
/// plus the icon theme and the coalesced-subscription table.
#[derive(Clone)]
pub struct AppIndex1 {
    index: Arc<Mutex<AppIndex>>,
    registry: Arc<Mutex<LaunchRegistry>>,
    theme: IconTheme,
    subscriptions: Arc<Mutex<Subscriptions>>,
    wake: Wake,
    /// The StatusNotifierWatcher's registered items (T-14.3); shared with the
    /// watcher object that owns the `org.kde.StatusNotifierWatcher` name.
    tray: Arc<Mutex<TrayRegistry>>,
}

impl AppIndex1 {
    /// A new object over a freshly scanned index, an empty launch registry,
    /// and the environment's icon theme.
    pub fn new() -> Self {
        Self::from_index(AppIndex::load(), IconTheme::from_env())
    }

    /// A new object over an explicit index and theme (tests, fixtures).
    pub fn from_index(index: AppIndex, theme: IconTheme) -> Self {
        Self::from_parts(index, LaunchRegistry::new(), theme)
    }

    /// A new object over an explicit index, launch registry, and theme.
    pub fn from_parts(index: AppIndex, registry: LaunchRegistry, theme: IconTheme) -> Self {
        AppIndex1 {
            index: Arc::new(Mutex::new(index)),
            registry: Arc::new(Mutex::new(registry)),
            theme,
            subscriptions: Arc::new(Mutex::new(Subscriptions::new())),
            wake: new_wake(),
            tray: Arc::new(Mutex::new(TrayRegistry::new())),
        }
    }

    /// The shared tray registry (T-14.3).
    pub fn tray(&self) -> &Arc<Mutex<TrayRegistry>> {
        &self.tray
    }

    /// The shared index.
    pub fn index(&self) -> &Arc<Mutex<AppIndex>> {
        &self.index
    }

    /// The shared launch registry.
    pub fn registry(&self) -> &Arc<Mutex<LaunchRegistry>> {
        &self.registry
    }

    /// The icon theme in use.
    pub fn theme(&self) -> &IconTheme {
        &self.theme
    }

    /// The shared coalesced-subscription table.
    pub fn subscriptions(&self) -> &Arc<Mutex<Subscriptions>> {
        &self.subscriptions
    }

    /// Note a change for the coalescer and wake it. Called by every mutation
    /// path; consumers receive at most one `Changed` per coalescing window.
    pub fn note_change(&self, kind: ChangeKind) {
        self.note_change_at(kind, registry::now_ms());
    }

    /// Note a change at an explicit clock reading (deterministic tests).
    pub fn note_change_at(&self, kind: ChangeKind, now_ms: u64) {
        lock_subs(&self.subscriptions).note(kind, now_ms);
        wake_subscriptions(&self.wake);
    }

    /// Project one tray item's SNI properties to the shell's JSON view.
    fn tray_item_json(
        &self,
        registration: &Registration,
        props: &HashMap<String, OwnedValue>,
    ) -> serde_json::Value {
        let icon_name = prop_str(props, "IconName").unwrap_or_default();
        let icon_path = self
            .theme
            .lookup(&icon_name, view::DEFAULT_ICON_SIZE)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        let status = prop_str(props, "Status").unwrap_or_else(|| "Active".to_owned());
        let attention_icon = prop_str(props, "AttentionIconName").unwrap_or_default();
        let icon_pixmap = props.get("IconPixmap").is_some();
        serde_json::json!({
            "name": registration.name,
            "destination": registration.destination,
            "path": registration.path,
            "id": prop_str(props, "Id").unwrap_or_default(),
            "title": prop_str(props, "Title").unwrap_or_default(),
            "iconName": icon_name,
            "iconPath": icon_path,
            "tooltip": tooltip_text(props),
            "menuPath": prop_str(props, "Menu").unwrap_or_default(),
            "itemIsMenu": prop_bool(props, "ItemIsMenu").unwrap_or(false),
            "status": status,
            "needsAttention": status == "NeedsAttention" || !attention_icon.is_empty(),
            "hasPixmap": icon_pixmap,
        })
    }

    /// One item's DBusMenu object path, if it exports one.
    async fn tray_menu_path(
        &self,
        connection: &Connection,
        registration: &Registration,
    ) -> Option<String> {
        let props = item_properties(connection, registration).await?;
        prop_str(&props, "Menu").filter(|path| !path.is_empty())
    }

    /// The registry key, display name, and icon for a window identity. A
    /// resolved identity keys by desktop id; an unresolved one keys by the
    /// canonical raw identifier (`WM_CLASS` class, else instance, else
    /// `app_id`) so a stable window still groups.
    fn activity_key(&self, app_id: &str, instance: &str, class: &str) -> (String, String, String) {
        let index = lock(&self.index);
        if let Some(resolved) = index.resolve_activity(app_id, instance, class) {
            return (
                resolved.record.desktop_id,
                resolved.record.name,
                resolved.record.icon,
            );
        }
        drop(index);
        let key = raw_activity_key(app_id, instance, class);
        let name = key.clone();
        (key, name, String::new())
    }
}

/// The canonical raw registry key for an unresolved window identity.
fn raw_activity_key(app_id: &str, instance: &str, class: &str) -> String {
    let class = class.trim();
    if !class.is_empty() {
        return class.to_ascii_lowercase();
    }
    let instance = instance.trim();
    if !instance.is_empty() {
        return instance.to_ascii_lowercase();
    }
    app_id.trim().to_ascii_lowercase()
}

impl Default for AppIndex1 {
    fn default() -> Self {
        Self::new()
    }
}

/// Lock the shared index, recovering from a poisoned mutex: a D-Bus method may
/// panic on a bad argument and the service must keep answering.
fn lock(index: &Arc<Mutex<AppIndex>>) -> std::sync::MutexGuard<'_, AppIndex> {
    index
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared launch registry, recovering from a poisoned mutex.
fn lock_registry(
    registry: &Arc<Mutex<LaunchRegistry>>,
) -> std::sync::MutexGuard<'_, LaunchRegistry> {
    registry
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared subscription table, recovering from a poisoned mutex.
fn lock_subs(
    subscriptions: &Arc<Mutex<Subscriptions>>,
) -> std::sync::MutexGuard<'_, Subscriptions> {
    subscriptions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Lock the shared tray registry, recovering from a poisoned mutex.
fn lock_tray(tray: &Arc<Mutex<TrayRegistry>>) -> std::sync::MutexGuard<'_, TrayRegistry> {
    tray.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Fetch every property of a StatusNotifierItem. `None` when the owning
/// process cannot be reached (the caller prunes the item).
async fn item_properties(
    connection: &Connection,
    registration: &Registration,
) -> Option<HashMap<String, OwnedValue>> {
    let reply = connection
        .call_method(
            Some(registration.destination.as_str()),
            registration.path.as_str(),
            Some("org.freedesktop.DBus.Properties"),
            "GetAll",
            &(ITEM_INTERFACE,),
        )
        .await
        .ok()?;
    reply
        .body()
        .deserialize::<HashMap<String, OwnedValue>>()
        .ok()
}

/// Read one string property from an `a{sv}` map. An object path (the SNI
/// `Menu` property) reads as its string form too.
fn prop_str(props: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = props.get(key)?.try_clone().ok()?;
    if let Ok(text) = String::try_from(value.try_clone().ok()?) {
        return Some(text);
    }
    if let Ok(path) = zbus::zvariant::ObjectPath::try_from(value) {
        return Some(path.as_str().to_owned());
    }
    None
}

/// Read one bool property from an `a{sv}` map.
fn prop_bool(props: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
    props
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| bool::try_from(value).ok())
}

/// The `ToolTip` property's display text: `(s, a(iiay), s, s)`; prefer the
/// text (4th) and fall back to the title (3rd).
fn tooltip_text(props: &HashMap<String, OwnedValue>) -> String {
    let Some(value) = props
        .get("ToolTip")
        .and_then(|value| value.try_clone().ok())
    else {
        return String::new();
    };
    if let Ok((_, _, title, text)) = <(String, Vec<OwnedValue>, String, String)>::try_from(value) {
        if !text.is_empty() {
            return text;
        }
        return title;
    }
    String::new()
}

/// Call `GetLayout` on an item's DBusMenu and project it. `AboutToShow` runs
/// first so dynamic menus populate before the layout is read.
async fn fetch_menu_layout(
    connection: &Connection,
    destination: &str,
    menu_path: &str,
) -> Option<tray::MenuNode> {
    let _ = call_dbusmenu(connection, destination, menu_path, "AboutToShow", &(0i32,)).await;
    let reply = zbus::Proxy::new(connection, destination, menu_path, DBUSMENU_INTERFACE)
        .await
        .ok()?
        .call_method("GetLayout", &(0i32, -1i32, Vec::<String>::new()))
        .await
        .ok()?;
    let (_, layout): (u32, OwnedValue) = reply.body().deserialize().ok()?;
    tray::parse_layout(&layout)
}

/// Call one DBusMenu method, returning whether it succeeded.
async fn call_dbusmenu(
    connection: &Connection,
    destination: &str,
    menu_path: &str,
    method: &str,
    body: &(impl serde::Serialize + zbus::zvariant::DynamicType),
) -> bool {
    let Ok(proxy) = zbus::Proxy::new(connection, destination, menu_path, DBUSMENU_INTERFACE).await
    else {
        return false;
    };
    proxy.call_method(method, body).await.is_ok()
}

#[interface(name = "org.dragonfruit.AppIndex1")]
impl AppIndex1 {
    /// Resolve a Wayland `app_id` (or desktop id) to a record object, `""` on
    /// a miss. The record carries the themed `iconPath`.
    fn resolve(&self, identity: &str) -> String {
        let mut index = lock(&self.index);
        match index.resolve(identity) {
            Some(resolved) => view::record_json(&resolved.record, resolved.source, &self.theme),
            None => String::new(),
        }
    }

    /// Resolve a window's identifiers to a record object, `""` on a miss.
    /// `app_id` is the Wayland identifier (empty under Xwayland);
    /// `instance`/`class` are the `WM_CLASS` pair (empty on Wayland).
    fn resolve_window(&self, app_id: &str, instance: &str, class: &str) -> String {
        let mut index = lock(&self.index);
        match index.resolve_window(app_id, instance, class) {
            Some(resolved) => view::record_json(&resolved.record, resolved.source, &self.theme),
            None => String::new(),
        }
    }

    /// Look up an installed record by exact desktop id, `""` on a miss.
    fn lookup(&self, desktop_id: &str) -> String {
        let index = lock(&self.index);
        match index.lookup(desktop_id) {
            Some(resolved) => view::record_json(&resolved.record, resolved.source, &self.theme),
            None => String::new(),
        }
    }

    /// Every installed record as a JSON array. The shell uses this for the
    /// Dock's default pins and its local fallback.
    fn enumerate(&self) -> String {
        let index = lock(&self.index);
        view::records_json(&index, &self.theme)
    }

    /// The identity misses recorded so far, as a JSON array of strings. This
    /// is the heuristic input (T-14.1b folds it back into the table).
    fn misses(&self) -> String {
        let index = lock(&self.index);
        view::misses_json(&index)
    }

    /// Resolve an icon name to a themed file path for `size`, `""` when no
    /// theme provides it. An absolute name that exists is passed through.
    fn icon_path(&self, name: &str, size: i32) -> String {
        let size = if size <= 0 {
            view::DEFAULT_ICON_SIZE
        } else {
            size
        };
        self.theme
            .lookup(name, size)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// `(entries, resolved, unresolved)` counts, for the audit log.
    fn stats(&self) -> (u32, u64, u64) {
        let index = lock(&self.index);
        (
            index.len() as u32,
            index.resolved_count(),
            index.unresolved_count(),
        )
    }

    /// Re-scan the `.desktop` directories and diff against the current corpus,
    /// updating the index. Returns the install/uninstall/update events as a
    /// JSON array (oldest scan first). The service also calls this from its
    /// inotify watcher, so consumers rarely need to ask.
    async fn refresh(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        let events = lock(&self.index).refresh();
        for event in &events {
            let _ = Self::index_changed(
                &emitter,
                event.kind.as_str(),
                &event.desktop_id,
                &event.name,
            )
            .await;
        }
        if !events.is_empty() {
            // An install/uninstall/update changes both the identity corpus and
            // the set of icons that resolve, so both kinds are noted.
            self.note_change(ChangeKind::Identity);
            self.note_change(ChangeKind::Icons);
        }
        view::index_events_json(&events)
    }

    /// Take and clear the retained install/uninstall/update events as a JSON
    /// array, oldest first.
    fn index_events(&self) -> String {
        let events = lock(&self.index).drain_events();
        view::index_events_json(&events)
    }

    /// A window for the app appeared. Resolves the identity (keying by desktop
    /// id when it resolves), marks the app running, moves it to the front of
    /// the recency order, and emits `AppRunning`. Returns the running-app
    /// object, or `""` when the identity is empty.
    async fn window_opened(
        &self,
        app_id: &str,
        instance: &str,
        class: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> String {
        let (key, name, icon) = self.activity_key(app_id, instance, class);
        if key.is_empty() {
            return String::new();
        }
        let event = lock_registry(&self.registry).app_running(&key, &name, &icon);
        let _ = Self::app_running(&emitter, &event.key, event.windows).await;
        self.note_change(ChangeKind::Recency);
        let value = lock_registry(&self.registry)
            .app(&key)
            .map(|app| view::running_app_value(app, &self.theme))
            .unwrap_or(serde_json::Value::Null);
        value.to_string()
    }

    /// A window for the app closed. Emits `AppExited` and returns whether the
    /// app was running.
    async fn window_closed(
        &self,
        app_id: &str,
        instance: &str,
        class: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let (key, _, _) = self.activity_key(app_id, instance, class);
        if key.is_empty() {
            return false;
        }
        let event = lock_registry(&self.registry).app_exited(&key);
        match event {
            Some(event) => {
                let _ = Self::app_exited(&emitter, &event.key, event.windows).await;
                self.note_change(ChangeKind::Recency);
                true
            }
            None => false,
        }
    }

    /// The app became the focused window. Recency only; returns whether the
    /// app was known (running) at the time.
    fn note_activity(&self, app_id: &str, instance: &str, class: &str) -> bool {
        let (key, _, _) = self.activity_key(app_id, instance, class);
        if key.is_empty() {
            return false;
        }
        let was_running = lock_registry(&self.registry).is_running(&key);
        lock_registry(&self.registry).note_activity(&key);
        self.note_change(ChangeKind::Recency);
        was_running
    }

    /// The running apps as a JSON array, ordered by key.
    fn running(&self) -> String {
        view::running_json(&lock_registry(&self.registry), &self.theme)
    }

    /// The recency order as a JSON array of record objects, most recent first.
    /// `limit <= 0` means the registry's own capacity.
    fn recent(&self, limit: i32) -> String {
        let limit = if limit <= 0 { 0 } else { limit as usize };
        let index = lock(&self.index);
        view::recent_json(&index, &lock_registry(&self.registry), limit, &self.theme)
    }

    /// Take and clear the `app_running`/`app_exited`/`focused` events as a
    /// JSON array, oldest first.
    fn activity_events(&self) -> String {
        let events = lock_registry(&self.registry).drain_events();
        view::activity_events_json(&events)
    }

    /// Register the caller for coalesced change signals. `interests` is a
    /// comma-separated set of `identity`, `recency`, and `icons`, or `all`
    /// (empty means all). Returns the canonical interest string; the caller's
    /// unique bus name is the destination of its `Changed` signals, so a
    /// reconnecting consumer re-subscribes simply by calling this again.
    fn subscribe(&self, interests: &str, #[zbus(header)] header: Header<'_>) -> String {
        let Some(sender) = header.sender() else {
            return String::new();
        };
        let interests = Interests::parse(interests);
        lock_subs(&self.subscriptions).subscribe(sender.as_str(), interests);
        interests.as_str()
    }

    /// Drop the caller's subscription (and any undelivered notice). Returns
    /// `true` when a subscription was removed.
    fn unsubscribe(&self, #[zbus(header)] header: Header<'_>) -> bool {
        match header.sender() {
            Some(sender) => lock_subs(&self.subscriptions).unsubscribe(sender.as_str()),
            None => false,
        }
    }

    /// Deliver every pending coalesced notice now instead of at the end of the
    /// window. Returns the number of subscribers notified. Consumers normally
    /// never call this; it is the deterministic hook for tests and an escape
    /// hatch for a consumer that must not wait out the window.
    async fn flush(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> u32 {
        let notices = lock_subs(&self.subscriptions).flush();
        emit_notices(&emitter, &notices).await
    }

    /// The number of registered subscribers (diagnostics).
    fn subscriber_count(&self) -> u32 {
        lock_subs(&self.subscriptions).subscribers() as u32
    }

    /// The live StatusNotifier/AppIndicator items as a JSON array (T-14.3).
    /// Each item is resolved fresh from its owning process, and an item whose
    /// process has vanished is pruned. A malformed or unresponsive item is
    /// skipped, never an error.
    async fn tray_items(&self, #[zbus(connection)] connection: &Connection) -> String {
        let registrations: Vec<Registration> = lock_tray(&self.tray).items().to_vec();
        let mut live: Vec<Registration> = Vec::with_capacity(registrations.len());
        let mut values: Vec<serde_json::Value> = Vec::with_capacity(registrations.len());
        for registration in registrations {
            let Some(props) = item_properties(connection, &registration).await else {
                lock_tray(&self.tray).remove(&registration.name);
                continue;
            };
            values.push(self.tray_item_json(&registration, &props));
            live.push(registration);
        }
        serde_json::Value::Array(values).to_string()
    }

    /// The registered tray item names, as a JSON array (diagnostics/tests).
    fn tray_names(&self) -> String {
        serde_json::to_string(&lock_tray(&self.tray).names()).unwrap_or_else(|_| "[]".to_owned())
    }

    /// One item's DBusMenu as a JSON array of design-system rows (T-14.3).
    /// Calls `AboutToShow` first so dynamic menus populate, then `GetLayout`.
    /// An unknown item or a menu failure is `[]`.
    async fn tray_menu(&self, name: &str, #[zbus(connection)] connection: &Connection) -> String {
        let Some(registration) = lock_tray(&self.tray).get(name).cloned() else {
            return "[]".to_owned();
        };
        let Some(menu_path) = self.tray_menu_path(connection, &registration).await else {
            return "[]".to_owned();
        };
        let Some(root) = fetch_menu_layout(connection, &registration.destination, &menu_path).await
        else {
            return "[]".to_owned();
        };
        serde_json::Value::Array(root.children.iter().map(tray::MenuNode::to_json).collect())
            .to_string()
    }

    /// Activate one tray menu row by its DBusMenu id (T-14.3). Returns whether
    /// the owning process accepted the event.
    async fn tray_menu_event(
        &self,
        name: &str,
        id: i32,
        #[zbus(connection)] connection: &Connection,
    ) -> bool {
        let Some(registration) = lock_tray(&self.tray).get(name).cloned() else {
            return false;
        };
        let Some(menu_path) = self.tray_menu_path(connection, &registration).await else {
            return false;
        };
        call_dbusmenu(
            connection,
            &registration.destination,
            &menu_path,
            "Event",
            &(id, "clicked", Value::from(0i32), 0u32),
        )
        .await
    }

    /// A primary (`activate`), secondary (`secondary`) or context (`context`)
    /// activation on a tray item (T-14.3). For an item whose `ItemIsMenu` is
    /// set, the shell opens the menu instead; this is the direct path.
    async fn tray_activate(
        &self,
        name: &str,
        kind: &str,
        #[zbus(connection)] connection: &Connection,
    ) -> bool {
        let Some(registration) = lock_tray(&self.tray).get(name).cloned() else {
            return false;
        };
        let method = match kind {
            "activate" => "Activate",
            "secondary" => "SecondaryActivate",
            "context" => "ContextMenu",
            _ => return false,
        };
        let proxy = match zbus::Proxy::new(
            connection,
            registration.destination.as_str(),
            registration.path.as_str(),
            ITEM_INTERFACE,
        )
        .await
        {
            Ok(proxy) => proxy,
            Err(_) => return false,
        };
        proxy.call_method(method, &(0i32, 0i32)).await.is_ok()
    }

    /// An app appeared on screen (`app_running`).
    #[zbus(signal)]
    async fn app_running(emitter: &SignalEmitter<'_>, key: &str, windows: u32) -> zbus::Result<()>;

    /// An app's window closed (`app_exited`); `windows` is the remaining
    /// count.
    #[zbus(signal)]
    async fn app_exited(emitter: &SignalEmitter<'_>, key: &str, windows: u32) -> zbus::Result<()>;

    /// The installed corpus changed; `kind` is `installed`, `updated`, or
    /// `uninstalled`.
    #[zbus(signal)]
    async fn index_changed(
        emitter: &SignalEmitter<'_>,
        kind: &str,
        desktop_id: &str,
        name: &str,
    ) -> zbus::Result<()>;

    /// A coalesced change for a subscriber; `interests` is the canonical
    /// comma-separated set of kinds that changed. Delivered only to the
    /// subscribers that asked for at least one of them.
    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>, interests: &str) -> zbus::Result<()>;

    /// The tray item set changed (an item registered, left, or updated). The
    /// shell re-reads `TrayItems`; there is no polling.
    #[zbus(signal)]
    async fn tray_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// The `org.kde.StatusNotifierWatcher` registry (T-14.3). It shares the tray
/// registry with [`AppIndex1`] and re-emits `TrayChanged` on the app-index
/// interface so the shell has one signal to watch.
#[derive(Clone)]
pub struct StatusNotifierWatcher {
    tray: Arc<Mutex<TrayRegistry>>,
}

impl Default for StatusNotifierWatcher {
    fn default() -> Self {
        StatusNotifierWatcher::new()
    }
}

impl StatusNotifierWatcher {
    /// A new watcher over a fresh registry.
    pub fn new() -> Self {
        StatusNotifierWatcher {
            tray: Arc::new(Mutex::new(TrayRegistry::new())),
        }
    }

    /// A watcher over an existing registry (shared with the app-index object).
    pub fn over(tray: Arc<Mutex<TrayRegistry>>) -> Self {
        StatusNotifierWatcher { tray }
    }

    /// The shared registry.
    pub fn tray(&self) -> &Arc<Mutex<TrayRegistry>> {
        &self.tray
    }
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcher {
    /// Register a StatusNotifierItem. `service` is a bus name (the item lives
    /// at `/StatusNotifierItem`) or an object path on the caller. Returns
    /// whether the registration changed the set.
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> bool {
        let Some(sender) = header.sender() else {
            return false;
        };
        let Some(registration) = Registration::parse(service, sender.as_str()) else {
            return false;
        };
        let changed = lock_tray(&self.tray).register(registration);
        if changed {
            emit_tray_changed(connection).await;
        }
        changed
    }

    /// The host registers itself (the shell is the host; app-index serves the
    /// watcher, so this is accepted and answered). Returns `true`.
    fn register_status_notifier_host(&self, _service: &str) -> bool {
        true
    }

    /// The item names currently registered.
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        lock_tray(&self.tray).names()
    }

    /// app-index always provides a host, so the flag is `true`.
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    /// The SNI protocol version this watcher speaks.
    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        0
    }

    /// An item registered.
    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// An item left. app-index emits this when a registered item's owner
    /// disconnects.
    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    /// A host registered.
    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// A host left.
    #[zbus(signal)]
    async fn status_notifier_host_unregistered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Emit `TrayChanged` on the app-index interface.
async fn emit_tray_changed(connection: &Connection) {
    let _ = connection
        .emit_signal(None::<&str>, DBUS_PATH, INTERFACE, "TrayChanged", &())
        .await;
}

/// Emit one directed `Changed` signal per notice, returning the count.
async fn emit_notices(emitter: &SignalEmitter<'_>, notices: &[ChangeNotice]) -> u32 {
    let connection = emitter.connection();
    for notice in notices {
        let _ = connection
            .emit_signal(
                Some(notice.destination.as_str()),
                DBUS_PATH,
                INTERFACE,
                "Changed",
                &(notice.interests_str()),
            )
            .await;
    }
    notices.len() as u32
}

/// Serve `org.dragonfruit.AppIndex1` on the session bus until the process is
/// asked to stop. Returns an error only when the bus or the name cannot be
/// taken; a session without a bus is reported and exited instead of blocking.
pub fn run() -> zbus::Result<()> {
    let object = AppIndex1::new();
    let watcher = StatusNotifierWatcher::over(object.tray().clone());
    let connection = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, object.clone())?
        .serve_at(WATCHER_PATH, watcher)?
        .build()?;
    // app-index is also the tray host: it takes the SNI watcher name the
    // third-party tray apps register with (T-14.3). On a session where another
    // host already owns it (a running desktop shell), the name is left alone;
    // the watcher object still answers at WATCHER_PATH, so a tray item can
    // register directly against app-index.
    if let Err(error) = connection.request_name(WATCHER_NAME) {
        eprintln!("dragonfruit-app-index: {WATCHER_NAME} already owned ({error}); serving the watcher object only");
    }

    spawn_watcher(connection.clone(), object.clone());
    spawn_tray_cleaner(connection.clone(), object.clone());
    spawn_coalescer(connection.clone(), object);

    // The blocking object server runs on its own executor; parking the main
    // thread keeps the process (and the name) alive without a poll loop.
    let _ = connection;
    loop {
        std::thread::park();
    }
}

/// Start the inotify watcher on the index's own `applications` directories.
/// Each monitor event re-scans and diffs the index, emits `IndexChanged` for
/// every install/uninstall/update, and notes the change for subscribers, so
/// the corpus stays live with no polling.
fn spawn_watcher(connection: connection::Connection, object: AppIndex1) {
    let dirs = lock(&object.index).dirs().to_vec();
    let index = object.index.clone();
    let subscriptions = object.clone();
    let _ = std::thread::Builder::new()
        .name("dragonfruit-app-index-watch".to_owned())
        .spawn(move || {
            let _ = watch::watch_dirs(dirs, move || {
                let events = lock(&index).refresh();
                for event in &events {
                    let _ = connection.emit_signal(
                        None::<&str>,
                        DBUS_PATH,
                        INTERFACE,
                        "IndexChanged",
                        &(
                            event.kind.as_str(),
                            event.desktop_id.as_str(),
                            event.name.as_str(),
                        ),
                    );
                }
                if !events.is_empty() {
                    subscriptions.note_change(ChangeKind::Identity);
                    subscriptions.note_change(ChangeKind::Icons);
                }
            });
        });
}

/// Watch `NameOwnerChanged` and drop tray items whose owning process left, so
/// a crashed tray app disappears from the bar without a poll. Emits
/// `TrayChanged` when the set actually shrinks.
pub fn spawn_tray_cleaner(connection: connection::Connection, object: AppIndex1) {
    let tray = object.tray().clone();
    let emitter = connection.clone();
    let _ = std::thread::Builder::new()
        .name("dragonfruit-app-index-tray-cleaner".to_owned())
        .spawn(move || {
            let Ok(proxy) = zbus::blocking::Proxy::new(
                &connection,
                "org.freedesktop.DBus",
                "/org/freedesktop/DBus",
                "org.freedesktop.DBus",
            ) else {
                return;
            };
            let Ok(mut signals) = proxy.receive_signal("NameOwnerChanged") else {
                return;
            };
            for message in signals.by_ref() {
                let Ok((name, old_owner, new_owner)) =
                    message.body().deserialize::<(String, String, String)>()
                else {
                    continue;
                };
                if old_owner.is_empty() || !new_owner.is_empty() {
                    continue;
                }
                if !lock_tray(&tray).remove_owner(&name).is_empty() {
                    let _ =
                        emitter.emit_signal(None::<&str>, DBUS_PATH, INTERFACE, "TrayChanged", &());
                }
            }
        });
}

/// Start the coalescer thread. It sleeps until [`Subscriptions::next_deadline_ms`]
/// and then delivers one directed `Changed` signal per subscriber; an idle
/// service is asleep, and a new change re-arms the sleep via the wake handle.
pub fn spawn_coalescer(connection: connection::Connection, object: AppIndex1) {
    let subscriptions = object.subscriptions.clone();
    let wake = object.wake.clone();
    let _ = std::thread::Builder::new()
        .name("dragonfruit-app-index-coalesce".to_owned())
        .spawn(move || coalescer_loop(connection, subscriptions, wake));
}

fn coalescer_loop(
    connection: connection::Connection,
    subscriptions: Arc<Mutex<Subscriptions>>,
    wake: Wake,
) {
    loop {
        let now = registry::now_ms();
        let notices = lock_subs(&subscriptions).due(now);
        for notice in notices {
            let _ = connection.emit_signal(
                Some(notice.destination.as_str()),
                DBUS_PATH,
                INTERFACE,
                "Changed",
                &(notice.interests_str()),
            );
        }

        let wait = match lock_subs(&subscriptions).next_deadline_ms() {
            Some(deadline) => {
                Duration::from_millis(deadline.saturating_sub(registry::now_ms()).max(1))
            }
            None => Duration::from_secs(3600),
        };

        let (flag, condvar) = &*wake;
        let mut woken = flag.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if *woken {
            *woken = false;
            continue;
        }
        let (mut woken, _) = condvar
            .wait_timeout(woken, wait)
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if *woken {
            *woken = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_and_path_follow_the_lockstep_policy() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME), "{DBUS_NAME}");
        assert_eq!(DBUS_NAME, df_ipc::dbus_name("AppIndex", 1).to_string());
        assert!(DBUS_PATH.starts_with("/org/dragonfruit/"));
        assert_eq!(INTERFACE, DBUS_NAME);
    }
}
