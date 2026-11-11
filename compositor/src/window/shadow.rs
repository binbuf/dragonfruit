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
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement};
use smithay::backend::renderer::gles::{GlesTexProgram, GlesTexture, Uniform};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::{Color32F, RendererSuper};
use smithay::utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size};
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

// --- GPU shadow (T238 decoration optimisation) -----------------------------
//
// The span decomposition is exact and testable but costs one GL draw call per
// span: at the full degrade tier a single window's shadow is ~2,300 solid
// elements. The scene and final composite each submit that list, so the CPU
// cost is dominated by draw-call setup rather than geometry.
//
// `SHADOW_SHADER` evaluates the same layered, truncated-Gaussian falloff and
// rounded-rect silhouette in one fragment shader. One quad replaces every span
// of one window's shadow, preserving the token formula while removing the
// per-span draw calls. The span path remains the fallback (and the reference
// the tests compare against).

/// The fragment shader for one window shadow. It is compiled through
/// [`GlesRenderer::compile_custom_texture_shader`](smithay::backend::renderer::gles::GlesRenderer::compile_custom_texture_shader),
/// so it carries the marker and built-in varyings/uniforms every custom
/// texture program declares. The texture is a 1×1 white probe; the shader
/// computes the shadow field itself.
pub const SHADOW_SHADER: &str = r#"#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision highp float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
varying vec2 v_coords;

// All geometry is output physical pixels. `shadow_size` is the quad; the
// window rect is quad-local (`xy` = origin, `zw` = size).
uniform vec2 shadow_size;
uniform vec4 shadow_window;
uniform float shadow_radius;
uniform float shadow_blur;
uniform float shadow_offset_y;
uniform float shadow_layers;
uniform float shadow_opacity;
uniform vec4 shadow_color;
uniform float shadow_cutout;

const int MAX_LAYERS = 32;
const float CUTOFF = 0.0111089965;

// 1px antialiased coverage of a rounded rectangle. `p` and `center` are
// physical pixels; positive distance is outside.
float rounded_coverage(vec2 p, vec2 center, vec2 half_size, float radius) {
    vec2 q = abs(p - center) - half_size + vec2(radius);
    vec2 outside = max(q, vec2(0.0));
    float dist = length(outside) + min(max(q.x, q.y), 0.0) - radius;
    return clamp(0.5 - dist, 0.0, 1.0);
}

void main() {
    vec2 p = v_coords * shadow_size;

    // The drop shadow is drawn outside the window's bounds, matching
    // `shadow_elements_outside_bounds`: the opaque surface owns the inside.
    if (shadow_cutout > 0.5) {
        vec2 wmin = shadow_window.xy;
        vec2 wmax = shadow_window.xy + shadow_window.zw;
        if (p.x > wmin.x && p.x < wmax.x && p.y > wmin.y && p.y < wmax.y) {
            discard;
        }
    }

    float base_alpha = shadow_color.a;
    float opacity = shadow_opacity * base_alpha;
    float previous = 0.0;
    // Accumulate straight-alpha source-over in the same back-to-front layer
    // order as `shadow_layers`.
    vec4 accum = vec4(0.0);
    for (int i = 0; i < MAX_LAYERS; i++) {
        if (float(i) >= shadow_layers) {
            break;
        }
        float index = float(i);
        float spread = (shadow_layers - index) / shadow_layers;
        float blur = shadow_blur * spread;
        vec2 center = shadow_window.xy + shadow_window.zw * 0.5
                    + vec2(0.0, shadow_offset_y);
        vec2 half_size = shadow_window.zw * 0.5 + vec2(blur);
        float radius = shadow_radius + blur;
        float coverage = rounded_coverage(p, center, half_size, radius);
        if (coverage <= 0.0) {
            continue;
        }
        float distance = (shadow_layers - index - 1.0) / shadow_layers;
        float a = opacity * (exp(-4.5 * distance * distance) - CUTOFF)
                / (1.0 - CUTOFF);
        float layer_alpha = (a - previous) / max(1.0 - previous, 0.000001);
        previous = a;
        float multiplier = layer_alpha / max(base_alpha, 0.000001);
        vec4 layer = vec4(shadow_color.rgb * multiplier,
                          base_alpha * multiplier * coverage);
        float out_alpha = layer.a + accum.a * (1.0 - layer.a);
        if (out_alpha > 0.0) {
            vec3 rgb = (layer.rgb * layer.a
                       + accum.rgb * accum.a * (1.0 - layer.a)) / out_alpha;
            accum = vec4(rgb, out_alpha);
        }
    }

    // Premultiplied for Smithay's (ONE, ONE_MINUS_SRC_ALPHA) blend. The
    // accumulator is straight-alpha source-over, so multiply the RGB by its
    // alpha once at the end.
    gl_FragColor = vec4(accum.rgb * accum.a, accum.a) * alpha;
}
"#;

