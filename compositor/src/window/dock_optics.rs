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

/// The measured sharp-scene fraction of the Dock blur mixture. The reference
/// fit is `M(f) = T·((1−k)·exp(−2π²σ²f²) + k)` with `k = 0.192`.
pub const DOCK_SHARP_FRACTION: f32 = 0.192;

/// The measured equivalent Gaussian sigma in logical points.
pub const DOCK_BLUR_SIGMA_PT: f32 = 3.86;

/// The measured low-frequency transmission of the blur mixture. It is slightly
/// above `1 - alpha` because the sharp path carries its own transmission.
pub const DOCK_LOW_FREQUENCY_TRANSMISSION: f32 = 0.777;

/// The measured checker AC ratios at 2/4/8/16/32 pt (reference capture pixels
/// 4/8/16/32/64 at 2x). Used by the fit tests and the capture scorer.
pub const DOCK_CHECKER_TARGETS: [f32; 5] = [0.161, 0.150, 0.325, 0.670, 0.701];

/// The checker square periods corresponding to [`DOCK_CHECKER_TARGETS`], in
/// points. The checker fundamental is twice the square period.
pub const DOCK_CHECKER_PERIODS: [f32; 5] = [2.0, 4.0, 8.0, 16.0, 32.0];

/// The accepted deviation for every checker period. The reference fit itself
/// has an RMSE of 0.029; individual points are allowed a little more.
pub const DOCK_MTF_TOLERANCE: f32 = 0.06;

/// Continuous-corner exponent for the measured Dock. `2` is the historical
/// circular arc; `4` is the fitted squircle. The Lp corner norm leaves the
/// straight sides untouched and only rounds the corner with continuous
/// curvature.
pub const DOCK_SQUIRCLE_POWER: f32 = 4.0;

/// The fitted bevel height field, in logical points. The reference measures
/// the *displacement*, not the height; these constants are the analytic height
/// profile fitted to the measured −4 / −8.5 / −9.6 pt short-end displacement
/// at 5 / 10 / 15 pt inside the rim. The profile is monotone and smooth:
/// `h(t) = H·(1 − (1−t)^m)^p`, with `t = depth / bezel`.
pub const DOCK_BEVEL_HEIGHT_PT: f32 = 45.0;
pub const DOCK_BEVEL_IOR: f32 = 1.5;
pub const DOCK_BEVEL_SHAPE_M: f32 = 4.0;
pub const DOCK_BEVEL_SHAPE_P: f32 = 1.4;
/// Fraction of the full bevel displacement retained on a long flat edge. The
/// measured flat top/bottom is ~0.5 pt against a ~9.6 pt short-end peak.
pub const DOCK_BEVEL_FLAT_GAIN: f32 = 0.05;

