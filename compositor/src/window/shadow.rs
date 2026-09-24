// SPDX-License-Identifier: MIT
//! Elevation-token-driven window shadows (T-04.1a).
//!
//! The compositor draws a soft drop shadow behind every decorated window from
//! the **shared** elevation tokens in `design_tokens.rs`, so a
//! compositor-drawn SSD window and a first-party QML `AppWindow`/`Shadow`
//! cannot drift (FR-2). The same formula the design system's `Shadow.qml`
//! approximation uses is reproduced here in logical pixels:
//!
//! * the shadow is `layer` translucent rectangles of `shadow color × opacity`
//!   stacked with a growing spread, which reads as a soft edge without a
//!   shader or a blur pass;
//! * `blur`, `offset_y`, and `layers` come from
//!   `component::elevation::{low,med,high,overlay}` (which reference
//!   `primitive.elevation.*`); `opacity` and `color` come from the active
//!   scheme's `material.shadowOpacity` / `color.shadowColor`.
//!
//! Rounded corners and a real backdrop blur are T-04.1b/T-04.2; this slice
//! deliberately stays a layered-rectangle approximation so the same geometry
//! is testable headless, and it is transformed with the window's lifecycle
//! [`MotionFrame`](crate::window::MotionFrame) so an appearing/minimizing
//! window's shadow shrinks with it.

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::Id;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Color32F;
use smithay::utils::{Logical, Point, Rectangle, Scale};

use crate::design_tokens::{component, semantic};
use crate::window::corner::{rounded_rect_spans, RoundedCorners};
use crate::window::decoration::{color_from_rgba, motion_transform, ColorScheme};
use crate::window::motion::MotionFrame;

/// The design-system elevation level a shadow is drawn at. The levels map
/// 1:1 onto the `component.elevation` token group; a surface's role decides
/// its level (a floating window is [`ShadowLevel::High`], popovers/overlays
/// are [`ShadowLevel::Overlay`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShadowLevel {
    Low,
    Med,
    #[default]
    High,
    Overlay,
}

impl ShadowLevel {
    pub const fn name(self) -> &'static str {
        match self {
            ShadowLevel::Low => "low",
            ShadowLevel::Med => "med",
            ShadowLevel::High => "high",
            ShadowLevel::Overlay => "overlay",
        }
    }

    /// The shadow geometry for this level under `scheme`, resolved from the
    /// generated tokens.
    pub fn spec(self, scheme: ColorScheme) -> ShadowSpec {
        let (blur, offset_y, layers) = match self {
            ShadowLevel::Low => (
                component::elevation::low::BLUR,
                component::elevation::low::OFFSET_Y,
                component::elevation::low::LAYERS,
            ),
            ShadowLevel::Med => (
                component::elevation::med::BLUR,
                component::elevation::med::OFFSET_Y,
                component::elevation::med::LAYERS,
            ),
            ShadowLevel::High => (
                component::elevation::high::BLUR,
                component::elevation::high::OFFSET_Y,
                component::elevation::high::LAYERS,
            ),
            ShadowLevel::Overlay => (
                component::elevation::overlay::BLUR,
                component::elevation::overlay::OFFSET_Y,
                component::elevation::overlay::LAYERS,
            ),
        };
        let (opacity, color) = match scheme {
            ColorScheme::Light => (
                semantic::light::material::SHADOW_OPACITY,
                semantic::light::color::SHADOW_COLOR,
            ),
            ColorScheme::Dark => (
                semantic::dark::material::SHADOW_OPACITY,
                semantic::dark::color::SHADOW_COLOR,
            ),
        };
        ShadowSpec {
            blur,
            offset_y,
            layers: layers.max(1.0) as u32,
            radius: component::window::RADIUS,
            opacity,
            color,
        }
    }
}

