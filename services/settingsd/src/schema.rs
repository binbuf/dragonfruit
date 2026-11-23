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
pub const SCHEMA_VERSION: u32 = 38;

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
    Focus,
    Lock,
    Printers,
    Network,
    General,
    QuickSettings,
    Search,
    ScreenSaver,
    Desktop,
    Accessibility,
    Privacy,
}

impl KeyGroup {
    /// Every group, in schema order.
    pub const ALL: [KeyGroup; 24] = [
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
        KeyGroup::Focus,
        KeyGroup::Lock,
        KeyGroup::Printers,
        KeyGroup::Network,
        KeyGroup::General,
        KeyGroup::QuickSettings,
        KeyGroup::Search,
        KeyGroup::ScreenSaver,
        KeyGroup::Desktop,
        KeyGroup::Accessibility,
        KeyGroup::Privacy,
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
            KeyGroup::Focus => "focus",
            KeyGroup::Lock => "lock",
            KeyGroup::Printers => "printers",
            KeyGroup::Network => "network",
            KeyGroup::General => "general",
            KeyGroup::QuickSettings => "quick-settings",
            KeyGroup::Search => "search",
            KeyGroup::ScreenSaver => "screen-saver",
            KeyGroup::Desktop => "desktop",
            KeyGroup::Accessibility => "accessibility",
            KeyGroup::Privacy => "privacy",
        }
    }
}

/// Extra meaning layered on a key's declared D-Bus type (T282).
///
/// The D-Bus type says how a value travels; the semantic says what the value
/// must mean. A time is a string on the wire but `HH:MM` in the store; a colour
/// is a string but `#rrggbb`; a path is a string but absolute. Validating these
/// at the one write path keeps a malformed preference from reaching a consumer
/// that would only discover it much later.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeySemantic {
    /// No extra rule beyond type/range/allowed.
    None,
    /// A 24-hour `HH:MM` wall-clock time.
    TimeOfDay,
    /// An empty string (the token default) or `#rrggbb`.
    Color,
    /// An empty string or an absolute path with no control characters.
    AbsolutePath,
    /// Quick Settings module ids from the one shell registry.
    ModuleList,
    /// Weekday ids (`mon`…`sun`).
    DayList,
    /// Keyboard layout ids: short, opaque, no whitespace or path separators.
    LayoutList,
    /// `from\tto` text-replacement entries.
    ReplacementList,
    /// Free text with a hard character bound (a lock message).
    BoundedText(usize),
    /// A shortcut chord with a hard character bound; empty disables it.
    Shortcut(usize),
}

impl KeySemantic {
    /// The Quick Settings inventory. Kept here as the daemon-side mirror of
    /// `shell/control-center/ControlCenterRegistry.qml` so a persisted page
    /// list cannot name a module the shell does not have.
    pub const MODULES: &'static [&'static str] = &[
        "wifi",
        "bluetooth",
        "airdrop",
        "vpn",
        "now-playing",
        "stage-manager",
        "edit-controls",
        "brightness",
        "dark",
        "keyboard-brightness",
        "focus",
        "volume",
        "airplay-audio",
        "battery",
        "clock",
        "lock-screen",
        "screen-mirroring",
        "capture-screen",
        "workspaces",
        "menu-bar",
        "accessibility",
        "home",
        "mission-control",
        "printers",
        "privacy",
        "keyboard",
        "storage",
        "users",
        "software-update",
        "screenshot",
        "shortcuts",
    ];

    /// The weekday ids the Focus schedule accepts.
    pub const DAYS: &'static [&'static str] =
        &["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
}

/// A 24-hour wall-clock time, `HH:MM` (00:00 through 23:59).
fn is_hh_mm(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 5 || bytes[2] != b':' {
        return false;
    }
    let digits = |slice: &[u8]| slice.iter().all(u8::is_ascii_digit);
    if !digits(&bytes[0..2]) || !digits(&bytes[3..5]) {
        return false;
    }
    let hour = (bytes[0] - b'0') * 10 + (bytes[1] - b'0');
    let minute = (bytes[3] - b'0') * 10 + (bytes[4] - b'0');
    hour <= 23 && minute <= 59
}

