// SPDX-License-Identifier: MIT
//! The session-bus surface: `org.dragonfruit.Settings1` (T-08.1a).
//!
//! One interface, one object path. Methods are typed against the schema:
//!
//! * `Get(key) -> v` — the current value as a D-Bus variant.
//! * `Set(key, value: v)` — validate against the schema, store, persist
//!   (when the object has a [`Persistence`]), and emit `Changed(key, value)`
//!   **only when the value actually changed**.
//! * `GetAll() -> a{sv}` — a resync snapshot for a client that (re)appeared.
//! * `ListKeys() -> as` — the schema's key names, in stable order.
//! * `SchemaVersion` property (`u`) — the persisted-schema revision.
//! * `Changed(key, value)` signal — the only notification; nothing polls.
//!
//! Rejections are typed D-Bus errors under `org.dragonfruit.Settings1.Error`
//! (`UnknownKey`, `TypeMismatch`, `OutOfRange`, `NotAllowed`) so a client can
//! tell a bad key from a bad value.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use zbus::blocking::connection;
use zbus::interface;
use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use crate::model::Settings;
use crate::persist::Persistence;
use crate::schema::SCHEMA_VERSION;
use crate::value::{SettingsError, Value};

/// The stable well-known name on the user session bus.
pub const DBUS_NAME: &str = "org.dragonfruit.Settings1";
/// The object path the interface is served at.
pub const DBUS_PATH: &str = "/org/dragonfruit/Settings1";

/// The typed errors `get`/`set` can raise.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.dragonfruit.Settings1.Error")]
pub enum Settings1Error {
    /// The key is not in the schema.
    UnknownKey(String),
    /// The value's type does not match the key's declared type.
    TypeMismatch(String),
    /// A numeric value is outside the key's declared range.
    OutOfRange(String),
    /// A text value is not one of the key's declared alternatives.
    NotAllowed(String),
    #[zbus(error)]
    ZBus(zbus::Error),
}

impl From<SettingsError> for Settings1Error {
    fn from(error: SettingsError) -> Self {
        match error {
            SettingsError::UnknownKey(message) => Settings1Error::UnknownKey(message),
            SettingsError::TypeMismatch { .. } => Settings1Error::TypeMismatch(error.to_string()),
            SettingsError::OutOfRange { .. } => Settings1Error::OutOfRange(error.to_string()),
            SettingsError::NotAllowed { .. } => Settings1Error::NotAllowed(error.to_string()),
        }
    }
}

/// The `org.dragonfruit.Settings1` object.
#[derive(Clone)]
pub struct Settings1 {
    settings: Arc<Mutex<Settings>>,
    /// The on-disk writer. `None` for an in-memory object (tests, a session
    /// with no config directory); `Some` when `Set` must persist.
    persistence: Option<Arc<Persistence>>,
}

impl Settings1 {
    /// A new object over a fresh store.
    pub fn new(settings: Settings) -> Self {
        Settings1 {
            settings: Arc::new(Mutex::new(settings)),
            persistence: None,
        }
    }

    /// A new object sharing an existing store (for tests and future clients
    /// in the same process).
    pub fn from_shared(settings: Arc<Mutex<Settings>>) -> Self {
        Settings1 {
            settings,
            persistence: None,
        }
    }

    /// A new object that persists every real change through `persistence`.
    pub fn with_persistence(settings: Settings, persistence: Persistence) -> Self {
        Settings1 {
            settings: Arc::new(Mutex::new(settings)),
            persistence: Some(Arc::new(persistence)),
        }
    }

    /// The shared store behind this object.
    pub fn store(&self) -> &Arc<Mutex<Settings>> {
        &self.settings
    }

    /// The on-disk writer, when this object was created with one.
    pub fn persistence(&self) -> Option<&Arc<Persistence>> {
        self.persistence.as_ref()
    }
}

