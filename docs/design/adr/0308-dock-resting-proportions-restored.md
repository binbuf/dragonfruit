# 0308 — Restore and pin the Dock plate's resting proportions

- Status: Accepted
- Date: 2026-11-20
- Area: `design-system/tokens/tokens.json`, `shell/src/dockmodel.*`,
  `compositor/src/window/*`, `shell/tests/tst_dock.qml`
- Restores: the token values in ADR [0108](0108-dock-reference-metrics-indicator-inset.md)
  as retuned by ADR [0186](0186-dock-reference-retune-profile-fit-and-states.md)

## Context

The resting Dock plate is a measured surface: `component.dock.padding` 15,
`paddingAlong` 16, and `radius` 27 give the 78 pt plate height
(`iconSize` 48 + 2 × 15) and the reference's ~0.35 radius-to-height ratio. Those
values were recorded in ADR [0108](0108-dock-reference-metrics-indicator-inset.md),
retuned to radius 27 in ADR [0186](0186-dock-reference-retune-profile-fit-and-states.md),
and are the values described by [04-shell.md](../04-shell.md). The T198 capture
`docs/captures/t198-dock-optics-light-1x.json` records the same geometry
(`panel_logical` height 78, `declared_radius_pt` 27).

The 2026-11-12 `dock plate fixes` change (`13e13a3`) also retuned the three
tokens to `padding` 16, `paddingAlong` 8.5, and `radius` 26.9 without a new
reference capture or a design-document update. The result was a 2 pt taller
plate and ends 7.5 pt tighter on each side than the measured surface — the
same class of "the plate reads tighter than the reference" regression that
T-14.7u was created to fix. The change also made `paddingAlong` fractional in
the shell's pure layout type, even though the restored value is integral.

## Decision

- **Restore the measured tokens.** `component.dock.padding` is 15,
  `paddingAlong` is 16, and `radius` is 27. `Theme.qml` and
  `design_tokens.rs` are regenerated from `tokens.json`; they are not edited
  by hand.
- **Keep the plate work that is not part of the regression.** The continuous
  squircle, the physical bevel, the measured two-path Gaussian, the adaptive
  tone, and the short-end lens stay as recorded in the amendments to ADR
  [0217](0217-dock-optics-fit.md) and ADR
  [0255](0255-dock-uniform-perimeter-and-corner-gate.md). This decision changes
  resting geometry only.
- **Pin all three tokens.** `tst_dock` now asserts `padding` 15,
  `paddingAlong` 16, and `radius` 27, in addition to the existing `gap` and
  indicator checks. The previous test omitted `paddingAlong`, which is how the
  8.5 value landed without a failure.
- **Keep the layout type integral.** `DockLayoutSpacing::paddingAlong` returns
  to `int`, and the overflow planner reads it with `qRound(...toReal())`, as it
  did before the fractional retune.

## Consequences

- The plate is again 78 pt tall with 16 pt end insets and a 27 pt radius, so
  the QML fill/rim, the compositor's declared panel, and the T198 optics
  capture agree.
- A future restyle that changes any of the three values fails `tst_dock`
  unless the test and a new reference capture are updated together.
- ADR 0108's radius 28 and ADR 0186's 27 remain the decision history; this ADR
  records the final values and the regression that briefly replaced them.

## References

- `design-system/tokens/tokens.json` → `component.dock`
- `shell/dock/Dock.qml`, `shell/src/dockmodel.{h,cpp}`,
  `shell/src/shellcontroller.cpp`, `shell/tests/tst_dock.qml`
- `compositor/src/design_tokens.rs`, `compositor/src/window/backdrop.rs`,
  `compositor/src/window/dock_optics.rs`
- `docs/captures/t198-dock-optics-light-1x.json`
- ADR [0108](0108-dock-reference-metrics-indicator-inset.md), ADR
  [0186](0186-dock-reference-retune-profile-fit-and-states.md), ADR
  [0217](0217-dock-optics-fit.md), ADR
  [0255](0255-dock-uniform-perimeter-and-corner-gate.md)
