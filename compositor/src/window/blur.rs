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

use smithay::backend::renderer::element::{Element, Id, Kind, RenderElement};
use smithay::backend::renderer::gles::{
    GlesError, GlesFrame, GlesTexProgram, GlesTexture, Uniform, UniformName, UniformType,
};
use smithay::backend::renderer::glow::{GlowFrame, GlowRenderer};
use smithay::backend::renderer::utils::CommitCounter;
use smithay::backend::renderer::{Renderer, RendererSuper};
use smithay::utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size, Transform};

use crate::window::corner::CornerRadii;

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
/// a plain bilinear sample). `v_coords` is normalized over the sampled texture;
/// the coverage uniforms reconstruct panel-local pixels even when the texture
/// or draw span only covers part of the panel. The shader evaluates a
/// rounded-rect signed distance field. On top of the blurred backdrop it adds:
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

// Geometry is in output physical pixels. With mediump/FP16, squared corner
// distances overflow in the middle of a wide or scaled panel; inf * 0 in the
// faded lens then poisons the sample UV. Keep geometry and UV arithmetic highp.
precision highp float;
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
uniform float flat_band;
uniform float flat_alpha;
uniform float rim_peak;
uniform float rim_half_width;
uniform float rim_tail;
uniform float rim_side;
uniform float top_glow;
uniform float top_glow_height;
uniform float bottom_glow;
uniform float bottom_glow_height;
uniform float corner_peak;
uniform float corner_distance;
uniform float corner_tail;
uniform vec2 panel_uv_offset;
uniform vec2 panel_uv_scale;
uniform vec2 uv_per_px;

#if defined(DEBUG_FLAGS)
uniform float tint;
#endif

float luminance(vec3 c) {
    return dot(c, vec3(0.299, 0.587, 0.114));
}

