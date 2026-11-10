// SPDX-License-Identifier: MIT
//! The measured Dock optical fit (T198).
//!
//! The reference Dock is not a single Gaussian: its checker response keeps a
//! sharp high-frequency floor (the measured sharp fraction) and a multi-scale
//! low-frequency shoulder. The renderer realises this by compositing the sharp
//! scene with several Kawase levels (see [`crate::backend::nested::blur_panel_texture`]),
//! and this module is the renderer-free model of that composite. It is used by
//! the compositor tests and by the capture scorer so the shader, the cache key
//! and the reported metrics cannot drift.
//!
//! Periods are logical points. The reference capture is 2x, so its checker
//! periods in capture pixels map to `period_px / 2` points.

use crate::design_tokens::glass::presets::dock;

/// The Dock uses five Kawase levels so the broad shoulder is available from
/// the existing chain. Other roles keep their token-derived iteration count.
pub const DOCK_BLUR_ITERATIONS: u32 = 5;

/// Composite weights for `[sharp, level1, level2, level3, level4, level5]`.
///
/// Fitted to the reference checker AC ratios at 2/4/8/16/32 pt. The weight for
/// levels 3/4 is zero, so the renderer only draws the sharp scene, the first
/// two levels and the broad fifth level. All weights are non-negative and sum
/// to one, so the composite is a convex mix and cannot invent contrast.
pub const DOCK_SCALE_WEIGHTS: [f32; 6] = [0.12, 0.26, 0.40, 0.0, 0.0, 0.22];

/// The measured checker AC ratios at 2/4/8/16/32 pt (reference capture pixels
/// 4/8/16/32/64 at 2x). Used by the fit tests and the capture scorer.
pub const DOCK_CHECKER_TARGETS: [f32; 5] = [0.161, 0.150, 0.325, 0.670, 0.701];

/// The checker periods corresponding to [`DOCK_CHECKER_TARGETS`], in points.
pub const DOCK_CHECKER_PERIODS: [f32; 5] = [2.0, 4.0, 8.0, 16.0, 32.0];

/// The accepted deviation for every checker period.
pub const DOCK_MTF_TOLERANCE: f32 = 0.05;

/// Shape of the measured rim tail: `exp(-ln2 * (d / width)^power)`. A plain
/// exponential (power 1) puts the 10 % point at 3.3 pt; the measured profile
/// reaches it at ~2.5 pt. The exponent is a profile-shape mapping constant, not
/// a measured token value.
pub const RIM_TAIL_POWER: f32 = 1.2;

/// Width of the narrow specular line in physical pixels. It is centred on the
/// first covered pixel (0.5 px inside the SDF edge) so the measured peak lands
/// on that pixel at every output scale instead of relying on a wider wash.
pub const RIM_SPECULAR_WIDTH_PX: f32 = 0.5;

/// The rim's narrow core carries most of the measured energy; a 2 % broad
/// tail keeps the 1 % point out at the measured 6 pt while the 10 % point
/// stays near 2.5 pt. Both are profile-shape mapping constants.
pub const RIM_CORE_WEIGHT: f32 = 0.98;
pub const RIM_BROAD_WEIGHT: f32 = 0.02;
pub const RIM_BROAD_SCALE: f32 = 4.0;

/// One resolved Dock optical fit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockOptics {
    /// Flat-band mix fraction toward [`Self::tint`].
    pub alpha: f32,
    /// Flat-band tint level (neutral sRGB, 0..1).
    pub tint: f32,
    /// Gaussian-equivalent blur seed in logical points.
    pub blur: f32,
    /// Rim peak over the interior on mid-grey.
    pub edge: f32,
    /// Rim half-amplitude width in logical points.
    pub edge_width: f32,
    /// Rim tail in logical points.
    pub edge_tail: f32,
    /// Sharp scene weight in the filtered composite.
    pub sharp_fraction: f32,
    /// Corner-lens peak displacement in logical points along the outward SDF
    /// normal. The measured horizontal component at the 45-degree diagonal is
    /// `corner_lens_peak / sqrt(2)`.
    pub corner_lens_peak: f32,
    /// Distance inside the rim where the lens peaks, in logical points.
    pub corner_lens_distance: f32,
    /// Distance inside the rim by which the corner lens has decayed.
    pub corner_lens_tail: f32,
}

