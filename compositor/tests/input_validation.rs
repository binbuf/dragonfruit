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

// --- T324: absolute/touch/tablet and locked input are output-scoped --------

use dragonfruit_compositor::output_input::{
    absolute_output, lock_output, map_into_output, parent_output,
};
use smithay::utils::{Logical, Point, Rectangle, Size};

/// Two headless-style outputs: 1280x720 primary at the origin and a 1920x1080
/// secondary to its right (the synthetic-output harness geometry).
fn two_outputs() -> Vec<Rectangle<i32, Logical>> {
    vec![
        Rectangle::new(Point::from((0, 0)), Size::from((1280, 720))),
        Rectangle::new(Point::from((1280, 0)), Size::from((1920, 1080))),
    ]
}

fn normalized(x: f64, y: f64) -> impl Fn(Size<i32, Logical>) -> Point<f64, Logical> {
    move |size| (x * f64::from(size.w), y * f64::from(size.h)).into()
}

#[test]
fn absolute_input_maps_to_the_output_under_the_device() {
    let outputs = two_outputs();
    // A device reporting 0.75 of the layout is on the secondary output even
    // when no pointer exists yet; 0.25 is on the primary.
    assert_eq!(
        absolute_output(&outputs, None, normalized(0.75, 0.5)),
        Some(outputs[1]),
        "absolute motion at 0.75 must resolve the second output"
    );
    assert_eq!(
        absolute_output(&outputs, None, normalized(0.25, 0.5)),
        Some(outputs[0])
    );
    // The resolved output's geometry interprets the device position.
    assert_eq!(
        map_into_output(outputs[1], normalized(0.75, 0.5)),
        Point::from((2720.0, 540.0)),
        "the second output starts at x=1280 and is 1920 wide"
    );
}

#[test]
fn touch_and_tablet_positions_use_the_same_two_output_mapping() {
    let outputs = two_outputs();
    // Touch/tablet share `absolute_output`; a touch at the second output's
    // centre resolves there and lands at that output's centre, not the
    // primary's.
    let output = absolute_output(&outputs, None, normalized(0.5, 0.5)).unwrap();
    assert_eq!(output, outputs[1]);
    assert_eq!(
        map_into_output(output, normalized(0.5, 0.5)),
        Point::from((2240.0, 540.0))
    );
}

#[test]
fn locked_absolute_uses_the_lock_surfaces_output() {
    let outputs = two_outputs();
    // A single lock surface painted only on the secondary output is the target
    // even though the pointer rests on the primary.
    let pointer = Some(Point::from((100.0, 100.0)));
    assert_eq!(
        lock_output(&[outputs[1]], pointer, normalized(0.1, 0.1)),
        Some(outputs[1]),
        "the lone secondary lock surface must be clickable"
    );
    // With lock surfaces on both outputs the event position selects one.
    assert_eq!(
        lock_output(&outputs, pointer, normalized(0.75, 0.5)),
        Some(outputs[1])
    );
    assert_eq!(
        lock_output(&outputs, pointer, normalized(0.25, 0.5)),
        Some(outputs[0])
    );
}

#[test]
fn popup_output_comes_from_the_parent_window_geometry() {
    let outputs = two_outputs();
    let parent_on_second = Rectangle::new(Point::from((2200, 400)), Size::from((200, 150)));
    assert_eq!(
        parent_output(&outputs, parent_on_second),
        Some(outputs[1]),
        "a popup follows its parent window's output"
    );
    assert_eq!(
        parent_output(&outputs, Rectangle::default()),
        None,
        "an untracked parent falls back to the focused/primary output"
    );
}