/// A resolved shadow: token-derived blur/offset/layer count plus the scheme's
/// shadow color and opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowSpec {
    /// The farthest spread of the softest layer, in logical pixels
    /// (`component.elevation.<level>.blur`, itself `primitive.elevation.*`).
    pub blur: f32,
    /// Downward offset, in logical pixels
    /// (`component.elevation.<level>.offsetY`).
    pub offset_y: f32,
    /// Number of stacked layers (`component.elevation.<level>.layers`).
    pub layers: u32,
    /// The window's corner radius (`component.window.radius`); each layer's
    /// rounded corner grows from this by its own blur spread, matching
    /// `Shadow.qml`'s `radius: root.radius + root.blur * spread` (T-04.1b).
    pub radius: f32,
    /// The scheme's `material.shadowOpacity`.
    pub opacity: f32,
    /// The scheme's `color.shadowColor`, with its token alpha.
    pub color: [u8; 4],
}

/// One translucent layer of a shadow. `rect` is in the same logical
/// coordinate space as the window it belongs to, and `radius` is the rounded
/// corner that follows the window radius (T-04.1b).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowLayer {
    pub rect: Rectangle<i32, Logical>,
    pub radius: f32,
    pub color: Color32F,
}

/// The layered geometry of `spec` for a window occupying `rect`
/// (the *whole* decorated window, titlebar included).
///
/// Layers are returned back-to-front: index `0` is the softest/farthest
/// spread and the last is the tightest against the window. This is the exact
/// formula of `design-system/components/Shadow.qml`, so the compositor and the
/// QML approximation produce the same shadow from the same tokens. Each layer
/// carries the rounded corner `window.radius + blur * spread`, so the shadow
/// corner follows the window it belongs to.
pub fn shadow_layers(rect: Rectangle<i32, Logical>, spec: ShadowSpec) -> Vec<ShadowLayer> {
    let layers = spec.layers.max(1);
    // Each layer carries `shadowOpacity / layers`; stacked, the centre is the
    // full shadow opacity and the outer fringe is a single faint layer.
    let per_layer = spec.opacity / layers as f32;
    let base = color_from_rgba(spec.color);
    let mut result = Vec::with_capacity(layers as usize);
    for index in 0..layers {
        // index 0 → spread 1.0 (widest), last → 1/layers (tightest).
        let spread = f64::from(layers - index) / f64::from(layers);
        let blur = f64::from(spec.blur) * spread;
        let x = (f64::from(rect.loc.x) - blur).round() as i32;
        let y = (f64::from(rect.loc.y) + f64::from(spec.offset_y) - blur).round() as i32;
        let w = (f64::from(rect.size.w) + 2.0 * blur).round() as i32;
        let h = (f64::from(rect.size.h) + 2.0 * blur).round() as i32;
        result.push(ShadowLayer {
            rect: Rectangle::new((x, y).into(), (w.max(1), h.max(1)).into()),
            radius: spec.radius + blur as f32,
            color: base * per_layer,
        });
    }
    result
}

/// The bounding rectangle of a shadow: the softest layer's rect, or the
/// window rect when the level has no spread.
pub fn shadow_bounds(rect: Rectangle<i32, Logical>, spec: ShadowSpec) -> Rectangle<i32, Logical> {
    shadow_layers(rect, spec)
        .into_iter()
        .map(|layer| layer.rect)
        .reduce(|a, b| a.merge(b))
        .unwrap_or(rect)
}

