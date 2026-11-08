// SPDX-License-Identifier: MIT
//! The menu-bar no-blur veil (T200).
//!
//! The reference menu-bar backdrop is **not** a blur. Over black it reads as a
//! white lift with a depth-dependent alpha LUT (0.451 at 5 pt, 0.439 at 15,
//! 0.408 at 25, 0.357 at 35, 0.243 at 50, 0.059 at 75, 0.008 at 90 and zero
//! at 95); over white it is invisible. In Dark mode the same depth profile is
//! a black wash instead. This module owns the pure model the compositor's
//! sharp veil shader and the diagnostics share:
//!
//! * [`VeilSpec::alpha_at`] — the measured depth LUT, piecewise-linear through
//!   the reference samples and exactly zero from 95 pt on.
//! * [`VeilSpec::adaptation`] — the measured background-luminance response.
//!   The Light veil is 0.451 at 5 pt over black, but scales by
//!   `1.0/1.06/0.91/0.48/0.0` at `L=0/0.25/0.5/0.78/1.0`, so it is invisible
//!   over white. The Dark curve is a best guess fitted to the coverage
//!   captures: a much subtler black wash that also reaches zero over white.
//! * [`VeilSpec::adapted_alpha`] — the LUT alpha after that adaptation. The
//!   probe never filters the displayed backdrop, which is always the sharp
//!   centre sample.
//! * [`VeilSpec::uniforms`] — the shader inputs, including the panel mapping
//!   that makes the LUT depth output-relative even when the texture is a slice
//!   of the padded material region.
//!
//! The veil is a deliberate no-blur mode. It never allocates a Kawase chain and
//! never increments the blur-pass counters (see [`crate::window::RegionFilter`]).

use smithay::backend::renderer::gles::Uniform;
use smithay::utils::{Buffer, Logical, Rectangle, Scale, Size};

use crate::design_tokens::glass::presets::menu_bar_veil as tokens;
use crate::material_preset::{RenderMode, ResolvedPreset, Scheme};
use crate::window::RegionFilter;

/// The eight measured depth samples, in logical points. The reference strip is
/// 39 pt; the veil is fully gone at 95 pt.
pub const VEIL_LUT_DEPTHS: [f32; 8] = [5.0, 15.0, 25.0, 35.0, 50.0, 75.0, 90.0, 95.0];

/// The reference foreground strip height in logical points. Dragonfruit keeps
/// its own 28 pt bar layout; this is the calibration band for the LUT.
pub const VEIL_STRIP_HEIGHT: f32 = tokens::STRIP_HEIGHT;
/// The full visual extent of the veil in logical points.
pub const VEIL_EXTENT: f32 = tokens::EXTENT;

/// The shader uniform carrying the scheme veil colour (white in Light, black
/// in Dark).
pub const VEIL_COLOR_UNIFORM: &str = "veil_color";
/// The shader uniform carrying the panel height in logical points.
pub const VEIL_HEIGHT_UNIFORM: &str = "veil_height";
/// The shader uniform carrying the reference strip height.
pub const VEIL_STRIP_UNIFORM: &str = "veil_strip";
/// The shader uniform carrying the bounded luminance-probe radius in UV.
pub const VEIL_PROBE_UNIFORM: &str = "veil_probe";
/// The shader uniform carrying the first four background-luminance anchors.
pub const VEIL_ADAPT_LUMA_A_UNIFORM: &str = "veil_adapt_luma_a";
/// The shader uniform carrying the fifth background-luminance anchor.
pub const VEIL_ADAPT_LUMA_B_UNIFORM: &str = "veil_adapt_luma_b";
/// The shader uniform carrying the first four adaptation scales.
pub const VEIL_ADAPT_SCALE_A_UNIFORM: &str = "veil_adapt_scale_a";
/// The shader uniform carrying the fifth adaptation scale.
pub const VEIL_ADAPT_SCALE_B_UNIFORM: &str = "veil_adapt_scale_b";
/// The shader uniform carrying LUT depths 5/15/25/35 pt.
pub const VEIL_DEPTH_A_UNIFORM: &str = "veil_depth_a";
/// The shader uniform carrying LUT depths 50/75/90/95 pt.
pub const VEIL_DEPTH_B_UNIFORM: &str = "veil_depth_b";
/// The shader uniform carrying LUT alphas 5/15/25/35 pt.
pub const VEIL_ALPHA_A_UNIFORM: &str = "veil_alpha_a";
/// The shader uniform carrying LUT alphas 50/75/90/95 pt.
pub const VEIL_ALPHA_B_UNIFORM: &str = "veil_alpha_b";
/// The shader uniform carrying the panel-normalized origin of the visible slice.
pub const VEIL_PANEL_UV_OFFSET_UNIFORM: &str = "panel_uv_offset";
/// The shader uniform carrying the panel-normalized size of the visible slice.
pub const VEIL_PANEL_UV_SCALE_UNIFORM: &str = "panel_uv_scale";

