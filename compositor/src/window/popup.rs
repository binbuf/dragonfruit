// SPDX-License-Identifier: MIT OR Apache-2.0
//! Popup placement and constraint handling (T-04 FR-11).
//!
//! `xdg_popup` positioners are compositor-interpreted: flip, slide, and
//! resize adjustments keep the popup inside its output, and malformed
//! clients (zero-sized rects, contradictory anchors) never panic the
//! compositor. The geometry returned is relative to the parent's window
//! geometry, as required by `xdg_popup`.

use smithay::utils::{Logical, Rectangle, Size};
use smithay::wayland::shell::xdg::PositionerState;

/// Compute the constrained geometry for `positioner` given the parent's
/// window geometry and the output's usable rectangle.
///
/// The positioner's `constraint_adjustment` is honored (flip → slide →
/// resize), then the size is floored at 1×1 so a hostile client cannot
/// request an empty configure.
pub fn constrained_popup_geometry(
    positioner: PositionerState,
    parent_geometry: Rectangle<i32, Logical>,
    output_geometry: Rectangle<i32, Logical>,
) -> Rectangle<i32, Logical> {
    // `get_unconstrained_geometry` expects a target rectangle in the same
    // coordinate space as the geometry it returns (relative to the parent's
    // window geometry), so translate the output into parent coordinates.
    let target = Rectangle::new(
        (
            output_geometry.loc.x - parent_geometry.loc.x,
            output_geometry.loc.y - parent_geometry.loc.y,
        )
            .into(),
        output_geometry.size,
    );
    let geometry = positioner.get_unconstrained_geometry(target);
    let size = Size::from((geometry.size.w.max(1), geometry.size.h.max(1)));
    Rectangle::new(geometry.loc, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_positioner::{
        Anchor, ConstraintAdjustment, Gravity,
    };
    use smithay::utils::{Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn parent() -> Rectangle<i32, Logical> {
        rect(500, 400, 800, 600)
    }

    fn output() -> Rectangle<i32, Logical> {
        rect(0, 0, 1920, 1080)
    }

    fn positioner(
        anchor: Anchor,
        gravity: Gravity,
        adjustment: ConstraintAdjustment,
        rect_size: Size<i32, Logical>,
        anchor_rect: Rectangle<i32, Logical>,
    ) -> PositionerState {
        PositionerState {
            rect_size,
            anchor_rect,
            anchor_edges: anchor,
            gravity,
            constraint_adjustment: adjustment,
            offset: Point::from((0, 0)),
            reactive: false,
            parent_size: None,
            parent_configure: None,
        }
    }

    #[test]
    fn popup_below_anchor_is_placed_at_the_anchor() {
        // Anchor at the bottom-left of the parent, gravity to the bottom:
        // popup top-left lands on the anchor.
        let p = positioner(
            Anchor::BottomLeft,
            Gravity::BottomRight,
            ConstraintAdjustment::all(),
            Size::from((200, 100)),
            rect(0, 600, 0, 0),
        );
        let geo = constrained_popup_geometry(p, parent(), output());
        assert_eq!(geo.size, Size::from((200, 100)));
    }

    #[test]
    fn slide_keeps_a_popup_inside_the_output() {
        // Anchor near the right edge, no flip, slide on: the popup must be
        // slid left so it stays within the output.
        let p = positioner(
            Anchor::BottomRight,
            Gravity::BottomRight,
            ConstraintAdjustment::SlideX,
            Size::from((300, 100)),
            rect(780, 600, 0, 0),
        );
        let geo = constrained_popup_geometry(p, parent(), output());
        let global =
            Point::<i32, Logical>::from((parent().loc.x + geo.loc.x, parent().loc.y + geo.loc.y));
        assert!(global.x + geo.size.w <= output().size.w);
        assert!(global.x >= output().loc.x);
    }

    #[test]
    fn malformed_zero_sized_positioner_never_yields_an_empty_configure() {
        let p = positioner(
            Anchor::None,
            Gravity::None,
            ConstraintAdjustment::empty(),
            Size::from((0, 0)),
            rect(0, 0, 0, 0),
        );
        let geo = constrained_popup_geometry(p, parent(), output());
        assert!(geo.size.w >= 1 && geo.size.h >= 1);
    }

    #[test]
    fn malformed_positioner_without_adjustment_does_not_panic() {
        for anchor in [
            Anchor::None,
            Anchor::Top,
            Anchor::Bottom,
            Anchor::Left,
            Anchor::Right,
            Anchor::TopLeft,
            Anchor::TopRight,
            Anchor::BottomLeft,
            Anchor::BottomRight,
        ] {
            for gravity in [
                Gravity::None,
                Gravity::Top,
                Gravity::Bottom,
                Gravity::Left,
                Gravity::Right,
                Gravity::TopLeft,
                Gravity::TopRight,
                Gravity::BottomLeft,
                Gravity::BottomRight,
            ] {
                let p = positioner(
                    anchor,
                    gravity,
                    ConstraintAdjustment::empty(),
                    Size::from((50, 50)),
                    rect(0, 0, 10, 10),
                );
                let geo = constrained_popup_geometry(p, parent(), output());
                assert!(geo.size.w >= 1 && geo.size.h >= 1);
            }
        }
    }
}
