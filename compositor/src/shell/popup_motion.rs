// SPDX-License-Identifier: GPL-3.0-only
//! Compositor-owned popup group motion (T355).
//!
//! The Dock chooser used to fade and scale in QML while the compositor drew
//! the sampled material at full strength. That left two independent owners of
//! one visual: the shell's per-frame opacity was on the animation's critical
//! path, and the material could survive the content's shrinking silhouette.
//!
//! This module is the pure half of the pilot that moves the outer transition
//! onto the owning output's clock. A [`PopupMotion`] samples one [`Tween`] for
//! the content, card/arrow silhouette, mask, rim and shadow together. The
//! compositor applies the resulting [`PopupMotionFrame`] to the accepted
//! popup contribution in one pass:
//!
//! * one scale about the popup's bottom edge (the QML `transformOrigin`), so
//!   the card, arrow and shadow stay rigidly attached;
//! * one group alpha for the entire contribution over the lower scene;
//! * a damage union of the previous and current visual extents.
//!
//! [`group_over_underlay`] is the premultiplied reference for that one
//! contribution. It exists so the deterministic pixel fixtures can prove the
//! result against a known underlay, including near-zero opacity, instead of
//! relying on a screenshot. [`independent_over_underlay`] is the *wrong*
//! composition (each overlapping layer multiplied by the same alpha); the
//! fixtures assert the two differ at the midpoint.
//!
//! Reduced motion and a zero-duration policy collapse the tween to one step:
//! the state change still happens, only the travel/scale disappears. A reopen
//! during the closing tail retargets from the current alpha and scale, never
//! from the beginning and never with a snap.

use smithay::utils::{Logical, Point, Rectangle, Scale};

use crate::animation::Tween;
use crate::design_tokens::{motion, Motion};

/// The measured popup close scale (design-system `contextMenu.closeScale`).
/// The pilot keeps the same 3 % shrink the QML motion used.
pub const POPUP_MIN_SCALE: f64 = 0.97;

/// The popup namespaces that use the compositor-owned group motion in this
/// pilot. The Dock chooser is T355; the attached menu chain is T356 and the
/// remaining families are T361.
pub fn namespace_piloted(namespace: &str) -> bool {
    matches!(namespace, "dock-popup" | "menubar-popup")
}

/// Where a popup group's transform is anchored.
///
/// The anchor is part of the accepted visual, not a per-frame choice: a Dock
/// chooser grows out of and shrinks back into its arrow tip (bottom-centre),
/// while an attached menu keeps its top edge under the bar (top-centre).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupMotionOrigin {
    /// Scale about the bottom-centre of the accepted paint extent.
    BottomCenter,
    /// Scale about the top-centre of the accepted paint extent.
    TopCenter,
}

impl PopupMotionOrigin {
    pub const fn name(self) -> &'static str {
        match self {
            PopupMotionOrigin::BottomCenter => "bottom",
            PopupMotionOrigin::TopCenter => "top",
        }
    }

    /// The origin a popup namespace's accepted visual uses.
    pub fn for_namespace(namespace: &str) -> Self {
        if namespace == "menubar-popup" {
            PopupMotionOrigin::TopCenter
        } else {
            PopupMotionOrigin::BottomCenter
        }
    }
}

/// Local A/B diagnostic: `compositor` (default) animates the accepted popup
/// group on the output clock; `qml` keeps the pre-T355 shell-owned QML
/// opacity/scale path. The shell reads the same variable so one setting
/// selects one owner, never both.
pub const ENV_POPUP_MOTION: &str = "DRAGONFRUIT_POPUP_MOTION";

/// Whether the compositor owns the popup outer transition.
pub fn compositor_motion_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        !matches!(
            std::env::var(ENV_POPUP_MOTION)
                .ok()
                .map(|value| value.trim().to_ascii_lowercase())
                .as_deref(),
            Some("qml" | "shell" | "legacy" | "0" | "false" | "off")
        )
    })
}

/// Which outer transition a [`PopupMotion`] is playing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupMotionKind {
    /// Accepted content grows/fades in from the bottom edge.
    Open,
    /// Accepted content shrinks/fades out toward the bottom edge.
    Close,
}

impl PopupMotionKind {
    pub const fn name(self) -> &'static str {
        match self {
            PopupMotionKind::Open => "open",
            PopupMotionKind::Close => "close",
        }
    }

    /// The shared motion token for this kind.
    pub const fn token(self) -> Motion {
        match self {
            PopupMotionKind::Open => motion::POPUP_OPEN,
            PopupMotionKind::Close => motion::POPUP_CLOSE,
        }
    }

    /// The motion token for a popup namespace. The attached menu chain keeps
    /// its measured menu tokens; the Dock chooser and the remaining families
    /// use the popup tokens.
    pub fn token_for(self, namespace: &str) -> Motion {
        match (self, namespace) {
            (PopupMotionKind::Open, "menubar-popup") => motion::MENU_OPEN,
            (PopupMotionKind::Close, "menubar-popup") => motion::MENU_CLOSE,
            _ => self.token(),
        }
    }
}

