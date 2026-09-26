# 0088 — Dock plate geometry, magnification growth, and the live panel rect

## Status

accepted

## Context

The legacy Dock design (`docs/tasks/legacy/10-dock.md` §2/§14) specified a bar
flush against its anchored screen edge, a reserved zone that reports the bar
only, magnified icons rising above the bar into the window area, and a
backdrop panel derived by the compositor from the client's input region
intersected with the reserved strip. The 2026-09 Dock review found the result
visually wrong: the plate touches the screen edge, the artwork has only 6 px
of padding, the plate never grows while the icons magnify, and the frosted
backdrop cannot follow even a larger plate because
`window/backdrop.rs::panel_bounds` deliberately clips it to the baseline
reserved rect. Separately, `shell/dock/Dock.qml` reads the raw pointer every
hover event; the design's short `motion.dock-magnify` spring was never
implemented.

## Decision

- **The plate floats.** `controls.dock.edgeMargin` separates the plate from its
  anchored edge on all four positions, and the reserved zone is the resting
  plate plus that margin (`barThickness + edgeMargin`). Auto-hide still
  translates the plate fully off the edge and reserves nothing while hidden.
  The token layer is the only source for the spacing and margin values
  (T-14.7a).
- **The plate wraps magnification, the reserved zone does not.** The plate
  grows in both axes to wrap the magnified row, inside the pre-reserved
  `magnifyBand`; the reserved zone stays the resting thickness and no surface
  reconfigure or window re-layout happens during a sweep (the legacy
  "magnification reserves nothing extra" rule is preserved). Launch/attention
  bounce overshoot is excluded from the plate so a bounce does not pump it
  (T-14.7b).
- **The pointer is smoothed, not sampled.** Magnification runs on a smoothed
  pointer position using `motion.dock-magnify` (whose bezier already carries
  the slight overshoot); reduced motion tracks the raw pointer. The geometry is
  still recomputed every frame from the smoothed pointer, so it stays
  progress-based and interruptible.
- **The shell declares the live panel rect.** An additive
  `df_layer_surface.set_panel_rect(x, y, w, h)` request (interface version
  bump per `docs/ipc-versioning.md`; lockstep version unchanged) lets a chrome
  surface name the rect it is actually painting. The Dock sends the live plate
  rect on every commit; the compositor intersects it with the surface geometry
  and uses it as the backdrop panel. When unset, the existing
  `panel_bounds` derivation stays in force for every other chrome surface.
- **The input region follows the plate.** It is the plate plus the currently
  magnified/bouncing entry rects, never the whole surface from a hover.

## Consequences

- The legacy §2 wording ("the reserved zone reports B only") is refined to
  "the resting plate plus its edge margin; magnification and bounce reserve
  nothing". The reserved zone still never reports M/R/P.
- The frosted material sits exactly under the visible plate at all times,
  including while its edges grow and spring.
- Windows never jump while the pointer sweeps the Dock; Zoom geometry is
  unchanged during magnification.
- The menu bar and popovers keep their derived panel; only surfaces that opt in
  via `set_panel_rect` change behavior.
- Per-output sizing/scaling remains T-16.1a's concern; this decision is
  geometry-local to one surface.