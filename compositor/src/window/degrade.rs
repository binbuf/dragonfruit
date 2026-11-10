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
/// Sustained over-budget time before the tier steps down. This is the old
/// 12-frame window at 60 Hz (12 × 16.666 ms); at 120/144 Hz it is the same
/// wall-clock window, not 12 faster frames (T-21.2c).
const DOWNGRADE_WINDOW: Duration = Duration::from_millis(200);
/// Sustained clearly-under-budget time before the tier recovers. This is the
/// old 180-frame window at 60 Hz (180 × 16.666 ms = 3 s).
const UPGRADE_WINDOW: Duration = Duration::from_secs(3);
/// "Clearly under" is this fraction of the budget: recovery needs real
/// headroom, so a tier does not oscillate around the threshold.
const RECOVERY_RATIO: f64 = 0.75;
/// While a boundary is latched, recovery across it needs this much headroom
/// (a genuine idle/lighter load, not the 0.75 recovery band).
const LATCHED_RECOVERY_RATIO: f64 = 0.4;
/// ...and this long, so a sweep that merely dips under the threshold does not
/// retry every few seconds.
const LATCHED_UPGRADE_WINDOW: Duration = Duration::from_secs(10);
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
    ///   and, on the sampled path, never turns the sampled pass off because
    ///   the global controller visited `Minimal`. The Dock and menu bar stay
    ///   glass at `Reduced` instead of swapping to the milky fallback under a
    ///   transient load; that swap was the visible native flicker (T-21.5).
    ///   The milky fallback is reserved for a genuinely absent/failed sampled
    ///   pass (`NoSampledBlur`), not for a tier selection.
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
            // Persistent chrome has a hard perceptual floor: it never drops
            // below Reduced, and it keeps the sampled pass on whenever the
            // backend supports it. A global Minimal selection may still be
            // reported by `degrade stats`, but it no longer changes the Dock
            // or menu-bar material identity.
            let tier = if selected == DegradeTier::Minimal {
                DegradeTier::Reduced
            } else {
                selected
            };
            let blur_active = blur_supported && tier.blur_enabled();
            let fallback = if !blur_supported && tier.blur_enabled() {
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
        Some(spec.scaled(self.geometry_scale()))
    }

    /// The Tahoe liquid-glass pass at this tier (T-20.2), or `None` when blur
    /// is off. `Full` keeps refraction + specular + tint; `Reduced` keeps the
    /// specular rim, the adaptive tint and the measured flat-band corner lens,
    /// and drops only the generic edge-lens `refraction`. The corner lens is a
    /// cheap UV displacement, not the expensive edge pass, and it is the
    /// Dock/Control Center perimeter identity: the persistent roles live at
    /// `Reduced` by design (T-21.2c), so dropping it there would make the
    /// measured plate disappear on the normal path (T235a). The smaller blur
    /// radius still comes from [`Self::blur`].
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
    /// Time spent continuously over budget / clearly under budget. The
    /// hysteresis windows are wall-clock, so the same 12/180-frame behavior
    /// holds at 60 Hz and does not speed up at 120/144 Hz.
    over_budget_time: Duration,
    under_budget_time: Duration,
    /// Time since the last downgrade. The predictive upgrade gate uses the
    /// same wall-clock streak as the latch so a failed tier is not immediately
    /// re-entered just because the lower tier is cheap.
    since_downgrade: Duration,
    /// The higher-tier index of a boundary that flapped. Recovery across it
    /// requires the stricter latched gate until a successful upgrade.
    latched_boundary: Option<usize>,
    /// The last observed EMA while actually rendering at each tier. A tier
    /// that recently missed the deadline is not promoted back into while the
    /// current tier is already consuming meaningful budget.
    tier_ema_us: [Option<f64>; 3],
    over_budget_frames: u64,
    downgrades: u64,
    upgrades: u64,
    tier_frames: [u64; 3],
}

