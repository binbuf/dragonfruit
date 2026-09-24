// SPDX-License-Identifier: MIT
//! The reusable scene-transform pass (T-04.3).
//!
//! T-05's Mission Control live-surface transform and T-06's app switcher both
//! need to draw the *real* surfaces scaled, translated, clipped to their
//! rounded corners, and (optionally) over a blurred backdrop — never
//! thumbnails, never client re-renders ([02-compositor.md](../../docs/design/02-compositor.md)).
//! Building that pipeline per feature is explicitly forbidden, so this module
//! is the **one** transform, built once and composed by every later feature.
//!
//! A [`SceneTransform`] is pure geometry plus two optional token-derived
//! attachments:
//!
//! * **scale/translate** — it maps a `source` rectangle onto a `target`
//!   rectangle. [`SceneTransform::from_motion`] derives it from a window's
//!   lifecycle [`MotionFrame`] so the T-02 appear/minimize/zoom motions and
//!   the T-05 grid share one mapping.
//! * **clip** — an optional [`CornerMask`], the same token-derived rounded
//!   shape the shadow/titlebar/backdrop use, so a transformed live surface
//!   rounds from one source (ADR 0012).
//! * **blur** — an optional [`BackdropSpec`], the token-driven frosted panel
//!   the T-04.2 chrome backdrop already builds; a transformed scene can draw
//!   that same material underneath itself.
//!
//! [`SceneTransformPass`] is the per-frame bookkeeping on top of the shared
//! [`FramePass`](crate::window::pass::FramePass): the renderer opens a frame
//! once and each output composes at most one scene transform, so the scene is
//! never transformed twice in a frame (the T-04.3 "one effect pass per frame"
//! invariant, instrumented as `scene_transform_passes` /
//! `scene_transform_skipped`).
//!
//! The model is deliberately renderer-free: the transform state is asserted
//! headless without a GPU, and the render layer is the only place that turns
//! it into Smithay elements.

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::utils::{Logical, Point, Rectangle, Scale, Size};

use crate::window::backdrop::{backdrop_elements, backdrop_layers, BackdropLayer, BackdropSpec};
use crate::window::corner::CornerMask;
use crate::window::motion::MotionFrame;
use crate::window::pass::FramePass;

/// A reusable scale/translate of a source rectangle onto a target rectangle,
/// with optional rounded-corner clip and optional backdrop blur.
///
/// All fields are in the same logical coordinate space (the scene/space
/// space); the render layer converts to physical only at element generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneTransform {
    /// The rectangle in source space that maps onto [`Self::target`].
    pub source: Rectangle<i32, Logical>,
    /// The destination rectangle [`Self::source`] maps onto.
    pub target: Rectangle<i32, Logical>,
    /// Optional token-derived rounded-corner clip for the transformed surface
    /// (ADR 0012). `None` leaves the surface square.
    pub clip: Option<CornerMask>,
    /// Optional token-derived frosted backdrop drawn under the transformed
    /// surface. `None` draws no material.
    pub blur: Option<BackdropSpec>,
}

impl SceneTransform {
    /// The identity transform for `rect`: no scale, no translate, no clip, no
    /// blur. A no-op window is drawn with this so the transform path and the
    /// plain path cannot diverge.
    pub fn identity(rect: Rectangle<i32, Logical>) -> Self {
        SceneTransform {
            source: rect,
            target: rect,
            clip: None,
            blur: None,
        }
    }

    /// A transform mapping `source` onto `target`.
    pub fn new(source: Rectangle<i32, Logical>, target: Rectangle<i32, Logical>) -> Self {
        SceneTransform {
            source,
            target,
            clip: None,
            blur: None,
        }
    }

