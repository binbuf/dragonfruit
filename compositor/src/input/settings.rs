// SPDX-License-Identifier: MIT
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

/// Pointer acceleration profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccelProfile {
    /// Device-dependent adaptive curve (libinput default).
    #[default]
    Adaptive,
    /// No acceleration; 1:1 movement.
    Flat,
}

/// Scroll method for pointer-like devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollMethod {
    #[default]
    TwoFinger,
    Edge,
    Button,
}

/// Per-device pointer/trackpad settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerSettings {
    /// Acceleration speed, `-1.0..=1.0` (0 = neutral).
    pub accel_speed: f64,
    pub accel_profile: AccelProfile,
    /// macOS-style natural (inverted) scrolling.
    pub natural_scroll: bool,
    pub scroll_method: ScrollMethod,
    pub tap_to_click: bool,
    pub left_handed: bool,
}

impl Default for PointerSettings {
    fn default() -> Self {
        PointerSettings {
            accel_speed: 0.0,
            accel_profile: AccelProfile::Adaptive,
            natural_scroll: true,
            scroll_method: ScrollMethod::TwoFinger,
            tap_to_click: true,
            left_handed: false,
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
    fn role_helpers_match_the_canonical_mapping() {
        assert_eq!(InputSettings::command_role().xkb_mod_name(), "Mod4");
        assert_eq!(InputSettings::option_role().xkb_mod_name(), "Mod1");
    }
}
