// SPDX-License-Identifier: MIT
//! The window appear transition (T-02.1b).
//!
//! A newly mapped window scales and fades in from an *origin* rectangle to
//! its final geometry. The origin is the owning Dock entry's tile geometry
//! when the shell supplied one over the private protocol
//! (`df_toplevel_manager.set_launch_origin`), and a centered, slightly
//! shrunk copy of the target otherwise — the headless/no-shell degradation
//! the T-02 design requires.
//!
//! The transition is pure timing plus geometry: it lives in the window model
//! ([`WindowModel`](super::WindowModel)) and is stepped by the shared
//! animation clock. The visible scale/fade is applied by the render layer
//! from [`AppearFrame`], so a headless backend observes the model while a
//! rendered backend transforms the window's surface (and its SSD titlebar)
//! with the same numbers.
//!
//! Reduced motion collapses the tween to a single step: the origin is the
//! target on the first step and the same commit path clears it.

use smithay::utils::{Logical, Point, Rectangle, Scale};

use crate::animation::Tween;
use crate::design_tokens::motion;

/// The scale of the centered fallback origin: a window with no Dock tile
/// appears from a copy of its final rect shrunk to this fraction about its
/// own center. Matches the design-system's "scale in from the center".
pub const APPEAR_MIN_SCALE: f64 = 0.8;

/// One window's in-flight appear: where it starts, where it ends, and the
/// shared-clock tween that carries it there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppearTransition {
    /// The rectangle the window appears *from* (a Dock tile, or the centered
    /// fallback derived from `target`).
    pub origin: Rectangle<i32, Logical>,
    /// The window's final geometry.
    pub target: Rectangle<i32, Logical>,
    /// Progress timing on the shared clock ([`motion::WINDOW_OPEN`]).
    pub tween: Tween,
    /// Frames this appearance has been advanced for. The reduced-motion
    /// acceptance is `frames == 1`; a full appear is the tween's frame
    /// count (`duration / FRAME_INTERVAL`).
    pub frames: u64,
    /// Set once the transition reached its end and committed the target.
    /// A completed appearance is kept for introspection (the `query appear`
    /// test hook) but never re-enters the clock.
    pub completed: bool,
}

/// The per-frame resolved form of an [`AppearTransition`]: the interpolated
/// rect plus the render transform the window (and its chrome) is drawn with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AppearFrame {
    /// The interpolated rectangle at this frame.
    pub rect: Rectangle<i32, Logical>,
    /// Non-uniform scale relative to the target.
    pub scale: Scale<f64>,
    /// Translation from the target's top-left, in logical pixels.
    pub offset: Point<i32, Logical>,
    /// Opacity in `0.0..=1.0` (the premultiplied fade).
    pub alpha: f32,
}

impl AppearTransition {
    /// A transition from `origin` to `target` starting at `now_ms`.
    pub fn new(
        origin: Rectangle<i32, Logical>,
        target: Rectangle<i32, Logical>,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        AppearTransition {
            origin,
            target,
            tween: Tween::from_motion(now_ms, motion::WINDOW_OPEN, reduced_motion),
            frames: 0,
            completed: false,
        }
    }

    /// The centered fallback origin for a window with no Dock tile: the
    /// target shrunk about its center by [`APPEAR_MIN_SCALE`].
    pub fn centered_origin(target: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        let w = ((f64::from(target.size.w) * APPEAR_MIN_SCALE).round() as i32).max(1);
        let h = ((f64::from(target.size.h) * APPEAR_MIN_SCALE).round() as i32).max(1);
        let center = (
            target.loc.x + target.size.w / 2,
            target.loc.y + target.size.h / 2,
        );
        Rectangle::new((center.0 - w / 2, center.1 - h / 2).into(), (w, h).into())
    }

    /// Eased progress in `0.0..=1.0` at `now_ms`.
    pub fn progress(&self, now_ms: u64) -> f64 {
        self.tween.progress(now_ms)
    }

    /// Whether the transition has reached its end time.
    pub fn is_done(&self, now_ms: u64) -> bool {
        self.tween.is_done(now_ms)
    }

