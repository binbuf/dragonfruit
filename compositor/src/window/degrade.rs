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
use crate::window::backdrop::{BackdropSpec, MaterialRole};
use crate::window::blur::{BlurSpec, GlassSpec};
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

/// Where the controller's frame budget came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetSource {
    /// The fastest active output's refresh deadline (the normal policy).
    #[default]
    Refresh,
    /// `DRAGONFRUIT_FRAME_BUDGET_US` (diagnostic/test override).
    Env,
    /// `set degrade-budget` / [`DegradeController::set_budget_us`] (manual).
    Manual,
}

impl BudgetSource {
    /// The stable token reported on the degrade stats line.
    pub const fn name(self) -> &'static str {
        match self {
            BudgetSource::Refresh => "refresh",
            BudgetSource::Env => "env",
            BudgetSource::Manual => "manual",
        }
    }

    /// Whether this source pins the budget against the refresh deadline.
    pub const fn is_override(self) -> bool {
        !matches!(self, BudgetSource::Refresh)
    }
}

/// The environment variable that overrides the frame budget, in microseconds.
/// Unset in a normal session; a nested/DRM acceptance run can pin it.
pub const ENV_FRAME_BUDGET_US: &str = "DRAGONFRUIT_FRAME_BUDGET_US";

/// The renderer class the chrome material runs on (T-20.4).
///
/// The GPU path (`Full`/`Reduced`) samples the live scene texture through the
/// custom shaders. The **software/headless** path has no GPU readback, so it
/// renders the deterministic `Minimal` mapping regardless of the budget tier
/// the controller selected — the material is then a *quality* fallback that
/// uses the same `MaterialRole`/`BackdropSpec` inputs, never a correctness one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MaterialPath {
    /// A GLES 3.x / Vulkan-capable GPU: the sampled blur + liquid-glass pass.
    #[default]
    Gpu,
    /// The software/headless path: no scene texture, no readback, the
    /// deterministic `Minimal` feather stack.
    Software,
}

impl MaterialPath {
    /// The stable name used by the render/degrade trace.
    pub const fn name(self) -> &'static str {
        match self {
            MaterialPath::Gpu => "gpu",
            MaterialPath::Software => "software",
        }
    }

    /// Whether the GPU sampled-material pass (scene texture + blur/glass) can
    /// run on this path. `false` means the material must stay on the
    /// deterministic feather stack.
    pub const fn gpu_blur(self) -> bool {
        matches!(self, MaterialPath::Gpu)
    }

    /// The tier the material actually renders at: the selected tier on the GPU
    /// path, always `Minimal` on the software path.
    pub const fn material_tier(self, selected: DegradeTier) -> DegradeTier {
        match self {
            MaterialPath::Gpu => selected,
            MaterialPath::Software => DegradeTier::Minimal,
        }
    }
}

/// Why the sampled material is not active on an output. This is the
/// **effective** state reported to the shell (P1), not the requested tier:
/// `NoSampledBlur` means the backend is on the GPU path but has no sampled
/// blur implementation (the current DRM rail), while `Software` is the
/// headless/no-readback path. `None` means the sampled pass is active, or the
/// tier intentionally turned it off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialFallback {
    None,
    NoSampledBlur,
    Software,
    MultiGpu,
    /// The sampled path exists, but the controller selected `Minimal` (even
    /// `Reduced` missed the frame deadline). Persistent chrome keeps its
    /// `Reduced` floor tier and takes the deliberate milky fallback rather
    /// than silently becoming a flat translucent slab (T-21.2c).
    FloorMissed,
}

impl MaterialFallback {
    /// The stable name used by logs and tests.
    pub const fn name(self) -> &'static str {
        match self {
            MaterialFallback::None => "none",
            MaterialFallback::NoSampledBlur => "no-sampled-blur",
            MaterialFallback::Software => "software",
            MaterialFallback::MultiGpu => "multi-gpu",
            MaterialFallback::FloorMissed => "floor-missed",
        }
    }

    /// The wire value for `df_output.material_state.fallback` and
    /// `df_output.material_role_state.fallback`.
    pub const fn wire(self) -> u32 {
        match self {
            MaterialFallback::None => 0,
            MaterialFallback::NoSampledBlur => 1,
            MaterialFallback::Software => 2,
            MaterialFallback::MultiGpu => 3,
            MaterialFallback::FloorMissed => 4,
        }
    }
}

