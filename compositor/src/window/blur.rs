// SPDX-License-Identifier: MIT
//! GPU-sampled backdrop blur for chrome panels (T-20.1).
//!
//! ADR [0182](../../docs/design/adr/0182-tahoe-liquid-glass-material-pass.md)
//! moves the chrome material off the flat feather stand-in
//! ([`crate::window::backdrop`]) and onto a real pass that samples the **live**
//! scene texture. This module owns the renderer-facing half of that pass:
//!
//! * [`BLUR_SHADER`] — a small custom texture program (a 4-tap Kawase
//!   downsample) compiled once per renderer with Smithay's
//!   `GlesRenderer::compile_custom_texture_shader`.
//! * [`BlurSpec`] — the token-resolved blur for one panel: the logical blur
//!   radius (from `material.{chrome,popup,dock}Blur`), the derived Kawase
//!   iteration count, and the panel corner radius.
//! * [`BackdropBlurElement`] — the render element that draws the blurred
//!   panel texture under the chrome surface, clipped to the shared
//!   [`CornerMask`](crate::window::corner::CornerMask) spans so the rounded
//!   edge matches the titlebar/shadow/backdrop geometry.
//!
//! Everything except the element is pure geometry, so the role → spec mapping
//! and the downsample plan are asserted headless (no GPU). The element itself
//! never invents a value: the radius, iteration count, and corners all trace
//! back to the generated tokens.

use std::borrow::BorrowMut;

use smithay::backend::renderer::element::texture::TextureRenderElement;
use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement};
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesTexProgram, GlesTexture, Uniform, UniformName, UniformType,
};
use smithay::backend::renderer::glow::{GlowFrame, GlowRenderer};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::Renderer;
use smithay::utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size, Transform};

/// The custom texture shader compiled once per renderer (T-20.1).
///
/// It samples the scene texture with a 4-tap Kawase downsample: the centre
/// texel is weighted `4` and the four diagonal neighbours at
/// `blur_radius * blur_texel` are weighted `1` each, so a chain of halvings
/// widens the kernel without a per-pixel Gaussian. It respects the renderer's
/// `EXTERNAL`, `NO_ALPHA`, and `DEBUG_FLAGS` variants exactly like the
/// built-in texture program (the `//_DEFINES_` marker is required by Smithay).
pub const BLUR_SHADER: &str = r#"#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision mediump float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
uniform vec2 blur_texel;
uniform float blur_radius;
varying vec2 v_coords;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

void main() {
    vec4 color = texture2D(tex, v_coords) * 4.0;
    color += texture2D(tex, v_coords + vec2( blur_radius,  blur_radius) * blur_texel);
    color += texture2D(tex, v_coords + vec2(-blur_radius,  blur_radius) * blur_texel);
    color += texture2D(tex, v_coords + vec2( blur_radius, -blur_radius) * blur_texel);
    color += texture2D(tex, v_coords + vec2(-blur_radius, -blur_radius) * blur_texel);
    color = color / 8.0;

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0) * alpha;
#else
    color = color * alpha;
#endif

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
"#;

/// The custom texture shader for the Tahoe liquid-glass pass (T-20.2).
///
/// It is compiled once per renderer like [`BLUR_SHADER`] and used on the final
/// compose of a blurred panel (the built-in texture program would otherwise do
/// a plain bilinear sample). `v_coords` is the panel-normalized coordinate
/// because the element maps the whole blurred panel texture onto the panel, so
/// the shader can reconstruct panel-local pixels and evaluate the rounded-rect
/// signed distance field. On top of the blurred backdrop it adds:
///
/// * an **edge lens** — the sample is pulled toward the panel centre near the
///   rim by `refraction * lens_falloff`, so the backdrop reads refracted
///   through a thick glass edge;
/// * a **specular inner rim** — a thin bright band `specular_width` inside the
///   edge, scaled by `specular`;
/// * an **adaptive tint** — the blurred backdrop's mean luminance (a 3×3
///   sample grid) is compared with the scheme tone's luminance and the colour
///   is blended toward the tone only when the backdrop drifts from it, so
///   label contrast is never inverted.
///
/// Every number comes from a semantic material token through a uniform; the
/// shader itself holds no literal token value.
pub const GLASS_SHADER: &str = r#"#version 100

