# 0163 — Phosphor glyphs are resources; `PhosphorIcon` owns the styling

## Status

accepted

## Context

The inherited `design-system/components/Icon.qml` draws original geometry we
own for a small, fixed vocabulary. That was the right call for a handful of
chrome marks, but Track 19 needs a consistent foreground glyph language across
the menu bar, Settings categories, the Dock, and the Applications drawer, and
hand-drawing that vocabulary does not scale. The IP rule in
[14-risks.md](../14-risks.md) forbids Apple's assets and pixel-copied
proprietary artwork; it does not forbid third-party open assets.

Phosphor is MIT-licensed and designed for exactly this use, so we adopt it for
*foreground glyphs* while keeping every container, gradient, and tint in QML.
This is a deliberate departure from "all geometry is ours", so it is recorded
here.

Qt 6.11 has no `QtSvg` module in this toolchain, and the two obvious tint
mechanisms are unreliable: `Image` over an SVG does not tint at all, and
`MultiEffect` colorization is a GPU shader effect that silently no-ops on the
headless software scene graph the gallery and QML tests run on (the same limit
`Shadow.qml` already documents). The tint mechanism therefore had to render
under **both** scene graphs.

## Decision

- **Phosphor 2.0.8 (`v2.0.8`, commit `d42782b2…`) is vendored** under
  `assets/icons/phosphor/`, flattened with upstream file names: `<name>.svg`
  for the `regular` weight and `<name>-fill.svg` for the solid `fill` weight.
  `regular` is the primitive's default; the pinned set is the glyphs the
  shipped surfaces need, not all 1,248 upstream icons, so the existence gate is
  meaningful. More glyphs are a file drop plus a regeneration.
- **A glyph is a resource; all styling stays in QML.** The vendored SVGs are
  monochrome `currentColor` paths with no baked color or container. The raw SVG
  ships to QML unchanged at `qrc:/icons/phosphor/<file>`, and the tint, size,
  and any tile are owned by `PhosphorIcon`/`SettingsCategoryIcon`.
- **`PhosphorIcon` renders a `ShapePath` with a bound `fillColor`,** using path
  data generated at build time from the vendored SVGs into the
  `PhosphorGlyphs` singleton (`scripts/gen-phosphor-glyphs.py`). A `ShapePath`
  fill renders and tints under the software *and* RHI scene graphs. The
  component keeps the glyph's square aspect (Phosphor's `viewBox="0 0 256
  256"`), exposes `name`, `color` (default `Theme.color.textPrimary`), `size`,
  and an optional `weight`, and draws no container.
- **A referenced glyph that does not exist fails the build.** `make
  check-phosphor` runs `gen-phosphor-glyphs.py --check` (the registry must
  match the SVG directory) and `scripts/check-phosphor-icons.py` (every
  literal glyph name in first-party QML must name a vendored file). The runtime
  component additionally warns and renders blank for an unknown name, but a
  typo never reaches a shipped build.

## Consequences

- The track's migration units (T-19.1b/c/d) consume `PhosphorIcon`: T-19.1b
  wraps it in the Settings gradient tile, T-19.1c maps menu-bar states to bare
  glyphs, and T-19.1d builds the Files/Settings app artwork from it. They do not
  draw SVG or tint themselves, and they must add a glyph to
  `assets/icons/phosphor/` (both weights) and re-run the generator when they
  need a name that is not vendored.
- One tinted set serves both color schemes, so there is no dark/light glyph
  file pair; the file and the theme stay independent.
- `MultiEffect` colorization was the task's suggested mechanism and is not used;
  the software-renderer constraint is why. If a future Qt gains a
  software-capable SVG tint (or the tests move to RHI), `PhosphorIcon` is the
  single place to switch.
- The generated `PhosphorGlyphs.qml` is committed and checked like the token
  artifacts; the SVGs remain the single source of truth.