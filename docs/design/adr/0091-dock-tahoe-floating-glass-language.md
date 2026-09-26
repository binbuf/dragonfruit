# 0091 — The Dock adopts the macOS Tahoe floating-glass language (original geometry)

## Status

accepted

## Context

The 2026-09 Dock review fixed the plate as a **floating** surface
([ADR 0089](0089-dock-plate-geometry-and-live-panel-rect.md)) and the product
direction is explicit: the Dock should match the design language, aesthetics,
and interaction model of the macOS Tahoe Dock. Two constraints bound this:

- **Original assets only** ([14-risks.md](../14-risks.md)): reproduce the
  interaction model and the visual language, never Apple's bitmap output. Local
  reference captures live under `docs/reference/macos/` (gitignored) and never
  ship.
- **The material renderer is a flat approximation.** ADR
  [0013](0013-backdrop-blur-pass.md) and the degrade tiers
  (ADR [0015](0015-material-degrade-tiers.md)) build the frosted backdrop from
  layered rounded rects; there is no texture sampler, so true refraction is out
  of reach today.

## Decision

- **Tahoe is the interaction/visual reference.** The Dock's resting geometry,
  spacing, glass treatment, icon treatment, hover affordances, running
  indicator, divider, Trash, and motion are derived from local Tahoe
  reference captures (measured, then expressed as tokens), never copied as
  assets.
- **The plate is a floating glass panel.** A continuous rounded rect with a
  translucent fill, a bright inner rim/highlight, a hairline border, and a
  soft shadow, using the dock and material tokens. The plate's geometry is
  ADR 0089's: floating with the gap matched to the reference, growing with
  magnification, live backdrop panel declared to the compositor.
- **Icons are squircles in a consistent inset.** Themed icons are masked/fit
  to a rounded-square tile with the reference inset; the placeholder tile, the
  hover highlight, the pressed scale, the keyboard focus ring, the running
  dot, and the divider hairline all follow the same spec. The Trash is original
  geometry at the token size (T-14.7d).
- **Hover reveals the name.** A hovered entry shows its name (and state) in a
  glass capsule above the icon, supplied by the design-system `Tooltip`
  (T-14.7i).
- **Motion follows the reference.** Magnification springs the plate and icons
  together on `motion.dock-magnify`; launch/attention bounces remain;
  reduced-motion variants remove translation/scale and keep the state legible.
- **Degrade honestly.** At the `Reduced`/`Minimal` material tiers the glass
  collapses to a translucent fill (and then a flat fill) with no refraction
  claim; the Dock must read as intentional at every tier.

## Consequences

- New dock material tokens (fill, rim, border, inner highlight, shadow) and
  squircle/inset values are added to `tokens.json` and generated into
  `Theme.qml`; `shell/dock/*.qml` stays token-only.
- The Dock is not a design-system gallery component, so visual verification is
  pixel tests plus committed nested/DRM captures, not a golden.
- A real liquid-glass refraction pass remains a future material-track item;
  this decision ships the best token-driven approximation and says so.
- The Linux-specific additions (Add Application picker, drop identity) stay;
  they extend the language rather than replace it.