// SPDX-License-Identifier: MIT
//! Elevation-token-driven window shadows (T-04.1a).
//!
//! The compositor draws a soft drop shadow behind every decorated window from
//! the **shared** elevation tokens in `design_tokens.rs`, so a
//! compositor-drawn SSD window and a first-party QML `AppWindow`/`Shadow`
//! cannot drift (FR-2). The same formula the design system's `Shadow.qml`
//! approximation uses is reproduced here in logical pixels:
//!
//! * rounded layers integrate a truncated Gaussian falloff with source-over
//!   alpha, producing a soft edge without equal-opacity terraces;
//! * `blur`, `offset_y`, and `layers` come from
//!   `component::elevation::{low,med,high,overlay}` (which reference
//!   `primitive.elevation.*`); `opacity` and `color` come from the active
//!   scheme's `material.shadowOpacity` / `color.shadowColor`.
//!
//! Geometry is cached and clipped outside the translucent window body. The
//! approximation is testable headless and follows the window's lifecycle
//! [`MotionFrame`](crate::window::MotionFrame) so an appearing/minimizing
//! window's shadow shrinks with it.

use smithay::backend::renderer::element::solid::SolidColorRenderElement;
use smithay::backend::renderer::element::Kind;
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Color32F;
use smithay::utils::{Logical, Physical, Point, Rectangle, Scale};
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

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
    // Integrate a truncated Gaussian falloff instead of equal-alpha terraces.
    // Correct for source-over so the combined centre reaches the token alpha.
    let mut previous_alpha = 0.0;
    let opacity = spec.opacity.clamp(0.0, 1.0) * f32::from(spec.color[3]) / 255.0;
    let base = color_from_rgba(spec.color);
    let mut result = Vec::with_capacity(layers as usize);
    for index in 0..layers {
        // index 0 → spread 1.0 (widest), last → 1/layers (tightest).
        let spread = f64::from(layers - index) / f64::from(layers);
        let blur = f64::from(spec.blur) * spread;
        let x = (f64::from(rect.loc.x) - blur).round() as i32;
        let y = (f64::from(rect.loc.y) + f64::from(spec.offset_y) - blur).round() as i32;
        let right = (f64::from(rect.loc.x + rect.size.w) + blur).round() as i32;
        let bottom =
            (f64::from(rect.loc.y + rect.size.h) + f64::from(spec.offset_y) + blur).round() as i32;
        let w = right - x;
        let h = bottom - y;
        let distance = (layers - index - 1) as f32 / layers as f32;
        let cutoff = (-4.5_f32).exp();
        let alpha = opacity * ((-4.5 * distance * distance).exp() - cutoff) / (1.0 - cutoff);
        let layer_alpha = (alpha - previous_alpha) / (1.0 - previous_alpha).max(f32::EPSILON);
        let multiplier = if base.a() > 0.0 {
            layer_alpha / base.a()
        } else {
            0.0
        };
        previous_alpha = alpha;
        result.push(ShadowLayer {
            rect: Rectangle::new((x, y).into(), (w.max(1), h.max(1)).into()),
            radius: spec.radius + blur as f32,
            color: base * multiplier,
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
    shadow_elements_with_occluder(rect, spec, scale, output_origin, motion, None)
}

/// The shadow ring outside a window's rounded body (T206).
///
/// The shadow layers are drawn behind the window and normally covered by its
/// opaque surface. A first-party window with app-local translucent chrome
/// leaves parts of that surface transparent, so the shadow would show through
/// and darken the measured sidebar/toolbar. This variant subtracts the
/// window's rounded rectangle from every shadow span, keeping the drop shadow
/// only where it belongs: outside the window body.
pub fn shadow_elements_outside(
    rect: Rectangle<i32, Logical>,
    spec: ShadowSpec,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
    motion: Option<MotionFrame>,
    window: Rectangle<i32, Logical>,
    radius: i32,
) -> Vec<SolidColorRenderElement> {
    shadow_elements_with_occluder(
        rect,
        spec,
        scale,
        output_origin,
        motion,
        Some((window, radius)),
    )
}

/// The shadow ring outside a window's **bounds**, not its rounded body.
///
/// The rounded-body cutout used by [`shadow_elements_outside`] keeps the
/// shadow out of the translucent chrome, but it also leaves shadow in the
/// square corner pixels between the rounded window arc and its bounding box.
/// On a rounded window those pixels are transparent in the surface, so the
/// shadow reads as a faint grey square behind the rounded titlebar. Cutting at
/// the bounds keeps the ring outside the window while leaving the corner
/// cutout clear; the rounded titlebar/`AppWindow` supplies the silhouette.
pub fn shadow_elements_outside_bounds(
    rect: Rectangle<i32, Logical>,
    spec: ShadowSpec,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
    motion: Option<MotionFrame>,
    window: Rectangle<i32, Logical>,
) -> Vec<SolidColorRenderElement> {
    shadow_elements_with_occluder(rect, spec, scale, output_origin, motion, Some((window, 0)))
}

fn shadow_elements_with_occluder(
    rect: Rectangle<i32, Logical>,
    spec: ShadowSpec,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
    motion: Option<MotionFrame>,
    occluder: Option<(Rectangle<i32, Logical>, i32)>,
) -> Vec<SolidColorRenderElement> {
    const SHADOW_SALT: u64 = 0x7368_6164_6f77_7773; // "shadowws"
    let geometry = cached_shadow_geometry(rect, spec, occluder);
    let mut elements = Vec::with_capacity(geometry.len());
    for &(index, span, layer_color) in geometry.iter() {
        let (span_rect, color) = motion_transform(span, layer_color, rect, motion);
        let local = Rectangle::new(
            (
                span_rect.loc.x - output_origin.x,
                span_rect.loc.y - output_origin.y,
            )
                .into(),
            span_rect.size,
        );
        let physical = physical_span(local, scale);
        if physical.is_empty() {
            continue;
        }
        elements.push(SolidColorRenderElement::new(
            crate::window::stable_solid_id(SHADOW_SALT ^ index as u64, physical, color),
            physical,
            CommitCounter::default(),
            color,
            Kind::Unspecified,
        ));
    }
    // `shadow_layers` is back-to-front; Smithay consumes front-to-back.
    elements.reverse();
    elements
}

// Adjacent spans share the same rounded endpoint at fractional output scales.
fn physical_span(rect: Rectangle<i32, Logical>, scale: Scale<f64>) -> Rectangle<i32, Physical> {
    let left = (f64::from(rect.loc.x) * scale.x).round() as i32;
    let top = (f64::from(rect.loc.y) * scale.y).round() as i32;
    let right = (f64::from(rect.loc.x + rect.size.w) * scale.x).round() as i32;
    let bottom = (f64::from(rect.loc.y + rect.size.h) * scale.y).round() as i32;
    Rectangle::new((left, top).into(), (right - left, bottom - top).into())
}

type ShadowGeometry = Vec<(usize, Rectangle<i32, Logical>, Color32F)>;
type ShadowGeometryKey = (
    Rectangle<i32, Logical>,
    ShadowSpec,
    Option<(Rectangle<i32, Logical>, i32)>,
);
thread_local! {
    // Keep settled windows out of CPU clipping when unrelated chrome animates.
    static SHADOW_GEOMETRY: RefCell<VecDeque<(ShadowGeometryKey, Rc<ShadowGeometry>)>> =
        const { RefCell::new(VecDeque::new()) };
}

fn cached_shadow_geometry(
    rect: Rectangle<i32, Logical>,
    spec: ShadowSpec,
    occluder: Option<(Rectangle<i32, Logical>, i32)>,
) -> Rc<ShadowGeometry> {
    let key = (rect, spec, occluder);
    SHADOW_GEOMETRY.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.iter().position(|(candidate, _)| *candidate == key) {
            let entry = cache.remove(index).unwrap();
            let geometry = Rc::clone(&entry.1);
            cache.push_back(entry);
            return geometry;
        }
        let occluder_spans = occluder
            .map(|(window, radius)| rounded_rect_spans(window, radius, RoundedCorners::All));
        let mut geometry = Vec::new();
        for (index, layer) in shadow_layers(rect, spec).into_iter().enumerate() {
            let radius = layer.radius.round() as i32;
            let mut spans = rounded_rect_spans(layer.rect, radius, RoundedCorners::All);
            if let Some(cuts) = &occluder_spans {
                spans = spans
                    .into_iter()
                    .flat_map(|span| {
                        cuts.iter().fold(vec![span], |pieces, cut| {
                            pieces
                                .into_iter()
                                .flat_map(|piece| subtract_rect(piece, *cut))
                                .collect()
                        })
                    })
                    .collect();
            }
            geometry.extend(spans.into_iter().map(|span| (index, span, layer.color)));
        }
        let geometry = Rc::new(geometry);
        if cache.len() >= 32 {
            cache.pop_front();
        }
        cache.push_back((key, Rc::clone(&geometry)));
        geometry
    })
}