#[interface(name = "org.dragonfruit.Settings1")]
impl Settings1 {
    /// The current value of a key, as a D-Bus variant.
    fn get(&self, key: &str) -> Result<OwnedValue, Settings1Error> {
        let settings = lock(&self.settings);
        settings
            .get(key)
            .map(Value::to_owned_value)
            .map_err(Settings1Error::from)
    }

    /// Set a key from a D-Bus variant. Emits `Changed` only on a real change.
    async fn set(
        &self,
        key: &str,
        value: OwnedValue,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Settings1Error> {
        let value = Value::from_owned_value(&value).ok_or_else(|| {
            Settings1Error::TypeMismatch(format!(
                "{key}: unsupported D-Bus type {}",
                value.value_signature()
            ))
        })?;
        let changed = {
            let mut settings = lock(&self.settings);
            settings.set(key, value).map_err(Settings1Error::from)?
        };
        if let Some(new_value) = changed {
            // Persist before signalling: a consumer that reacts to `Changed`
            // can rely on the durable file already holding the new value. A
            // disk failure must not take the live daemon down, so it is
            // reported and the in-memory value stays authoritative.
            if let Some(persistence) = &self.persistence {
                let snapshot = lock(&self.settings).clone();
                if let Err(error) = persistence.save(&snapshot) {
                    eprintln!(
                        "dragonfruit-settingsd: cannot persist settings to {}: {error}",
                        persistence.path().display()
                    );
                }
            }
            Settings1::changed(&emitter, key, new_value.to_owned_value()).await?;
        }
        Ok(())
    }

    /// Every key's current value, for a client that just (re)appeared.
    fn get_all(&self) -> HashMap<String, OwnedValue> {
        lock(&self.settings)
            .snapshot()
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value.to_owned_value()))
            .collect()
    }

    /// The schema's key names, in stable order.
    fn list_keys(&self) -> Vec<String> {
        crate::schema::key_names()
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    /// The persisted-schema revision (T-08.1b reads it from disk).
    #[zbus(property(emits_changed_signal = "const"))]
    fn schema_version(&self) -> u32 {
        SCHEMA_VERSION
    }

    /// A key changed to a new value. This is the only notification mechanism;
    /// clients never poll.
    #[zbus(signal)]
    async fn changed(emitter: &SignalEmitter<'_>, key: &str, value: OwnedValue)
        -> zbus::Result<()>;
}

/// Lock the shared store, recovering from a poisoned mutex: a D-Bus method
/// may panic on a bad argument and the service must keep answering.
fn lock(settings: &Arc<Mutex<Settings>>) -> std::sync::MutexGuard<'_, Settings> {
    settings
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Serve the interface on the session bus until the process is asked to stop.
/// `persistence` is `Some` when a config path was resolved; every real `Set`
/// is then written to disk before its signal. Returns an error only when the
/// bus or the name cannot be taken; a session without a bus is not this
/// daemon's problem to block on.
pub fn run(settings: Settings, persistence: Option<Persistence>) -> zbus::Result<()> {
    let object = match persistence {
        Some(persistence) => Settings1::with_persistence(settings, persistence),
        None => Settings1::new(settings),
    };
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
    fn from_a_settings_error_the_typed_name_survives() {
        let error = Settings1Error::from(SettingsError::UnknownKey("dock.nope".to_owned()));
        assert!(matches!(error, Settings1Error::UnknownKey(ref key) if key == "dock.nope"));
        let error = Settings1Error::from(SettingsError::OutOfRange {
            key: "dock.size".to_owned(),
            value: 2.0,
            min: 0.0,
            max: 1.0,
        });
        assert!(matches!(error, Settings1Error::OutOfRange(_)));
    }

    #[test]
    fn the_interface_names_follow_the_dbus_policy() {
        assert!(df_ipc::is_valid_dbus_name(DBUS_NAME));
        assert_eq!(DBUS_NAME, "org.dragonfruit.Settings1");
        assert!(DBUS_PATH.starts_with('/'));
    }
}