/// The material the compositor actually applies to chrome on an output
/// (P1). The shell uses this to choose the translucent glass complement or
/// the milky fallback; it must not re-derive the degrade policy in QML.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveMaterial {
    /// The role this snapshot applies to. The legacy v3 `material_state`
    /// event reports the global snapshot as [`MaterialRole::Chrome`]; the v4
    /// `material_role_state` event carries one snapshot per role.
    pub role: MaterialRole,
    pub tier: DegradeTier,
    /// True only when the sampled backdrop pass actually runs.
    pub blur_active: bool,
    /// Why the sampled pass is absent, when it is absent for a non-tier
    /// reason.
    pub fallback: MaterialFallback,
}

impl EffectiveMaterial {
    /// Resolve the **global** effective material from the renderer path,
    /// whether the backend implements the sampled pass, and the selected
    /// degrade tier. This keeps the legacy v3 `material_state` meaning: the
    /// selected tier is not role-floored here. Use [`Self::resolve_for_role`]
    /// for the per-role state the shell applies.
    pub fn resolve(path: MaterialPath, blur_supported: bool, selected: DegradeTier) -> Self {
        let tier = path.material_tier(selected);
        let blur_active = path.gpu_blur() && blur_supported && tier.blur_enabled();
        let fallback = if !path.gpu_blur() {
            MaterialFallback::Software
        } else if !blur_supported && tier.blur_enabled() {
            MaterialFallback::NoSampledBlur
        } else {
            MaterialFallback::None
        };
        Self {
            role: MaterialRole::Chrome,
            tier,
            blur_active,
            fallback,
        }
    }

