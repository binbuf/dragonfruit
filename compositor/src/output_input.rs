// SPDX-License-Identifier: MIT
//! Output scoping for popups and absolute input (T324).
//!
//! The compositor's multi-monitor model is per-output: one `wl_output` per
//! output, per-output workspaces, and per-output chrome. Two paths used to
//! assume the first output:
//!
//! * an `xdg_popup` was constrained against `space.outputs().next()` instead of
//!   the output its parent window lives on;
//! * `PointerMotionAbsolute`, touch, and tablet events were mapped into the
//!   first output, so a touch on a secondary display moved the pointer on the
//!   primary.
//!
//! This module is the pure geometry half shared by the compositor router and
//! the headless conformance tests. It never reads the scene; callers pass the
//! live output rectangles. The decision is frozen in
//! [ADR 0315](../../docs/design/adr/0315-one-output-global-and-output-scoped-input.md).
//!
//! The rules:
//!
//! * **Absolute input** maps the device's normalized position across the
//!   output-layout bounding box. The output containing that point is the
//!   authority; the output under the current pointer is the fallback when the
//!   layout has a gap; the nearest output is the last resort.
//! * **Locked absolute input** uses the lock surface's output. A single live
//!   lock surface always wins, so a lock UI painted only on a secondary output
//!   is clickable there.
//! * **Popups** resolve the output containing their parent window's centre,
//!   falling back to the caller's focused/primary output.
//!
//! An explicit per-device `map-to-output` configuration is not part of this
//! task: a device is scoped by the position it reports. That matches the
//! synthetic/libinput normalized-coordinate model and leaves a configured
//! override for a later input-settings task.

use smithay::utils::{Logical, Point, Rectangle, Size};

/// The bounding box of every non-empty output rectangle.
///
/// This is the logical layout absolute devices report against. `None` when no
/// output has a positive size (a session before the first output is ready).
pub fn layout_bounds(outputs: &[Rectangle<i32, Logical>]) -> Option<Rectangle<i32, Logical>> {
    let mut outputs = outputs
        .iter()
        .copied()
        .filter(|output| output.size.w > 0 && output.size.h > 0);
    let first = outputs.next()?;
    let mut min_x = first.loc.x;
    let mut min_y = first.loc.y;
    let mut max_x = first.loc.x + first.size.w;
    let mut max_y = first.loc.y + first.size.h;
    for output in outputs {
        min_x = min_x.min(output.loc.x);
        min_y = min_y.min(output.loc.y);
        max_x = max_x.max(output.loc.x + output.size.w);
        max_y = max_y.max(output.loc.y + output.size.h);
    }
    Some(Rectangle::new(
        Point::from((min_x, min_y)),
        Size::from((max_x - min_x, max_y - min_y)),
    ))
}

/// The output rectangle that contains `point`, if any.
pub fn output_containing(
    outputs: &[Rectangle<i32, Logical>],
    point: Point<f64, Logical>,
) -> Option<Rectangle<i32, Logical>> {
    outputs
        .iter()
        .copied()
        .find(|output| output.size.w > 0 && output.size.h > 0 && output.to_f64().contains(point))
}

