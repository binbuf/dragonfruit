# 0165 — Menu-bar status marks are Phosphor; the battery composes an outline with a token fill

## Status

accepted

## Context

T-19.1c replaces the original-geometry `Canvas` marks in
`shell/menubar/StatusGlyph.qml` with Phosphor glyphs (vendored in T-19.1a,
rendered by `PhosphorIcon`). The menu bar is a plain-mark surface: monochrome,
no tile, no gradient. `StatusItem` still sizes the slot from the glyph's
`implicitWidth`/`implicitHeight`, so the migrated marks must keep the same
footprint.

Two states do not map one-to-one:

- The Wi-Fi adapter distinguishes more states than Phosphor has glyphs for the
  menu-bar silhouette (`wifi-off`/`wifi-disabled`, `wifi-connecting`,
  `wifi-error`, plus the generic connected-but-open `wifi`), and it carries no
  per-network strength to the mark.
- Battery has a **continuous** level (`level`, 0..1); Phosphor's battery set is
  quantized (`battery-empty`/`-low`/`-medium`/`-high`/`-full`).

`PhosphorIcon` renders each glyph as one `ShapePath` with a bound `fillColor`.
Qt Quick's ancestor `clip` is not applied to a child `Shape` under the headless
software scene graph the QML tests use, so a full battery glyph cannot be
clipped to the level by wrapping it in a clipped `Item`.

## Decision

- **Every state maps to a bare `PhosphorIcon`** (`fill` weight), one literal
  name per glyph so `scripts/check-phosphor-icons.py` sees it, selected by a
  `StatusGlyph.glyphName` switch: `wifi`/`wifi-secure` → `wifi-high`;
  `wifi-off`/`wifi-disabled` → `wifi-slash` (dimmed to 0.4 as before);
  `wifi-connecting` → `wifi-high` (dimmed to 0.5, static so it is
  reduced-motion safe); `wifi-error` → `wifi-x`; `bluetooth`;
  `volume` → `speaker-high`; `volume-muted` → `speaker-x`; `focus` → `moon`;
  `accessibility` → `person`; `control-center` → `sliders-horizontal`;
  `mission-control` → `squares-four`. The marks stay monochrome in `color`
  with no container, and `DragonfruitLogo.qml` is untouched.
- **Battery keeps a continuous level** by composing the Phosphor
  `battery-empty` outline (regular weight) with a plain `Rectangle` fill
  overlay inside the cell, sized from Phosphor's `battery-full` interior
  (viewBox x 40..192, y 88..168) times `level`. The fill is token geometry, not
  Phosphor path data, because of the software-backend clipping limitation
  above; the outline and every other mark still come from Phosphor.
  `battery-charging` uses Phosphor's `battery-charging` (outline + bolt) and
  draws no level fill, matching the original geometry's behavior.
- **The public API is unchanged** (`name`, `color`, `size`, `level`,
  `backgroundColor`), so `StatusItem`, `WifiMenu`, `VolumeMenu`, and
  `BatteryMenu` need no changes and the slot footprint does not shift.

## Consequences

- A new state is one row in `StatusGlyph.glyphName` plus its literal
  `PhosphorIcon`; `make check-phosphor` rejects an unvendored name.
- `glyphName` is a bound expression, so `check-phosphor-icons.py` cannot
  validate the mapping itself; `tst_menubar` renders every state and asserts a
  non-empty mark (and that a high/low battery level inks/clears the cell).
- `backgroundColor` is retained for API compatibility but the Phosphor `moon`
  is self-contained, so the Focus mark no longer needs a background cut-out.
- The battery fill is the one menu-bar mark that is not Phosphor path
  geometry; if Qt later clips a child `Shape` under software, the overlay can
  switch to a clipped `battery-full` glyph without changing the public shape.