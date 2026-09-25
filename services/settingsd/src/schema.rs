// SPDX-License-Identifier: MIT
//! The `org.dragonfruit.Settings1` key schema (T-08.1a).
//!
//! Every desktop-settings key this daemon owns is declared exactly once in
//! [`KEYS`], with its D-Bus type, default, allowed values or numeric range,
//! and the **owner** (which component writes the key) and **consumer**
//! (which components read it). The table is the in-code key documentation
//! required by T-08.1a and the frozen contract T-08.1b and T-08.2 build on.
//!
//! # Additive-only within a release
//!
//! Keys may be added, never renamed or removed. [`SCHEMA_VERSION`] is the
//! persisted-schema revision; T-08.1b writes it as the `schema` field of
//! `$XDG_CONFIG_HOME/dragonfruit/settings.json` and migrates older files up
//! to it at startup. A key added in a later revision carries its `since`
//! revision so the migration can fill its default.

use crate::value::{SettingsError, Value};

/// The current schema revision. Bump only when a key is added or a default
/// changes; renames and removals are forbidden within the `1` series.
pub const SCHEMA_VERSION: u32 = 4;

/// The D-Bus type of a settings value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    Bool,
    Number,
    Integer,
    Text,
    TextList,
}

impl KeyType {
    /// The D-Bus signature this type travels as.
    pub const fn signature(self) -> &'static str {
        match self {
            KeyType::Bool => "b",
            KeyType::Number => "d",
            KeyType::Integer => "x",
            KeyType::Text => "s",
            KeyType::TextList => "as",
        }
    }

    /// A human name for docs and error messages.
    pub const fn name(self) -> &'static str {
        match self {
            KeyType::Bool => "bool",
            KeyType::Number => "double",
            KeyType::Integer => "int64",
            KeyType::Text => "string",
            KeyType::TextList => "string list",
        }
    }
}

/// The provider group a key belongs to. Consumers subscribe per key, but the
/// Settings app and the docs are organized by group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyGroup {
    Dock,
    Workspaces,
    Gestures,
    Appearance,
    Wallpaper,
    Displays,
    Animation,
    Input,
}

impl KeyGroup {
    /// Every group, in schema order.
    pub const ALL: [KeyGroup; 8] = [
        KeyGroup::Dock,
        KeyGroup::Workspaces,
        KeyGroup::Gestures,
        KeyGroup::Appearance,
        KeyGroup::Wallpaper,
        KeyGroup::Displays,
        KeyGroup::Animation,
        KeyGroup::Input,
    ];

    /// The group name used in docs and tests.
    pub const fn name(self) -> &'static str {
        match self {
            KeyGroup::Dock => "dock",
            KeyGroup::Workspaces => "workspaces",
            KeyGroup::Gestures => "gestures",
            KeyGroup::Appearance => "appearance",
            KeyGroup::Wallpaper => "wallpaper",
            KeyGroup::Displays => "displays",
            KeyGroup::Animation => "animation",
            KeyGroup::Input => "input",
        }
    }
}

/// A key's declared default, in a form a `const` table can hold.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KeyDefault {
    Bool(bool),
    Number(f64),
    Integer(i64),
    Text(&'static str),
    List(&'static [&'static str]),
}

impl KeyDefault {
    /// The default as a runtime value.
    pub fn to_value(self) -> Value {
        match self {
            KeyDefault::Bool(v) => Value::Bool(v),
            KeyDefault::Number(v) => Value::Number(v),
            KeyDefault::Integer(v) => Value::Integer(v),
            KeyDefault::Text(v) => Value::Text(v.to_owned()),
            KeyDefault::List(items) => {
                Value::TextList(items.iter().map(|s| (*s).to_owned()).collect())
            }
        }
    }
}

/// One key's full declaration: type, default, constraints, and the
/// owner/consumer pair that makes ownership explicit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeySpec {
    /// The dotted, namespaced key name (`dock.size`).
    pub key: &'static str,
    pub group: KeyGroup,
    pub kind: KeyType,
    pub default: KeyDefault,
    /// The exhaustive alternatives for an enumerated text key; empty for
    /// free-form text.
    pub allowed: &'static [&'static str],
    /// Inclusive lower bound for numeric keys.
    pub min: Option<f64>,
    /// Inclusive upper bound for numeric keys.
    pub max: Option<f64>,
    /// The component that writes this key.
    pub owner: &'static str,
    /// The components that read this key and react to its signal.
    pub consumer: &'static str,
    /// The schema revision that introduced the key.
    pub since: u32,
    /// One-line description for the docs and the Settings app.
    pub summary: &'static str,
}

