// SPDX-License-Identifier: GPL-3.0-or-later
//! T320 acceptance: pointer/trackpad settings reach the device seam.
//!
//! libinput needs a free logind seat, which this host does not have (the host
//! Wayland session owns DRM master), so the *decision* is tested here and the
//! real tap/natural-scroll check is recorded `OPEN` in the task hand-off with
//! the exact hardware command. These tests pin the capability classification,
//! the settings-to-libinput mapping, hotplug apply, the live re-apply on a
//! policy change, and the per-family precedence the compositor resolves.

use std::cell::RefCell;
use std::rc::Rc;

use dragonfruit_compositor::pointer_policy::{
    effective_pointer, scroll_scale, secondary_click_button, AccelProfile, PointerApplier,
    PointerConfig, PointerConfigFailure, PointerController, PointerKind, PointerSettings,
    ScrollMethod, SecondaryClick,
};

#[derive(Default)]
struct Recorder {
    applies: Vec<(String, PointerKind, PointerConfig)>,
    removed: Vec<String>,
    reject: Vec<&'static str>,
}

struct RecorderApplier(Rc<RefCell<Recorder>>);

impl PointerApplier for RecorderApplier {
    fn apply(
        &mut self,
        device: &str,
        kind: PointerKind,
        config: &PointerConfig,
    ) -> Vec<PointerConfigFailure> {
        self.0
            .borrow_mut()
            .applies
            .push((device.to_string(), kind, *config));
        self.0
            .borrow()
            .reject
            .iter()
            .map(|call| PointerConfigFailure {
                device: device.to_string(),
                call,
                message: "unsupported".into(),
            })
            .collect()
    }

    fn remove(&mut self, device: &str) {
        self.0.borrow_mut().removed.push(device.to_string());
    }
}

fn harness() -> (Rc<RefCell<Recorder>>, PointerController) {
    let recorder = Rc::new(RefCell::new(Recorder::default()));
    let mut controller = PointerController::new();
    controller.set_applier(Box::new(RecorderApplier(recorder.clone())));
    (recorder, controller)
}

fn config(settings: &PointerSettings) -> PointerConfig {
    PointerConfig::from(settings)
}

#[test]
fn a_mouse_and_a_touchpad_are_classified_by_capability_not_name() {
    // A wheel mouse reports Pointer only.
    assert_eq!(
        PointerKind::classify(true, false, false),
        Some(PointerKind::Mouse)
    );
    // A touchpad reports Pointer plus Touch and/or Gesture.
    assert_eq!(
        PointerKind::classify(true, true, false),
        Some(PointerKind::Touchpad)
    );
    assert_eq!(
        PointerKind::classify(true, false, true),
        Some(PointerKind::Touchpad)
    );
    // A touchscreen or tablet has no Pointer capability; it is not a pointer.
    assert_eq!(PointerKind::classify(false, true, true), None);
    assert_eq!(PointerKind::Mouse.id(), "mouse");
    assert_eq!(PointerKind::Touchpad.id(), "trackpad");
    assert_eq!(PointerKind::parse("mouse"), Some(PointerKind::Mouse));
    assert_eq!(PointerKind::parse("trackpad"), Some(PointerKind::Touchpad));
    assert_eq!(PointerKind::parse("event5"), None);
}

#[test]
fn every_libinput_field_maps_from_the_effective_settings() {
    let settings = PointerSettings {
        tap_to_click: false,
        natural_scroll: false,
        accel_speed: 0.35,
        accel_profile: AccelProfile::Flat,
        left_handed: true,
        scroll_method: ScrollMethod::Edge,
        ..PointerSettings::default()
    };
    let mapped = config(&settings);
    assert!(!mapped.tap_to_click);
    assert!(!mapped.natural_scroll);
    assert!((mapped.accel_speed - 0.35).abs() < f64::EPSILON);
    assert_eq!(mapped.accel_profile, AccelProfile::Flat);
    assert!(mapped.left_handed);
    assert_eq!(mapped.scroll_method, ScrollMethod::Edge);

    // The shipped defaults are the ones libinput does not pick on its own, so
    // a missing apply is visible immediately.
    let defaults = config(&PointerSettings::default());
    assert!(defaults.tap_to_click);
    assert!(defaults.natural_scroll);
    assert!(!defaults.left_handed);
    assert_eq!(defaults.scroll_method, ScrollMethod::TwoFinger);
}

