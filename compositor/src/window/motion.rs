// SPDX-License-Identifier: MIT
//! The window lifecycle motion (T-02.1b appear; T-02.2 minimize/restore;
//! T-02.3 zoom/fullscreen; T-02.4a close).
//!
//! A window scales and fades between an *origin* rectangle and its final
//! geometry. For the launch motions the origin is the owning Dock entry's
//! tile geometry when the shell supplied one over the private protocol
//! (`df_toplevel_manager.set_launch_origin`), and a centered, slightly shrunk
//! copy of the target otherwise — the headless/no-shell degradation the T-02
//! design requires. For zoom/fullscreen the origin is the geometry the window
//! occupied *before* the state change.
//!
//! Six motions share this one type:
//!
//! * **appear** — a newly mapped window grows/fades in from the origin;
//! * **restore** — a minimized window grows back out of the origin;
//! * **minimize** — a visible window shrinks/fades into the origin;
//! * **close** — a closing window shrinks/fades out into the origin;
//! * **zoom** — a floating window grows into / out of the usable area;
//! * **fullscreen** — a window grows into / out of the fullscreen geometry.
//!
//! Appear and restore are the same interpolation (origin → target, alpha
//! `0 → 1`); minimize and close are its reverse (target → origin, alpha
//! `1 → 0`). Zoom and fullscreen interpolate geometry with **alpha fixed at
//! 1.0**: the window is visible at both ends, so there is nothing to fade. One
//! [`MotionFrame`] carries the render transform for all six and the render
//! layer needs no second effect path.
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

use smithay::utils::{Logical, Point, Rectangle, Scale, Size};

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
    /// A closing window fades/scales out into the origin. The window is
    /// removed from the model only when the motion completes (T-02.4a).
    Close,
    /// A floating window grows into (or shrinks out of) its zoomed geometry.
    Zoom,
    /// A window grows into (or shrinks out of) its fullscreen geometry.
    Fullscreen,
}

