# macOS UI inventory (reference screenshots)

Distilled from the local art-direction screenshots in `docs/reference/macos/`
(real macOS captures; **local reference only, never shipped** — see
[14-risks.md](../design/14-risks.md)). Each image was reviewed with the vision
model configured for this project; the raw per-image inventories live in
`docs/reference/macos/notes/` (local, git-ignored).

This is a **style and information-architecture guide, not a copy list**:
reproduce the interaction structure and quality with Dragonfruit's own
symbols, terms, and materials ([10-design-system.md](../design/10-design-system.md)).
System Settings panes have their own reference:
[System_Preferences.md](System_Preferences.md).

## Finder (Files)

Reference: `Screenshot 2026-09-18 at 9.20.30 PM.png` (window over Desktop),
`Screenshot 2026-09-18 at 9.20.18 PM.png` (desktop, Dock, menu bar).

### Window and toolbar

- Rounded window, drop shadow, light appearance; traffic lights; centered
  window title (`Desktop` in the reference).
- Left: back/forward **chevron pills**; back enabled, forward dimmed at the
  end of history.
- Center-left: a **four-way view segmented control** (icon, list, column,
  gallery glyphs) with a trailing dropdown for view options.
- An action group at the trailing edge of the titlebar controls (share, tag,
  more) — reference-only; our MVP keeps the actions that have backends.
- Right: a **search field** with a magnifier glyph and `Search` placeholder.
- Footer: a **path bar** breadcrumb (`Macintosh HD > Users > <user> >
  Desktop`) with chevrons between segments.

### Sidebar

- Sections with muted headers: `Favorites`, `Locations`, `Tags`; a
  `Recents`/`Shared` group above Favorites in the reference.
- Icon + label rows; the selected row uses a tinted rounded highlight with
  accent-colored text (reference light mode: light blue fill, blue text).
- Favorites carry user folders; Locations carry the machine/volumes/network
  and `Trash` sits last; Tags render as color dots.
- Scrollable, ~25% of window width; the detail pane is fluid.

### Detail pane

- Icon view: tiles of roughly 64 px with the label centered below (two-line
  wrap); selected items get the accent treatment.
- List view rows are uniform height with sortable headers (reference shows
  the icon view).
- Empty areas are simply background; the reference has no adornments.

## Desktop

Reference: `Screenshot 2026-09-18 at 9.20.18 PM.png`.

- Plain wallpaper with grid-aligned desktop icons, label under the icon.
- Desktop is owned by the file manager (see
  [09-files.md](../design/09-files.md)); no separate desktop window.
- Light appearance default; the menu bar and Dock float over the wallpaper.

## Dock

Reference: `Screenshot 2026-09-18 at 9.20.18 PM.png`.

- Translucent, rounded slab at the bottom center, icons evenly spaced.
- Left-to-right: pinned apps, then a separator before the right-end items —
  `Downloads` (folder) and `Trash` (full/empty states).
- Running apps show the **running indicator** below the icon; no labels.
- Icon size and magnification are uniform across the row; the Dock has no
  visible background border beyond the material.

## Menu bar

Reference: `Screenshot 2026-09-18 at 9.20.18 PM.png`,
`Screenshot 2026-09-18 at 9.20.46 PM.png` (system menu).

- Left: bold application name, then the app's menus (`File`, `Edit`, `View`,
  `Go`, `Window`, `Help` in Finder).
- The **system menu** opens as a rounded translucent dropdown with session
  items and their shortcut hints: About This Mac, System Settings…,
  App Store… (updates badge), Recent Items, Force Quit…, Sleep, Restart…,
  Shut Down…, Lock Screen, Log Out…. Separators group the sections; the
  menu has no header row.
- Right: status items (reference: Launchpad/iCloud, Bluetooth, Wi-Fi,
  battery, Spotlight/search, Control Center) plus the date and time.
- Items are uniformly sized and behave identically in light/dark.