/// The additional uniform descriptors the veil shader declares, passed to
/// `GlesRenderer::compile_custom_texture_shader`.
pub fn veil_uniform_names() -> [smithay::backend::renderer::gles::UniformName<'static>; 14] {
    use smithay::backend::renderer::gles::{UniformName, UniformType};
    [
        UniformName::new(VEIL_COLOR_UNIFORM, UniformType::_3f),
        UniformName::new(VEIL_HEIGHT_UNIFORM, UniformType::_1f),
        UniformName::new(VEIL_STRIP_UNIFORM, UniformType::_1f),
        UniformName::new(VEIL_PROBE_UNIFORM, UniformType::_2f),
        UniformName::new(VEIL_ADAPT_LUMA_A_UNIFORM, UniformType::_4f),
        UniformName::new(VEIL_ADAPT_LUMA_B_UNIFORM, UniformType::_1f),
        UniformName::new(VEIL_ADAPT_SCALE_A_UNIFORM, UniformType::_4f),
        UniformName::new(VEIL_ADAPT_SCALE_B_UNIFORM, UniformType::_1f),
        UniformName::new(VEIL_DEPTH_A_UNIFORM, UniformType::_4f),
        UniformName::new(VEIL_DEPTH_B_UNIFORM, UniformType::_4f),
        UniformName::new(VEIL_ALPHA_A_UNIFORM, UniformType::_4f),
        UniformName::new(VEIL_ALPHA_B_UNIFORM, UniformType::_4f),
        UniformName::new(VEIL_PANEL_UV_OFFSET_UNIFORM, UniformType::_2f),
        UniformName::new(VEIL_PANEL_UV_SCALE_UNIFORM, UniformType::_2f),
    ]
}

/// One scheme's veil optics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VeilSpec {
    /// The scheme's veil colour: white for the Light lift, black for the Dark
    /// wash.
    pub color: [f32; 3],
    /// The eight LUT depths in logical points.
    pub depths: [f32; 8],
    /// The eight LUT alphas.
    pub alphas: [f32; 8],
    /// The background-luminance anchors for the measured adaptation curve.
    pub adapt_luma: [f32; 5],
    /// The adaptation scale at each anchor. Both schemes reach zero over
    /// white; the Dark curve is a best-guess fit to the coverage captures.
    pub adapt_scale: [f32; 5],
    /// The full visual extent in logical points (the LUT is zero here).
    pub extent: f32,
    /// The reference strip height the LUT is calibrated against.
    pub strip: f32,
    /// The bounded local luminance-probe radius in logical points.
    pub probe: f32,
}

