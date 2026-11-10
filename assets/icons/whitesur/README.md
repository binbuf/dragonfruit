# WhiteSur — place and application artwork

WhiteSur is the desktop's **colored place and app artwork** set: the Files
folder icons, the Trash bin, the desktop/home locations, and the Files and
Applications launcher tiles. The monochrome interface vocabulary stays on
Tabler ([`assets/icons/tabler/`](../tabler/README.md)); WhiteSur is deliberately
not tinted, because its folder/app artwork carries its own color.

| | |
|---|---|
| Upstream | [github.com/vinceliuice/WhiteSur-icon-theme](https://github.com/vinceliuice/WhiteSur-icon-theme) |
| Version | `2026-09-10` tag, commit `73d8040da51a9ed74e47c7366e7e9ff437601a5c` |
| Source | `alternative/places/scalable/`, `alternative/apps/scalable/`, `src/apps/scalable/` |
| License | GPL-3.0-only — [COPYING](COPYING), also shipped as [LICENSES/GPL-3.0-only.txt](../../../LICENSES/GPL-3.0-only.txt) |

## The pinned set

The files are copied unchanged from upstream. Only the assets a shipped
surface uses are vendored, so the set stays reviewable:

| File | Upstream path | Used for |
|---|---|---|
| `folder.svg` | `alternative/places/scalable/folder.svg` | Generic folders |
| `folder-documents.svg` | `alternative/places/scalable/folder-documents.svg` | Documents |
| `folder-download.svg` | `alternative/places/scalable/folder-download.svg` | Downloads |
| `folder-images.svg` | `alternative/places/scalable/folder-images.svg` | Pictures / wallpaper |
| `folder-music.svg` | `alternative/places/scalable/folder-music.svg` | Music |
| `folder-videos.svg` | `alternative/places/scalable/folder-videos.svg` | Videos / movies |
| `folder-public.svg` | `alternative/places/scalable/folder-public.svg` | Public |
| `folder-templates.svg` | `alternative/places/scalable/folder-templates.svg` | Templates |
| `user-home.svg` | `alternative/places/scalable/user-home.svg` | Home location |
| `user-desktop.svg` | `alternative/places/scalable/user-desktop.svg` | Desktop location |
| `user-trash.svg` | `alternative/places/scalable/user-trash.svg` | Empty Trash |
| `user-trash-full.svg` | `alternative/places/scalable/user-trash-full.svg` | Full Trash |
| `computer.svg` | `alternative/devices/scalable/computer.svg` | Computer location |
| `drive-harddisk.svg` | `alternative/devices/scalable/drive-harddisk.svg` | Internal volumes |
| `drive-removable-media.svg` | `alternative/devices/scalable/drive-removable-media.svg` | Removable volumes |
| `file-manager.svg` | `alternative/apps/scalable/file-manager.svg` | Files app icon |
| `view-app-grid.svg` | `src/apps/scalable/view-app-grid.svg` | Applications launcher |

`Icon.qml` resolves the legacy place names (`folder`, `documents`,
`downloads`, `wallpaper`, `music`, `movies`, `home`, `displays`, `trash`) to
these files. `DockGlyph.qml` uses `folder.svg` for folder stacks,
`view-app-grid.svg` for the Applications launcher, and the two trash files for
the empty/full Dock Trash.

## Licensing and distribution

WhiteSur is **GPL-3.0-only**, unlike the MIT Tabler/Phosphor glyph sets. It is
handled as a separable data asset: the SVGs are loaded by QML at runtime, never
linked into first-party code, and the theme directory can be replaced
independently. Any binary distribution must ship the GPL-3.0 text, this
provenance record, and the corresponding source. The project-level exception is
recorded in
[docs/design/adr/0251-whitesur-place-icons.md](../../../docs/design/adr/0251-whitesur-place-icons.md)
and [docs/licensing.md](../../../docs/licensing.md).

The files are third-party open artwork, not Apple assets. Do not substitute
Apple's Finder/Trash artwork or add files whose upstream license is unclear.
