// SPDX-License-Identifier: MIT
//! T-03.3 acceptance: the hardware input validation *decision* is explicit and
//! testable without hardware.
//!
//! The compositor can only drive real libinput devices when it owns a logind
//! seat, and this host has no free seat (the host Wayland session owns DRM
//! master — see `docs/SLICING-REVIEW.md`), so the unit is marked **open, not
//! skipped**. These tests pin the classification the matrix shares with
//! `scripts/input-validation.sh` and prove the one hardware-free input class
//! the tree can fully exercise: a non-US xkb layout.

use dragonfruit_compositor::input_validation::{
    layout_keysym_for_evdev, Coverage, GapReason, InputClass, InputValidation, SeatInventory,
};

const KEY_Y: u32 = 21;
const KEY_Z: u32 = 44;

#[test]
fn every_required_class_has_a_row_and_a_stable_id() {
    let validation = InputValidation::classify(false, &SeatInventory::default());
    assert_eq!(validation.rows.len(), InputClass::ALL.len());
    let ids: Vec<&str> = validation.rows.iter().map(|row| row.class.id()).collect();
    assert_eq!(
        ids,
        [
            "mouse",
            "keyboard",
            "touchpad-gestures",
            "hot-corners",
            "tablet-pen",
            "non-us-layout"
        ]
    );
    for class in InputClass::ALL {
        assert_eq!(validation.row(class).class, class);
    }
}

#[test]
fn a_full_seat_validates_every_class_on_device() {
    let validation = InputValidation::classify(true, &SeatInventory::full());
    assert!(!validation.is_open(), "no gaps on a full seat");
    assert_eq!(validation.marker(), "Input validation: READY (6/6 classes)");
    for row in &validation.rows {
        assert_eq!(row.coverage, Coverage::Device, "{}", row.class.id());
    }
}

#[test]
fn without_a_seat_the_router_covered_classes_are_headless() {
    let validation = InputValidation::classify(true, &SeatInventory::default());
    for class in [
        InputClass::Mouse,
        InputClass::Keyboard,
        InputClass::TouchpadGestures,
        InputClass::HotCorners,
        InputClass::NonUsLayout,
    ] {
        assert_eq!(
            validation.row(class).coverage,
            Coverage::Headless,
            "{} has a synthetic route",
            class.id()
        );
    }
    // The pen has no synthetic route, and no device here: a real gap.
    assert_eq!(
        validation.row(InputClass::TabletPen).coverage,
        Coverage::Gap(GapReason::NoDevice)
    );
}

#[test]
fn a_pen_device_without_a_seat_is_a_no_seat_gap() {
    let inventory = SeatInventory {
        pointer: true,
        keyboard: true,
        touchpad: false,
        tablet: true,
    };
    let validation = InputValidation::classify(false, &inventory);
    assert_eq!(
        validation.row(InputClass::TabletPen).coverage,
        Coverage::Gap(GapReason::NoSeat)
    );
}

#[test]
fn the_marker_names_every_gap_and_stays_one_line() {
    // No seat, no devices: only the pen is a hard gap.
    let validation = InputValidation::classify(false, &SeatInventory::default());
    assert!(validation.is_open());
    assert_eq!(
        validation.marker(),
        "Input validation: OPEN (1 gap: tablet-pen)"
    );
    assert!(!validation.marker().contains('\n'));

    // No seat, with a tablet present: still a gap, but for the seat reason.
    let with_tablet = InputValidation::classify(
        false,
        &SeatInventory {
            tablet: true,
            ..SeatInventory::default()
        },
    );
    assert_eq!(
        with_tablet.marker(),
        "Input validation: OPEN (1 gap: tablet-pen)"
    );

    for line in validation.lines() {
        assert!(line.starts_with("Input validation: "), "{line}");
        assert!(!line.contains('\n'), "{line}");
    }
    assert_eq!(
        validation.row(InputClass::TabletPen).marker(),
        "Input validation: tablet-pen = gap (no-device)"
    );
    assert_eq!(
        validation.row(InputClass::Mouse).marker(),
        "Input validation: mouse = headless"
    );
}

#[test]
fn a_non_us_layout_compiles_and_remaps_keys() {
    // The US layout is the baseline.
    assert_eq!(layout_keysym_for_evdev("us", KEY_Z), Some('z' as u32));
    assert_eq!(layout_keysym_for_evdev("us", KEY_Y), Some('y' as u32));

    // German (QWERTZ) swaps the physical y/z keys.
    assert_eq!(layout_keysym_for_evdev("de", KEY_Z), Some('y' as u32));
    assert_eq!(layout_keysym_for_evdev("de", KEY_Y), Some('z' as u32));
}
