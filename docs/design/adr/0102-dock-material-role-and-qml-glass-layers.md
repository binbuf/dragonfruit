# 0102 — The Dock owns a material role and draws its glass in token layers

## Status

accepted

## Context

ADR [0091](0091-dock-tahoe-floating-glass-language.md) fixed the Tahoe
floating-glass language for the Dock, and ADR
[0013](0013-backdrop-blur-pass.md)/[0015](0015-material-degrade-tiers.md) built
the flat layered frosted backdrop. Until T-14.7j, however, the compositor chose
a material from `df_shell.layer` alone, so the menu bar and Dock both resolved
`MaterialRole::Chrome` and shared `material.chrome*`: the Dock's frost could not
be tuned without moving the menu bar, and the QML plate was a single flat
`Rectangle` (chrome at `chromeOpacity`) that read as a slab, not glass.

## Decision

- **The Dock is its own material role.** `MaterialRole::Dock` is selected by the
  `df_layer_surface` namespace (`"dock"`) for persistent surfaces, not by
  layer; the menu bar stays `Chrome` and overlays stay `Popup`. The
  compositor's `ChromeSurface` carries the namespace so the role selection has
  one input.
- **The Dock's frost is token-isolated.** New semantic tokens
  `material.dockBlur`/`material.dockOpacity` and `color.dockFill` (per scheme)
  feed the role, and `component.dock.radius` now drives the backdrop corner.
  Tuning the Dock leaves the menu bar untouched, and the one-backdrop-pass /
  degrade-tier invariants (ADR 0013/0015) are unchanged.
- **The QML plate draws the language in layers.** `Dock.qml` layers a
  translucent fill, a bright inner top-edge rim, a hairline border, and a soft
  shadow above the compositor's live backdrop panel. All geometry and tone come
  from `controls.dock.plate` and the semantic `dock*` colors, never literals.
- **Icons are token squircles with an inset.** `controls.dock.icon.radiusRatio`
  and `controls.dock.icon.inset` define the app tile, and the hover highlight,
  lift shadow, focus ring, status badge, running dot, and divider
  (`controls.dock.hover`/`indicator`/`divider`) share the spec.

## Consequences

- The material sweep for the menu bar and other chrome is now independent; a
  later task tunes `chrome*` without a Dock regression.
- The Dock still uses the layered flat approximation, not true refraction;
  that remains deferred (ADR 0091, `14-risks.md`).
- Because the shell receives no degrade-tier signal, the QML plate fill is
  designed to read as a clean capsule on its own; the tier only changes the
  compositor frost (Reduced scales it, Minimal drops it). A future tier signal
  to the shell would let the QML rim drop its \"glass claim\" at `Minimal`.
- Arbitrary square theme artwork used to be inset and fitted rather than
  squircle masked. T-14.7w replaced that with a real tile clip: `DockGlyph.qml`
  draws the themed artwork into a `Canvas` clipped to the token squircle, and
  `component.dock.icon.inset` is now 0 so the artwork reaches the tile edge.
  A `Canvas` clip is executed by the headless software scene graph, so the
  pixel tests observe the masked corners; true liquid-glass refraction (a
  texture sampler) remains deferred (ADR 0091, `14-risks.md`). See
  [ADR 0113](0113-dock-icon-squircle-canvas-clip.md) for the tier behavior.