//_DEFINES_

#if defined(EXTERNAL)
#extension GL_OES_EGL_image_external : require
#endif

precision mediump float;
#if defined(EXTERNAL)
uniform samplerExternalOES tex;
#else
uniform sampler2D tex;
#endif

uniform float alpha;
varying vec2 v_coords;

uniform vec2 glass_size;
uniform float glass_radius;
uniform float refraction;
uniform float specular;
uniform float specular_width;
uniform float tint_amount;
uniform vec3 tint_color;
uniform float lens_falloff;
uniform float tint_scale;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

float luminance(vec3 c) {
    return dot(c, vec3(0.299, 0.587, 0.114));
}

void main() {
    vec2 p = (v_coords - 0.5) * glass_size;
    vec2 half_size = max(glass_size * 0.5 - vec2(glass_radius), vec2(0.0));
    vec2 q = abs(p) - half_size;
    vec2 outside = max(q, vec2(0.0));
    float dist = length(outside) + min(max(q.x, q.y), 0.0) - glass_radius;
    float inside = max(-dist, 0.0);

    // The outward SDF normal, used as the lens bend direction.
    float ol = length(outside);
    vec2 normal = (ol > 0.0) ? outside / ol : normalize(p + vec2(1e-5));
    normal = normal * sign(p + vec2(1e-6));

    // Edge lens: falloff is one at the rim and reaches zero at lens_falloff.
    float lens = clamp(1.0 - inside / max(lens_falloff, 1.0), 0.0, 1.0);
    lens = lens * lens * (3.0 - 2.0 * lens);
    vec2 uv = v_coords - normal * (refraction * lens) / max(glass_size, vec2(1.0));

    vec4 color = texture2D(tex, uv);

    // Adaptive tint: 3x3 mean luminance of the blurred backdrop.
    vec3 mean = vec3(0.0);
    mean += texture2D(tex, vec2(0.25, 0.25)).rgb;
    mean += texture2D(tex, vec2(0.50, 0.25)).rgb;
    mean += texture2D(tex, vec2(0.75, 0.25)).rgb;
    mean += texture2D(tex, vec2(0.25, 0.50)).rgb;
    mean += texture2D(tex, vec2(0.50, 0.50)).rgb;
    mean += texture2D(tex, vec2(0.75, 0.50)).rgb;
    mean += texture2D(tex, vec2(0.25, 0.75)).rgb;
    mean += texture2D(tex, vec2(0.50, 0.75)).rgb;
    mean += texture2D(tex, vec2(0.75, 0.75)).rgb;
    mean = mean / 9.0;
    float adapt = clamp(abs(luminance(mean) - luminance(tint_color)) * tint_scale, 0.0, 1.0);
    color.rgb = mix(color.rgb, tint_color, clamp(tint_amount * adapt, 0.0, 1.0));

    // Specular inner rim: a thin bright band just inside the rounded edge.
    float band = clamp(1.0 - abs(inside - specular_width) / max(specular_width, 1.0), 0.0, 1.0);
    color.rgb += vec3(specular * band);

#if defined(NO_ALPHA)
    color = vec4(color.rgb, 1.0) * alpha;
#else
    color = color * alpha;
#endif

#if defined(DEBUG_FLAGS)
    if (tint == 1.0)
        color = vec4(0.0, 0.2, 0.0, 0.2) + color * 0.8;
#endif

    gl_FragColor = color;
}
"#;

