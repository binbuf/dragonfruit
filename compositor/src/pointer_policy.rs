// SPDX-License-Identifier: GPL-3.0-or-later
#![allow(dead_code)] // The binary and the test library use different halves of this surface.

//! The pointer/trackpad settings model and its libinput application (T320).
//!
//! This module is pure so `compositor/tests/input_settings.rs` can pin the
//! mapping and the live re-apply without a libinput seat. The DRM backend
//! supplies the hardware [`PointerApplier`]; the compositor's input router
//! supplies the device add/remove events.
//!
//! Mouse vs touchpad is resolved from libinput capabilities (a pointer with
//! touch or gesture support is a touchpad), never from a guessed device name.

use std::collections::{BTreeMap, BTreeSet};

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

impl ScrollMethod {
    /// The stable id the settings schema and the protocol use.
    pub const fn id(self) -> &'static str {
        match self {
            ScrollMethod::TwoFinger => "two-finger",
            ScrollMethod::Edge => "edge",
            ScrollMethod::Button => "button",
        }
    }

    /// Parse the schema's stable id; an unknown spelling is `None`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "two-finger" => Some(ScrollMethod::TwoFinger),
            "edge" => Some(ScrollMethod::Edge),
            "button" => Some(ScrollMethod::Button),
            _ => None,
        }
    }
}

/// Which button a secondary click uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SecondaryClick {
    #[default]
    Right,
    Left,
}

impl SecondaryClick {
    /// The stable id the settings schema and the protocol use.
    pub const fn id(self) -> &'static str {
        match self {
            SecondaryClick::Right => "right",
            SecondaryClick::Left => "left",
        }
    }

    /// Parse the schema's stable id; an unknown spelling is `None`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "right" => Some(SecondaryClick::Right),
            "left" => Some(SecondaryClick::Left),
            _ => None,
        }
    }
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
    /// The secondary-click button (T229).
    pub secondary_click: SecondaryClick,
    /// Scroll speed, `-1.0..=1.0` (0 neutral) (T229).
    pub scroll_speed: f64,
    /// Double-click interval in milliseconds (T229).
    pub double_click_ms: i32,
    /// Trackpad haptic feedback, where the hardware supports it (T229).
    pub haptics: bool,
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
            secondary_click: SecondaryClick::Right,
            scroll_speed: 0.0,
            double_click_ms: 400,
            haptics: true,
        }
    }
}

/// The two pointer families the Settings pane exposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PointerKind {
    Mouse,
    Touchpad,
}

impl PointerKind {
    /// The stable id the shell protocol and the settings schema use.
    pub const fn id(self) -> &'static str {
        match self {
            PointerKind::Mouse => "mouse",
            PointerKind::Touchpad => "trackpad",
        }
    }

    /// Parse the shell protocol's family id; an unknown spelling is `None`.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "mouse" => Some(PointerKind::Mouse),
            "trackpad" => Some(PointerKind::Touchpad),
            _ => None,
        }
    }

    /// Classify a device from its libinput capabilities.
    ///
    /// A pointer that also reports touch or gesture support is a touchpad;
    /// every other pointer (a wheel mouse, a trackball, a tablet puck) is a
    /// mouse. A device with no pointer capability is not a pointer at all.
    pub fn classify(has_pointer: bool, has_touch: bool, has_gesture: bool) -> Option<Self> {
        if !has_pointer {
            return None;
        }
        if has_touch || has_gesture {
            Some(PointerKind::Touchpad)
        } else {
            Some(PointerKind::Mouse)
        }
    }
}

/// The libinput-appliable half of a [`PointerSettings`] snapshot.
///
/// Scroll speed, secondary click, the double-click interval, and haptics have
/// no libinput setter; they are applied by the compositor's input path (or
/// shipped stored-only) and are deliberately absent here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PointerConfig {
    pub tap_to_click: bool,
    pub natural_scroll: bool,
    pub accel_speed: f64,
    pub accel_profile: AccelProfile,
    pub left_handed: bool,
    pub scroll_method: ScrollMethod,
}