/// The nearest non-empty output to `point`, measured by squared distance to
/// the rectangle.
///
/// Used when the absolute position falls in a gap between displays: the user
/// still expects the pointer/touch to land on a real screen rather than
/// disappear.
pub fn nearest_output(
    outputs: &[Rectangle<i32, Logical>],
    point: Point<f64, Logical>,
) -> Option<Rectangle<i32, Logical>> {
    outputs
        .iter()
        .copied()
        .filter(|output| output.size.w > 0 && output.size.h > 0)
        .min_by(|a, b| {
            distance_sq(*a, point)
                .partial_cmp(&distance_sq(*b, point))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Squared distance from `point` to `output`.
fn distance_sq(output: Rectangle<i32, Logical>, point: Point<f64, Logical>) -> f64 {
    let min_x = f64::from(output.loc.x);
    let min_y = f64::from(output.loc.y);
    let max_x = min_x + f64::from(output.size.w);
    let max_y = min_y + f64::from(output.size.h);
    let dx = (min_x - point.x).max(0.0).max(point.x - max_x);
    let dy = (min_y - point.y).max(0.0).max(point.y - max_y);
    dx * dx + dy * dy
}

/// Resolve the output an absolute event belongs to.
///
/// `transform` maps a coordinate-space size to the device position in that
/// space (the backend's `position_transformed`). The event position is first
/// mapped across the whole layout; the output containing it is authoritative.
/// When the mapped point is in a layout gap, the output under `pointer` is the
/// fallback (the device's current output), then the nearest output.
///
/// `None` only when there is no usable output.
pub fn absolute_output<F>(
    outputs: &[Rectangle<i32, Logical>],
    pointer: Option<Point<f64, Logical>>,
    transform: F,
) -> Option<Rectangle<i32, Logical>>
where
    F: Fn(Size<i32, Logical>) -> Point<f64, Logical>,
{
    let layout = layout_bounds(outputs)?;
    let local = transform(layout.size);
    let point = Point::from((
        f64::from(layout.loc.x) + local.x,
        f64::from(layout.loc.y) + local.y,
    ));
    output_containing(outputs, point)
        .or_else(|| pointer.and_then(|pointer| output_containing(outputs, pointer)))
        .or_else(|| nearest_output(outputs, point))
}

/// Map a device position into `output`'s global logical rectangle.
///
/// `transform` is the same size-to-position mapping [`absolute_output`] takes;
/// this call interprets the device coordinate space as the resolved output,
/// which is what "use that output's geometry" means.
pub fn map_into_output<F>(output: Rectangle<i32, Logical>, transform: F) -> Point<f64, Logical>
where
    F: Fn(Size<i32, Logical>) -> Point<f64, Logical>,
{
    let local = transform(output.size);
    Point::from((
        f64::from(output.loc.x) + local.x,
        f64::from(output.loc.y) + local.y,
    ))
}

/// Resolve the output a **locked** absolute event belongs to.
///
/// Each lock surface covers exactly one output. A single live lock surface is
/// always the target, even when it is on a secondary output and the pointer
/// rests on the primary: that is the "lock surface on a secondary output cannot
/// be clicked" fix. With several lock surfaces the event position (then the
/// pointer, then the nearest lock output) selects one.
///
/// `None` when no lock surface has a live output.
pub fn lock_output<F>(
    lock_outputs: &[Rectangle<i32, Logical>],
    pointer: Option<Point<f64, Logical>>,
    transform: F,
) -> Option<Rectangle<i32, Logical>>
where
    F: Fn(Size<i32, Logical>) -> Point<f64, Logical>,
{
    if lock_outputs.len() == 1 {
        return lock_outputs.first().copied();
    }
    let layout = layout_bounds(lock_outputs)?;
    let local = transform(layout.size);
    let point = Point::from((
        f64::from(layout.loc.x) + local.x,
        f64::from(layout.loc.y) + local.y,
    ));
    output_containing(lock_outputs, point)
        .or_else(|| pointer.and_then(|pointer| output_containing(lock_outputs, pointer)))
        .or_else(|| nearest_output(lock_outputs, point))
}

/// Resolve the output a popup belongs to from its parent window's geometry.
///
/// The parent window's centre is the protocol-correct anchor. `None` when the
/// parent has no usable geometry or no output contains it; the caller then
/// falls back to the focused output, then the primary.
pub fn parent_output(
    outputs: &[Rectangle<i32, Logical>],
    parent: Rectangle<i32, Logical>,
) -> Option<Rectangle<i32, Logical>> {
    if parent.size.w <= 0 || parent.size.h <= 0 {
        return None;
    }
    let center = Point::from((
        f64::from(parent.loc.x) + f64::from(parent.size.w) / 2.0,
        f64::from(parent.loc.y) + f64::from(parent.size.h) / 2.0,
    ));
    output_containing(outputs, center)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn two_outputs() -> Vec<Rectangle<i32, Logical>> {
        vec![rect(0, 0, 1280, 720), rect(1280, 0, 1920, 1080)]
    }

    fn normalized(x: f64, y: f64) -> impl Fn(Size<i32, Logical>) -> Point<f64, Logical> {
        move |size| (x * f64::from(size.w), y * f64::from(size.h)).into()
    }

    #[test]
    fn layout_bounds_is_the_union_of_the_outputs() {
        assert_eq!(layout_bounds(&two_outputs()), Some(rect(0, 0, 3200, 1080)));
        assert_eq!(layout_bounds(&[]), None);
        assert_eq!(layout_bounds(&[rect(0, 0, 0, 0)]), None);
    }

    #[test]
    fn an_absolute_point_picks_the_output_it_maps_onto() {
        let outputs = two_outputs();
        // 0.75 across the 3200 px layout is 2400, inside the second output.
        let second = absolute_output(&outputs, None, normalized(0.75, 0.5)).unwrap();
        assert_eq!(second, rect(1280, 0, 1920, 1080));
        // 0.25 is 800, inside the first.
        let first = absolute_output(&outputs, None, normalized(0.25, 0.5)).unwrap();
        assert_eq!(first, rect(0, 0, 1280, 720));
    }

    #[test]
    fn mapping_into_the_resolved_output_uses_that_outputs_geometry() {
        let second = rect(1280, 0, 1920, 1080);
        let point = map_into_output(second, normalized(0.75, 0.5));
        assert_eq!(point, Point::from((2720.0, 540.0)));
    }

    #[test]
    fn a_layout_gap_falls_back_to_the_pointer_output_then_the_nearest() {
        // A gap between 1280 and 2000: 0.5 of the union (0..3920) is 1960.
        let outputs = vec![rect(0, 0, 1280, 720), rect(2000, 0, 1920, 1080)];
        let pointer = Some(Point::from((100.0, 100.0)));
        let chosen = absolute_output(&outputs, pointer, normalized(0.5, 0.5)).unwrap();
        assert_eq!(chosen, rect(0, 0, 1280, 720));
        let nearest = absolute_output(&outputs, None, normalized(0.5, 0.5)).unwrap();
        assert_eq!(nearest, rect(2000, 0, 1920, 1080));
    }

    #[test]
    fn a_single_lock_surface_is_the_target_even_off_pointer() {
        let second = rect(1280, 0, 1920, 1080);
        let pointer = Some(Point::from((100.0, 100.0)));
        let chosen = lock_output(&[second], pointer, normalized(0.1, 0.1)).unwrap();
        assert_eq!(chosen, second);
    }

    #[test]
    fn several_lock_surfaces_use_the_event_position() {
        let outputs = two_outputs();
        let chosen = lock_output(&outputs, None, normalized(0.75, 0.5)).unwrap();
        assert_eq!(chosen, rect(1280, 0, 1920, 1080));
        let first = lock_output(&outputs, None, normalized(0.25, 0.5)).unwrap();
        assert_eq!(first, rect(0, 0, 1280, 720));
    }

    #[test]
    fn a_popup_follows_its_parent_windows_output() {
        let outputs = two_outputs();
        let parent = rect(2200, 400, 200, 150);
        assert_eq!(
            parent_output(&outputs, parent),
            Some(rect(1280, 0, 1920, 1080))
        );
        // No usable parent geometry: the caller's focused/primary fallback.
        assert_eq!(parent_output(&outputs, Rectangle::default()), None);
    }
}