void main() {
    // `v_coords` is normalized over the entire texture, including when this
    // draw only covers one rounded span. Reconstruct
    // the panel-normalized coordinate so a panel clipped by the output edge
    // still evaluates the same SDF, rim and corner lens.
    vec2 panel_uv = panel_uv_offset + v_coords * panel_uv_scale;
    vec2 p = (panel_uv - 0.5) * glass_size;
    vec2 half_size = max(glass_size * 0.5 - vec2(glass_radius), vec2(0.0));
    vec2 q = abs(p) - half_size;
    vec2 outside = max(q, vec2(0.0));
    float dist = length(outside) + min(max(q.x, q.y), 0.0) - glass_radius;
    float inside = max(-dist, 0.0);

    // Nearest-edge SDF normal for rim lighting. Refraction has its own smooth
    // direction: the SDF normal jumps across interior medial axes.
    float ol = length(outside);
    vec2 rim_normal = (ol > 0.0) ? outside / ol
        : ((q.x > q.y) ? vec2(1.0, 0.0) : vec2(0.0, 1.0));

    vec2 uv = v_coords;
    if (flat_band > 0.5) {
        // Corner-only lens: the distance to the nearest rounded-corner centre
        // drives a bump that peaks 15 pt in and decays by 50 pt. A flat-edge
        // midpoint is far outside the corner field, so it does not bend.
        vec2 corner_delta = abs(p) - half_size;
        // Reject outside the finite support before length/pow/exp. Besides
        // avoiding invalid far-field arithmetic, the broad flat middle needs
        // no corner-lens square root or exponential at all.
        if (corner_peak > 0.0 && max(abs(corner_delta.x), abs(corner_delta.y)) < corner_tail) {
            float corner_distance_px = length(corner_delta);
            if (corner_distance_px < corner_tail) {
                float t = corner_distance_px / max(corner_distance, 1.0);
                float bump = t * t * t * exp(3.0 * (1.0 - t));
                float fade = 1.0 - smoothstep(corner_tail * 0.75, max(corner_tail, 1.0), corner_distance_px);
                // Blend the interior's centre-directed bend into the exact
                // corner normal at the perimeter. Switching directions at
                // ol == 0 causes visible jumps as a fractional-scale corner
                // crosses a pixel centre. Normalize safely, including p == 0.
                vec2 centre_direction = p / max(length(p), 1e-5);
                vec2 edge_direction = outside * sign(p) / max(ol, 1e-5);
                vec2 direction = mix(centre_direction, edge_direction,
                    smoothstep(0.0, max(glass_radius, 1.0), ol));
                direction /= max(length(direction), 1e-5);
                uv -= direction * (corner_peak * bump * fade) * uv_per_px;
            }
        }
    } else {
        // Edge lens: falloff is one at the rim and reaches zero at lens_falloff.
        vec2 normal = (ol > 0.0) ? outside / ol : normalize(p + vec2(1e-5));
        normal = normal * sign(p + vec2(1e-6));
        float lens = clamp(1.0 - inside / max(lens_falloff, 1.0), 0.0, 1.0);
        lens = lens * lens * (3.0 - 2.0 * lens);
        uv -= normal * (refraction * lens) * uv_per_px;
    }

    vec4 color = texture2D(tex, uv);

    if (flat_band > 0.5) {
        // Measured flat band: mix the filtered composite toward the neutral
        // tint level, then add the sharp rim profile. The exponential has its
        // half-amplitude at `rim_half_width` and a ~6 pt tail.
        color.rgb = mix(color.rgb, tint_color, clamp(flat_alpha, 0.0, 1.0));

        // Directional rim: the horizontal (top/bottom) edges catch the light,
        // while the vertical sides sit in a darker outline. `rim_normal` is the
        // outward SDF normal, so abs(rim_normal.y) is 1 on the horizontal edges
        // and 0 on the sides; the corners blend continuously.
        float vertical = abs(rim_normal.y);
        float rim_strength = mix(rim_side, rim_peak, vertical);
        float rim = rim_strength * exp(-0.693147 * inside / max(rim_half_width, 1.0));
        float rim_fade = 1.0 - smoothstep(rim_tail * 0.75, max(rim_tail, 1.0), inside);
        color.rgb += vec3(rim * rim_fade);

        // Interior edge lifts: a narrow top gradient and a slightly wider
        // bottom brightening that fades into the flat centre.
        float from_top = p.y + glass_size.y * 0.5;
        float top = top_glow * (1.0 - smoothstep(0.0, max(top_glow_height, 1.0), from_top));
        float from_bottom = glass_size.y * 0.5 - p.y;
        float bottom = bottom_glow * (1.0 - smoothstep(0.0, max(bottom_glow_height, 1.0), from_bottom));
        color.rgb += vec3(top + bottom);
    } else {
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
    }

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

/// The menu-bar no-blur veil shader (T200).
///
/// The veil samples the **sharp** live scene at the current pixel and blends
/// it toward the scheme's veil colour with a depth-dependent alpha LUT. A
/// bounded 5-tap luminance probe selects the measured background-luminance
/// adaptation; it never filters the displayed backdrop. The LUT is supplied as
/// four `vec4`
/// uniforms (eight depths and eight alphas), so the shader holds no measured
/// literal. `panel_uv_offset/scale` reconstruct the panel-normalized depth
/// even when the drawn span is a slice of the padded material region.
pub const VEIL_SHADER: &str = r#"#version 100

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

uniform vec3 veil_color;
uniform float veil_height;
uniform float veil_strip;
uniform vec2 veil_probe;
uniform vec4 veil_depth_a;
uniform vec4 veil_depth_b;
uniform vec4 veil_alpha_a;
uniform vec4 veil_alpha_b;
uniform vec4 veil_adapt_luma_a;
uniform float veil_adapt_luma_b;
uniform vec4 veil_adapt_scale_a;
uniform float veil_adapt_scale_b;
uniform vec2 panel_uv_offset;
uniform vec2 panel_uv_scale;

float luminance(vec3 c) {
    return dot(c, vec3(0.299, 0.587, 0.114));
}

float adapt_scale(float lum) {
    if (lum <= veil_adapt_luma_a.x)
        return veil_adapt_scale_a.x;
    if (lum <= veil_adapt_luma_a.y)
        return mix(veil_adapt_scale_a.x, veil_adapt_scale_a.y,
                   (lum - veil_adapt_luma_a.x) / max(veil_adapt_luma_a.y - veil_adapt_luma_a.x, 1e-4));
    if (lum <= veil_adapt_luma_a.z)
        return mix(veil_adapt_scale_a.y, veil_adapt_scale_a.z,
                   (lum - veil_adapt_luma_a.y) / max(veil_adapt_luma_a.z - veil_adapt_luma_a.y, 1e-4));
    if (lum <= veil_adapt_luma_a.w)
        return mix(veil_adapt_scale_a.z, veil_adapt_scale_a.w,
                   (lum - veil_adapt_luma_a.z) / max(veil_adapt_luma_a.w - veil_adapt_luma_a.z, 1e-4));
    if (lum <= veil_adapt_luma_b)
        return mix(veil_adapt_scale_a.w, veil_adapt_scale_b,
                   (lum - veil_adapt_luma_a.w) / max(veil_adapt_luma_b - veil_adapt_luma_a.w, 1e-4));
    return veil_adapt_scale_b;
}

float lut_alpha(float depth) {
    if (depth <= veil_depth_a.x)
        return veil_alpha_a.x;
    if (depth <= veil_depth_a.y)
        return mix(veil_alpha_a.x, veil_alpha_a.y,
                   (depth - veil_depth_a.x) / max(veil_depth_a.y - veil_depth_a.x, 1e-4));
    if (depth <= veil_depth_a.z)
        return mix(veil_alpha_a.y, veil_alpha_a.z,
                   (depth - veil_depth_a.y) / max(veil_depth_a.z - veil_depth_a.y, 1e-4));
    if (depth <= veil_depth_a.w)
        return mix(veil_alpha_a.z, veil_alpha_a.w,
                   (depth - veil_depth_a.z) / max(veil_depth_a.w - veil_depth_a.z, 1e-4));
    if (depth <= veil_depth_b.x)
        return mix(veil_alpha_a.w, veil_alpha_b.x,
                   (depth - veil_depth_a.w) / max(veil_depth_b.x - veil_depth_a.w, 1e-4));
    if (depth <= veil_depth_b.y)
        return mix(veil_alpha_b.x, veil_alpha_b.y,
                   (depth - veil_depth_b.x) / max(veil_depth_b.y - veil_depth_b.x, 1e-4));
    if (depth <= veil_depth_b.z)
        return mix(veil_alpha_b.y, veil_alpha_b.z,
                   (depth - veil_depth_b.y) / max(veil_depth_b.z - veil_depth_b.y, 1e-4));
    if (depth <= veil_depth_b.w)
        return mix(veil_alpha_b.z, veil_alpha_b.w,
                   (depth - veil_depth_b.z) / max(veil_depth_b.w - veil_depth_b.z, 1e-4));
    return 0.0;
}

float local_luminance(vec2 uv) {
    vec3 color = texture2D(tex, uv).rgb * 0.4;
    color += texture2D(tex, clamp(uv + vec2(veil_probe.x, 0.0), 0.0, 1.0)).rgb * 0.15;
    color += texture2D(tex, clamp(uv - vec2(veil_probe.x, 0.0), 0.0, 1.0)).rgb * 0.15;
    color += texture2D(tex, clamp(uv + vec2(0.0, veil_probe.y), 0.0, 1.0)).rgb * 0.15;
    color += texture2D(tex, clamp(uv - vec2(0.0, veil_probe.y), 0.0, 1.0)).rgb * 0.15;
    return luminance(color);
}

void main() {
    vec2 panel_uv = panel_uv_offset + v_coords * panel_uv_scale;
    float depth = panel_uv.y * veil_height;
    vec4 base = texture2D(tex, v_coords);
    float lum = local_luminance(v_coords);
    float veil = lut_alpha(depth) * adapt_scale(lum);
    vec3 color = mix(base.rgb, veil_color, veil);

#if defined(NO_ALPHA)
    gl_FragColor = vec4(color, 1.0) * alpha;
#else
    gl_FragColor = vec4(color, base.a) * alpha;
#endif
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
/// T198: flat-band model switch.
pub const GLASS_FLAT_BAND_UNIFORM: &str = "flat_band";
/// T198: flat-band mix fraction toward `tint_color`.
pub const GLASS_FLAT_ALPHA_UNIFORM: &str = "flat_alpha";
/// T198: rim peak over the interior.
pub const GLASS_RIM_PEAK_UNIFORM: &str = "rim_peak";
/// T198: rim half-amplitude width (physical px).
pub const GLASS_RIM_HALF_UNIFORM: &str = "rim_half_width";
/// T198: rim tail (physical px).
pub const GLASS_RIM_TAIL_UNIFORM: &str = "rim_tail";
/// Directional rim addition on the vertical sides (negative darkens).
pub const GLASS_RIM_SIDE_UNIFORM: &str = "rim_side";
/// Additive lift along the top interior edge.
pub const GLASS_TOP_GLOW_UNIFORM: &str = "top_glow";
/// Distance over which the top lift fades into the interior (physical px).
pub const GLASS_TOP_GLOW_HEIGHT_UNIFORM: &str = "top_glow_height";
/// Additive lift along the bottom interior edge.
pub const GLASS_BOTTOM_GLOW_UNIFORM: &str = "bottom_glow";
/// Distance over which the bottom lift fades into the interior (physical px).
pub const GLASS_BOTTOM_GLOW_HEIGHT_UNIFORM: &str = "bottom_glow_height";
/// T198: corner-lens peak displacement (physical px).
pub const GLASS_CORNER_PEAK_UNIFORM: &str = "corner_peak";
/// T198: corner-lens peak distance (physical px).
pub const GLASS_CORNER_DISTANCE_UNIFORM: &str = "corner_distance";
/// T198: corner-lens decay tail (physical px).
pub const GLASS_CORNER_TAIL_UNIFORM: &str = "corner_tail";
/// T198: panel-normalized origin of the visible texture slice.
pub const GLASS_PANEL_UV_OFFSET_UNIFORM: &str = "panel_uv_offset";
/// T198: panel-normalized size of the visible texture slice.
pub const GLASS_PANEL_UV_SCALE_UNIFORM: &str = "panel_uv_scale";
/// T198: texture UV per output physical pixel.
pub const GLASS_UV_PER_PX_UNIFORM: &str = "uv_per_px";

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
pub fn glass_uniform_names() -> [UniformName<'static>; 25] {
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
        UniformName::new(GLASS_FLAT_BAND_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_FLAT_ALPHA_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_RIM_PEAK_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_RIM_HALF_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_RIM_TAIL_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_RIM_SIDE_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_TOP_GLOW_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_TOP_GLOW_HEIGHT_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_BOTTOM_GLOW_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_BOTTOM_GLOW_HEIGHT_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_CORNER_PEAK_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_CORNER_DISTANCE_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_CORNER_TAIL_UNIFORM, UniformType::_1f),
        UniformName::new(GLASS_PANEL_UV_OFFSET_UNIFORM, UniformType::_2f),
        UniformName::new(GLASS_PANEL_UV_SCALE_UNIFORM, UniformType::_2f),
        UniformName::new(GLASS_UV_PER_PX_UNIFORM, UniformType::_2f),
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
    /// T198: a convex mix of the sharp scene and Kawase levels. `None` keeps
    /// the single final-level path every other role uses.
    pub sharp_weights: Option<[f32; 6]>,
    /// Logical sigma for frosted cards, separate from legacy blur radii.
    pub sigma: Option<f32>,
}

impl BlurSpec {
    /// Resolve a spec from a token blur radius and a token corner radius.
    pub fn new(radius: f32, corner_radius: f32) -> Self {
        BlurSpec {
            radius,
            iterations: iterations_for_radius(radius),
            corner_radius,
            sharp_weights: None,
            sigma: None,
        }
    }

    /// Frosted cards use a low-pass kernel calibrated in logical pixels.
    /// Half-weight diagonal taps accumulate variance (4^n - 1) / 6;
    /// compensate their distance for output scale without stretching the card.
    pub fn for_frosted(sigma: f32, corner_radius: f32) -> Self {
        let sigma = sigma.max(0.1);
        Self {
            radius: sigma * 3.0,
            iterations: ((sigma * 6.0f32.sqrt()).log2().ceil() as u32).clamp(1, 5),
            corner_radius,
            sharp_weights: None,
            sigma: Some(sigma),
        }
    }

    pub fn tap_offset(&self, scale: f64) -> f32 {
        self.sigma.map_or(1.0, |sigma| {
            sigma * scale as f32 / ((4.0f32.powi(self.iterations as i32) - 1.0) / 6.0).sqrt()
        })
    }

    /// The T198 Dock fit: five Kawase levels plus the sharp scene composite.
    pub fn for_dock(radius: f32, corner_radius: f32) -> Self {
        BlurSpec {
            radius,
            iterations: crate::window::dock_optics::DOCK_BLUR_ITERATIONS,
            corner_radius,
            sharp_weights: Some(crate::window::dock_optics::DOCK_SCALE_WEIGHTS),
            sigma: None,
        }
    }

    /// Scale the geometry for a degrade tier while preserving the T198
    /// multi-scale levels (re-deriving iterations from the scaled radius would
    /// silently drop the measured shoulder).
    pub fn scaled(self, scale: f32) -> Self {
        if let Some(sigma) = self.sigma {
            Self::for_frosted(sigma * scale, self.corner_radius * scale)
        } else if self.sharp_weights.is_some() {
            BlurSpec {
                radius: self.radius * scale,
                corner_radius: self.corner_radius * scale,
                ..self
            }
        } else {
            BlurSpec::new(self.radius * scale, self.corner_radius * scale)
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

    /// The scene margin the blur needs beyond a panel edge, in logical pixels
    /// (T-21.3).
    ///
    /// Two reaches matter:
    /// * the token [`Self::radius`] is the requested blur spread, so a region
    ///   that stops at the panel edge would clamp the soft edge;
    /// * the Kawase chain's discrete taps reach `2^iterations - 1` source
    ///   pixels, and the first bilinear downsample adds one more texel, so the
    ///   chain footprint is `2^iterations` ([`Self::downsample`]).
    ///
    /// Taking the larger of the two covers both without a literal padding
    /// constant. A degenerate radius still keeps the minimum chain footprint.
    pub fn padding(&self) -> f32 {
        self.radius.max(0.0).max(self.downsample() as f32)
    }

    /// The padding rounded up to whole logical pixels, for the region helper.
    /// Rounding up keeps the full kernel inside the region at the panel edge.
    pub fn padding_px(&self) -> i32 {
        self.padding().ceil() as i32
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
    /// T198: use the measured flat-band + rim + corner-lens model instead of
    /// the legacy adaptive tint / edge lens.
    pub flat_band: bool,
    /// T198: flat-band mix fraction toward `tint_color`.
    pub flat_alpha: f32,
    /// T198: rim peak over the interior.
    pub rim_peak: f32,
    /// T198: rim half-amplitude width in logical pixels.
    pub rim_half_width: f32,
    /// T198: rim tail in logical pixels.
    pub rim_tail: f32,
    /// Directional rim addition on the vertical sides (negative darkens).
    pub rim_side: f32,
    /// Additive lift along the top interior edge.
    pub top_glow: f32,
    /// Distance over which the top lift fades into the interior.
    pub top_glow_height: f32,
    /// Additive lift along the bottom interior edge.
    pub bottom_glow: f32,
    /// Distance over which the bottom lift fades into the interior.
    pub bottom_glow_height: f32,
    /// T198: corner-lens peak displacement in logical pixels.
    pub corner_peak: f32,
    /// T198: corner-lens peak distance from the corner in logical pixels.
    pub corner_distance: f32,
    /// T198: corner-lens decay tail in logical pixels.
    pub corner_tail: f32,
}

impl Default for GlassSpec {
    fn default() -> Self {
        Self {
            refraction: 0.0,
            specular: 0.0,
            specular_width: 0.0,
            tint: 0.0,
            tint_color: [0.0; 3],
            flat_band: false,
            flat_alpha: 0.0,
            rim_peak: 0.0,
            rim_half_width: 0.0,
            rim_tail: 0.0,
            rim_side: 0.0,
            top_glow: 0.0,
            top_glow_height: 0.0,
            bottom_glow: 0.0,
            bottom_glow_height: 0.0,
            corner_peak: 0.0,
            corner_distance: 0.0,
            corner_tail: 0.0,
        }
    }
}

impl GlassSpec {
    /// The measured T198 Dock optics, including the flat band, rim profile and
    /// corner lens. The tint is a neutral sRGB level, not a scheme hue.
    pub fn for_dock(optics: crate::window::dock_optics::DockOptics) -> Self {
        GlassSpec {
            refraction: optics.corner_lens_peak,
            specular: optics.edge,
            specular_width: optics.edge_width,
            tint: optics.alpha,
            tint_color: [optics.tint, optics.tint, optics.tint],
            flat_band: true,
            flat_alpha: optics.alpha,
            rim_peak: optics.edge,
            rim_half_width: optics.edge_width,
            rim_tail: optics.edge_tail,
            rim_side: optics.edge_side,
            top_glow: optics.top_glow,
            top_glow_height: optics.top_glow_height,
            bottom_glow: optics.bottom_glow,
            bottom_glow_height: optics.bottom_glow_height,
            corner_peak: optics.corner_lens_peak,
            corner_distance: optics.corner_lens_distance,
            corner_tail: optics.corner_lens_tail,
        }
    }
}

impl GlassSpec {
    /// The lens falloff width in logical pixels: the token refraction maps to
    /// a band through [`LENS_BAND_FACTOR`].
    pub fn lens_width(&self) -> f32 {
        self.refraction * LENS_BAND_FACTOR
    }

    /// The scene reach the refraction/lens kernel needs beyond the panel edge,
    /// in logical pixels (T197). The shader bends samples inward from the rim
    /// by up to [`Self::lens_width`], so a region that stops at the panel edge
    /// would clamp that read. One extra logical pixel covers the bilinear
    /// footprint of the rim sample.
    pub fn sample_padding_px(&self) -> i32 {
        (self.lens_width().max(0.0).ceil() as i32).saturating_add(1)
    }

    /// The shader uniforms for a panel of `physical` size at output `scale`.
    /// The logical tokens are scaled to physical pixels so the SDF, the lens,
    /// and the specular band are device-pixel consistent. `panel` and
    /// `texture_region` are logical rects: the region is what the entire
    /// texture covers, while the SDF and corner lens use the whole panel.
    /// This mapping is independent of the draw span and texture downsampling.
    pub fn uniforms(
        &self,
        physical: Size<i32, Physical>,
        corner_radius: i32,
        scale: Scale<f64>,
        panel: Rectangle<i32, Logical>,
        texture_region: Rectangle<i32, Logical>,
    ) -> [Uniform<'static>; 25] {
        let w = physical.w.max(1) as f32;
        let h = physical.h.max(1) as f32;
        let sx = scale.x.max(f64::EPSILON) as f32;
        let sy = scale.y.max(f64::EPSILON) as f32;
        let panel_w = panel.size.w.max(1) as f32;
        let panel_h = panel.size.h.max(1) as f32;
        let texture_w = texture_region.size.w.max(1) as f32;
        let texture_h = texture_region.size.h.max(1) as f32;
        let offset_x = (texture_region.loc.x - panel.loc.x) as f32 / panel_w;
        let offset_y = (texture_region.loc.y - panel.loc.y) as f32 / panel_h;
        let uv_scale_x = texture_w / panel_w;
        let uv_scale_y = texture_h / panel_h;
        // A displacement in output physical pixels maps to UV over the whole
        // texture's logical coverage, never over a single rounded draw row.
        let uv_per_px_x = 1.0 / (texture_w * sx);
        let uv_per_px_y = 1.0 / (texture_h * sy);
        [
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
            Uniform::new(GLASS_FLAT_BAND_UNIFORM, if self.flat_band { 1.0 } else { 0.0 }),
            Uniform::new(GLASS_FLAT_ALPHA_UNIFORM, self.flat_alpha),
            Uniform::new(GLASS_RIM_PEAK_UNIFORM, self.rim_peak),
            Uniform::new(GLASS_RIM_HALF_UNIFORM, self.rim_half_width * sx),
            Uniform::new(GLASS_RIM_TAIL_UNIFORM, self.rim_tail * sx),
            Uniform::new(GLASS_RIM_SIDE_UNIFORM, self.rim_side),
            Uniform::new(GLASS_TOP_GLOW_UNIFORM, self.top_glow),
            Uniform::new(GLASS_TOP_GLOW_HEIGHT_UNIFORM, self.top_glow_height * sy),
            Uniform::new(GLASS_BOTTOM_GLOW_UNIFORM, self.bottom_glow),
            Uniform::new(GLASS_BOTTOM_GLOW_HEIGHT_UNIFORM, self.bottom_glow_height * sy),
            Uniform::new(GLASS_CORNER_PEAK_UNIFORM, self.corner_peak * sx),
            Uniform::new(GLASS_CORNER_DISTANCE_UNIFORM, self.corner_distance * sx),
            Uniform::new(GLASS_CORNER_TAIL_UNIFORM, self.corner_tail * sx),
            Uniform::new(GLASS_PANEL_UV_OFFSET_UNIFORM, (offset_x, offset_y)),
            Uniform::new(GLASS_PANEL_UV_SCALE_UNIFORM, (uv_scale_x, uv_scale_y)),
            Uniform::new(GLASS_UV_PER_PX_UNIFORM, (uv_per_px_x, uv_per_px_y)),
        ]
    }
}

/// How one material region samples the scene (T197).
///
/// The filtered path runs the Kawase chain; the sharp path samples the
/// compositor's scene texture directly with no blur allocation. A veil-only
/// region is the sharp path plus its own paint, so T200 can compose it without
/// ever scheduling a blur pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RegionFilter {
    /// The filtered backdrop: Kawase downsample + liquid-glass compose.
    #[default]
    Filtered,
    /// The sharp backdrop: direct scene-texture sample, no blur chain.
    Sharp,
}

impl RegionFilter {
    pub const fn name(self) -> &'static str {
        match self {
            RegionFilter::Filtered => "filtered",
            RegionFilter::Sharp => "sharp",
        }
    }

    /// Whether this region allocates/runs a Kawase chain.
    pub const fn schedules_blur(self) -> bool {
        matches!(self, RegionFilter::Filtered)
    }
}

/// A renderer that can draw a `GlesTexture` into its current frame. The
/// nested backend draws through [`GlowRenderer`]'s borrowed `GlesFrame`; the
/// DRM backend draws through `MultiFrame`'s `AsMut<GlesFrame>`. Keeping the
/// trait independent of `Renderer::TextureId` is what lets the same blur and
/// scene elements serve both paths: `MultiRenderer::TextureId` is
/// `MultiTexture`, not `GlesTexture`.
pub trait TextureRenderer: Renderer {
    /// Draw `src` (texture buffer coordinates) into `dst` (output-local
    /// physical), clipped to `damage`. When `program` is set the material uses
    /// the liquid-glass texture program with `uniforms`; otherwise the built-in
    /// texture program does a plain bilinear sample.
    fn draw_texture_from_to(
        frame: &mut <Self as RendererSuper>::Frame<'_, '_>,
        texture: &GlesTexture,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        program: Option<&GlesTexProgram>,
        uniforms: &[Uniform<'_>],
        alpha: f32,
    ) -> Result<(), Self::Error>;
}

impl TextureRenderer for GlowRenderer {
    fn draw_texture_from_to(
        frame: &mut GlowFrame<'_, '_>,
        texture: &GlesTexture,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        program: Option<&GlesTexProgram>,
        uniforms: &[Uniform<'_>],
        alpha: f32,
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
            alpha,
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
    commit: CommitCounter,
    texture: GlesTexture,
    /// The panel in output-local logical coordinates.
    panel: Rectangle<i32, Logical>,
    /// The logical region the texture covers. For a filtered panel this is
    /// the visible part of the panel; for a sharp/veil region it is the whole
    /// scene region, including the sample padding (T197). The element keeps
    /// the full panel for its rounded spans and only samples the intersection.
    texture_region: Rectangle<i32, Logical>,
    /// The texture's buffer size (the downsampled panel region).
    texture_size: Size<i32, Buffer>,
    /// The panel's per-corner clipping radii in logical pixels (T197).
    corners: CornerRadii,
    scale: Scale<f64>,
    /// The liquid-glass parameters and compiled program (T-20.2), or `None`
    /// for the plain blurred compose (the scene texture, or a driver that
    /// refused the glass shader).
    glass: Option<(GlassSpec, GlesTexProgram)>,
    /// The menu-bar no-blur veil parameters and compiled program (T200).
    /// Mutually exclusive with `glass`; the sharp path uses this to paint the
    /// depth LUT over the live scene.
    veil: Option<(crate::window::menubar_veil::VeilSpec, GlesTexProgram)>,
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
            commit: CommitCounter::default(),
            texture,
            panel,
            texture_region: panel,
            texture_size,
            corners: CornerRadii::uniform(corner_radius),
            scale,
            glass: None,
            veil: None,
        }
    }

    /// Use the stable render-element id of the panel slot (T-21.3).
    ///
    /// The caller owns the id across frames (the panel texture cache creates
    /// it once per stable panel key), so rebuilding this element does not make
    /// Smithay's damage tracker treat an unchanged panel as new damage.
    pub fn with_id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    /// Set the element commit from its panel or scene texture cache.
    pub fn with_commit(mut self, commit: CommitCounter) -> Self {
        self.commit = commit;
        self
    }

    /// Limit the texture to the part of the panel the material region
    /// actually rendered (T-21.3). The element still uses the full `panel` for
    /// its rounded spans, so clipping at an output edge does not invent a new
    /// rounded corner.
    pub fn with_texture_panel(mut self, texture_panel: Rectangle<i32, Logical>) -> Self {
        self.texture_region = texture_panel;
        self
    }

    /// Set the logical region a scene texture covers (T197). The sharp/veil
    /// path passes the whole padded material region; a filtered panel passes
    /// the visible panel slice through [`Self::with_texture_panel`].
    pub fn with_texture_region(mut self, texture_region: Rectangle<i32, Logical>) -> Self {
        self.texture_region = texture_region;
        self
    }

    /// Use per-corner clipping radii (T197). The default from [`Self::new`] is
    /// a uniform radius for the legacy single-rect panel.
    pub fn with_corners(mut self, corners: CornerRadii) -> Self {
        self.corners = corners;
        self
    }

    /// Apply the Tahoe liquid-glass compose (T-20.2) on top of the blurred
    /// backdrop using the compiled [`GLASS_SHADER`] `program`.
    pub fn with_glass(mut self, spec: GlassSpec, program: GlesTexProgram) -> Self {
        self.glass = Some((spec, program));
        self
    }

    /// Apply the menu-bar no-blur veil (T200). The element samples the sharp
    /// scene texture directly; the shader only blends the depth LUT over it.
    pub fn with_veil(
        mut self,
        spec: crate::window::menubar_veil::VeilSpec,
        program: GlesTexProgram,
    ) -> Self {
        self.veil = Some((spec, program));
        self
    }

    /// The rounded spans of the panel at the token/per-region corner radii.
    fn spans(&self) -> Vec<Rectangle<i32, Logical>> {
        crate::window::corner::rounded_rect_spans_with_radii(self.panel, self.corners)
    }
}

impl Element for BackdropBlurElement {
    fn id(&self) -> &Id {
        &self.id
    }

    fn current_commit(&self) -> CommitCounter {
        self.commit
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

impl<R: TextureRenderer> RenderElement<R> for BackdropBlurElement {
    fn draw(
        &self,
        frame: &mut <R as RendererSuper>::Frame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        panel_physical: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        // The Kawase texture is smaller than the panel; each rounded span maps
        // its slice of the panel onto the corresponding slice of the texture.
        // The texture may cover only the visible part of a panel clipped by
        // the material region, so the span is intersected with that part.
        // The glass uniforms describe the entire texture's coverage so the
        // SDF and corner lens stay panel-relative even when drawing a slice.
        let program = self
            .veil
            .as_ref()
            .map(|(_, program)| program)
            .or_else(|| self.glass.as_ref().map(|(_, program)| program));
        // Resolve once per element, rather than per rounded row. Glass has
        // a fixed-size uniform array and needs no heap allocation.
        let veil_uniforms;
        let glass_uniforms;
        let uniforms: &[Uniform<'static>] = if let Some((spec, _)) = self.veil.as_ref() {
            veil_uniforms = spec.uniforms(
                self.scale,
                self.panel,
                self.texture_region,
                self.texture_size,
            );
            &veil_uniforms
        } else if let Some((spec, _)) = self.glass.as_ref() {
            glass_uniforms = spec.uniforms(
                panel_physical.size,
                self.corners.max(),
                self.scale,
                self.panel,
                self.texture_region,
            );
            &glass_uniforms
        } else {
            &[]
        };
        for span in self.spans() {
            let Some((visible, src)) =
                map_span_to_texture(span, self.texture_region, self.texture_size)
            else {
                continue;
            };
            let dst = physical_span(visible, self.panel, panel_physical);
            // Smithay hands the element damage in the element's local
            // coordinates, but `render_texture_from_to` expects it local to
            // `dst`. A rounded panel is a stack of spans whose origins differ
            // (the central band starts one corner-radius below the panel), so
            // passing the panel-local damage through unchanged shifts the
            // scissor and leaves the real damaged strip unfrosted. That read as
            // the raw scene showing through a tile/cursor-sized area (T-21.5).
            let span_damage = damage_in_span(damage, panel_physical, dst);
            if span_damage.is_empty() {
                continue;
            }
            R::draw_texture_from_to(
                frame,
                &self.texture,
                src,
                dst,
                &span_damage,
                program,
                uniforms,
                1.0,
            )?;
        }
        Ok(())
    }
}

/// Round shared span edges to the same physical pixel at fractional scale.
/// Rounding each span's origin and height independently can leave a row of
/// raw background between adjacent spans.
fn physical_span(
    span: Rectangle<i32, Logical>,
    panel: Rectangle<i32, Logical>,
    dst: Rectangle<i32, Physical>,
) -> Rectangle<i32, Physical> {
    let scale = Scale::from((
        f64::from(dst.size.w) / f64::from(panel.size.w),
        f64::from(dst.size.h) / f64::from(panel.size.h),
    ));
    let start = (span.loc - panel.loc).to_physical_precise_round(scale);
    let end = (span.loc + span.size.to_point() - panel.loc).to_physical_precise_round(scale);
    Rectangle::new(dst.loc + start, (end - start).to_size())
}

/// Translate element-local damage into one rounded span's local coordinates.
///
/// `panel` and `span` are output-local physical rects; `damage` is local to
/// `panel`. The returned rects are local to `span` and clipped to its bounds,
/// which is what [`TextureRenderer::draw_texture_from_to`] expects.
fn damage_in_span(
    damage: &[Rectangle<i32, Physical>],
    panel: Rectangle<i32, Physical>,
    span: Rectangle<i32, Physical>,
) -> Vec<Rectangle<i32, Physical>> {
    let offset = Point::from((panel.loc.x - span.loc.x, panel.loc.y - span.loc.y));
    let span_bounds = Rectangle::from_size(span.size);
    damage
        .iter()
        .filter_map(|damage| {
            Rectangle::new(damage.loc + offset, damage.size).intersection(span_bounds)
        })
        .collect()
}

/// Map one rounded span of a panel into the blurred texture.
///
/// `texture_region` is the logical region the texture covers (the visible
/// panel slice for a filtered panel, or the whole padded material region for
/// a sharp/veil sample). The returned logical rectangle is the visible span
/// and the returned buffer rectangle is its source slice. `None` means the
/// span lies outside the rendered part.
fn map_span_to_texture(
    span: Rectangle<i32, Logical>,
    texture_region: Rectangle<i32, Logical>,
    texture_size: Size<i32, Buffer>,
) -> Option<(Rectangle<i32, Logical>, Rectangle<f64, Buffer>)> {
    if texture_region.size.w <= 0 || texture_region.size.h <= 0 {
        return None;
    }
    let visible = span.intersection(texture_region)?;
    if visible.size.w <= 0 || visible.size.h <= 0 {
        return None;
    }
    let ratio_x = texture_size.w as f64 / f64::from(texture_region.size.w);
    let ratio_y = texture_size.h as f64 / f64::from(texture_region.size.h);
    let rel_x = f64::from(visible.loc.x - texture_region.loc.x);
    let rel_y = f64::from(visible.loc.y - texture_region.loc.y);
    let src = Rectangle::new(
        Point::from((rel_x * ratio_x, rel_y * ratio_y)),
        Size::from((
            f64::from(visible.size.w) * ratio_x,
            f64::from(visible.size.h) * ratio_y,
        )),
    );
    Some((visible, src))
}

/// A full-output texture element for the composed scene (P2). The nested
/// backend draws its scene through [`BackdropBlurElement`]; the DRM backend
/// needs an element whose `RenderElement` impl works for `MultiRenderer`,
/// whose `TextureId` is `MultiTexture`. This element therefore draws through
/// [`TextureRenderer`] instead of `TextureRenderElement<GlesTexture>`.
pub struct SceneTextureElement {
    id: Id,
    commit: CommitCounter,
    texture: GlesTexture,
    /// Output-local physical geometry the texture covers.
    geometry: Rectangle<i32, Physical>,
    /// The texture's buffer size.
    texture_size: Size<i32, Buffer>,
}

impl SceneTextureElement {
    pub fn new(
        texture: GlesTexture,
        geometry: Rectangle<i32, Physical>,
        texture_size: Size<i32, Buffer>,
    ) -> Self {
        SceneTextureElement {
            id: Id::new(),
            commit: CommitCounter::default(),
            texture,
            geometry,
            texture_size,
        }
    }

    /// Use the scene cache's stable id and commit (T-21.3). The commit
    /// advances only when the offscreen scene changed, so a settled scene
    /// leaves this full-output element damage-free.
    pub fn with_identity(mut self, id: Id, commit: CommitCounter) -> Self {
        self.id = id;
        self.commit = commit;
        self
    }
}

impl Element for SceneTextureElement {
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
        Rectangle::from_size(self.texture_size.to_f64())
    }

    fn alpha(&self) -> f32 {
        1.0
    }

    fn kind(&self) -> Kind {
        Kind::Unspecified
    }
}

impl<R: TextureRenderer> RenderElement<R> for SceneTextureElement {
    fn draw(
        &self,
        frame: &mut <R as RendererSuper>::Frame<'_, '_>,
        _src: Rectangle<f64, Buffer>,
        _dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        _opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), R::Error> {
        let src = Rectangle::from_size(self.texture_size.to_f64());
        R::draw_texture_from_to(frame, &self.texture, src, self.geometry, damage, None, &[], 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frosting_preserves_logical_sigma_at_fractional_output_scales() {
        let spec = BlurSpec::for_frosted(10.0, 12.5);
        assert_eq!(spec.iterations, 5, "a one-level menu is barely blurred");
        for scale in [1.0, 1.5, 2.0] {
            let variance = (4.0f32.powi(spec.iterations as i32) - 1.0) / 6.0;
            let logical_sigma = variance.sqrt() * spec.tap_offset(scale) / scale as f32;
            assert!((logical_sigma - 10.0).abs() < 0.001);
        }
        assert_eq!(spec.scaled(0.5).sigma, Some(5.0));
        assert_eq!(BlurSpec::new(10.0, 12.5).tap_offset(2.0), 1.0);
    }

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
    fn the_padding_comes_from_the_radius_and_iteration_count() {
        // The token radius is the requested reach; the Kawase chain reaches
        // `2^iterations` source pixels. The padding is the larger of the two,
        // never a literal margin.
        for (radius, iterations, expected) in [
            (0.0, 1, 2.0),
            (8.0, 1, 8.0),
            (12.0, 2, 12.0),
            (24.0, 3, 24.0),
            (28.0, 4, 28.0),
        ] {
            let spec = BlurSpec::new(radius, 10.0);
            assert_eq!(spec.iterations, iterations, "radius {radius}");
            assert_eq!(spec.padding(), expected, "radius {radius}");
            assert_eq!(spec.padding_px(), expected as i32, "radius {radius}");
        }

        // A tiny radius still keeps the minimum one-iteration chain footprint.
        assert_eq!(BlurSpec::new(0.5, 10.0).padding(), 2.0);
        // Fractional padding rounds up so the region never clips the kernel.
        assert_eq!(BlurSpec::new(8.2, 10.0).padding_px(), 9);
        // More blur never means less padding.
        assert!(BlurSpec::new(24.0, 10.0).padding() > BlurSpec::new(8.0, 10.0).padding());
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
            GLASS_RIM_SIDE_UNIFORM,
            GLASS_TOP_GLOW_UNIFORM,
            GLASS_TOP_GLOW_HEIGHT_UNIFORM,
            GLASS_BOTTOM_GLOW_UNIFORM,
            GLASS_BOTTOM_GLOW_HEIGHT_UNIFORM,
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
    fn every_glass_uniform_has_a_compiled_descriptor() {
        // Smithay only exposes the uniforms named in `glass_uniform_names()`.
        // A spec that sets an undeclared uniform fails the whole material
        // frame with `UnknownUniform` and disables the sampled path, so the
        // descriptor list and the resolved uniform list must stay in sync.
        let optics = crate::window::dock_optics::DockOptics::light();
        let spec = GlassSpec::for_dock(optics);
        let panel = Rectangle::new((0, 0).into(), (800, 80).into());
        let uniforms = spec.uniforms(Size::from((800, 80)), 27, Scale::from(1.0), panel, panel);
        let descriptors = glass_uniform_names();
        for uniform in &uniforms {
            assert!(
                descriptors.iter().any(|desc| desc.name == uniform.name),
                "missing compiled descriptor for glass uniform {}",
                uniform.name
            );
        }
        assert_eq!(descriptors.len(), uniforms.len());
    }

    #[test]
    fn the_veil_shader_declares_its_lut_and_probe_uniforms() {
        assert!(VEIL_SHADER.contains("//_DEFINES_"));
        assert!(VEIL_SHADER.contains("varying vec2 v_coords;"));
        assert!(VEIL_SHADER.contains("uniform sampler2D tex;"));
        assert!(VEIL_SHADER.contains("uniform float alpha;"));
        for name in [
            crate::window::menubar_veil::VEIL_COLOR_UNIFORM,
            crate::window::menubar_veil::VEIL_HEIGHT_UNIFORM,
            crate::window::menubar_veil::VEIL_STRIP_UNIFORM,
            crate::window::menubar_veil::VEIL_PROBE_UNIFORM,
            crate::window::menubar_veil::VEIL_ADAPT_LUMA_A_UNIFORM,
            crate::window::menubar_veil::VEIL_ADAPT_LUMA_B_UNIFORM,
            crate::window::menubar_veil::VEIL_ADAPT_SCALE_A_UNIFORM,
            crate::window::menubar_veil::VEIL_ADAPT_SCALE_B_UNIFORM,
            crate::window::menubar_veil::VEIL_DEPTH_A_UNIFORM,
            crate::window::menubar_veil::VEIL_DEPTH_B_UNIFORM,
            crate::window::menubar_veil::VEIL_ALPHA_A_UNIFORM,
            crate::window::menubar_veil::VEIL_ALPHA_B_UNIFORM,
            crate::window::menubar_veil::VEIL_PANEL_UV_OFFSET_UNIFORM,
            crate::window::menubar_veil::VEIL_PANEL_UV_SCALE_UNIFORM,
        ] {
            assert!(VEIL_SHADER.contains(name), "veil shader is missing {name}");
        }
        // The sharp centre sample is separate from the luminance probe; the
        // displayed backdrop is never spatially filtered.
        assert!(VEIL_SHADER.contains("vec4 base = texture2D(tex, v_coords);"));
        assert!(VEIL_SHADER.contains("local_luminance"));
        assert!(VEIL_SHADER.contains("adapt_scale"));
        assert!(VEIL_SHADER.contains("gl_FragColor"));
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
            ..GlassSpec::default()
        };
        // The lens falloff is a derived mapping constant, not a token.
        assert_eq!(spec.lens_width(), 6.0 * LENS_BAND_FACTOR);

        // At scale 2 the logical tokens become physical pixels; the corner
        // radius is scaled with them so the SDF is device-pixel consistent.
        let panel = Rectangle::new((0, 0).into(), (800, 28).into());
        let uniforms = spec.uniforms(
            Size::from((1600, 56)),
            20,
            Scale::from(2.0),
            panel,
            panel,
        );
        assert_eq!(uniforms.len(), 25);
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
        let empty = Rectangle::new((0, 0).into(), (1, 1).into());
        let uniforms = spec.uniforms(Size::from((0, 0)), 0, Scale::from(1.0), empty, empty);
        assert_eq!(uniforms[0].value, UniformValue::_2f(1.0, 1.0));
    }

    #[test]
    fn the_dock_blur_spec_keeps_its_measured_multi_scale_levels() {
        let spec = BlurSpec::for_dock(3.86, 27.0);
        assert_eq!(spec.iterations, crate::window::dock_optics::DOCK_BLUR_ITERATIONS);
        assert_eq!(spec.sharp_weights, Some(crate::window::dock_optics::DOCK_SCALE_WEIGHTS));
        // A degrade tier scales geometry but must not re-derive the chain and
        // drop the broad level.
        let reduced = spec.scaled(0.5);
        assert_eq!(reduced.iterations, spec.iterations);
        assert_eq!(reduced.sharp_weights, spec.sharp_weights);
        assert_eq!(reduced.radius, 1.93);
        assert_eq!(reduced.corner_radius, 13.5);
        // Every other role keeps the token-derived iteration count.
        let legacy = BlurSpec::new(3.86, 27.0);
        assert_eq!(legacy.sharp_weights, None);
        assert_eq!(legacy.iterations, 1);
    }

    #[test]
    fn the_dock_glass_spec_carries_the_flat_band_rim_and_corner_lens() {
        let optics = crate::window::dock_optics::DockOptics::light();
        let spec = GlassSpec::for_dock(optics);
        assert!(spec.flat_band);
        assert_eq!(spec.flat_alpha, optics.alpha);
        assert_eq!(spec.rim_peak, optics.edge);
        assert_eq!(spec.rim_half_width, optics.edge_width);
        assert_eq!(spec.rim_side, optics.edge_side);
        assert_eq!(spec.top_glow, optics.top_glow);
        assert_eq!(spec.top_glow_height, optics.top_glow_height);
        assert_eq!(spec.bottom_glow, optics.bottom_glow);
        assert_eq!(spec.bottom_glow_height, optics.bottom_glow_height);
        assert_eq!(spec.corner_peak, optics.corner_lens_peak);
        assert_eq!(spec.corner_distance, optics.corner_lens_distance);
        assert_eq!(spec.tint_color, [optics.tint; 3]);
        let panel = Rectangle::new((10, 20).into(), (400, 80).into());
        let uniforms = spec.uniforms(
            Size::from((800, 160)),
            27,
            Scale::from(2.0),
            panel,
            panel,
        );
        // The flat-band switch is set and the corner lens is in physical px.
        assert_eq!(
            uniforms[9].value,
            smithay::backend::renderer::gles::UniformValue::_1f(1.0)
        );
        assert_eq!(
            uniforms[19].value,
            smithay::backend::renderer::gles::UniformValue::_1f(19.2)
        );
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

    #[test]
    fn a_region_keeps_attached_top_corners_square_and_rounds_the_bottom() {
        use crate::window::corner::{rounded_rect_spans_with_radii, CornerRadii};
        let panel = Rectangle::new(Point::from((100, 20)), Size::from((800, 120)));
        let attached = CornerRadii {
            top_left: 0,
            top_right: 0,
            bottom_right: 12,
            bottom_left: 12,
        };
        let spans = rounded_rect_spans_with_radii(panel, attached);
        assert_eq!(spans.first().unwrap().size.w, panel.size.w);
        assert_eq!(spans.last().unwrap().loc.y, panel.loc.y + panel.size.h - 1);
        assert!(spans.last().unwrap().size.w < panel.size.w);

        let detached = CornerRadii::uniform(12);
        let spans = rounded_rect_spans_with_radii(panel, detached);
        assert!(spans.first().unwrap().size.w < panel.size.w);
        assert!(spans.last().unwrap().size.w < panel.size.w);
    }

    #[test]
    fn a_sharp_region_maps_the_whole_scene_region_without_a_blur_chain() {
        // A veil/sharp element covers the padded scene region, not just the
        // panel slice. The span source is translated into that region.
        let region = Rectangle::new(Point::from((40, 0)), Size::from((400, 200)));
        let panel = Rectangle::new(Point::from((100, 20)), Size::from((200, 80)));
        let texture = Size::from((400, 200));
        let (visible, src) =
            map_span_to_texture(panel, region, texture).expect("visible sharp span");
        assert_eq!(visible, panel);
        assert_eq!(src.loc, Point::from((60.0, 20.0)));
        assert_eq!(src.size, Size::from((200.0, 80.0)));
    }

    #[test]
    fn a_span_maps_its_slice_of_the_panel_to_the_blur_texture() {
        let panel = Rectangle::new(Point::from((100, 20)), Size::from((800, 40)));
        let texture = Size::from((200, 10));

        // Unclipped: the full panel span maps to the full texture.
        let (visible, src) = map_span_to_texture(panel, panel, texture).expect("unclipped span");
        assert_eq!(visible, panel);
        assert_eq!(src.loc, Point::from((0.0, 0.0)));
        assert_eq!(src.size, Size::from((200.0, 10.0)));

        // A half-panel span maps to half the texture.
        let half = Rectangle::new(Point::from((500, 20)), Size::from((400, 40)));
        let (visible, src) = map_span_to_texture(half, panel, texture).expect("half span");
        assert_eq!(visible, half);
        assert_eq!(src.loc, Point::from((100.0, 0.0)));
        assert_eq!(src.size, Size::from((100.0, 10.0)));

        // A texture panel clipped at an output edge only covers the visible
        // part; the span is clipped and the source is offset into the texture.
        let clipped_panel = Rectangle::new(Point::from((100, 20)), Size::from((600, 40)));
        let (visible, src) =
            map_span_to_texture(panel, clipped_panel, texture).expect("clipped span");
        assert_eq!(
            visible,
            Rectangle::new(Point::from((100, 20)), Size::from((600, 40)))
        );
        assert_eq!(src.loc, Point::from((0.0, 0.0)));
        assert_eq!(src.size, Size::from((200.0, 10.0)));

        // A span entirely outside the rendered part draws nothing.
        let outside = Rectangle::new(Point::from((800, 20)), Size::from((50, 40)));
        assert_eq!(map_span_to_texture(outside, clipped_panel, texture), None);
    }

    #[test]
    fn rounded_spans_share_pixel_edges_at_fractional_scale() {
        let panel = Rectangle::new((101, 201).into(), (201, 81).into());
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let dst = panel.to_physical_precise_round(Scale::from(scale));
            let mut next_y = dst.loc.y;
            for span in crate::window::corner::rounded_rect_spans(
                panel,
                14,
                crate::window::corner::RoundedCorners::All,
            ) {
                let physical = physical_span(span, panel, dst);
                assert_eq!(physical.loc.y, next_y, "scale {scale}");
                assert!(physical.size.h > 0);
                next_y += physical.size.h;
            }
            assert_eq!(next_y, dst.loc.y + dst.size.h);
        }
    }

    #[test]
    fn glass_coordinates_follow_texture_coverage_across_rounded_spans() {
        use smithay::backend::renderer::gles::UniformValue;

        let panel = Rectangle::new((100, 200).into(), (483, 78).into());
        let clipped = Rectangle::new((120, 210).into(), (400, 60).into());
        let spec = GlassSpec::for_dock(crate::window::dock_optics::DockOptics::light());
        for scale in [1.0, 1.5, 2.0] {
            for region in [panel, clipped] {
                let uniforms = spec.uniforms(
                    panel.to_physical_precise_round(Scale::from(scale)).size,
                    27,
                    Scale::from(scale),
                    panel,
                    region,
                );
                let pair =
                    |name: &str| match uniforms.iter().find(|u| u.name == name).unwrap().value {
                        UniformValue::_2f(x, y) => (f64::from(x), f64::from(y)),
                        _ => panic!("expected vec2 for {name}"),
                    };
                let offset = pair(GLASS_PANEL_UV_OFFSET_UNIFORM);
                let uv_scale = pair(GLASS_PANEL_UV_SCALE_UNIFORM);
                let per_px = pair(GLASS_UV_PER_PX_UNIFORM);
                assert!((per_px.0 * f64::from(region.size.w) * scale - 1.0).abs() < 1e-6);
                assert!((per_px.1 * f64::from(region.size.h) * scale - 1.0).abs() < 1e-6);

                // Changing the texture resolution (Kawase downsampling) or
                // the rounded draw span must not change panel coordinates.
                for divisor in [1, 4] {
                    let texture = Size::from((region.size.w / divisor, region.size.h / divisor));
                    for span in crate::window::corner::rounded_rect_spans_with_radii(
                        panel,
                        CornerRadii::uniform(27),
                    ) {
                        let Some((visible, src)) = map_span_to_texture(span, region, texture)
                        else {
                            continue;
                        };
                        for fraction in [0.0, 0.5, 1.0] {
                            let u = (src.loc.x + src.size.w * fraction) / f64::from(texture.w);
                            let v = (src.loc.y + src.size.h * fraction) / f64::from(texture.h);
                            let expected_x = (f64::from(visible.loc.x - panel.loc.x)
                                + f64::from(visible.size.w) * fraction)
                                / f64::from(panel.size.w);
                            let expected_y = (f64::from(visible.loc.y - panel.loc.y)
                                + f64::from(visible.size.h) * fraction)
                                / f64::from(panel.size.h);
                            assert!((offset.0 + u * uv_scale.0 - expected_x).abs() < 1e-6);
                            assert!((offset.1 + v * uv_scale.1 - expected_y).abs() < 1e-6);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn panel_damage_is_translated_into_each_rounded_span() {
        // A 200x80 panel with radius 14: the central span starts at y=214,
        // one radius below the panel origin (200).
        let panel = Rectangle::new(Point::from((100, 200)), Size::from((200, 80)));
        let central = Rectangle::new(Point::from((100, 214)), Size::from((200, 52)));

        // Damage local to the panel near the cursor/tile area.
        let damage = [Rectangle::new(Point::from((10, 20)), Size::from((20, 10)))];
        assert_eq!(
            damage_in_span(&damage, panel, central),
            vec![Rectangle::new(Point::from((10, 6)), Size::from((20, 10)))],
            "the span scissor must be shifted by the panel/span origin delta"
        );

        // Damage outside the span is clipped away rather than drawn at the
        // wrong place.
        let outside = [Rectangle::new(Point::from((10, 0)), Size::from((20, 8)))];
        assert!(damage_in_span(&outside, panel, central).is_empty());

        // Full-panel damage still covers the central span exactly.
        let full = [Rectangle::from_size(panel.size)];
        assert_eq!(
            damage_in_span(&full, panel, central),
            vec![Rectangle::from_size(central.size)]
        );
    }
}
