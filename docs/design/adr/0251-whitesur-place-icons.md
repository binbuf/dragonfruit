# 0251 — WhiteSur place and application artwork

| | |
|---|---|
| **Status** | accepted (2026-11-09) |
| **Scope** | Files folder icons, desktop and sidebar places, Trash, the Files app icon, and the Applications launcher |
| **Related** | [interface-naming-and-licensing.md](../interface-naming-and-licensing.md), [licensing.md](../../licensing.md), [10-design-system.md](../10-design-system.md), [ADR 0166](0166-first-party-app-icons.md), [ADR 0185](0185-tabler-line-icons.md) |

## Context

The monochrome Tabler vocabulary is right for chrome, but Files, the desktop,
the sidebar, and the Dock need **colored place artwork** that reads as a
folder, a home, a bin, or an app tile. The original project rule preferred
original or permissively licensed assets, and the earlier icon sets (Tabler,
Phosphor) are MIT. The requested WhiteSur set is a macOS-like open icon theme
under **GPL-3.0-only**, so it cannot be treated like the MIT glyph sets.

WhiteSur's artwork is third-party original work, not Apple's files. The
project still does not copy Apple assets; this ADR records a deliberate visual
and licensing exception, not a change to the no-Apple-artwork rule.

## Decision

- **Vendor a bounded WhiteSur subset** under `assets/icons/whitesur/`, copied
  unchanged from the `2026-09-10` tag (commit
  `73d8040da51a9ed74e47c7366e7e9ff437601a5c`): the generic/XDC folder icons,
  `user-home`, `user-desktop`, the empty/full Trash, computer/volume icons,
  `file-manager.svg`, and `view-app-grid.svg`.
- **Expose the files at `qrc:/icons/whitesur/<name>.svg`** from the design
  system. `Icon.qml` maps the legacy place names to the colored resources and
  draws them as images; `DockGlyph.qml` uses the folder, Trash, and
  Applications-launcher files directly.
- **Make the Files app icon the WhiteSur `file-manager.svg`.** The generator
  copies it byte-for-byte to `assets/icons/apps/org.dragonfruit.Files.svg`, so
  the qrc bundle and the installed hicolor icon stay one reviewable file. The
  Settings app mark remains the Tabler-generated gradient tile.
- **Treat WhiteSur as a separable runtime data asset.** QML loads the SVGs as
  files; no first-party code links against them and the set can be replaced
  independently. The full GPL-3.0 text ships in
  `assets/icons/whitesur/COPYING` and `LICENSES/GPL-3.0-only.txt`; the
  provenance record is `assets/icons/whitesur/README.md`; `NOTICE`, the asset
  manifest, and `docs/licensing.md` carry the row. Any binary distribution
  must ship the license, the provenance record, and the corresponding upstream
  source.
- **Record the exception explicitly.** A new copyleft icon set is a review
  blocker unless it is added this way: separable, bounded, attributed, and
  covered by the asset gate. This is the second copyleft data asset after the
  Capitaine cursor theme.

## Consequences

- Files, the desktop, the sidebar, the Dock stack/Trash/launcher, the file
  chooser, and the Files empty state all resolve the same colored artwork
  through `Icon.qml` or `DockGlyph.qml`.
- The Tabler/Phosphor registries and `make check-tabler` / `make check-phosphor`
  are unchanged; `make check-assets` now validates the WhiteSur row, its
  license text, and the copied Files icon.
- The GPL-3.0-only exception needs legal review before the first public
  release, exactly like the other packaging/licensing review items. If that
  review rejects it, the fallback is to keep the MIT Tabler artwork and remove
  the WhiteSur rows; the call sites stay on `Icon`/`DockGlyph`, so the swap is
  local.
