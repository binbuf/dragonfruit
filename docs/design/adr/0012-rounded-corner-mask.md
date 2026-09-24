# 0012 — Rounded corners are a token-derived geometry mask

## Status

accepted

## Context

T-04.1b adds rounded-corner clipping for chrome, SSD titlebars, and floating
windows, on the same surfaces as the T-04.1a shadow pass. The compositor
renderer is a flat solid-color renderer with no vector rasterizer and no
shader (every shape so far — traffic lights, glyphs, shadow layers — is built
from solid rectangles), while the QML design system rounds with Qt's
`Rectangle.radius`. Without one shared shape, the compositor-drawn SSD and the
first-party QML `TitleBar`/`AppWindow` would drift (FR-2/FR-3), which is the
same failure mode [ADR 0011](0011-elevation-shadow-tokens.md) fixed for
shadows. The material track's later tasks (T-04.2 backdrop blur, T-04.4 degrade
radii) and the T-04.3 reusable transform all need this shape.

There are two possible owners for "what a rounded rect is": each renderer
re-deriving its own corner geometry, or one token-derived decomposition both
sides share.

## Decision

- Add `compositor/src/window/corner.rs` as the single rounded-rectangle
  geometry source. A `CornerMask` is a radius plus `RoundedCorners`
  (`All`/`Top`/`Bottom`); `rounded_rect_spans` decomposes it into
  **non-overlapping horizontal spans** (the same flat-renderer trick as the
  traffic-light circles), and `corner_squares` exposes the four radius squares
  the mask clips.
- The radii are token-only: `WINDOW_RADIUS` = `component.window.radius` and
  `TITLEBAR_RADIUS` = `component.titlebar.cornerRadius`, both generated from
  `tokens.json`. No radius is hardcoded in either consumer; changing a corner
  means editing the token and regenerating (`make check-tokens`).
- The compositor clips the surfaces **it draws**: the SSD titlebar fill is a
  `CornerMask::titlebar()` (top corners rounded, square bottom — byte-for-byte
  the QML `TitleBar`'s full-radius rectangle plus square bottom patch), and
  every shadow layer is rounded to `window.radius + blur * spread` (exactly
  `Shadow.qml`'s per-layer `radius`). `ShadowLayer` carries that radius.
- **A client surface's own pixels are not split per window here.** The window
  surface elements are Smithay-provided and keyed for presentation feedback;
  per-window cropping would duplicate element ids and risk the frame-callback
  and direct-scanout paths. The reusable scene-transform pass (T-04.3) owns
  the scale/translate/**clip** of live surfaces and consumes this mask; T-04.2
  blurs inside the same mask. First-party QML windows already round themselves.

## Consequences

- The rounded shape is pure geometry, so it is asserted headless against the
  tokens (span tiling, area is a subset of the bounding box, cutout squares)
  without a GPU or a pixel diff. `make qml-test`, `make e2e`, and the gallery
  goldens stay green.
- The SSD titlebar and the shadow now match the QML titlebar/shadow by
  construction; the FR-3 `TitleBar` vs `SsdTitlebarReference` test remains the
  guardrail, and the compositor mirrors that geometry.
- Third-party windows keep square **client** bottom corners until T-04.3 folds
  the clip into the reusable pass; their compositor-drawn chrome and shadow are
  already rounded. This is a known, intentional limitation of this slice.
- T-04.2/T-04.4a consume `CornerMask`/`RoundedCorners`; the degrade tier can
  shrink `CornerMask.radius` without a new shape model.