    /// Resolve the effective material for one [`MaterialRole`] (T-21.2c).
    ///
    /// * Persistent chrome (`Chrome`/`Dock`) never selects below `Reduced`
    ///   while the sampled path is available. If the controller selected
    ///   `Minimal`, even `Reduced` missed the deadline: keep the `Reduced`
    ///   floor tier, turn the sampled pass off, and report
    ///   [`MaterialFallback::FloorMissed`] so the shell draws the deliberate
    ///   milky plate instead of a flat translucent slab.
    /// * Transient roles (`Popup`/`Drawer`) follow the ladder to `Minimal`.
    /// * The software/headless path always resolves to `Minimal` with the
    ///   [`MaterialFallback::Software`] reason for every role: there is no
    ///   sampled path to floor, and the deterministic milky plate is the
    ///   contract.
    pub fn resolve_for_role(
        role: MaterialRole,
        path: MaterialPath,
        blur_supported: bool,
        selected: DegradeTier,
    ) -> Self {
        if !path.gpu_blur() {
            return Self {
                role,
                tier: DegradeTier::Minimal,
                blur_active: false,
                fallback: MaterialFallback::Software,
            };
        }

        if role.is_persistent() {
            let floor_missed = selected == DegradeTier::Minimal;
            let tier = if floor_missed {
                DegradeTier::Reduced
            } else {
                selected
            };
            let blur_active = blur_supported && tier.blur_enabled() && !floor_missed;
            let fallback = if floor_missed {
                if blur_supported {
                    MaterialFallback::FloorMissed
                } else {
                    MaterialFallback::NoSampledBlur
                }
            } else if !blur_supported && tier.blur_enabled() {
                MaterialFallback::NoSampledBlur
            } else {
                MaterialFallback::None
            };
            return Self {
                role,
                tier,
                blur_active,
                fallback,
            };
        }

        let tier = selected;
        let blur_active = blur_supported && tier.blur_enabled();
        let fallback = if !blur_supported && tier.blur_enabled() {
            MaterialFallback::NoSampledBlur
        } else {
            MaterialFallback::None
        };
        Self {
            role,
            tier,
            blur_active,
            fallback,
        }
    }
}

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

    /// The GPU backdrop blur at this tier, or `None` when blur is off. The
    /// token blur radius and corner radius scale with the tier, and the Kawase
    /// iteration count is re-derived from the scaled radius (T-20.1).
    pub fn blur(self, spec: BlurSpec) -> Option<BlurSpec> {
        if !self.blur_enabled() {
            return None;
        }
        Some(BlurSpec::new(
            spec.radius * self.geometry_scale(),
            spec.corner_radius * self.geometry_scale(),
        ))
    }

    /// The Tahoe liquid-glass pass at this tier (T-20.2), or `None` when blur
    /// is off. `Full` keeps refraction + specular + tint; `Reduced` keeps the
    /// specular rim and the adaptive tint but drops the edge lens (the
    /// expensive part), and the smaller blur radius comes from [`Self::blur`].
    pub fn glass(self, spec: GlassSpec) -> Option<GlassSpec> {
        if !self.blur_enabled() {
            return None;
        }
        Some(match self {
            DegradeTier::Full => spec,
            DegradeTier::Reduced => GlassSpec {
                refraction: 0.0,
                ..spec
            },
            // `blur_enabled` is false at `Minimal`, handled above.
            DegradeTier::Minimal => spec,
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
    /// Where `budget_us` came from. A non-refresh source pins the budget so
    /// the output refresh deadline cannot overwrite a diagnostic override.
    budget_source: BudgetSource,
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
    /// ([`animation::FRAME_INTERVAL`]) until the output refresh deadline is
    /// wired, overridable through [`ENV_FRAME_BUDGET_US`].
    pub fn new() -> Self {
        let env_budget = std::env::var(ENV_FRAME_BUDGET_US)
            .ok()
            .and_then(|value| value.parse::<u32>().ok());
        let mut controller =
            Self::with_budget_us(env_budget.unwrap_or(FRAME_INTERVAL.as_micros() as u32));
        if env_budget.is_some() {
            controller.budget_source = BudgetSource::Env;
            eprintln!(
                "dragonfruit-compositor: frame budget override active \
                 ({ENV_FRAME_BUDGET_US}={}us); the output deadline will not replace it",
                controller.budget_us
            );
        }
        controller
    }

    /// A controller at [`Full`] with an explicit budget.
    pub fn with_budget_us(budget_us: u32) -> Self {
        Self {
            tier: DegradeTier::Full,
            forced: None,
            budget_us: budget_us.clamp(MIN_BUDGET_US, MAX_BUDGET_US),
            budget_source: BudgetSource::Refresh,
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

    /// Where the current budget came from.
    pub fn budget_source(&self) -> BudgetSource {
        self.budget_source
    }

    /// Set the frame budget manually. This pins it as a diagnostic override:
    /// later output refresh changes do not replace it. Does not change the
    /// tier; the next observations decide. Clamped to a sane range.
    pub fn set_budget_us(&mut self, budget_us: u32) {
        self.budget_us = budget_us.clamp(MIN_BUDGET_US, MAX_BUDGET_US);
        self.budget_source = BudgetSource::Manual;
    }

    /// Set the budget from the fastest active output's refresh deadline. A
    /// diagnostic override (environment or manual) wins, so an acceptance run
    /// can pin a budget without the clock policy overwriting it.
    pub fn set_budget_from_deadline(&mut self, deadline: Duration) {
        if self.budget_source.is_override() {
            return;
        }
        let us = deadline.as_micros().min(u128::from(u32::MAX)) as u32;
        self.budget_us = us.clamp(MIN_BUDGET_US, MAX_BUDGET_US);
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
             upgrades={} tiers=full:{},reduced:{},minimal:{} budget_source={}",
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
            self.budget_source.name(),
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
    fn the_software_path_always_resolves_to_minimal() {
        assert_eq!(MaterialPath::default(), MaterialPath::Gpu);
        assert!(MaterialPath::Gpu.gpu_blur());
        assert!(!MaterialPath::Software.gpu_blur());
        // The GPU path honors the selected tier; the software path collapses
        // every tier to Minimal so headless/software output is deterministic.
        for tier in DegradeTier::ALL {
            assert_eq!(MaterialPath::Gpu.material_tier(tier), tier);
            assert_eq!(
                MaterialPath::Software.material_tier(tier),
                DegradeTier::Minimal
            );
        }
        assert_eq!(MaterialPath::Gpu.name(), "gpu");
        assert_eq!(MaterialPath::Software.name(), "software");
    }

    #[test]
    fn effective_material_reports_the_applied_pass_not_the_requested_tier() {
        // Nested/GPU with a sampled pass: the tier is the selected one and
        // blur is active whenever the tier keeps it on.
        let full = EffectiveMaterial::resolve(MaterialPath::Gpu, true, DegradeTier::Full);
        assert_eq!(full.tier, DegradeTier::Full);
        assert!(full.blur_active);
        assert_eq!(full.fallback, MaterialFallback::None);

        let reduced = EffectiveMaterial::resolve(MaterialPath::Gpu, true, DegradeTier::Reduced);
        assert_eq!(reduced.tier, DegradeTier::Reduced);
        assert!(reduced.blur_active);
        assert_eq!(reduced.fallback, MaterialFallback::None);

        // Minimal is an intentional blur-off, not a fallback failure.
        let minimal = EffectiveMaterial::resolve(MaterialPath::Gpu, true, DegradeTier::Minimal);
        assert_eq!(minimal.tier, DegradeTier::Minimal);
        assert!(!minimal.blur_active);
        assert_eq!(minimal.fallback, MaterialFallback::None);

        // The current DRM rail: GPU renderer, no sampled pass. This is the
        // state that must make the shell choose the milky fallback.
        let drm = EffectiveMaterial::resolve(MaterialPath::Gpu, false, DegradeTier::Full);
        assert_eq!(drm.tier, DegradeTier::Full);
        assert!(!drm.blur_active);
        assert_eq!(drm.fallback, MaterialFallback::NoSampledBlur);

        // Headless/software always resolves to Minimal and reports the
        // software path even if the controller selected Full.
        let software = EffectiveMaterial::resolve(MaterialPath::Software, false, DegradeTier::Full);
        assert_eq!(software.tier, DegradeTier::Minimal);
        assert!(!software.blur_active);
        assert_eq!(software.fallback, MaterialFallback::Software);

        assert_eq!(MaterialFallback::None.wire(), 0);
        assert_eq!(MaterialFallback::NoSampledBlur.wire(), 1);
        assert_eq!(MaterialFallback::Software.wire(), 2);
        assert_eq!(MaterialFallback::MultiGpu.wire(), 3);
        assert_eq!(MaterialFallback::FloorMissed.wire(), 4);
        assert_eq!(MaterialFallback::NoSampledBlur.name(), "no-sampled-blur");
        assert_eq!(MaterialFallback::FloorMissed.name(), "floor-missed");
    }

    #[test]
    fn material_roles_classify_as_persistent_or_transient() {
        assert_eq!(MaterialRole::ALL.len(), MaterialRole::COUNT);
        for (index, role) in MaterialRole::ALL.into_iter().enumerate() {
            assert_eq!(role.wire(), index as u32);
            assert_eq!(MaterialRole::from_wire(role.wire()), Some(role));
            assert_eq!(role.is_transient(), !role.is_persistent());
        }
        for role in [MaterialRole::Chrome, MaterialRole::Dock] {
            assert!(role.is_persistent(), "{} must be persistent", role.name());
            assert!(!role.is_transient());
        }
        for role in [MaterialRole::Popup, MaterialRole::Drawer] {
            assert!(role.is_transient(), "{} must be transient", role.name());
            assert!(!role.is_persistent());
        }
        assert_eq!(MaterialRole::from_wire(4), None);
    }

    #[test]
    fn persistent_roles_hold_the_reduced_floor_on_the_sampled_path() {
        // Full/Reduced pass through unchanged for both persistent roles.
        for role in [MaterialRole::Chrome, MaterialRole::Dock] {
            for tier in [DegradeTier::Full, DegradeTier::Reduced] {
                let material =
                    EffectiveMaterial::resolve_for_role(role, MaterialPath::Gpu, true, tier);
                assert_eq!(material.role, role);
                assert_eq!(material.tier, tier);
                assert!(material.blur_active);
                assert_eq!(material.fallback, MaterialFallback::None);
            }

            // The controller selected Minimal because even Reduced missed the
            // deadline. Persistent chrome keeps the Reduced tier and reports
            // the explicit milky-fallback reason instead of dropping to a flat
            // translucent Minimal slab.
            let floor_missed = EffectiveMaterial::resolve_for_role(
                role,
                MaterialPath::Gpu,
                true,
                DegradeTier::Minimal,
            );
            assert_eq!(floor_missed.role, role);
            assert_eq!(floor_missed.tier, DegradeTier::Reduced);
            assert!(!floor_missed.blur_active);
            assert_eq!(floor_missed.fallback, MaterialFallback::FloorMissed);
        }
    }

    #[test]
    fn transient_roles_may_reach_minimal() {
        for role in [MaterialRole::Popup, MaterialRole::Drawer] {
            for tier in DegradeTier::ALL {
                let material =
                    EffectiveMaterial::resolve_for_role(role, MaterialPath::Gpu, true, tier);
                assert_eq!(material.role, role);
                assert_eq!(material.tier, tier);
                assert_eq!(material.blur_active, tier.blur_enabled());
                assert_eq!(material.fallback, MaterialFallback::None);
            }
        }
    }

    #[test]
    fn an_absent_sampled_path_reports_its_reason_per_role() {
        // GPU without the sampled pass: persistent chrome still keeps the
        // Reduced floor and the existing no-sampled-blur reason.
        for role in [MaterialRole::Chrome, MaterialRole::Dock] {
            let material = EffectiveMaterial::resolve_for_role(
                role,
                MaterialPath::Gpu,
                false,
                DegradeTier::Full,
            );
            assert_eq!(material.tier, DegradeTier::Full);
            assert!(!material.blur_active);
            assert_eq!(material.fallback, MaterialFallback::NoSampledBlur);

            let floor_missed = EffectiveMaterial::resolve_for_role(
                role,
                MaterialPath::Gpu,
                false,
                DegradeTier::Minimal,
            );
            assert_eq!(floor_missed.tier, DegradeTier::Reduced);
            assert!(!floor_missed.blur_active);
            assert_eq!(floor_missed.fallback, MaterialFallback::NoSampledBlur);
        }
        for role in [MaterialRole::Popup, MaterialRole::Drawer] {
            let material = EffectiveMaterial::resolve_for_role(
                role,
                MaterialPath::Gpu,
                false,
                DegradeTier::Full,
            );
            assert_eq!(material.tier, DegradeTier::Full);
            assert!(!material.blur_active);
            assert_eq!(material.fallback, MaterialFallback::NoSampledBlur);

            // Minimal is an intentional blur-off, not a missing-pass reason.
            let minimal = EffectiveMaterial::resolve_for_role(
                role,
                MaterialPath::Gpu,
                false,
                DegradeTier::Minimal,
            );
            assert_eq!(minimal.tier, DegradeTier::Minimal);
            assert!(!minimal.blur_active);
            assert_eq!(minimal.fallback, MaterialFallback::None);
        }
    }

    #[test]
    fn the_software_path_stays_minimal_for_every_role() {
        for role in MaterialRole::ALL {
            for selected in DegradeTier::ALL {
                let material = EffectiveMaterial::resolve_for_role(
                    role,
                    MaterialPath::Software,
                    false,
                    selected,
                );
                assert_eq!(material.role, role);
                assert_eq!(material.tier, DegradeTier::Minimal);
                assert!(!material.blur_active);
                assert_eq!(material.fallback, MaterialFallback::Software);
            }
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
        assert_eq!(controller.budget_source(), BudgetSource::Refresh);
        controller.set_budget_us(0);
        assert_eq!(controller.budget_us(), MIN_BUDGET_US);
        controller.set_budget_us(u32::MAX);
        assert_eq!(controller.budget_us(), MAX_BUDGET_US);
        assert_eq!(controller.budget_source(), BudgetSource::Manual);
    }

    #[test]
    fn the_refresh_deadline_sets_the_budget_until_an_override_pins_it() {
        let mut controller = DegradeController::with_budget_us(16_000);

        // A 144 Hz output's deadline is 6.944 ms.
        controller.set_budget_from_deadline(Duration::from_nanos(6_944_444));
        assert_eq!(controller.budget_us(), 6_944);
        assert_eq!(controller.budget_source(), BudgetSource::Refresh);

        // A later mode change follows the new deadline.
        controller.set_budget_from_deadline(Duration::from_nanos(16_666_666));
        assert_eq!(controller.budget_us(), 16_666);

        // A manual diagnostic override wins and is recorded.
        controller.set_budget_us(12_000);
        controller.set_budget_from_deadline(Duration::from_nanos(4_166_666));
        assert_eq!(controller.budget_us(), 12_000);
        assert_eq!(controller.budget_source(), BudgetSource::Manual);
        assert!(controller.summary().contains("budget_source=manual"));
    }

    #[test]
    fn an_env_budget_pins_the_deadline_like_a_manual_override() {
        let mut controller = DegradeController::with_budget_us(12_000);
        controller.budget_source = BudgetSource::Env;
        controller.set_budget_from_deadline(Duration::from_nanos(6_944_444));
        assert_eq!(controller.budget_us(), 12_000);
        assert_eq!(controller.budget_source(), BudgetSource::Env);
        assert!(controller.summary().contains("budget_source=env"));
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
            "budget_source=refresh",
        ] {
            assert!(summary.contains(field), "missing {field:?} in {summary:?}");
        }
        assert!(!summary.contains('\n'), "the summary is one line");
    }
}
