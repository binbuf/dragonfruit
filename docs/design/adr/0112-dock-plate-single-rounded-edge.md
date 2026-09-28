# 0112 — The Dock plate has one integer rounded edge and a corner-following rim

## Status

accepted

## Context

A 2026-09-27 live review of the Dock found the plate did not read as one
rounded rectangle:

- `Dock.qml` drew the bright rim as a 1 px-tall straight `Rectangle` inset by
  `plateRadius * 0.6` (≈16.8) from each end. Because the inset was smaller than
  the 28 px radius, the straight line ran ~11 px *into* each top corner arc —
  the "artifacting at the top edges before the curves begin".
- The QML plate drew the fractional `plateRect`, while the shell declared the
  compositor backdrop panel as `qFloor(x), qFloor(y), qCeil(w), qCeil(h)`. The
  two rounded rects were rasterized independently (QML anti-aliasing vs the
  compositor's piecewise spans) and disagreed by up to a pixel, so the frost
  edge and the fill edge showed a fringe that moved frame to frame.
- `setDockPanelRect` was called on every commit even when the integer rect was
  unchanged, so sub-pixel motion could ping-pong the declared panel.

Two independently rasterized rounded rects will always show a seam; the fix is
to make them the same geometry, or make one invisible. The radius is static and
shared (T-14.7u, ADR 0108) and must not become size-tracking here.

## Decision

- **One integer rounded rect owns the edge.** `Dock.qml` exposes `panelRect`:
  each edge of the live `plateRect` snapped with `Math.round`. The plate group
  draws that rect (fill, rim, border, shadow, clip), and `renderDock` declares
  the same rect to the compositor via `df_layer_surface.set_panel_rect`. The
  logical `plateRect` is unchanged for input, magnify math, and tests; only the
  *drawn* edge snaps.
- **The shell commits the declared panel only when the integer rect changes.**
  A new `m_dockPanelRect`/`m_dockPanelRectValid` pair in `ShellController`
  guards `setDockPanelRect`, so a smooth animation that crosses no pixel
  boundary cannot ping-pong the backdrop.
- **The rim follows the corner arcs.** `dockRim` is a `Shape` whose `PathSvg`
  traces the plate's interior edge (flat segment plus both interior corner
  arcs), stroked at `plate.rimHeight` and inset by half the stroke so every rim
  pixel lies inside the plate shape. It replaces the straight hairline. Only
  the interior edge is traced; the anchored edge stays clean.
- **No new tokens and no radius change.** The rim stroke width/opacity and the
  plate radius stay `controls.dock.radius` / `plate.rimHeight` / `rimOpacity`.

## Consequences

- The QML fill and the compositor frost are the same rounded rectangle on the
  integer grid, so there is no visible fringe while the plate grows. The QML
  fill remains the visible edge owner; the frost stays the material *behind* it
  (ADR 0102), so the one-backdrop-pass invariant and the degrade tiers are
  untouched.
- `shell/tests/tst_dock.qml` gains a rim pixel test (hide the rim, diff the
  frames: no changed pixel outside the plate rounded rect, at least one on the
  corner arc), a panel-rect integer/parity test, a sweep stability test, and a
  magnified padding-parity test. The layered-glass test reads the `ShapePath`
  stroke instead of a `Rectangle` colour.
- The compositor code is unchanged: its panel is now fed an integer rect, so
  `radius.round()` / `to_physical_precise_round` produce the same edge.
- A future size-tracking radius (T-16.1a) must keep the single-rect rule and
  re-snap `panelRect`; true refraction remains deferred (ADR 0091).

## References

- [04-shell.md](../04-shell.md) "Dock plate and materials".
- ADRs [0089](0089-dock-plate-geometry-and-live-panel-rect.md),
  [0102](0102-dock-material-role-and-qml-glass-layers.md),
  [0108](0108-dock-reference-metrics-indicator-inset.md),
  [0111](0111-dock-magnification-tracking-stability.md).