impl KeySpec {
    /// Check a value against the declared type, range, and alternatives.
    pub fn validate(&self, value: &Value) -> Result<(), SettingsError> {
        if value.kind() != self.kind {
            return Err(SettingsError::TypeMismatch {
                key: self.key.to_owned(),
                expected: self.kind.name(),
                got: value.type_name(),
            });
        }
        match value {
            Value::Number(n) => self.check_range(*n),
            Value::Integer(i) => self.check_range(*i as f64),
            Value::Text(text) if !self.allowed.is_empty() => {
                if !self.allowed.contains(&text.as_str()) {
                    return Err(SettingsError::NotAllowed {
                        key: self.key.to_owned(),
                        value: text.clone(),
                        allowed: self.allowed,
                    });
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn check_range(&self, number: f64) -> Result<(), SettingsError> {
        let out_of_range =
            self.min.is_some_and(|min| number < min) || self.max.is_some_and(|max| number > max);
        if out_of_range {
            return Err(SettingsError::OutOfRange {
                key: self.key.to_owned(),
                value: number,
                min: self.min.unwrap_or(f64::MIN),
                max: self.max.unwrap_or(f64::MAX),
            });
        }
        Ok(())
    }
}

/// The complete desktop-settings schema, in stable key order. See the module
/// docs for the additive-only rule.
pub const KEYS: &[KeySpec] = &[
    // ── Dock ────────────────────────────────────────────────────────────
    KeySpec {
        key: "dock.size",
        group: KeyGroup::Dock,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.5),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Dock icon scale, 0 (small) to 1 (large).",
    },
    KeySpec {
        key: "dock.magnification",
        group: KeyGroup::Dock,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.5),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Magnification strength under the pointer; 0 disables it.",
    },
    KeySpec {
        key: "dock.position",
        group: KeyGroup::Dock,
        kind: KeyType::Text,
        default: KeyDefault::Text("bottom"),
        allowed: &["bottom", "left", "right"],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Screen edge the Dock sits on.",
    },
    KeySpec {
        key: "dock.autohide",
        group: KeyGroup::Dock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Hide the Dock off-edge until the pointer reaches it.",
    },
    KeySpec {
        key: "dock.animateOpening",
        group: KeyGroup::Dock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Bounce a Dock tile when it launches an app.",
    },
    KeySpec {
        key: "dock.showIndicators",
        group: KeyGroup::Dock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Show the running-application dot under a tile.",
    },
    KeySpec {
        key: "dock.minimizeIntoTileIcon",
        group: KeyGroup::Dock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock, compositor/window motion",
        since: 1,
        summary: "Minimize into the app's tile instead of a separate entry.",
    },
    KeySpec {
        key: "dock.minimizedAnimation",
        group: KeyGroup::Dock,
        kind: KeyType::Text,
        default: KeyDefault::Text("scale"),
        allowed: &["genie", "scale", "none"],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "compositor/window motion",
        since: 1,
        summary: "Minimize/restore animation style.",
    },
    KeySpec {
        key: "dock.titlebarDoubleClick",
        group: KeyGroup::Dock,
        kind: KeyType::Text,
        default: KeyDefault::Text("zoom"),
        allowed: &["zoom", "minimize", "none"],
        min: None,
        max: None,
        owner: "settingsd",
        consumer: "compositor/window decoration (SSD)",
        since: 1,
        summary: "Action on a titlebar double-click.",
    },
    KeySpec {
        key: "dock.showRecentApps",
        group: KeyGroup::Dock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Show recent/suggested apps in the Dock.",
    },
    KeySpec {
        key: "dock.pinned",
        group: KeyGroup::Dock,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 1,
        summary: "Ordered desktop ids pinned to the Dock; empty seeds defaults.",
    },
    // ── Workspaces ──────────────────────────────────────────────────────
    KeySpec {
        key: "workspaces.count",
        group: KeyGroup::Workspaces,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(3),
        allowed: &[],
        min: Some(1.0),
        max: Some(16.0),
        owner: "settingsd",
        consumer: "compositor/workspace model, shell",
        since: 1,
        summary: "Number of Spaces every output starts with.",
    },
    // ── Gestures ────────────────────────────────────────────────────────
    KeySpec {
        key: "gestures.enabled",
        group: KeyGroup::Gestures,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "settingsd",
        consumer: "compositor/input",
        since: 1,
        summary: "Master switch for trackpad gesture recognition.",
    },
    KeySpec {
        key: "gestures.spaceSwitch",
        group: KeyGroup::Gestures,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "settingsd",
        consumer: "compositor/input",
        since: 1,
        summary: "Horizontal swipe switches Spaces.",
    },
    KeySpec {
        key: "gestures.missionControl",
        group: KeyGroup::Gestures,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "settingsd",
        consumer: "compositor/input",
        since: 1,
        summary: "Vertical swipe opens Mission Control.",
    },
    // ── Appearance ──────────────────────────────────────────────────────
    KeySpec {
        key: "appearance.colorScheme",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text("auto"),
        allowed: &["light", "dark", "auto"],
        min: None,
        max: None,
        owner: "settingsd",
        consumer: "compositor/window decoration, shell/design-system Theme",
        since: 1,
        summary: "Light/dark scheme; auto follows the host style hint.",
    },
    KeySpec {
        key: "appearance.accent",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "settingsd",
        consumer: "shell/design-system Theme",
        since: 1,
        summary: "Accent color override ('#rrggbb'); empty uses the token default.",
    },
    // ── Wallpaper ───────────────────────────────────────────────────────
    KeySpec {
        key: "wallpaper.source",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/wallpaper forwarder, compositor/workspace model",
        since: 2,
        summary: "Image path for the selected wallpaper; empty keeps the solid color.",
    },
    KeySpec {
        key: "wallpaper.fit",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Text,
        default: KeyDefault::Text("fill"),
        allowed: &["fill", "fit", "stretch", "center"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/wallpaper forwarder, compositor/workspace model",
        since: 2,
        summary: "How the wallpaper image maps onto the output.",
    },
    KeySpec {
        key: "wallpaper.showOnAllSpaces",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/wallpaper forwarder, compositor/workspace model",
        since: 2,
        summary: "Apply the selection to every Space, or only the active one.",
    },
    // ── Displays ────────────────────────────────────────────────────────
    KeySpec {
        key: "display.scale",
        group: KeyGroup::Displays,
        kind: KeyType::Number,
        default: KeyDefault::Number(1.0),
        allowed: &[],
        min: Some(0.5),
        max: Some(2.0),
        owner: "apps/settings",
        consumer: "shell/display forwarder, compositor/output",
        since: 3,
        summary: "Output scale / scaled-resolution factor; 1.0 is the native mode.",
    },
    KeySpec {
        key: "display.rotation",
        group: KeyGroup::Displays,
        kind: KeyType::Text,
        default: KeyDefault::Text("normal"),
        allowed: &["normal", "90", "180", "270"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/display forwarder, compositor/output",
        since: 3,
        summary: "Output rotation as clock-wise degrees.",
    },
    KeySpec {
        key: "display.brightness",
        group: KeyGroup::Displays,
        kind: KeyType::Number,
        default: KeyDefault::Number(1.0),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "shell/control-center",
        consumer: "shell/display forwarder, compositor/output",
        since: 4,
        summary: "Output brightness level; 1.0 is full brightness.",
    },
    // ── Animation policy ────────────────────────────────────────────────
    KeySpec {
        key: "accessibility.reduceMotion",
        group: KeyGroup::Animation,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "settingsd",
        consumer: "shell/design-system Theme, compositor/window motion",
        since: 1,
        summary: "Global animation policy: collapse motion to instant transitions.",
    },
    // ── Input ───────────────────────────────────────────────────────────
    KeySpec {
        key: "input.repeatDelay",
        group: KeyGroup::Input,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(200),
        allowed: &[],
        min: Some(0.0),
        max: Some(5000.0),
        owner: "settingsd",
        consumer: "compositor/input keyboard repeat",
        since: 1,
        summary: "Milliseconds before a held key begins repeating.",
    },
    KeySpec {
        key: "input.repeatRate",
        group: KeyGroup::Input,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(25),
        allowed: &[],
        min: Some(0.0),
        max: Some(200.0),
        owner: "settingsd",
        consumer: "compositor/input keyboard repeat",
        since: 1,
        summary: "Key repeat rate in keys per second; 0 disables repeat.",
    },
];

/// Look up a key's declaration.
pub fn spec(key: &str) -> Option<&'static KeySpec> {
    KEYS.iter().find(|spec| spec.key == key)
}

/// Every key name, in schema order.
pub fn key_names() -> Vec<&'static str> {
    KEYS.iter().map(|spec| spec.key).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_fully_documented_and_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for spec in KEYS {
            assert!(seen.insert(spec.key), "duplicate key {}", spec.key);
            assert!(!spec.owner.is_empty(), "{} has no owner", spec.key);
            assert!(!spec.consumer.is_empty(), "{} has no consumer", spec.key);
            assert!(!spec.summary.is_empty(), "{} has no summary", spec.key);
            assert!(
                spec.since <= SCHEMA_VERSION,
                "{} is from the future",
                spec.key
            );
            assert_eq!(
                spec.default.to_value().kind(),
                spec.kind,
                "{} default kind disagrees with its declaration",
                spec.key
            );
            assert!(spec.key.contains('.'), "{} is not namespaced", spec.key);
        }
        for group in KeyGroup::ALL {
            assert!(
                KEYS.iter().any(|spec| spec.group == group),
                "group {} has no keys",
                group.name()
            );
        }
    }

    #[test]
    fn every_declared_default_validates() {
        for spec in KEYS {
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{} default fails its own validation",
                spec.key
            );
        }
    }

