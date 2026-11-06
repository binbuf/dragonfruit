# 0214 — Glass role presets resolve through one generated vocabulary

- Status: Accepted
- Date: 2026-11-05
- Task: T195

## Context

T194 pinned the reference evidence and the v0.1.1 baseline. The next step needs
one optical vocabulary for the Dock, the menu-bar veil, attached/detached/status
menus, tooltips, Control Center regions, notification regions, and first-party
window chrome. It must stay separate from the existing `MaterialRole` wire
values and the `DegradeTier` ladder, because those describe transport and
budget pressure rather than the look of a particular surface.

The evidence is uneven. The Dock and the light attached-menu slider are
measured; several dark surfaces are adaptations or named baselines. The
material must be able to say which is which, and an unsupported sampled path
must never be mistaken for a deliberately veil-only surface.

## Decision

1. **The vocabulary lives in `design-system/tokens/tokens.json`.** A new
   top-level `glass` group holds 13 named presets, per-scheme calibration,
   provenance and confidence, the three measured light slider anchors, the
   accessibility overrides, and the rollout default. `scripts/gen-tokens.py`
   validates it and generates `Theme.glass` and `design_tokens::glass`. The
   generator's `--check` gate fails on a non-finite, out-of-range, missing, or
   moved-anchor value.
2. **Neutral and perceptual fits are separate records.** Each preset/scheme
   carries `neutralAlpha`/`neutralTint` plus a perceptual fit and a
   `perceptualWeight`. Resolution mixes them once; it never paints both fits as
   full-strength overlays. Every value records its sRGB colour space.
3. **The slider is a measured exception, not a global formula.** Only the light
   `menuAttached` preset follows the slider, with piecewise interpolation
   through the measured 0.0 / 0.5255 / 1.0 anchors (alpha 0.732 / 0.552 /
   0.415, tint 0.953 / 0.993 / 0.997). Dark and every unmeasured preset keep
   their named baseline.
4. **Resolution precedence is explicit.** Reduce Transparency wins over the
   slider and any sampled path, using a scheme-appropriate opaque plate.
   Increase Contrast is applied after the fill decision, so it still strengthens
   the edge/foreground on that opaque plate. The rollout look selects legacy
   versus replacement values. Backend capability and tier select the rendering
   mode; Reduce Motion is orthogonal.
5. **Rendering mode is a first-class result.** A preset resolves to
   `sampled-glass`, `no-blur-veil`, `solid`, or `software-fallback`. The
   menu-bar veil is `no-blur-veil` even when sampling is unavailable, so it is
   never reported as a missing sampled pass. Unsupported sampled presets become
   `solid` (or `software-fallback`) with an opaque scheme plate and a named
   reason. Persistent roles keep their sampled floor at `Minimal`.
6. **One internal rollout switch.** `DRAGONFRUIT_GLASS_ROLLOUT` selects
   `legacy` (the default until T208) or `replacement`. The shell's `Glass`
   singleton exposes it to QML and the compositor's `Rollout::from_env` reads
   the same value, so nested and native fixtures select either look
   reproducibly. An unset or unknown value is always the existing appearance.
7. **Two runtime resolvers, one data source.** `design-system/Glass.qml` and
   `compositor/src/material_preset.rs` implement the same policy over the
   generated values. `tests/glass/test_material_presets.py`, the QML
   `test_glass_*` cases, and `compositor/tests/material_presets.rs` pin the
   shared outcomes.

## Consequences

- Consumers migrate later (T196 settings, T197 draw geometry, T198–T206
  surfaces); this task only defines the vocabulary, policy, and switch.
- The reference evidence remains honest: measured values say `measured`, the
  dark attached menu says `adapted`, and unmeasured dark/lower-panel values say
  `baseline` or `observed-luma`.
- T208 flips the default only after the nested/native acceptance rows pass; the
  switch stays available for rollback.

## References

- `docs/tasks/glass/plan/REFERENCE.md` — evidence resolution rules.
- `design-system/tokens/tokens.json`, `scripts/gen-tokens.py`,
  `scripts/material_presets.py`, `design-system/Glass.qml`,
  `compositor/src/material_preset.rs`.
- ADR [0203](0203-role-keyed-material-floor.md), ADR
  [0204](0204-shell-role-material-application-policy.md), ADR
  [0205](0205-popup-family-material-fallback.md).
