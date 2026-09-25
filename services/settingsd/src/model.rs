// SPDX-License-Identifier: MIT
//! The in-memory desktop-settings store (T-08.1a).
//!
//! [`Settings`] holds the current value of every key in [`crate::schema::KEYS`]
//! and enforces the schema on every write: unknown keys, wrong types, values
//! outside a range, and text outside an enumerated set are all rejected before
//! anything changes. A write that changes a value reports the new value so the
//! D-Bus layer can emit exactly one `Changed` signal; a write that repeats the
//! current value is a silent no-op, which is what lets consumers treat every
//! signal as a real change.
//!
//! Persistence is intentionally absent here. T-08.1b owns the on-disk format,
//! atomic writes, and startup migrations; this store is the live state they
//! will save and restore.

use std::collections::BTreeMap;

use crate::schema::{self, KeySpec, KEYS};
use crate::value::{SettingsError, Value};

/// The current value of every settings key.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    values: BTreeMap<&'static str, Value>,
}

impl Settings {
    /// A store seeded with every schema default.
    pub fn new() -> Self {
        let values = KEYS
            .iter()
            .map(|spec| (spec.key, spec.default.to_value()))
            .collect();
        Settings { values }
    }

    /// Read a key.
    pub fn get(&self, key: &str) -> Result<&Value, SettingsError> {
        self.values
            .get(key)
            .ok_or_else(|| SettingsError::UnknownKey(key.to_owned()))
    }

    /// Write a key. Returns `Some(new_value)` when the value changed and
    /// `None` when it was already equal; both are success. An invalid write
    /// leaves the store untouched.
    pub fn set(&mut self, key: &str, value: Value) -> Result<Option<Value>, SettingsError> {
        let spec = schema::spec(key).ok_or_else(|| SettingsError::UnknownKey(key.to_owned()))?;
        spec.validate(&value)?;
        let current = self
            .values
            .get(key)
            .expect("the schema seeds every key at construction");
        if current == &value {
            return Ok(None);
        }
        self.values.insert(spec.key, value.clone());
        Ok(Some(value))
    }

    /// The live value of every key, for a resync snapshot.
    pub fn snapshot(&self) -> Vec<(&'static str, Value)> {
        self.values
            .iter()
            .map(|(key, value)| (*key, value.clone()))
            .collect()
    }

    /// The declaration of a key, if it exists.
    pub fn spec(&self, key: &str) -> Option<&'static KeySpec> {
        schema::spec(key)
    }

    /// Drop all values and reseed the schema defaults (a full reset).
    pub fn reset(&mut self) {
        *self = Settings::new();
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_store_holds_every_schema_default() {
        let settings = Settings::new();
        for spec in KEYS {
            assert_eq!(
                settings.get(spec.key).unwrap(),
                &spec.default.to_value(),
                "{} is not seeded with its default",
                spec.key
            );
        }
        assert_eq!(
            settings.get("appearance.colorScheme").unwrap(),
            &Value::Text("auto".into())
        );
    }

    #[test]
    fn a_changing_write_reports_the_new_value_and_a_repeat_is_silent() {
        let mut settings = Settings::new();
        assert_eq!(
            settings.set("appearance.colorScheme", Value::Text("dark".into())),
            Ok(Some(Value::Text("dark".into())))
        );
        assert_eq!(
            settings.get("appearance.colorScheme").unwrap(),
            &Value::Text("dark".into())
        );
        assert_eq!(
            settings.set("appearance.colorScheme", Value::Text("dark".into())),
            Ok(None)
        );
    }

    #[test]
    fn an_invalid_write_leaves_the_store_untouched() {
        let mut settings = Settings::new();
        let before = settings.clone();
        assert!(settings.set("dock.nope", Value::Bool(true)).is_err());
        assert!(settings
            .set("dock.autohide", Value::Text("yes".into()))
            .is_err());
        assert!(settings.set("dock.size", Value::Number(9.0)).is_err());
        assert_eq!(settings, before);
    }

    #[test]
    fn the_snapshot_has_one_entry_per_key_and_reset_restores_defaults() {
        let mut settings = Settings::new();
        settings.set("workspaces.count", Value::Integer(5)).unwrap();
        assert_eq!(settings.snapshot().len(), KEYS.len());
        assert_eq!(
            settings.get("workspaces.count").unwrap(),
            &Value::Integer(5)
        );
        settings.reset();
        assert_eq!(
            settings.get("workspaces.count").unwrap(),
            &Value::Integer(3)
        );
    }
}
