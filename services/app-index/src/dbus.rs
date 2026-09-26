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
//! The subscription API (coalesced signals to consumers) is T-14.1c; the
//! signals below are emitted additively now so that task only has to add the
//! subscription bookkeeping.

use std::sync::{Arc, Mutex};

use zbus::blocking::connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::icons::IconTheme;
use crate::index::AppIndex;
use crate::registry::LaunchRegistry;
use crate::view;
use crate::watch;

/// The well-known name on the user session bus.
pub const DBUS_NAME: &str = "org.dragonfruit.AppIndex1";
/// The object path the interface is served at.
pub const DBUS_PATH: &str = "/org/dragonfruit/AppIndex1";
/// The interface name.
pub const INTERFACE: &str = "org.dragonfruit.AppIndex1";

/// The served object: one shared index and launch registry behind mutexes,
/// plus the icon theme.
#[derive(Clone)]
pub struct AppIndex1 {
    index: Arc<Mutex<AppIndex>>,
    registry: Arc<Mutex<LaunchRegistry>>,
    theme: IconTheme,
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
        }
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
}

/// Serve `org.dragonfruit.AppIndex1` on the session bus until the process is
/// asked to stop. Returns an error only when the bus or the name cannot be
/// taken; a session without a bus is reported and exited instead of blocking.
pub fn run() -> zbus::Result<()> {
    let object = AppIndex1::new();
    let connection = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, object.clone())?
        .build()?;

    spawn_watcher(connection.clone(), object);

    // The blocking object server runs on its own executor; parking the main
    // thread keeps the process (and the name) alive without a poll loop.
    let _ = connection;
    loop {
        std::thread::park();
    }
}

/// Start the inotify watcher on the index's own `applications` directories.
/// Each monitor event re-scans and diffs the index and emits `IndexChanged`
/// for every install/uninstall/update, so the corpus stays live with no
/// polling.
fn spawn_watcher(connection: connection::Connection, object: AppIndex1) {
    let dirs = lock(&object.index).dirs().to_vec();
    let index = object.index.clone();
    let _ = std::thread::Builder::new()
        .name("dragonfruit-app-index-watch".to_owned())
        .spawn(move || {
            let _ = watch::watch_dirs(dirs, move || {
                let events = lock(&index).refresh();
                for event in events {
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
            });
        });
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
