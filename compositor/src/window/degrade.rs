// SPDX-License-Identifier: MIT
//! Budget-pressure material degrade tiers (T-04.4a).
//!
//! The material track's named risk is the frame budget on an iGPU, so the
//! chrome backdrop and the elevation shadows must be able to give ground
//! **before** they cost a frame. This module is the one policy for that:
//!
//! * [`DegradeTier`] is the ordered quality ladder — [`Full`] (the token
//!   material as designed), [`Reduced`] (smaller radius and fewer layers),
//!   and [`Minimal`] (blur off, a small tight shadow). Every tier is
//!   renderer-free geometry, so it is asserted headless.
//! * [`DegradeTier::backdrop`] / [`DegradeTier::shadow`] map a token-resolved
//!   `BackdropSpec`/`ShadowSpec` through the tier. They never invent values:
//!   they scale the token geometry and drop the backdrop entirely at the
//!   bottom tier, so the design tokens stay the single source (ADR 0011/0013).
//! * [`DegradeController`] *selects* the tier from observed rendered-frame
//!   durations against a frame budget (default [`animation::FRAME_INTERVAL`]).
//!   It steps down under sustained pressure and recovers with hysteresis, and
//!   it can be pinned with [`DegradeController::force`] so a test (or the T-05
//!   overview) can choose a tier deterministically.
//!
//! The controller is deliberately pure (it takes a `Duration`, no clock) and
//! carries its own counters, so the render-stats trace can report which tier
//! ran and how the selection moved. On headless nothing renders, so the
//! controller is exercised by its unit tests and the `query degrade` hook.
//!
//! [`Full`]: DegradeTier::Full
//! [`Reduced`]: DegradeTier::Reduced
//! [`Minimal`]: DegradeTier::Minimal

use std::time::Duration;

use crate::animation::FRAME_INTERVAL;
use crate::window::backdrop::BackdropSpec;
use crate::window::shadow::ShadowSpec;

/// A frame is "over budget" when its smoothed duration exceeds the budget by
/// any amount; the smoothing means a stray scheduling spike does not count.
const EMA_ALPHA: f64 = 0.3;
/// Consecutive over-budget frames before the tier steps down.
const DOWNGRADE_FRAMES: u32 = 12;
/// Consecutive clearly-under-budget frames before the tier recovers.
const UPGRADE_FRAMES: u32 = 180;
/// "Clearly under" is this fraction of the budget: recovery needs real
/// headroom, so a tier does not oscillate around the threshold.
const RECOVERY_RATIO: f64 = 0.75;
/// Bounds an operator/test-provided budget (1 ms..100 ms).
const MIN_BUDGET_US: u32 = 1_000;
const MAX_BUDGET_US: u32 = 100_000;

/// The environment variable that overrides the frame budget, in microseconds.
/// Unset in a normal session; a nested/DRM acceptance run can pin it.
pub const ENV_FRAME_BUDGET_US: &str = "DRAGONFRUIT_FRAME_BUDGET_US";

/// Ordered material-quality tiers, best first. The ladder is the T-04 track's
/// "smaller radius, then blur off" rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DegradeTier {
    /// The token material as designed: full radius, full layers, blur on.
    #[default]
    Full,
    /// Budget pressure: smaller radius and roughly half the layers; blur
    /// stays on (it is the visual floor the track exists for).
    Reduced,
    /// Sustained pressure: blur is off entirely and shadows collapse to a
    /// small tight ring. The window still reads as elevated and rounded.
    Minimal,
}

impl DegradeTier {
    /// The ladder from best to worst; the controller walks this order.
    pub const ALL: [DegradeTier; 3] = [
        DegradeTier::Full,
        DegradeTier::Reduced,
        DegradeTier::Minimal,
    ];