pub const SHADOW_SIZE_UNIFORM: &str = "shadow_size";
pub const SHADOW_WINDOW_UNIFORM: &str = "shadow_window";
pub const SHADOW_RADIUS_UNIFORM: &str = "shadow_radius";
pub const SHADOW_BLUR_UNIFORM: &str = "shadow_blur";
pub const SHADOW_OFFSET_UNIFORM: &str = "shadow_offset_y";
pub const SHADOW_LAYERS_UNIFORM: &str = "shadow_layers";
pub const SHADOW_OPACITY_UNIFORM: &str = "shadow_opacity";
pub const SHADOW_COLOR_UNIFORM: &str = "shadow_color";
pub const SHADOW_CUTOUT_UNIFORM: &str = "shadow_cutout";

/// The additional uniforms [`SHADOW_SHADER`] declares.
pub fn shadow_uniform_names() -> [smithay::backend::renderer::gles::UniformName<'static>; 9] {
    use smithay::backend::renderer::gles::{UniformName, UniformType};
    [
        UniformName::new(SHADOW_SIZE_UNIFORM, UniformType::_2f),
        UniformName::new(SHADOW_WINDOW_UNIFORM, UniformType::_4f),
        UniformName::new(SHADOW_RADIUS_UNIFORM, UniformType::_1f),
        UniformName::new(SHADOW_BLUR_UNIFORM, UniformType::_1f),
        UniformName::new(SHADOW_OFFSET_UNIFORM, UniformType::_1f),
        UniformName::new(SHADOW_LAYERS_UNIFORM, UniformType::_1f),
        UniformName::new(SHADOW_OPACITY_UNIFORM, UniformType::_1f),
        UniformName::new(SHADOW_COLOR_UNIFORM, UniformType::_4f),
        UniformName::new(SHADOW_CUTOUT_UNIFORM, UniformType::_1f),
    ]
}

/// Opt-in GPU shadow path (T238 decoration optimisation). Off by default: the
/// span path is the reference until the three-way A/B validates the shader
/// against captured frames at 1.0× and fractional scales.
pub const ENV_SHADOW_GPU: &str = "DRAGONFRUIT_SHADOW_GPU";

/// The process-wide GPU-shadow switch. Runtime-switchable so a capture A/B
/// can flip it inside one compositor session, where the window placement and
/// focus state are identical. The environment variable only seeds it.
static SHADOW_GPU: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SHADOW_GPU_SEEDED: std::sync::OnceLock<()> = std::sync::OnceLock::new();

fn seed_shadow_gpu() {
    SHADOW_GPU_SEEDED.get_or_init(|| {
        SHADOW_GPU.store(
            crate::window::material_region::env_flag_enabled(
                std::env::var(ENV_SHADOW_GPU).ok().as_deref(),
            ),
            std::sync::atomic::Ordering::Relaxed,
        );
    });
}

/// Whether the one-quad shader shadow replaces the span decomposition.
pub fn shadow_gpu_enabled() -> bool {
    seed_shadow_gpu();
    SHADOW_GPU.load(std::sync::atomic::Ordering::Relaxed)
}