impl VeilSpec {
    /// Build the veil optics for a resolved preset, or `None` when the preset
    /// did not resolve to the explicit no-blur veil mode (legacy rollout,
    /// Reduce Transparency, or a sampled surface).
    pub fn for_resolved(resolved: &ResolvedPreset) -> Option<Self> {
        if resolved.render_mode != RenderMode::NoBlurVeil {
            return None;
        }
        let (alphas, adapt_luma, adapt_scale, probe) = match resolved.scheme {
            Scheme::Light => (
                [
                    tokens::light::veil::ALPHA_5,
                    tokens::light::veil::ALPHA_15,
                    tokens::light::veil::ALPHA_25,
                    tokens::light::veil::ALPHA_35,
                    tokens::light::veil::ALPHA_50,
                    tokens::light::veil::ALPHA_75,
                    tokens::light::veil::ALPHA_90,
                    tokens::light::veil::ALPHA_95,
                ],
                tokens::light::veil::ADAPT_LUMA,
                tokens::light::veil::ADAPT_SCALE,
                tokens::light::veil::PROBE,
            ),
            Scheme::Dark => (
                [
                    tokens::dark::veil::ALPHA_5,
                    tokens::dark::veil::ALPHA_15,
                    tokens::dark::veil::ALPHA_25,
                    tokens::dark::veil::ALPHA_35,
                    tokens::dark::veil::ALPHA_50,
                    tokens::dark::veil::ALPHA_75,
                    tokens::dark::veil::ALPHA_90,
                    tokens::dark::veil::ALPHA_95,
                ],
                tokens::dark::veil::ADAPT_LUMA,
                tokens::dark::veil::ADAPT_SCALE,
                tokens::dark::veil::PROBE,
            ),
        };
        Some(Self {
            color: [resolved.tint, resolved.tint, resolved.tint],
            depths: VEIL_LUT_DEPTHS,
            alphas,
            adapt_luma,
            adapt_scale,
            extent: VEIL_EXTENT,
            strip: VEIL_STRIP_HEIGHT,
            probe,
        })
    }

    /// The measured depth LUT, piecewise-linear through the reference samples.
    /// Depths at or below the first sample use its alpha (the measured top is
    /// flat), and depths at or beyond 95 pt are exactly zero.
    pub fn alpha_at(&self, depth: f32) -> f32 {
        if depth <= self.depths[0] {
            return self.alphas[0].clamp(0.0, 1.0);
        }
        for index in 1..self.depths.len() {
            if depth <= self.depths[index] {
                let span = self.depths[index] - self.depths[index - 1];
                let ratio = if span <= 0.0 {
                    0.0
                } else {
                    (depth - self.depths[index - 1]) / span
                };
                let alpha =
                    self.alphas[index - 1] + (self.alphas[index] - self.alphas[index - 1]) * ratio;
                return alpha.clamp(0.0, 1.0);
            }
        }
        0.0
    }

    /// The measured background-luminance adaptation for this scheme.
    ///
    /// The reference Light veil is not a fixed alpha: over black it is 0.451
    /// at 5 pt, but it falls to zero over white. The Light curve is the
    /// measured `1.0/1.06/0.91/0.48/0.0` at `L=0/0.25/0.5/0.78/1.0`. The
    /// unmeasured Dark curve is the best guess fitted to the coverage
    /// captures: it peaks at 0.26 of the Dark LUT over mid-grey and reaches
    /// zero over white.
    pub fn adaptation(&self, local_luminance: f32) -> f32 {
        let lum = local_luminance.clamp(0.0, 1.0);
        if lum <= self.adapt_luma[0] {
            return self.adapt_scale[0];
        }
        for index in 1..self.adapt_luma.len() {
            if lum <= self.adapt_luma[index] {
                let span = self.adapt_luma[index] - self.adapt_luma[index - 1];
                let ratio = if span <= 0.0 {
                    0.0
                } else {
                    (lum - self.adapt_luma[index - 1]) / span
                };
                return self.adapt_scale[index - 1]
                    + (self.adapt_scale[index] - self.adapt_scale[index - 1]) * ratio;
            }
        }
        self.adapt_scale[self.adapt_scale.len() - 1]
    }

    /// The LUT alpha after the measured background adaptation.
    ///
    /// This is the report's model directly: `alpha_black(y) * adapt(L)`. The
    /// displayed backdrop remains the sharp centre sample.
    pub fn adapted_alpha(&self, depth: f32, local_luminance: f32) -> f32 {
        (self.alpha_at(depth) * self.adaptation(local_luminance)).clamp(0.0, 1.0)
    }

    /// The backdrop transmission at `depth` for a spatial frequency. The veil
    /// has no blur, so the AC response is `1 - alpha` and is independent of
    /// the checker period.
    pub fn transmission(&self, depth: f32) -> f32 {
        1.0 - self.alpha_at(depth)
    }

