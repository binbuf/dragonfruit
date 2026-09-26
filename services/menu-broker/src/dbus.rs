// SPDX-License-Identifier: MIT
//! The session-bus surface (T-14.2a).
//!
//! The service owns one well-known name, `org.dragonfruit.MenuBroker1`, and
//! serves the interface of the same name at
//! `/org/dragonfruit/MenuBroker1`.
//!
//! It is a two-way contract:
//!
//! * **Publishers** (first-party apps) call `Publish(appId, modelJson)` with
//!   the design-system published shape (ADR
//!   [0041](../../docs/design/adr/0041-native-menu-model-publication-shape.md))
//!   and `Withdraw(appId)` when they go away. The mediator side stays
//!   versioned and additive.
//! * **The consumer** (the shell) pushes the live window state with
//!   `SetFocusedApp` and `SetWindowStates`, then reads the resolved menu with
//!   `Resolve`/`ResolveFocused`. `Policy` exposes the fixed application menu
//!   alone (the Tier-3 projection) for the bar's fallback.
//!
//! Every query returns a flat JSON string so the Qt side decodes with
//! `QJsonDocument` and needs no `a{sv}` type knowledge. A malformed publish is
//! rejected (`false`) and never clobbers a good model; a resolve never errors.
//!
//! `Changed` is emitted after any mutation so a consumer can re-read lazily;
//! the consumer never has to poll.

use std::sync::{Arc, Mutex};

use zbus::blocking::connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;

use crate::accelerators::Dispatch;
use crate::model::Broker;

/// The well-known name on the user session bus.
pub const DBUS_NAME: &str = "org.dragonfruit.MenuBroker1";
/// The object path the interface is served at.
pub const DBUS_PATH: &str = "/org/dragonfruit/MenuBroker1";
/// The interface name.
pub const INTERFACE: &str = "org.dragonfruit.MenuBroker1";

fn lock(broker: &Arc<Mutex<Broker>>) -> std::sync::MutexGuard<'_, Broker> {
    broker
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The served object: one shared broker behind a mutex.
#[derive(Clone)]
pub struct MenuBroker1 {
    broker: Arc<Mutex<Broker>>,
}

impl Default for MenuBroker1 {
    fn default() -> Self {
        MenuBroker1::new()
    }
}

impl MenuBroker1 {
    /// A new object over an empty broker.
    pub fn new() -> Self {
        MenuBroker1::from_broker(Broker::new())
    }

    /// A new object over an explicit broker (tests, fixtures).
    pub fn from_broker(broker: Broker) -> Self {
        MenuBroker1 {
            broker: Arc::new(Mutex::new(broker)),
        }
    }

    /// The shared broker.
    pub fn broker(&self) -> &Arc<Mutex<Broker>> {
        &self.broker
    }
}

#[interface(name = "org.dragonfruit.MenuBroker1")]
impl MenuBroker1 {
    /// Publish or replace one app's menu model (the design-system shape).
    /// Returns `false` for an empty id or a malformed payload.
    async fn publish(
        &self,
        app_id: &str,
        model: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let changed = lock(&self.broker).publish(app_id, model);
        if changed {
            let _ = Self::changed(&emitter, "publish", app_id).await;
        }
        changed
    }

    /// Withdraw an app's published model. Returns whether one was present.
    async fn withdraw(
        &self,
        app_id: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let removed = lock(&self.broker).withdraw(app_id);
        if removed {
            let _ = Self::changed(&emitter, "withdraw", app_id).await;
        }
        removed
    }

    /// Set the focused app (empty clears it). Returns whether it changed.
    async fn set_focused_app(
        &self,
        app_id: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let changed = lock(&self.broker).set_focused(app_id);
        if changed {
            let _ = Self::changed(&emitter, "focus", app_id).await;
        }
        changed
    }

    /// Replace the app window-state list (`[{appId, windows, minimized}]`).
    /// Returns `false` only when the payload is not a JSON array; malformed
    /// rows are skipped.
    async fn set_window_states(
        &self,
        states: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let changed = lock(&self.broker).set_window_states(states);
        if changed {
            let _ = Self::changed(&emitter, "window-state", "").await;
        }
        changed
    }

    /// Resolve an app's menu (fixed application menu with live state + its
    /// exported top-level menus), as a JSON object.
    fn resolve(&self, app_id: &str) -> String {
        lock(&self.broker).resolve(app_id).to_string()
    }

    /// Resolve the focused app's menu (the empty desktop is Files).
    fn resolve_focused(&self) -> String {
        lock(&self.broker).resolve_focused().to_string()
    }

    /// The fixed application menu alone, with live hide-verb state.
    fn policy(&self, app_id: &str) -> String {
        lock(&self.broker).fixed_menu(app_id).to_string()
    }

    /// One app's registered accelerators, as a JSON array of
    /// `{action, chord}` (T-14.2b). Empty for an unpublished app.
    fn accelerators(&self, app_id: &str) -> String {
        let broker = lock(&self.broker);
        let rows: Vec<serde_json::Value> = broker
            .accelerators_for(app_id)
            .iter()
            .map(crate::accelerators::Accelerator::to_json)
            .collect();
        serde_json::Value::Array(rows).to_string()
    }

    /// The focused app's accelerators, as a JSON array of `{action, chord}`.
    fn focused_accelerators(&self) -> String {
        let broker = lock(&self.broker);
        let rows: Vec<serde_json::Value> = broker
            .focused_accelerators()
            .iter()
            .map(crate::accelerators::Accelerator::to_json)
            .collect();
        serde_json::Value::Array(rows).to_string()
    }

    /// Resolve a chord spec (`"Super+Q"`) against the focused app, with the
    /// reserved system chords winning. Returns `{"kind":"application",
    /// "appId":…, "action":…}`, `{"kind":"system"}`, or `{"kind":"none"}`.
    fn dispatch(&self, spec: &str) -> String {
        let dispatch = lock(&self.broker).resolve_accelerator(spec);
        match dispatch {
            Dispatch::System => serde_json::json!({ "kind": "system" }),
            Dispatch::Application { app_id, action } => {
                serde_json::json!({ "kind": "application", "appId": app_id, "action": action })
            }
            Dispatch::None => serde_json::json!({ "kind": "none" }),
        }
        .to_string()
    }

    /// Declare the reserved system chords (`["Ctrl+Right", …]`); the broker
    /// never dispatches one. Returns `false` only when the payload is not a
    /// JSON array.
    async fn set_system_accelerators(
        &self,
        chords: &str,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> bool {
        let accepted = lock(&self.broker).set_system_accelerators(chords);
        if accepted {
            let _ = Self::changed(&emitter, "system-accelerators", "").await;
        }
        accepted
    }

    /// How many apps have published a model.
    fn publisher_count(&self) -> u32 {
        lock(&self.broker).publisher_count() as u32
    }

    /// The monotonic broker revision.
    fn revision(&self) -> u64 {
        lock(&self.broker).revision()
    }

    /// A publish/withdraw/focus/window-state change landed.
    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>, reason: &str, app_id: &str) -> zbus::Result<()>;
}

/// Serve `org.dragonfruit.MenuBroker1` on the session bus until the process is
/// asked to stop. Returns an error only when the bus or the name cannot be
/// taken; a session without a bus is reported and exited instead of blocking.
pub fn run() -> zbus::Result<()> {
    let object = MenuBroker1::new();
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