/// One popup's outer transition on the shared output clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupMotion {
    pub kind: PopupMotionKind,
    pub tween: Tween,
    /// The accepted paint extent in output-local logical coordinates. The
    /// transform is relative to this rectangle, so content, mask, rim and
    /// shadow never drift apart.
    pub target: Rectangle<i32, Logical>,
    pub from_alpha: f32,
    pub to_alpha: f32,
    pub from_scale: f64,
    pub to_scale: f64,
    /// The transform anchor. The whole group shares it, so the content and
    /// its material panels never drift apart.
    pub origin: PopupMotionOrigin,
    /// The visual rect at the previous sample, for the damage union.
    pub last_rect: Rectangle<i32, Logical>,
    /// Frames this motion has been advanced for. The reduced-motion
    /// acceptance is `frames <= 1`.
    pub frames: u64,
    /// Set once the tween reached its end. A completed motion is kept until
    /// the surface unmaps or the retained snapshot expires; it draws nothing.
    pub completed: bool,
}

/// The per-frame form the renderer consumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopupMotionFrame {
    pub kind: PopupMotionKind,
    /// The transformed visual rect in output-local logical coordinates.
    pub rect: Rectangle<i32, Logical>,
    /// The uniform scale applied about the origin.
    pub scale: f64,
    /// The group's transform anchor.
    pub origin: PopupMotionOrigin,
    /// The one group alpha for the whole contribution.
    pub alpha: f32,
    /// Eased progress in `0.0..=1.0`.
    pub progress: f64,
    /// The union of the previous and current visual extents, clipped to the
    /// owning output by the caller.
    pub damage: Rectangle<i32, Logical>,
    /// Whether the tween has reached its end. A completed close contributes
    /// nothing (alpha 0) and a completed open is the settled visual.
    pub completed: bool,
}

impl PopupMotion {
    /// An opening motion into `target`.
    pub fn open(
        target: Rectangle<i32, Logical>,
        origin: PopupMotionOrigin,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        Self::open_for(
            target,
            origin,
            PopupMotionKind::Open.token(),
            now_ms,
            reduced_motion,
        )
    }

    /// An opening motion into `target` using a namespace's token.
    pub fn open_for(
        target: Rectangle<i32, Logical>,
        origin: PopupMotionOrigin,
        token: Motion,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        Self::new(
            PopupMotionKind::Open,
            target,
            origin,
            token,
            now_ms,
            reduced_motion,
            0.0,
            1.0,
            POPUP_MIN_SCALE,
            1.0,
        )
    }

    /// A closing motion out of `target`.
    pub fn close(
        target: Rectangle<i32, Logical>,
        origin: PopupMotionOrigin,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        Self::close_for(
            target,
            origin,
            PopupMotionKind::Close.token(),
            now_ms,
            reduced_motion,
        )
    }

    /// A closing motion out of `target` using a namespace's token.
    pub fn close_for(
        target: Rectangle<i32, Logical>,
        origin: PopupMotionOrigin,
        token: Motion,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        Self::new(
            PopupMotionKind::Close,
            target,
            origin,
            token,
            now_ms,
            reduced_motion,
            1.0,
            0.0,
            1.0,
            POPUP_MIN_SCALE,
        )
    }

    /// Start `kind` from the current sample of `previous`.
    ///
    /// A reopen during the closing tail (or a close during the opening tail)
    /// must not jump: the new tween begins at the visible alpha and scale and
    /// runs the new kind's token from there. This is the review's
    /// "retarget from the current visible state" rule.
    pub fn retarget(
        previous: &PopupMotion,
        kind: PopupMotionKind,
        origin: PopupMotionOrigin,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        Self::retarget_for(previous, kind, origin, kind.token(), now_ms, reduced_motion)
    }