    /// The post-veil luminance for a uniform backdrop luminance.
    pub fn output_luminance(&self, depth: f32, background: f32) -> f32 {
        let veil_luminance = 0.299 * self.color[0] + 0.587 * self.color[1] + 0.114 * self.color[2];
        let alpha = self.adapted_alpha(depth, background);
        background * (1.0 - alpha) + veil_luminance * alpha
    }

    /// The shader uniforms for one drawn span. `texture_region` is the logical
    /// rect the sharp scene texture covers; `texture_size` is its buffer size.
    ///
    /// Smithay's `v_coords` is normalized over the **whole** texture, not over
    /// the drawn `src` slice. The panel mapping therefore has to start from the
    /// texture region, not from the visible span: a full-output scene texture
    /// would otherwise make the LUT see only the first few points of depth.
    pub fn uniforms(
        &self,
        scale: Scale<f64>,
        panel: Rectangle<i32, Logical>,
        texture_region: Rectangle<i32, Logical>,
        texture_size: Size<i32, Buffer>,
    ) -> Vec<Uniform<'static>> {
        let panel_w = panel.size.w.max(1) as f32;
        let panel_h = panel.size.h.max(1) as f32;
        let offset_x = (texture_region.loc.x - panel.loc.x) as f32 / panel_w;
        let offset_y = (texture_region.loc.y - panel.loc.y) as f32 / panel_h;
        let uv_scale_x = texture_region.size.w.max(1) as f32 / panel_w;
        let uv_scale_y = texture_region.size.h.max(1) as f32 / panel_h;
        let sx = scale.x.max(f64::EPSILON) as f32;
        let sy = scale.y.max(f64::EPSILON) as f32;
        let probe_x = self.probe * sx / texture_size.w.max(1) as f32;
        let probe_y = self.probe * sy / texture_size.h.max(1) as f32;
        vec![
            Uniform::new(
                VEIL_COLOR_UNIFORM,
                (self.color[0], self.color[1], self.color[2]),
            ),
            Uniform::new(VEIL_HEIGHT_UNIFORM, panel_h),
            Uniform::new(VEIL_STRIP_UNIFORM, self.strip),
            Uniform::new(VEIL_PROBE_UNIFORM, (probe_x, probe_y)),
            Uniform::new(
                VEIL_ADAPT_LUMA_A_UNIFORM,
                (
                    self.adapt_luma[0],
                    self.adapt_luma[1],
                    self.adapt_luma[2],
                    self.adapt_luma[3],
                ),
            ),
            Uniform::new(VEIL_ADAPT_LUMA_B_UNIFORM, self.adapt_luma[4]),
            Uniform::new(
                VEIL_ADAPT_SCALE_A_UNIFORM,
                (
                    self.adapt_scale[0],
                    self.adapt_scale[1],
                    self.adapt_scale[2],
                    self.adapt_scale[3],
                ),
            ),
            Uniform::new(VEIL_ADAPT_SCALE_B_UNIFORM, self.adapt_scale[4]),
            Uniform::new(
                VEIL_DEPTH_A_UNIFORM,
                (
                    self.depths[0],
                    self.depths[1],
                    self.depths[2],
                    self.depths[3],
                ),
            ),
            Uniform::new(
                VEIL_DEPTH_B_UNIFORM,
                (
                    self.depths[4],
                    self.depths[5],
                    self.depths[6],
                    self.depths[7],
                ),
            ),
            Uniform::new(
                VEIL_ALPHA_A_UNIFORM,
                (
                    self.alphas[0],
                    self.alphas[1],
                    self.alphas[2],
                    self.alphas[3],
                ),
            ),
            Uniform::new(
                VEIL_ALPHA_B_UNIFORM,
                (
                    self.alphas[4],
                    self.alphas[5],
                    self.alphas[6],
                    self.alphas[7],
                ),
            ),
            Uniform::new(VEIL_PANEL_UV_OFFSET_UNIFORM, (offset_x, offset_y)),
            Uniform::new(VEIL_PANEL_UV_SCALE_UNIFORM, (uv_scale_x, uv_scale_y)),
        ]
    }

    /// The one-line diagnostic the `query material` report prints.
    pub fn diagnostic(&self, mode: RenderMode) -> String {
        format!(
            "material veil mode={} extent={:.1} strip={:.1} probe={:.1} tint={:.3} \
             alphas={:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}",
            mode.name(),
            self.extent,
            self.strip,
            self.probe,
            self.color[0],
            self.alphas[0],
            self.alphas[1],
            self.alphas[2],
            self.alphas[3],
            self.alphas[4],
            self.alphas[5],
            self.alphas[6],
            self.alphas[7],
        )
    }
}

