# 0166 — First-party app icons: one shipped SVG per app, keyed by desktop id for nested dev

## Status

accepted

## Context

Track 19 gives our first-party applications Phosphor-based artwork (T-19.1d):
the Dock's **Files** tile (our Finder) and the **System Settings** app icon.
Both must appear in three places that resolve artwork differently:

- the Dock / Applications drawer, which render an app-index themed `iconPath`
  when a theme provides it, and otherwise draw a placeholder initial tile;
- menus and the launcher, which look the icon up by the `.desktop` `Icon=`
  name in the active icon theme;
- the portal file chooser, which resolves the same theme name.

Before an install (nested dev), app-index has no theme containing our apps, so
a themed lookup for `org.dragonfruit.Files` misses and the Dock shows the
placeholder. The previous names, `system-file-manager` and `preferences-system`,
belonged to the host theme and could not guarantee our own artwork.

The System Settings app icon is the one app icon that carries the
`SettingsCategoryIcon` gradient container (ADR
[0164](0164-settings-category-tile.md)); the Files icon is a plain app tile.

## Decision

- **The artwork is one SVG per app, generated from Phosphor.**
  `scripts/gen-app-icons.py` emits `assets/icons/apps/org.dragonfruit.Files.svg`
  (Phosphor `folders` on a flat blue rounded square) and
  `org.dragonfruit.Settings.svg` (the gradient-container treatment — rounded
  square, vertical gradient, top inner highlight, Phosphor `gear`). The glyph
  paths come from the vendored Phosphor SVGs; only geometry/colour constants
  live in the generator. `make check-phosphor` runs the generator `--check`, so
  the assets cannot drift.
- **The same file is both the bundled resource and the installed icon.** The
  design system exposes `assets/icons/apps/` at the stable prefix
  `qrc:/icons/apps/<name>.svg`, and each app's `CMakeLists.txt` installs its
  SVG into `${datadir}/icons/hicolor/scalable/apps/`. Thus the Dock, the
  drawer, menus, and the portal resolve one artwork source.
- **The Dock's first-party mapping is keyed by desktop id.** `DockGlyph` gains
  a `desktopId`; a small id → bundled-tile map (`org.dragonfruit.Files` /
  `org.dragonfruit.Settings` after stripping `.desktop`) selects the `qrc`
  resource. The bundled tile wins for our own apps; app-index's themed
  `iconPath` remains the fallback for third-party apps (and would resolve the
  identical file once installed). `DockEntry`, `DockAppPicker`, and
  `DockOverflowPopover` forward the desktop id.
- **The `.desktop` names change to the app ids** (`Icon=org.dragonfruit.Files`,
  `Icon=org.dragonfruit.Settings`), which resolve in a clean session against
  the installed hicolor theme.
- **`DragonfruitLogo.qml` and the Dock's `stack`/`trash`/`overflow` artwork are
  untouched.**

## Consequences

- Nested dev is correct before any install: the Dock renders the bundled tile
  from `qrc`, and the id is the join key the drawer reuses in T-19.2.
- A grep/scan can find an app icon by desktop id; the Rust test
  `shipped_first_party_app_icons_resolve_in_hicolor` ties the shipped `.desktop`
  `Icon=` names to the asset files and the theme resolver, and the QML Dock test
  asserts Files → bundled tile plus the third-party `iconPath` preference.
- The generator is the only place to change a first-party icon's colour or
  glyph; re-run it and `make check-phosphor`. A new first-party app adds one
  generator entry, one qrc asset, one install rule, and one table row in
  `DockGlyph`'s map.