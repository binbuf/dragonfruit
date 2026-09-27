# 0108 — Dock reference metrics: the running indicator lives inside the padding

## Status

accepted

## Context

T-14.7a/T-14.7j chose the resting Dock's spacing against an early reference;
by 2026-09 the mature local capture `docs/reference/macos/Dock.png` (measured
by pixel analysis, never shipped) showed the result was visibly tighter and
squarer than the reference. The old model also made the running indicator
band *additive*: `barThickness = iconSize + indicatorSpace + 2 * padding`, so
the artwork's cross-axis inset differed on the two sides (top `padding`, bottom
`padding + indicatorSpace`) and the plate had to reserve a whole extra band for
the dot.

## Decision

- **Retune the resting tokens** to the measured table: `padding` 15,
  `paddingAlong` 16, `gap` 14, and `radius` 28 (≈0.36 × the resting plate
  thickness). These keep the plate radius a single static token shared with the
  compositor's Dock backdrop (`component::dock::RADIUS`, ADR
  [0102](0102-dock-material-role-and-qml-glass-layers.md)) and the gallery,
  rather than a QML-computed ratio that the compositor cannot mirror.
- **The indicator sits inside the padding.** `barThickness = iconSize + 2 *
  padding`. Every entry's cross extent is still `iconSize + indicatorSpace`
  (dot + gap), but the layout anchors the *artwork* `padding` from the plate's
  interior edge, so the dot occupies the anchored-edge padding. At rest the
  artwork is centered in the plate (the same inset on both sides) and the dot
  sits `indicatorGap` (8) below it. `indicatorSpace` (12) is required to be ≤
  `padding`, which holds across `iconSizeMin`…`iconSizeMax`.
- **Magnification grows toward the interior.** The artwork is anchored on the
  anchored-edge side, so a magnified entry grows into the pre-reserved magnify
  band while the dot stays put — the same progress-based, interruptible
  geometry as ADR [0089](0089-dock-plate-geometry-and-live-panel-rect.md), with
  no change to the reserved zone or the live panel rect.
- **No material change.** The plate fill/rim/border/shadow opacity and the
  semantic `dock*` colors are unchanged; only geometry tokens moved.

## Consequences

- The dock reads like the reference at rest: plate ≈78 px tall, corners ≈28 px,
  cross-axis inset 15 px, dot 8 px below the artwork, gaps 14 px tile-to-tile.
- `barThickness`/`surfaceThickness`/`reservedThickness` shrink by the old
  additive indicator band (and grow with the larger padding), so
  `surfaceThickness` is 159 px at the default size; the reserved zone follows.
- The vertical (left/right) Dock mirrors the same rule: the artwork keeps
  `padding` from both plate edges and the dot sits in the anchored-edge
  padding, so the current test that expected `entry.x == edgeMargin + padding`
  now expects the artwork (not the entry box) at that inset.
- A plate radius that tracks the icon size would require the compositor to know
  the panel thickness; that remains deferred (per-output/chrome sizing is
  T-16.1a's concern).