/// The measured OKLab tone curve for saturated backgrounds. Neutral
/// backgrounds keep the sRGB flat-band fit; the shader blends these in by
/// chroma so the measured neutral RMSE is untouched.
pub const DOCK_OKLAB_L_OFFSET: f32 = 0.2696;
pub const DOCK_OKLAB_L_GAIN: f32 = 0.6559;
pub const DOCK_OKLAB_L_POWER: f32 = 1.244;
pub const DOCK_OKLAB_CHROMA_GAIN: f32 = 1.0916;

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
    /// Lens peak displacement magnitude in logical points along the outward
    /// SDF normal. It is fitted to the short-end displacement; applied along a
    /// 45-degree corner normal, the horizontal component is
    /// `corner_lens_peak / sqrt(2)`.
    pub corner_lens_peak: f32,
    /// Distance inside the rim where the lens peaks, in logical points.
    pub corner_lens_distance: f32,
    /// Distance inside the rim by which the lens has decayed. It is also the
    /// reach of the short-end gain along the long edges.
    pub corner_lens_tail: f32,
    /// Measured OKLab tone weight for saturated backgrounds.
    pub tone_weight: f32,
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
            dock::light::TONE_WEIGHT,
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
            dock::dark::TONE_WEIGHT,
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
        tone_weight: f32,
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
            tone_weight,
        }
    }

    /// The fitted Snell displacement at `depth` logical points inside the rim,
    /// as the displacement magnitude along the outward SDF normal.
    ///
    /// The height field is `h(t) = H·(1 − (1−t)^m)^p`; the surface slope feeds
    /// Snell's law (`n = 1.5`). The measured −4 / −8.5 / −9.6 pt at 5 / 10 /
    /// 15 pt was tracked on a vertical grid line at the **short end** of the
    /// Dock, where the normal is horizontal, so the fitted value is the
    /// short-side displacement. Applied along a 45-degree corner normal, the
    /// horizontal component is smaller; the flat top/bottom gets the measured
    /// ~0.5 pt from [`DOCK_BEVEL_FLAT_GAIN`]. This is the renderer-free model
    /// the shader mirrors, so a change to the fit fails here before it reaches
    /// a capture.
    pub fn bevel_shift(&self, depth: f32) -> f32 {
        let bezel = self.corner_lens_tail.max(1.0);
        let t = (depth / bezel).clamp(0.0, 1.0);
        if t <= 0.0 || t >= 1.0 {
            return 0.0;
        }
        let base = 1.0 - (1.0 - t).powf(DOCK_BEVEL_SHAPE_M);
        let height = DOCK_BEVEL_HEIGHT_PT * base.powf(DOCK_BEVEL_SHAPE_P);
        let slope = DOCK_BEVEL_HEIGHT_PT
            * DOCK_BEVEL_SHAPE_P
            * base.powf(DOCK_BEVEL_SHAPE_P - 1.0)
            * DOCK_BEVEL_SHAPE_M
            * (1.0 - t).powf(DOCK_BEVEL_SHAPE_M - 1.0)
            / bezel;
        let theta_i = slope.atan();
        let theta_r = (theta_i.sin() / DOCK_BEVEL_IOR).clamp(-1.0, 1.0).asin();
        height * (theta_i - theta_r).tan() / std::f32::consts::SQRT_2
    }

    /// The measured plate lens is an end lens: strongest across the short
    /// sides (the left/right ends of a horizontal Dock, the top/bottom ends of
    /// a vertical one) and fading along the long edges.
    ///
    /// `half_size` is the inner-rectangle half extent (`size/2 − radius`) and
    /// `p` is the point relative to the panel centre, both in logical points.
    /// Only the long-axis component controls the gain, so the whole short side
    /// shares one rounded refraction instead of the two 45-degree lobes the
    /// old `4·nx²·ny²` normal product produced.
    pub fn bevel_end_gain(&self, half_size: [f32; 2], p: [f32; 2]) -> f32 {
        let offset = [
            (half_size[0] - p[0].abs()).max(0.0),
            (half_size[1] - p[1].abs()).max(0.0),
        ];
        let along = if half_size[0] >= half_size[1] {
            offset[0]
        } else {
            offset[1]
        };
        let reach = self.corner_lens_tail.max(1.0);
        DOCK_BEVEL_FLAT_GAIN + (1.0 - DOCK_BEVEL_FLAT_GAIN) * (-along / reach).exp()
    }

    /// The measured mixture's checker response at `period_pt` logical points.
    ///
    /// The reference model is
    /// `M(f) = T·((1−k)·exp(−2π²σ²f²) + k)`, where `f` is the checker
    /// fundamental (`1 / (2 · square period)`), `σ` is the equivalent Gaussian,
    /// `k` is the sharp fraction, and `T` is the low-frequency transmission.
    /// The renderer implements the same mixture as one Gaussian pass plus a
    /// sharp-scene composite.
    pub fn checker_mtf(&self, period_pt: f32) -> f32 {
        let square = period_pt.max(0.01);
        let frequency = 1.0 / (2.0 * square);
        let blurred = (-2.0
            * std::f32::consts::PI
            * std::f32::consts::PI
            * self.blur
            * self.blur
            * frequency
            * frequency)
            .exp();
        let sharp = self.sharp_fraction.clamp(0.0, 1.0);
        DOCK_LOW_FREQUENCY_TRANSMISSION * (sharp + (1.0 - sharp) * blurred)
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
            * (-std::f32::consts::LN_2 * (distance / width).powf(RIM_TAIL_POWER)).exp();
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
            for (index, (actual, target)) in curve.iter().zip(DOCK_CHECKER_TARGETS).enumerate() {
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
        assert!((light.rim_profile(light.rim_ten_percent_distance()) - 0.397 * 0.1).abs() < 0.01);
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
    fn the_bevel_height_field_matches_the_measured_displacement() {
        let optics = DockOptics::light();
        // The measured short-end displacement is −4 / −8.5 / −9.6 pt at
        // 5 / 10 / 15 pt inside the rim (tracked on a vertical grid line at
        // the Dock's left end, where the outward normal is horizontal).
        for (depth, target) in [(5.0, 4.0), (10.0, 8.5), (15.0, 9.6)] {
            let shift = optics.bevel_shift(depth);
            assert!(
                (shift - target).abs() <= 0.7,
                "bevel shift at {depth} pt is {shift}, expected {target}"
            );
        }
        // The surface starts at zero displacement at the rim and returns to
        // zero by the measured influence radius.
        assert!(optics.bevel_shift(0.0).abs() < 1e-6);
        assert!(optics.bevel_shift(optics.corner_lens_tail).abs() < 1e-6);
        // A smooth rise and decay: no hard gate at the corner-to-side join.
        assert!(optics.bevel_shift(2.0) > 0.0);
        assert!(optics.bevel_shift(30.0) > 0.0);
    }

    #[test]
    fn the_bevel_is_a_short_end_lens_not_a_45_degree_corner_gate() {
        let optics = DockOptics::light();
        // Standard horizontal Dock: 1163 x 80 pt, 26.9 pt radius.
        let half = [1163.0 / 2.0 - 26.9, 80.0 / 2.0 - 26.9];
        // The whole short side (left/right end) keeps the full measured lens,
        // including its middle. The old gate left this at the flat gain and
        // put the two maxima on the 45-degree diagonals.
        for y in [-half[1], 0.0, half[1]] {
            let gain = optics.bevel_end_gain(half, [-1163.0 / 2.0, y]);
            assert!((gain - 1.0).abs() < 1e-6, "short end at y={y}: {gain}");
        }
        // The long edge middle is the measured ~0.5 pt flat.
        let gain = optics.bevel_end_gain(half, [0.0, -80.0 / 2.0]);
        assert!((gain - DOCK_BEVEL_FLAT_GAIN).abs() < 0.01);
        // The gain decays monotonically along the long edge: no saddle and no
        // 45-degree lobes between the end and the interior.
        let mut previous = 1.0;
        for x in [0.0, 10.0, 25.0, 50.0, 100.0, 200.0] {
            let gain = optics.bevel_end_gain(half, [half[0] - x, 0.0]);
            assert!(gain <= previous + 1e-6, "gain rises at {x} pt: {gain}");
            previous = gain;
        }
        // A vertical Dock is the same model on its short top/bottom ends.
        let tall = [80.0 / 2.0 - 26.9, 1163.0 / 2.0 - 26.9];
        assert!((optics.bevel_end_gain(tall, [0.0, 1163.0 / 2.0]) - 1.0).abs() < 1e-6);
        assert!((optics.bevel_end_gain(tall, [0.0, 0.0]) - DOCK_BEVEL_FLAT_GAIN).abs() < 0.01);
    }

    #[test]
    fn the_dock_tone_weight_is_the_measured_saturated_background_curve() {
        let light = DockOptics::light();
        assert!((light.tone_weight - 1.0).abs() < 1e-6);
        assert!((DOCK_OKLAB_L_OFFSET - 0.2696).abs() < 1e-6);
        assert!((DOCK_OKLAB_L_GAIN - 0.6559).abs() < 1e-6);
        assert!((DOCK_OKLAB_L_POWER - 1.244).abs() < 1e-6);
        assert!((DOCK_OKLAB_CHROMA_GAIN - 1.0916).abs() < 1e-6);
    }

    #[test]
    fn the_dock_blur_is_the_measured_sharp_gaussian_mixture() {
        let light = DockOptics::light();
        assert!((light.sharp_fraction - DOCK_SHARP_FRACTION).abs() < 1e-6);
        assert!((light.blur - DOCK_BLUR_SIGMA_PT).abs() < 1e-6);
        // At very high frequency the Gaussian vanishes and the response is the
        // sharp floor times the low-frequency transmission.
        let floor = DOCK_LOW_FREQUENCY_TRANSMISSION * DOCK_SHARP_FRACTION;
        assert!((light.checker_mtf(0.01) - floor).abs() < 1e-3);
        // At very low frequency the Gaussian is unity and the response is the
        // measured transmission.
        assert!((light.checker_mtf(1024.0) - DOCK_LOW_FREQUENCY_TRANSMISSION).abs() < 1e-3);
    }
}