/// The liquid-glass shader uniform carrying the panel size in physical pixels.
pub const GLASS_SIZE_UNIFORM: &str = "glass_size";
/// The liquid-glass shader uniform carrying the corner radius (physical px).
pub const GLASS_RADIUS_UNIFORM: &str = "glass_radius";
/// The liquid-glass shader uniform carrying the edge-lens strength (px).
pub const REFRACTION_UNIFORM: &str = "refraction";
/// The liquid-glass shader uniform carrying the specular intensity.
pub const SPECULAR_UNIFORM: &str = "specular";
/// The liquid-glass shader uniform carrying the specular band width (px).
pub const SPECULAR_WIDTH_UNIFORM: &str = "specular_width";
/// The liquid-glass shader uniform carrying the adaptive-tint amount.
pub const TINT_AMOUNT_UNIFORM: &str = "tint_amount";
/// The liquid-glass shader uniform carrying the scheme tone to blend toward.
pub const TINT_COLOR_UNIFORM: &str = "tint_color";
/// The liquid-glass shader uniform carrying the lens falloff width (px).
pub const LENS_FALLOFF_UNIFORM: &str = "lens_falloff";
/// The liquid-glass shader uniform carrying the tint adaptation scale.
pub const TINT_SCALE_UNIFORM: &str = "tint_scale";

/// The lens falloff width as a multiple of the token refraction strength: a
/// mapping constant, not a radius. The refraction token is the displacement at
/// the rim; the falloff reaches zero `LENS_BAND_FACTOR * refraction` inside.
pub const LENS_BAND_FACTOR: f32 = 2.5;
/// How sharply the adaptive tint responds to the backdrop/scheme luminance gap:
/// `clamp(|lum_backdrop - lum_tone| * TINT_ADAPT_SCALE)` decides the fraction
/// of the token tint amount that applies at a pixel. A mapping constant.
pub const TINT_ADAPT_SCALE: f32 = 2.5;

/// Convert a scheme tone (the generated token `[r,g,b,a]`) to the shader's
/// normalized RGB triple, ignoring its alpha (the tint does not change the
/// panel's opacity).
pub fn tone_to_rgb(tone: [u8; 4]) -> [f32; 3] {
    [
        f32::from(tone[0]) / 255.0,
        f32::from(tone[1]) / 255.0,
        f32::from(tone[2]) / 255.0,
    ]
}

/// The additional shader uniform carrying `1 / texture_size`, so the Kawase
/// taps are expressed in source texels.
pub const BLUR_TEXEL_UNIFORM: &str = "blur_texel";
/// The additional shader uniform carrying the tap radius in source texels.
pub const BLUR_RADIUS_UNIFORM: &str = "blur_radius";

/// Logical pixels of blur per Kawase iteration. The token blur maps to an
/// iteration count through this constant (a mapping, not a radius: the radius
/// itself is always the token value). Bounded so an oversized token cannot
/// request an unbounded blur chain.
pub const BLUR_PER_ITERATION: f32 = 8.0;
/// Upper bound on the Kawase iteration count (a 1/16 downsample).
pub const MAX_BLUR_ITERATIONS: u32 = 4;

/// The number of Kawase iterations for a token blur radius: larger radii blur
/// more, saturating at [`MAX_BLUR_ITERATIONS`] and never below one (a panel
/// always gets at least a single downsample).
pub fn iterations_for_radius(radius: f32) -> u32 {
    let iterations = (radius / BLUR_PER_ITERATION).round() as i32;
    iterations.clamp(1, MAX_BLUR_ITERATIONS as i32) as u32
}

/// The linear downsample factor a Kawase chain of `iterations` produces.
pub const fn downsample_factor(iterations: u32) -> u32 {
    1u32 << iterations
}

/// The additional uniform descriptors [`BLUR_SHADER`] declares, passed to
/// `GlesRenderer::compile_custom_texture_shader`.
pub fn blur_uniform_names() -> [UniformName<'static>; 2] {
    [
        UniformName::new(BLUR_TEXEL_UNIFORM, UniformType::_2f),
        UniformName::new(BLUR_RADIUS_UNIFORM, UniformType::_1f),
    ]
}

/// The additional uniform descriptors [`GLASS_SHADER`] declares, passed to
/// `GlesRenderer::compile_custom_texture_shader`.
pub fn glass_uniform_names() -> [UniformName<'static>; 9] {
    [
        UniformName::new(GLASS_SIZE_UNIFORM, UniformType::_2f),
        UniformName::new(GLASS_RADIUS_UNIFORM, UniformType::_1f),
        UniformName::new(REFRACTION_UNIFORM, UniformType::_1f),
        UniformName::new(SPECULAR_UNIFORM, UniformType::_1f),
        UniformName::new(SPECULAR_WIDTH_UNIFORM, UniformType::_1f),
        UniformName::new(TINT_AMOUNT_UNIFORM, UniformType::_1f),
        UniformName::new(TINT_COLOR_UNIFORM, UniformType::_3f),
        UniformName::new(LENS_FALLOFF_UNIFORM, UniformType::_1f),
        UniformName::new(TINT_SCALE_UNIFORM, UniformType::_1f),
    ]
}

