# Phosphor — the Dragonfruit foreground glyph set

Phosphor is the desktop's icon language for **foreground glyphs** (T-19.1a).
A vendor glyph here is a *resource*: it is a monochrome path with no color or
container baked in. The tint, size, and any backing tile are owned by the QML
that consumes it (`design-system/components/PhosphorIcon.qml`,
`design-system/components/SettingsCategoryIcon.qml`). See
[docs/design/adr/0163-phosphor-icons.md](../../../docs/design/adr/0163-phosphor-icons.md).

| | |
|---|---|
| Upstream | [github.com/phosphor-icons/core](https://github.com/phosphor-icons/core) |
| Version | 2.0.8 (`v2.0.8`, `d42782b2abe747d904b971ccab48b182a1455f86`) |
| Source | `assets/regular/`, `assets/fill/` release tarballs |
| License | MIT — [LICENSE](LICENSE) |

## Layout

The vendored files keep their upstream names, flattened into this directory:

| File | Weight |
|---|---|
| `<name>.svg` | `regular` (the default; a 1.5 px-stroke outline glyph) |
| `<name>-fill.svg` | `fill` (the solid glyph used on gradient tiles and near-white app marks) |

`PhosphorIcon` resolves `(name, weight)` to one of these and renders its path
tinted to `color`. The raw SVG is exposed to QML unchanged at
`qrc:/icons/phosphor/<file>`, so a consumer can always reach the upstream
resource.

## The pinned set

Only the glyphs a shipped surface uses are vendored, not all 1,248 upstream
icons: a bounded set is what makes the build-time existence check meaningful
(a referenced glyph that is not here fails `make check-phosphor`). Add a glyph
by copying the two upstream files (both weights) into this directory and
running `scripts/gen-phosphor-glyphs.py`; the generator and the check gate pick
it up automatically.