/// Convert the shadow layers into solid render elements for one window, in
/// output-local physical coordinates and front-to-back order (the tightest
/// layer first), optionally carrying the window's lifecycle motion.
///
/// Each layer is decomposed into the token-derived rounded-corner spans
/// (T-04.1b) so the shadow's corner follows the window, matching
/// `Shadow.qml`'s per-layer `radius`. Spans never overlap, so the translucent
/// layers stack exactly as before.
pub fn shadow_elements(
    rect: Rectangle<i32, Logical>,
    spec: ShadowSpec,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
    motion: Option<MotionFrame>,
) -> Vec<SolidColorRenderElement> {
    let mut elements = Vec::new();
    for layer in shadow_layers(rect, spec) {
        let radius = layer.radius.round() as i32;
        for span in rounded_rect_spans(layer.rect, radius, RoundedCorners::All) {
            let (span_rect, color) = motion_transform(span, layer.color, rect, motion);
            let local = Rectangle::new(
                (
                    span_rect.loc.x - output_origin.x,
                    span_rect.loc.y - output_origin.y,
                )
                    .into(),
                span_rect.size,
            );
            elements.push(SolidColorRenderElement::new(
                Id::new(),
                local.to_physical_precise_round(scale),
                CommitCounter::default(),
                color,
                Kind::Unspecified,
            ));
        }
    }
    // `shadow_layers` is back-to-front; Smithay consumes front-to-back.
    elements.reverse();
    elements
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::renderer::element::Element;
    use smithay::utils::{Physical, Size};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
    }

    /// The bounding rectangle of a shadow element list at `scale`.
    fn merged_bounds(
        elements: &[SolidColorRenderElement],
        scale: Scale<f64>,
    ) -> Rectangle<i32, Physical> {
        elements
            .iter()
            .map(|element| element.geometry(scale))
            .reduce(|a, b| a.merge(b))
            .expect("at least one element")
    }

    #[test]
    fn every_level_reads_its_component_elevation_tokens() {
        let dark = ShadowLevel::High.spec(ColorScheme::Dark);
        assert_eq!(dark.blur, component::elevation::high::BLUR);
        assert_eq!(dark.blur, 40.0);
        assert_eq!(dark.offset_y, component::elevation::high::OFFSET_Y);
        assert_eq!(dark.offset_y, 5.0);
        assert_eq!(dark.layers, component::elevation::high::LAYERS as u32);
        assert_eq!(dark.layers, 8);
        assert_eq!(dark.radius, component::window::RADIUS);
        assert_eq!(dark.radius, 14.0);
        assert_eq!(dark.opacity, semantic::dark::material::SHADOW_OPACITY);
        assert_eq!(dark.color, semantic::dark::color::SHADOW_COLOR);

        // The lighter levels scale from the primitive elevation scale.
        assert_eq!(ShadowLevel::Low.spec(ColorScheme::Dark).blur, 8.0);
        assert_eq!(ShadowLevel::Med.spec(ColorScheme::Dark).blur, 20.0);
        assert_eq!(ShadowLevel::Overlay.spec(ColorScheme::Dark).blur, 64.0);
        assert_eq!(ShadowLevel::default(), ShadowLevel::High);
    }

    #[test]
    fn light_and_dark_shadows_use_the_scheme_opacity_and_color() {
        let light = ShadowLevel::High.spec(ColorScheme::Light);
        let dark = ShadowLevel::High.spec(ColorScheme::Dark);
        assert_eq!(light.opacity, semantic::light::material::SHADOW_OPACITY);
        assert_ne!(light.opacity, dark.opacity, "dark shadows read stronger");
        assert_eq!(light.color, semantic::light::color::SHADOW_COLOR);
        assert_eq!(dark.color, semantic::dark::color::SHADOW_COLOR);
    }

    #[test]
    fn layers_are_token_derived_and_ordered_widest_to_tightest() {
        let window = rect(100, 200, 400, 300);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let layers = shadow_layers(window, spec);
        assert_eq!(layers.len(), spec.layers as usize);

        // The first (softest) layer spreads the full blur and is offset down.
        let first = layers[0].rect;
        assert_eq!(first.loc.x, window.loc.x - spec.blur as i32);
        assert_eq!(
            first.loc.y,
            window.loc.y + spec.offset_y as i32 - spec.blur as i32
        );
        assert_eq!(first.size.w, window.size.w + 2 * spec.blur as i32);
        assert_eq!(first.size.h, window.size.h + 2 * spec.blur as i32);

        // Every later layer is contained by the previous one, and the last is
        // the tightest against the window.
        for pair in layers.windows(2) {
            assert!(pair[0].rect.contains_rect(pair[1].rect));
        }
        let last = layers.last().unwrap().rect;
        assert!(last.size.w < first.size.w && last.size.h < first.size.h);
        assert!(last.contains_rect(window));

        // The rounded corner follows the window radius plus each layer's blur
        // spread (matching `Shadow.qml`), so the softest layer is roundest.
        assert_eq!(layers[0].radius, spec.radius + spec.blur);
        assert_eq!(
            layers.last().unwrap().radius,
            spec.radius + spec.blur / spec.layers as f32
        );
        for pair in layers.windows(2) {
            assert!(
                pair[0].radius >= pair[1].radius,
                "the outer layer must be rounder"
            );
        }

        // Each layer is one `shadowOpacity / layers` slice of the color.
        let expected = color_from_rgba(spec.color) * (spec.opacity / spec.layers as f32);
        assert_eq!(layers[0].color, expected);

        // The bounds are the softest layer.
        assert_eq!(shadow_bounds(window, spec), first);
    }

    #[test]
    fn zero_layers_clamp_to_one_never_panicking() {
        let window = rect(0, 0, 100, 100);
        let spec = ShadowSpec {
            blur: 40.0,
            offset_y: 5.0,
            layers: 0,
            radius: component::window::RADIUS,
            opacity: 0.45,
            color: [0, 0, 0, 255],
        };
        // A zero-layer spec clamps to a single layer rather than indexing
        // nothing; the bounds are still the window.
        assert_eq!(shadow_layers(window, spec).len(), 1);
        assert_eq!(
            shadow_bounds(window, spec),
            shadow_layers(window, spec)[0].rect
        );
    }

    #[test]
    fn shadow_elements_span_the_rounded_bounds_front_to_back() {
        let window = rect(100, 200, 400, 300);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let elements = shadow_elements(window, spec, 1.0.into(), Point::from((0, 0)), None);
        // The rounded decomposition emits one span per corner row plus the
        // central band, so a shadow is more than one element per layer.
        assert!(elements.len() > spec.layers as usize);

        // The union of every span is exactly the shadow bounds.
        let union = merged_bounds(&elements, 1.0.into());
        assert_eq!(
            union,
            shadow_bounds(window, spec).to_physical_precise_round(1.0)
        );

        // The front-to-back list starts with the tightest layer and ends with
        // the softest: the first span is narrower than the widest span.
        let first = elements.first().unwrap().geometry(1.0.into());
        let widest = elements
            .iter()
            .map(|element| element.geometry(1.0.into()).size.w)
            .max()
            .unwrap();
        assert!(first.size.w < widest);

        // The corners are clipped: no element covers the softest layer's
        // outermost pixel (only the arc touches it).
        assert!(!elements
            .iter()
            .any(|element| element.geometry(1.0.into()).contains(union.loc)));
    }

    #[test]
    fn shadow_elements_are_offset_by_the_output_origin_and_scaled() {
        let window = rect(100, 200, 400, 300);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let local = shadow_elements(window, spec, 1.0.into(), Point::from((0, 0)), None);
        let shifted = shadow_elements(window, spec, 1.0.into(), Point::from((100, 200)), None);
        // The whole shadow moves by exactly the output origin.
        let local_bounds = merged_bounds(&local, 1.0.into());
        let shifted_bounds = merged_bounds(&shifted, 1.0.into());
        assert_eq!(local_bounds.loc.x - shifted_bounds.loc.x, 100);
        assert_eq!(local_bounds.loc.y - shifted_bounds.loc.y, 200);
        assert_eq!(local_bounds.size, shifted_bounds.size);

        // At scale 2 the physical geometry is exactly double.
        let doubled = shadow_elements(window, spec, 2.0.into(), Point::from((0, 0)), None);
        let doubled_bounds = merged_bounds(&doubled, 2.0.into());
        assert_eq!(doubled_bounds.size, local_bounds.size * 2);
    }

    #[test]
    fn shadow_carries_the_lifecycle_motion_transform() {
        use crate::window::motion::{WindowMotion, WindowMotionKind};
        let window = rect(100, 200, 400, 300);
        let tile = rect(10, 10, 48, 48);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let motion = WindowMotion::new(WindowMotionKind::Minimize, tile, window, 0, false);

        let begin = shadow_elements(
            window,
            spec,
            1.0.into(),
            Point::from((0, 0)),
            Some(motion.frame(0)),
        );
        let start = merged_bounds(&begin, 1.0.into());

        // At the end of the minimize the shadow has shrunk to the tile (the
        // rounded span union is much smaller than the full window shadow).
        let end_ms = crate::design_tokens::motion::WINDOW_CLOSE.duration_ms as u64;
        let end = shadow_elements(
            window,
            spec,
            1.0.into(),
            Point::from((0, 0)),
            Some(motion.frame(end_ms)),
        );
        let end_bounds = merged_bounds(&end, 1.0.into());
        assert!(
            end_bounds.size.w < start.size.w,
            "the shadow shrinks with the window"
        );
    }
}
