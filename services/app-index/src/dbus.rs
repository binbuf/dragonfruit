// SPDX-License-Identifier: MIT
//! The session-bus surface (T-14.1a).
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
//! The surface is deliberately minimal for T-14.1a — identity resolution and
//! themed icons. Install/uninstall/update events and the launch registry are
//! T-14.1b; the subscription API is T-14.1c. Methods are additive-only.

use std::sync::{Arc, Mutex};

use zbus::blocking::connection;
use zbus::interface;

use crate::icons::IconTheme;
use crate::index::AppIndex;
use crate::view;

/// The well-known name on the user session bus.
pub const DBUS_NAME: &str = "org.dragonfruit.AppIndex1";
/// The object path the interface is served at.
pub const DBUS_PATH: &str = "/org/dragonfruit/AppIndex1";
/// The interface name.
pub const INTERFACE: &str = "org.dragonfruit.AppIndex1";

/// The served object: one shared index behind a mutex, plus the icon theme.
#[derive(Clone)]
pub struct AppIndex1 {
    index: Arc<Mutex<AppIndex>>,
    theme: IconTheme,
}

impl AppIndex1 {
    /// A new object over a freshly scanned index and the environment's icon
    /// theme.
    pub fn new() -> Self {
        Self::from_index(AppIndex::load(), IconTheme::from_env())
    }

    /// A new object over an explicit index and theme (tests, fixtures).
    pub fn from_index(index: AppIndex, theme: IconTheme) -> Self {
        AppIndex1 {
            index: Arc::new(Mutex::new(index)),
            theme,
        }
    }

    /// The shared index.
    pub fn index(&self) -> &Arc<Mutex<AppIndex>> {
        &self.index
    }

    /// The icon theme in use.
    pub fn theme(&self) -> &IconTheme {
        &self.theme
    }
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
}

/// Serve `org.dragonfruit.AppIndex1` on the session bus until the process is
/// asked to stop. Returns an error only when the bus or the name cannot be
/// taken; a session without a bus is reported and exited instead of blocking.
pub fn run() -> zbus::Result<()> {
    let object = AppIndex1::new();
    let connection = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, object)?
        .build()?;

    // The blocking object server runs on its own executor; parking the main
    // thread keeps the process (and the name) alive without a poll loop.
    let _ = connection;
    loop {
        std::thread::park();
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