/// Override the GPU shadow path at runtime (synthetic `set shadow-gpu`).
pub fn set_shadow_gpu(enabled: bool) {
    SHADOW_GPU_SEEDED.get_or_init(|| ());
    SHADOW_GPU.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// A compiled shadow program plus the 1×1 white probe texture it samples.
/// Both are context-local, so the backend owns one per render GPU.
#[derive(Debug, Clone)]
pub struct ShadowResources {
    pub program: GlesTexProgram,
    pub texture: GlesTexture,
}

/// One window shadow drawn as a single quad (T238 decoration optimisation).
///
/// The geometry is the shadow's outer bounds; the fragment shader evaluates
/// every layer and the window cutout. The stable id hashes the full parameter
/// set, so moving or restyling a window invalidates the damage tracker while a
/// settled window reuses its cached damage state.
pub struct ShadowElement {
    id: Id,
    commit: CommitCounter,
    geometry: Rectangle<i32, Physical>,
    /// Window rect relative to `geometry`, in physical pixels.
    window: Rectangle<i32, Physical>,
    radius: f32,
    blur: f32,
    offset_y: f32,
    layers: f32,
    opacity: f32,
    color: Color32F,
    program: GlesTexProgram,
    texture: GlesTexture,
}

impl ShadowElement {
    /// Build the element for a window rect in output-local physical pixels.
    /// `radius`, `blur`, and `offset_y` are already physical.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window: Rectangle<i32, Physical>,
        radius: f32,
        blur: f32,
        offset_y: f32,
        layers: u32,
        opacity: f32,
        color: Color32F,
        resources: &ShadowResources,
    ) -> Self {
        let spread = blur.max(0.0).ceil() as i32;
        let offset = offset_y.round() as i32;
        let geometry = Rectangle::new(
            (window.loc.x - spread, window.loc.y + offset - spread).into(),
            (window.size.w + spread * 2, window.size.h + spread * 2).into(),
        );
        let local = Rectangle::new(
            (window.loc.x - geometry.loc.x, window.loc.y - geometry.loc.y).into(),
            window.size,
        );
        let key = stable_shadow_key(window, radius, blur, offset_y, layers, opacity, color);
        Self {
            id: crate::window::stable_id_from_key(key),
            commit: CommitCounter::default(),
            geometry,
            window: local,
            radius,
            blur,
            offset_y,
            layers: layers as f32,
            opacity,
            color,
            program: resources.program.clone(),
            texture: resources.texture.clone(),
        }
    }

    pub fn bounds(&self) -> Rectangle<i32, Physical> {
        self.geometry
    }
}