/// A target tier whose last measured EMA is above this multiple of the budget
/// is treated as a known-risky promotion while the current tier is already
/// using more than [`CURRENT_COMFORT_RATIO`] of the budget.
const TARGET_EMA_RISK_RATIO: f64 = 1.05;
/// The current tier must be below this fraction of the budget before a
/// known-risky target is allowed back without waiting for the latch gate.
const CURRENT_COMFORT_RATIO: f64 = 0.5;

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
            over_budget_time: Duration::ZERO,
            under_budget_time: Duration::ZERO,
            since_downgrade: Duration::ZERO,
            latched_boundary: None,
            tier_ema_us: [None; 3],
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
        self.reset_selection_memory();
    }

    /// Resume automatic selection. The current tier is kept until the
    /// observations move it, so releasing a pin never causes a jump.
    pub fn release(&mut self) {
        self.forced = None;
        self.reset_selection_memory();
    }

    /// Observe one rendered frame's duration and advance the selection.
    ///
    /// The duration feeds an exponential moving average so a single spike
    /// cannot degrade the UI; sustained pressure (or sustained headroom) walks
    /// the ladder one tier at a time. The windows are accumulated wall-clock
    /// time, so the 12-frame/180-frame 60 Hz reference becomes the same
    /// 200 ms/3 s behavior at 120 and 144 Hz. While pinned the tier never
    /// changes.
    pub fn observe(&mut self, duration: Duration) {
        let us = duration.as_micros() as f64;
        self.ema_us = if self.samples == 0 {
            us
        } else {
            self.ema_us * (1.0 - EMA_ALPHA) + us * EMA_ALPHA
        };
        self.samples += 1;
        let current_tier = self.tier.index();
        self.tier_frames[current_tier] += 1;
        // Keep a per-tier cost estimate. It is only a promotion guard: a
        // target tier that recently missed the deadline is not re-entered
        // while the current tier is already using meaningful budget.
        let tier_ema = self.tier_ema_us[current_tier].unwrap_or(us);
        self.tier_ema_us[current_tier] = Some(tier_ema * (1.0 - EMA_ALPHA) + us * EMA_ALPHA);
        if self.forced.is_some() {
            return;
        }
        self.since_downgrade = self.since_downgrade.saturating_add(duration);

        let budget = f64::from(self.budget_us);
        let (recovery_ratio, upgrade_window) = self.recovery_gate();
        if self.ema_us > budget {
            self.over_budget_time = self.over_budget_time.saturating_add(duration);
            self.under_budget_time = Duration::ZERO;
            self.over_budget_frames += 1;
        } else if self.ema_us < budget * recovery_ratio {
            self.under_budget_time = self.under_budget_time.saturating_add(duration);
            self.over_budget_time = Duration::ZERO;
        } else {
            self.reset_streaks();
        }

        if self.over_budget_time >= DOWNGRADE_WINDOW {
            let from = self.tier;
            self.reset_streaks();
            self.step(1);
            if self.tier != from {
                self.note_downgrade(from);
            }
        } else if self.tier.index() > 0 && self.under_budget_time >= upgrade_window {
            let target = self.tier.index() - 1;
            let target_risky =
                self.tier_ema_us[target].is_some_and(|cost| cost > budget * TARGET_EMA_RISK_RATIO);
            let current_comfortable = self.ema_us < budget * CURRENT_COMFORT_RATIO;
            if target_risky && !current_comfortable {
                // Keep accumulating the under-budget streak; the gate opens
                // as soon as the current tier has real headroom (or the load
                // changes enough to update the target estimate).
                return;
            }
            let from = self.tier;
            self.reset_streaks();
            self.step(-1);
            if self.tier != from {
                self.note_upgrade(from);
            }
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
             upgrades={} tiers=full:{},reduced:{},minimal:{} budget_source={} \
             down_window_us={} up_window_us={} latched={}",
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
            DOWNGRADE_WINDOW.as_micros(),
            UPGRADE_WINDOW.as_micros(),
            self.latched_boundary.is_some() as u32,
        )
    }

    /// The recovery gate for the boundary immediately above the current
    /// tier. A boundary that just flapped needs a genuinely lighter load
    /// (below [`LATCHED_RECOVERY_RATIO`]) for the longer
    /// [`LATCHED_UPGRADE_WINDOW`], so a steady pointer sweep cannot climb
    /// back into the tier that just missed the deadline.
    fn recovery_gate(&self) -> (f64, Duration) {
        if self.tier.index() > 0 && self.latched_boundary == Some(self.tier.index() - 1) {
            (LATCHED_RECOVERY_RATIO, LATCHED_UPGRADE_WINDOW)
        } else {
            (RECOVERY_RATIO, UPGRADE_WINDOW)
        }
    }

    /// Record a real step down.
    ///
    /// Every downgrade latches the boundary, not only a downgrade that follows
    /// a recent upgrade. A tier that just missed the deadline is presumed
    /// unstable until the session proves sustained headroom below the stricter
    /// latch threshold. This is the second half of the native Dock fix: the
    /// previous probation-only latch still allowed the first Full -> Reduced
    /// -> Full cycle and a Reduced <-> Minimal cycle.
    fn note_downgrade(&mut self, from: DegradeTier) {
        self.latched_boundary = Some(from.index());
        self.since_downgrade = Duration::ZERO;
    }

    /// Record a real step up. A successful recovery across a latched
    /// boundary clears the latch; the next downgrade will latch it again if
    /// the load has not actually settled.
    fn note_upgrade(&mut self, from: DegradeTier) {
        let boundary = from.index() - 1;
        if self.latched_boundary == Some(boundary) {
            self.latched_boundary = None;
        }
        self.since_downgrade = Duration::ZERO;
    }

    fn reset_streaks(&mut self) {
        self.over_budget_time = Duration::ZERO;
        self.under_budget_time = Duration::ZERO;
    }

    fn reset_selection_memory(&mut self) {
        self.reset_streaks();
        self.since_downgrade = Duration::ZERO;
        self.latched_boundary = None;
        self.tier_ema_us = [None; 3];
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
    fn persistent_roles_hold_the_reduced_floor_with_blur_on() {
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

            // A global Minimal selection keeps persistent chrome at the
            // Reduced floor with the sampled pass still on. It must not swap
            // the Dock/menu bar to the milky fallback while the user is
            // interacting; the cost problem belongs to the region/blur path.
            let floored = EffectiveMaterial::resolve_for_role(
                role,
                MaterialPath::Gpu,
                true,
                DegradeTier::Minimal,
            );
            assert_eq!(floored.role, role);
            assert_eq!(floored.tier, DegradeTier::Reduced);
            assert!(floored.blur_active);
            assert_eq!(floored.fallback, MaterialFallback::None);
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
    fn reduced_keeps_the_measured_flat_band_corner_lens() {
        // T235a: the persistent Dock/Control Center roles live at Reduced by
        // design (T-21.2c). The flat-band corner lens is the measured plate
        // identity, not the expensive generic edge pass, so Reduced must keep
        // it. Only the non-flat edge-lens refraction drops.
        let flat = GlassSpec {
            flat_band: true,
            refraction: 10.0,
            corner_peak: 13.6,
            ..GlassSpec::default()
        };
        let reduced = DegradeTier::Reduced
            .glass(flat)
            .expect("reduced keeps the sampled pass");
        assert_eq!(reduced.corner_peak, flat.corner_peak);
        assert_eq!(reduced.refraction, 0.0);

        // The non-flat path still drops its generic edge lens and keeps the
        // corner field untouched (the shader only reads it for flat bands).
        let edge = GlassSpec {
            flat_band: false,
            ..flat
        };
        let reduced_edge = DegradeTier::Reduced.glass(edge).expect("reduced keeps blur");
        assert_eq!(reduced_edge.refraction, 0.0);
        assert_eq!(reduced_edge.corner_peak, edge.corner_peak);

        // Minimal has no sampled pass, so the fallback owns the plate.
        assert_eq!(DegradeTier::Minimal.glass(flat), None);
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

    /// Feed frames for `duration` of wall-clock time at `frame_us` per frame,
    /// at least once. This is how the refresh-aware tests express a window
    /// without depending on a frame count.
    fn frames_for(controller: &mut DegradeController, duration: Duration, frame_us: u64) {
        let frame = Duration::from_micros(frame_us);
        let mut elapsed = Duration::ZERO;
        while elapsed < duration {
            controller.observe(frame);
            elapsed = elapsed.saturating_add(frame);
        }
    }

    #[test]
    fn sustained_over_budget_time_steps_the_ladder_down_to_minimal() {
        let mut controller = DegradeController::with_budget_us(16_000);
        assert_eq!(controller.tier(), DegradeTier::Full);

        // One full 200 ms downgrade window of clearly-over-budget frames.
        frames(&mut controller, 10, 20_000);
        assert_eq!(controller.tier(), DegradeTier::Reduced);
        assert_eq!(controller.downgrades(), 1);

        frames(&mut controller, 10, 20_000);
        assert_eq!(controller.tier(), DegradeTier::Minimal);
        assert_eq!(controller.downgrades(), 2);

        // Minimal saturates: it cannot degrade further.
        frames(&mut controller, 40, 50_000);
        assert_eq!(controller.tier(), DegradeTier::Minimal);
        assert_eq!(controller.downgrades(), 2);
        assert!(controller.over_budget_frames() > 0);
        assert_eq!(controller.samples(), 60);
    }

    #[test]
    fn the_hysteresis_windows_are_refresh_aware_at_60_120_and_144_hz() {
        // The reference is 12 over-budget frames at 60 Hz: 200 ms down. A
        // faster panel must take the same wall-clock time, not 2.4x fewer
        // milliseconds. A real downgrade latches the boundary, so recovery
        // uses the stricter 10 s latch window; the normal 3 s window is
        // covered by the forced/released controller below.
        for (hz, period_us, over_us, latched_under_us) in [
            (60u32, 16_666u64, 20_000u64, 5_000u64),
            (120, 8_333, 10_000, 2_500),
            (144, 6_944, 8_333, 2_000),
        ] {
            let mut controller = DegradeController::with_budget_us(period_us as u32);

            // Downgrade after ~200 ms, independent of the refresh rate.
            let mut downgrade_at = Duration::ZERO;
            let mut downgrade_frames = 0u32;
            let mut elapsed = Duration::ZERO;
            while controller.tier() == DegradeTier::Full {
                controller.observe(Duration::from_micros(over_us));
                elapsed = elapsed.saturating_add(Duration::from_micros(over_us));
                downgrade_frames += 1;
                if controller.tier() == DegradeTier::Reduced {
                    downgrade_at = elapsed;
                }
                assert!(elapsed < Duration::from_secs(1), "{hz} Hz never downgraded");
            }
            assert!(
                downgrade_at >= Duration::from_millis(190)
                    && downgrade_at <= Duration::from_millis(220),
                "{hz} Hz downgraded after {downgrade_at:?}, not ~200 ms"
            );

            // Every downgrade latches the boundary. Recovery needs the
            // stricter under-0.4x-budget headroom for ~10 s, not the normal
            // 3 s window.
            let mut upgrade_at = Duration::ZERO;
            let mut upgrade_frames = 0u32;
            elapsed = Duration::ZERO;
            while controller.tier() == DegradeTier::Reduced {
                controller.observe(Duration::from_micros(latched_under_us));
                elapsed = elapsed.saturating_add(Duration::from_micros(latched_under_us));
                upgrade_frames += 1;
                if controller.tier() == DegradeTier::Full {
                    upgrade_at = elapsed;
                }
                assert!(
                    elapsed < Duration::from_secs(12),
                    "{hz} Hz never recovered across the latched boundary"
                );
            }
            assert!(
                upgrade_at >= Duration::from_millis(9_950)
                    && upgrade_at <= Duration::from_millis(10_150),
                "{hz} Hz recovered after {upgrade_at:?}, not ~10 s"
            );
            println!(
                "degrade-trace: rate_hz={hz} period_us={period_us} \
                 downgrade_frames={downgrade_frames} downgrade_ms={:.3} \
                 upgrade_frames={upgrade_frames} latched_upgrade_ms={:.3}",
                downgrade_at.as_secs_f64() * 1_000.0,
                upgrade_at.as_secs_f64() * 1_000.0,
            );
        }

        // Without a latch (a forced/released tier), the normal 3 s recovery
        // window still applies at 60/120/144 Hz.
        for (hz, period_us, under_us) in [
            (60u32, 16_666u64, 8_000u64),
            (120, 8_333, 4_000),
            (144, 6_944, 3_472),
        ] {
            let mut controller = DegradeController::with_budget_us(period_us as u32);
            controller.force(DegradeTier::Reduced);
            controller.release();
            let mut elapsed = Duration::ZERO;
            while controller.tier() == DegradeTier::Reduced {
                controller.observe(Duration::from_micros(under_us));
                elapsed = elapsed.saturating_add(Duration::from_micros(under_us));
                assert!(elapsed < Duration::from_secs(5), "{hz} Hz never recovered");
            }
            assert!(
                elapsed >= Duration::from_millis(2_950) && elapsed <= Duration::from_millis(3_150),
                "{hz} Hz normal recovery took {elapsed:?}, not ~3 s"
            );
            println!(
                "degrade-trace: normal_recovery_hz={hz} recovery_ms={:.3}",
                elapsed.as_secs_f64() * 1_000.0,
            );
        }
    }

    #[test]
    fn a_steady_pointer_sweep_does_not_repeat_full_reduced_flapping() {
        // A steady sweep whose cost depends on the tier: Full misses the
        // budget, Reduced is comfortably under the recovery threshold, and
        // Minimal is cheaper still. Without a flap guard this is the classic
        // Full -> Reduced -> Full -> Reduced limit cycle.
        for (hz, period_us) in [(60u32, 16_666u64), (120, 8_333), (144, 6_944)] {
            let budget_us = period_us as u32;
            let mut controller = DegradeController::with_budget_us(budget_us);
            let full_us = (period_us as f64 * 1.25) as u64;
            let reduced_us = (period_us as f64 * 0.6) as u64;
            let minimal_us = (period_us as f64 * 0.4) as u64;

            let mut elapsed = Duration::ZERO;
            let sweep = Duration::from_secs(60);
            while elapsed < sweep {
                let frame_us = match controller.tier() {
                    DegradeTier::Full => full_us,
                    DegradeTier::Reduced => reduced_us,
                    DegradeTier::Minimal => minimal_us,
                };
                controller.observe(Duration::from_micros(frame_us));
                elapsed = elapsed.saturating_add(Duration::from_micros(frame_us));
            }

            // One initial downgrade, then the always-on latch holds Reduced
            // for the rest of the sweep. There is no probationary climb back
            // into the tier that just missed the deadline.
            assert_eq!(
                controller.tier(),
                DegradeTier::Reduced,
                "{hz} Hz ended at the wrong tier after a steady sweep"
            );
            assert_eq!(
                controller.downgrades(),
                1,
                "{hz} Hz downgrades must stay bounded"
            );
            assert_eq!(
                controller.upgrades(),
                0,
                "{hz} Hz must not climb back into the failing tier"
            );
            assert!(
                controller.summary().contains("latched=1"),
                "{hz} Hz must latch the flapping boundary"
            );
            println!(
                "degrade-trace: steady_sweep_hz={hz} duration_s=60 \
                 full_frame_us={full_us} reduced_frame_us={reduced_us} \
                 downgrades={} upgrades={} latched=1 final_tier={}",
                controller.downgrades(),
                controller.upgrades(),
                controller.tier().name(),
            );

            // The latch is not permanent: a genuinely lighter load (below the
            // 0.4 budget latch threshold) for 10 s recovers to Full.
            let idle_us = (period_us as f64 * 0.3) as u64;
            frames_for(&mut controller, Duration::from_secs(11), idle_us);
            assert_eq!(
                controller.tier(),
                DegradeTier::Full,
                "{hz} Hz must recover after a sustained idle load"
            );
            println!(
                "degrade-trace: steady_sweep_idle_hz={hz} idle_frame_us={idle_us} \
                 final_tier={} upgrades={}",
                controller.tier().name(),
                controller.upgrades(),
            );
        }
    }

    #[test]
    fn isolated_spikes_do_not_degrade() {
        let mut controller = DegradeController::with_budget_us(16_000);
        // A single 100 ms hitch, then many fast frames: the over-budget time
        // never reaches the downgrade window.
        controller.observe(Duration::from_millis(100));
        frames(&mut controller, 200, 5_000);
        assert_eq!(controller.tier(), DegradeTier::Full);
        assert_eq!(controller.downgrades(), 0);
    }

    #[test]
    fn sustained_headroom_recovers_toward_full() {
        let mut controller = DegradeController::with_budget_us(16_000);
        frames(&mut controller, 20, 25_000);
        assert_eq!(controller.tier(), DegradeTier::Minimal);

        // Every downgrade latches its boundary, so recovery needs ~10 s of
        // genuinely light frames per boundary and climbs one tier at a time.
        // The first seconds also absorb the EMA warm-up from the degraded
        // frame time before the under-budget streak can start.
        frames_for(&mut controller, Duration::from_secs(11), 4_000);
        assert_eq!(controller.tier(), DegradeTier::Reduced);
        assert_eq!(controller.upgrades(), 1);

        frames_for(&mut controller, Duration::from_secs(11), 4_000);
        assert_eq!(controller.tier(), DegradeTier::Full);
        assert_eq!(controller.upgrades(), 2);
    }

    #[test]
    fn forcing_a_tier_pins_it_through_load_and_release_resumes() {
        let mut controller = DegradeController::with_budget_us(16_000);
        controller.force(DegradeTier::Full);
        assert_eq!(controller.forced(), Some(DegradeTier::Full));

        // Heavy load cannot move a pinned tier.
        frames(&mut controller, 40, 90_000);
        assert_eq!(controller.tier(), DegradeTier::Full);
        assert_eq!(controller.downgrades(), 0);

        // Releasing keeps the tier until the observations move it.
        controller.release();
        assert_eq!(controller.forced(), None);
        assert_eq!(controller.tier(), DegradeTier::Full);
        frames(&mut controller, 8, 30_000);
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
            "down_window_us=200000",
            "up_window_us=3000000",
            "latched=0",
        ] {
            assert!(summary.contains(field), "missing {field:?} in {summary:?}");
        }
        assert!(!summary.contains('\n'), "the summary is one line");
    }
}