/// The token-resolved blur for one chrome panel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlurSpec {
    /// The softest blur spread in logical pixels (`material.*Blur`), after the
    /// active degrade tier.
    pub radius: f32,
    /// Number of Kawase downsample iterations, derived from [`Self::radius`].
    pub iterations: u32,
    /// The panel's rounded-corner radius (`component.*.radius`).
    pub corner_radius: f32,
}

impl BlurSpec {
    /// Resolve a spec from a token blur radius and a token corner radius.
    pub fn new(radius: f32, corner_radius: f32) -> Self {
        BlurSpec {
            radius,
            iterations: iterations_for_radius(radius),
            corner_radius,
        }
    }

    /// The linear downsample factor `2^iterations`.
    pub const fn downsample(&self) -> u32 {
        downsample_factor(self.iterations)
    }

    /// The blurred texture size for a panel of `region` logical pixels: the
    /// region halved `iterations` times, never empty.
    pub fn texture_size(&self, region: Size<i32, Logical>) -> Size<i32, Buffer> {
        let w = (region.w >> self.iterations).max(1);
        let h = (region.h >> self.iterations).max(1);
        Size::from((w, h))
    }
}

/// The token-resolved Tahoe liquid-glass parameters for one chrome panel
/// (T-20.2, ADR 0182). Every field traces back to a semantic `material` token;
/// the shader takes no literal value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlassSpec {
    /// Edge-lens displacement at the rim in logical pixels
    /// (`material.*Refraction`).
    pub refraction: f32,
    /// Specular inner-rim intensity, `0..=1` (`material.*Specular`).
    pub specular: f32,
    /// Specular band width in logical pixels (`material.*SpecularWidth`).
    pub specular_width: f32,
    /// Adaptive-tint amount, `0..=1` (`material.*Tint`).
    pub tint: f32,
    /// The scheme surface/chrome tone the tint blends toward, normalized RGB.
    pub tint_color: [f32; 3],
}

impl GlassSpec {
    /// The lens falloff width in logical pixels: the token refraction maps to
    /// a band through [`LENS_BAND_FACTOR`].
    pub fn lens_width(&self) -> f32 {
        self.refraction * LENS_BAND_FACTOR
    }

    /// The shader uniforms for a panel of `physical` size at output `scale`.
    /// The logical tokens are scaled to physical pixels so the SDF, the lens,
    /// and the specular band are device-pixel consistent.
    pub fn uniforms(
        &self,
        physical: Size<i32, Physical>,
        corner_radius: i32,
        scale: Scale<f64>,
    ) -> Vec<Uniform<'static>> {
        let w = physical.w.max(1) as f32;
        let h = physical.h.max(1) as f32;
        let sx = scale.x.max(f64::EPSILON) as f32;
        vec![
            Uniform::new(GLASS_SIZE_UNIFORM, (w, h)),
            Uniform::new(GLASS_RADIUS_UNIFORM, corner_radius as f32 * sx),
            Uniform::new(REFRACTION_UNIFORM, self.refraction * sx),
            Uniform::new(SPECULAR_UNIFORM, self.specular),
            Uniform::new(SPECULAR_WIDTH_UNIFORM, self.specular_width * sx),
            Uniform::new(TINT_AMOUNT_UNIFORM, self.tint),
            Uniform::new(
                TINT_COLOR_UNIFORM,
                (self.tint_color[0], self.tint_color[1], self.tint_color[2]),
            ),
            Uniform::new(LENS_FALLOFF_UNIFORM, self.lens_width() * sx),
            Uniform::new(TINT_SCALE_UNIFORM, TINT_ADAPT_SCALE),
        ]
    }
}

