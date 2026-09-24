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
    /// The scheme's `material.shadowOpacity`.
    pub opacity: f32,
    /// The scheme's `color.shadowColor`, with its token alpha.
    pub color: [u8; 4],
}

/// One translucent rectangle of a shadow. `rect` is in the same logical
/// coordinate space as the window it belongs to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowLayer {
    pub rect: Rectangle<i32, Logical>,
    pub color: Color32F,
}

/// The layered geometry of `spec` for a window occupying `rect`
/// (the *whole* decorated window, titlebar included).
///
/// Layers are returned back-to-front: index `0` is the softest/farthest
/// spread and the last is the tightest against the window. This is the exact
/// formula of `design-system/components/Shadow.qml`, so the compositor and the
/// QML approximation produce the same shadow from the same tokens.
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
pub fn shadow_elements(
    rect: Rectangle<i32, Logical>,
    spec: ShadowSpec,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
    motion: Option<MotionFrame>,
) -> Vec<SolidColorRenderElement> {
    let mut elements = Vec::new();
    for layer in shadow_layers(rect, spec) {
        let (layer_rect, color) = motion_transform(layer.rect, layer.color, rect, motion);
        let local = Rectangle::new(
            (
                layer_rect.loc.x - output_origin.x,
                layer_rect.loc.y - output_origin.y,
            )
                .into(),
            layer_rect.size,
        );
        elements.push(SolidColorRenderElement::new(
            Id::new(),
            local.to_physical_precise_round(scale),
            CommitCounter::default(),
            color,
            Kind::Unspecified,
        ));
    }
    // `shadow_layers` is back-to-front; Smithay consumes front-to-back.
    elements.reverse();
    elements
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::renderer::element::Element;
    use smithay::utils::Size;

    fn rect(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new(Point::from((x, y)), Size::from((w, h)))
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
    fn shadow_elements_are_physical_front_to_back_with_the_tightest_first() {
        let window = rect(100, 200, 400, 300);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let elements = shadow_elements(window, spec, 1.0.into(), Point::from((0, 0)), None);
        assert_eq!(elements.len(), spec.layers as usize);

        // front-to-back: the tightest layer is first, the widest last.
        let first = elements.first().unwrap().geometry(1.0.into());
        let last = elements.last().unwrap().geometry(1.0.into());
        assert!(first.size.w < last.size.w);
        assert_eq!(last.loc.x, window.loc.x - spec.blur as i32);
        assert_eq!(last.size.w, window.size.w + 2 * spec.blur as i32);
    }

    #[test]
    fn shadow_elements_are_offset_by_the_output_origin_and_scaled() {
        let window = rect(100, 200, 400, 300);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let local = shadow_elements(window, spec, 1.0.into(), Point::from((0, 0)), None);
        let shifted = shadow_elements(window, spec, 1.0.into(), Point::from((100, 200)), None);
        // The widest (backmost) layer moves by exactly the output origin.
        let local_widest = local.last().unwrap().geometry(1.0.into());
        let shifted_widest = shifted.last().unwrap().geometry(1.0.into());
        assert_eq!(local_widest.loc.x - shifted_widest.loc.x, 100);
        assert_eq!(local_widest.loc.y - shifted_widest.loc.y, 200);

        // At scale 2 the physical geometry is exactly double.
        let doubled = shadow_elements(window, spec, 2.0.into(), Point::from((0, 0)), None);
        let doubled_widest = doubled.last().unwrap().geometry(2.0.into());
        let doubled_logical = Rectangle::from_size(doubled_widest.size / 2);
        assert_eq!(doubled_logical.size, local_widest.size);
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
        let start = begin.last().unwrap().geometry(1.0.into());
        assert_eq!(start.size.w, window.size.w + 2 * spec.blur as i32);

        // At the end of the minimize the shadow has shrunk to the tile and
        // faded to nothing.
        let end_ms = crate::design_tokens::motion::WINDOW_CLOSE.duration_ms as u64;
        let end = shadow_elements(
            window,
            spec,
            1.0.into(),
            Point::from((0, 0)),
            Some(motion.frame(end_ms)),
        );
        let end_last = end.last().unwrap().geometry(1.0.into());
        assert!(
            end_last.size.w < start.size.w,
            "the shadow shrinks with the window"
        );
    }
}