impl DockOptics {
    /// Resolve the measured fit for `scheme`.
    pub fn light() -> Self {
        Self::from_tokens(
            dock::light::calibration::NEUTRAL_ALPHA,
            dock::light::calibration::NEUTRAL_TINT,
            dock::light::BLUR,
            dock::light::EDGE,
            dock::light::EDGE_WIDTH,
            dock::light::EDGE_TAIL,
            dock::light::SHARP_FRACTION,
            dock::light::CORNER_LENS_PEAK,
            dock::light::CORNER_LENS_DISTANCE,
            dock::light::CORNER_LENS_TAIL,
        )
    }

    /// Resolve the measured fit for the dark scheme.
    pub fn dark() -> Self {
        Self::from_tokens(
            dock::dark::calibration::NEUTRAL_ALPHA,
            dock::dark::calibration::NEUTRAL_TINT,
            dock::dark::BLUR,
            dock::dark::EDGE,
            dock::dark::EDGE_WIDTH,
            dock::dark::EDGE_TAIL,
            dock::dark::SHARP_FRACTION,
            dock::dark::CORNER_LENS_PEAK,
            dock::dark::CORNER_LENS_DISTANCE,
            dock::dark::CORNER_LENS_TAIL,
        )
    }

    #[allow(clippy::too_many_arguments)]
    const fn from_tokens(
        alpha: f32,
        tint: f32,
        blur: f32,
        edge: f32,
        edge_width: f32,
        edge_tail: f32,
        sharp_fraction: f32,
        corner_lens_peak: f32,
        corner_lens_distance: f32,
        corner_lens_tail: f32,
    ) -> Self {
        Self {
            alpha,
            tint,
            blur,
            edge,
            edge_width,
            edge_tail,
            sharp_fraction,
            corner_lens_peak,
            corner_lens_distance,
            corner_lens_tail,
        }
    }

    /// The composite's checker response at `period_pt` logical points.
    ///
    /// The model mirrors the renderer's 4-tap Kawase pass (centre 4/8, four
    /// diagonal taps 1/8) followed by a bilinear upsample. The sharp scene is
    /// level zero; levels 1..=5 are the successive halvings.
    pub fn checker_mtf(&self, period_pt: f32) -> f32 {
        let period = period_pt.max(0.01);
        let mut total = 0.0;
        for (level, weight) in DOCK_SCALE_WEIGHTS.iter().enumerate() {
            if *weight == 0.0 {
                continue;
            }
            total += weight * level_mtf(period, level as u32);
        }
        total
    }

    /// The fitted checker response at every reference period.
    pub fn checker_mtf_curve(&self) -> [f32; 5] {
        let mut curve = [0.0; 5];
        for (index, period) in DOCK_CHECKER_PERIODS.iter().enumerate() {
            curve[index] = self.checker_mtf(*period);
        }
        curve
    }

    /// The largest absolute deviation from the reference checker targets.
    pub fn checker_error(&self) -> f32 {
        self.checker_mtf_curve()
            .iter()
            .zip(DOCK_CHECKER_TARGETS)
            .map(|(actual, target)| (actual - target).abs())
            .fold(0.0, f32::max)
    }

    /// The rim profile at `distance_pt` inside the plate edge: a sharp
    /// super-exponential with half-amplitude at [`Self::edge_width`], a 10 %
    /// point near 2.5 pt and a fade that ends at [`Self::edge_tail`]. The value
    /// is the addition over the interior. The narrow first-pixel specular line
    /// is a physical-pixel term the shader adds on top; this model is the
    /// logical-distance envelope.
    pub fn rim_profile(&self, distance_pt: f32) -> f32 {
        let distance = distance_pt.max(0.0);
        let width = self.edge_width.max(0.01);
        let core = RIM_CORE_WEIGHT
            * (-std::f32::consts::LN_2 * (distance / width).powf(RIM_TAIL_POWER))
                .exp();
        let broad = RIM_BROAD_WEIGHT
            * (-std::f32::consts::LN_2 * distance / (width * RIM_BROAD_SCALE)).exp();
        let tail = self.edge_tail.max(0.01);
        let fade = 1.0 - smoothstep(tail * 0.75, tail, distance);
        self.edge * (core + broad) * fade
    }

    /// The first distance at which the profile is at or below `fraction` of
    /// its peak, sampled at 0.01 pt.
    fn rim_distance_at(&self, fraction: f32) -> f32 {
        let target = self.edge * fraction;
        let mut distance = 0.0;
        while distance < self.edge_tail.max(0.01) {
            if self.rim_profile(distance) <= target {
                return distance;
            }
            distance += 0.01;
        }
        self.edge_tail
    }

    /// The distance at which the rim profile has decayed to half its peak.
    /// The profile is shaped so this is exactly the token width.
    pub fn rim_half_amplitude_distance(&self) -> f32 {
        self.edge_width
    }