/// A renderer that can draw a sampled backdrop span. Implemented for the
/// nested backend's [`GlowRenderer`]; the marker keeps the nested element enum
/// generic while only the GL path can construct a blur element.
pub trait BlurRenderer: Renderer<TextureId = GlesTexture> {
    /// Draw `src` (texture buffer coordinates) into `dst` (output-local
    /// physical), clipped to `damage`. When `program` is set the material uses
    /// the liquid-glass texture program with `uniforms`; otherwise the built-in
    /// texture program does a plain bilinear sample.
    fn draw_blur_span(
        frame: &mut Self::Frame<'_, '_>,
        texture: &GlesTexture,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        program: Option<&GlesTexProgram>,
        uniforms: &[Uniform<'_>],
    ) -> Result<(), Self::Error>;
}

impl BlurRenderer for GlowRenderer {
    fn draw_blur_span(
        frame: &mut GlowFrame<'_, '_>,
        texture: &GlesTexture,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        program: Option<&GlesTexProgram>,
        uniforms: &[Uniform<'_>],
    ) -> Result<(), GlesError> {
        // With no glass program the upsample is a plain bilinear sample of the
        // already-blurred texture; with one it is the liquid-glass compose.
        let frame: &mut GlesFrame<'_, '_> = BorrowMut::borrow_mut(frame);
        frame.render_texture_from_to(
            texture,
            src,
            dst,
            damage,
            &[],
            Transform::Normal,
            1.0,
            program,
            uniforms,
        )
    }
}

/// A blurred chrome panel's texture composited under the chrome surface
/// (T-20.1). The texture is the Kawase-downsampled scene region; the element
/// draws it stretched back over the panel, clipped to the token corner radius.
pub struct BackdropBlurElement {
    id: Id,
    texture: GlesTexture,
    /// The panel in output-local logical coordinates.
    panel: Rectangle<i32, Logical>,
    /// The texture's buffer size (the downsampled panel region).
    texture_size: Size<i32, Buffer>,
    /// The panel corner radius in logical pixels.
    corner_radius: i32,
    scale: Scale<f64>,
    /// The liquid-glass parameters and compiled program (T-20.2), or `None`
    /// for the plain blurred compose (the scene texture, or a driver that
    /// refused the glass shader).
    glass: Option<(GlassSpec, GlesTexProgram)>,
}

impl BackdropBlurElement {
    /// Build the element for `panel` (output-local logical) sampling
    /// `texture`, the blurred scene region. `scale` is the output scale.
    pub fn new(
        texture: GlesTexture,
        panel: Rectangle<i32, Logical>,
        texture_size: Size<i32, Buffer>,
        corner_radius: i32,
        scale: Scale<f64>,
    ) -> Self {
        BackdropBlurElement {
            id: Id::new(),
            texture,
            panel,
            texture_size,
            corner_radius,
            scale,
            glass: None,
        }
    }

    /// Apply the Tahoe liquid-glass compose (T-20.2) on top of the blurred
    /// backdrop using the compiled [`GLASS_SHADER`] `program`.
    pub fn with_glass(mut self, spec: GlassSpec, program: GlesTexProgram) -> Self {
        self.glass = Some((spec, program));
        self
    }

    /// The rounded spans of the panel at the token corner radius.
    fn spans(&self) -> Vec<Rectangle<i32, Logical>> {
        crate::window::corner::rounded_rect_spans(
            self.panel,
            self.corner_radius,
            crate::window::corner::RoundedCorners::All,
        )
    }
}

impl Element for BackdropBlurElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        CommitCounter::default()
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.panel.to_physical_precise_round(scale)
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        Rectangle::from_size(self.texture_size.to_f64())
    }

