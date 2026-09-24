// SPDX-License-Identifier: MIT
//! The window lifecycle motion (T-02.1b appear; T-02.2 minimize/restore).
//!
//! A window scales and fades between an *origin* rectangle and its final
//! geometry. The origin is the owning Dock entry's tile geometry when the
//! shell supplied one over the private protocol
//! (`df_toplevel_manager.set_launch_origin`), and a centered, slightly shrunk
//! copy of the target otherwise — the headless/no-shell degradation the T-02
//! design requires.
//!
//! Three motions share this one type:
//!
//! * **appear** — a newly mapped window grows/fades in from the origin;
//! * **restore** — a minimized window grows back out of the origin;
//! * **minimize** — a visible window shrinks/fades into the origin.
//!
//! Appear and restore are the same interpolation (origin → target, alpha
//! `0 → 1`); minimize is its reverse (target → origin, alpha `1 → 0`). Only
//! the direction differs, so one [`MotionFrame`] carries the render transform
//! for all three and the render layer needs no second effect path.
//!
//! The transition is pure timing plus geometry: it lives in the window model
//! ([`WindowModel`](super::WindowModel)) and is stepped by the shared
//! animation clock. The visible scale/fade is applied by the render layer
//! from [`MotionFrame`], so a headless backend observes the model while a
//! rendered backend transforms the window's surface (and its SSD titlebar)
//! with the same numbers.
//!
//! Reduced motion collapses the tween to a single step: the origin is the
//! target on the first step and the same commit path clears it. For minimize
//! `dock.minimizedAnimation=none` is exactly the reduced-motion behavior
//! (the states still change; only the motion collapses).

use smithay::utils::{Logical, Point, Rectangle, Scale};

use crate::animation::Tween;
use crate::design_tokens::{motion, Motion};

/// The scale of the centered fallback origin: a window with no Dock tile
/// appears from (or minimizes into) a copy of its final rect shrunk to this
/// fraction about its own center. Matches the design-system's "scale in from
/// the center".
pub const APPEAR_MIN_SCALE: f64 = 0.8;

/// Which lifecycle motion a [`WindowMotion`] is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowMotionKind {
    /// A newly mapped window fades/scales in from the origin.
    Appear,
    /// A visible window fades/scales into the origin (the Dock tile).
    Minimize,
    /// A minimized window fades/scales back out of the origin.
    Restore,
}

impl WindowMotionKind {
    /// A stable label for the shell protocol and the `query motion` test hook.
    pub const fn name(self) -> &'static str {
        match self {
            WindowMotionKind::Appear => "appear",
            WindowMotionKind::Minimize => "minimize",
            WindowMotionKind::Restore => "restore",
        }
    }

    /// Whether the interpolation runs `target → origin` (a shrink/fade-out).
    pub const fn is_reversing(self) -> bool {
        matches!(self, WindowMotionKind::Minimize)
    }

    /// The design-system motion token this kind is timed with. Minimize
    /// shrinks out on the close curve; appear/restore grow in on the open
    /// curve.
    const fn token(self) -> Motion {
        match self {
            WindowMotionKind::Minimize => motion::WINDOW_CLOSE,
            WindowMotionKind::Appear | WindowMotionKind::Restore => motion::WINDOW_OPEN,
        }
    }
}

/// One window's in-flight (or completed) lifecycle motion: where it starts,
/// where it ends, and the shared-clock tween that carries it there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowMotion {
    /// Which lifecycle transition this is (drives the interpolation
    /// direction and the timing token).
    pub kind: WindowMotionKind,
    /// The Dock tile (or centered fallback) the window lives at when the
    /// motion is at its origin end.
    pub origin: Rectangle<i32, Logical>,
    /// The window's final geometry — always the render transform's reference
    /// rectangle, so input and layout never move.
    pub target: Rectangle<i32, Logical>,
    /// Progress timing on the shared clock.
    pub tween: Tween,
    /// Frames this motion has been advanced for. The reduced-motion
    /// acceptance is `frames == 1`.
    pub frames: u64,
    /// Set once the transition reached its end. A completed motion is kept
    /// for introspection (the `query motion` test hook) but never re-enters
    /// the clock.
    pub completed: bool,
}

/// The per-frame resolved form of a [`WindowMotion`]: the interpolated rect
/// plus the render transform the window (and its chrome) is drawn with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionFrame {
    /// The interpolated rectangle at this frame.
    pub rect: Rectangle<i32, Logical>,
    /// Non-uniform scale relative to the target.
    pub scale: Scale<f64>,
    /// Translation from the target's top-left, in logical pixels.
    pub offset: Point<i32, Logical>,
    /// Opacity in `0.0..=1.0` (the premultiplied fade).
    pub alpha: f32,
}