    /// The distance at which the rim profile has decayed to 10 % of its peak.
    pub fn rim_ten_percent_distance(&self) -> f32 {
        self.rim_distance_at(0.1)
    }

    /// The distance at which the profile has decayed to 1 % of its peak.
    pub fn rim_one_percent_distance(&self) -> f32 {
        self.rim_distance_at(0.01)
    }

    /// The tail support: the profile is faded out by this distance.
    pub fn rim_tail_distance(&self) -> f32 {
        self.edge_tail
    }

    /// The first covered pixel's rim addition. The shader's specular line is
    /// centred on that pixel, so the measured peak is the token edge value.
    pub fn rim_first_pixel_delta(&self) -> f32 {
        self.edge
    }

    /// The outward corner-lens displacement at `inside_pt` inside the rim. The
    /// bump peaks at [`Self::corner_lens_distance`] and decays to the tail.
    /// The scalar is the displacement along the outward SDF normal; a flat
    /// edge or flat side has a zero gate and therefore no displacement.
    pub fn corner_displacement(&self, inside_pt: f32) -> f32 {
        if inside_pt < 0.0 || inside_pt > self.corner_lens_tail.max(0.01) {
            return 0.0;
        }
        let t = inside_pt / self.corner_lens_distance.max(0.01);
        // t^3 e^{3(1-t)} peaks at t=1 (value 1) and is ~0.034 at t=3.33.
        let bump = t.powi(3) * (3.0 * (1.0 - t)).exp();
        // Fade only near the measured tail so the bump still reaches its
        // nominal peak at `corner_lens_distance`.
        let tail = self.corner_lens_tail.max(0.01);
        let fade = 1.0 - smoothstep(tail * 0.75, tail, inside_pt);
        self.corner_lens_peak * bump * fade
    }

    /// The corner gate `4 * nx^2 * ny^2` for an outward SDF normal: zero on a
    /// flat top/bottom edge or a flat side, one at the 45-degree diagonal.
    pub fn corner_gate(normal_x: f32, normal_y: f32) -> f32 {
        4.0 * normal_x * normal_x * normal_y * normal_y
    }

    /// The horizontal component of the corner lens at `inside_pt` on the
    /// 45-degree corner diagonal, which is the measured probe. At that angle
    /// the gate is one and `|normal.x|` is `1/sqrt(2)`, so the scalar peak is
    /// calibrated to the measured horizontal component.
    pub fn corner_horizontal_component(&self, inside_pt: f32) -> f32 {
        self.corner_displacement(inside_pt) * std::f32::consts::FRAC_1_SQRT_2
    }

    /// The flat-edge displacement: the lens field is corner-only, so a point
    /// more than one tail from every corner sees no bend.
    pub const fn flat_edge_displacement(&self) -> f32 {
        0.0
    }

    /// Whether the composite needs the custom five-level chain.
    pub const fn uses_multi_scale(&self) -> bool {
        self.sharp_fraction > 0.0
    }
}

/// GLSL-style smoothstep.
fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    let span = (edge1 - edge0).abs().max(1e-6);
    let t = ((value - edge0) / span).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The amplitude response of Kawase level `level` at `period` output points.///