impl Element for ShadowElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
    }

    fn geometry(&self, _scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.geometry
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        Rectangle::from_size(Size::from((1.0, 1.0)))
    }

    fn alpha(&self) -> f32 {
        1.0
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl<R: crate::window::blur::TextureRenderer> RenderElement<R> for ShadowElement {
    fn draw(
        &self,
        frame: &mut <R as RendererSuper>::Frame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        let uniforms = [
            Uniform::new(SHADOW_SIZE_UNIFORM, (dst.size.w as f32, dst.size.h as f32)),
            Uniform::new(
                SHADOW_WINDOW_UNIFORM,
                (
                    self.window.loc.x as f32,
                    self.window.loc.y as f32,
                    self.window.size.w as f32,
                    self.window.size.h as f32,
                ),
            ),
            Uniform::new(SHADOW_RADIUS_UNIFORM, self.radius),
            Uniform::new(SHADOW_BLUR_UNIFORM, self.blur),
            Uniform::new(SHADOW_OFFSET_UNIFORM, self.offset_y),
            Uniform::new(SHADOW_LAYERS_UNIFORM, self.layers),
            Uniform::new(SHADOW_OPACITY_UNIFORM, self.opacity),
            Uniform::new(
                SHADOW_COLOR_UNIFORM,
                (
                    self.color.r(),
                    self.color.g(),
                    self.color.b(),
                    self.color.a(),
                ),
            ),
            Uniform::new(SHADOW_CUTOUT_UNIFORM, 1.0f32),
        ];
        R::draw_texture_from_to(
            frame,
            &self.texture,
            Rectangle::from_size(Size::from((1.0, 1.0))),
            dst,
            damage,
            Some(&self.program),
            &uniforms,
            1.0,
        )
    }
}

/// Hash every parameter that changes the shadow pixels. The element id is the
/// damage tracker's identity, so geometry, radius, falloff, colour, and layer
/// count all participate.
fn stable_shadow_key(
    window: Rectangle<i32, Physical>,
    radius: f32,
    blur: f32,
    offset_y: f32,
    layers: u32,
    opacity: f32,
    color: Color32F,
) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    0x7368_6164_6f77_6750u64.hash(&mut hasher); // "shadowgpu"
    window.loc.x.hash(&mut hasher);
    window.loc.y.hash(&mut hasher);
    window.size.w.hash(&mut hasher);
    window.size.h.hash(&mut hasher);
    radius.to_bits().hash(&mut hasher);
    blur.to_bits().hash(&mut hasher);
    offset_y.to_bits().hash(&mut hasher);
    layers.hash(&mut hasher);
    opacity.to_bits().hash(&mut hasher);
    for component in color.components() {
        component.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

/// Convert a resolved [`ShadowSpec`] and a logical window rect into one GPU
/// shadow element, or `None` when the shadow has no spread. The rect and the
/// spec are transformed by the window's lifecycle motion first, matching the
/// span path.
pub fn shadow_element(
    window: Rectangle<i32, Logical>,
    spec: ShadowSpec,
    scale: Scale<f64>,
    output_origin: Point<i32, Logical>,
    motion: Option<MotionFrame>,
    resources: &ShadowResources,
) -> Option<ShadowElement> {
    // Only the rect comes from the lifecycle transform; the fade is applied
    // once through `opacity`. Passing the faded colour here *and* multiplying
    // opacity by `motion_alpha` would square the appear/minimize fade.
    let (rect, _) = motion_transform(window, Color32F::new(0.0, 0.0, 0.0, 0.0), window, motion);
    let color = color_from_rgba(spec.color);
    let motion_alpha = motion.map(|frame| frame.alpha).unwrap_or(1.0);
    let motion_scale = motion
        .map(|frame| (frame.scale.x as f32, frame.scale.y as f32))
        .unwrap_or((1.0, 1.0));
    let opacity = (spec.opacity * motion_alpha).clamp(0.0, 1.0);
    let radius = spec.radius * scale.x as f32 * motion_scale.0;
    let blur = spec.blur * scale.x as f32 * motion_scale.0;
    let offset_y = spec.offset_y * scale.y as f32 * motion_scale.1;
    let physical = Rectangle::new(
        (
            (f64::from(rect.loc.x - output_origin.x) * scale.x).round() as i32,
            (f64::from(rect.loc.y - output_origin.y) * scale.y).round() as i32,
        )
            .into(),
        (
            (f64::from(rect.size.w) * scale.x).round().max(1.0) as i32,
            (f64::from(rect.size.h) * scale.y).round().max(1.0) as i32,
        )
            .into(),
    );
    if physical.size.w <= 0 || physical.size.h <= 0 {
        return None;
    }
    Some(ShadowElement::new(
        physical,
        radius,
        blur,
        offset_y,
        spec.layers,
        opacity,
        color,
        resources,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shadow_shader_declares_its_uniforms() {
        assert!(SHADOW_SHADER.contains("//_DEFINES_"));
        assert!(SHADOW_SHADER.contains("varying vec2 v_coords;"));
        assert!(SHADOW_SHADER.contains("uniform sampler2D tex;"));
        assert!(SHADOW_SHADER.contains("uniform float alpha;"));
        for name in [
            SHADOW_SIZE_UNIFORM,
            SHADOW_WINDOW_UNIFORM,
            SHADOW_RADIUS_UNIFORM,
            SHADOW_BLUR_UNIFORM,
            SHADOW_OFFSET_UNIFORM,
            SHADOW_LAYERS_UNIFORM,
            SHADOW_OPACITY_UNIFORM,
            SHADOW_COLOR_UNIFORM,
            SHADOW_CUTOUT_UNIFORM,
        ] {
            assert!(
                SHADOW_SHADER.contains(&format!("uniform")),
                "shader missing {name}"
            );
            assert!(SHADOW_SHADER.contains(name), "shader missing {name}");
        }
        assert!(SHADOW_SHADER.contains("gl_FragColor"));
    }

    #[test]
    fn the_shadow_uniform_names_match_the_shader() {
        use smithay::backend::renderer::gles::UniformType;
        let names = shadow_uniform_names();
        let by_name: Vec<(&str, UniformType)> =
            names.iter().map(|name| (&*name.name, name.type_)).collect();
        assert!(by_name.contains(&(SHADOW_SIZE_UNIFORM, UniformType::_2f)));
        assert!(by_name.contains(&(SHADOW_WINDOW_UNIFORM, UniformType::_4f)));
        assert!(by_name.contains(&(SHADOW_COLOR_UNIFORM, UniformType::_4f)));
        assert!(by_name.contains(&(SHADOW_LAYERS_UNIFORM, UniformType::_1f)));
    }
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
