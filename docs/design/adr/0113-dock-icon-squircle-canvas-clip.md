# 0113 — Dock icon tiles are Canvas-clipped squircles (a software-observable mask)

## Status

accepted

## Context

ADR [0102](0102-dock-material-role-and-qml-glass-layers.md) gave the Dock
token squircles for its placeholder tile, but arbitrary themed artwork was
inset and fitted rather than masked: `DockGlyph.qml` drew the raster/SVG icon
with `PreserveAspectFit` and a `component.dock.icon.inset` margin, so a
full-bleed square theme icon showed a fitted square inside the tile with a
visible letterbox. T-14.7j deferred a real mask because "a software-renderer-
safe mask needs a GPU pass": `MultiEffect`, `OpacityMask`, and `ShaderEffect`
all silently no-op on the headless software scene graph (`QSGSoftwareContext`)
that `tst_dock.qml` runs on, so a GPU-only mask would be invisible to the
Dock's pixel tests (the same constraint `design-system/components/Shadow.qml`
documents for shadows).

## Decision

- **Mask in `DockGlyph.qml` with a `Canvas` clip.** A `Canvas` with
  `renderStrategy: Canvas.Immediate` traces the tile rounded rect at the token
  radius, `ctx.clip()`s to it, and `ctx.drawImage()`s the themed artwork
  (preserving aspect). The software scene graph executes `Canvas` through
  `QPainter`, so the clip is real in both backends — no tier split is needed at
  the pixel level.
- **The hidden `Image` loader is the load oracle.** `Canvas.drawImage(url)`
  loads asynchronously, so a hidden `Image` watches the same URL: its `Ready`
  status drives a `requestPaint()` (and a second `requestPaint()` arrives via
  the Canvas `imageLoaded` signal), and its `Error` status selects the
  placeholder tile (raster) or the `VectorImage` fallback (SVG, when the
  optional image-format plugin is absent).
- **One tile, token-driven.** `component.dock.icon.radiusRatio` (0.24) and
  `.inset` (now 0.0) define the tile; the placeholder tile, the masked Canvas,
  and the SVG fallback all read the same geometry. Setting `.inset` to 0 makes
  the artwork reach the tile edge, matching
  `docs/reference/macos/Dock.md` (icons ~48 px, no letterbox), and removes the
  artwork inset T-14.7j stacked on top of an icon's own padding.
- **The running indicator, window-count badge, tooltip, and focus ring are not
  part of the clip** — they stay siblings of the artwork in `DockEntry.qml`.

## Consequences

- The masked corners are ordinary pixels; `tst_dock.qml` asserts them for a
  full-bleed square (corners are background), a circle (reaches the tile edge,
  no double inset), and against the placeholder tile (same extent).
- A padded theme icon keeps its own transparent padding — the mask only shapes
  the tile, it never scales artwork to cover. That is the "not double-inset"
  behavior, not a promise to squircle the icon's inner artwork.
- `VectorImage` is now a fallback tier rather than the primary path; on systems
  without the SVG image-format plugin the icon still renders but is not masked.
- True liquid-glass refraction and animated squircle morphs remain deferred.