    #[test]
    fn lookup_finds_keys_and_rejects_unknown_ones() {
        assert_eq!(spec("dock.size").map(|s| s.key), Some("dock.size"));
        assert!(spec("dock.nope").is_none());
        assert_eq!(key_names().len(), KEYS.len());
    }

    #[test]
    fn type_range_and_enum_constraints_are_enforced() {
        let scheme = spec("appearance.colorScheme").unwrap();
        assert!(scheme.validate(&Value::Text("dark".into())).is_ok());
        assert!(matches!(
            scheme.validate(&Value::Text("sepia".into())),
            Err(SettingsError::NotAllowed { .. })
        ));
        assert!(matches!(
            scheme.validate(&Value::Bool(true)),
            Err(SettingsError::TypeMismatch { .. })
        ));

        let size = spec("dock.size").unwrap();
        assert!(size.validate(&Value::Number(0.75)).is_ok());
        assert!(matches!(
            size.validate(&Value::Number(1.5)),
            Err(SettingsError::OutOfRange { .. })
        ));

        let count = spec("workspaces.count").unwrap();
        assert!(count.validate(&Value::Integer(0)).is_err());
        assert!(count.validate(&Value::Integer(4)).is_ok());
    }

    /// A frozen manifest of the v1 key set. Adding a key is allowed (extend
    /// this list); renaming or removing one fails here, which is the
    /// additive-only guardrail the design calls for.
    #[test]
    fn the_v1_key_set_is_frozen_against_renames_and_removals() {
        const V1: &[&str] = &[
            "dock.size",
            "dock.magnification",
            "dock.position",
            "dock.autohide",
            "dock.animateOpening",
            "dock.showIndicators",
            "dock.minimizeIntoTileIcon",
            "dock.minimizedAnimation",
            "dock.titlebarDoubleClick",
            "dock.showRecentApps",
            "dock.pinned",
            "workspaces.count",
            "gestures.enabled",
            "gestures.spaceSwitch",
            "gestures.missionControl",
            "appearance.colorScheme",
            "appearance.accent",
            "accessibility.reduceMotion",
            "input.repeatDelay",
            "input.repeatRate",
        ];
        for key in V1 {
            assert!(spec(key).is_some(), "v1 key {key} was renamed or removed");
        }
        assert!(
            KEYS.len() >= V1.len(),
            "the v1 keys must remain a subset of the schema"
        );
    }
}
