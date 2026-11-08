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
pub const SCHEMA_VERSION: u32 = 21;

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
    Session,
    Menu,
    Sound,
    Overview,
    Notifications,
    Lock,
    Printers,
    Network,
    General,
    QuickSettings,
    Search,
    ScreenSaver,
    Desktop,
}

impl KeyGroup {
    /// Every group, in schema order.
    pub const ALL: [KeyGroup; 21] = [
        KeyGroup::Dock,
        KeyGroup::Workspaces,
        KeyGroup::Gestures,
        KeyGroup::Appearance,
        KeyGroup::Wallpaper,
        KeyGroup::Displays,
        KeyGroup::Animation,
        KeyGroup::Input,
        KeyGroup::Session,
        KeyGroup::Menu,
        KeyGroup::Sound,
        KeyGroup::Overview,
        KeyGroup::Notifications,
        KeyGroup::Lock,
        KeyGroup::Printers,
        KeyGroup::Network,
        KeyGroup::General,
        KeyGroup::QuickSettings,
        KeyGroup::Search,
        KeyGroup::ScreenSaver,
        KeyGroup::Desktop,
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
            KeyGroup::Session => "session",
            KeyGroup::Menu => "menu",
            KeyGroup::Sound => "sound",
            KeyGroup::Overview => "overview",
            KeyGroup::Notifications => "notifications",
            KeyGroup::Lock => "lock",
            KeyGroup::Printers => "printers",
            KeyGroup::Network => "network",
            KeyGroup::General => "general",
            KeyGroup::QuickSettings => "quick-settings",
            KeyGroup::Search => "search",
            KeyGroup::ScreenSaver => "screen-saver",
            KeyGroup::Desktop => "desktop",
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
    KeySpec {
        key: "dock.pinnedFolders",
        group: KeyGroup::Dock,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 7,
        summary: "Ordered absolute folder paths pinned to the Dock as stacks; empty seeds the Downloads default.",
    },
    KeySpec {
        key: "dock.chooserOnHover",
        group: KeyGroup::Dock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 8,
        summary: "Open a grouped app's window chooser on hover dwell, and retarget it along the Dock.",
    },
    KeySpec {
        key: "dock.minimizeReaction",
        group: KeyGroup::Dock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "shell/Dock",
        consumer: "shell/Dock",
        since: 9,
        summary: "Bounce an app's Dock tile once when one of its windows minimizes; off by default.",
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
    KeySpec {
        key: "appearance.transparency",
        group: KeyGroup::Appearance,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.5255),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "shell/design-system Glass, compositor/material",
        since: 18,
        summary: "Attached-menu glass transparency, 0 (opaque) to 1 (clearer); the measured midpoint is 0.5255.",
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
    KeySpec {
        key: "wallpaper.provider",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Text,
        default: KeyDefault::Text("wikimedia"),
        allowed: &[],
        min: None,
        max: None,
        owner: "wallpaperd",
        consumer: "services/wallpaperd",
        since: 10,
        summary: "Active online wallpaper content provider; `wikimedia` today.",
    },
    KeySpec {
        key: "wallpaper.providerAutoFetch",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "wallpaperd",
        consumer: "services/wallpaperd",
        since: 10,
        summary: "Let the provider refresh its Featured catalogue in the background.",
    },
    KeySpec {
        key: "wallpaper.providerLastFetch",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: Some(0.0),
        max: None,
        owner: "wallpaperd",
        consumer: "services/wallpaperd",
        since: 10,
        summary: "Unix seconds of the provider's last successful catalogue fetch; 0 = never.",
    },
    KeySpec {
        key: "wallpaper.providerSource",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "wallpaperd",
        consumer: "shell/wallpaper forwarder",
        since: 10,
        summary: "Fetched Featured default/fallback image path; empty until a catalogue exists.",
    },
    KeySpec {
        key: "wallpaper.builtinDefault",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "wallpaperd",
        consumer: "shell/wallpaper forwarder, apps/settings",
        since: 10,
        summary: "Resolved path of the shipped original Default.jpg; the out-of-box background.",
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
    KeySpec {
        key: "accessibility.reduceTransparency",
        group: KeyGroup::Animation,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Glass, compositor/material",
        since: 18,
        summary: "Replace sampled glass with a scheme-appropriate opaque plate; wins over the transparency slider.",
    },
    KeySpec {
        key: "accessibility.increaseContrast",
        group: KeyGroup::Animation,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Glass, compositor/material",
        since: 18,
        summary: "Strengthen glass edges and foreground contrast, including when Reduce Transparency is active.",
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
    // ── Pointer and keyboard-brightness keys (T-15.4b) ─────────────────
    // `Key repeat rate`/`Delay until repeat` are the revision-1 keys above.
    // These are the rest of the Keyboard/Mouse/Trackpad pane: the pointer
    // half maps to `PointerSettings` in `compositor/src/input/settings.rs`
    // and the keyboard-brightness half is a hardware bridge that is still a
    // follow-up, so its consumer is the Settings app for now. The read-only
    // libinput adapter (T-15.4a) supplies the device inventory, not these
    // preferences.
    KeySpec {
        key: "input.pointerSpeed",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.0),
        allowed: &[],
        min: Some(-1.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "compositor/input pointer acceleration",
        since: 12,
        summary: "Pointer tracking speed, -1.0 (slow) to 1.0 (fast); 0 is neutral.",
    },
    KeySpec {
        key: "input.naturalScroll",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input pointer scrolling",
        since: 12,
        summary: "Natural (content follows finger) scrolling for pointers.",
    },
    KeySpec {
        key: "input.tapToClick",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input pointer tapping",
        since: 12,
        summary: "Tap the trackpad to click.",
    },
    KeySpec {
        key: "input.leftHanded",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input pointer handedness",
        since: 12,
        summary: "Swap the primary and secondary pointer buttons.",
    },
    KeySpec {
        key: "input.scrollMethod",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text("two-finger"),
        allowed: &["two-finger", "edge", "button"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input pointer scrolling",
        since: 12,
        summary: "How a trackpad scrolls: two-finger, edge, or button.",
    },
    KeySpec {
        key: "input.keyboardBrightness",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.5),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "compositor/keyboard backlight (hardware bridge deferred)",
        since: 12,
        summary: "Keyboard backlight level, 0.0 (off) to 1.0 (bright).",
    },
    KeySpec {
        key: "input.adjustBrightnessLowLight",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/keyboard backlight (hardware bridge deferred)",
        since: 12,
        summary: "Adjust the keyboard backlight automatically in low light.",
    },
    KeySpec {
        key: "input.backlightOffAfter",
        group: KeyGroup::Input,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: Some(0.0),
        max: Some(3600.0),
        owner: "apps/settings",
        consumer: "compositor/keyboard backlight (hardware bridge deferred)",
        since: 12,
        summary: "Seconds of inactivity before the keyboard backlight turns off; 0 keeps it on.",
    },
    KeySpec {
        key: "input.keyboardNavigation",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input keyboard navigation",
        since: 12,
        summary: "Move focus between controls with Tab and Shift+Tab.",
    },
    KeySpec {
        key: "input.emojiKeyAction",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text("emoji"),
        allowed: &["emoji", "none"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input (emoji panel bridge deferred)",
        since: 12,
        summary: "Action when the Compose/Super key is pressed: show the emoji panel or nothing.",
    },
    // ── Session (lock / idle / suspend policy, T-12.5b) ─────────────────
    KeySpec {
        key: "idle.dim",
        group: KeyGroup::Session,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(150),
        allowed: &[],
        min: Some(0.0),
        max: Some(86_400.0),
        owner: "apps/settings",
        consumer: "session/idle engine",
        since: 5,
        summary: "Seconds of inactivity before the screen dims; 0 disables the stage.",
    },
    KeySpec {
        key: "idle.blank",
        group: KeyGroup::Session,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(300),
        allowed: &[],
        min: Some(0.0),
        max: Some(86_400.0),
        owner: "apps/settings",
        consumer: "session/idle engine",
        since: 5,
        summary: "Seconds of inactivity before the screen blanks; 0 disables the stage.",
    },
    KeySpec {
        key: "idle.lock",
        group: KeyGroup::Session,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(600),
        allowed: &[],
        min: Some(0.0),
        max: Some(86_400.0),
        owner: "apps/settings",
        consumer: "session/idle engine",
        since: 5,
        summary: "Seconds of inactivity before the session locks; 0 disables the stage.",
    },
    KeySpec {
        key: "idle.suspend",
        group: KeyGroup::Session,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: Some(0.0),
        max: Some(86_400.0),
        owner: "apps/settings",
        consumer: "session/idle engine, session/suspend",
        since: 5,
        summary: "Seconds of inactivity before the session suspends; 0 disables the stage.",
    },
    // ── Menu bar (T-14.2b) ──────────────────────────────────────────────
    KeySpec {
        key: "menu.global",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar, services/menu-broker",
        since: 6,
        summary:
            "Show the focused app's menus in the global menu bar; off restores local app menus.",
    },
    // ── Sound effects and balance (T-15.3b) ─────────────────────────────
    // Device selection, `Output volume`, and `Mute` are not settings keys:
    // they live on the PipeWire/WirePlumber adapter (T-15.3a) and round-trip
    // through the bridge host. These keys are the `Sound Effects`/`Balance`
    // half of the pane, owned and persisted by settingsd.
    KeySpec {
        key: "sound.alertSound",
        group: KeyGroup::Sound,
        kind: KeyType::Text,
        default: KeyDefault::Text("Chime"),
        allowed: &["Chime", "Marimba", "Pulse", "Woodblock", "Breeze"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (alert playback engine deferred)",
        since: 11,
        summary: "Alert sound name selected in the Sound pane.",
    },
    KeySpec {
        key: "sound.playEffectsThrough",
        group: KeyGroup::Sound,
        kind: KeyType::Text,
        default: KeyDefault::Text("output"),
        allowed: &["output", "alerts"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (alert playback engine deferred)",
        since: 11,
        summary: "Which device sound effects play through: the selected output or the alerts device.",
    },
    KeySpec {
        key: "sound.alertVolume",
        group: KeyGroup::Sound,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.8),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "apps/settings (alert playback engine deferred)",
        since: 11,
        summary: "Alert sound volume, 0.0 to 1.0, set by the Sound pane's Alert volume slider.",
    },
    KeySpec {
        key: "sound.playOnStartup",
        group: KeyGroup::Sound,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (startup chime engine deferred)",
        since: 11,
        summary: "Play the startup sound when the session begins.",
    },
    KeySpec {
        key: "sound.uiEffects",
        group: KeyGroup::Sound,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (UI sound engine deferred)",
        since: 11,
        summary: "Play user-interface sound effects.",
    },
    KeySpec {
        key: "sound.volumeFeedback",
        group: KeyGroup::Sound,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center, apps/settings (UI sound engine deferred)",
        since: 11,
        summary: "Play feedback when the output volume is changed.",
    },
    KeySpec {
        key: "sound.balance",
        group: KeyGroup::Sound,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.5),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "apps/settings (balance write deferred)",
        since: 11,
        summary: "Output balance from Left (0.0) to Right (1.0); 0.5 is centered.",
    },
    // ── Mission Control / hot corners (T-15.5b) ─────────────────────────
    // Mission Control and hot corners are compositor-native: the compositor
    // owns the one overview machine and the corner detector, and the shell
    // mirrors both over `df_toplevel_manager`. These four keys are the
    // durable trigger *configuration* the Mission Control pane writes; the
    // action ids are the `HotCornerAction::id()` spellings the compositor's
    // `HotCornerConfig` already uses. The gesture trio that also reaches
    // Mission Control is the revision-1 `gestures.*` keys above. The pane
    // reports the adapter/projection directly; applying an assignment needs
    // the append-only compositor policy request ADR 0126 names, deferred.
    KeySpec {
        key: "overview.hotCornerTopLeft",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("mission-control"),
        allowed: &[
            "none",
            "mission-control",
            "notification-center",
            "desktop-reveal",
            "lock-screen",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners (apply deferred, ADR 0126)",
        since: 13,
        summary: "Action assigned to the top-left hot corner; `none` disables it.",
    },
    KeySpec {
        key: "overview.hotCornerTopRight",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("notification-center"),
        allowed: &[
            "none",
            "mission-control",
            "notification-center",
            "desktop-reveal",
            "lock-screen",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners (apply deferred, ADR 0126)",
        since: 13,
        summary: "Action assigned to the top-right hot corner; `none` disables it.",
    },
    KeySpec {
        key: "overview.hotCornerBottomLeft",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("desktop-reveal"),
        allowed: &[
            "none",
            "mission-control",
            "notification-center",
            "desktop-reveal",
            "lock-screen",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners (apply deferred, ADR 0126)",
        since: 13,
        summary: "Action assigned to the bottom-left hot corner; `none` disables it.",
    },
    KeySpec {
        key: "overview.hotCornerBottomRight",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("lock-screen"),
        allowed: &[
            "none",
            "mission-control",
            "notification-center",
            "desktop-reveal",
            "lock-screen",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners (apply deferred, ADR 0126)",
        since: 13,
        summary: "Action assigned to the bottom-right hot corner; `none` disables it.",
    },
    // ── Notifications presentation (T-15.7b) ────────────────────────────
    // The four global "Notification Center" preferences the Notifications
    // pane owns. They are presentation policy, not the notification service's
    // queue or Focus admission rule (that lives in `services/notifications`,
    // ADR 0058/0130), so they belong in settingsd like the other desktop
    // preferences. The Focus mode and the per-app allow list are *not* here:
    // they ride the notification adapter (T-15.7a).
    KeySpec {
        key: "notifications.showPreviews",
        group: KeyGroup::Notifications,
        kind: KeyType::Text,
        default: KeyDefault::Text("when-unlocked"),
        allowed: &["always", "when-unlocked", "never"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (stored policy)",
        since: 14,
        summary: "When notification previews show: always, when unlocked, or never.",
    },
    KeySpec {
        key: "notifications.showWhenSleeping",
        group: KeyGroup::Notifications,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (stored policy)",
        since: 14,
        summary: "Show notification banners while the display is sleeping.",
    },
    KeySpec {
        key: "notifications.showWhenLocked",
        group: KeyGroup::Notifications,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (stored policy)",
        since: 14,
        summary: "Show notification banners while the screen is locked.",
    },
    KeySpec {
        key: "notifications.showWhenMirroring",
        group: KeyGroup::Notifications,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (stored policy)",
        since: 14,
        summary: "Show notification banners while mirroring or sharing the display.",
    },
    // ── Lock Screen display policy (T-15.8b) ────────────────────────────
    // The idle/lock *timing* keys already live in the `Session` group
    // (`idle.blank`, `idle.lock`; revision 5, ADR 0070) and the session idle
    // engine applies them live. These keys are the lock-screen *display*
    // options the pane owns; the stable suffix is `LockDisplayOption::id()`
    // (dragonfruit-lock-adapter). The compositor lock and the session timing
    // stay owned by their engines (ADR 0132).
    KeySpec {
        key: "lock.showUserNameAndPhoto",
        group: KeyGroup::Lock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen (stored policy)",
        since: 15,
        summary: "Show the user name and photo on the lock screen.",
    },
    KeySpec {
        key: "lock.showPasswordHints",
        group: KeyGroup::Lock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen (stored policy)",
        since: 15,
        summary: "Show the password hint on the lock screen.",
    },
    KeySpec {
        key: "lock.showMessageWhenLocked",
        group: KeyGroup::Lock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen (stored policy)",
        since: 15,
        summary: "Show a custom message on the lock screen.",
    },
    KeySpec {
        key: "lock.message",
        group: KeyGroup::Lock,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen (stored policy)",
        since: 15,
        summary: "The custom lock-screen message set by the Set... editor.",
    },
    KeySpec {
        key: "lock.showPowerButtons",
        group: KeyGroup::Lock,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen (stored policy)",
        since: 15,
        summary: "Show the Sleep, Restart, and Shut Down buttons on the lock screen.",
    },
    // ── Menu Bar configuration (T-15.9b) ────────────────────────────────
    // The menu bar is shell-native: the shell's `MenuBar` owns the chrome, the
    // status row, and the clock, and the menu-broker resolves the focused
    // app's menus. These are the durable preferences the Menu Bar pane owns
    // (ADR 0135). The shell applies the clock options, the background, and the
    // per-control visibility live; the auto-hide mode and the recent-items
    // count are stored policy the shell does not consume yet (see the
    // follow-ups). The spellings mirror `dragonfruit-menubar-adapter`'s
    // `MenuBarAutoHide::id()` and `MenuBarControl::id()` values.
    KeySpec {
        key: "menu.autoHide",
        group: KeyGroup::Menu,
        kind: KeyType::Text,
        default: KeyDefault::Text("full-screen"),
        allowed: &["never", "always", "full-screen"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar, compositor/menu-bar visibility",
        since: 16,
        summary: "When the menu bar auto-hides: never, always, or only in full screen.",
    },
    KeySpec {
        key: "menu.showBackground",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar, compositor/menu-bar veil",
        since: 16,
        summary: "Draw the menu bar's background material behind its items.",
    },
    KeySpec {
        key: "menu.recentItems",
        group: KeyGroup::Menu,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(10),
        allowed: &[],
        min: Some(0.0),
        max: Some(50.0),
        owner: "apps/settings",
        consumer: "apps/settings (recent-items consumer deferred)",
        since: 16,
        summary: "How many recent documents, applications, and servers to remember.",
    },
    KeySpec {
        key: "menu.clock.showDate",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar clock",
        since: 16,
        summary: "Show the date beside the time in the menu-bar clock.",
    },
    KeySpec {
        key: "menu.clock.showSeconds",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar clock",
        since: 16,
        summary: "Display the menu-bar clock with seconds.",
    },
    KeySpec {
        key: "menu.control.wifi",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        since: 16,
        summary: "Show the Wi-Fi control in the menu bar when its adapter is present.",
    },
    KeySpec {
        key: "menu.control.bluetooth",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        since: 16,
        summary: "Show the Bluetooth control in the menu bar when its adapter is present.",
    },
    KeySpec {
        key: "menu.control.battery",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        since: 16,
        summary: "Show the Battery control in the menu bar when its adapter is present.",
    },
    KeySpec {
        key: "menu.control.volume",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        since: 16,
        summary: "Show the Sound control in the menu bar when its adapter is present.",
    },
    KeySpec {
        key: "menu.control.focus",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        since: 16,
        summary: "Show the Focus control in the menu bar when its service is present.",
    },
    KeySpec {
        key: "menu.control.accessibility",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        since: 16,
        summary: "Show the Accessibility control in the menu bar when it is available.",
    },
    // ── Printers & Scanners (T-15.12b) ──────────────────────────────────
    // CUPS owns the queues, the system default destination, and the job
    // queue; SANE owns the scanner inventory (ADR 0140). The one durable
    // presentation preference the pane owns is the default paper size, which
    // has no CUPS client-tool equivalent for a system-wide default. The
    // spellings are the pane's stable paper-size ids.
    KeySpec {
        key: "printers.defaultPaperSize",
        group: KeyGroup::Printers,
        kind: KeyType::Text,
        default: KeyDefault::Text("us-letter"),
        allowed: &["us-letter", "us-legal", "a3", "a4", "a5"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (stored preference)",
        since: 17,
        summary: "The default paper size new print jobs assume.",
    },
    // ── Network prompt policy (T225) ─────────────────────────────────────
    // NetworkManager owns the radio, connections, and autoconnect. These two
    // keys only decide whether the shell prompts before a network is joined;
    // they never override NetworkManager's own autoconnect. The stable ids
    // mirror the pane's popup labels.
    KeySpec {
        key: "network.wifi.askToJoin",
        group: KeyGroup::Network,
        kind: KeyType::Text,
        default: KeyDefault::Text("notify"),
        allowed: &["off", "notify", "ask", "new"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar Wi-Fi prompt policy",
        since: 19,
        summary: "When to prompt before joining a nearby Wi-Fi network.",
    },
    KeySpec {
        key: "network.wifi.askToJoinHotspots",
        group: KeyGroup::Network,
        kind: KeyType::Text,
        default: KeyDefault::Text("ask"),
        allowed: &["off", "ask", "automatic"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar Wi-Fi prompt policy",
        since: 19,
        summary: "When to prompt before joining a captive-portal hotspot.",
    },
    // ── General group (T226) ─────────────────────────────────────────────
    // Date & Time, Language & Region, Software Update policy, and Backup.
    // Each sub-pane owns its own state: the host adapters own the system
    // clock/locale and the backup tools; these keys are the durable
    // presentation and policy values the panes write. All defaults are the
    // conservative/off choice, so a fresh profile never enables automatic
    // updates or a backup schedule silently.
    KeySpec {
        key: "clock.use24Hour",
        group: KeyGroup::General,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar clock",
        since: 20,
        summary: "Force the menu-bar clock to 24-hour time instead of the locale format.",
    },
    KeySpec {
        key: "updates.auto",
        group: KeyGroup::General,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "dragonfruit-system-status update provider",
        since: 20,
        summary: "Check for software updates automatically; never enabled by default.",
    },
    KeySpec {
        key: "updates.autoInstall",
        group: KeyGroup::General,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "dragonfruit-system-status update provider",
        since: 20,
        summary: "Install the updates a check finds automatically; never enabled by default.",
    },
    KeySpec {
        key: "locale.calendar",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("gregorian"),
        allowed: &["gregorian", "iso", "buddhist", "japanese"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (date preview)",
        since: 20,
        summary: "The calendar system the Date & Time and Language previews use.",
    },
    KeySpec {
        key: "locale.temperature",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("celsius"),
        allowed: &["celsius", "fahrenheit"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (weather and date previews)",
        since: 20,
        summary: "The temperature unit the desktop shows.",
    },
    KeySpec {
        key: "locale.measurement",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("metric"),
        allowed: &["metric", "imperial"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (measurement previews)",
        since: 20,
        summary: "The measurement system the desktop shows.",
    },
    KeySpec {
        key: "locale.dateFormat",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("medium"),
        allowed: &["short", "medium", "long"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (date preview)",
        since: 20,
        summary: "The date-format length the Language & Region preview uses.",
    },
    KeySpec {
        key: "locale.numberFormat",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("locale"),
        allowed: &["locale", "decimal", "comma"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (number preview)",
        since: 20,
        summary: "How numbers group and separate their decimals.",
    },
    KeySpec {
        key: "locale.firstDay",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("locale"),
        allowed: &["locale", "sunday", "monday", "saturday"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (calendar preview)",
        since: 20,
        summary: "The first day of the week the desktop calendar uses.",
    },
    KeySpec {
        key: "backup.auto",
        group: KeyGroup::General,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Backup pane / backup backend",
        since: 20,
        summary: "Run the selected backup backend on its schedule; off by default.",
    },
    KeySpec {
        key: "backup.backend",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("auto"),
        allowed: &["auto", "restic", "pika", "timeshift"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Backup pane",
        since: 20,
        summary: "The backup backend to prefer; auto detects restic, Pika, or Timeshift.",
    },
    KeySpec {
        key: "backup.destination",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Backup pane / backup backend",
        since: 20,
        summary: "The destination the selected backup backend writes to.",
    },
    KeySpec {
        key: "backup.schedule",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text("off"),
        allowed: &["off", "hourly", "daily", "weekly", "monthly"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Backup pane / backup backend",
        since: 20,
        summary: "How often the selected backup backend runs.",
    },
    // ── Appearance completion (T227) ─────────────────────────────────────
    // The remaining Appearance rows are durable design-system preferences.
    // Highlight is a text colour override (empty = the token selection);
    // the sidebar size and scroll-bar policy are enumerated spellings the
    // Sidebar and ScrollView components consume.
    KeySpec {
        key: "appearance.highlight",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Theme, apps/settings Theme",
        since: 21,
        summary: "Selection/highlight colour override ('#rrggbb'); empty uses the token selection.",
    },
    KeySpec {
        key: "appearance.sidebarIconSize",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text("medium"),
        allowed: &["small", "medium", "large"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Sidebar",
        since: 21,
        summary: "Sidebar icon size: small, medium, or large.",
    },
    KeySpec {
        key: "appearance.wallpaperTint",
        group: KeyGroup::Appearance,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Glass, apps/settings Glass",
        since: 21,
        summary: "Let glass surfaces take their tint from the wallpaper; off uses the neutral plate.",
    },
    KeySpec {
        key: "appearance.scrollBars",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text("automatic"),
        allowed: &["automatic", "when-scrolling", "always"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system ScrollView, apps/settings ScrollView",
        since: 21,
        summary: "When scroll bars show: automatically, only while scrolling, or always.",
    },
    // ── Quick Settings customization (T227) ──────────────────────────────
    // The ordered module ids the Control Center panel renders. An empty list
    // means the built-in order. `menuBarOnly` names modules that stay in the
    // menu bar but are removed from the panel.
    KeySpec {
        key: "quickSettings.modules",
        group: KeyGroup::QuickSettings,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[
            "wifi", "bluetooth", "storage", "focus", "volume", "brightness",
            "dark", "keyboard", "mission-control", "battery", "lock-screen",
            "menu-bar", "software-update", "users", "printers", "privacy",
            "accessibility", "vpn",
        ]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center",
        since: 21,
        summary: "Ordered Control Center module ids; empty means the built-in order.",
    },
    KeySpec {
        key: "quickSettings.menuBarOnly",
        group: KeyGroup::QuickSettings,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center, shell/MenuBar",
        since: 21,
        summary: "Module ids that stay in the menu bar and are hidden from the Control Center panel.",
    },
    // ── Search (T227) ────────────────────────────────────────────────────
    // Durable Spotlight-equivalent preferences. T246 owns the search service
    // that reads them; until then the pane reports the service as absent.
    KeySpec {
        key: "search.categories",
        group: KeyGroup::Search,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[
            "applications", "files", "folders", "actions", "calculations",
            "definitions", "web",
        ]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "search service (T246)",
        since: 21,
        summary: "Result categories the desktop search includes.",
    },
    KeySpec {
        key: "search.indexLocations",
        group: KeyGroup::Search,
        kind: KeyType::TextList,
        default: KeyDefault::List(&["home"]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "search service (T246)",
        since: 21,
        summary: "Folders the file index includes; `home` is the user's home directory.",
    },
    KeySpec {
        key: "search.shortcut",
        group: KeyGroup::Search,
        kind: KeyType::Text,
        default: KeyDefault::Text("Super+Space"),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "search service (T246), shell shortcuts",
        since: 21,
        summary: "Global keyboard shortcut that opens Search.",
    },
    KeySpec {
        key: "search.web",
        group: KeyGroup::Search,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "search service (T246)",
        since: 21,
        summary: "Offer a web-search result for the query using the default browser.",
    },
    KeySpec {
        key: "menu.control.search",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        since: 21,
        summary: "Show the Search control in the menu bar when the search service is present.",
    },
    // ── Screen Saver (T227) ──────────────────────────────────────────────
    // The shell's lock screen is the shipped screen-saver surface. The style
    // selects the clock presentation; the idle blank delay is `idle.blank`.
    KeySpec {
        key: "screenSaver.style",
        group: KeyGroup::ScreenSaver,
        kind: KeyType::Text,
        default: KeyDefault::Text("clock"),
        allowed: &["clock", "none"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen",
        since: 21,
        summary: "Screen-saver style: the lock-screen clock, or no screen saver.",
    },
    KeySpec {
        key: "screenSaver.clock",
        group: KeyGroup::ScreenSaver,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen",
        since: 21,
        summary: "Show the clock on the screen-saver/lock surface.",
    },
    KeySpec {
        key: "screenSaver.showDate",
        group: KeyGroup::ScreenSaver,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen",
        since: 21,
        summary: "Show the date under the screen-saver clock.",
    },
    KeySpec {
        key: "screenSaver.showSeconds",
        group: KeyGroup::ScreenSaver,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen",
        since: 21,
        summary: "Show seconds in the screen-saver clock.",
    },
    // ── Desktop (T227) ───────────────────────────────────────────────────
    KeySpec {
        key: "desktop.showItems",
        group: KeyGroup::Desktop,
        kind: KeyType::Text,
        default: KeyDefault::Text("on-desktop"),
        allowed: &["on-desktop", "in-stage"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/files desktop process",
        since: 21,
        summary: "Show desktop items on the desktop or leave them to Stage Manager.",
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

    /// The session policy keys are the exact four ADR 0070 freezes, as whole
    /// seconds; `0` disables a stage, and the defaults match the idle engine's
    /// `IdlePolicy::new()` (dim 150, blank 300, lock 600, no suspend).
    #[test]
    fn the_session_policy_keys_follow_adr_0070() {
        for (key, default) in [
            ("idle.dim", 150),
            ("idle.blank", 300),
            ("idle.lock", 600),
            ("idle.suspend", 0),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.kind, KeyType::Integer, "{key}");
            assert_eq!(spec.default, KeyDefault::Integer(default), "{key}");
            assert_eq!(spec.min, Some(0.0), "{key}");
            assert_eq!(spec.max, Some(86_400.0), "{key}");
            // The `IdlePolicy::from_keys` spellings: whole seconds, 0 is off.
            assert!(spec.validate(&Value::Integer(0)).is_ok(), "{key} off");
            assert!(spec.validate(&Value::Integer(86_400)).is_ok(), "{key} max");
            assert!(
                spec.validate(&Value::Integer(-1)).is_err(),
                "{key} negative"
            );
        }
    }

    /// The global application-menu toggle (T-14.2b): a boolean, default on, in
    /// its own `menu` group.
    #[test]
    fn the_global_menu_toggle_is_a_declared_bool() {
        let spec = spec("menu.global").expect("menu.global is declared");
        assert_eq!(spec.group, KeyGroup::Menu);
        assert_eq!(spec.kind, KeyType::Bool);
        assert_eq!(spec.default, KeyDefault::Bool(true));
        assert_eq!(spec.since, 6);
        assert!(spec.validate(&Value::Bool(false)).is_ok());
        assert!(spec.validate(&Value::Text("on".into())).is_err());
    }

    /// The Dock folder-pin list (T-14.7k): an ordered string list of absolute
    /// folder paths, additive in revision 7, defaulting to empty so the shell
    /// can seed the Downloads default on first run.
    #[test]
    fn the_dock_folder_pin_list_is_declared_in_revision_seven() {
        let spec = spec("dock.pinnedFolders").expect("dock.pinnedFolders is declared");
        assert_eq!(spec.group, KeyGroup::Dock);
        assert_eq!(spec.kind, KeyType::TextList);
        assert_eq!(spec.default, KeyDefault::List(&[]));
        assert_eq!(spec.since, 7);
        assert!(spec.since <= SCHEMA_VERSION);
        assert!(spec
            .validate(&Value::TextList(vec!["/home/user/Documents".into()]))
            .is_ok());
        assert!(spec.validate(&Value::Text("nope".into())).is_err());
        // A single path is a valid list of one.
        assert!(matches!(
            spec.default.to_value(),
            Value::TextList(items) if items.is_empty()
        ));
    }

    /// The Dock on-hover chooser opt-in (T-14.7p): a boolean, default off, so
    /// the shipping macOS click contract is unchanged, additive in revision 8.
    #[test]
    fn the_dock_on_hover_chooser_is_declared_in_revision_eight() {
        let spec = spec("dock.chooserOnHover").expect("dock.chooserOnHover is declared");
        assert_eq!(spec.group, KeyGroup::Dock);
        assert_eq!(spec.kind, KeyType::Bool);
        assert_eq!(spec.default, KeyDefault::Bool(false));
        assert_eq!(spec.since, 8);
        assert!(spec.since <= SCHEMA_VERSION);
        assert!(spec.validate(&Value::Bool(true)).is_ok());
        assert!(spec.validate(&Value::Text("on".into())).is_err());
    }

    /// The minimize-to-icon reaction opt-in (T-14.7s): a boolean, default off,
    /// so the Dock is unchanged unless the user asks for the acknowledgment,
    /// additive in revision 9.
    #[test]
    fn the_dock_minimize_reaction_is_declared_in_revision_nine() {
        let spec = spec("dock.minimizeReaction").expect("dock.minimizeReaction is declared");
        assert_eq!(spec.group, KeyGroup::Dock);
        assert_eq!(spec.kind, KeyType::Bool);
        assert_eq!(spec.default, KeyDefault::Bool(false));
        assert_eq!(spec.since, 9);
        assert!(spec.since <= SCHEMA_VERSION);
        assert!(spec.validate(&Value::Bool(true)).is_ok());
        assert!(spec.validate(&Value::Text("on".into())).is_err());
    }

    /// The wallpaper provider keys (T-18.1b, ADR 0094/0055): additive in
    /// revision 10, `wallpaperd`-owned, all defaults off/empty so the shipped
    /// default and a user choice keep winning.
    #[test]
    fn the_wallpaper_provider_keys_are_declared_in_revision_ten() {
        for (key, kind, default) in [
            (
                "wallpaper.provider",
                KeyType::Text,
                KeyDefault::Text("wikimedia"),
            ),
            (
                "wallpaper.providerAutoFetch",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            (
                "wallpaper.providerLastFetch",
                KeyType::Integer,
                KeyDefault::Integer(0),
            ),
            (
                "wallpaper.providerSource",
                KeyType::Text,
                KeyDefault::Text(""),
            ),
            (
                "wallpaper.builtinDefault",
                KeyType::Text,
                KeyDefault::Text(""),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Wallpaper, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "wallpaperd", "{key}");
            assert_eq!(spec.since, 10, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
        }
        // `wallpaper.source` stays the user override owned by apps/settings.
        assert_eq!(spec("wallpaper.source").unwrap().owner, "apps/settings");
    }

    /// The Sound-pane keys (T-15.3b): the `Sound Effects`/`Balance` rows,
    /// additive in revision 11. Device selection, `Output volume`, and `Mute`
    /// stay on the audio adapter, not here.
    #[test]
    fn the_sound_keys_are_declared_in_revision_eleven() {
        for (key, kind, default) in [
            ("sound.alertSound", KeyType::Text, KeyDefault::Text("Chime")),
            (
                "sound.playEffectsThrough",
                KeyType::Text,
                KeyDefault::Text("output"),
            ),
            (
                "sound.alertVolume",
                KeyType::Number,
                KeyDefault::Number(0.8),
            ),
            ("sound.playOnStartup", KeyType::Bool, KeyDefault::Bool(true)),
            ("sound.uiEffects", KeyType::Bool, KeyDefault::Bool(true)),
            (
                "sound.volumeFeedback",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            ("sound.balance", KeyType::Number, KeyDefault::Number(0.5)),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Sound, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 11, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
        }
    }

    /// The Keyboard/Mouse/Trackpad-pane keys (T-15.4b): the pointer and
    /// keyboard-brightness rows, additive in revision 12. `input.repeatDelay`
    /// and `input.repeatRate` stay the revision-1 keyboard-repeat keys.
    #[test]
    fn the_pointer_and_keyboard_pane_keys_are_declared_in_revision_twelve() {
        for (key, kind, default) in [
            (
                "input.pointerSpeed",
                KeyType::Number,
                KeyDefault::Number(0.0),
            ),
            ("input.naturalScroll", KeyType::Bool, KeyDefault::Bool(true)),
            ("input.tapToClick", KeyType::Bool, KeyDefault::Bool(true)),
            ("input.leftHanded", KeyType::Bool, KeyDefault::Bool(false)),
            (
                "input.scrollMethod",
                KeyType::Text,
                KeyDefault::Text("two-finger"),
            ),
            (
                "input.keyboardBrightness",
                KeyType::Number,
                KeyDefault::Number(0.5),
            ),
            (
                "input.adjustBrightnessLowLight",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            (
                "input.backlightOffAfter",
                KeyType::Integer,
                KeyDefault::Integer(0),
            ),
            (
                "input.keyboardNavigation",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "input.emojiKeyAction",
                KeyType::Text,
                KeyDefault::Text("emoji"),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Input, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 12, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        // The pointer speed is signed so slow is expressible.
        let speed = spec("input.pointerSpeed").unwrap();
        assert!(speed.validate(&Value::Number(-1.0)).is_ok());
        assert!(speed.validate(&Value::Number(1.0)).is_ok());
        assert!(speed.validate(&Value::Number(1.5)).is_err());
        // The scroll method is an enumeration.
        assert!(spec("input.scrollMethod")
            .unwrap()
            .validate(&Value::Text("edge".into()))
            .is_ok());
        assert!(spec("input.scrollMethod")
            .unwrap()
            .validate(&Value::Text("pinch".into()))
            .is_err());
    }

    /// The Mission Control / hot corners keys (T-15.5b): the four corner
    /// assignments, additive in revision 13. The gesture trio that also opens
    /// Mission Control stays the revision-1 `gestures.*` keys.
    #[test]
    fn the_hot_corner_keys_are_declared_in_revision_thirteen() {
        for (key, default) in [
            ("overview.hotCornerTopLeft", "mission-control"),
            ("overview.hotCornerTopRight", "notification-center"),
            ("overview.hotCornerBottomLeft", "desktop-reveal"),
            ("overview.hotCornerBottomRight", "lock-screen"),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Overview, "{key}");
            assert_eq!(spec.kind, KeyType::Text, "{key}");
            assert_eq!(spec.default, KeyDefault::Text(default), "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 13, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
            // The action vocabulary is the `HotCornerAction::id()` set.
            for action in [
                "none",
                "mission-control",
                "notification-center",
                "desktop-reveal",
                "lock-screen",
            ] {
                assert!(
                    spec.validate(&Value::Text(action.into())).is_ok(),
                    "{key} accepts {action}"
                );
            }
            assert!(spec.validate(&Value::Text("corners".into())).is_err());
        }
    }

    /// The Notifications presentation keys (T-15.7b): the four global
    /// Notification Center preferences the Notifications pane owns, additive
    /// in revision 14. The Focus mode and per-app allow list stay on the
    /// notification adapter, not here.
    #[test]
    fn the_notification_keys_are_declared_in_revision_fourteen() {
        for (key, kind, default) in [
            (
                "notifications.showPreviews",
                KeyType::Text,
                KeyDefault::Text("when-unlocked"),
            ),
            (
                "notifications.showWhenSleeping",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "notifications.showWhenLocked",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            (
                "notifications.showWhenMirroring",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Notifications, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 14, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        // The previews value is an enumeration of the three capture choices.
        let previews = spec("notifications.showPreviews").unwrap();
        assert!(previews.validate(&Value::Text("always".into())).is_ok());
        assert!(previews.validate(&Value::Text("never".into())).is_ok());
        assert!(previews.validate(&Value::Text("sometimes".into())).is_err());
        // The three toggles reject a non-bool.
        for key in [
            "notifications.showWhenSleeping",
            "notifications.showWhenLocked",
            "notifications.showWhenMirroring",
        ] {
            assert!(
                spec(key).unwrap().validate(&Value::Bool(true)).is_ok(),
                "{key}"
            );
            assert!(
                spec(key)
                    .unwrap()
                    .validate(&Value::Text("on".into()))
                    .is_err(),
                "{key}"
            );
        }
    }

    /// The Lock Screen display keys (T-15.8b): the four `LockDisplayOption`
    /// toggles plus the custom-message text, additive in revision 15. The
    /// idle/lock *timing* keys already exist in the `Session` group
    /// (`idle.blank`, `idle.lock`), so the pane reuses them; the compositor
    /// lock and the session timing stay owned by their engines (ADR 0132).
    #[test]
    fn the_lock_screen_keys_are_declared_in_revision_fifteen() {
        for (key, kind, default) in [
            (
                "lock.showUserNameAndPhoto",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            (
                "lock.showPasswordHints",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "lock.showMessageWhenLocked",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            ("lock.message", KeyType::Text, KeyDefault::Text("")),
            (
                "lock.showPowerButtons",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Lock, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 15, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        // The four toggles reject a non-bool; the message is free text.
        for key in [
            "lock.showUserNameAndPhoto",
            "lock.showPasswordHints",
            "lock.showMessageWhenLocked",
            "lock.showPowerButtons",
        ] {
            assert!(
                spec(key).unwrap().validate(&Value::Bool(true)).is_ok(),
                "{key}"
            );
            assert!(
                spec(key)
                    .unwrap()
                    .validate(&Value::Text("on".into()))
                    .is_err(),
                "{key}"
            );
        }
        assert!(spec("lock.message")
            .unwrap()
            .validate(&Value::Text("Back at 3.".into()))
            .is_ok());
    }

    /// The Menu Bar configuration keys (T-15.9b): the auto-hide mode, the
    /// background toggle, the recent-items count, the two clock options, and
    /// the per-control visibility toggles, additive in revision 16. The
    /// spellings mirror `dragonfruit-menubar-adapter` (ADR 0134/0135).
    #[test]
    fn the_menu_bar_keys_are_declared_in_revision_sixteen() {
        for (key, kind, default) in [
            (
                "menu.autoHide",
                KeyType::Text,
                KeyDefault::Text("full-screen"),
            ),
            ("menu.showBackground", KeyType::Bool, KeyDefault::Bool(true)),
            (
                "menu.recentItems",
                KeyType::Integer,
                KeyDefault::Integer(10),
            ),
            ("menu.clock.showDate", KeyType::Bool, KeyDefault::Bool(true)),
            (
                "menu.clock.showSeconds",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            ("menu.control.wifi", KeyType::Bool, KeyDefault::Bool(true)),
            (
                "menu.control.bluetooth",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            (
                "menu.control.battery",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            ("menu.control.volume", KeyType::Bool, KeyDefault::Bool(true)),
            ("menu.control.focus", KeyType::Bool, KeyDefault::Bool(true)),
            (
                "menu.control.accessibility",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Menu, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 16, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        // The auto-hide value is the adapter's three-mode enumeration.
        let auto_hide = spec("menu.autoHide").unwrap();
        for mode in ["never", "always", "full-screen"] {
            assert!(
                auto_hide.validate(&Value::Text(mode.into())).is_ok(),
                "{mode}"
            );
        }
        assert!(auto_hide
            .validate(&Value::Text("sometimes".into()))
            .is_err());
        // The recent-items count is bounded and integral.
        let recent = spec("menu.recentItems").unwrap();
        assert!(recent.validate(&Value::Integer(0)).is_ok());
        assert!(recent.validate(&Value::Integer(10)).is_ok());
        assert!(recent.validate(&Value::Integer(50)).is_ok());
        assert!(recent.validate(&Value::Integer(-1)).is_err());
        assert!(recent.validate(&Value::Integer(51)).is_err());
        // The visibility and clock toggles reject a non-bool.
        for key in [
            "menu.showBackground",
            "menu.clock.showDate",
            "menu.clock.showSeconds",
            "menu.control.wifi",
            "menu.control.bluetooth",
            "menu.control.battery",
            "menu.control.volume",
            "menu.control.focus",
            "menu.control.accessibility",
        ] {
            assert!(
                spec(key).unwrap().validate(&Value::Bool(false)).is_ok(),
                "{key}"
            );
            assert!(
                spec(key)
                    .unwrap()
                    .validate(&Value::Text("on".into()))
                    .is_err(),
                "{key}"
            );
        }
    }

    /// The Printers & Scanners key (T-15.12b): the one durable presentation
    /// The glass preference and its two accessibility overrides are additive
    /// in revision 18 (T196). The slider is a bounded double with the measured
    /// midpoint as its default; both overrides are off by default and apply
    /// independently of the appearance rollout.
    #[test]
    fn the_glass_keys_are_declared_in_revision_eighteen() {
        let transparency = spec("appearance.transparency").expect("transparency is declared");
        assert_eq!(transparency.group, KeyGroup::Appearance);
        assert_eq!(transparency.kind, KeyType::Number);
        assert_eq!(transparency.default, KeyDefault::Number(0.5255));
        assert_eq!(transparency.min, Some(0.0));
        assert_eq!(transparency.max, Some(1.0));
        assert_eq!(transparency.since, 18);
        assert!(transparency.validate(&Value::Number(0.0)).is_ok());
        assert!(transparency.validate(&Value::Number(0.5255)).is_ok());
        assert!(transparency.validate(&Value::Number(1.0)).is_ok());
        assert!(transparency.validate(&Value::Number(1.01)).is_err());
        assert!(transparency.validate(&Value::Number(-0.01)).is_err());

        for key in [
            "accessibility.reduceTransparency",
            "accessibility.increaseContrast",
        ] {
            let override_spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(override_spec.group, KeyGroup::Animation, "{key}");
            assert_eq!(override_spec.kind, KeyType::Bool, "{key}");
            assert_eq!(override_spec.default, KeyDefault::Bool(false), "{key}");
            assert_eq!(override_spec.since, 18, "{key}");
            assert!(override_spec.validate(&Value::Bool(true)).is_ok(), "{key}");
            assert!(override_spec.validate(&Value::Bool(false)).is_ok(), "{key}");
        }
    }

    /// preference the pane owns (CUPS and SANE own their own state), additive
    /// in revision 17 (ADR 0141).
    #[test]
    fn the_printer_key_is_declared_in_revision_seventeen() {
        let spec =
            spec("printers.defaultPaperSize").expect("printers.defaultPaperSize is declared");
        assert_eq!(spec.group, KeyGroup::Printers);
        assert_eq!(spec.kind, KeyType::Text);
        assert_eq!(spec.default, KeyDefault::Text("us-letter"));
        assert_eq!(spec.owner, "apps/settings");
        assert_eq!(spec.since, 17);
        assert!(spec.since <= SCHEMA_VERSION);
        for size in ["us-letter", "us-legal", "a3", "a4", "a5"] {
            assert!(
                spec.validate(&Value::Text(size.into())).is_ok(),
                "{size} is a valid paper size"
            );
        }
        assert!(spec.validate(&Value::Text("tabloid".into())).is_err());
        assert_eq!(
            KeyGroup::Printers.name(),
            "printers",
            "the group name is stable"
        );
    }

    /// The Wi-Fi prompt policy (T225), additive in revision 19. The pane owns
    /// the two popup rows; NetworkManager still owns autoconnect.
    #[test]
    fn the_network_wifi_prompt_keys_are_declared_in_revision_nineteen() {
        for (key, default, allowed) in [
            (
                "network.wifi.askToJoin",
                KeyDefault::Text("notify"),
                &["off", "notify", "ask", "new"][..],
            ),
            (
                "network.wifi.askToJoinHotspots",
                KeyDefault::Text("ask"),
                &["off", "ask", "automatic"][..],
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Network, "{key}");
            assert_eq!(spec.kind, KeyType::Text, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.allowed, allowed, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 19, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            for value in allowed {
                assert!(
                    spec.validate(&Value::Text((*value).into())).is_ok(),
                    "{key}: {value}"
                );
            }
            assert!(
                spec.validate(&Value::Text("sometimes".into())).is_err(),
                "{key} rejects an unknown mode"
            );
        }
        assert_eq!(KeyGroup::Network.name(), "network");
    }

    /// The General group keys (T226), additive in revision 20. Every default
    /// is the conservative/off choice: automatic updates and backups are
    /// never silently enabled.
    #[test]
    fn the_general_group_keys_are_declared_in_revision_twenty() {
        for (key, default) in [
            ("clock.use24Hour", KeyDefault::Bool(false)),
            ("updates.auto", KeyDefault::Bool(false)),
            ("updates.autoInstall", KeyDefault::Bool(false)),
            ("locale.calendar", KeyDefault::Text("gregorian")),
            ("locale.temperature", KeyDefault::Text("celsius")),
            ("locale.measurement", KeyDefault::Text("metric")),
            ("locale.dateFormat", KeyDefault::Text("medium")),
            ("locale.numberFormat", KeyDefault::Text("locale")),
            ("locale.firstDay", KeyDefault::Text("locale")),
            ("backup.auto", KeyDefault::Bool(false)),
            ("backup.backend", KeyDefault::Text("auto")),
            ("backup.destination", KeyDefault::Text("")),
            ("backup.schedule", KeyDefault::Text("off")),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::General, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.since, 20, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
        }
        assert_eq!(KeyGroup::General.name(), "general");
        assert!(spec("updates.auto")
            .unwrap()
            .validate(&Value::Bool(false))
            .is_ok());
        assert!(spec("backup.schedule")
            .unwrap()
            .validate(&Value::Text("daily".into()))
            .is_ok());
        assert!(spec("backup.schedule")
            .unwrap()
            .validate(&Value::Text("sometimes".into()))
            .is_err());
    }

    /// The T227 Appearance/Quick Settings/Search/Screen Saver/Desktop keys,
    /// additive in revision 21.
    #[test]
    fn the_t227_keys_are_declared_in_revision_twenty_one() {
        for (key, group, kind) in [
            ("appearance.highlight", KeyGroup::Appearance, KeyType::Text),
            ("appearance.sidebarIconSize", KeyGroup::Appearance, KeyType::Text),
            ("appearance.wallpaperTint", KeyGroup::Appearance, KeyType::Bool),
            ("appearance.scrollBars", KeyGroup::Appearance, KeyType::Text),
            ("quickSettings.modules", KeyGroup::QuickSettings, KeyType::TextList),
            ("quickSettings.menuBarOnly", KeyGroup::QuickSettings, KeyType::TextList),
            ("search.categories", KeyGroup::Search, KeyType::TextList),
            ("search.indexLocations", KeyGroup::Search, KeyType::TextList),
            ("search.shortcut", KeyGroup::Search, KeyType::Text),
            ("search.web", KeyGroup::Search, KeyType::Bool),
            ("menu.control.search", KeyGroup::Menu, KeyType::Bool),
            ("screenSaver.style", KeyGroup::ScreenSaver, KeyType::Text),
            ("screenSaver.clock", KeyGroup::ScreenSaver, KeyType::Bool),
            ("screenSaver.showDate", KeyGroup::ScreenSaver, KeyType::Bool),
            ("screenSaver.showSeconds", KeyGroup::ScreenSaver, KeyType::Bool),
            ("desktop.showItems", KeyGroup::Desktop, KeyType::Text),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, group, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.since, 21, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        assert!(spec("appearance.sidebarIconSize")
            .unwrap()
            .validate(&Value::Text("large".into()))
            .is_ok());
        assert!(spec("appearance.sidebarIconSize")
            .unwrap()
            .validate(&Value::Text("huge".into()))
            .is_err());
        assert!(spec("screenSaver.style")
            .unwrap()
            .validate(&Value::Text("none".into()))
            .is_ok());
        assert!(spec("desktop.showItems")
            .unwrap()
            .validate(&Value::Text("in-stage".into()))
            .is_ok());
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