    /// Resolve the transition at `now_ms` into the frame the renderer draws.
    pub fn frame(&self, now_ms: u64) -> AppearFrame {
        let t = self.progress(now_ms);
        let lerp =
            |a: i32, b: i32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as i32;
        let rect = Rectangle::new(
            (
                lerp(self.origin.loc.x, self.target.loc.x),
                lerp(self.origin.loc.y, self.target.loc.y),
            )
                .into(),
            (
                lerp(self.origin.size.w, self.target.size.w).max(1),
                lerp(self.origin.size.h, self.target.size.h).max(1),
            )
                .into(),
        );
        let scale = Scale::from((
            f64::from(rect.size.w) / f64::from(self.target.size.w.max(1)),
            f64::from(rect.size.h) / f64::from(self.target.size.h.max(1)),
        ));
        let offset = (
            rect.loc.x - self.target.loc.x,
            rect.loc.y - self.target.loc.y,
        )
            .into();
        AppearFrame {
            rect,
            scale,
            offset,
            alpha: t.clamp(0.0, 1.0) as f32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), (w, h).into())
    }

    #[test]
    fn centered_origin_shrinks_about_the_window_center() {
        let target = rect(100, 200, 400, 300);
        let origin = AppearTransition::centered_origin(target);
        // Same center.
        assert_eq!(
            (
                origin.loc.x + origin.size.w / 2,
                origin.loc.y + origin.size.h / 2
            ),
            (300, 350)
        );
        assert_eq!((origin.size.w, origin.size.h), (320, 240));
        // A degenerate target never collapses the origin to zero.
        let tiny = AppearTransition::centered_origin(rect(0, 0, 0, 0));
        assert!(tiny.size.w >= 1 && tiny.size.h >= 1);
    }

    #[test]
    fn full_motion_interpolates_from_origin_to_target() {
        let origin = rect(0, 0, 48, 48);
        let target = rect(100, 200, 400, 300);
        let transition = AppearTransition::new(origin, target, 1000, false);

        // At the start of the tween the frame sits at the origin, fully
        // transparent; at the end it is exactly the target.
        let start = transition.frame(1000);
        assert_eq!(start.rect, origin);
        assert_eq!(start.alpha, 0.0);

        let end = transition.frame(1160);
        assert_eq!(end.rect, target);
        assert_eq!(end.scale, Scale::from((1.0, 1.0)));
        assert_eq!(end.offset, Point::from((0, 0)));
        assert_eq!(end.alpha, 1.0);
        assert!(transition.is_done(1160));

        // Mid-flight the rect is strictly between the two, and the scale is
        // the current size over the target size.
        let mid = transition.frame(1080);
        assert!(mid.rect.loc.x > origin.loc.x && mid.rect.loc.x < target.loc.x);
        assert!(mid.rect.size.w > origin.size.w && mid.rect.size.w < target.size.w);
        assert!((f64::from(mid.rect.size.w) / f64::from(target.size.w) - mid.scale.x).abs() < 1e-9);
        assert!(mid.alpha > 0.0 && mid.alpha <= 1.0);
    }

    #[test]
    fn reduced_motion_is_the_target_on_the_first_step() {
        let transition =
            AppearTransition::new(rect(0, 0, 48, 48), rect(100, 200, 400, 300), 1000, true);
        assert!(transition.is_done(1000), "a zero-duration tween is done");
        let frame = transition.frame(1000);
        assert_eq!(frame.rect, rect(100, 200, 400, 300));
        assert_eq!(frame.alpha, 1.0);
        assert_eq!(frame.offset, Point::from((0, 0)));
    }

    #[test]
    fn a_dock_tile_origin_scales_up_to_one() {
        // A Dock tile is much smaller than the window: the scale starts near
        // zero and reaches 1.0.
        let origin = rect(10, 10, 48, 48);
        let target = rect(200, 200, 800, 600);
        let transition = AppearTransition::new(origin, target, 0, false);
        let start = transition.frame(0);
        assert!(start.scale.x > 0.0 && start.scale.x < 1.0);
        assert!(
            start.scale.x < start.scale.y,
            "tile aspect differs from window"
        );
        let end = transition.frame(motion::WINDOW_OPEN.duration_ms as u64);
        assert_eq!(end.scale, Scale::from((1.0, 1.0)));
    }
}