    fn alpha(&self) -> f32 {
        1.0
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl<R: BlurRenderer> RenderElement<R> for BackdropBlurElement {
    fn draw(
        &self,
        frame: &mut R::Frame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        _dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        // The Kawase texture is smaller than the panel; each rounded span maps
        // its slice of the panel onto the corresponding slice of the texture.
        let ratio_x = self.texture_size.w as f64 / f64::from(self.panel.size.w);
        let ratio_y = self.texture_size.h as f64 / f64::from(self.panel.size.h);
        // The glass uniforms are per-panel, not per-span, so build them once.
        let program = self.glass.as_ref().map(|(_, program)| program);
        let uniforms = self
            .glass
            .as_ref()
            .map(|(spec, _)| {
                spec.uniforms(
                    self.panel.to_physical_precise_round(self.scale).size,
                    self.corner_radius,
                    self.scale,
                )
            })
            .unwrap_or_default();
        for span in self.spans() {
            let dst = span.to_physical_precise_round(self.scale);
            let rel_x = f64::from(span.loc.x - self.panel.loc.x);
            let rel_y = f64::from(span.loc.y - self.panel.loc.y);
            let src = Rectangle::new(
                Point::from((rel_x * ratio_x, rel_y * ratio_y)),
                Size::from((
                    f64::from(span.size.w) * ratio_x,
                    f64::from(span.size.h) * ratio_y,
                )),
            );
            R::draw_blur_span(frame, &self.texture, src, dst, damage, program, &uniforms)?;
        }
        Ok(())
    }
}

/// A plain texture element for the composed scene, reused from Smithay's
/// texture element. Kept as a type alias so nested.rs can name it.
pub type SceneTextureElement = TextureRenderElement<GlesTexture>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_blur_radius_maps_to_bounded_kawase_iterations() {
        // Larger token blur → more iterations; never zero, never unbounded.
        assert_eq!(iterations_for_radius(0.0), 1);
        assert_eq!(iterations_for_radius(8.0), 1);
        assert_eq!(iterations_for_radius(24.0), 3);
        assert_eq!(iterations_for_radius(28.0), 4);
        assert_eq!(iterations_for_radius(1_000.0), MAX_BLUR_ITERATIONS);
        for radius in [0.0, 1.0, 12.0, 16.0, 24.0, 30.0, 34.0, 100.0] {
            let n = iterations_for_radius(radius);
            assert!((1..=MAX_BLUR_ITERATIONS).contains(&n));
        }
    }

    #[test]
    fn a_spec_derives_its_downsample_and_texture_size() {
        let spec = BlurSpec::new(24.0, 10.0);
        assert_eq!(spec.radius, 24.0);
        assert_eq!(spec.iterations, 3);
        assert_eq!(spec.downsample(), 8);
        assert_eq!(spec.corner_radius, 10.0);

        // A 1920x28 menu bar at /8 is 240x3 (widened to at least 1).
        let size = spec.texture_size(Size::from((1920, 28)));
        assert_eq!(size, Size::from((240, 3)));
        // A degenerate panel never produces an empty texture.
        assert_eq!(spec.texture_size(Size::from((1, 1))), Size::from((1, 1)));
    }

    #[test]
    fn the_shader_declares_the_uniforms_the_renderer_sets() {
        // The custom program's contract: the marker Smithay replaces, the
        // built-in varyings/uniforms, and the two additional uniforms.
        assert!(BLUR_SHADER.contains("//_DEFINES_"));
        assert!(BLUR_SHADER.contains("varying vec2 v_coords;"));
        assert!(BLUR_SHADER.contains("uniform sampler2D tex;"));
        assert!(BLUR_SHADER.contains("uniform float alpha;"));
        assert!(BLUR_SHADER.contains(BLUR_TEXEL_UNIFORM));
        assert!(BLUR_SHADER.contains(BLUR_RADIUS_UNIFORM));
        assert!(BLUR_SHADER.contains("gl_FragColor"));
    }

    #[test]
    fn the_glass_shader_declares_its_uniforms_and_no_token_literals() {
        // The liquid-glass program is a custom texture shader like the blur,
        // so it carries the marker, the built-in uniforms, and every one of
        // its own uniforms. The parameters themselves arrive by uniform, so
        // the shader body holds no token value.
        assert!(GLASS_SHADER.contains("//_DEFINES_"));
        assert!(GLASS_SHADER.contains("varying vec2 v_coords;"));
        assert!(GLASS_SHADER.contains("uniform sampler2D tex;"));
        assert!(GLASS_SHADER.contains("uniform float alpha;"));
        for name in [
            GLASS_SIZE_UNIFORM,
            GLASS_RADIUS_UNIFORM,
            REFRACTION_UNIFORM,
            SPECULAR_UNIFORM,
            SPECULAR_WIDTH_UNIFORM,
            TINT_AMOUNT_UNIFORM,
            TINT_COLOR_UNIFORM,
            LENS_FALLOFF_UNIFORM,
            TINT_SCALE_UNIFORM,
        ] {
            assert!(GLASS_SHADER.contains(name), "shader is missing {name}");
        }
        assert!(GLASS_SHADER.contains("gl_FragColor"));
        // The lens, specular, and adaptive tint are all present.
        assert!(GLASS_SHADER.contains("luminance"));
        assert!(GLASS_SHADER.contains("refraction"));
        assert!(GLASS_SHADER.contains("specular_width"));
        assert!(GLASS_SHADER.contains("tint_amount"));
    }

    #[test]
    fn a_glass_spec_maps_tokens_to_physical_uniforms() {
        use smithay::backend::renderer::gles::UniformValue;
        let spec = GlassSpec {
            refraction: 6.0,
            specular: 0.45,
            specular_width: 2.0,
            tint: 0.16,
            tint_color: [0.1, 0.2, 0.3],
        };
        // The lens falloff is a derived mapping constant, not a token.
        assert_eq!(spec.lens_width(), 6.0 * LENS_BAND_FACTOR);

        // At scale 2 the logical tokens become physical pixels; the corner
        // radius is scaled with them so the SDF is device-pixel consistent.
        let uniforms = spec.uniforms(Size::from((1600, 56)), 20, Scale::from(2.0));
        assert_eq!(uniforms.len(), 9);
        assert_eq!(
            uniforms[0].value,
            UniformValue::_2f(1600.0, 56.0),
            "glass size is the physical panel"
        );
        assert_eq!(uniforms[1].value, UniformValue::_1f(40.0));
        assert_eq!(uniforms[2].value, UniformValue::_1f(12.0));
        assert_eq!(uniforms[3].value, UniformValue::_1f(0.45));
        assert_eq!(uniforms[4].value, UniformValue::_1f(4.0));
        assert_eq!(uniforms[5].value, UniformValue::_1f(0.16));
        assert_eq!(uniforms[6].value, UniformValue::_3f(0.1, 0.2, 0.3));
        // The lens falloff is the derived mapping constant scaled to physical.
        assert_eq!(uniforms[7].value, UniformValue::_1f(30.0));
        assert_eq!(uniforms[8].value, UniformValue::_1f(TINT_ADAPT_SCALE));

        // A degenerate panel is clamped, never a zero-size SDF.
        let uniforms = spec.uniforms(Size::from((0, 0)), 0, Scale::from(1.0));
        assert_eq!(uniforms[0].value, UniformValue::_2f(1.0, 1.0));
    }

    #[test]
    fn tone_to_rgb_drops_alpha_and_normalizes() {
        assert_eq!(tone_to_rgb([255, 128, 0, 130]), [1.0, 128.0 / 255.0, 0.0]);
        assert_eq!(tone_to_rgb([0, 0, 0, 255]), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn the_rounded_panel_is_clipped_to_the_corner_radius() {
        // The element's span plan is the shared CornerMask geometry, so the
        // rounded edge matches the titlebar/shadow. Assert it here without a
        // GPU: a panel with a corner radius covers strictly less than its
        // bounding box, and every span stays inside it.
        use crate::window::corner::{rounded_rect_spans, RoundedCorners};
        let panel = Rectangle::new(Point::from((100, 20)), Size::from((800, 24)));
        let spans = rounded_rect_spans(panel, 10, RoundedCorners::All);
        let covered: i64 = spans
            .iter()
            .map(|s| i64::from(s.size.w) * i64::from(s.size.h))
            .sum();
        let full = i64::from(panel.size.w) * i64::from(panel.size.h);
        assert!(covered > 0 && covered < full);
        for span in &spans {
            assert!(panel.contains_rect(*span));
        }
    }
}
