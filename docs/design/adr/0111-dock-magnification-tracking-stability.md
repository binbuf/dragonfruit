# 0111 — Dock magnification tracking: a non-overshooting pointer and a peak-hold plate edge

## Status

accepted

## Context

The Dock magnified the row by reading a smoothed pointer
(`Dock.smoothPointerAlong`). That value was produced by a `NumberAnimation`
retargeted on **every** pointer-move sample with the `motion.dockMagnify`
bezier, whose control points `[0.34, 1.56, 0.64, 1.0]` overshoot. Retargeting
an overshooting curve per input sample makes the smoothed value ring around the
true position: the icon row visibly vibrated during a slow sweep, and the plate
— whose top edge was the raw per-frame `min()` over the magnified entries —
grew and shrank by tiny amounts back and forth because the ringing modulated
the peak icon. Two further couplings worsened it:

- `anchorIndex` picked the tile nearest the **smoothed** pointer, so the
  anchored tile could flip at a boundary as the filter rang across it.
- The plate top read the raw `min()` every frame, so it followed the ringing
  and the per-tile magnification ripple with no damping.

The overshoot is a deliberate *entrance* effect and is correct for the discrete
`iconSize` spring and the reveal; it is wrong for a continuously-sampled
tracker.

## Decision

- **The per-sample tracker cannot overshoot.** A new `motion.dockMagnifyTrack`
  token (short `fast` duration, curve `[0.2, 0.0, 0.0, 1.0]`) drives the
  pointer low-pass. `motion.dockMagnify` keeps its overshoot only for discrete
  changes (`iconSize`, reveal). Reduced motion tracks the raw pointer.
- **The anchor is chosen from the raw pointer.** `anchorIndex` reads
  `pointerAlong`, not the filter, so the anchored tile cannot oscillate with the
  filter; the existing rule (the tile under the pointer stays put) is unchanged.
- **The plate's magnify edge is a peak-hold.** A new `plateTrackRaw` derives the
  entry union edge (bounce added back) and `smoothPlateTrack` follows a *deeper*
  peak with the tracking low-pass, holds while the pointer stays on the Dock,
  ignores a sub-pixel deadband, and snaps on discrete magnify entry/exit and
  under reduced motion. The edge therefore moves with the peak and never
  against it, including across the per-tile ripple, while the plate still grows
  into the magnify band with magnification.

## Consequences

- One smoothing source of truth on the pointer; the plate adds one damped
  edge signal. No second animation sits on top of the tracker.
- `tst_dock` pins the fix: a one-tile jump asserts the tracker never passes its
  target (pre-fix overshoot 11.0 px; fixed 0.0 px), a raw jump asserts the
  anchor already points at the raw tile, and a 30-step sweep asserts the anchor
  tracks the raw pointer and the plate top does not reverse by more than a
  device pixel (pre-fix: 19 reversals, max 1.11 px; fixed: 0).
- The plate edge is held, not released, while the pointer remains on the Dock;
  the strict per-frame entry-union wrap is intentionally given up between tiles
  in exchange for stability. Leaving the Dock snaps it back to rest.
- `component.dock.magnifyFalloff` (3.0) and the peak tokens are unchanged, so
  T-14.7z/T-14.7w see identical resting and peak geometry.