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
    GlesError, GlesFrame, GlesTexture, UniformName, UniformType,
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

/// A renderer that can draw a sampled backdrop span. Implemented for the
/// nested backend's [`GlowRenderer`]; the marker keeps the nested element enum
/// generic while only the GL path can construct a blur element.
pub trait BlurRenderer: Renderer<TextureId = GlesTexture> {
    /// Draw `src` (texture buffer coordinates) into `dst` (output-local
    /// physical), clipped to `damage`.
    fn draw_blur_span(
        frame: &mut Self::Frame<'_, '_>,
        texture: &GlesTexture,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), Self::Error>;
}

impl BlurRenderer for GlowRenderer {
    fn draw_blur_span(
        frame: &mut GlowFrame<'_, '_>,
        texture: &GlesTexture,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        // The upsample on compose uses the built-in texture program: the
        // texture is already blurred, so a plain bilinear sample is enough.
        let frame: &mut GlesFrame<'_, '_> = BorrowMut::borrow_mut(frame);
        frame.render_texture_from_to(
            texture,
            src,
            dst,
            damage,
            &[],
            Transform::Normal,
            1.0,
            None,
            &[],
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
        }
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
            R::draw_blur_span(frame, &self.texture, src, dst, damage)?;
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