impl From<&PointerSettings> for PointerConfig {
    fn from(settings: &PointerSettings) -> Self {
        PointerConfig {
            tap_to_click: settings.tap_to_click,
            natural_scroll: settings.natural_scroll,
            accel_speed: settings.accel_speed.clamp(-1.0, 1.0),
            accel_profile: settings.accel_profile,
            left_handed: settings.left_handed,
            scroll_method: settings.scroll_method,
        }
    }
}

/// One libinput config call that the device rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PointerConfigFailure {
    pub device: String,
    /// The libinput config call, e.g. `config_tap_set_enabled`.
    pub call: &'static str,
    pub message: String,
}

/// The hardware seam: apply one snapshot to one live device.
pub trait PointerApplier {
    /// Apply `config` to `device`. Unsupported calls are returned, not fatal;
    /// the remaining calls in the snapshot still run.
    fn apply(
        &mut self,
        device: &str,
        kind: PointerKind,
        config: &PointerConfig,
    ) -> Vec<PointerConfigFailure>;

    /// Forget a device that libinput removed.
    fn remove(&mut self, device: &str);
}

/// Resolve the effective pointer settings: an explicit per-device override
/// wins, then the device family's snapshot, then the shared default.
pub fn effective_pointer(
    default: PointerSettings,
    family: Option<PointerSettings>,
    device: Option<PointerSettings>,
) -> PointerSettings {
    device.or(family).unwrap_or(default)
}