/// Level zero is the sharp scene. Each later level is one 4-tap pass and a
/// halving; the final bilinear upsample contributes a `sinc^2` shoulder.
fn level_mtf(period: f32, level: u32) -> f32 {
    if level == 0 {
        return 1.0;
    }
    let frequency = 1.0 / period.max(0.01);
    let mut response = 1.0f32;
    for pass in 0..level {
        let source_frequency = frequency * (1u32 << pass) as f32;
        response *= 0.5 + 0.5 * (std::f32::consts::TAU * source_frequency).cos();
    }
    let x = std::f32::consts::PI * frequency * (1u32 << level) as f32;
    let sinc = if x.abs() > 1e-9 { x.sin() / x } else { 1.0 };
    response * sinc * sinc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dock_perimeter_is_one_uniform_rim_with_no_directional_terms() {
        // T235a: the measured plate is one specular rim plus an inner glow on
        // every edge and around every corner. There is no darker side and no
        // straight top/bottom lift.
        for optics in [DockOptics::light(), DockOptics::dark()] {
            assert!((optics.rim_first_pixel_delta() - 0.397).abs() <= 0.02);
            // The profile is edge-agnostic by construction: it is a function
            // of the inside-rim distance only, so top, bottom, left and right
            // probes at the same distance are identical. It is monotone
            // decreasing and finite everywhere.
            let mut previous = optics.rim_profile(0.0);
            for distance in [0.25, 0.5, 1.0, 2.5, 4.0, 6.0] {
                let value = optics.rim_profile(distance);
                assert!(value <= previous + 1e-6, "rim rises at {distance} pt");
                assert!(value.is_finite());
                previous = value;
            }
            // Every field is finite and non-negative; there is no signed side
            // term left in the resolved fit.
            assert!(optics.edge > 0.0);
            assert!(optics.edge_width > 0.0);
            assert!(optics.edge_tail > 0.0);
        }
    }

    #[test]
    fn the_dock_fit_matches_every_measured_checker_period() {
        for optics in [DockOptics::light(), DockOptics::dark()] {
            let curve = optics.checker_mtf_curve();
            for (index, (actual, target)) in curve
                .iter()
                .zip(DOCK_CHECKER_TARGETS)
                .enumerate()
            {
                assert!(
                    (actual - target).abs() <= DOCK_MTF_TOLERANCE,
                    "period {} pt: {actual} vs {target}",
                    DOCK_CHECKER_PERIODS[index]
                );
            }
            assert!(optics.checker_error() <= DOCK_MTF_TOLERANCE);
        }
    }

    #[test]
    fn the_dock_flat_band_and_rim_match_the_reference_fit() {
        let light = DockOptics::light();
        assert!((light.alpha - 0.253).abs() < 1e-6);
        assert!((light.tint - 0.604).abs() < 1e-6);
        // The measured first covered pixel is the token peak.
        assert!((light.rim_first_pixel_delta() - 0.397).abs() <= 0.02);
        assert!((light.rim_profile(0.0) - 0.397).abs() <= 0.02);
        // Half-amplitude at 1 pt +/- 0.25, 10 % at 2.5 pt +/- 0.5, tail 6 pt
        // +/- 1. The tail fades to zero by the token tail.
        assert!((light.rim_half_amplitude_distance() - 1.0).abs() <= 0.25);
        assert!((light.rim_profile(1.0) - 0.397 / 2.0).abs() < 0.02);
        assert!((light.rim_ten_percent_distance() - 2.5).abs() <= 0.5);
        assert!((light.rim_profile(light.rim_ten_percent_distance()) - 0.397 * 0.1).abs()
            < 0.01);
        assert!((light.rim_tail_distance() - 6.0).abs() <= 1.0);
        assert!((light.rim_one_percent_distance() - 6.0).abs() <= 1.0);
        assert!(light.rim_profile(6.0).abs() < 0.01);
        assert_eq!(light.rim_profile(7.0), 0.0);

        let dark = DockOptics::dark();
        assert!((dark.alpha - 0.149).abs() < 1e-6);
        assert!((dark.tint - 0.207).abs() < 1e-6);
        assert!((dark.rim_profile(0.0) - 0.397).abs() <= 0.02);
    }

    #[test]
    fn the_dock_corner_lens_is_inside_rim_and_gated_by_the_normal() {
        let optics = DockOptics::light();
        let peak_distance = optics.corner_lens_distance;
        let peak = optics.corner_displacement(peak_distance);
        // The scalar peak is calibrated so the measured horizontal component
        // at the 45-degree diagonal is 9.6 pt at 15 pt inside the rim.
        let horizontal = optics.corner_horizontal_component(peak_distance);
        assert!((horizontal - 9.6).abs() <= 1.0, "horizontal {horizontal}");
        assert!((peak - 9.6 / std::f32::consts::FRAC_1_SQRT_2).abs() < 0.2);
        assert!(optics.corner_displacement(0.0).abs() < 0.01);
        assert!(optics.corner_displacement(50.0).abs() < 0.5);
        // The gate is exactly zero on a flat top/bottom edge and flat side.
        assert_eq!(DockOptics::corner_gate(0.0, 1.0), 0.0);
        assert_eq!(DockOptics::corner_gate(1.0, 0.0), 0.0);
        assert!((DockOptics::corner_gate(
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2
        ) - 1.0)
            .abs()
            < 1e-6);
        assert_eq!(optics.flat_edge_displacement(), 0.0);
        // Monotone rise to the peak, then decay.
        assert!(optics.corner_displacement(7.5) < peak);
        assert!(optics.corner_displacement(30.0) < peak);
    }

    #[test]
    fn the_scale_weights_are_a_convex_composite() {
        let sum: f32 = DOCK_SCALE_WEIGHTS.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6);
        assert!(DOCK_SCALE_WEIGHTS.iter().all(|weight| *weight >= 0.0));
        assert!((DOCK_SCALE_WEIGHTS[0] - DockOptics::light().sharp_fraction).abs() < 1e-6);
        assert_eq!(DOCK_BLUR_ITERATIONS, 5);
    }
}
