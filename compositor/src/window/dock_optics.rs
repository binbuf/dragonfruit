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
    /// Corner-lens peak displacement in logical points.
    pub corner_lens_peak: f32,
    /// Distance from the corner where the lens peaks, in logical points.
    pub corner_lens_distance: f32,
    /// Distance by which the corner lens has decayed, in logical points.
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
    /// exponential with half-amplitude at [`Self::edge_width`] and a tail by
    /// [`Self::edge_tail`]. The value is the addition over the interior.
    pub fn rim_profile(&self, distance_pt: f32) -> f32 {
        let distance = distance_pt.max(0.0);
        self.edge * (-std::f32::consts::LN_2 * distance / self.edge_width.max(0.01)).exp()
    }

    /// The distance at which the rim profile has decayed to 1% of its peak.
    pub fn rim_tail_distance(&self) -> f32 {
        self.edge_width.max(0.01) * (100.0f32.ln() / std::f32::consts::LN_2)
    }

    /// The outward corner-lens displacement at `distance_pt` from the corner
    /// centre. The bump peaks at [`Self::corner_lens_distance`] and decays to
    /// the tail; flat edges are outside the corner field and return zero.
    pub fn corner_displacement(&self, distance_pt: f32) -> f32 {
        if distance_pt < 0.0 || distance_pt > self.corner_lens_tail.max(0.01) {
            return 0.0;
        }
        let t = distance_pt / self.corner_lens_distance.max(0.01);
        // t^3 e^{3(1-t)} peaks at t=1 (value 1) and is ~0.034 at t=3.33.
        let bump = t.powi(3) * (3.0 * (1.0 - t)).exp();
        // Fade only near the measured tail so the bump still reaches its
        // nominal peak at `corner_lens_distance`.
        let tail = self.corner_lens_tail.max(0.01);
        let fade = 1.0 - smoothstep(tail * 0.75, tail, distance_pt);
        self.corner_lens_peak * bump * fade
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
        assert!((light.rim_profile(0.0) - 0.397).abs() <= 0.397 * 0.05);
        assert!((light.rim_profile(1.0) - 0.397 / 2.0).abs() < 0.02);
        assert!((light.rim_tail_distance() - 6.0).abs() <= 1.0);
        assert!((light.rim_profile(6.0)).abs() < 0.02);

        let dark = DockOptics::dark();
        assert!((dark.alpha - 0.149).abs() < 1e-6);
        assert!((dark.tint - 0.207).abs() < 1e-6);
        assert!((dark.rim_profile(0.0) - 0.397).abs() <= 0.397 * 0.05);
    }

    #[test]
    fn the_dock_corner_lens_peaks_inside_and_leaves_flat_edges_alone() {
        let optics = DockOptics::light();
        let peak_distance = optics.corner_lens_distance;
        let peak = optics.corner_displacement(peak_distance);
        assert!((peak - 9.6).abs() <= 2.0, "corner peak {peak}");
        assert!(optics.corner_displacement(0.0).abs() < 0.01);
        assert!(optics.corner_displacement(50.0).abs() < 0.5);
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
