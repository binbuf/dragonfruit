// SPDX-License-Identifier: MIT
//! Desktop Reveal layout (T-05.5).
//!
//! Desktop Reveal (Ctrl+Down) is not a second transition: it is the same
//! overview pipeline ([`crate::overview::OverviewMachine`]) and the same
//! render-time scene transform as Mission Control and the workspace slide. The
//! only thing it contributes is the geometry: at progress `1` each live window
//! has slid out of its **nearest horizontal edge**, exposing the compositor-
//! drawn wallpaper ([04-shell.md](../../../docs/design/04-shell.md)).
//!
//! This module is pure geometry — no Smithay scene, no GPU — so the escape
//! direction, the interpolation, and the reduced-motion fade are unit-testable
//! headless. The render layer turns [`reveal_frame`] into the reusable
//! [`SceneTransform`](crate::window::SceneTransform)/[`MotionFrame`] exactly
//! like the grid, so the window surface, SSD titlebar, and shadow all follow
//! one mapping.
//!
//! Reduced motion takes the legacy "fade states without translation" path:
//! the rect never moves and the window fades out instead (FR-3).

use smithay::utils::{Logical, Point, Rectangle, Scale};

use crate::window::motion::MotionFrame;

/// The off-screen rectangle `source` escapes to at full progress: wholly past
/// the edge of `area` nearest the window's center. The direction is the sign
/// of the window center relative to the area center, so it is stable for the
/// whole transition (a window never changes sides mid-gesture).
pub fn escape_rect(
    source: Rectangle<i32, Logical>,
    area: Rectangle<i32, Logical>,
) -> Rectangle<i32, Logical> {
    let center_x = f64::from(source.loc.x) + f64::from(source.size.w) / 2.0;
    let area_center_x = f64::from(area.loc.x) + f64::from(area.size.w) / 2.0;
    let x = if center_x < area_center_x {
        area.loc.x - source.size.w
    } else {
        area.loc.x + area.size.w
    };
    Rectangle::new((x, source.loc.y).into(), source.size)
}

/// The render frame a live window is drawn with while the desktop is revealed.
///
/// * **full motion** (`reduced_motion == false`) translates the window from its
///   committed `source` rect out past its nearest edge ([`escape_rect`]) and
///   keeps it opaque, so the background is exposed and the motion is
///   reversible.
/// * **reduced motion** leaves the rect in place and fades `1 → 0`, the
///   documented reduced-motion variant.
///
/// At `progress == 0` the frame is the identity (the normal scene), so the
/// reveal is continuous and interruptible at any value.
pub fn reveal_frame(
    source: Rectangle<i32, Logical>,
    area: Rectangle<i32, Logical>,
    progress: f64,
    reduced_motion: bool,
) -> MotionFrame {
    let t = progress.clamp(0.0, 1.0);
    if reduced_motion {
        return MotionFrame {
            rect: source,
            scale: Scale::from((1.0, 1.0)),
            offset: Point::from((0, 0)),
            alpha: (1.0 - t) as f32,
        };
    }
    let rect = lerp_rect(source, escape_rect(source, area), t);
    MotionFrame {
        rect,
        scale: Scale::from((1.0, 1.0)),
        offset: Point::from((rect.loc.x - source.loc.x, rect.loc.y - source.loc.y)),
        alpha: 1.0,
    }
}

/// Linear interpolation between two rectangles at `t`.
fn lerp_rect(
    from: Rectangle<i32, Logical>,
    to: Rectangle<i32, Logical>,
    t: f64,
) -> Rectangle<i32, Logical> {
    let lerp = |a: i32, b: i32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as i32;
    Rectangle::new(
        (lerp(from.loc.x, to.loc.x), lerp(from.loc.y, to.loc.y)).into(),
        (
            lerp(from.size.w, to.size.w).max(1),
            lerp(from.size.h, to.size.h).max(1),
        )
            .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), (w, h).into())
    }

    #[test]
    fn a_left_window_escapes_left_and_a_right_window_escapes_right() {
        let area = rect(0, 0, 1280, 720);

        let left = rect(100, 200, 300, 200);
        assert_eq!(escape_rect(left, area), rect(-300, 200, 300, 200));

        let right = rect(900, 200, 300, 200);
        assert_eq!(escape_rect(right, area), rect(1280, 200, 300, 200));

        // A window wider than the output still clears the edge entirely.
        let wide = rect(-500, 0, 2000, 400);
        assert_eq!(escape_rect(wide, area).loc.x + wide.size.w, 0);
    }

    #[test]
    fn reveal_lerps_from_the_committed_rect_to_the_escape_rect() {
        let area = rect(0, 0, 1280, 720);
        let source = rect(900, 200, 300, 200);

        // progress 0 is the identity: the normal scene.
        let start = reveal_frame(source, area, 0.0, false);
        assert_eq!(start.rect, source);
        assert_eq!(start.offset, Point::from((0, 0)));
        assert_eq!(start.alpha, 1.0);

        // progress 1 is wholly off the nearest edge, still opaque.
        let end = reveal_frame(source, area, 1.0, false);
        assert_eq!(end.rect, escape_rect(source, area));
        assert_eq!(end.rect.loc.x, area.size.w);
        assert_eq!(end.alpha, 1.0);
        assert_eq!(end.scale, Scale::from((1.0, 1.0)));
        assert_eq!(end.offset, Point::from((area.size.w - source.loc.x, 0)));

        // Mid-flight it is strictly between the committed and escape rects.
        let mid = reveal_frame(source, area, 0.5, false);
        assert!(mid.rect.loc.x > source.loc.x && mid.rect.loc.x < end.rect.loc.x);
        assert!(mid.alpha > 0.0 && mid.alpha <= 1.0);
    }

    #[test]
    fn reduced_motion_fades_in_place_without_translating() {
        let area = rect(0, 0, 1280, 720);
        let source = rect(100, 200, 300, 200);

        let start = reveal_frame(source, area, 0.0, true);
        assert_eq!(start.rect, source);
        assert_eq!(start.offset, Point::from((0, 0)));
        assert_eq!(start.alpha, 1.0);

        let end = reveal_frame(source, area, 1.0, true);
        assert_eq!(end.rect, source, "reduced motion never translates");
        assert_eq!(end.offset, Point::from((0, 0)));
        assert_eq!(end.alpha, 0.0, "the window fades out instead");

        let mid = reveal_frame(source, area, 0.25, true);
        assert_eq!(mid.rect, source);
        assert!(mid.alpha > 0.0 && mid.alpha < 1.0);
    }

    #[test]
    fn progress_is_clamped_so_overshoot_never_moves_past_the_edge() {
        let area = rect(0, 0, 1280, 720);
        let source = rect(900, 200, 300, 200);
        assert_eq!(
            reveal_frame(source, area, 2.0, false).rect,
            escape_rect(source, area)
        );
        assert_eq!(reveal_frame(source, area, -1.0, false).rect, source);
    }
}
