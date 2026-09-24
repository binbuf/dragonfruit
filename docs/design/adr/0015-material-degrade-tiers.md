# 0015 — Material degrade tiers are one token-scaling ladder and one budget selector

## Status

accepted

## Context

The T-04 materials are the first compositor work with a real iGPU frame-budget
risk: the chrome backdrop (`BackdropPass`, ADR 0013) and the elevation shadows
(T-04.1a) emit many translucent layers per surface, and T-05's Mission Control
and T-11's Control Center will compose them over live surfaces. T-04.4a must
make them give ground under budget pressure — the track names "smaller radius
→ blur off" — without each future feature inventing its own fallback. T-05.1b
("degrade tier applied"), T-05.6 ("apply T-04 degrade tiers under pressure"),
and T-04.4b (schemes/reduced-motion sign-off) all consume the same policy.

The open questions were: what the tier ladder is, how a tier maps onto a
token-resolved material, and how the tier is *selected* and *instrumented*
without a second effect pass or an idle redraw.

## Decision

- Add `compositor/src/window/degrade.rs` as the one degrade policy.
  `DegradeTier` is an ordered ladder `Full` → `Reduced` → `Minimal`. A tier
  maps a token-resolved `BackdropSpec`/`ShadowSpec` through pure geometry
  scaling (`geometry_scale` / `layer_scale`, layer count never below 1);
  `Minimal` turns the backdrop blur **off** (`backdrop()` returns `None`) and
  collapses the shadow to a small tight ring. No token opacity or tone is
  invented — the material tokens stay the single source (ADR 0011/0013).
- `DegradeController` selects the tier from observed rendered-frame durations
  against a frame budget (default `animation::FRAME_INTERVAL`, 16 ms;
  overridable with `DRAGONFRUIT_FRAME_BUDGET_US`). It uses an exponential
  moving average plus hysteresis (a downgrade window of sustained over-budget
  frames, a longer recovery window of clearly-under-budget frames), so a stray
  spike cannot degrade the UI and a tier cannot oscillate. `force()` pins a
  tier for tests and for a feature that wants determinism.
- The session loop hands the controller each rendered frame's duration (the
  same duration `RenderStats` records), so selection is fed by the existing
  frame path and is inert while idle: it never forces a redraw.
- The render layer applies the tier where the material is resolved:
  `render::chrome_backdrop_render_elements` maps each `MaterialRole` spec
  through the tier and, at `Minimal`, emits no backdrop elements and records
  no pass damage; `render::window_shadow_render_elements` maps the elevation
  spec through the tier. The lifecycle motion and the T-04.3 scene transform
  are untouched: the tier changes material geometry, never the scene mapping,
  so T-02 transitions render identically at every tier.
- The tier and its selection are instrumented on a new `degrade stats`
  render-stats line (tier, pinned, budget, samples, over-budget frames,
  downgrades, upgrades, per-tier frame counts), emitted on SIGUSR1/exit like
  the other T-03/T-04 counters, and by the `query degrade` synthetic hook.

## Consequences

- Every budget-sensitive material consumes one ladder and one selector; a new
  effect degrades by routing its token spec through `DegradeTier`, not by
  growing a new pass or a new guard. T-05.1b/T-05.6 apply the same tier.
- The degrade policy is assertable headless: the tier classifiers are pure,
  the selector is driven by synthetic durations, and the forced tier
  round-trips over the synthetic harness. On headless nothing renders, so the
  controller reports zero samples until a backend actually renders.
- `Minimal` is deliberately still legible (blur off, small shadow, rounded
  window); it is a quality floor, not an effects kill-switch. A real
  texture-sampling blur (still deferred, ADR 0013) swaps in behind the same
  tier contract.