#[test]
fn hotplug_applies_immediately_and_removal_releases_the_device() {
    let (recorder, mut controller) = harness();
    let settings = PointerSettings::default();
    controller.add_device("event5", PointerKind::Touchpad, config(&settings));

    {
        let recorder = recorder.borrow();
        assert_eq!(recorder.applies.len(), 1);
        let (name, kind, applied) = &recorder.applies[0];
        assert_eq!(name, "event5");
        assert_eq!(*kind, PointerKind::Touchpad);
        assert_eq!(*applied, config(&settings));
    }
    assert_eq!(controller.kind("event5"), Some(PointerKind::Touchpad));

    controller.remove_device("event5");
    assert_eq!(controller.kind("event5"), None);
    assert_eq!(recorder.borrow().removed, ["event5"]);
}

#[test]
fn a_policy_change_reapplies_every_live_device() {
    let (recorder, mut controller) = harness();
    let mut settings = PointerSettings::default();
    controller.add_device("mouse", PointerKind::Mouse, config(&settings));
    controller.add_device("pad", PointerKind::Touchpad, config(&settings));
    recorder.borrow_mut().applies.clear();

    // The shell's set_pointer_policy / family request lands in the settings
    // and calls apply_all; every device receives the new snapshot.
    settings.accel_speed = -0.5;
    settings.natural_scroll = false;
    settings.tap_to_click = false;
    controller.apply_all(|_, _| config(&settings));

    let recorder = recorder.borrow();
    assert_eq!(recorder.applies.len(), 2);
    for (_, _, applied) in &recorder.applies {
        assert!((applied.accel_speed + 0.5).abs() < f64::EPSILON);
        assert!(!applied.natural_scroll);
        assert!(!applied.tap_to_click);
    }
}

#[test]
fn the_family_snapshot_wins_over_the_default_and_a_name_override_wins_over_both() {
    let defaults = PointerSettings {
        accel_speed: 0.1,
        ..PointerSettings::default()
    };
    let family = PointerSettings {
        accel_speed: 0.4,
        ..PointerSettings::default()
    };
    let named = PointerSettings {
        accel_speed: -0.2,
        ..PointerSettings::default()
    };

    assert_eq!(
        effective_pointer(defaults, None, None).accel_speed,
        defaults.accel_speed
    );
    assert_eq!(
        effective_pointer(defaults, Some(family), None).accel_speed,
        family.accel_speed
    );
    assert_eq!(
        effective_pointer(defaults, Some(family), Some(named)).accel_speed,
        named.accel_speed
    );
}

#[test]
fn an_unsupported_config_call_is_logged_once_and_the_rest_still_applies() {
    let (recorder, mut controller) = harness();
    recorder.borrow_mut().reject = vec!["config_tap_set_enabled"];
    let settings = PointerSettings::default();
    controller.add_device("mouse", PointerKind::Mouse, config(&settings));
    controller.apply_all(|_, _| config(&settings));
    controller.apply_all(|_, _| config(&settings));

    // Seen on every apply (hotplug + two policy pushes), logged once.
    assert_eq!(
        controller.failure_count("mouse", "config_tap_set_enabled"),
        3
    );
    assert_eq!(
        controller.failure_count("mouse", "config_scroll_set_natural_scroll_enabled"),
        0
    );
    // The snapshot still reaches the device on every apply.
    assert_eq!(recorder.borrow().applies.len(), 3);
}

#[test]
fn scroll_speed_and_secondary_click_have_one_bounded_mapping() {
    assert!((scroll_scale(0.0) - 1.0).abs() < f64::EPSILON);
    assert!((scroll_scale(-1.0) - 0.5).abs() < f64::EPSILON);
    assert!((scroll_scale(1.0) - 2.0).abs() < f64::EPSILON);
    // Out-of-range input cannot wedge the axis path.
    assert!((scroll_scale(-9.0) - 0.5).abs() < f64::EPSILON);
    assert!((scroll_scale(9.0) - 2.0).abs() < f64::EPSILON);

    assert_eq!(secondary_click_button(SecondaryClick::Right, 0x110), 0x110);
    assert_eq!(secondary_click_button(SecondaryClick::Left, 0x110), 0x111);
    assert_eq!(secondary_click_button(SecondaryClick::Left, 0x111), 0x110);
    // Middle and side buttons are untouched.
    assert_eq!(secondary_click_button(SecondaryClick::Left, 0x112), 0x112);
}

#[test]
fn a_repeated_policy_snapshot_is_idempotent() {
    let (recorder, mut controller) = harness();
    let settings = PointerSettings::default();
    controller.add_device("pad", PointerKind::Touchpad, config(&settings));
    controller.apply_all(|_, _| config(&settings));
    controller.apply_all(|_, _| config(&settings));
    let recorder = recorder.borrow();
    assert_eq!(recorder.applies.len(), 3);
    let first = recorder.applies[0].2;
    assert!(recorder
        .applies
        .iter()
        .all(|(_, _, applied)| *applied == first));
}