    /// The stable name used by the render-stats line and `query degrade`.
    pub const fn name(self) -> &'static str {
        match self {
            DegradeTier::Full => "full",
            DegradeTier::Reduced => "reduced",
            DegradeTier::Minimal => "minimal",
        }
    }

    /// Parse a tier name (the inverse of [`Self::name`]).
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "full" => Some(DegradeTier::Full),
            "reduced" => Some(DegradeTier::Reduced),
            "minimal" => Some(DegradeTier::Minimal),
            _ => None,
        }
    }

    /// The ladder position, best (`0`) to worst.
    pub const fn index(self) -> usize {
        match self {
            DegradeTier::Full => 0,
            DegradeTier::Reduced => 1,
            DegradeTier::Minimal => 2,
        }
    }

    /// Whether the backdrop blur is drawn at all. `false` is the "blur off"
    /// step, and the render pass then emits no backdrop elements and owns no
    /// backdrop damage.
    pub const fn blur_enabled(self) -> bool {
        !matches!(self, DegradeTier::Minimal)
    }

    /// The scale applied to radius/blur geometry. `1.0` at [`Full`], then
    /// `0.5` and `0.25` so the corners shrink and the feather spread tightens
    /// without ever reaching zero (a rounded window stays rounded).
    ///
    /// [`Full`]: DegradeTier::Full
    pub const fn geometry_scale(self) -> f32 {
        match self {
            DegradeTier::Full => 1.0,
            DegradeTier::Reduced => 0.5,
            DegradeTier::Minimal => 0.25,
        }
    }

    /// The scale applied to layer counts, clamped so a degraded material never
    /// drops below a single layer. `1.0` at [`Full`], then `0.5`/`0.25`.
    ///
    /// [`Full`]: DegradeTier::Full
    pub const fn layer_scale(self) -> f32 {
        match self {
            DegradeTier::Full => 1.0,
            DegradeTier::Reduced => 0.5,
            DegradeTier::Minimal => 0.25,
        }
    }

    /// The backdrop material at this tier, or `None` when blur is off. The
    /// token opacity and tone are preserved; only geometry/layers scale.
    pub fn backdrop(self, spec: BackdropSpec) -> Option<BackdropSpec> {
        if !self.blur_enabled() {
            return None;
        }
        Some(BackdropSpec {
            blur: spec.blur * self.geometry_scale(),
            opacity: spec.opacity,
            layers: scale_layers(spec.layers, self.layer_scale()),
            radius: spec.radius * self.geometry_scale(),
            color: spec.color,
        })
    }

    /// The elevation shadow at this tier. The scheme opacity/color and the
    /// downward offset are preserved; the spread, layers, and corner follow
    /// the tier so `Shadow.qml`-equivalent depth is kept at every tier.
    pub fn shadow(self, spec: ShadowSpec) -> ShadowSpec {
        ShadowSpec {
            blur: spec.blur * self.geometry_scale(),
            offset_y: spec.offset_y,
            layers: scale_layers(spec.layers, self.layer_scale()),
            radius: spec.radius * self.geometry_scale(),
            opacity: spec.opacity,
            color: spec.color,
        }
    }
}

/// Scale a token layer count, never below one.
fn scale_layers(layers: u32, scale: f32) -> u32 {
    ((layers as f32 * scale).round() as u32).max(1)
}

/// Selects a [`DegradeTier`] from observed rendered-frame durations against a
/// frame budget, with hysteresis, plus the tier counters the trace reports.
///
/// The controller never forces a redraw and never reads the clock: the session
/// loop hands it each rendered frame's duration, so it is inert while idle
/// exactly like the render pass it degrades.
#[derive(Debug)]
pub struct DegradeController {
    tier: DegradeTier,
    forced: Option<DegradeTier>,
    budget_us: u32,
    ema_us: f64,
    samples: u64,
    over_budget_streak: u32,
    under_budget_streak: u32,
    over_budget_frames: u64,
    downgrades: u64,
    upgrades: u64,
    tier_frames: [u64; 3],
}

impl Default for DegradeController {
    fn default() -> Self {
        Self::new()
    }
}

impl DegradeController {
    /// A controller at [`Full`] with the default budget
    /// ([`animation::FRAME_INTERVAL`]), overridable through
    /// [`ENV_FRAME_BUDGET_US`].
    pub fn new() -> Self {
        let budget_us = std::env::var(ENV_FRAME_BUDGET_US)
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(FRAME_INTERVAL.as_micros() as u32)
            .clamp(MIN_BUDGET_US, MAX_BUDGET_US);
        Self::with_budget_us(budget_us)
    }

