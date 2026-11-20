# 0261 — Applications drawer keeps a fixed mostly-opaque glass card

- Status: accepted
- Date: 2026-11-14
- Follow-up to: [ADR 0260](0260-apps-drawer-translucent-glass.md)

## Context

ADR [0260](0260-apps-drawer-translucent-glass.md) lowered the `appsDrawer`
preset to 0.72 / 0.82 alpha and kept the menu-family `lightRelative` slider
response. On this build the Appearance transparency setting was at **0.1149**
(near the opaque end of the measured menu curve), so the relative fit resolved
the drawer to **0.9787 alpha** — the same flat, near-opaque slab the change was
meant to remove. The drawer was also a transient material role, so a global
`Minimal` degrade turned the sampled pass off and painted the 0.98
accessibility plate regardless of the preset.

An initial follow-up lowered the base to 0.64 and gave the drawer a glass
floor. That overcorrected: at the live slider position the card resolved to
about 0.81 alpha, and the wallpaper read through the large surface as a washed
out, too-transparent pane. The reference in
the local surface review §4 is a **large light glass
panel**, not a clear pane and not a flat plate: it sits between the two.

## Decision

- **The drawer no longer follows the menu slider's opacity curve.** Its
  `glass.presets.appsDrawer.slider` is `none`, so the card keeps the fixed
  calibrated alpha instead of scaling with `lightRelative`. The Appearance
  slider still drives the orthogonal `materialize` lens/frost axis, so the
  glass character still responds to the setting without turning the card into a
  slab.
- **The card is fixed at 0.88 light / 0.90 dark.** That is deliberately closer
  to the old readable plate than the 0.64 overcorrection, while the 16 pt light
  / 18 pt dark frost, 0.28 / 0.26 rim and 9 pt / 18 pt / 56 pt corner lens
  carry the glass. The wallpaper and windows remain faintly visible through the
  card; they are not a clear transmission.
- **The drawer keeps the sampled glass floor.** `MaterialRole::Drawer` is still
  transient for the popup-family semantics, but it now shares the
  `keeps_glass_floor()` classification with `Chrome` and `Dock`: a global
  `Minimal` degrade resolves it to `Reduced` with the sampled pass on. Small
  popups (`Popup`) still follow the ladder to `Minimal`.
- **The fallback plate is unchanged.** Reduce Transparency, no-sampled-blur,
  software and multi-GPU still select the shared opaque accessibility plate.
  The floor only prevents a *tier selection* from removing the glass.
- **One policy in all three resolvers.** `Glass.qml`, the Python preset module
  and `compositor/src/material_preset.rs` use the same fixed calibration and
  the same `keeps_glass_floor()` set; `EffectiveMaterial::resolve_for_role()`
  and the shell's `MaterialPolicy::keepsGlassFloor()` apply the render floor.

## Consequences

- At the live 0.1149 slider position the drawer resolves to 0.88 / 0.90, not
  0.98. It reads as a mostly-opaque glass card with a visible frost, rim and
  corner lens rather than a flat plate or a clear pane.
- A global `Minimal` degrade no longer swaps the drawer for the 0.98 plate; it
  renders at `Reduced` with the same card optics and the half-radius frost.
- The Appearance slider still affects the lens/frost balance through
  `materialize`, but it no longer controls the drawer's opacity. That is an
  intentional departure from the menu curve because the drawer is a large,
  actively read launcher and the measured menu response is not a drawer fit.
- The design-system, Python preset and compositor tests pin the fixed alpha,
  the no-slider behavior, the stronger optics and the glass floor.