/// `#rrggbb`, lower or upper case.
fn is_rrggbb(text: &str) -> bool {
    text.len() == 7
        && text.starts_with('#')
        && text[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// Bound a free-text value and reject NUL or a line break. Tabs are left alone
/// for a replacement, but a message/hint is one line.
fn check_clean_text(text: &str, max: usize, key: &str) -> Result<(), SettingsError> {
    if text.chars().count() > max {
        return Err(SettingsError::Invalid {
            key: key.to_owned(),
            message: format!("is longer than {max} characters"),
        });
    }
    if text
        .chars()
        .any(|c| c == '\0' || c == '\n' || c == '\r')
    {
        return Err(SettingsError::Invalid {
            key: key.to_owned(),
            message: "contains a control character".to_owned(),
        });
    }
    Ok(())
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
    /// Extra meaning layered on the declared type (T282).
    pub semantic: KeySemantic,
    /// True when no shipped consumer reads this key yet; the Settings app
    /// stores it honestly and the pane says so. A flagged key is not a
    /// pretend-live control.
    pub stored_only: bool,
    /// One-line description for the docs and the Settings app.
    pub summary: &'static str,
}

impl KeySpec {
    /// Check a value against the declared type, range, alternatives, and
    /// semantic (T282). A non-finite number is rejected before any range
    /// check, because every comparison with NaN is false and would otherwise
    /// pass.
    pub fn validate(&self, value: &Value) -> Result<(), SettingsError> {
        if value.kind() != self.kind {
            return Err(SettingsError::TypeMismatch {
                key: self.key.to_owned(),
                expected: self.kind.name(),
                got: value.type_name(),
            });
        }
        match value {
            Value::Number(n) => {
                if !n.is_finite() {
                    return Err(SettingsError::NonFinite {
                        key: self.key.to_owned(),
                    });
                }
                self.check_range(*n)?;
            }
            Value::Integer(i) => self.check_range(*i as f64)?,
            Value::Text(text) => {
                if !self.allowed.is_empty() && !self.allowed.contains(&text.as_str()) {
                    return Err(SettingsError::NotAllowed {
                        key: self.key.to_owned(),
                        value: text.clone(),
                        allowed: self.allowed,
                    });
                }
                self.check_text_semantic(text)?;
            }
            Value::TextList(items) => self.check_list_semantic(items)?,
            Value::Bool(_) => {}
        }
        Ok(())
    }

    fn invalid(&self, message: impl Into<String>) -> SettingsError {
        SettingsError::Invalid {
            key: self.key.to_owned(),
            message: message.into(),
        }
    }

    fn check_text_semantic(&self, text: &str) -> Result<(), SettingsError> {
        match self.semantic {
            KeySemantic::None => Ok(()),
            KeySemantic::TimeOfDay => {
                if is_hh_mm(text) {
                    Ok(())
                } else {
                    Err(self.invalid(format!("{text:?} is not a 24-hour HH:MM time")))
                }
            }
            KeySemantic::Color => {
                if text.is_empty() || is_rrggbb(text) {
                    Ok(())
                } else {
                    Err(self.invalid(format!("{text:?} is not a #rrggbb colour")))
                }
            }
            KeySemantic::AbsolutePath => {
                if text.is_empty() {
                    return Ok(());
                }
                if !text.starts_with('/') {
                    return Err(self.invalid(format!("{text:?} is not an absolute path")));
                }
                check_clean_text(text, 4096, self.key)
            }
            KeySemantic::BoundedText(max) => check_clean_text(text, max, self.key),
            KeySemantic::Shortcut(max) => check_clean_text(text, max, self.key),
            KeySemantic::ModuleList
            | KeySemantic::DayList
            | KeySemantic::LayoutList
            | KeySemantic::ReplacementList => Ok(()),
        }
    }

    fn check_list_semantic(&self, items: &[String]) -> Result<(), SettingsError> {
        match self.semantic {
            KeySemantic::None
            | KeySemantic::TimeOfDay
            | KeySemantic::Color
            | KeySemantic::AbsolutePath
            | KeySemantic::BoundedText(_)
            | KeySemantic::Shortcut(_) => Ok(()),
            KeySemantic::ModuleList => {
                if items.len() > 64 {
                    return Err(self.invalid("more than 64 modules"));
                }
                for item in items {
                    if !KeySemantic::MODULES.contains(&item.as_str()) {
                        return Err(self.invalid(format!("unknown Quick Settings module {item:?}")));
                    }
                }
                Ok(())
            }
            KeySemantic::DayList => {
                for item in items {
                    if !KeySemantic::DAYS.contains(&item.as_str()) {
                        return Err(self.invalid(format!("unknown weekday {item:?}")));
                    }
                }
                Ok(())
            }
            KeySemantic::LayoutList => {
                if items.len() > 16 {
                    return Err(self.invalid("more than 16 keyboard layouts"));
                }
                for item in items {
                    let valid = !item.is_empty()
                        && item.len() <= 32
                        && item
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '+' | '-' | '(' | ')'));
                    if !valid {
                        return Err(self.invalid(format!("invalid keyboard layout {item:?}")));
                    }
                }
                Ok(())
            }
            KeySemantic::ReplacementList => {
                if items.len() > 500 {
                    return Err(self.invalid("more than 500 text replacements"));
                }
                for item in items {
                    let Some((from, to)) = item.split_once('\t') else {
                        return Err(self.invalid(format!(
                            "replacement {item:?} is not a `from<TAB>to` entry"
                        )));
                    };
                    if from.is_empty() || to.is_empty() || from.len() > 128 || to.len() > 128 {
                        return Err(self.invalid(format!(
                            "replacement {item:?} must have a 1-128 character source and target"
                        )));
                    }
                    if item.chars().any(|c| c == '\0' || c == '\n' || c == '\r') {
                        return Err(self.invalid("replacement contains a control character"));
                    }
                }
                Ok(())
            }
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 1,
        summary: "Vertical swipe opens Overview.",
    },
    // ── Trackpad gesture breadth (T280) ─────────────────────────────────
    // The More Gestures rows. The recognizer already owns the master switch,
    // the Space swipe, and Overview; these five name the remaining families
    // so the pane's rows are real preferences rather than aliases.
    KeySpec {
        key: "gestures.pages",
        group: KeyGroup::Gestures,
        kind: KeyType::Text,
        default: KeyDefault::Text("two-finger"),
        allowed: &["off", "two-finger", "three-finger"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input gesture recognizer does not read this family yet",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Finger count for the page-swipe gesture; off disables it.",
    },
    KeySpec {
        key: "gestures.fullscreen",
        group: KeyGroup::Gestures,
        kind: KeyType::Text,
        default: KeyDefault::Text("three-finger"),
        allowed: &["off", "three-finger", "four-finger"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input gesture recognizer does not read this family yet",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Finger count for the full-screen-app swipe; off disables it.",
    },
    KeySpec {
        key: "gestures.notifications",
        group: KeyGroup::Gestures,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input gesture recognizer does not read this family yet",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Two-finger swipe from the right edge opens Notifications.",
    },
    KeySpec {
        key: "gestures.showDesktop",
        group: KeyGroup::Gestures,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input gesture recognizer does not read this family yet",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Spread the thumb and three fingers to reveal the desktop.",
    },
    KeySpec {
        key: "gestures.appWindows",
        group: KeyGroup::Gestures,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input gesture recognizer does not read this family yet",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Swipe down with three fingers to show the app's windows.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::Color,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::AbsolutePath,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 2,
        summary: "Apply the selection to every Space, or only the active one.",
    },
    KeySpec {
        key: "wallpaper.slideshow",
        group: KeyGroup::Wallpaper,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: Some(0.0),
        max: Some(1440.0),
        owner: "apps/settings",
        consumer: "stored-only: the shell has no wallpaper-rotation timer yet",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 38,
        summary: "Minutes between automatic wallpaper changes; 0 keeps the selection fixed.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::AbsolutePath,
        stored_only: false,
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
        semantic: KeySemantic::AbsolutePath,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 12,
        summary: "Action when the Compose/Super key is pressed: show the emoji panel or nothing.",
    },
    // ── Input breadth (T229) ────────────────────────────────────────────
    // The remaining macOS pointer rows and the input-source/replacement/
    // dictation rows. The pointer keys join the revision-12 `input.*` family
    // and travel to the compositor in the same `set_pointer_policy` snapshot;
    // input sources are the first entry of a durable layout list the shell
    // forwards to the compositor keymap.
    KeySpec {
        key: "input.secondaryClick",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text("right"),
        allowed: &["right", "left"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input pointer buttons",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 22,
        summary: "Which button a secondary click uses: right (default) or left.",
    },
    KeySpec {
        key: "input.scrollSpeed",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.0),
        allowed: &[],
        min: Some(-1.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "compositor/input pointer scrolling",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 22,
        summary: "Pointer scroll speed, -1.0 (slow) to 1.0 (fast); 0 is neutral.",
    },
    KeySpec {
        key: "input.doubleClickSpeed",
        group: KeyGroup::Input,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(400),
        allowed: &[],
        min: Some(100.0),
        max: Some(900.0),
        owner: "apps/settings",
        consumer: "compositor/input pointer double-click interval",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 22,
        summary: "Maximum milliseconds between two clicks for a double-click.",
    },
    KeySpec {
        key: "input.haptics",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input trackpad haptics (hardware bridge deferred)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 22,
        summary: "Haptic feedback for a trackpad that supports it.",
    },
    KeySpec {
        key: "input.lookUp",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text("force-click"),
        allowed: &["force-click", "three-finger", "off"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input gesture mapping",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 22,
        summary: "How a trackpad look-up gesture is triggered; off disables it.",
    },
    KeySpec {
        key: "input.sources",
        group: KeyGroup::Input,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/compositor keymap",
        semantic: KeySemantic::LayoutList,
        stored_only: false,
        since: 22,
        summary: "Ordered keyboard layout ids; the first is the active source. Empty follows the host keymap.",
    },
    KeySpec {
        key: "input.replacements",
        group: KeyGroup::Input,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "input method (T253)",
        semantic: KeySemantic::ReplacementList,
        stored_only: true,
        since: 22,
        summary: "Text replacements as `from\\tto` entries for the session input method.",
    },
    KeySpec {
        key: "input.dictation",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "speech-to-text delegate (when installed)",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 22,
        summary: "Enable dictation when a speech-to-text service is installed.",
    },
    // ── Dictation options (T280) ────────────────────────────────────────
    // The delegate owns recognition; these four preferences select the
    // language, microphone, shortcut, and punctuation behavior it reads.
    KeySpec {
        key: "input.dictationLanguage",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "speech-to-text delegate (T253)",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "BCP-47 language id for dictation; empty follows the session locale.",
    },
    KeySpec {
        key: "input.dictationMicrophone",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "speech-to-text delegate (T253)",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Input device name for dictation; empty uses the default source.",
    },
    KeySpec {
        key: "input.dictationShortcut",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shortcut engine (T245a)",
        semantic: KeySemantic::Shortcut(64),
        stored_only: true,
        since: 37,
        summary: "Keyboard shortcut that starts dictation; empty uses the delegate default.",
    },
    KeySpec {
        key: "input.dictationAutoPunctuation",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "speech-to-text delegate (T253)",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Let the dictation delegate insert punctuation automatically.",
    },
    // ── Mouse-specific pointer keys (T280) ──────────────────────────────
    // A mouse and a trackpad are separate devices with separate preferences;
    // the shared `input.pointerSpeed` family is the legacy fallback only.
    KeySpec {
        key: "input.mouse.pointerSpeed",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.0),
        allowed: &[],
        min: Some(-1.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse pointer acceleration bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Mouse tracking speed, -1.0 (slow) to 1.0 (fast); 0 is neutral.",
    },
    KeySpec {
        key: "input.mouse.naturalScroll",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse scrolling bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Natural scrolling for the mouse wheel.",
    },
    KeySpec {
        key: "input.mouse.secondaryClick",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text("right"),
        allowed: &["right", "left"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse buttons bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Which mouse button a secondary click uses.",
    },
    KeySpec {
        key: "input.mouse.smartZoom",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse gestures bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Double-tap with one finger to smart-zoom the content under the pointer.",
    },
    KeySpec {
        key: "input.mouse.pointerAcceleration",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse acceleration bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Apply pointer acceleration to mouse motion; off uses a flat curve.",
    },
    KeySpec {
        key: "input.mouse.leftHanded",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse buttons bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Swap the mouse's primary and secondary buttons.",
    },
    KeySpec {
        key: "input.mouse.scrollSpeed",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.0),
        allowed: &[],
        min: Some(-1.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse scrolling bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Mouse wheel scroll speed, -1.0 (slow) to 1.0 (fast).",
    },
    KeySpec {
        key: "input.mouse.scrollMethod",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text("button"),
        allowed: &["two-finger", "edge", "button"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse scrolling bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "How the mouse scrolls; a wheel reports button scrolling.",
    },
    KeySpec {
        key: "input.mouse.doubleClickSpeed",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(400.0),
        allowed: &[],
        min: Some(100.0),
        max: Some(900.0),
        owner: "apps/settings",
        consumer: "stored-only: compositor/input mouse double-click interval bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Maximum milliseconds between two mouse clicks for a double-click.",
    },
    // ── Trackpad-specific pointer keys (T280) ───────────────────────────
    KeySpec {
        key: "input.trackpad.pointerSpeed",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.0),
        allowed: &[],
        min: Some(-1.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "stored-only: compositor/input trackpad pointer acceleration bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Trackpad tracking speed, -1.0 (slow) to 1.0 (fast); 0 is neutral.",
    },
    KeySpec {
        key: "input.trackpad.naturalScroll",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input trackpad scrolling bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Natural scrolling for the trackpad.",
    },
    KeySpec {
        key: "input.trackpad.secondaryClick",
        group: KeyGroup::Input,
        kind: KeyType::Text,
        default: KeyDefault::Text("right"),
        allowed: &["right", "left"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input trackpad buttons bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Which button a trackpad secondary click uses.",
    },
    KeySpec {
        key: "input.trackpad.clickPressure",
        group: KeyGroup::Input,
        kind: KeyType::Number,
        default: KeyDefault::Number(0.5),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "stored-only: compositor/input trackpad pressure bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Trackpad click pressure, 0.0 (light) to 1.0 (firm).",
    },
    KeySpec {
        key: "input.trackpad.smartZoom",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input trackpad gestures bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Double-tap with two fingers to smart-zoom the content under the pointer.",
    },
    KeySpec {
        key: "input.trackpad.pinchZoom",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input trackpad gestures bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Pinch with two fingers to zoom.",
    },
    KeySpec {
        key: "input.trackpad.rotate",
        group: KeyGroup::Input,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: compositor/input trackpad gestures bridge pending",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 37,
        summary: "Rotate with two fingers to rotate the content.",
    },
    // ── Battery options (T229) ──────────────────────────────────────────
    KeySpec {
        key: "power.showPercentage",
        group: KeyGroup::Session,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar battery item",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 22,
        summary: "Show the battery percentage beside the menu-bar glyph.",
    },
    KeySpec {
        key: "power.historyRange",
        group: KeyGroup::Session,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: Some(0.0),
        max: Some(1.0),
        owner: "apps/settings",
        consumer: "apps/settings Energy pane",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 22,
        summary: "Battery usage range: 0 = last 24 hours, 1 = last 10 days.",
    },
    KeySpec {
        key: "power.dimOnBattery",
        group: KeyGroup::Session,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/display forwarder",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 30,
        summary: "Reduce the display brightness while the machine runs on battery.",
    },
    KeySpec {
        key: "power.preventSleep",
        group: KeyGroup::Session,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/idle controller",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 30,
        summary: "Keep the session awake by disabling the automatic suspend stage.",
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
        consumer: "compositor/idle timer, shell/idle effects",
        semantic: KeySemantic::None,
        stored_only: false,
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
        consumer: "compositor/idle timer, shell/idle effects",
        semantic: KeySemantic::None,
        stored_only: false,
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
        consumer: "compositor/idle timer, shell/idle effects",
        semantic: KeySemantic::None,
        stored_only: false,
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
        consumer: "compositor/idle timer, shell/idle effects, session/suspend",
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 11,
        summary: "Output balance from Left (0.0) to Right (1.0); 0.5 is centered.",
    },
    // ── Overview / hot corners (T-15.5b) ─────────────────────────
    // Overview and hot corners are compositor-native: the compositor
    // owns the one overview machine and the corner detector, and the shell
    // mirrors both over `df_toplevel_manager`. These keys are the durable
    // trigger *configuration* the Overview pane writes; the action ids
    // are the `HotCornerAction::id()` spellings the compositor's
    // `HotCornerConfig` uses. T276 adds the per-corner modifier and
    // `overview.switchToAppSpace`; the shell forwards the whole snapshot in
    // the append-only v13 `set_overview_policy` request (ADR 0126/0277).
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
            "launchpad",
            "control-center",
            "show-dock",
            "menu-bar",
            "focus-dock",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
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
            "launchpad",
            "control-center",
            "show-dock",
            "menu-bar",
            "focus-dock",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
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
            "launchpad",
            "control-center",
            "show-dock",
            "menu-bar",
            "focus-dock",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
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
            "launchpad",
            "control-center",
            "show-dock",
            "menu-bar",
            "focus-dock",
        ],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 13,
        summary: "Action assigned to the bottom-right hot corner; `none` disables it.",
    },
    KeySpec {
        key: "overview.hotCornerTopLeftModifier",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("none"),
        allowed: &["none", "command", "shift", "option", "control"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 33,
        summary: "Modifier held while the pointer enters the top-left corner; `none` needs no modifier.",
    },
    KeySpec {
        key: "overview.hotCornerTopRightModifier",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("none"),
        allowed: &["none", "command", "shift", "option", "control"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 33,
        summary: "Modifier held while the pointer enters the top-right corner; `none` needs no modifier.",
    },
    KeySpec {
        key: "overview.hotCornerBottomLeftModifier",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("none"),
        allowed: &["none", "command", "shift", "option", "control"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 33,
        summary: "Modifier held while the pointer enters the bottom-left corner; `none` needs no modifier.",
    },
    KeySpec {
        key: "overview.hotCornerBottomRightModifier",
        group: KeyGroup::Overview,
        kind: KeyType::Text,
        default: KeyDefault::Text("none"),
        allowed: &["none", "command", "shift", "option", "control"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/input hot corners",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 33,
        summary: "Modifier held while the pointer enters the bottom-right corner; `none` needs no modifier.",
    },
    KeySpec {
        key: "overview.switchToAppSpace",
        group: KeyGroup::Overview,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/workspace activation",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 33,
        summary: "Activating an app window on another Space switches to that Space.",
    },
    // ── Notifications presentation (T-15.7b) ────────────────────────────
    // The four global "Notifications" preferences the Notifications
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 14,
        summary: "Show notification banners while mirroring or sharing the display.",
    },
    KeySpec {
        key: "notifications.appOptions",
        group: KeyGroup::Notifications,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (stored policy)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 35,
        summary: "Per-app notification options the service does not own yet (JSON, keyed by app).",
    },
    // ── Focus schedule and shortcut (T229a) ─────────────────────────────
    // The Focus *mode* and the per-app allow list stay on the notification
    // service (ADR 0058/0130). These keys are the durable schedule and
    // shortcut the shell's focus scheduler reads; the pane writes them and
    // the shell applies the mode at the window boundary (ADR 0239).
    KeySpec {
        key: "focus.schedule.enabled",
        group: KeyGroup::Focus,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/FocusSchedule",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 23,
        summary: "Enable the daily Focus schedule.",
    },
    KeySpec {
        key: "focus.schedule.start",
        group: KeyGroup::Focus,
        kind: KeyType::Text,
        default: KeyDefault::Text("22:00"),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/FocusSchedule",
        semantic: KeySemantic::TimeOfDay,
        stored_only: false,
        since: 23,
        summary: "Focus schedule start time (24-hour HH:MM).",
    },
    KeySpec {
        key: "focus.schedule.end",
        group: KeyGroup::Focus,
        kind: KeyType::Text,
        default: KeyDefault::Text("07:00"),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/FocusSchedule",
        semantic: KeySemantic::TimeOfDay,
        stored_only: false,
        since: 23,
        summary: "Focus schedule end time (24-hour HH:MM); an earlier end crosses midnight.",
    },
    KeySpec {
        key: "focus.schedule.days",
        group: KeyGroup::Focus,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[
            "mon", "tue", "wed", "thu", "fri", "sat", "sun",
        ]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/FocusSchedule",
        semantic: KeySemantic::DayList,
        stored_only: false,
        since: 23,
        summary: "Weekdays the Focus schedule runs (mon…sun); empty means every day.",
    },
    KeySpec {
        key: "focus.shortcut",
        group: KeyGroup::Focus,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell shortcuts (registration deferred)",
        semantic: KeySemantic::Shortcut(64),
        stored_only: true,
        since: 23,
        summary: "Global shortcut that toggles Focus; empty disables it.",
    },
    KeySpec {
        key: "focus.customProfiles",
        group: KeyGroup::Focus,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Focus pane",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 35,
        summary: "User-added Focus names layered over the service's custom profile.",
    },
    KeySpec {
        key: "focus.customProfile",
        group: KeyGroup::Focus,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Focus pane",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 35,
        summary: "The active user-added Focus name; empty means the built-in Custom profile.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::BoundedText(120),
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 15,
        summary: "Show the Sleep, Restart, and Shut Down buttons on the lock screen.",
    },
    KeySpec {
        key: "lock.passwordHint",
        group: KeyGroup::Lock,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/LockScreen",
        semantic: KeySemantic::BoundedText(120),
        stored_only: false,
        since: 23,
        summary: "The hint shown under the password field when password hints are on.",
    },
    KeySpec {
        key: "login.windowStyle",
        group: KeyGroup::Lock,
        kind: KeyType::Text,
        default: KeyDefault::Text("list"),
        allowed: &["list", "name-and-password"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "display manager adapter (not exposed; row disabled)",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 23,
        summary: "Login-window style: a user list or name-and-password fields.",
    },
    KeySpec {
        key: "lock.requirePasswordAfter",
        group: KeyGroup::Lock,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(300),
        allowed: &[],
        min: Some(-1.0),
        max: Some(86400.0),
        owner: "apps/settings",
        consumer: "apps/settings Lock Screen pane (derives idle.lock)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 35,
        summary: "Seconds after display-off before the password is required; -1 never, 0 immediately.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        consumer: "apps/settings, shell/MenuBar system menu",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 16,
        summary: "How many recent documents, applications, and servers to remember.",
    },
    KeySpec {
        key: "menu.osGlyph",
        group: KeyGroup::Menu,
        kind: KeyType::Text,
        default: KeyDefault::Text("dragonfruit"),
        allowed: &["dragonfruit", "tux"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar system menu",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 26,
        summary: "The OS menu glyph at the far left of the menu bar: the Dragonfruit mark or the Tux mark.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 16,
        summary: "Show the Accessibility control in the menu bar when it is available.",
    },
    // ── Menu-bar extras registry (T231) ─────────────────────────────────
    // The three extras that became registry items in T231. The existing
    // `menu.control.search` key now has a live consumer (the Search item);
    // these add the clock, Notifications and Quick Settings visibility
    // choices. The pane rows are T233's; the keys are the durable contract.
    KeySpec {
        key: "menu.control.clock",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 24,
        summary: "Show the Date & Time (clock) item in the menu bar.",
    },
    KeySpec {
        key: "menu.control.notifications",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 24,
        summary: "Show Notifications item in the menu bar when the service is present.",
    },
    KeySpec {
        key: "menu.control.controlCenter",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 24,
        summary: "Show the Quick Settings item in the menu bar.",
    },
    // ── T277: per-control options, tray allowances, Linux-only rows ─────
    // These are the durable options the Menu Bar pane's per-control sheets
    // and Linux-only rows own. They never add a second visibility path: the
    // `menu.control.*` key still decides whether the control is offered.
    KeySpec {
        key: "menu.control.focusAlways",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 34,
        summary: "Show the Focus mark even when no Focus mode is active.",
    },
    KeySpec {
        key: "menu.control.volumeWhenActive",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 34,
        summary: "Hide the Sound mark while the default output is muted.",
    },
    KeySpec {
        key: "menu.control.workspaces",
        group: KeyGroup::Menu,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 34,
        summary: "Show the active workspace indicator in the menu bar.",
    },
    KeySpec {
        key: "menu.overflow",
        group: KeyGroup::Menu,
        kind: KeyType::Text,
        default: KeyDefault::Text("automatic"),
        allowed: &["automatic", "always"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 34,
        summary: "When the status row overflows: only when the measured bar is full, or always.",
    },
    KeySpec {
        key: "menu.tray.hidden",
        group: KeyGroup::Menu,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/MenuBar status row",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 34,
        summary: "Third-party StatusNotifier item names the user has removed from the menu bar.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        // T271: stored-only until the shell grows a Wi-Fi prompt consumer.
        // NetworkManager's own autoconnect remains authoritative.
        consumer: "stored-only: no shell Wi-Fi prompt consumer yet",
        semantic: KeySemantic::None,
        stored_only: true,
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
        consumer: "stored-only: no shell Wi-Fi prompt consumer yet",
        semantic: KeySemantic::None,
        stored_only: true,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 20,
        summary: "Install the updates a check finds automatically; never enabled by default.",
    },
    KeySpec {
        key: "defaults.passwordManager",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Autofill & Passwords provider selection",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 28,
        summary: "The installed password manager Autofill delegates to; empty follows the active provider. No credential or secret is stored.",
    },
    KeySpec {
        key: "defaults.browser",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings default-apps delegate (xdg-settings; settings portal T258)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 33,
        summary: "Desktop id of the default web browser; empty follows the host default. The delegate writes the freedesktop default-apps entry.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 20,
        summary: "The first day of the week the desktop calendar uses.",
    },
    KeySpec {
        key: "locale.languages",
        group: KeyGroup::General,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings (preferred-language order and preview)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 29,
        summary: "The ordered preferred languages the Language & Region pane keeps; the first entry is the system locale the host is asked to apply.",
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
        semantic: KeySemantic::None,
        stored_only: true,
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
        semantic: KeySemantic::None,
        stored_only: true,
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
        semantic: KeySemantic::AbsolutePath,
        stored_only: true,
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
        semantic: KeySemantic::None,
        stored_only: true,
        since: 20,
        summary: "How often the selected backup backend runs.",
    },
    // ── System Settings window state (T263) ──────────────────────────────
    // The shell's remembered last pane and the host window's geometry. A
    // fresh profile has an empty pane (first launch opens General) and the
    // measured 740x820 size; `positioned` is false until the user moves the
    // window, so the compositor's default placement is not overridden by a
    // meaningless (0,0) position.
    KeySpec {
        key: "settings.lastPane",
        group: KeyGroup::General,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings shell (last pane)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 27,
        summary: "The last visited System Settings pane id; empty opens General.",
    },
    KeySpec {
        key: "settings.window.width",
        group: KeyGroup::General,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(740),
        allowed: &[],
        min: Some(720.0),
        max: Some(10000.0),
        owner: "apps/settings",
        consumer: "apps/settings window",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 27,
        summary: "Remembered System Settings window width in points.",
    },
    KeySpec {
        key: "settings.window.height",
        group: KeyGroup::General,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(820),
        allowed: &[],
        min: Some(480.0),
        max: Some(10000.0),
        owner: "apps/settings",
        consumer: "apps/settings window",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 27,
        summary: "Remembered System Settings window height in points.",
    },
    KeySpec {
        key: "settings.window.x",
        group: KeyGroup::General,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings window",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 27,
        summary: "Remembered System Settings window x position; used only when positioned is true.",
    },
    KeySpec {
        key: "settings.window.y",
        group: KeyGroup::General,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings window",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 27,
        summary: "Remembered System Settings window y position; used only when positioned is true.",
    },
    KeySpec {
        key: "settings.window.positioned",
        group: KeyGroup::General,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings window",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 27,
        summary: "Whether the remembered x/y position has been set by the user.",
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
        semantic: KeySemantic::Color,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 21,
        summary: "When scroll bars show: automatically, only while scrolling, or always.",
    },
    // ── Appearance completion (T274) ─────────────────────────────────────
    // The macOS Appearance rows that need a durable preference: the track
    // click, the Files folder colour, and the Linux icon/cursor adapters.
    // `iconStyle` and `iconTheme` share one provider consumer; an explicit
    // theme wins over the style mapping.
    KeySpec {
        key: "appearance.scrollBarClick",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text("jump-to-next-page"),
        allowed: &["jump-to-spot", "jump-to-next-page"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system ScrollView, apps/settings ScrollView",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Scroll-bar track click: jump to the clicked spot or the next page.",
    },
    KeySpec {
        key: "appearance.folderColor",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Theme, apps/files Theme",
        semantic: KeySemantic::Color,
        stored_only: false,
        since: 31,
        summary: "Folder glyph colour override ('#rrggbb'); empty keeps the shipped artwork.",
    },
    KeySpec {
        key: "appearance.iconStyle",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text("default"),
        allowed: &["default", "dark", "clear", "tinted"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings icon-theme provider",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Icon and widget style, mapped to an installed icon theme.",
    },
    KeySpec {
        key: "appearance.iconTheme",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings icon-theme provider, shell/app-index icon lookup",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Explicit installed icon theme; empty follows the icon style mapping.",
    },
    KeySpec {
        key: "appearance.cursorTheme",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/cursor theme (next session)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Cursor theme name; the compositor reads it at the next session start.",
    },
    KeySpec {
        key: "appearance.cursorSize",
        group: KeyGroup::Appearance,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(24),
        allowed: &[],
        min: Some(16.0),
        max: Some(64.0),
        owner: "apps/settings",
        consumer: "compositor/cursor theme (next session)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Cursor size in logical pixels, 16 to 64; applied at the next session start.",
    },
    KeySpec {
        key: "appearance.windowControls",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text("left"),
        allowed: &["left", "right"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/window decoration, session GTK button-layout",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Window control side; the compositor keeps traffic lights on the left today.",
    },
    KeySpec {
        key: "appearance.material",
        group: KeyGroup::Appearance,
        kind: KeyType::Text,
        default: KeyDefault::Text("auto"),
        allowed: &["auto", "full", "reduced", "minimal"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Glass (degrade budget owns the effective tier)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Material effects preference; auto follows the GPU budget and rollout policy.",
    },
    KeySpec {
        key: "appearance.accentFromWallpaper",
        group: KeyGroup::Appearance,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Theme (wallpaper color sampler)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 31,
        summary: "Derive the accent from the wallpaper when no explicit accent is set.",
    },
    // ── Quick Settings customization (T227) ──────────────────────────────
    // The ordered module ids the Quick Settings panel renders. An empty list
    // means the built-in order. `menuBarOnly` names modules that stay in the
    // menu bar but are removed from the panel.
    KeySpec {
        key: "quickSettings.modules",
        group: KeyGroup::QuickSettings,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[
            "wifi", "bluetooth", "now-playing", "airdrop", "stage-manager",
            "screen-mirroring", "dark", "capture-screen", "focus",
            "brightness", "volume", "airplay-audio", "edit-controls",
        ]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center",
        semantic: KeySemantic::ModuleList,
        stored_only: false,
        since: 21,
        summary: "Ordered Quick Settings module ids; empty means the built-in order.",
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 21,
        summary: "Module ids that stay in the menu bar and are hidden from the Quick Settings panel.",
    },
    // ── T235: measured grid, pages and per-display policy ────────────────
    KeySpec {
        key: "quickSettings.page2",
        group: KeyGroup::QuickSettings,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center",
        semantic: KeySemantic::ModuleList,
        stored_only: false,
        since: 25,
        summary: "Ordered module ids on the second Quick Settings page.",
    },
    KeySpec {
        key: "quickSettings.page3",
        group: KeyGroup::QuickSettings,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center",
        semantic: KeySemantic::ModuleList,
        stored_only: false,
        since: 25,
        summary: "Ordered module ids on the third Quick Settings page.",
    },
    KeySpec {
        key: "quickSettings.pageCount",
        group: KeyGroup::QuickSettings,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(1),
        allowed: &[],
        min: Some(1.0),
        max: Some(3.0),
        owner: "apps/settings",
        consumer: "shell/control-center",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 25,
        summary: "How many Quick Settings pages exist (1-3).",
    },
    KeySpec {
        key: "quickSettings.activePage",
        group: KeyGroup::QuickSettings,
        kind: KeyType::Integer,
        default: KeyDefault::Integer(0),
        allowed: &[],
        min: Some(0.0),
        max: Some(2.0),
        owner: "apps/settings",
        consumer: "shell/control-center",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 25,
        summary: "The Quick Settings page shown when the panel opens.",
    },
    KeySpec {
        key: "quickSettings.perDisplay",
        group: KeyGroup::QuickSettings,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 25,
        summary: "Show Quick Settings on every display instead of the active display only.",
    },
    KeySpec {
        key: "quickSettings.spans",
        group: KeyGroup::QuickSettings,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/control-center",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 25,
        summary: "Per-module measured span overrides, encoded as id=1x1|2x1|2x2|4x1.",
    },
    KeySpec {
        key: "quickSettings.pageNames",
        group: KeyGroup::QuickSettings,
        kind: KeyType::TextList,
        default: KeyDefault::List(&[]),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Quick Settings pane",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 34,
        summary: "User-facing names for the Quick Settings pages; empty entries fall back to Page N.",
    },
    // ── Search (T227) ────────────────────────────────────────────────────
    // Durable Search-equivalent preferences. T246 owns the search service
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
        semantic: KeySemantic::None,
        stored_only: true,
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
        semantic: KeySemantic::None,
        stored_only: true,
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
        semantic: KeySemantic::Shortcut(64),
        stored_only: true,
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
        semantic: KeySemantic::None,
        stored_only: true,
        since: 21,
        summary: "Offer a web-search result for the query using the default browser.",
    },
    KeySpec {
        key: "search.clipboardHistory",
        group: KeyGroup::Search,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "stored-only: shell clipboard history always observes; no preference consumer yet",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 38,
        summary: "Keep the desktop clipboard history available in Quick Settings.",
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
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
        semantic: KeySemantic::None,
        stored_only: false,
        since: 21,
        summary: "Show desktop items on the desktop or leave them to Stage Manager.",
    },
    // ── Accessibility (T275) ─────────────────────────────────────────────
    // The pane's live accommodation preferences. The delegated rows (screen
    // reader, captions, switch control, live speech) have no settingsd key:
    // their owner is an external service and the pane names it. Rows with no
    // live consumer are disabled with a reason instead of storing a key.
    KeySpec {
        key: "accessibility.textSize",
        group: KeyGroup::Accessibility,
        kind: KeyType::Text,
        default: KeyDefault::Text("default"),
        allowed: &["small", "default", "large", "extra-large"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Theme",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 32,
        summary: "Interface text size: one design-system type scale (small, default, large, extra-large).",
    },
    KeySpec {
        key: "accessibility.differentiateWithoutColor",
        group: KeyGroup::Accessibility,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/design-system Theme/Toggle",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 32,
        summary: "Add a non-colour state cue to controls (the checked toggle carries a check glyph).",
    },
    KeySpec {
        key: "accessibility.dimFlashingLights",
        group: KeyGroup::Accessibility,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/Dock attention bounce",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 32,
        summary: "Reduce the amplitude of attention-seeking flashes and bounces.",
    },
    KeySpec {
        key: "accessibility.magnifier",
        group: KeyGroup::Accessibility,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/compositor policy, compositor/magnifier",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 32,
        summary: "Enable the compositor's whole-output screen magnifier.",
    },
    KeySpec {
        key: "accessibility.magnifierZoom",
        group: KeyGroup::Accessibility,
        kind: KeyType::Number,
        default: KeyDefault::Number(2.0),
        allowed: &[],
        min: Some(1.0),
        max: Some(8.0),
        owner: "apps/settings",
        consumer: "compositor/magnifier",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 32,
        summary: "Magnifier zoom factor, 1.0 (off) to 8.0.",
    },
    KeySpec {
        key: "accessibility.magnifierMode",
        group: KeyGroup::Accessibility,
        kind: KeyType::Text,
        default: KeyDefault::Text("follow-focus"),
        allowed: &["follow-focus", "follow-caret"],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "compositor/magnifier",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 32,
        summary: "What the magnifier keeps in view: the focused window or its caret.",
    },
    KeySpec {
        key: "accessibility.shortcut",
        group: KeyGroup::Accessibility,
        kind: KeyType::Text,
        default: KeyDefault::Text(""),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "shell/compositor shortcut engine",
        semantic: KeySemantic::Shortcut(64),
        stored_only: false,
        since: 32,
        summary: "Recorded chord that opens the Accessibility pane; empty disables it.",
    },
    // ── Privacy policy (T279) ───────────────────────────────────────────
    // The portal PermissionStore has no global switch and no analytics
    // collector. The master gate is the desktop's own policy: when it is off
    // the pane disables every per-app location write. Analytics is opt-in and
    // stored-only until a collector exists (ADR 0262's stored-only rule).
    KeySpec {
        key: "privacy.locationServices",
        group: KeyGroup::Privacy,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(true),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Privacy pane (gates per-app location writes)",
        semantic: KeySemantic::None,
        stored_only: false,
        since: 36,
        summary: "Desktop-level Location Services master; off disables every per-app location write.",
    },
    KeySpec {
        key: "privacy.analytics",
        group: KeyGroup::Privacy,
        kind: KeyType::Bool,
        default: KeyDefault::Bool(false),
        allowed: &[],
        min: None,
        max: None,
        owner: "apps/settings",
        consumer: "apps/settings Privacy pane (stored-only; no collector in this build)",
        semantic: KeySemantic::None,
        stored_only: true,
        since: 36,
        summary: "Opt-in analytics preference; off by default and stored only.",
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

    /// The Overview / hot corners keys (T-15.5b): the four corner
    /// assignments, additive in revision 13. The gesture trio that also opens
    /// Overview stays the revision-1 `gestures.*` keys.
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

    /// Notifications presentation keys (T-15.7b): the four global
    /// Notifications preferences Notifications pane owns, additive
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

    /// T231 adds the three menu-bar extras that became registry items: the
    /// Date & Time clock, Notifications and Quick Settings. They are bool
    /// visibility keys in the existing `menu.control.*` family; the Search key
    /// (`menu.control.search`, revision 21) now has a live consumer too.
    #[test]
    fn the_menu_bar_extras_registry_keys_are_declared_in_revision_twenty_four() {
        for key in [
            "menu.control.clock",
            "menu.control.notifications",
            "menu.control.controlCenter",
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Menu, "{key}");
            assert_eq!(spec.kind, KeyType::Bool, "{key}");
            assert_eq!(spec.default, KeyDefault::Bool(true), "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.consumer, "shell/MenuBar status row", "{key}");
            assert_eq!(spec.since, 24, "{key}");
            assert!(spec.validate(&Value::Bool(false)).is_ok(), "{key}");
            assert!(
                spec.validate(&Value::Text("on".into())).is_err(),
                "{key}"
            );
        }
        assert_eq!(
            spec("menu.control.search").map(|spec| spec.since),
            Some(21)
        );
    }

    /// The OS-menu glyph choice is a new Menu key in revision 26. The default
    /// keeps the Dragonfruit mark, and the only alternate is the Tux mark.
    #[test]
    fn the_menu_os_glyph_key_is_declared_in_revision_twenty_six() {
        let spec = spec("menu.osGlyph").expect("menu.osGlyph is declared");
        assert_eq!(spec.group, KeyGroup::Menu);
        assert_eq!(spec.kind, KeyType::Text);
        assert_eq!(spec.default, KeyDefault::Text("dragonfruit".into()));
        assert_eq!(spec.owner, "apps/settings");
        assert_eq!(spec.consumer, "shell/MenuBar system menu");
        assert_eq!(spec.since, 26);
        assert!(spec.since <= SCHEMA_VERSION);
        for value in ["dragonfruit", "tux"] {
            assert!(spec.validate(&Value::Text(value.into())).is_ok(), "{value}");
        }
        assert!(spec.validate(&Value::Text("penguin".into())).is_err());
    }

    /// T277 completes the Quick Settings and Menu Bar management surfaces.
    /// The page names, the per-control options, the tray allowance list, and
    /// the Linux-only overflow/workspace rows are additive in revision 34.
    #[test]
    fn the_t277_management_keys_are_declared_in_revision_thirty_four() {
        for (key, kind, default) in [
            (
                "quickSettings.pageNames",
                KeyType::TextList,
                KeyDefault::List(&[]),
            ),
            (
                "menu.control.focusAlways",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "menu.control.volumeWhenActive",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "menu.control.workspaces",
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            (
                "menu.overflow",
                KeyType::Text,
                KeyDefault::Text("automatic"),
            ),
            ("menu.tray.hidden", KeyType::TextList, KeyDefault::List(&[])),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 34, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        let overflow = spec("menu.overflow").unwrap();
        for value in ["automatic", "always"] {
            assert!(
                overflow.validate(&Value::Text(value.into())).is_ok(),
                "{value}"
            );
        }
        assert!(overflow.validate(&Value::Text("sometimes".into())).is_err());
    }

    /// T278 completes the personalization/attention panes. The per-app
    /// notification option blob, the user-added Focus names, and the
    /// after-display-off password delay are additive in revision 35.
    #[test]
    fn the_t278_personalization_keys_are_declared_in_revision_thirty_five() {
        for (key, kind, default) in [
            (
                "notifications.appOptions",
                KeyType::Text,
                KeyDefault::Text(""),
            ),
            (
                "focus.customProfiles",
                KeyType::TextList,
                KeyDefault::List(&[]),
            ),
            ("focus.customProfile", KeyType::Text, KeyDefault::Text("")),
            (
                "lock.requirePasswordAfter",
                KeyType::Integer,
                KeyDefault::Integer(300),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 35, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        let after = spec("lock.requirePasswordAfter").unwrap();
        assert_eq!(after.min, Some(-1.0));
        assert!(after.validate(&Value::Integer(-1)).is_ok());
        assert!(after.validate(&Value::Integer(0)).is_ok());
        assert!(after.validate(&Value::Integer(86400)).is_ok());
        assert!(after.validate(&Value::Integer(-2)).is_err());
        assert!(after.validate(&Value::Integer(86401)).is_err());
    }

    /// The glass preference and its two accessibility overrides are additive
    /// in revision 18 (T196). The slider is a bounded double with the measured
    /// midpoint as its default; both overrides are off by default. These are
    /// the settings defaults for the shipped replacement glass (ADR 0234).
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

    /// The T229 input breadth and battery-option keys, additive in revision
    /// 22.
    #[test]
    fn the_t229_keys_are_declared_in_revision_twenty_two() {
        for (key, group, kind, default) in [
            (
                "input.secondaryClick",
                KeyGroup::Input,
                KeyType::Text,
                KeyDefault::Text("right"),
            ),
            (
                "input.scrollSpeed",
                KeyGroup::Input,
                KeyType::Number,
                KeyDefault::Number(0.0),
            ),
            (
                "input.doubleClickSpeed",
                KeyGroup::Input,
                KeyType::Integer,
                KeyDefault::Integer(400),
            ),
            (
                "input.haptics",
                KeyGroup::Input,
                KeyType::Bool,
                KeyDefault::Bool(true),
            ),
            (
                "input.lookUp",
                KeyGroup::Input,
                KeyType::Text,
                KeyDefault::Text("force-click"),
            ),
            (
                "input.sources",
                KeyGroup::Input,
                KeyType::TextList,
                KeyDefault::List(&[]),
            ),
            (
                "input.replacements",
                KeyGroup::Input,
                KeyType::TextList,
                KeyDefault::List(&[]),
            ),
            (
                "input.dictation",
                KeyGroup::Input,
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "power.showPercentage",
                KeyGroup::Session,
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "power.historyRange",
                KeyGroup::Session,
                KeyType::Integer,
                KeyDefault::Integer(0),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, group, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 22, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        assert!(spec("input.secondaryClick")
            .unwrap()
            .validate(&Value::Text("left".into()))
            .is_ok());
        assert!(spec("input.secondaryClick")
            .unwrap()
            .validate(&Value::Text("middle".into()))
            .is_err());
        assert!(spec("input.lookUp")
            .unwrap()
            .validate(&Value::Text("off".into()))
            .is_ok());
    }

    /// The T229a Focus schedule/shortcut and Lock Screen hint/login keys,
    /// additive in revision 23.
    #[test]
    fn the_t229a_focus_and_lock_keys_are_declared_in_revision_twenty_three() {
        for (key, group, kind, default) in [
            (
                "focus.schedule.enabled",
                KeyGroup::Focus,
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "focus.schedule.start",
                KeyGroup::Focus,
                KeyType::Text,
                KeyDefault::Text("22:00"),
            ),
            (
                "focus.schedule.end",
                KeyGroup::Focus,
                KeyType::Text,
                KeyDefault::Text("07:00"),
            ),
            (
                "focus.schedule.days",
                KeyGroup::Focus,
                KeyType::TextList,
                KeyDefault::List(&["mon", "tue", "wed", "thu", "fri", "sat", "sun"]),
            ),
            (
                "focus.shortcut",
                KeyGroup::Focus,
                KeyType::Text,
                KeyDefault::Text(""),
            ),
            (
                "lock.passwordHint",
                KeyGroup::Lock,
                KeyType::Text,
                KeyDefault::Text(""),
            ),
            (
                "login.windowStyle",
                KeyGroup::Lock,
                KeyType::Text,
                KeyDefault::Text("list"),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, group, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 23, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        assert!(spec("login.windowStyle")
            .unwrap()
            .validate(&Value::Text("name-and-password".into()))
            .is_ok());
        assert!(spec("login.windowStyle")
            .unwrap()
            .validate(&Value::Text("cards".into()))
            .is_err());
        assert!(spec("focus.schedule.start")
            .unwrap()
            .validate(&Value::Text("23:59".into()))
            .is_ok());
    }

    /// The T235 measured-grid keys, additive in revision 25.
    #[test]
    fn the_t235_keys_are_declared_in_revision_twenty_five() {
        for (key, kind, default) in [
            (
                "quickSettings.page2",
                KeyType::TextList,
                KeyDefault::List(&[]),
            ),
            (
                "quickSettings.page3",
                KeyType::TextList,
                KeyDefault::List(&[]),
            ),
            (
                "quickSettings.pageCount",
                KeyType::Integer,
                KeyDefault::Integer(1),
            ),
            (
                "quickSettings.activePage",
                KeyType::Integer,
                KeyDefault::Integer(0),
            ),
            (
                "quickSettings.perDisplay",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "quickSettings.spans",
                KeyType::TextList,
                KeyDefault::List(&[]),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::QuickSettings, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.since, 25, "{key}");
            assert!(spec.validate(&spec.default.to_value()).is_ok(), "{key}");
        }
        assert!(spec("quickSettings.pageCount")
            .unwrap()
            .validate(&Value::Integer(4))
            .is_err());
        assert!(spec("quickSettings.activePage")
            .unwrap()
            .validate(&Value::Integer(-1))
            .is_err());
    }

    /// The T263 System Settings window-state keys, additive in revision 27.
    #[test]
    fn the_t263_window_state_keys_are_declared_in_revision_twenty_seven() {
        for (key, kind, default) in [
            ("settings.lastPane", KeyType::Text, KeyDefault::Text("")),
            (
                "settings.window.width",
                KeyType::Integer,
                KeyDefault::Integer(740),
            ),
            (
                "settings.window.height",
                KeyType::Integer,
                KeyDefault::Integer(820),
            ),
            (
                "settings.window.x",
                KeyType::Integer,
                KeyDefault::Integer(0),
            ),
            (
                "settings.window.y",
                KeyType::Integer,
                KeyDefault::Integer(0),
            ),
            (
                "settings.window.positioned",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::General, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.since, 27, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(spec.validate(&spec.default.to_value()).is_ok(), "{key}");
        }
        assert!(spec("settings.window.width")
            .unwrap()
            .validate(&Value::Integer(719))
            .is_err());
        assert!(spec("settings.window.height")
            .unwrap()
            .validate(&Value::Integer(479))
            .is_err());
        assert!(spec("settings.window.positioned")
            .unwrap()
            .validate(&Value::Bool(true))
            .is_ok());
    }

    /// The T227 Appearance/Quick Settings/Search/Screen Saver/Desktop keys,
    /// additive in revision 21.
    #[test]
    fn the_t227_keys_are_declared_in_revision_twenty_one() {
        for (key, group, kind) in [
            ("appearance.highlight", KeyGroup::Appearance, KeyType::Text),
            (
                "appearance.sidebarIconSize",
                KeyGroup::Appearance,
                KeyType::Text,
            ),
            (
                "appearance.wallpaperTint",
                KeyGroup::Appearance,
                KeyType::Bool,
            ),
            ("appearance.scrollBars", KeyGroup::Appearance, KeyType::Text),
            (
                "quickSettings.modules",
                KeyGroup::QuickSettings,
                KeyType::TextList,
            ),
            (
                "quickSettings.menuBarOnly",
                KeyGroup::QuickSettings,
                KeyType::TextList,
            ),
            ("search.categories", KeyGroup::Search, KeyType::TextList),
            ("search.indexLocations", KeyGroup::Search, KeyType::TextList),
            ("search.shortcut", KeyGroup::Search, KeyType::Text),
            ("search.web", KeyGroup::Search, KeyType::Bool),
            ("menu.control.search", KeyGroup::Menu, KeyType::Bool),
            ("screenSaver.style", KeyGroup::ScreenSaver, KeyType::Text),
            ("screenSaver.clock", KeyGroup::ScreenSaver, KeyType::Bool),
            ("screenSaver.showDate", KeyGroup::ScreenSaver, KeyType::Bool),
            (
                "screenSaver.showSeconds",
                KeyGroup::ScreenSaver,
                KeyType::Bool,
            ),
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

    /// The T274 Appearance completion keys, additive in revision 31.
    #[test]
    fn the_t274_appearance_keys_are_declared_in_revision_thirty_one() {
        for (key, kind) in [
            ("appearance.scrollBarClick", KeyType::Text),
            ("appearance.folderColor", KeyType::Text),
            ("appearance.iconStyle", KeyType::Text),
            ("appearance.iconTheme", KeyType::Text),
            ("appearance.cursorTheme", KeyType::Text),
            ("appearance.cursorSize", KeyType::Integer),
            ("appearance.windowControls", KeyType::Text),
            ("appearance.material", KeyType::Text),
            ("appearance.accentFromWallpaper", KeyType::Bool),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, KeyGroup::Appearance, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.since, 31, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(
                spec.validate(&spec.default.to_value()).is_ok(),
                "{key} default validates"
            );
        }
        assert!(spec("appearance.scrollBarClick")
            .unwrap()
            .validate(&Value::Text("jump-to-spot".into()))
            .is_ok());
        assert!(spec("appearance.scrollBarClick")
            .unwrap()
            .validate(&Value::Text("page".into()))
            .is_err());
        assert!(spec("appearance.cursorSize")
            .unwrap()
            .validate(&Value::Integer(15))
            .is_err());
        assert!(spec("appearance.cursorSize")
            .unwrap()
            .validate(&Value::Integer(64))
            .is_ok());
    }

    /// The T275 accessibility accommodation keys are additive in revision 32.
    /// The three live booleans feed the design-system Theme, the magnifier
    /// pair/zoom/mode feed the compositor, and the recorded chord feeds the
    /// shortcut engine. Delegated rows (screen reader, captions, switch
    /// control, live speech) have no settingsd key on purpose: their owner is
    /// an external service.
    #[test]
    fn the_accessibility_keys_are_declared_in_revision_thirty_two() {
        for (key, kind, default) in [
            (
                "accessibility.textSize",
                KeyType::Text,
                KeyDefault::Text("default"),
            ),
            (
                "accessibility.differentiateWithoutColor",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "accessibility.dimFlashingLights",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "accessibility.magnifier",
                KeyType::Bool,
                KeyDefault::Bool(false),
            ),
            (
                "accessibility.magnifierZoom",
                KeyType::Number,
                KeyDefault::Number(2.0),
            ),
            (
                "accessibility.magnifierMode",
                KeyType::Text,
                KeyDefault::Text("follow-focus"),
            ),
            (
                "accessibility.shortcut",
                KeyType::Text,
                KeyDefault::Text(""),
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is not declared"));
            assert_eq!(spec.group, KeyGroup::Accessibility);
            assert_eq!(spec.kind, kind);
            assert_eq!(spec.default, default);
            assert_eq!(spec.since, 32);
            assert!(
                spec.validate(&default.to_value()).is_ok(),
                "{key} default fails"
            );
        }
    }

    /// T279: the Privacy & Security master gate and the opt-in analytics
    /// preference are additive in revision 36. The master gates the pane's
    /// per-app location writes; analytics is stored-only and defaults off.
    #[test]
    fn the_privacy_keys_are_declared_in_revision_thirty_six() {
        for (key, default) in [
            ("privacy.locationServices", KeyDefault::Bool(true)),
            ("privacy.analytics", KeyDefault::Bool(false)),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is not declared"));
            assert_eq!(spec.group, KeyGroup::Privacy);
            assert_eq!(spec.kind, KeyType::Bool);
            assert_eq!(spec.default, default);
            assert_eq!(spec.since, 36);
            assert!(
                spec.validate(&default.to_value()).is_ok(),
                "{key} default fails"
            );
        }
        assert_eq!(
            spec("privacy.analytics").unwrap().default,
            KeyDefault::Bool(false),
            "analytics must default off"
        );
    }

    /// The T281 Linux-only preference keys, additive in revision 38. Both are
    /// explicit stored-only preferences: the observable behaviour is unchanged
    /// until a rotation timer or clipboard-policy consumer lands.
    #[test]
    fn the_t281_linux_addition_keys_are_declared_in_revision_thirty_eight() {
        for (key, kind, default, group) in [
            (
                "wallpaper.slideshow",
                KeyType::Integer,
                KeyDefault::Integer(0),
                KeyGroup::Wallpaper,
            ),
            (
                "search.clipboardHistory",
                KeyType::Bool,
                KeyDefault::Bool(true),
                KeyGroup::Search,
            ),
        ] {
            let spec = spec(key).unwrap_or_else(|| panic!("{key} is declared"));
            assert_eq!(spec.group, group, "{key}");
            assert_eq!(spec.kind, kind, "{key}");
            assert_eq!(spec.default, default, "{key}");
            assert_eq!(spec.owner, "apps/settings", "{key}");
            assert_eq!(spec.since, 38, "{key}");
            assert!(spec.since <= SCHEMA_VERSION, "{key}");
            assert!(spec.consumer.starts_with("stored-only:"), "{key}");
            assert!(
                spec.validate(&default.to_value()).is_ok(),
                "{key} default fails"
            );
        }
        let slideshow = spec("wallpaper.slideshow").unwrap();
        assert!(slideshow.validate(&Value::Integer(0)).is_ok());
        assert!(slideshow.validate(&Value::Integer(1440)).is_ok());
        assert!(slideshow.validate(&Value::Integer(1441)).is_err());
        assert!(slideshow.validate(&Value::Integer(-1)).is_err());
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

    /// T282: a NaN or infinity must be rejected before a range check. Every
    /// comparison with NaN is false, so the old code let it through and
    /// `serde_json::Number::from_f64(...).expect` later panicked in the daemon.
    #[test]
    fn non_finite_numbers_are_rejected_at_the_key() {
        let size = spec("dock.size").unwrap();
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(
                matches!(
                    size.validate(&Value::Number(number)),
                    Err(SettingsError::NonFinite { .. })
                ),
                "dock.size accepted {number}"
            );
        }
        // Finite values still follow the declared range.
        assert!(size.validate(&Value::Number(0.5)).is_ok());
        assert!(matches!(
            size.validate(&Value::Number(2.0)),
            Err(SettingsError::OutOfRange { .. })
        ));
    }

    /// T282: semantic rules reject a value the D-Bus type alone accepts.
    #[test]
    fn times_colours_paths_lists_and_bounds_are_validated() {
        for key in ["focus.schedule.start", "focus.schedule.end"] {
            let spec = spec(key).unwrap();
            assert!(spec.validate(&Value::Text("00:00".into())).is_ok(), "{key}");
            assert!(spec.validate(&Value::Text("23:59".into())).is_ok(), "{key}");
            for bad in ["24:00", "12:60", "7:00", "0700", "noon", " 07:00"] {
                assert!(
                    matches!(
                        spec.validate(&Value::Text(bad.into())),
                        Err(SettingsError::Invalid { .. })
                    ),
                    "{key} accepted {bad:?}"
                );
            }
        }

        for key in ["appearance.accent", "appearance.highlight", "appearance.folderColor"] {
            let spec = spec(key).unwrap();
            assert!(spec.validate(&Value::Text(String::new())).is_ok(), "{key}");
            assert!(spec.validate(&Value::Text("#4A7DFF".into())).is_ok(), "{key}");
            for bad in ["4a7dff", "#4a7df", "#4a7dfff", "#gggggg", "red"] {
                assert!(
                    matches!(
                        spec.validate(&Value::Text(bad.into())),
                        Err(SettingsError::Invalid { .. })
                    ),
                    "{key} accepted {bad:?}"
                );
            }
        }

        for key in [
            "wallpaper.source",
            "wallpaper.providerSource",
            "wallpaper.builtinDefault",
            "backup.destination",
        ] {
            let spec = spec(key).unwrap();
            assert!(spec.validate(&Value::Text(String::new())).is_ok(), "{key}");
            assert!(spec.validate(&Value::Text("/home/a/b.png".into())).is_ok(), "{key}");
            for bad in ["relative/path.png", "./x", "~/x"] {
                assert!(
                    matches!(
                        spec.validate(&Value::Text(bad.into())),
                        Err(SettingsError::Invalid { .. })
                    ),
                    "{key} accepted {bad:?}"
                );
            }
        }

        let days = spec("focus.schedule.days").unwrap();
        assert!(days
            .validate(&Value::TextList(vec!["mon".into(), "sun".into()]))
            .is_ok());
        assert!(matches!(
            days.validate(&Value::TextList(vec!["monday".into()])),
            Err(SettingsError::Invalid { .. })
        ));

        let modules = spec("quickSettings.modules").unwrap();
        assert!(modules
            .validate(&Value::TextList(vec!["wifi".into(), "volume".into()]))
            .is_ok());
        assert!(matches!(
            modules.validate(&Value::TextList(vec!["made-up".into()])),
            Err(SettingsError::Invalid { .. })
        ));

        let layouts = spec("input.sources").unwrap();
        assert!(layouts
            .validate(&Value::TextList(vec!["us".into(), "de(nodeadkeys)".into()]))
            .is_ok());
        for bad in ["", "us intl", "a/b"] {
            assert!(
                matches!(
                    layouts.validate(&Value::TextList(vec![bad.into()])),
                    Err(SettingsError::Invalid { .. })
                ),
                "input.sources accepted {bad:?}"
            );
        }

        let replacements = spec("input.replacements").unwrap();
        assert!(replacements
            .validate(&Value::TextList(vec!["teh\tthe".into()]))
            .is_ok());
        for bad in ["teh the", "\tvalue", "from\t"] {
            assert!(
                matches!(
                    replacements.validate(&Value::TextList(vec![bad.into()])),
                    Err(SettingsError::Invalid { .. })
                ),
                "input.replacements accepted {bad:?}"
            );
        }

        let message = spec("lock.message").unwrap();
        assert!(message.validate(&Value::Text("Back at 5".into())).is_ok());
        assert!(matches!(
            message.validate(&Value::Text("x".repeat(121))),
            Err(SettingsError::Invalid { .. })
        ));
        assert!(matches!(
            message.validate(&Value::Text("two\nlines".into())),
            Err(SettingsError::Invalid { .. })
        ));

        let shortcut = spec("accessibility.shortcut").unwrap();
        assert!(shortcut.validate(&Value::Text(String::new())).is_ok());
        assert!(shortcut
            .validate(&Value::Text("Ctrl+Shift+A".into()))
            .is_ok());
        assert!(matches!(
            shortcut.validate(&Value::Text("x".repeat(65))),
            Err(SettingsError::Invalid { .. })
        ));
    }

    /// T282: the stored-only mark is the honest half of every key. A key with
    /// a shipped consumer names one of the process/module roots the tree
    /// actually builds; everything else says `stored_only` and its consumer
    /// text starts with `stored-only:` unless the row is a Settings-owned
    /// preference with no runtime reader.
    #[test]
    fn every_non_stored_only_key_names_a_shipped_consumer_module() {
        const ROOTS: &[&str] = &[
            "shell/",
            "apps/settings",
            "apps/files",
            "compositor/",
            "session/",
            "services/wallpaperd",
            "dragonfruit-system-status",
            "settingsd",
        ];
        let mut stored = 0;
        for spec in KEYS {
            if spec.stored_only {
                stored += 1;
                assert!(!spec.consumer.is_empty(), "{} has no consumer", spec.key);
                continue;
            }
            assert!(
                ROOTS.iter().any(|root| spec.consumer.starts_with(root)),
                "{} is not stored-only but its consumer {:?} is not a shipped module",
                spec.key,
                spec.consumer
            );
        }
        assert!(stored > 0, "the schema must label at least one stored-only key");
        // The two T281 Linux additions and the analytics preference stay
        // explicitly stored-only until their consumers land.
        for key in [
            "wallpaper.slideshow",
            "search.clipboardHistory",
            "privacy.analytics",
        ] {
            assert!(spec(key).unwrap().stored_only, "{key} must be stored-only");
        }
    }
}