    /// A controller at [`Full`] with an explicit budget.
    pub fn with_budget_us(budget_us: u32) -> Self {
        Self {
            tier: DegradeTier::Full,
            forced: None,
            budget_us: budget_us.clamp(MIN_BUDGET_US, MAX_BUDGET_US),
            ema_us: 0.0,
            samples: 0,
            over_budget_streak: 0,
            under_budget_streak: 0,
            over_budget_frames: 0,
            downgrades: 0,
            upgrades: 0,
            tier_frames: [0; 3],
        }
    }

    /// The tier materials should render at.
    pub fn tier(&self) -> DegradeTier {
        self.tier
    }

    /// The pinned tier, if selection is suspended.
    pub fn forced(&self) -> Option<DegradeTier> {
        self.forced
    }

    /// The frame budget in microseconds.
    pub fn budget_us(&self) -> u32 {
        self.budget_us
    }

    /// Set the frame budget. Does not change the tier; the next observations
    /// decide. Clamped to a sane range.
    pub fn set_budget_us(&mut self, budget_us: u32) {
        self.budget_us = budget_us.clamp(MIN_BUDGET_US, MAX_BUDGET_US);
    }

    /// Pin the tier, suspending automatic selection. Used by a test or by a
    /// feature (T-05 overview) that wants a deterministic tier.
    pub fn force(&mut self, tier: DegradeTier) {
        self.forced = Some(tier);
        self.tier = tier;
        self.reset_streaks();
    }

    /// Resume automatic selection. The current tier is kept until the
    /// observations move it, so releasing a pin never causes a jump.
    pub fn release(&mut self) {
        self.forced = None;
        self.reset_streaks();
    }

    /// Observe one rendered frame's duration and advance the selection.
    ///
    /// The duration feeds an exponential moving average so a single spike
    /// cannot degrade the UI; sustained pressure (or sustained headroom) walks
    /// the ladder one tier at a time. While pinned the tier never changes.
    pub fn observe(&mut self, duration: Duration) {
        let us = duration.as_micros() as f64;
        self.ema_us = if self.samples == 0 {
            us
        } else {
            self.ema_us * (1.0 - EMA_ALPHA) + us * EMA_ALPHA
        };
        self.samples += 1;
        self.tier_frames[self.tier.index()] += 1;
        if self.forced.is_some() {
            return;
        }
        let budget = f64::from(self.budget_us);
        if self.ema_us > budget {
            self.over_budget_streak += 1;
            self.under_budget_streak = 0;
            self.over_budget_frames += 1;
        } else if self.ema_us < budget * RECOVERY_RATIO {
            self.under_budget_streak += 1;
            self.over_budget_streak = 0;
        } else {
            self.reset_streaks();
        }
        if self.over_budget_streak >= DOWNGRADE_FRAMES {
            self.reset_streaks();
            self.step(1);
        } else if self.under_budget_streak >= UPGRADE_FRAMES {
            self.reset_streaks();
            self.step(-1);
        }
    }

    /// Number of frame durations observed.
    pub fn samples(&self) -> u64 {
        self.samples
    }

    /// Total frames whose smoothed duration exceeded the budget.
    pub fn over_budget_frames(&self) -> u64 {
        self.over_budget_frames
    }

    /// Number of times the ladder stepped toward [`Minimal`].
    ///
    /// [`Minimal`]: DegradeTier::Minimal
    pub fn downgrades(&self) -> u64 {
        self.downgrades
    }

    /// Number of times the ladder stepped back toward [`Full`].
    ///
    /// [`Full`]: DegradeTier::Full
    pub fn upgrades(&self) -> u64 {
        self.upgrades
    }

    /// Frames rendered at `tier`.
    pub fn tier_frames(&self, tier: DegradeTier) -> u64 {
        self.tier_frames[tier.index()]
    }

