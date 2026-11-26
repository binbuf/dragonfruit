// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(dead_code)] // Forward-looking input API consumed by T-05/T-07/T-11/T-16/T-22/T-27.

//! The compositor's input settings model (T-03, FR-7).
//!
//! Keyboard repeat, per-device pointer acceleration, and scroll
//! configuration are read/write here and applied live to the seat and the
//! gesture/hot-corner detectors. The Settings app (T-16) and `settingsd`
//! (T-15) reach this surface; persistence is theirs, application is the
//! compositor's.

use std::collections::BTreeMap;

use super::gestures::{GestureConfig, ProgressConfig};
use super::hot_corners::HotCornerConfig;
use super::keymap::ModifierRole;
use super::shortcuts::Shortcut;
use crate::pointer_policy::PointerKind;

pub use crate::pointer_policy::{AccelProfile, PointerSettings, ScrollMethod, SecondaryClick};

/// Keyboard repeat settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyboardSettings {
    /// Delay before repeat starts, in milliseconds.
    pub repeat_delay_ms: i32,
    /// Repeat rate in keys per second (0 disables repeat).
    pub repeat_rate_hz: i32,
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        KeyboardSettings {
            repeat_delay_ms: 200,
            repeat_rate_hz: 25,
        }
    }
}

/// The complete input settings surface.
#[derive(Debug, Clone, PartialEq)]
pub struct InputSettings {
    pub keyboard: KeyboardSettings,
    pub default_pointer: PointerSettings,
    /// Per-device overrides keyed by libinput device name.
    pub devices: BTreeMap<String, PointerSettings>,
    /// The per-family snapshots the shell forwards from `input.mouse.*` and
    /// `input.trackpad.*` (T320). `None` keeps the shared `default_pointer`
    /// for that family, which is what an older shell that never sends the
    /// family request gets.
    pub mouse_pointer: Option<PointerSettings>,
    pub trackpad_pointer: Option<PointerSettings>,
    pub gestures: GestureConfig,
    pub progress: ProgressConfig,
    pub hot_corners: HotCornerConfig,
    /// The live system binding table (rebindable through this surface).
    pub system_bindings: Vec<Shortcut>,
}

impl Default for InputSettings {
    fn default() -> Self {
        InputSettings {
            keyboard: KeyboardSettings::default(),
            default_pointer: PointerSettings::default(),
            devices: BTreeMap::new(),
            mouse_pointer: None,
            trackpad_pointer: None,
            gestures: GestureConfig::default(),
            progress: ProgressConfig::default(),
            hot_corners: HotCornerConfig::default(),
            system_bindings: super::shortcuts::default_system_bindings(),
        }
    }
}

impl InputSettings {
    /// The effective pointer settings for a device (override or default).
    pub fn pointer(&self, device: &str) -> PointerSettings {
        self.devices
            .get(device)
            .copied()
            .unwrap_or(self.default_pointer)
    }

    /// The effective pointer settings for a device, resolving the precedence
    /// the compositor applies: an explicit per-device override wins, then the
    /// device family's snapshot, then the shared default.
    pub fn effective_pointer(&self, device: &str, kind: PointerKind) -> PointerSettings {
        crate::pointer_policy::effective_pointer(
            self.default_pointer,
            self.family(kind),
            self.devices.get(device).copied(),
        )
    }

    /// The family snapshot, if the shell has forwarded one.
    pub fn family(&self, kind: PointerKind) -> Option<PointerSettings> {
        match kind {
            PointerKind::Mouse => self.mouse_pointer,
            PointerKind::Touchpad => self.trackpad_pointer,
        }
    }

    /// Replace a family snapshot (the shell's `input.mouse.*` /
    /// `input.trackpad.*` request).
    pub fn set_family(&mut self, kind: PointerKind, settings: PointerSettings) {
        match kind {
            PointerKind::Mouse => self.mouse_pointer = Some(settings),
            PointerKind::Touchpad => self.trackpad_pointer = Some(settings),
        }
    }

    /// Drop a family snapshot so the family follows the shared default again.
    pub fn clear_family(&mut self, kind: PointerKind) {
        match kind {
            PointerKind::Mouse => self.mouse_pointer = None,
            PointerKind::Touchpad => self.trackpad_pointer = None,
        }
    }

    /// Set a per-device override.
    pub fn set_pointer(&mut self, device: impl Into<String>, settings: PointerSettings) {
        self.devices.insert(device.into(), settings);
    }

    /// The role mapping is not configurable per-app: it is the fixed
    /// contract in `df-ipc`. Exposed here so callers do not hard-code it.
    pub const fn command_role() -> ModifierRole {
        ModifierRole::Command
    }

    pub const fn option_role() -> ModifierRole {
        ModifierRole::Option
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_device_overrides_fall_back_to_default() {
        let mut settings = InputSettings::default();
        assert_eq!(settings.pointer("trackpad-0"), settings.default_pointer);
        settings.set_pointer(
            "trackpad-0",
            PointerSettings {
                natural_scroll: false,
                ..Default::default()
            },
        );
        assert!(!settings.pointer("trackpad-0").natural_scroll);
        assert!(settings.pointer("other").natural_scroll);
    }

    #[test]
    fn scroll_method_and_secondary_click_parse_their_schema_ids() {
        assert_eq!(
            ScrollMethod::parse("two-finger"),
            Some(ScrollMethod::TwoFinger)
        );
        assert_eq!(ScrollMethod::parse("edge"), Some(ScrollMethod::Edge));
        assert_eq!(ScrollMethod::parse("button"), Some(ScrollMethod::Button));
        assert_eq!(ScrollMethod::parse("sideways"), None);
        assert_eq!(SecondaryClick::parse("right"), Some(SecondaryClick::Right));
        assert_eq!(SecondaryClick::parse("left"), Some(SecondaryClick::Left));
        assert_eq!(SecondaryClick::parse("middle"), None);
        assert_eq!(ScrollMethod::Edge.id(), "edge");
        assert_eq!(SecondaryClick::Left.id(), "left");
    }

    #[test]
    fn role_helpers_match_the_canonical_mapping() {
        assert_eq!(InputSettings::command_role().xkb_mod_name(), "Mod4");
        assert_eq!(InputSettings::option_role().xkb_mod_name(), "Mod1");
    }
}