    /// The transform a window's lifecycle motion draws a surface of
    /// `surface_size` with.
    ///
    /// The source is anchored at the motion's *target* location, so the scale
    /// is [`MotionFrame::scale_for`] (the interpolated size over the committed
    /// surface size) rather than the target-relative chrome scale: a zoom that
    /// resizes the client mid-flight still maps each committed buffer onto the
    /// interpolated rect. This is the one mapping the lifecycle motion and the
    /// T-05 grid share.
    pub fn from_motion(frame: MotionFrame, surface_size: Size<i32, Logical>) -> Self {
        let size = Size::from((surface_size.w.max(1), surface_size.h.max(1)));
        // The motion's target location: the interpolated rect minus the
        // translate the frame carries from the target.
        let source_loc = frame.rect.loc - frame.offset;
        SceneTransform::new(Rectangle::new(source_loc, size), frame.rect)
    }

    /// Attach a token-derived rounded-corner clip.
    pub fn with_clip(mut self, mask: CornerMask) -> Self {
        self.clip = Some(mask);
        self
    }

    /// Attach a token-derived frosted backdrop.
    pub fn with_blur(mut self, spec: BackdropSpec) -> Self {
        self.blur = Some(spec);
        self
    }

    /// The per-axis scale mapping source size onto target size.
    pub fn scale(&self) -> Scale<f64> {
        Scale::from((
            f64::from(self.target.size.w) / f64::from(self.source.size.w.max(1)),
            f64::from(self.target.size.h) / f64::from(self.source.size.h.max(1)),
        ))
    }

    /// The translation from the source origin to the target origin, in logical
    /// pixels.
    pub fn offset(&self) -> Point<i32, Logical> {
        (
            self.target.loc.x - self.source.loc.x,
            self.target.loc.y - self.source.loc.y,
        )
            .into()
    }

    /// Map a point from source space into target space.
    pub fn map_point(&self, point: Point<i32, Logical>) -> Point<i32, Logical> {
        let scale = self.scale();
        let x = f64::from(point.x - self.source.loc.x) * scale.x;
        let y = f64::from(point.y - self.source.loc.y) * scale.y;
        (
            self.target.loc.x + x.round() as i32,
            self.target.loc.y + y.round() as i32,
        )
            .into()
    }

    /// Map a rectangle from source space into target space, preserving its
    /// top-left anchor and scaling its size (never collapsing to zero).
    pub fn map_rect(&self, rect: Rectangle<i32, Logical>) -> Rectangle<i32, Logical> {
        let scale = self.scale();
        let loc = self.map_point(rect.loc);
        let size = Size::from((
            (f64::from(rect.size.w) * scale.x).round().max(1.0) as i32,
            (f64::from(rect.size.h) * scale.y).round().max(1.0) as i32,
        ));
        Rectangle::new(loc, size)
    }

    /// The non-overlapping horizontal spans that clip `rect` to this
    /// transform's rounded corners, or `[rect]` when no clip is attached.
    /// Consumes the shared [`CornerMask`] decomposition (ADR 0012).
    pub fn clip_spans(&self, rect: Rectangle<i32, Logical>) -> Vec<Rectangle<i32, Logical>> {
        match self.clip {
            Some(mask) => mask.spans(rect),
            None => vec![rect],
        }
    }

    /// The frosted feather layers under `rect` when a blur is attached, in the
    /// same logical space; empty when there is no blur.
    pub fn blur_layers(&self, rect: Rectangle<i32, Logical>) -> Vec<BackdropLayer> {
        self.blur
            .map(|spec| backdrop_layers(rect, spec))
            .unwrap_or_default()
    }

    /// The blur as solid render elements at `scale`, or empty when no blur is
    /// attached.
    pub fn blur_elements(
        &self,
        rect: Rectangle<i32, Logical>,
        scale: Scale<f64>,
    ) -> Vec<SolidColorRenderElement> {
        self.blur
            .map(|spec| backdrop_elements(rect, spec, scale))
            .unwrap_or_default()
    }

    /// Whether this transform is a no-op (identity geometry and no clip/blur).
    pub fn is_identity(&self) -> bool {
        self.source == self.target && self.clip.is_none() && self.blur.is_none()
    }
}

