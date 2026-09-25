// SPDX-License-Identifier: MIT
//! The value model behind `org.dragonfruit.Settings1` (T-08.1a).
//!
//! A setting is one of five shapes, matching the value kinds a desktop
//! preference needs: a boolean, a floating-point number, an integer, a
//! string, or a list of strings. These map one-to-one onto the D-Bus types a
//! client sends and receives (`b`, `d`, `x`, `s`, `as`), so the wire format
//! is just [`zvariant::OwnedValue`] under the hood and no consumer ever
//! parses a home-grown encoding.
//!
//! The model is deliberately independent of D-Bus types so the schema and the
//! store stay testable without a bus; the conversions are confined to
//! [`Value::to_owned_value`] and [`Value::from_owned_value`].

use std::fmt;

use zbus::zvariant::{Array, OwnedValue, Str};

/// One setting value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Integer(i64),
    Text(String),
    TextList(Vec<String>),
}

impl Value {
    /// The key-schema type this value satisfies.
    pub const fn kind(&self) -> crate::schema::KeyType {
        use crate::schema::KeyType;
        match self {
            Value::Bool(_) => KeyType::Bool,
            Value::Number(_) => KeyType::Number,
            Value::Integer(_) => KeyType::Integer,
            Value::Text(_) => KeyType::Text,
            Value::TextList(_) => KeyType::TextList,
        }
    }

    /// The human name of the value's type, for error messages.
    pub const fn type_name(&self) -> &'static str {
        match self {
            Value::Bool(_) => "bool",
            Value::Number(_) => "double",
            Value::Integer(_) => "int64",
            Value::Text(_) => "string",
            Value::TextList(_) => "string list",
        }
    }

    /// Encode as the D-Bus variant a `Get` returns or a `Set` accepts.
    pub fn to_owned_value(&self) -> OwnedValue {
        match self {
            Value::Bool(v) => OwnedValue::from(*v),
            Value::Number(v) => OwnedValue::from(*v),
            Value::Integer(v) => OwnedValue::from(*v),
            Value::Text(v) => OwnedValue::from(Str::from(v.as_str())),
            Value::TextList(items) => {
                let values: Vec<Str<'_>> = items.iter().map(|s| Str::from(s.as_str())).collect();
                OwnedValue::try_from(Array::from(values))
                    .expect("string array converts to a variant")
            }
        }
    }

    /// Decode a D-Bus variant. Returns `None` for a type no key uses, so the
    /// caller can report the offending D-Bus signature.
    pub fn from_owned_value(value: &OwnedValue) -> Option<Value> {
        let owned = value.try_clone().ok()?;
        if let Ok(v) = bool::try_from(owned.clone()) {
            return Some(Value::Bool(v));
        }
        if let Ok(v) = f64::try_from(owned.clone()) {
            return Some(Value::Number(v));
        }
        if let Ok(v) = i64::try_from(owned.clone()) {
            return Some(Value::Integer(v));
        }
        if let Ok(v) = String::try_from(owned.clone()) {
            return Some(Value::Text(v));
        }
        if let Ok(v) = Vec::<String>::try_from(owned) {
            return Some(Value::TextList(v));
        }
        None
    }
}

/// Why a `get`/`set` was rejected. Every variant maps to one D-Bus error
/// name under `org.dragonfruit.Settings1.Error` so clients can branch on it.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingsError {
    /// No key with that name is in the schema.
    UnknownKey(String),
    /// The value's type does not match the key's declared type.
    TypeMismatch {
        key: String,
        expected: &'static str,
        got: &'static str,
    },
    /// A numeric key's value is outside its declared range.
    OutOfRange {
        key: String,
        value: f64,
        min: f64,
        max: f64,
    },
    /// A text key's value is not one of its declared alternatives.
    NotAllowed {
        key: String,
        value: String,
        allowed: &'static [&'static str],
    },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SettingsError::UnknownKey(key) => write!(f, "unknown settings key {key:?}"),
            SettingsError::TypeMismatch { key, expected, got } => {
                write!(f, "{key}: expected {expected}, got {got}")
            }
            SettingsError::OutOfRange {
                key,
                value,
                min,
                max,
            } => write!(f, "{key}: {value} is outside {min}..={max}"),
            SettingsError::NotAllowed {
                key,
                value,
                allowed,
            } => write!(f, "{key}: {value:?} is not one of {allowed:?}"),
        }
    }
}

impl std::error::Error for SettingsError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(value: Value) {
        let owned = value.to_owned_value();
        assert_eq!(Value::from_owned_value(&owned), Some(value));
    }

    #[test]
    fn every_value_shape_round_trips_through_a_dbus_variant() {
        round_trip(Value::Bool(true));
        round_trip(Value::Number(0.5));
        round_trip(Value::Integer(25));
        round_trip(Value::Text("dark".to_owned()));
        round_trip(Value::TextList(vec!["a".to_owned(), "b".to_owned()]));
    }

    #[test]
    fn an_unused_dbus_type_decodes_to_none() {
        let owned = OwnedValue::from(7u32);
        assert_eq!(Value::from_owned_value(&owned), None);
    }

    #[test]
    fn error_messages_name_the_key_and_the_problem() {
        let error = SettingsError::UnknownKey("dock.nope".to_owned());
        assert!(error.to_string().contains("dock.nope"));
        let error = SettingsError::TypeMismatch {
            key: "dock.autohide".to_owned(),
            expected: "bool",
            got: "string",
        };
        assert_eq!(
            error.to_string(),
            "dock.autohide: expected bool, got string"
        );
    }
}