    /// A one-line summary for the `dump_stats` / `query degrade` trace.
    pub fn summary(&self) -> String {
        format!(
            "tier={} forced={} budget_us={} samples={} over_budget={} downgrades={} \
             upgrades={} tiers=full:{},reduced:{},minimal:{}",
            self.tier.name(),
            self.forced.is_some() as u32,
            self.budget_us,
            self.samples,
            self.over_budget_frames,
            self.downgrades,
            self.upgrades,
            self.tier_frames[DegradeTier::Full.index()],
            self.tier_frames[DegradeTier::Reduced.index()],
            self.tier_frames[DegradeTier::Minimal.index()],
        )
    }

    fn reset_streaks(&mut self) {
        self.over_budget_streak = 0;
        self.under_budget_streak = 0;
    }

    /// Move `steps` positions along the ladder (positive = toward degraded),
    /// saturating at both ends and counting real moves.
    fn step(&mut self, steps: i32) {
        let target =
            (self.tier.index() as i32 + steps).clamp(0, DegradeTier::ALL.len() as i32 - 1) as usize;
        let target = DegradeTier::ALL[target];
        if target == self.tier {
            return;
        }
        if target.index() > self.tier.index() {
            self.downgrades += 1;
        } else {
            self.upgrades += 1;
        }
        self.tier = target;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::window::backdrop::MaterialRole;
    use crate::window::decoration::ColorScheme;
    use crate::window::shadow::ShadowLevel;

    fn frames(controller: &mut DegradeController, count: u32, micros: u64) {
        for _ in 0..count {
            controller.observe(Duration::from_micros(micros));
        }
    }

    #[test]
    fn names_round_trip_and_the_default_is_full() {
        assert_eq!(DegradeTier::default(), DegradeTier::Full);
        for tier in DegradeTier::ALL {
            assert_eq!(DegradeTier::from_name(tier.name()), Some(tier));
        }
        assert_eq!(DegradeTier::from_name("ultra"), None);
        assert_eq!(DegradeTier::ALL.len(), 3);
    }

    #[test]
    fn tiers_scale_the_backdrop_and_turn_blur_off_at_minimal() {
        let base = MaterialRole::Chrome.spec(ColorScheme::Light);

        // Full is the token material untouched.
        assert_eq!(DegradeTier::Full.backdrop(base), Some(base));

        // Reduced shrinks the radius/blur and halves the layers, keeping the
        // token opacity and tone.
        let reduced = DegradeTier::Reduced
            .backdrop(base)
            .expect("reduced keeps the blur");
        assert!(reduced.blur < base.blur);
        assert!(reduced.radius < base.radius);
        assert!(reduced.layers < base.layers);
        assert!(reduced.layers >= 1);
        assert_eq!(reduced.opacity, base.opacity);
        assert_eq!(reduced.color, base.color);

        // Minimal is blur off: the render pass emits nothing.
        assert_eq!(DegradeTier::Minimal.backdrop(base), None);
        assert!(!DegradeTier::Minimal.blur_enabled());
        assert!(DegradeTier::Reduced.blur_enabled());
    }

    #[test]
    fn tiers_shrink_the_shadow_but_always_keep_a_layer() {
        let base = ShadowLevel::High.spec(ColorScheme::Dark);
        assert_eq!(DegradeTier::Full.shadow(base), base);

        let reduced = DegradeTier::Reduced.shadow(base);
        assert!(reduced.blur < base.blur);
        assert!(reduced.radius < base.radius);
        assert!(reduced.layers < base.layers && reduced.layers >= 1);
        assert_eq!(reduced.opacity, base.opacity);
        assert_eq!(reduced.offset_y, base.offset_y);

        let minimal = DegradeTier::Minimal.shadow(base);
        assert!(minimal.blur < reduced.blur);
        assert!(minimal.layers >= 1);
    }

    #[test]
    fn sustained_over_budget_frames_step_the_ladder_down_to_minimal() {
        let mut controller = DegradeController::with_budget_us(16_000);
        assert_eq!(controller.tier(), DegradeTier::Full);

        // One full downgrade window of clearly-over-budget frames.
        frames(&mut controller, DOWNGRADE_FRAMES, 20_000);
        assert_eq!(controller.tier(), DegradeTier::Reduced);
        assert_eq!(controller.downgrades(), 1);

        frames(&mut controller, DOWNGRADE_FRAMES, 20_000);
        assert_eq!(controller.tier(), DegradeTier::Minimal);
        assert_eq!(controller.downgrades(), 2);

        // Minimal saturates: it cannot degrade further.
        frames(&mut controller, DOWNGRADE_FRAMES * 4, 50_000);
        assert_eq!(controller.tier(), DegradeTier::Minimal);
        assert_eq!(controller.downgrades(), 2);
        assert!(controller.over_budget_frames() > 0);
        assert!(controller.samples() == u64::from(DOWNGRADE_FRAMES * 6));
    }

    #[test]
    fn isolated_spikes_do_not_degrade() {
        let mut controller = DegradeController::with_budget_us(16_000);
        // A single 100 ms hitch, then many fast frames: the streak never
        // reaches the downgrade window.
        controller.observe(Duration::from_millis(100));
        frames(&mut controller, 200, 5_000);
        assert_eq!(controller.tier(), DegradeTier::Full);
        assert_eq!(controller.downgrades(), 0);
    }

    #[test]
    fn sustained_headroom_recovers_toward_full() {
        let mut controller = DegradeController::with_budget_us(16_000);
        frames(&mut controller, DOWNGRADE_FRAMES * 2, 25_000);
        assert_eq!(controller.tier(), DegradeTier::Minimal);

        // Recovery needs the longer under-budget window, and climbs one tier
        // at a time. The extra frames absorb the EMA's warm-up from the
        // degraded frame time before the under-budget streak can start.
        frames(&mut controller, UPGRADE_FRAMES + 100, 4_000);
        assert_eq!(controller.tier(), DegradeTier::Reduced);
        assert_eq!(controller.upgrades(), 1);

        frames(&mut controller, UPGRADE_FRAMES + 100, 4_000);
        assert_eq!(controller.tier(), DegradeTier::Full);
        assert_eq!(controller.upgrades(), 2);
    }

    #[test]
    fn forcing_a_tier_pins_it_through_load_and_release_resumes() {
        let mut controller = DegradeController::with_budget_us(16_000);
        controller.force(DegradeTier::Full);
        assert_eq!(controller.forced(), Some(DegradeTier::Full));

        // Heavy load cannot move a pinned tier.
        frames(&mut controller, DOWNGRADE_FRAMES * 4, 90_000);
        assert_eq!(controller.tier(), DegradeTier::Full);
        assert_eq!(controller.downgrades(), 0);

        // Releasing keeps the tier until the observations move it.
        controller.release();
        assert_eq!(controller.forced(), None);
        assert_eq!(controller.tier(), DegradeTier::Full);
        frames(&mut controller, DOWNGRADE_FRAMES, 30_000);
        assert_eq!(controller.tier(), DegradeTier::Reduced);
    }

    #[test]
    fn the_budget_is_selectable_and_clamped() {
        let mut controller = DegradeController::with_budget_us(20_000);
        assert_eq!(controller.budget_us(), 20_000);
        controller.set_budget_us(0);
        assert_eq!(controller.budget_us(), MIN_BUDGET_US);
        controller.set_budget_us(u32::MAX);
        assert_eq!(controller.budget_us(), MAX_BUDGET_US);
    }

    #[test]
    fn the_summary_is_a_single_parseable_line() {
        let mut controller = DegradeController::with_budget_us(16_000);
        frames(&mut controller, 3, 5_000);
        let summary = controller.summary();
        for field in [
            "tier=full",
            "forced=0",
            "budget_us=16000",
            "samples=3",
            "over_budget=0",
            "downgrades=0",
            "upgrades=0",
            "tiers=full:3,reduced:0,minimal:0",
        ] {
            assert!(summary.contains(field), "missing {field:?} in {summary:?}");
        }
        assert!(!summary.contains('\n'), "the summary is one line");
    }
}
