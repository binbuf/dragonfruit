// SPDX-License-Identifier: MIT OR Apache-2.0
//! Window placement: centered cascade and transient dialogs (T-04).
//!
//! New windows open near the center of the active Space with a per-output
//! cascade offset. The cascade **wraps** once it would walk the window off
//! the output rather than continuing past the edge (FR-5). Transient
//! dialogs center on their parent instead (FR-6).

use smithay::utils::{Logical, Point, Rectangle, Size};

/// Cascade step in logical pixels.
pub const CASCADE_STEP: i32 = 24;

/// The most cascade slots a single output ever uses, so the offset stays
/// visually bounded even on very large displays.
pub const CASCADE_SLOTS: i32 = 8;

/// A monotonically increasing cascade index. One exists per output so
/// consecutive windows on the same output offset from each other while
/// windows on different outputs start fresh.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cascade {
    index: i32,
}

impl Cascade {
    /// A fresh cascade starting at the first slot.
    pub const fn new() -> Self {
        Cascade { index: 0 }
    }

    /// The current slot and the next one.
    pub fn next(&mut self) -> i32 {
        let current = self.index;
        self.index = self.index.wrapping_add(1);
        current
    }

    /// Reset to the first slot (e.g. the last window on the output closed).
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// The current slot without advancing.
    pub fn current(&self) -> i32 {
        self.index
    }
}

/// How many cascade slots keep a window of `window` size inside `output`.
///
/// A window larger than the output (or an output with no slack) yields a
/// single slot, so the window is simply centered rather than pushed off.
pub fn cascade_slots(output: Size<i32, Logical>, window: Size<i32, Logical>, step: i32) -> i32 {
    if step <= 0 {
        return 1;
    }
    let slack_x = (output.w - window.w).max(0);
    let slack_y = (output.h - window.h).max(0);
    (slack_x / step).min(slack_y / step).clamp(1, CASCADE_SLOTS)
}

/// The geometry for the `index`-th cascaded window of `window` size on
/// `output`: centered, then offset by a wrapped cascade.
pub fn cascaded_geometry(
    output: Rectangle<i32, Logical>,
    window: Size<i32, Logical>,
    index: i32,
    step: i32,
) -> Rectangle<i32, Logical> {
    let slots = cascade_slots(output.size, window, step);
    let offset = step * index.rem_euclid(slots);
    let base = Point::<i32, Logical>::from((
        output.loc.x + (output.size.w - window.w).max(0) / 2,
        output.loc.y + (output.size.h - window.h).max(0) / 2,
    ));
    Rectangle::new(base + Point::from((offset, offset)), window)
}

/// Center a transient dialog of `window` size on `parent`.
pub fn centered_on(
    parent: Rectangle<i32, Logical>,
    window: Size<i32, Logical>,
) -> Rectangle<i32, Logical> {
    let loc = Point::<i32, Logical>::from((
        parent.loc.x + (parent.size.w - window.w) / 2,
        parent.loc.y + (parent.size.h - window.h) / 2,
    ));
    Rectangle::new(loc, window)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn output() -> Rectangle<i32, Logical> {
        rect(0, 0, 1920, 1080)
    }

    #[test]
    fn first_window_is_centered() {
        let geo = cascaded_geometry(output(), Size::from((800, 600)), 0, CASCADE_STEP);
        assert_eq!(geo, rect(560, 240, 800, 600));
    }

    #[test]
    fn consecutive_windows_offset_from_center() {
        let mut cascade = Cascade::new();
        let first = cascaded_geometry(
            output(),
            Size::from((800, 600)),
            cascade.next(),
            CASCADE_STEP,
        );
        let second = cascaded_geometry(
            output(),
            Size::from((800, 600)),
            cascade.next(),
            CASCADE_STEP,
        );
        assert_eq!(second.loc.x - first.loc.x, CASCADE_STEP);
        assert_eq!(second.loc.y - first.loc.y, CASCADE_STEP);
    }

    #[test]
    fn cascade_wraps_instead_of_leaving_the_output() {
        let size = Size::from((800, 600));
        let slots = cascade_slots(output().size, size, CASCADE_STEP);
        assert!(slots > 1);
        let positions: Vec<_> = (0..slots)
            .map(|i| cascaded_geometry(output(), size, i, CASCADE_STEP))
            .collect();
        // Every slot stays inside the output.
        for geo in &positions {
            assert!(geo.loc.x >= output().loc.x);
            assert!(geo.loc.y >= output().loc.y);
            assert!(geo.loc.x + geo.size.w <= output().loc.x + output().size.w);
            assert!(geo.loc.y + geo.size.h <= output().loc.y + output().size.h);
        }
        // The slot after the last wraps back to the first.
        let wrapped = cascaded_geometry(output(), size, slots, CASCADE_STEP);
        assert_eq!(wrapped, positions[0]);
    }

    #[test]
    fn oversized_window_is_pinned_to_the_output_origin() {
        let big = Size::from((3000, 2000));
        let geo = cascaded_geometry(output(), big, 5, CASCADE_STEP);
        assert_eq!(geo.loc, output().loc);
        assert_eq!(geo.size, big);
    }

    #[test]
    fn transient_dialog_centers_on_parent() {
        let parent = rect(200, 150, 800, 600);
        let dialog = centered_on(parent, Size::from((400, 300)));
        assert_eq!(dialog, rect(400, 300, 400, 300));
    }

    #[test]
    fn transient_dialog_centers_on_odd_sized_parent() {
        let parent = rect(0, 0, 101, 51);
        let dialog = centered_on(parent, Size::from((40, 20)));
        assert_eq!(dialog, rect(30, 15, 40, 20));
    }
}