/// The compositor's pointer-device registry and applier driver.
#[derive(Default)]
pub struct PointerController {
    devices: BTreeMap<String, PointerKind>,
    applied: BTreeMap<String, PointerConfig>,
    applier: Option<Box<dyn PointerApplier>>,
    /// Failures already written to the log, so a hotplug storm or a policy
    /// re-apply cannot spam the session log.
    logged: BTreeSet<(String, &'static str)>,
    /// Every failure seen, including repeats (tests and diagnostics).
    failures: BTreeMap<(String, &'static str), u32>,
}

impl PointerController {
    pub fn new() -> Self {
        PointerController::default()
    }

    /// Install the hardware applier. The DRM backend calls this once; tests
    /// install a recorder.
    pub fn set_applier(&mut self, applier: Box<dyn PointerApplier>) {
        self.applier = Some(applier);
    }

    /// The pointer devices the compositor currently knows about.
    pub fn devices(&self) -> impl Iterator<Item = (&str, PointerKind)> {
        self.devices
            .iter()
            .map(|(name, kind)| (name.as_str(), *kind))
    }

    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    pub fn kind(&self, device: &str) -> Option<PointerKind> {
        self.devices.get(device).copied()
    }

    /// The snapshot last handed to the applier for `device`.
    pub fn applied(&self, device: &str) -> Option<&PointerConfig> {
        self.applied.get(device)
    }

    /// How many times a rejected config call has been seen for a device.
    pub fn failure_count(&self, device: &str, call: &str) -> u32 {
        self.failures
            .iter()
            .find(|((name, seen), _)| name == device && *seen == call)
            .map(|(_, count)| *count)
            .unwrap_or(0)
    }

    /// Record a pointer device and apply the effective settings immediately
    /// (hotplug is not a reason to wait for the next policy change).
    pub fn add_device(
        &mut self,
        device: &str,
        kind: PointerKind,
        config: PointerConfig,
    ) -> PointerConfig {
        self.devices.insert(device.to_string(), kind);
        self.push(device, kind, config);
        config
    }

    /// Forget a device libinput removed.
    pub fn remove_device(&mut self, device: &str) {
        self.devices.remove(device);
        self.applied.remove(device);
        if let Some(applier) = self.applier.as_mut() {
            applier.remove(device);
        }
    }

    /// Re-apply a resolved snapshot to every known device. `resolve` receives
    /// the device name and kind so the caller can apply the override/family/
    /// default precedence.
    pub fn apply_all(&mut self, mut resolve: impl FnMut(&str, PointerKind) -> PointerConfig) {
        let devices: Vec<(String, PointerKind)> = self
            .devices
            .iter()
            .map(|(name, kind)| (name.clone(), *kind))
            .collect();
        for (name, kind) in devices {
            let config = resolve(&name, kind);
            self.push(&name, kind, config);
        }
    }

    fn push(&mut self, device: &str, kind: PointerKind, config: PointerConfig) {
        let Some(applier) = self.applier.as_mut() else {
            // No hardware applier (headless/nested): keep the registry only;
            // do not claim the snapshot reached a device.
            return;
        };
        let failures = applier.apply(device, kind, &config);
        for failure in failures {
            let key = (failure.device.clone(), failure.call);
            *self.failures.entry(key.clone()).or_insert(0) += 1;
            if self.logged.insert(key) {
                eprintln!(
                    "dragonfruit-compositor: pointer setting {} on {}: {}",
                    failure.call, failure.device, failure.message
                );
            }
        }
        self.applied.insert(device.to_string(), config);
    }
}

/// The multiplier the input router applies to pointer-axis amounts.
///
/// libinput exposes no scroll-speed setter, so the shared `input.scrollSpeed`
/// preference is applied at the compositor's one axis path instead. `-1.0`
/// halves the delta, `0` leaves it alone, and `1.0` doubles it.
pub fn scroll_scale(scroll_speed: f64) -> f64 {
    2f64.powf(scroll_speed.clamp(-1.0, 1.0))
}

/// Apply the shared secondary-click preference to a raw pointer button code.
///
/// libinput has no right/left secondary-button setter (its click method picks
/// button areas vs clickfinger, not the secondary side), so the compositor
/// swaps the primary and secondary button codes at its one button path.
pub fn secondary_click_button(secondary: SecondaryClick, button: u32) -> u32 {
    if secondary == SecondaryClick::Left {
        match button {
            0x110 => 0x111, // BTN_LEFT -> BTN_RIGHT
            0x111 => 0x110, // BTN_RIGHT -> BTN_LEFT
            other => other,
        }
    } else {
        button
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct Recorder {
        applies: Vec<(String, PointerKind, PointerConfig)>,
        removed: Vec<String>,
        fail: Vec<&'static str>,
    }

    impl PointerApplier for Rc<RefCell<Recorder>> {
        fn apply(
            &mut self,
            device: &str,
            kind: PointerKind,
            config: &PointerConfig,
        ) -> Vec<PointerConfigFailure> {
            self.borrow_mut()
                .applies
                .push((device.to_string(), kind, *config));
            self.borrow()
                .fail
                .iter()
                .map(|call| PointerConfigFailure {
                    device: device.to_string(),
                    call,
                    message: "unsupported".into(),
                })
                .collect()
        }

        fn remove(&mut self, device: &str) {
            self.borrow_mut().removed.push(device.to_string());
        }
    }

    fn recorder() -> (Rc<RefCell<Recorder>>, PointerController) {
        let recorder = Rc::new(RefCell::new(Recorder::default()));
        let mut controller = PointerController::new();
        controller.set_applier(Box::new(recorder.clone()));
        (recorder, controller)
    }

    #[test]
    fn capability_not_name_decides_the_family() {
        assert_eq!(
            PointerKind::classify(true, false, false),
            Some(PointerKind::Mouse)
        );
        assert_eq!(
            PointerKind::classify(true, true, false),
            Some(PointerKind::Touchpad)
        );
        assert_eq!(
            PointerKind::classify(true, false, true),
            Some(PointerKind::Touchpad)
        );
        assert_eq!(PointerKind::classify(false, true, true), None);
        assert_eq!(PointerKind::Mouse.id(), "mouse");
        assert_eq!(PointerKind::Touchpad.id(), "trackpad");
        assert_eq!(PointerKind::parse("trackpad"), Some(PointerKind::Touchpad));
        assert_eq!(PointerKind::parse("pointer"), None);
    }

    #[test]
    fn hotplug_applies_and_removal_forgets() {
        let (recorder, mut controller) = recorder();
        let config = PointerConfig::from(&PointerSettings::default());
        controller.add_device("event5", PointerKind::Touchpad, config);
        assert_eq!(config.tap_to_click, true);
        assert_eq!(config.natural_scroll, true);
        assert_eq!(controller.kind("event5"), Some(PointerKind::Touchpad));
        assert_eq!(controller.applied("event5"), Some(&config));

        controller.remove_device("event5");
        assert_eq!(controller.kind("event5"), None);
        assert_eq!(controller.applied("event5"), None);
        assert_eq!(recorder.borrow().removed, ["event5"]);
    }

    #[test]
    fn policy_change_reapplies_every_device() {
        let (recorder, mut controller) = recorder();
        let mut settings = PointerSettings::default();
        controller.add_device("mouse", PointerKind::Mouse, PointerConfig::from(&settings));
        controller.add_device("pad", PointerKind::Touchpad, PointerConfig::from(&settings));
        recorder.borrow_mut().applies.clear();

        settings.tap_to_click = false;
        settings.accel_speed = 0.5;
        controller.apply_all(|_, _| PointerConfig::from(&settings));

        let applies = &recorder.borrow().applies;
        assert_eq!(applies.len(), 2);
        assert!(applies.iter().all(|(_, _, config)| {
            !config.tap_to_click && (config.accel_speed - 0.5).abs() < f64::EPSILON
        }));
    }

    #[test]
    fn a_rejected_config_call_is_logged_once_and_does_not_stop_the_snapshot() {
        let (recorder, mut controller) = recorder();
        recorder.borrow_mut().fail = vec!["config_tap_set_enabled"];
        let settings = PointerSettings::default();
        controller.add_device("mouse", PointerKind::Mouse, PointerConfig::from(&settings));
        controller.apply_all(|_, _| PointerConfig::from(&settings));
        controller.apply_all(|_, _| PointerConfig::from(&settings));

        assert_eq!(
            controller.failure_count("mouse", "config_tap_set_enabled"),
            3
        );
        assert_eq!(
            controller.failure_count("mouse", "config_natural_scroll_set_enabled"),
            0
        );
        // The snapshot still reaches the device on every apply.
        assert_eq!(recorder.borrow().applies.len(), 3);
    }

    #[test]
    fn a_controller_without_a_backend_still_tracks_devices() {
        let mut controller = PointerController::new();
        controller.add_device(
            "event9",
            PointerKind::Mouse,
            PointerConfig::from(&PointerSettings::default()),
        );
        assert_eq!(controller.device_count(), 1);
        // No applier: the resolved snapshot is not claimed as applied.
        assert_eq!(controller.applied("event9"), None);
    }

    #[test]
    fn scroll_scale_and_secondary_click_are_bounded_and_exact() {
        assert!((scroll_scale(0.0) - 1.0).abs() < f64::EPSILON);
        assert!((scroll_scale(-1.0) - 0.5).abs() < f64::EPSILON);
        assert!((scroll_scale(1.0) - 2.0).abs() < f64::EPSILON);
        assert!((scroll_scale(4.0) - 2.0).abs() < f64::EPSILON);
        assert_eq!(secondary_click_button(SecondaryClick::Right, 0x110), 0x110);
        assert_eq!(secondary_click_button(SecondaryClick::Left, 0x110), 0x111);
        assert_eq!(secondary_click_button(SecondaryClick::Left, 0x111), 0x110);
        assert_eq!(secondary_click_button(SecondaryClick::Left, 0x112), 0x112);
    }
}