/// Per-frame bookkeeping for the scene-transform pass (T-04.3): one transform
/// per output per frame, or it is counted as `skipped`.
///
/// The renderer calls [`SceneTransformPass::begin_frame`] once per rendered
/// frame (via [`DfState::begin_render_frame`](crate::state::DfState::begin_render_frame)),
/// then [`SceneTransformPass::apply`] once per output that composes a transform.
#[derive(Debug, Default)]
pub struct SceneTransformPass(FramePass);

impl SceneTransformPass {
    pub fn new() -> Self {
        SceneTransformPass(FramePass::new())
    }

    /// Open a frame. `frame` is the renderer's monotonic render serial.
    pub fn begin_frame(&mut self, frame: u64) {
        self.0.begin_frame(frame);
    }

    /// Apply the scene transform for `output` this frame. `region` is the
    /// transformed scene's bounds in output-local logical coordinates. Returns
    /// the region when it applies now, or `None` when the output already
    /// composed a transform this frame (the double-transform guard).
    pub fn apply(
        &mut self,
        output: &str,
        region: Rectangle<i32, Logical>,
    ) -> Option<Rectangle<i32, Logical>> {
        self.0.apply(output, region)
    }

    /// The frame serial this pass is open for.
    pub fn frame(&self) -> u64 {
        self.0.frame()
    }

    /// Number of output-frames that composed a scene transform.
    pub fn applications(&self) -> u64 {
        self.0.applications()
    }

    /// Number of duplicate transform requests skipped this run (must stay `0`
    /// in a correct render loop).
    pub fn skipped(&self) -> u64 {
        self.0.skipped()
    }