/// Subtract `cut` from `rect`, returning the remaining non-overlapping
/// rectangles (top, bottom, left, right). Used to keep a shadow out of the
/// translucent window body while preserving the outside fringe.
fn subtract_rect(
    rect: Rectangle<i32, Logical>,
    cut: Rectangle<i32, Logical>,
) -> Vec<Rectangle<i32, Logical>> {
    let left = rect.loc.x.max(cut.loc.x);
    let top = rect.loc.y.max(cut.loc.y);
    let right = (rect.loc.x + rect.size.w).min(cut.loc.x + cut.size.w);
    let bottom = (rect.loc.y + rect.size.h).min(cut.loc.y + cut.size.h);
    if left >= right || top >= bottom {
        return vec![rect];
    }
    let mut pieces = Vec::with_capacity(4);
    let mut push = |x: i32, y: i32, w: i32, h: i32| {
        if w > 0 && h > 0 {
            pieces.push(Rectangle::new((x, y).into(), (w, h).into()));
        }
    };
    push(rect.loc.x, rect.loc.y, rect.size.w, top - rect.loc.y);
    push(
        rect.loc.x,
        bottom,
        rect.size.w,
        rect.loc.y + rect.size.h - bottom,
    );
    push(rect.loc.x, top, left - rect.loc.x, bottom - top);
    push(right, top, rect.loc.x + rect.size.w - right, bottom - top);
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;
    use smithay::backend::renderer::element::Element;
    use smithay::utils::{Physical, Size};

    #[test]
    fn fractional_scale_shadow_spans_share_edges_without_overdraw() {
        // The old location+size rounding produces alternating overlaps at
        // 1.5x; stacking translucent spans makes those rows dark stripes.
        for scale in [1.25, 1.5, 1.75, 2.0] {
            for y in -20..20 {
                let a = physical_span(rect(-3, y, 40, 1), scale.into());
                let b = physical_span(rect(-3, y + 1, 40, 1), scale.into());
                assert_eq!(a.loc.y + a.size.h, b.loc.y);
                assert!(a.intersection(b).is_none());
            }
        }
    }

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
        assert_eq!(dark.layers, 24);
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
        let shifted_window = Rectangle::new(
            (window.loc.x, window.loc.y + spec.offset_y as i32).into(),
            window.size,
        );
        assert!(last.contains_rect(shifted_window));

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

        // The tail starts almost transparent; source-over reaches the token
        // opacity without uniform dark bands at each layer boundary.
        assert!(layers[0].color.a() < 0.003);
        let mut alpha = 0.0;
        for layer in &layers {
            assert!(layer.color.a().is_finite() && layer.color.a() >= 0.0);
            let next = layer.color.a() + alpha * (1.0 - layer.color.a());
            assert!(next >= alpha && next - alpha < 0.04);
            alpha = next;
        }
        assert!((alpha - spec.opacity).abs() < 1e-6);

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
    fn shadow_outside_omits_the_window_body_but_keeps_the_fringe() {
        let window = rect(100, 200, 400, 300);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let elements = shadow_elements_outside(
            window,
            spec,
            1.0.into(),
            Point::from((0, 0)),
            None,
            window,
            spec.radius.round() as i32,
        );
        // The centre of the window (where translucent chrome would sample the
        // lower scene) must never receive a shadow element.
        let centre = Point::from((300, 350)).to_physical_precise_round(1.0);
        assert!(!elements
            .iter()
            .any(|element| element.geometry(1.0.into()).contains(centre)));
        // The outside fringe still reaches the full shadow bounds.
        assert_eq!(
            merged_bounds(&elements, 1.0.into()),
            shadow_bounds(window, spec).to_physical_precise_round(1.0)
        );
    }

    #[test]
    fn shadow_outside_bounds_keeps_the_rounded_corner_cutout_clear() {
        let window = rect(100, 200, 400, 300);
        let spec = ShadowLevel::High.spec(ColorScheme::Dark);
        let corners = [
            Point::from((window.loc.x, window.loc.y)),
            Point::from((window.loc.x + window.size.w - 1, window.loc.y)),
            Point::from((window.loc.x, window.loc.y + window.size.h - 1)),
            Point::from((
                window.loc.x + window.size.w - 1,
                window.loc.y + window.size.h - 1,
            )),
        ];
        let elements = shadow_elements_outside_bounds(
            window,
            spec,
            1.0.into(),
            Point::from((0, 0)),
            None,
            window,
        );
        for corner in corners {
            assert!(
                !elements
                    .iter()
                    .any(|element| element.geometry(1.0.into()).contains(corner)),
                "the corner cutout must be completely clear: {corner:?}"
            );
        }
        // The ring still reaches the full shadow bounds outside the window.
        assert_eq!(
            merged_bounds(&elements, 1.0.into()),
            shadow_bounds(window, spec).to_physical_precise_round(1.0)
        );

        // The rounded-body variant is the old behaviour: the square corner
        // outside the arc carries shadow. This is the regression guard for the
        // faint corner square the user saw.
        let rounded = shadow_elements_outside(
            window,
            spec,
            1.0.into(),
            Point::from((0, 0)),
            None,
            window,
            spec.radius.round() as i32,
        );
        let bottom_left = Point::from((window.loc.x, window.loc.y + window.size.h - 1));
        assert!(rounded
            .iter()
            .any(|element| element.geometry(1.0.into()).contains(bottom_left)));
    }

    #[test]
    fn subtract_rect_leaves_four_non_overlapping_pieces() {
        let pieces = subtract_rect(rect(0, 0, 10, 10), rect(3, 3, 4, 4));
        assert_eq!(pieces.len(), 4);
        let area: i32 = pieces.iter().map(|piece| piece.size.w * piece.size.h).sum();
        assert_eq!(area, 100 - 16);
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
