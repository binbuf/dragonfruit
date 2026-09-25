# macOS UI inventory (reference screenshots)

Distilled from the local art-direction screenshots in `docs/reference/macos/`
(real macOS captures; **local reference only, never shipped** — see
[14-risks.md](../design/14-risks.md)). Each image was reviewed with the vision
model configured for this project; the raw per-image inventories live in
`docs/reference/macos/notes/` (local, git-ignored).

This is an interaction and information-architecture guide: reproduce the
structure and quality with Dragonfruit's own symbols, terms, and materials
([10-design-system.md](../design/10-design-system.md)). It also covers
Spotlight search, the Control Center quick-settings sheets, About This
System, the applications grid, and Activity Monitor. System Settings panes
have their own reference: [System_Preferences.md](System_Preferences.md).

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

## System menu

Reference: `Screenshot 2026-09-18 at 9.20.46 PM.png` (Apple menu).

- The rounded translucent dropdown summarized under **Menu bar**: thin
  separators group the rows, and there is no header row.
- Items in order: `About This Mac`, `System Settings...`, `App Store...`
  (badge `6 updates`), `Recent Items` (submenu chevron), `Force Quit System
  Settings` (`⌥⌘⎋`), `Sleep`, `Restart...`, `Shut Down...`, `Lock Screen`
  (`⌃⌘Q`), `Log Out <user>...` (`⇧⌘Q`).
- Uniform rows, ~320–350 px wide; monochrome leading icons; badges,
  chevrons, and shortcut hints right-aligned.
- Ours: the shell's system menu, with `About This System` in place of the
  Apple name ([04-shell.md](../design/04-shell.md)).

## Spotlight search

Reference: `Screenshot 2026-09-18 at 9.24.16 PM.png` (search field),
`Screenshot 2026-09-18 at 9.24.19 PM.png` (field and quick actions).

- Rounded, horizontally elongated search pill: solid light-gray fill, subtle
  inner shadow, dark-gray text, magnifier glyph on the left. Placeholder is
  `Spotlight Search`, left-aligned.
- Empty but focused: cursor active at the leading edge of the placeholder.
- Trailing quick-action strip: icon-only circular buttons in light-gray
  fill with dark-gray glyphs and even spacing (reference: App Store, folder,
  overlapping-windows, and overlapping-documents glyphs). No pressed,
  disabled, or hover state captured.
- Ours: the app-index work
  ([T-14](../design/tracks/14-global-menu-app-index-compat.md)) is the
  Spotlight equivalent; the shell search UI has no tracked task yet.

## Control Center

Reference: `Screenshot 2026-09-18 at 9.23.47 PM.png` (Bluetooth),
`Screenshot 2026-09-18 at 9.23.53 PM.png` (Wi-Fi).

- Quick-settings sheets open as modal rounded panels floating over the
  desktop; no title bar, traffic lights, or toolbar. Frosted translucent in
  the Bluetooth capture, flatter in the Wi-Fi capture.
- Header row: bold pane title (`Bluetooth` / `Wi-Fi`) at top-left with an
  on/off toggle at top-right (on = accent blue).
- Rows: circular leading icon tile (accent blue when connected or selected,
  gray otherwise) plus label; a small monochrome lock glyph trails secured
  networks.
- Wi-Fi groups rows under `Known Network` and a collapsible `Other Networks`
  header, then `Other...` and `Wi-Fi Settings...`; Bluetooth lists connected
  devices, then `Bluetooth Settings...`.
- The `... Settings...` rows link into the full System Settings pane.
- The full Control Center panel itself was not captured.

## About This System

Reference: `Screenshot 2026-09-18 at 9.22.28 PM.png` (About This Mac).

- Standard rounded window with traffic lights; no sidebar or toolbar; solid
  light background.
- Centered device illustration (line-art laptop, accent-blue screen), then
  the centered device name `MacBook Pro` with a muted generation line
  `14-inch, M5`.
- Centered label/value spec block, labels right-aligned: `Chip` `Apple M5`,
  `Memory` `16 GB`, `Serial number` `<serial>`, `macOS` `Tahoe 26.6.2`.
- Rounded primary button `More Info...`; footer hyperlink `Regulatory
  Certification` over "™ and © 1983-2026 Apple Inc. All Rights Reserved.".
- Ours: `About This System` in the system menu, plus Settings > General >
  About.

## Applications grid (Launchpad)

Reference: `Screenshot 2026-09-18 at 9.22.17 PM.png`.

- Near-full-screen rounded applications window: leading launcher glyph, a
  large `Applications` search field, and a trailing three-dot menu button;
  light translucent chrome.
- Category filter bar of pill-shaped segments: `Developer Tools`,
  `Productivity & Finance`, `Utilities`, `Entertainment`, `Games`, `Social`,
  `Creativity`, `Information`; none selected in the reference.
- Grid: 7 columns of ~64 px rounded app tiles with centered single-line
  labels below, truncated with ellipsis (`Adobe Illustrato...`); flat, no
  grouping borders; scrollable, bottom row clipped.
- No tracked task — recorded as a gap.

## Activity Monitor

Reference: `Screenshot 2026-09-18 at 9.23.13 PM.png`.

- Standard rounded window with a translucent title bar; title `Activity
  Monitor`, muted subtitle `All Processes`, traffic lights.
- Toolbar: circular info and options (`⋯`) buttons by the traffic lights;
  segmented tabs `CPU` / `Memory` / `Energy` / `Disk` / `Network` (CPU
  selected); search field at the trailing edge with `Search` placeholder and
  magnifier.
- Detail pane is one full-width process table with sortable headers:
  `Process Name` (sorted), `% CPU`, `CPU Time`, `Threads`, `Idle Wake Ups`,
  `Kind`, `% GPU`, `GPU Time`, `PID`, `User`; numeric values right-aligned,
  small leading icons on process rows.
- Footer: three columns — `System:`, `User:`, `Idle:` (each colored), a
  `CPU LOAD` sparkline, and `Threads:` / `Processes:` counts.
- No tracked task — recorded as a gap.