# 0186 — Dock reference retune: slot-pitch zoom profile, edge fit, and indicator states

## Status

accepted — amends [0162](0162-dock-hover-zoom-profile-and-label-tail.md) and
[0111](0111-dock-magnification-tracking-stability.md); the zoom profile and
tooltip timing are superseded by
[0218](0218-dock-motion-raised-cosine-and-tooltip.md)

## Context

A new screenshot-based study (`docs/dock.md`, 2026-11) measured a macOS Dock at
rest and with magnification engaged. It agrees with the shipped plate geometry
(end padding, icon pitch, icon-bottom inset, fixed panel height, bottom-anchored
growth) but differs from the older `Dock_Tile_Mouseover.png` fit in four
places:

- The zoom influence is expressed in **resting slot pitches**, with measured
  knots `1.00 / 0.90 / 0.45 / 0.00` at `0 / 1 / 2 / 3` pitches. The shipped
  quadratic reached ~0.60 at two pitches and ~0.11 at three, so the
  neighbourhood was too wide and too soft.
- A full Dock can be wide enough that the *magnified* union exceeds the output.
  The old plate clamped to the surface width, leaving the outermost tiles to be
  clipped by the layer surface.
- The reference distinguishes a running app that owns the focused window from a
  merely running app: the active app's dot is dark, an inactive one is muted.
  The projection did not expose focus at the app-entry level, so every dot was
  identical.
- The reference label appears after 150–300 ms with wider horizontal padding
  and a slightly larger text size than the shipped 600 ms / 8 px / 12 px
  values.

The study also measured the divider (`~0.78` of the plate cross-axis, a
visible dark core) and the plate radius (`~0.34` of plate height). Our radius
and divider were at the edge of the measured range and the divider's
`neutral400` at 0.5 opacity nearly disappeared into the light plate.

## Decision

- **The zoom profile is normalized by resting slot pitch.** `Dock.qml` divides
  the pointer-to-baseline-centre distance by `iconSize + gap` and evaluates a
  monotone cubic Hermite through the measured knots. The curve is flat at the
  peak and at the zero crossing, so it has no cosine spike and no ringing.
  `component.dock.magnifyFalloff` remains the zero point and is now 3.0
  pitches; `magnifyWeightNear` / `magnifyWeightMiddle` hold the interior knots.
  The distance is still measured from **stable baseline centres**, never the
  animated layout, so hit testing and the anchor rule cannot chase themselves.
- **The zoomed union is fitted to the output.** `magnifyFit` bisects a uniform
  peak fraction until `magnifiedPlateLengthFor(peak)` fits the axis, then
  `effectiveMagnifyPeakFactor` feeds both the row layout and the fixed plate
  length. The fit is a function of resting geometry, icon size, and
  magnification only; it does not re-evaluate per pointer sample. A Dock whose
  resting row already exceeds the axis stays at rest (the overflow planner owns
  that error state).
- **Discrete Dock motion does not overshoot.** `motion.dockMagnify` is now an
  ease-out cubic (`0.22, 1.0, 0.36, 1.0`) instead of the magnet spring. The
  pointer tracker was already non-overshooting; this removes the remaining
  elastic pop from icon-size and drag-gap changes. The reveal keeps its own
  token.
- **The row keeps the continuous warp, not a fresh centering each frame.** The
  study's pseudocode centers the content strip; its measured hover positions
  instead keep the pointed tile near the pointer and spread the growth
  asymmetrically (the left end moves farther than the right). The existing
  continuous resting-centre → magnified-centre warp reproduces that, keeps the
  anchored tile under the pointer, and is already pinned by the boundary-sweep
  tests. The plate itself stays centered on the output and only changes at the
  engagement level.
- **The running dot is stateful.** `buildDockProjection` sets `active` when any
  window in the app group is focused, and `buildDockEntries` preserves it
  through the pinned merge. `DockEntry` draws `indicator.opacity` for the active
  app and the new `indicator.inactiveOpacity` for a merely running app. A
  direct-model entry without the flag is treated as active, preserving the
  previous fixture behavior.
- **Label timing and geometry follow the reference.** `component.tooltip.dwell`
  is 250 ms, `paddingH` is the 12 px `spacing.md` step, and `fontSize` is the
  13 px `font.sizeMd` step. The pill radius is still half the capsule height,
  so it lands in the measured ~18–20 px range at the reference scale.
- **The app-provided badge is drawn again.** The entry model already carried a
  `badge`, but only the window-count/stack badges rendered, so an app badge was
  silently dropped. `DockEntry` now draws a `component.dock.appBadge` red pill
  that floats partly outside the tile's top-right corner, scales with the
  magnified tile, and suppresses the window-count mark (the existing
  status > app badge > window count precedence). The shell does not currently
  source app badges from notifications (T-11.1b keeps those in the banner and
  notification center); this closes the presentation gap for callers and for
  the folder-stack path that already uses `badge`.
- **Small measured corrections.** Plate radius 27 (≈0.35 of the 78 px resting
  bar, matching the reference's ~0.34), divider height ratio 0.78, divider
  opacity 0.7, and a darker light-scheme `dockDivider` (neutral500). The
  running dot, end padding, icon pitch, icon-bottom inset, and fixed panel
  height were already within the measured range and are unchanged.
- **The material baseline is unchanged.** `docs/dock.md` is explicit that the
  reference's `#E5E5E5` is a composited output against white, not an opacity
  measurement. The shipped layered `dockFill` + compositor frost + rim/gloss
  stack already reads as the required flat, milky glass on our wallpapers; the
  study's warning against loud gradients/tints is satisfied. The non-magnified
  hover wash is retained deliberately (ADR 0162) as the zoom-off affordance;
  the magnified anchor tile still drops it.

## Consequences

- The default magnified neighbourhood is tighter: one pitch ≈ 90 % of the peak,
  two ≈ 45 %, three at rest. The bubble still grows and shrinks with
  `motion.dockHover` around a held pointer/anchor.
- A Dock near the screen edge now reduces its peak instead of clipping tiles;
  the resting overflow planner still shrinks the base icon size when the
  *resting* row does not fit.
- `tst_dock` pins the knot values, the edge fit, the active/inactive dot, and
  the measured divider/radius tokens. `tst_dockcore` pins the projection's
  `active` flag and its survival through the pinned merge.
- The compositor's `design_tokens.rs` regenerates (radius, divider color,
  motion curve); its material tests assert the new radius.
- The exact reference colors remain validation fixtures, not values copied into
  the product: the plate still uses Dragonfruit's own neutral/brand palette and
  the user's wallpaper.