impl WindowMotionKind {
    /// A stable label for the shell protocol and the `query motion` test hook.
    pub const fn name(self) -> &'static str {
        match self {
            WindowMotionKind::Appear => "appear",
            WindowMotionKind::Minimize => "minimize",
            WindowMotionKind::Restore => "restore",
            WindowMotionKind::Close => "close",
            WindowMotionKind::Zoom => "zoom",
            WindowMotionKind::Fullscreen => "fullscreen",
        }
    }

    /// Whether the interpolation runs `target → origin` (a shrink/fade-out).
    pub const fn is_reversing(self) -> bool {
        matches!(self, WindowMotionKind::Minimize | WindowMotionKind::Close)
    }

    /// Whether the motion fades the window at either end.
    ///
    /// Appear/restore fade in and minimize/close fade out; zoom/fullscreen are
    /// visible at both ends, so they interpolate geometry with alpha `1.0`.
    pub const fn fades(self) -> bool {
        !matches!(self, WindowMotionKind::Zoom | WindowMotionKind::Fullscreen)
    }

    /// Whether the window is held as a **ghost** (unmapped from `Space`) for
    /// the duration of this motion.
    ///
    /// Minimize and close take the window out of the layout and the input path
    /// immediately; the render layer draws their surface (and titlebar) from
    /// [`WindowModel::active_motions`](super::WindowModel::active_motions)
    /// instead of the `Space` walk. Appear/restore/zoom/fullscreen stay
    /// mapped.
    pub const fn is_ghost(self) -> bool {
        matches!(self, WindowMotionKind::Minimize | WindowMotionKind::Close)
    }

    /// Whether this is the close ghost. A live close owns the model entry's
    /// removal; reversing one (T-02.4b) replaces the motion, which drops the
    /// pending removal before it can commit.
    pub const fn is_close(self) -> bool {
        matches!(self, WindowMotionKind::Close)
    }

    /// The design-system motion token this kind is timed with. Minimize and
    /// close shrink out on the close curve; every other motion
    /// grows/retargets on the open curve.
    const fn token(self) -> Motion {
        match self {
            WindowMotionKind::Minimize | WindowMotionKind::Close => motion::WINDOW_CLOSE,
            WindowMotionKind::Appear
            | WindowMotionKind::Restore
            | WindowMotionKind::Zoom
            | WindowMotionKind::Fullscreen => motion::WINDOW_OPEN,
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
    /// Non-uniform scale relative to the target (the reference rectangle for
    /// chrome, whose natural size is always the target).
    pub scale: Scale<f64>,
    /// Translation from the target's top-left, in logical pixels.
    pub offset: Point<i32, Logical>,
    /// Opacity in `0.0..=1.0` (the premultiplied fade).
    pub alpha: f32,
}

impl MotionFrame {
    /// The non-uniform scale that maps a surface of `size` onto this frame's
    /// interpolated [`Self::rect`].
    ///
    /// Appear/restore/minimize draw a surface already committed at the
    /// motion's target size, so this equals [`Self::scale`]. A zoom or
    /// fullscreen transition resizes the client mid-flight, so each committed
    /// buffer is scaled to the interpolated rect instead; the geometry stays
    /// continuous even though the surface's intrinsic size changes under it.
    pub fn scale_for(&self, size: Size<i32, Logical>) -> Scale<f64> {
        Scale::from((
            f64::from(self.rect.size.w) / f64::from(size.w.max(1)),
            f64::from(self.rect.size.h) / f64::from(size.h.max(1)),
        ))
    }
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
        } else if self.kind.fades() {
            (self.origin, self.target, t)
        } else {
            // Zoom/fullscreen are visible at both ends: geometry only.
            (self.origin, self.target, 1.0)
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

    #[test]
    fn close_runs_target_to_origin_and_fades_out() {
        let tile = rect(24, 30, 48, 48);
        let target = rect(200, 200, 800, 600);
        let transition = WindowMotion::new(WindowMotionKind::Close, tile, target, 0, false);

        // At t=0 the window is exactly its geometry, opaque, and it is a
        // ghost (out of the layout) for the whole motion.
        let start = transition.frame(0);
        assert_eq!(start.rect, target);
        assert_eq!(start.scale, Scale::from((1.0, 1.0)));
        assert_eq!(start.alpha, 1.0);
        assert!(WindowMotionKind::Close.is_ghost());
        assert!(WindowMotionKind::Close.is_reversing());
        assert!(WindowMotionKind::Close.fades());
        assert!(!WindowMotionKind::Appear.is_ghost());

        // At the end it has shrunk into the tile and faded out.
        let end_ms = motion::WINDOW_CLOSE.duration_ms as u64;
        let end = transition.frame(end_ms);
        assert_eq!(end.rect, tile);
        assert_eq!(end.alpha, 0.0);
        assert!(end.scale.x < 1.0);

        // Mid-flight it is strictly between the tile and the window.
        let mid = transition.frame(end_ms / 2);
        assert!(mid.rect.size.w > tile.size.w && mid.rect.size.w < target.size.w);
        assert!(mid.alpha > 0.0 && mid.alpha < 1.0);
        assert!(transition.is_done(end_ms));
    }

    #[test]
    fn reduced_motion_close_completes_on_the_first_step() {
        let tile = rect(24, 30, 48, 48);
        let target = rect(200, 200, 800, 600);
        let transition = WindowMotion::new(WindowMotionKind::Close, tile, target, 1000, true);
        assert!(transition.is_done(1000));
        let frame = transition.frame(1000);
        assert_eq!(frame.rect, tile);
        assert_eq!(frame.alpha, 0.0);
    }

    /// T-02.4b: a close caught mid-flight reverses into a restore from the
    /// *current* interpolated rect (not the tile), so an interruption never
    /// jumps.
    #[test]
    fn a_close_reversed_mid_flight_restores_from_the_partial_rect() {
        let tile = rect(24, 30, 48, 48);
        let target = rect(200, 200, 800, 600);
        let close = WindowMotion::new(WindowMotionKind::Close, tile, target, 0, false);
        let end_ms = motion::WINDOW_CLOSE.duration_ms as u64;
        let partial = close.frame(end_ms / 2).rect;
        assert!(
            partial.size.w > tile.size.w && partial.size.w < target.size.w,
            "the partial rect is between the tile and the window: {partial:?}"
        );

        // The reversal is a restore whose origin is that partial ghost rect.
        let restore = WindowMotion::new(
            WindowMotionKind::Restore,
            partial,
            target,
            end_ms / 2,
            false,
        );
        let start = restore.frame(end_ms / 2);
        assert_eq!(
            start.rect, partial,
            "the restore starts where the close was"
        );
        assert!(start.alpha < 1.0, "the ghost was fading out");
        // It grows back to the exact window geometry, opaque, and does not
        // remove the window (restore is not a ghost).
        let end = restore.frame(end_ms / 2 + motion::WINDOW_OPEN.duration_ms as u64);
        assert_eq!(end.rect, target);
        assert_eq!(end.alpha, 1.0);
        assert!(!WindowMotionKind::Restore.is_ghost());
        assert!(!WindowMotionKind::Restore.is_close());
    }

    #[test]
    fn zoom_interpolates_geometries_without_fading() {
        let floating = rect(100, 120, 200, 150);
        let zoomed = rect(0, 0, 1280, 680);
        let transition = WindowMotion::new(WindowMotionKind::Zoom, floating, zoomed, 0, false);

        // At t=0 the window is still at its floating geometry, fully opaque.
        // The target-relative scale (used by the compositor-drawn chrome)
        // starts at the origin/target ratio and grows to 1.0.
        let start = transition.frame(0);
        assert_eq!(start.rect, floating);
        assert_eq!(start.alpha, 1.0, "a zoom never fades");
        let expected_start = Scale::from((
            f64::from(floating.size.w) / f64::from(zoomed.size.w),
            f64::from(floating.size.h) / f64::from(zoomed.size.h),
        ));
        assert_eq!(start.scale, expected_start);
        assert_eq!(
            start.offset,
            Point::from((floating.loc.x - zoomed.loc.x, floating.loc.y - zoomed.loc.y))
        );

        // At the end it is exactly the zoomed geometry.
        let end_ms = motion::WINDOW_OPEN.duration_ms as u64;
        let end = transition.frame(end_ms);
        assert_eq!(end.rect, zoomed);
        assert_eq!(end.alpha, 1.0);
        assert_eq!(end.scale, Scale::from((1.0, 1.0)));
        assert_eq!(end.offset, Point::from((0, 0)));
        assert!(transition.is_done(end_ms));

        // Mid-flight the rect is strictly between the two (the translate and
        // scale the renderer draws), still opaque.
        let mid = transition.frame(end_ms / 2);
        assert!(mid.rect.size.w > floating.size.w && mid.rect.size.w < zoomed.size.w);
        assert_eq!(mid.alpha, 1.0);
        assert!(mid.scale.x > 0.0 && mid.scale.x < 1.0);
    }

    #[test]
    fn zoom_unzoom_runs_target_to_origin() {
        // An unzoom is a zoom whose origin is the zoomed geometry.
        let floating = rect(100, 120, 200, 150);
        let zoomed = rect(0, 0, 1280, 680);
        let transition = WindowMotion::new(WindowMotionKind::Zoom, zoomed, floating, 0, false);

        let start = transition.frame(0);
        assert_eq!(start.rect, zoomed);
        assert_eq!(start.alpha, 1.0);
        let end_ms = motion::WINDOW_OPEN.duration_ms as u64;
        assert_eq!(transition.frame(end_ms).rect, floating);
        let mid = transition.frame(end_ms / 2);
        assert!(mid.rect.size.w < zoomed.size.w && mid.rect.size.w > floating.size.w);
    }

    #[test]
    fn fullscreen_motion_is_geometry_only_and_reduced_motion_is_one_frame() {
        let floating = rect(100, 120, 200, 150);
        let fullscreen = rect(0, 0, 1280, 720);
        let transition =
            WindowMotion::new(WindowMotionKind::Fullscreen, floating, fullscreen, 0, false);
        assert_eq!(transition.frame(0).alpha, 1.0);
        let end_ms = motion::WINDOW_OPEN.duration_ms as u64;
        assert_eq!(transition.frame(end_ms).rect, fullscreen);

        let reduced = WindowMotion::new(
            WindowMotionKind::Fullscreen,
            floating,
            fullscreen,
            1000,
            true,
        );
        assert!(reduced.is_done(1000));
        assert_eq!(reduced.frame(1000).rect, fullscreen);
        assert_eq!(reduced.frame(1000).alpha, 1.0);
    }

    #[test]
    fn surface_scale_maps_each_buffer_size_onto_the_interpolated_rect() {
        let floating = rect(100, 120, 200, 150);
        let zoomed = rect(0, 0, 1280, 680);
        let transition = WindowMotion::new(WindowMotionKind::Zoom, floating, zoomed, 0, false);
        let end_ms = motion::WINDOW_OPEN.duration_ms as u64;
        let mid = transition.frame(end_ms / 2);

        // A surface already at the target size uses the target-relative scale.
        assert_eq!(mid.scale_for(zoomed.size), mid.scale);
        // A surface still at the old (floating) size is scaled down so that it
        // exactly fills the interpolated rect, even though the target is
        // bigger.
        let scaled = mid.scale_for(floating.size);
        let rendered_w = (f64::from(floating.size.w) * scaled.x).round() as i32;
        assert_eq!(rendered_w, mid.rect.size.w);
        // A degenerate size never divides by zero.
        let tiny = mid.scale_for(Size::from((0, 0)));
        assert!(tiny.x.is_finite() && tiny.y.is_finite());
    }
}