impl WindowMotion {
    /// A motion of `kind` from `origin` to `target` starting at `now_ms`.
    pub fn new(
        kind: WindowMotionKind,
        origin: Rectangle<i32, Logical>,
        target: Rectangle<i32, Logical>,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        WindowMotion {
            kind,
            origin,
            target,
            tween: Tween::from_motion(now_ms, kind.token(), reduced_motion),
            frames: 0,
            completed: false,
        }
    }

    /// A convenience constructor for a window appearing into `target` from
    /// `origin`.
    pub fn appear(
        origin: Rectangle<i32, Logical>,
        target: Rectangle<i32, Logical>,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        WindowMotion::new(
            WindowMotionKind::Appear,
            origin,
            target,
            now_ms,
            reduced_motion,
        )
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
    ///
    /// The rect always interpolates from the motion's start to its end; the
    /// scale/offset are relative to `target` (the window's committed
    /// geometry), so the render transform never depends on the direction.
    pub fn frame(&self, now_ms: u64) -> MotionFrame {
        let t = self.progress(now_ms);
        let (start, end, alpha) = if self.kind.is_reversing() {
            (self.target, self.origin, 1.0 - t)
        } else {
            (self.origin, self.target, t)
        };
        let lerp =
            |a: i32, b: i32| (f64::from(a) + (f64::from(b) - f64::from(a)) * t).round() as i32;
        let rect = Rectangle::new(
            (lerp(start.loc.x, end.loc.x), lerp(start.loc.y, end.loc.y)).into(),
            (
                lerp(start.size.w, end.size.w).max(1),
                lerp(start.size.h, end.size.h).max(1),
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
        MotionFrame {
            rect,
            scale,
            offset,
            alpha: alpha.clamp(0.0, 1.0) as f32,
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
        let origin = WindowMotion::centered_origin(target);
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
        let tiny = WindowMotion::centered_origin(rect(0, 0, 0, 0));
        assert!(tiny.size.w >= 1 && tiny.size.h >= 1);
    }

    #[test]
    fn full_motion_interpolates_from_origin_to_target() {
        let origin = rect(0, 0, 48, 48);
        let target = rect(100, 200, 400, 300);
        let transition = WindowMotion::appear(origin, target, 1000, false);

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
            WindowMotion::appear(rect(0, 0, 48, 48), rect(100, 200, 400, 300), 1000, true);
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
        let transition = WindowMotion::appear(origin, target, 0, false);
        let start = transition.frame(0);
        assert!(start.scale.x > 0.0 && start.scale.x < 1.0);
        assert!(
            start.scale.x < start.scale.y,
            "tile aspect differs from window"
        );
        let end = transition.frame(motion::WINDOW_OPEN.duration_ms as u64);
        assert_eq!(end.scale, Scale::from((1.0, 1.0)));
    }

    #[test]
    fn minimize_runs_target_to_origin_and_fades_out() {
        let tile = rect(10, 10, 48, 48);
        let target = rect(200, 200, 800, 600);
        let transition = WindowMotion::new(WindowMotionKind::Minimize, tile, target, 0, false);

        // At t=0 the window is exactly its geometry, opaque.
        let start = transition.frame(0);
        assert_eq!(start.rect, target);
        assert_eq!(start.scale, Scale::from((1.0, 1.0)));
        assert_eq!(start.offset, Point::from((0, 0)));
        assert_eq!(start.alpha, 1.0);

        // At the end it has shrunk into the tile and faded out.
        let end_ms = motion::WINDOW_CLOSE.duration_ms as u64;
        let end = transition.frame(end_ms);
        assert_eq!(end.rect, tile);
        assert!((f64::from(end.rect.size.w) / f64::from(target.size.w) - end.scale.x).abs() < 1e-9);
        assert!(end.scale.x < 1.0);
        assert_eq!(
            end.offset,
            Point::from((tile.loc.x - target.loc.x, tile.loc.y - target.loc.y))
        );
        assert_eq!(end.alpha, 0.0);

        // Mid-flight it is strictly between the tile and the window.
        let mid = transition.frame(end_ms / 2);
        assert!(mid.rect.size.w > tile.size.w && mid.rect.size.w < target.size.w);
        assert!(mid.alpha > 0.0 && mid.alpha < 1.0);
    }

    #[test]
    fn reduced_motion_minimize_still_reports_the_tile() {
        let tile = rect(10, 10, 48, 48);
        let target = rect(200, 200, 800, 600);
        let transition = WindowMotion::new(WindowMotionKind::Minimize, tile, target, 1000, true);
        assert!(transition.is_done(1000));
        let frame = transition.frame(1000);
        assert_eq!(frame.rect, tile);
        assert_eq!(frame.alpha, 0.0);
    }
}