    /// Retarget using a namespace's token.
    pub fn retarget_for(
        previous: &PopupMotion,
        kind: PopupMotionKind,
        origin: PopupMotionOrigin,
        token: Motion,
        now_ms: u64,
        reduced_motion: bool,
    ) -> Self {
        let current = previous.frame(now_ms);
        let (from_alpha, to_alpha, from_scale, to_scale) = match kind {
            PopupMotionKind::Open => (current.alpha, 1.0, current.scale, 1.0),
            PopupMotionKind::Close => (current.alpha, 0.0, current.scale, POPUP_MIN_SCALE),
        };
        Self::new(
            kind,
            previous.target,
            origin,
            token,
            now_ms,
            reduced_motion,
            from_alpha,
            to_alpha,
            from_scale,
            to_scale,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new(
        kind: PopupMotionKind,
        target: Rectangle<i32, Logical>,
        origin: PopupMotionOrigin,
        token: Motion,
        now_ms: u64,
        reduced_motion: bool,
        from_alpha: f32,
        to_alpha: f32,
        from_scale: f64,
        to_scale: f64,
    ) -> Self {
        PopupMotion {
            kind,
            tween: Tween::from_motion(now_ms, token, reduced_motion),
            target,
            from_alpha,
            to_alpha,
            from_scale,
            to_scale,
            origin,
            last_rect: scale_about_origin(target, from_scale, origin),
            frames: 0,
            completed: false,
        }
    }

    /// Eased progress in `0.0..=1.0` at `now_ms`.
    pub fn progress(&self, now_ms: u64) -> f64 {
        self.tween.progress(now_ms)
    }

    /// Whether the tween has reached its end time.
    pub fn is_done(&self, now_ms: u64) -> bool {
        self.tween.is_done(now_ms)
    }

    /// Resolve the transition at `now_ms`.
    ///
    /// The visual rectangle is the accepted paint extent scaled about its
    /// origin: the bottom-centre for a Dock chooser (the arrow stays attached)
    /// and the top-centre for an attached menu (its top edge stays under the
    /// bar). The whole group shares the transform, so the content and its
    /// material panels never drift apart.
    pub fn frame(&self, now_ms: u64) -> PopupMotionFrame {
        let progress = self.progress(now_ms);
        let alpha = lerp_f32(self.from_alpha, self.to_alpha, progress).clamp(0.0, 1.0);
        let scale = lerp_f64(self.from_scale, self.to_scale, progress).max(0.0);
        let rect = scale_about_origin(self.target, scale, self.origin);
        let damage = self.last_rect.merge(rect);
        PopupMotionFrame {
            kind: self.kind,
            rect,
            scale,
            origin: self.origin,
            alpha,
            progress,
            damage,
            completed: self.is_done(now_ms) || self.completed,
        }
    }

    /// Advance one output-clock sample. Returns whether the motion is still
    /// live. The damage union is carried from the previous sample.
    pub fn advance(&mut self, now_ms: u64) -> bool {
        self.frames = self.frames.saturating_add(1);
        let live = !self.is_done(now_ms);
        self.last_rect = self.frame(now_ms).rect;
        if !live {
            self.completed = true;
        }
        live
    }

    /// Whether this motion still needs output-clock frames.
    ///
    /// Completion is the stored flag, not a second time comparison: the tick
    /// that crosses the tween's end still has to step once, set the flag and
    /// present the final alpha-0 (close) or settled (open) frame.
    pub fn is_live(&self, _now_ms: u64) -> bool {
        !self.completed
    }

    /// Whether the visual contribution is empty at `now_ms` (a completed
    /// close, or a zero-duration close).
    pub fn draws_nothing(&self, now_ms: u64) -> bool {
        self.kind == PopupMotionKind::Close && self.frame(now_ms).alpha <= f32::EPSILON
    }
}

/// Scale `target` about its origin, preserving the anchor point.
pub fn scale_about_origin(
    target: Rectangle<i32, Logical>,
    scale: f64,
    origin: PopupMotionOrigin,
) -> Rectangle<i32, Logical> {
    let scale = scale.max(0.0);
    let w = (f64::from(target.size.w.max(1)) * scale).round().max(1.0) as i32;
    let h = (f64::from(target.size.h.max(1)) * scale).round().max(1.0) as i32;
    let center_x = target.loc.x + target.size.w / 2;
    let x = center_x - w / 2;
    let y = match origin {
        PopupMotionOrigin::BottomCenter => target.loc.y + target.size.h - h,
        PopupMotionOrigin::TopCenter => target.loc.y,
    };
    Rectangle::new((x, y).into(), (w, h).into())
}

/// Scale `target` about its bottom-centre, preserving the arrow and shadow
/// attachment point (the Dock chooser's origin).
pub fn scale_about_bottom(target: Rectangle<i32, Logical>, scale: f64) -> Rectangle<i32, Logical> {
    scale_about_origin(target, scale, PopupMotionOrigin::BottomCenter)
}

/// Scale a rectangle about `target`'s origin. Used for a material region so
/// the sampled card shrinks with the accepted content instead of staying at
/// full size behind a fading card.
pub fn scale_rect_about_origin(
    rect: Rectangle<i32, Logical>,
    target: Rectangle<i32, Logical>,
    scale: f64,
    origin: PopupMotionOrigin,
) -> Rectangle<i32, Logical> {
    let scale = scale.max(0.0);
    let center_x = target.loc.x + target.size.w / 2;
    let anchor_y = match origin {
        PopupMotionOrigin::BottomCenter => target.loc.y + target.size.h,
        PopupMotionOrigin::TopCenter => target.loc.y,
    };
    let x = center_x + ((f64::from(rect.loc.x - center_x)) * scale).round() as i32;
    let y = anchor_y + ((f64::from(rect.loc.y - anchor_y)) * scale).round() as i32;
    let w = (f64::from(rect.size.w.max(1)) * scale).round().max(1.0) as i32;
    let h = (f64::from(rect.size.h.max(1)) * scale).round().max(1.0) as i32;
    Rectangle::new((x, y).into(), (w, h).into())
}

/// Scale a rectangle about `target`'s bottom-centre.
pub fn scale_rect_about_bottom(
    rect: Rectangle<i32, Logical>,
    target: Rectangle<i32, Logical>,
    scale: f64,
) -> Rectangle<i32, Logical> {
    scale_rect_about_origin(rect, target, scale, PopupMotionOrigin::BottomCenter)
}

/// The physical origin used by `RescaleRenderElement`.
pub fn origin_physical(
    target: Rectangle<i32, Logical>,
    scale: Scale<f64>,
    origin: PopupMotionOrigin,
) -> Point<i32, smithay::utils::Physical> {
    let physical: Rectangle<i32, smithay::utils::Physical> =
        target.to_physical_precise_round(scale);
    Point::from((
        physical.loc.x + physical.size.w / 2,
        match origin {
            PopupMotionOrigin::BottomCenter => physical.loc.y + physical.size.h,
            PopupMotionOrigin::TopCenter => physical.loc.y,
        },
    ))
}

/// The physical bottom-centre origin used by `RescaleRenderElement`.
pub fn bottom_center_physical(
    target: Rectangle<i32, Logical>,
    scale: Scale<f64>,
) -> Point<i32, smithay::utils::Physical> {
    origin_physical(target, scale, PopupMotionOrigin::BottomCenter)
}

fn lerp_f32(from: f32, to: f32, t: f64) -> f32 {
    from + (to - from) * t as f32
}

fn lerp_f64(from: f64, to: f64, t: f64) -> f64 {
    from + (to - from) * t
}

// --- premultiplied group composition (reference) ---------------------------

/// A premultiplied RGBA colour in `0.0..=1.0`.
pub type Premultiplied = [f32; 4];

/// `top` over `bottom`, both premultiplied.
pub fn over(top: Premultiplied, bottom: Premultiplied) -> Premultiplied {
    let inv = 1.0 - top[3];
    [
        top[0] + bottom[0] * inv,
        top[1] + bottom[1] * inv,
        top[2] + bottom[2] * inv,
        top[3] + bottom[3] * inv,
    ]
}

/// Multiply a premultiplied colour by a group alpha.
pub fn scale_alpha(color: Premultiplied, alpha: f32) -> Premultiplied {
    let alpha = alpha.clamp(0.0, 1.0);
    [
        color[0] * alpha,
        color[1] * alpha,
        color[2] * alpha,
        color[3] * alpha,
    ]
}

/// The correct group contribution over the **original** lower scene.
///
/// `content` is the accepted chooser texture (card, arrow, mask, rim and
/// shadow); `material` is the compositor's sampled backdrop for the same
/// group; `underlay` is the lower scene pixel. The group is first composited
/// internally (`content over material`), then faded once over the underlay.
/// This is why a near-zero alpha returns the underlay exactly.
pub fn group_over_underlay(
    content: Premultiplied,
    material: Premultiplied,
    underlay: Premultiplied,
    alpha: f32,
) -> Premultiplied {
    let group = over(content, material);
    over(scale_alpha(group, alpha), underlay)
}

/// The *wrong* composition: fade the overlapping content and material
/// independently with the same alpha. Retained as the diagnostic comparison
/// so the fixtures can show where the relative contribution changes.
pub fn independent_over_underlay(
    content: Premultiplied,
    material: Premultiplied,
    underlay: Premultiplied,
    alpha: f32,
) -> Premultiplied {
    let faded_material = scale_alpha(material, alpha);
    let faded_content = scale_alpha(content, alpha);
    over(faded_content, over(faded_material, underlay))
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::utils::{Point, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    fn token_ms(kind: PopupMotionKind) -> u64 {
        u64::from(kind.token().duration_ms)
    }

    #[test]
    fn open_starts_transparent_and_ends_settled() {
        let target = rect(100, 200, 240, 320);
        let motion = PopupMotion::open(target, PopupMotionOrigin::BottomCenter, 1_000, false);
        let start = motion.frame(1_000);
        assert_eq!(start.alpha, 0.0);
        assert!((start.scale - POPUP_MIN_SCALE).abs() < 1e-9);
        assert_eq!(start.rect.size.w, (240.0 * POPUP_MIN_SCALE).round() as i32);
        assert_eq!(
            start.rect.loc.y + start.rect.size.h,
            target.loc.y + target.size.h,
            "the bottom edge is the transform origin"
        );

        let end = motion.frame(1_000 + token_ms(PopupMotionKind::Open));
        assert_eq!(end.alpha, 1.0);
        assert_eq!(end.scale, 1.0);
        assert_eq!(end.rect, target);
        assert!(end.completed);
    }

    #[test]
    fn close_starts_settled_and_ends_transparent() {
        let target = rect(100, 200, 240, 320);
        let motion = PopupMotion::close(target, PopupMotionOrigin::BottomCenter, 2_000, false);
        let start = motion.frame(2_000);
        assert_eq!(start.alpha, 1.0);
        assert_eq!(start.rect, target);
        let end = motion.frame(2_000 + token_ms(PopupMotionKind::Close));
        assert_eq!(end.alpha, 0.0);
        assert!((end.scale - POPUP_MIN_SCALE).abs() < 1e-9);
        assert!(end.completed);
        assert!(motion.draws_nothing(2_000 + token_ms(PopupMotionKind::Close)));
    }

    #[test]
    fn midpoint_is_between_the_endpoints_and_the_damage_unions_them() {
        let target = rect(40, 60, 200, 260);
        let motion = PopupMotion::close(target, PopupMotionOrigin::BottomCenter, 0, false);
        let end_ms = token_ms(PopupMotionKind::Close);
        let mid = motion.frame(end_ms / 2);
        assert!(mid.alpha > 0.0 && mid.alpha < 1.0);
        assert!(mid.rect.size.w < target.size.w);
        assert!(mid.rect.size.w > scale_about_bottom(target, POPUP_MIN_SCALE).size.w);
        assert_eq!(mid.damage, target.merge(mid.rect));
        assert!(mid.damage.size.h >= target.size.h);
    }

    #[test]
    fn reduced_motion_completes_on_the_first_step() {
        let target = rect(0, 0, 100, 100);
        for kind in [PopupMotionKind::Open, PopupMotionKind::Close] {
            let motion = match kind {
                PopupMotionKind::Open => {
                    PopupMotion::open(target, PopupMotionOrigin::BottomCenter, 500, true)
                }
                PopupMotionKind::Close => {
                    PopupMotion::close(target, PopupMotionOrigin::BottomCenter, 500, true)
                }
            };
            assert!(motion.is_done(500));
            assert_eq!(motion.frames, 0);
            let frame = motion.frame(500);
            assert!(frame.completed);
            let expected = match kind {
                PopupMotionKind::Open => 1.0,
                PopupMotionKind::Close => 0.0,
            };
            assert_eq!(frame.alpha, expected);
        }
    }

    #[test]
    fn a_quick_reopen_retargets_from_the_current_alpha_and_scale() {
        let target = rect(20, 30, 220, 300);
        let close = PopupMotion::close(target, PopupMotionOrigin::BottomCenter, 0, false);
        let close_ms = token_ms(PopupMotionKind::Close);
        let partial = close.frame(close_ms / 2);
        assert!(partial.alpha < 1.0 && partial.scale < 1.0);

        let reopen = PopupMotion::retarget(
            &close,
            PopupMotionKind::Open,
            PopupMotionOrigin::BottomCenter,
            close_ms / 2,
            false,
        );
        let start = reopen.frame(close_ms / 2);
        assert!((start.alpha - partial.alpha).abs() < 1e-6);
        assert!((start.scale - partial.scale).abs() < 1e-6);
        assert_eq!(
            start.rect, partial.rect,
            "a reopen must not snap to the settled rect"
        );
        let end = reopen.frame(close_ms / 2 + token_ms(PopupMotionKind::Open));
        assert_eq!(end.alpha, 1.0);
        assert_eq!(end.rect, target);
    }

    #[test]
    fn a_close_during_open_retargets_from_the_current_alpha() {
        let target = rect(10, 10, 180, 240);
        let open = PopupMotion::open(target, PopupMotionOrigin::BottomCenter, 0, false);
        let open_ms = token_ms(PopupMotionKind::Open);
        let partial = open.frame(open_ms / 2);
        let close = PopupMotion::retarget(
            &open,
            PopupMotionKind::Close,
            PopupMotionOrigin::BottomCenter,
            open_ms / 2,
            false,
        );
        let start = close.frame(open_ms / 2);
        assert!((start.alpha - partial.alpha).abs() < 1e-6);
        assert_eq!(start.rect, partial.rect);
        let end = close.frame(open_ms / 2 + token_ms(PopupMotionKind::Close));
        assert_eq!(end.alpha, 0.0);
    }

    #[test]
    fn the_full_paint_extent_including_arrow_and_shadow_scales_together() {
        // The chooser's committed paint extent is the card plus the shadow
        // ring plus the rotated arrow tip. The transform is applied to that
        // whole rectangle, never to the card alone.
        let paint = rect(30, 40, 260, 340);
        let motion = PopupMotion::close(paint, PopupMotionOrigin::BottomCenter, 0, false);
        let mid = motion.frame(token_ms(PopupMotionKind::Close) / 2);
        assert!(
            mid.rect.loc.x > paint.loc.x,
            "the left shadow reach shrinks inward with the group"
        );
        assert!(
            mid.rect.loc.x + mid.rect.size.w < paint.loc.x + paint.size.w,
            "the right shadow reach shrinks inward with the group"
        );
        assert_eq!(
            mid.rect.loc.y + mid.rect.size.h,
            paint.loc.y + paint.size.h,
            "the arrow tip stays anchored to the bottom edge"
        );
    }

    #[test]
    fn the_top_center_origin_keeps_the_attached_menu_edge() {
        // An attached menu hangs from the bar. Its accepted paint extent grows
        // downward, so the whole group must shrink toward the top edge: the
        // card stays attached while the shadow and the submenu chain leave
        // with it.
        let paint = rect(100, 28, 300, 360);
        let motion = PopupMotion::close(paint, PopupMotionOrigin::TopCenter, 0, false);
        let mid = motion.frame(token_ms(PopupMotionKind::Close) / 2);
        assert_eq!(
            mid.rect.loc.y, paint.loc.y,
            "the attached top edge stays under the bar"
        );
        assert!(mid.rect.loc.x > paint.loc.x);
        assert!(mid.rect.loc.x + mid.rect.size.w < paint.loc.x + paint.size.w);
        assert_eq!(mid.origin, PopupMotionOrigin::TopCenter);
        // A material card in the chain follows the same anchor, so its top
        // edge cannot detach from the accepted card while the group shrinks.
        let card = rect(104, 32, 120, 200);
        let scaled = scale_rect_about_origin(card, paint, mid.scale, mid.origin);
        assert_eq!(
            scaled.loc.y,
            paint.loc.y + ((card.loc.y - paint.loc.y) as f64 * mid.scale).round() as i32
        );
        assert!(scaled.size.w < card.size.w);
    }

    #[test]
    fn the_menu_bar_popup_is_piloted_and_the_dock_keeps_its_anchor() {
        assert!(namespace_piloted("menubar-popup"));
        assert!(namespace_piloted("dock-popup"));
        assert!(!namespace_piloted("menubar"));
        assert_eq!(
            PopupMotionOrigin::for_namespace("menubar-popup"),
            PopupMotionOrigin::TopCenter
        );
        assert_eq!(
            PopupMotionOrigin::for_namespace("dock-popup"),
            PopupMotionOrigin::BottomCenter
        );
    }

    #[test]
    fn the_attached_menu_uses_the_measured_menu_tokens() {
        assert_eq!(
            PopupMotionKind::Close
                .token_for("menubar-popup")
                .duration_ms,
            motion::MENU_CLOSE.duration_ms
        );
        assert_eq!(
            PopupMotionKind::Open.token_for("menubar-popup").duration_ms,
            motion::MENU_OPEN.duration_ms
        );
        assert_eq!(
            PopupMotionKind::Close.token_for("dock-popup").duration_ms,
            motion::POPUP_CLOSE.duration_ms
        );
        let target = rect(0, 20, 240, 320);
        let menu = PopupMotion::close_for(
            target,
            PopupMotionOrigin::TopCenter,
            PopupMotionKind::Close.token_for("menubar-popup"),
            0,
            false,
        );
        assert!(!menu.is_done(u64::from(motion::MENU_CLOSE.duration_ms) - 1));
        assert!(menu.is_done(u64::from(motion::MENU_CLOSE.duration_ms)));
    }

    #[test]
    fn a_zero_duration_tween_is_immediately_complete() {
        let target = rect(0, 0, 64, 64);
        let motion = PopupMotion {
            tween: Tween::new(100, 0, motion::POPUP_CLOSE.curve),
            ..PopupMotion::close(target, PopupMotionOrigin::BottomCenter, 100, false)
        };
        assert!(motion.is_done(100));
        assert!(motion.draws_nothing(100));
    }

    // --- deterministic pixel fixtures ------------------------------------

    /// A tiny premultiplied RGBA raster used by the pixel fixtures. It is
    /// deliberately renderer-free: the same math the compositor applies to one
    /// contribution is checked against a known underlay.
    struct Raster {
        w: usize,
        h: usize,
        pixels: Vec<Premultiplied>,
    }

    impl Raster {
        fn new(w: usize, h: usize, fill: Premultiplied) -> Self {
            Raster {
                w,
                h,
                pixels: vec![fill; w * h],
            }
        }

        fn set(&mut self, x: usize, y: usize, color: Premultiplied) {
            if x < self.w && y < self.h {
                self.pixels[y * self.w + x] = color;
            }
        }

        fn get(&self, x: usize, y: usize) -> Premultiplied {
            self.pixels[y * self.w + x]
        }

        /// Write the raster as an RGBA PNG (straight alpha) when a capture
        /// directory is configured. The test itself never depends on the
        /// file; it is local evidence for the hand-off.
        fn save(&self, path: &std::path::Path) {
            let mut image = image::RgbaImage::new(self.w as u32, self.h as u32);
            for (index, pixel) in self.pixels.iter().enumerate() {
                let a = pixel[3];
                let straight = if a > 0.0 {
                    [
                        (pixel[0] / a * 255.0).round().clamp(0.0, 255.0) as u8,
                        (pixel[1] / a * 255.0).round().clamp(0.0, 255.0) as u8,
                        (pixel[2] / a * 255.0).round().clamp(0.0, 255.0) as u8,
                        (a * 255.0).round().clamp(0.0, 255.0) as u8,
                    ]
                } else {
                    [0, 0, 0, 0]
                };
                image.put_pixel(
                    (index % self.w) as u32,
                    (index / self.w) as u32,
                    image::Rgba(straight),
                );
            }
            let _ = image.save(path);
        }
    }

    fn checker(w: usize, h: usize) -> Raster {
        let mut raster = Raster::new(w, h, [0.0, 0.0, 0.0, 1.0]);
        for y in 0..h {
            for x in 0..w {
                let on = ((x / 8) + (y / 8)) % 2 == 0;
                let value = if on { 0.85 } else { 0.15 };
                raster.set(x, y, [value, value, value, 1.0]);
            }
        }
        raster
    }

    /// A premultiplied rounded card with a bottom arrow and a soft shadow
    /// ring. The exact shape is not the production chooser; it is a known
    /// fixture whose silhouette, arrow reach and shadow reach are observable.
    fn chooser_group(w: usize, h: usize, tint: [f32; 3], alpha: f32) -> Raster {
        let mut raster = Raster::new(w, h, [0.0; 4]);
        let card_h = h * 3 / 4;
        let radius = 10usize;
        for y in 0..card_h {
            for x in 0..w {
                let inside_x = x >= 2 && x + 2 < w;
                let inside_y = y >= 2 && y + 2 < card_h;
                if !inside_x || !inside_y {
                    continue;
                }
                let corner = |cx: usize, cy: usize| {
                    let dx = x as i32 - cx as i32;
                    let dy = y as i32 - cy as i32;
                    (dx * dx + dy * dy) as f64 <= (radius * radius) as f64
                };
                let in_corner =
                    (x < 2 + radius && y < 2 + radius && !corner(2 + radius, 2 + radius))
                        || (x + 2 + radius >= w
                            && y < 2 + radius
                            && !corner(w - 2 - radius - 1, 2 + radius))
                        || (x < 2 + radius
                            && y + 2 + radius >= card_h
                            && !corner(2 + radius, card_h - 2 - radius - 1))
                        || (x + 2 + radius >= w
                            && y + 2 + radius >= card_h
                            && !corner(w - 2 - radius - 1, card_h - 2 - radius - 1));
                if in_corner {
                    continue;
                }
                raster.set(
                    x,
                    y,
                    [tint[0] * alpha, tint[1] * alpha, tint[2] * alpha, alpha],
                );
            }
        }
        // A bottom-centre arrow, three rows tall and one quarter wide.
        let arrow_w = (w / 4).max(4);
        let cx = w / 2;
        for row in 0..(h - card_h) {
            let half = (arrow_w / 2).saturating_sub(row);
            for x in cx.saturating_sub(half)..=(cx + half).min(w - 1) {
                raster.set(
                    x,
                    card_h + row,
                    [tint[0] * alpha, tint[1] * alpha, tint[2] * alpha, alpha],
                );
            }
        }
        // A faint shadow ring outside the card (premultiplied).
        for y in 0..h {
            for x in 0..w {
                if raster.get(x, y)[3] > 0.0 {
                    continue;
                }
                let near_card = x >= 1 && x + 1 < w && y >= 1 && y < card_h + 1;
                if near_card {
                    raster.set(x, y, [0.0, 0.0, 0.0, 0.12]);
                }
            }
        }
        raster
    }

    fn material(w: usize, h: usize, underlay: &Raster) -> Raster {
        // A blurred, tinted copy of the underlay, as the sampled backdrop
        // contributes it inside the group.
        let mut raster = Raster::new(w, h, [0.0; 4]);
        let card_h = h * 3 / 4;
        for y in 0..card_h {
            for x in 0..w {
                let sample = underlay.get((x + 1).min(w - 1), (y + 1).min(h - 1));
                let tint = 0.72;
                let alpha = 0.62;
                let color = [
                    (sample[0] * 0.4 + tint * 0.6) * alpha,
                    (sample[1] * 0.4 + tint * 0.6) * alpha,
                    (sample[2] * 0.4 + tint * 0.6) * alpha,
                    alpha,
                ];
                raster.set(x, y, color);
            }
        }
        raster
    }

    fn composite(
        content: &Raster,
        material: &Raster,
        underlay: &Raster,
        alpha: f32,
        exact_group: bool,
    ) -> Raster {
        let mut out = Raster::new(underlay.w, underlay.h, [0.0; 4]);
        for y in 0..underlay.h {
            for x in 0..underlay.w {
                let u = underlay.get(x, y);
                let c = content.get(x, y);
                let m = material.get(x, y);
                let pixel = if exact_group {
                    group_over_underlay(c, m, u, alpha)
                } else {
                    independent_over_underlay(c, m, u, alpha)
                };
                out.set(x, y, pixel);
            }
        }
        out
    }

    fn fixture_dir() -> Option<std::path::PathBuf> {
        std::env::var_os("DF_POPUP_MOTION_CAPTURE_DIR").map(std::path::PathBuf::from)
    }

    #[test]
    fn group_opacity_returns_the_underlay_at_near_zero_alpha() {
        let underlay = checker(48, 64);
        let content = chooser_group(48, 64, [0.2, 0.3, 0.4], 0.9);
        let material = material(48, 64, &underlay);
        // Zero alpha is the limit: the group contributes nothing and the
        // underlay is returned exactly.
        let gone = composite(&content, &material, &underlay, 0.0, true);
        for y in 0..underlay.h {
            for x in 0..underlay.w {
                let u = underlay.get(x, y);
                let e = gone.get(x, y);
                assert!((e[0] - u[0]).abs() < 1e-6 && (e[3] - u[3]).abs() < 1e-6);
            }
        }
        // A 5 % group alpha is still visually "nearly gone"; the independent
        // composition leaves a measurable film of the material behind.
        let alpha = 0.05;
        let exact = composite(&content, &material, &underlay, alpha, true);
        let naive = composite(&content, &material, &underlay, alpha, false);
        let mut exact_max = 0.0f32;
        let mut naive_max = 0.0f32;
        let mut naive_bad = 0;
        for y in 0..underlay.h {
            for x in 0..underlay.w {
                let u = underlay.get(x, y);
                let e = exact.get(x, y);
                let n = naive.get(x, y);
                exact_max = exact_max.max((e[0] - u[0]).abs()).max((e[3] - u[3]).abs());
                let error = (n[0] - u[0]).abs().max((n[3] - u[3]).abs());
                naive_max = naive_max.max(error);
                if error > 0.01 {
                    naive_bad += 1;
                }
            }
        }
        assert!(exact_max < 0.06, "the group is within its 5 % alpha");
        assert!(
            naive_max > exact_max * 1.2,
            "the independent composition must be observably worse"
        );
        assert!(
            naive_bad > 0,
            "the independent composition leaves a film behind"
        );
        if let Some(dir) = fixture_dir() {
            let _ = std::fs::create_dir_all(&dir);
            exact.save(&dir.join("t355-group-near-zero.png"));
            naive.save(&dir.join("t355-independent-near-zero.png"));
        }
    }

    #[test]
    fn group_opacity_matches_the_reference_at_each_phase() {
        let underlay = checker(64, 80);
        let content = chooser_group(64, 80, [0.2, 0.3, 0.4], 0.9);
        let material = material(64, 80, &underlay);
        let motion = PopupMotion::close(
            rect(0, 0, 64, 80),
            PopupMotionOrigin::BottomCenter,
            0,
            false,
        );
        let end_ms = token_ms(PopupMotionKind::Close);
        let phases = [
            ("settled", 0u64),
            ("near-start", 1),
            ("midpoint", end_ms / 2),
            ("end", end_ms),
        ];
        let mut diffs = Vec::new();
        for (name, at) in phases {
            let frame = motion.frame(at);
            let exact = composite(&content, &material, &underlay, frame.alpha, true);
            let naive = composite(&content, &material, &underlay, frame.alpha, false);
            // The reference group is the definition; compare the naive
            // composition against it and record the phase's error.
            let mut max = 0.0f32;
            for y in 0..underlay.h {
                for x in 0..underlay.w {
                    let e = exact.get(x, y);
                    let n = naive.get(x, y);
                    max = max.max((e[0] - n[0]).abs()).max((e[3] - n[3]).abs());
                }
            }
            diffs.push((name, frame.alpha, max));
            if let Some(dir) = fixture_dir() {
                let _ = std::fs::create_dir_all(&dir);
                exact.save(&dir.join(format!("t355-group-{name}.png")));
            }
        }
        // Settled and end are the same composition; the midpoint is where the
        // relative contribution changes.
        let settled = diffs.iter().find(|(n, _, _)| *n == "settled").unwrap().2;
        let midpoint = diffs.iter().find(|(n, _, _)| *n == "midpoint").unwrap().2;
        let end = diffs.iter().find(|(n, _, _)| *n == "end").unwrap().2;
        assert!(settled < 1e-6);
        assert!(end < 1e-6);
        assert!(
            midpoint > 0.02,
            "the independent composition changes the relative contribution at the midpoint: {midpoint}"
        );
    }

    #[test]
    fn the_group_fixture_covers_the_full_arrow_and_shadow_reach() {
        let underlay = checker(64, 80);
        let content = chooser_group(64, 80, [0.2, 0.3, 0.4], 0.9);
        let material = material(64, 80, &underlay);
        let settled = composite(&content, &material, &underlay, 1.0, true);
        // The arrow lives below the card body and the shadow ring outside it.
        let card_h = 80 * 3 / 4;
        assert!(settled.get(32, card_h + 1)[3] > 0.0, "the arrow paints");
        assert!(settled.get(1, 1)[3] > 0.0, "the shadow ring paints");
        // At the end the group contributes nothing; the underlay is exact.
        let end = composite(&content, &material, &underlay, 0.0, true);
        for y in 0..underlay.h {
            for x in 0..underlay.w {
                let u = underlay.get(x, y);
                let e = end.get(x, y);
                assert!((e[0] - u[0]).abs() < 1e-6 && (e[3] - u[3]).abs() < 1e-6);
            }
        }
    }
}
