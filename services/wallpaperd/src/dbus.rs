// SPDX-License-Identifier: MIT
//! The session-bus surface: `org.dragonfruit.Wallpaper1` (T-18.1a).
//!
//! One interface at `/org/dragonfruit/Wallpaper1`:
//!
//! * Properties `Status` (`idle|fetching|ready|offline`), `LastFetch`,
//!   `DefaultSource` (the fetched Featured default/fallback),
//!   `BuiltinDefaultSource` (the resolved shipped `Default.jpg`), and `Items`
//!   (the cached catalogue as JSON).
//! * Methods `Refresh` (explicit, user-driven) and `Preload` (eager foreground
//!   load for an open Wallpapers pane); both refresh only when the cache is
//!   empty or stale.
//! * Signals `ItemsChanged` and `StatusChanged`.
//!
//! T-18.1b wires System Settings and the shell to this interface. The service
//! stays lazy without a `Preload`: [`run`] spawns one low-priority background
//! warm and then only answers.

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use zbus::blocking::{connection, Connection};
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedValue, Value};

use crate::provider::{now_secs, RefreshReport, ServiceState};

/// The stable well-known name on the user session bus.
pub const DBUS_NAME: &str = "org.dragonfruit.Wallpaper1";
/// The object path the interface is served at.
pub const DBUS_PATH: &str = "/org/dragonfruit/Wallpaper1";
/// The interface name.
pub const INTERFACE: &str = "org.dragonfruit.Wallpaper1";

/// The delay before the background warm starts, so session bring-up settles
/// first (the design notes ask for low priority, off the frame path).
pub const WARM_DELAY: Duration = Duration::from_millis(250);

/// The `org.dragonfruit.Wallpaper1` object.
#[derive(Clone)]
pub struct Wallpaper1 {
    state: Arc<ServiceState>,
}

impl Wallpaper1 {
    /// An object over `state`.
    pub fn new(state: Arc<ServiceState>) -> Self {
        Wallpaper1 { state }
    }

    /// The shared state, for tests.
    pub fn state(&self) -> &Arc<ServiceState> {
        &self.state
    }
}

#[interface(name = "org.dragonfruit.Wallpaper1")]
impl Wallpaper1 {
    /// The provider lifecycle status: `idle`, `fetching`, `ready`, `offline`.
    #[zbus(property)]
    fn status(&self) -> String {
        self.state.status_str()
    }

    /// Unix seconds of the last successful refresh (0 = never).
    #[zbus(property)]
    fn last_fetch(&self) -> u64 {
        self.state.last_fetch()
    }

    /// The fetched Featured default/fallback local path, or `""`.
    #[zbus(property)]
    fn default_source(&self) -> String {
        self.state.default_source()
    }

    /// The resolved shipped `Default.jpg` local path, or `""`.
    #[zbus(property)]
    fn builtin_default_source(&self) -> String {
        self.state.builtin_default_source()
    }

    /// The cached catalogue as a JSON array.
    #[zbus(property)]
    fn items(&self) -> String {
        self.state.items_json()
    }

    /// Explicit, user-driven refresh. Returns the JSON snapshot.
    async fn refresh(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        let report = self.state.refresh_if_needed(now_secs());
        emit_report(self, &emitter, report).await;
        self.state.snapshot_json()
    }

    /// Eager foreground catalogue load for an open Wallpapers pane. Without
    /// it the service stays lazy. Returns the JSON snapshot.
    async fn preload(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        let report = self.state.refresh_if_needed(now_secs());
        emit_report(self, &emitter, report).await;
        self.state.snapshot_json()
    }

    /// The catalogue changed (a first fill, or a refreshed set). The Rust method
    /// name avoids the property-changed helper zbus generates for `Items`.
    #[zbus(name = "ItemsChanged", signal)]
    async fn notify_items_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    /// The provider status changed.
    #[zbus(name = "StatusChanged", signal)]
    async fn notify_status_changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// Emit the signals a refresh report calls for.
async fn emit_report(iface: &Wallpaper1, emitter: &SignalEmitter<'_>, report: RefreshReport) {
    if report.status_changed {
        // The standard property notification keeps a caching proxy in sync ...
        let _ = iface.status_changed(emitter).await;
        // ... and the contract signal notifies a consumer directly.
        let _ = Wallpaper1::notify_status_changed(emitter).await;
    }
    if report.items_changed {
        let _ = iface.items_changed(emitter).await;
        let _ = Wallpaper1::notify_items_changed(emitter).await;
    }
}

/// Emit the change signals on a blocking connection (the background warm
/// thread's path): the custom signals plus the standard `PropertiesChanged`
/// so a caching consumer is invalidated.
fn emit_report_blocking(connection: &Connection, state: &ServiceState, report: RefreshReport) {
    if report.status_changed {
        emit_properties_changed_blocking(connection, &[("Status", state.status_str())]);
        let _ = connection.emit_signal(None::<&str>, DBUS_PATH, INTERFACE, "StatusChanged", &());
    }
    if report.items_changed {
        emit_properties_changed_blocking(connection, &[("Items", state.items_json())]);
        let _ = connection.emit_signal(None::<&str>, DBUS_PATH, INTERFACE, "ItemsChanged", &());
    }
}

/// Emit `org.freedesktop.DBus.Properties.PropertiesChanged` for `props`.
fn emit_properties_changed_blocking(connection: &Connection, props: &[(&str, String)]) {
    let mut changed: HashMap<String, OwnedValue> = HashMap::new();
    for (name, value) in props {
        if let Ok(owned) = OwnedValue::try_from(Value::from(value.clone())) {
            changed.insert((*name).to_owned(), owned);
        }
    }
    let _ = connection.emit_signal(
        None::<&str>,
        DBUS_PATH,
        "org.freedesktop.DBus.Properties",
        "PropertiesChanged",
        &(INTERFACE, changed, Vec::<String>::new()),
    );
}

/// Serve `org.dragonfruit.Wallpaper1` on the session bus until the process is
/// asked to stop, starting one lazy background warm.
///
/// Returns an error only when the bus or the name cannot be taken; an absent
/// session bus is reported and exited instead of blocking a session.
pub fn run(state: ServiceState) -> zbus::Result<()> {
    let state = Arc::new(state);
    let connection = connection::Builder::session()?
        .name(DBUS_NAME)?
        .serve_at(DBUS_PATH, Wallpaper1::new(Arc::clone(&state)))?
        .build()?;

    spawn_warm(connection.clone(), state);

    // The blocking object server runs on its own executor; parking the main
    // thread keeps the process (and the name) alive without a poll loop.
    let _ = connection;
    loop {
        thread::park();
    }
}

/// Start the one lazy background warm: wait for bring-up, then refresh only if
/// the cache is empty or stale, emitting the change signals when it lands.
pub fn spawn_warm(connection: Connection, state: Arc<ServiceState>) {
    let _ = thread::Builder::new()
        .name("dragonfruit-wallpaperd-warm".to_owned())
        .spawn(move || {
            thread::sleep(WARM_DELAY);
            let report = state.refresh_if_needed(now_secs());
            emit_report_blocking(&connection, &state, report);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interface_constants_are_the_contract() {
        assert_eq!(DBUS_NAME, "org.dragonfruit.Wallpaper1");
        assert_eq!(DBUS_PATH, "/org/dragonfruit/Wallpaper1");
        assert_eq!(INTERFACE, "org.dragonfruit.Wallpaper1");
    }
}
