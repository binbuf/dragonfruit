# 0162 — Dock hover: the reference zoom bubble, engagement ease, fixed bar, and label tail

## Status

accepted

## Context

`docs/reference/macos/Dock_Tile_Mouseover.png` (a local-only macOS capture,
never shipped) shows the hover state with magnification configured: the
hovered tile is zoomed with its neighbourhood, the label above it carries a
pointer tail, the plate reads as glass with a soft interior gloss and a bright
lip along its screen-edge side, and there is **no** rounded-rect highlight
wash behind the hovered tile. Pixel analysis (2026-09-28, recorded in the
task/PROGRESS note) measured the zoom profile as a wide quadratic bubble —
~90 % of the peak effect one tile away, ~55 % two tiles, zero by ~3.5-4 icon
widths — not the cosine falloff (`magnifyFalloff` 3.0) the Dock shipped, and
the transition into and out of the hovered state was a snap: the pointer
tracker is smoothed, but the *amplitude* changed instantly when the pointer
entered or left the surface.

Three gaps were visible against the capture:

- The falloff reached zero at 3 icon widths with a sharp near-peak slope, so a
  macOS-like neighbourhood (about seven tiles rising together) was impossible.
- `smoothPointerAlong` snapped on entry/leave, so the zoom popped in and out
  instead of growing and shrinking.
- The reference hover state carries no wash, and the label is a tailed pill;
  the Dock drew a flat rounded wash and a borderless rectangle with no tail.

## Decision

- **The profile is a quadratic bubble with `magnifyFalloff` 4.1.**
  `effect = 1 - (d / (magnifyFalloff * iconSize))²`, clamped at 0. The falloff
  token stays the profile's radius in icon widths; only its value and the
  profile curve changed. One tile away keeps ~0.90, two ~0.60, three ~0.11 —
  within a few points of the capture. The peak mapping is retuned to the
  capture's measured 1.245× (`magnifyPeak` 1.6 → 1.25 at the default
  `magnification` 0.5, `magnifyPeakMax` 2.2 → 1.5), so the default hover now
  matches the reference and the magnified artwork stays inside the fixed bar
  instead of spilling far above it.
- **`magnifyEngagement` scales the bubble.** The pointer drives the profile's
  *shape* (`magnifyPointer`, the smoothed pointer); a 0..1 engagement driven
  by `magnifying` and eased with the new `motion.dockHover` token drives its
  *amplitude*, so entering and leaving the Dock grows and shrinks the zoom
  around the pointer. Leaving holds the last pointer and anchor
  (`magnifyPointer`/`magnifyPointerIndex`), so the bubble collapses around the
  tile the pointer left instead of sliding to the Dock edge. `dockHover` is a
  plain CSS-style ease, not `motion.dockMagnify`'s magnet spring: the spring
  reaches most of its target in one frame, which reads as a pop when the whole
  row translates with the bubble. Reduced motion collapses the token duration
  to zero, so the state change stays legible with no translation.
- **Magnified positions are a continuous warp, not an anchor pin.** The row is
  accumulated from the resting leading edge using the magnified widths and
  scaled gaps, then translated so that the pointer maps to itself through the
  piecewise-linear resting-centre → magnified-centre map. Every position is
  then a continuous function of the pointer. The previous discrete pin held
  the anchored tile on its resting centre while the magnified pitch pushed its
  neighbours out, so the moment the anchor flipped the row translated by a
  whole magnified pitch — ~35 px per boundary with the reference profile, the
  hover twitch a stop-motion sweep exposes. The anchored tile still tracks the
  raw pointer, is exactly on its resting centre when the pointer is on that
  centre, and stays under the pointer proportionally in between.
- **The bar has a fixed cross axis and one horizontal zoom level.** The
  reference keeps the dock background's height constant, and its horizontal
  size has exactly two levels — base and one zoomed level — that do not vary
  while the pointer moves along the Dock. `plateRect` now does the same: the
  cross axis is always the resting `barThickness`, and the along axis is the
  base length blended to a single `magnifiedPlateLength` by the hover
  engagement. `magnifiedPlateLength` is the swept union of the fully magnified
  row at every resting centre, centred on the surface with the end padding, so
  the one fixed bar always contains the icons even though the row translates
  under the pointer. The bar therefore resizes once on entry and once on
  release and is otherwise completely static; the T-14.7y plate-edge peak-hold
  (and its animation) is deleted because there is no edge signal left to damp.
  The magnified artwork grows into the pre-reserved `magnifyBand` above the
  fixed bar.
- **The hovered (anchored) tile drops the flat wash and lifts.** The
  reference's hover treatment is the zoom itself plus a soft tile shadow;
  `DockEntry.zoomed` suppresses `hoverHighlight` and shows a small `Shadow`
  at `component.dock.hover.shadowBlur`/`.shadowOpacity`. A zoom-off Dock keeps
  the existing wash as the non-magnified hover affordance.
- **The plate gains an interior gloss band and an anchored-edge lip.** Both
  are token-driven (`plate.glossHeight`/`.glossOpacity`,
  `plate.edgeHeight`/`.edgeOpacity`), orientation-aware, and follow the
  existing single rounded edge: the gloss reuses `rimOutlinePath` with its own
  stroke width so it follows the corner arcs, and the lip is a straight
  hairline inset past the corner radius on the flat middle of the anchored
  edge.
- **The Tooltip gains an opt-in pointer tail and a pill capsule.** New
  `component.tooltip.tailWidth`/`.tailHeight`/`.rimColor`/`.rimOpacity`
  tokens; `tailVisible` is default-off, and the Dock opts in. The tail is a
  filled triangle aimed at the anchor centre (clamped inside the pill's end
  arcs), clears the anchor by `offset + tailHeight`, ride the same overlay
  rect (which is inflated by the tail), and overlaps the capsule border by one
  pixel so the border does not cross its base.

## Consequences

- The Dock's hover state now reads like the capture: a wide, flat-topped zoom
  bubble that grows and shrinks with a short ease, a tailed pill label, glass
  layers, a fixed-height background whose width has one zoomed level, and no
  flat wash behind the zoomed tile.
- The magnified layout is now a continuous function of the pointer: a
  boundary-crossing test sweeps 150 positions in 1.53 px steps and asserts no
  tile or plate edge moves more than a few pixels per step; the old pin moved
  the row by the full magnified pitch in one step. A companion test sweeps the
  pointer and asserts the plate rect is byte-identical at every position while
  engaged, then returns to base on release.
- Existing magnification geometry tests were updated to the fixed-bar contract
  (height constant; magnified artwork may pass the interior edge but stays in
  the reserved band). `tst_dock` adds cases for the profile shape, the
  engage/release, the single zoomed level, the zoomed tile's wash/shadow, the
  plate layers, and the tooltip tail. The gallery Tooltip page opts into the
  tail and its goldens were regenerated.
- `magnifyFalloff` and the peak mapping are consumed only by `Dock.qml`; the
  compositor's `design_tokens.rs` regenerates but reads nothing new.
- The engagement animation is time-based on top of a pointer-driven profile.
  It is interruptible (the Behavior retargets from its current value) and the
  geometry stays progress-based; the pointer tracker itself is unchanged, so
  ADR [0111](0111-dock-magnification-tracking-stability.md)'s stability
  guarantees hold.
- ADR [0089](0089-dock-plate-geometry-and-live-panel-rect.md)'s live panel rect
  becomes *more* stable: because the cross axis is fixed and the along axis has
  one zoomed level, the declared backdrop panel changes only on hover
  engage/release, not per frame during a sweep.