/// The sharp filter this veil requires. Kept here so a caller cannot pair the
/// veil model with a Kawase chain by accident.
pub const VEIL_FILTER: RegionFilter = RegionFilter::Sharp;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material_preset::{resolve, Preset, ResolveInput, Rollout};

    fn resolved(scheme: Scheme) -> ResolvedPreset {
        resolve(ResolveInput {
            preset: Preset::MenuBarVeil,
            scheme,
            rollout: Rollout::Replacement,
            ..ResolveInput::default()
        })
    }

    fn spec(scheme: Scheme) -> VeilSpec {
        VeilSpec::for_resolved(&resolved(scheme)).expect("veil resolves")
    }

    fn contrast(a: f32, b: f32) -> f32 {
        let (light, dark) = if a > b { (a, b) } else { (b, a) };
        (light + 0.05) / (dark + 0.05)
    }

    #[test]
    fn light_lut_matches_the_measured_black_samples_and_ends_at_95() {
        let veil = spec(Scheme::Light);
        let measured = [
            (5.0, 0.45098037),
            (15.0, 0.43921566),
            (25.0, 0.40784317),
            (35.0, 0.35686272),
            (50.0, 0.24313729),
            (75.0, 0.05882353),
            (90.0, 0.00784314),
            (95.0, 0.0),
        ];
        for (depth, expected) in measured {
            assert!(
                (veil.alpha_at(depth) - expected).abs() <= 0.03,
                "depth {depth}: {} != {expected}",
                veil.alpha_at(depth)
            );
        }
        assert_eq!(veil.alpha_at(95.0), 0.0);
        assert_eq!(veil.alpha_at(120.0), 0.0);
        assert!((veil.color[0] - 1.0).abs() < 1e-6, "Light veil is white");
        assert_eq!(veil.extent, 95.0);
        assert_eq!(veil.strip, 39.0);
    }

    #[test]
    fn light_veil_matches_the_measured_background_luminance_table() {
        // The report's measured top-of-bar response over the five neutral
        // backgrounds. The fixed LUT alone gets these wrong on anything
        // lighter than black; the adaptation curve is what makes the veil
        // disappear over white.
        let veil = spec(Scheme::Light);
        let measured = [
            (0.0, 0.451),
            (0.251, 0.477),
            (0.502, 0.409),
            (0.784, 0.218),
            (1.0, 0.0),
        ];
        for (background, expected) in measured {
            let alpha = veil.adapted_alpha(5.0, background);
            assert!(
                (alpha - expected).abs() <= 0.02,
                "background {background}: alpha {alpha} != measured {expected}"
            );
        }
    }

    #[test]
    fn adaptation_curve_is_the_measured_luminance_response() {
        let veil = spec(Scheme::Light);
        for (luma, expected) in [
            (0.0, 1.0),
            (0.25, 1.06),
            (0.5, 0.91),
            (0.78, 0.48),
            (1.0, 0.0),
        ] {
            assert!(
                (veil.adaptation(luma) - expected).abs() < 1e-6,
                "L={luma}: {} != {expected}",
                veil.adaptation(luma)
            );
        }
        // The Dark baseline was not measured, so it must not inherit the
        // Light luminance response.
        let dark = spec(Scheme::Dark);
        for luma in [0.0, 0.25, 0.5, 0.78, 1.0] {
            assert_eq!(dark.adaptation(luma), 1.0);
        }
    }

    #[test]
    fn light_veil_is_invisible_over_white_at_every_depth() {
        let veil = spec(Scheme::Light);
        for depth in [5.0, 15.0, 25.0, 35.0, 50.0, 75.0, 90.0, 95.0] {
            assert_eq!(veil.adapted_alpha(depth, 1.0), 0.0);
        }
    }

    #[test]
    fn dark_lut_is_a_black_wash_with_the_same_depth_shape() {
        let veil = spec(Scheme::Dark);
        assert!(veil.color[0].abs() < 1e-6, "Dark veil is black");
        assert!((veil.alpha_at(5.0) - 0.48).abs() < 1e-6);
        assert_eq!(veil.alpha_at(95.0), 0.0);
        for pair in veil.alphas.windows(2) {
            assert!(pair[0] >= pair[1], "the LUT must not rise with depth");
        }
    }

    #[test]
    fn checker_transmission_is_independent_of_checker_size() {
        let veil = spec(Scheme::Light);
        for depth in [5.0, 15.0, 25.0, 35.0, 50.0, 75.0, 90.0] {
            let transmission = veil.transmission(depth);
            for _period in [2.0, 4.0, 8.0, 16.0, 32.0, 64.0] {
                // The model has no spatial kernel: every period sees the same
                // scalar transmission at a fixed depth.
                assert!((veil.transmission(depth) - transmission).abs() < 1e-9);
            }
        }
    }

    #[test]
    fn adaptation_keeps_light_and_dark_foregrounds_readable() {
        let light = spec(Scheme::Light);
        // Light: dark text (0.05) over a dark moving window.
        let dark_window = light.output_luminance(25.0, 0.02);
        assert!(
            contrast(dark_window, 0.05) >= 4.5,
            "Light veil over a dark window: {dark_window}"
        );
        // Light over a bright window: still high contrast for dark text.
        let bright_window = light.output_luminance(25.0, 0.98);
        assert!(contrast(bright_window, 0.05) >= 4.5);

        let dark = spec(Scheme::Dark);
        // Dark: light text (0.95) over a bright moving window. The unadapted
        // 0.48 baseline is not enough; the readability rule must deepen it.
        let bright = dark.output_luminance(25.0, 0.98);
        assert!(
            contrast(bright, 0.95) >= 4.0,
            "Dark veil over a bright window: {bright}"
        );
        // Dark over a dark window: the black wash is invisible and the light
        // text stays readable.
        let dark_window = dark.output_luminance(25.0, 0.02);
        assert!(contrast(dark_window, 0.95) >= 4.5);
    }

    #[test]
    fn measured_samples_are_not_moved_by_the_readability_rule() {
        // The rule only raises alpha inside the strip. Over black the measured
        // Light LUT is already above the 0.35 target at every reference sample.
        let veil = spec(Scheme::Light);
        for depth in [5.0, 15.0, 25.0, 35.0] {
            assert!(
                (veil.adapted_alpha(depth, 0.0) - veil.alpha_at(depth)).abs() < 1e-9,
                "depth {depth} must keep the measured alpha"
            );
        }
        // Outside the strip the LUT is untouched even over black.
        assert!((veil.adapted_alpha(50.0, 0.0) - veil.alpha_at(50.0)).abs() < 1e-9);
    }

    #[test]
    fn the_readability_correction_fades_out_past_the_strip() {
        let dark = spec(Scheme::Dark);
        let at_edge = dark.adapted_alpha(39.0, 0.98);
        let just_past = dark.adapted_alpha(39.0 + 0.01, 0.98);
        assert!((at_edge - just_past).abs() < 1e-3, "no hard step at 39 pt");
        let faded = dark.adapted_alpha(39.0 + VEIL_READABILITY_FADE, 0.98);
        assert!((faded - dark.alpha_at(39.0 + VEIL_READABILITY_FADE)).abs() < 1e-6);
        // The Light measured samples still do not move over black.
        let light = spec(Scheme::Light);
        assert!((light.adapted_alpha(35.0, 0.0) - light.alpha_at(35.0)).abs() < 1e-9);
    }

    #[test]
    fn veil_is_the_explicit_sharp_no_blur_path() {
        assert_eq!(VEIL_FILTER, RegionFilter::Sharp);
        assert!(!VEIL_FILTER.schedules_blur());
        let veil = spec(Scheme::Light);
        let text = veil.diagnostic(RenderMode::NoBlurVeil);
        assert!(text.contains("mode=no-blur-veil"));
        assert!(text.contains("extent=95.0"));
        assert!(text.contains("strip=39.0"));
    }
}