    /// The damage owned by this frame's transforms (output-local logical).
    pub fn damage(&self) -> &[Rectangle<i32, Logical>] {
        self.0.damage()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::design_tokens::{component, semantic};
    use crate::window::backdrop::MaterialRole;
    use crate::window::decoration::ColorScheme;
    use crate::window::motion::{WindowMotion, WindowMotionKind};
    use smithay::utils::Point;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    #[test]
    fn identity_maps_every_rect_onto_itself() {
        let window = rect(100, 200, 400, 300);
        let transform = SceneTransform::identity(window);
        assert!(transform.is_identity());
        assert_eq!(transform.scale(), Scale::from((1.0, 1.0)));
        assert_eq!(transform.offset(), Point::from((0, 0)));
        assert_eq!(transform.map_rect(window), window);
        assert_eq!(transform.map_point((250, 300).into()), (250, 300).into());
        // Any attachment makes it non-identity.
        assert!(!transform.with_clip(CornerMask::window()).is_identity());
    }

    #[test]
    fn scale_and_offset_follow_the_target() {
        let source = rect(0, 0, 200, 100);
        let target = rect(400, 50, 400, 200);
        let transform = SceneTransform::new(source, target);
        assert_eq!(transform.scale(), Scale::from((2.0, 2.0)));
        assert_eq!(transform.offset(), Point::from((400, 50)));
        // The source rect maps exactly onto the target.
        assert_eq!(transform.map_rect(source), target);
        // The source origin maps to the target origin; interior points scale.
        assert_eq!(transform.map_point(source.loc), target.loc);
        assert_eq!(transform.map_point((100, 50).into()), (600, 150).into());
    }

    #[test]
    fn non_uniform_scale_maps_the_source_onto_the_target() {
        let source = rect(10, 20, 320, 240);
        let target = rect(100, 100, 200, 100);
        let transform = SceneTransform::new(source, target);
        assert_eq!(transform.map_rect(source), target);
        // The scale is per-axis: 200/320 and 100/240.
        let scale = transform.scale();
        assert!((scale.x - 0.625).abs() < 1e-9);
        assert!((scale.y - 100.0 / 240.0).abs() < 1e-9);
    }

    #[test]
    fn from_motion_matches_the_lifecycle_frame() {
        let target = rect(200, 200, 800, 600);
        let tile = rect(10, 10, 48, 48);
        let motion = WindowMotion::new(WindowMotionKind::Minimize, tile, target, 0, false);
        let end_ms = crate::design_tokens::motion::WINDOW_CLOSE.duration_ms as u64;
        let frame = motion.frame(end_ms / 2);

        let transform = SceneTransform::from_motion(frame, target.size);
        // The scale is the motion's surface-relative scale, and the offset is
        // the motion's translate, so the render layer reproduces the
        // interpolated rect exactly.
        assert_eq!(transform.scale(), frame.scale_for(target.size));
        assert_eq!(transform.offset(), frame.offset);
        // The whole committed surface maps onto the interpolated rect.
        let source = rect(
            frame.rect.loc.x - frame.offset.x,
            frame.rect.loc.y - frame.offset.y,
            target.size.w,
            target.size.h,
        );
        assert_eq!(transform.map_rect(source), frame.rect);

        // At the end of an appear the transform is the identity.
        let appear = WindowMotion::appear(tile, target, 0, false);
        let end = appear.frame(crate::design_tokens::motion::WINDOW_OPEN.duration_ms as u64);
        let settled = SceneTransform::from_motion(end, target.size);
        assert!(settled.is_identity());
        assert_eq!(settled.map_rect(target), target);
    }

    #[test]
    fn clip_spans_are_the_shared_corner_mask() {
        let window = rect(100, 100, 400, 300);
        let transform = SceneTransform::identity(window).with_clip(CornerMask::window());
        let spans = transform.clip_spans(window);
        assert!(spans.len() > 1, "the corners are rounded");
        // Every span is inside the surface and the union excludes the corner.
        for span in &spans {
            assert!(window.contains_rect(*span));
        }
        let corner = window.loc;
        assert!(!spans.iter().any(|span| span.contains(corner)));

        // No clip degrades to the plain rectangle.
        assert_eq!(
            SceneTransform::identity(window).clip_spans(window),
            vec![window]
        );
    }

    #[test]
    fn blur_layers_come_from_the_material_tokens() {
        let region = rect(0, 0, 800, 600);
        let spec = MaterialRole::Popup.spec(ColorScheme::Dark);
        let transform = SceneTransform::identity(region).with_blur(spec);
        let layers = transform.blur_layers(region);
        let expected = ((semantic::dark::material::POPUP_BLUR / 6.0).round() as u32).clamp(1, 8);
        assert_eq!(layers.len(), expected as usize);
        assert_eq!(layers[0].rect, region);
        // The panel tone is the role's token color, and the radius is the
        // component token, not a literal.
        assert_eq!(spec.radius, component::popup::RADIUS);
        // No blur attaches nothing.
        assert!(SceneTransform::identity(region)
            .blur_layers(region)
            .is_empty());
        assert!(SceneTransform::identity(region)
            .blur_elements(region, 1.0.into())
            .is_empty());
        assert!(!transform.blur_elements(region, 1.0.into()).is_empty());
    }

    #[test]
    fn pass_applies_once_per_output_and_frame() {
        let region = rect(0, 0, 1920, 1080);
        let mut pass = SceneTransformPass::new();
        pass.begin_frame(1);
        assert_eq!(pass.apply("NESTED-1", region), Some(region));
        assert_eq!(pass.applications(), 1);
        assert_eq!(pass.skipped(), 0);
        assert_eq!(pass.damage(), &[region]);
        // A second compose for the same output this frame is skipped: the
        // scene is never transformed twice.
        assert_eq!(pass.apply("NESTED-1", region), None);
        assert_eq!(pass.skipped(), 1);
        // A new frame re-opens it.
        pass.begin_frame(2);
        assert_eq!(pass.frame(), 2);
        assert_eq!(pass.apply("NESTED-1", region), Some(region));
        assert_eq!(pass.applications(), 2);
    }
}
