# Tabler — the Dragonfruit line-glyph set

Tabler is the desktop's **line-glyph** language. A vendored glyph here is a
*resource*: it is a monochrome path with no color or container baked in. The
tint, size, stroke weight, and any backing tile are owned by the QML that
consumes it (`design-system/components/TablerIcon.qml`,
`design-system/components/Icon.qml`). See
[docs/design/adr/0185-tabler-line-icons.md](../../../docs/design/adr/0185-tabler-line-icons.md).

| | |
|---|---|
| Upstream | [github.com/tabler/tabler-icons](https://github.com/tabler/tabler-icons) |
| Version | 3.50.0 (`v3.50.0`, commit `6904efd245391ad396df17654738f52f6da8c40e`) |
| Source | the npm package `@tabler/icons@3.50.0`, `icons/outline/` and `icons/filled/` |
| License | MIT — [LICENSE](LICENSE), also shipped as [LICENSES/Tabler.txt](../../../LICENSES/Tabler.txt) |

## Layout

The vendored files keep their upstream names, flattened into this directory:

| File | Weight |
|---|---|
| `<name>.svg` | `outline` (the default; a 2 px-stroke glyph in a 24 px viewBox) |
| `<name>-filled.svg` | `filled` (the solid glyph used by the Dock launcher and the first-party app artwork) |

`TablerIcon` resolves `(name, weight)` to one of these and renders its path
tinted to `color`; outline glyphs stroke, filled glyphs fill. The raw SVG is
exposed to QML unchanged at `qrc:/icons/tabler/<file>`, so a consumer can
always reach the upstream resource.

The inherited `design-system/components/Icon.qml` vocabulary is a compatibility
facade over this set: Files, Settings, and the shell still say `folder`,
`chevron-right`, `wifi`, and so on, and `Icon.qml` maps each name to the Tabler
glyph. New code should use `TablerIcon` directly.

## The pinned set

Only the glyphs a shipped surface uses are vendored, not all 5,000+ upstream
icons: a bounded set is what makes the build-time existence check meaningful
(a referenced glyph that is not here fails `make check-tabler`). Add a glyph by
copying the upstream file into this directory and running
`scripts/gen-tabler-glyphs.py`; the generator and the check gate pick it up
automatically.

## Why both Tabler and Phosphor?

Tabler's outline set is the right match for the Tahoe-like line language of the
menu bar, Files, and the shell chrome. The System Settings category tile needs
a **solid** near-white mark, and Tabler's filled set does not cover every
category, so the tile keeps the pinned Phosphor `fill` weight
(`assets/icons/phosphor/`). The first-party app artwork uses Tabler's filled
`folders`/`settings` marks. The split is recorded in
[ADR 0185](../../../docs/design/adr/0185-tabler-line-icons.md).
