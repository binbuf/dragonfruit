# ADR 0306 — Menu Bar Controls is an in-place sub-pane, not a top-level pane

- **Status:** accepted (T293b)
- **Date:** 2026-11-20

## Context

The former **Quick Settings** pane (T227/T277) was a top-level settings pane:
Pages, Control sizes, Page names, Modules, Menu Bar Only, a live preview, Add
Controls…, per-display, and Reset, all over `quickSettings.*` and the one
`ControlCenterRegistry`. The reference design has no such pane: its Menu Bar
page lists the status items and offers **Customize Controls…** for the panel
layout. T285 already removed the top-level id and aliased
`settings://pane/control-center` and `settings://pane/quick-settings` to
Menu Bar → Menu Bar Controls, and T293a mounted the body through a shell-level
alias (`aliasSubPaneId` + `PaneLoader.remount()`).

That alias was a second sub-pane mechanism: the shell swapped the whole pane
body by pane id, `currentSubPaneId` read shell state instead of the body's,
and the pane itself had no `subPaneLoader`, so the ordinary deep-link reveal
path could not find its rows.

## Decision

1. **The surface becomes the `menu-bar` sub-pane `controls`.** `MenuBarPane`
   owns `subPaneId`, `subPaneTitle`, `openSubPane`/`closeSubPane`,
   `subPaneComponent`, and a `subPaneLoader` — the same shape General and
   Accessibility use — with a **‹ Menu Bar** back button. The list and the
   sub-pane are two columns; only one is visible.
2. **The body is the old pane, unchanged.** `QuickSettingsPane.qml` is renamed
   `MenuBarControlsPane.qml` and registered in `Dragonfruit.ControlCenter`
   (source in `shell/control-center/`). It keeps the same `quickSettings.*`
   keys and the same `ControlCenterRegistry`; the Control Center panel and the
   menu-bar entry are untouched.
3. **Placement over an import cycle.** `MenuBarPane` lives in
   `Dragonfruit.Settings`; importing that module from its own directory is a
   cycle, and a QML file under `apps/settings` registered in
   `Dragonfruit.ControlCenter` lands outside the module directory
   (`build/qml/apps/settings/…`) and cannot be resolved. The body therefore
   lives in the Control Center module, whose registry it consumes.
4. **The map owns the placement.** The `control-center` row list is deleted;
   its rows keep their ids, owners, and keys under `menu-bar`
   (`menuBarControlRows`). `subPaneForRow("menu-bar", id)` returns `"controls"`,
   `rowSurface` returns `"subpane"`, and `isRowLandable` needs no special case.
   The shell's alias state and `PaneLoader.remount()` are removed.
5. **Duplicate ids are allowed and disambiguated.** Wi-Fi, Bluetooth, Focus,
   Sound, Battery, and Display exist twice on Menu Bar (status item and moved
   control). `rowFor` returns the first; `rowsWithId` lists all; a row deep
   link opens the sub-pane before it lands.

## Consequences

- One sub-pane mechanism for every in-place surface; Back, history, and deep
  links behave like General's.
- `tst_settings_quick_settings` becomes `tst_settings_menu_bar_controls`, and
  the T277/T227 capture scripts open Menu Bar → Menu Bar Controls.
- The moved duplicate ids mean an exact row label can now outrank its pane in
  global search (`"dark mode"` ranks the moved `menu-bar/darkMode` row above
  Appearance). That ranking needs a scope-aware rule; it is recorded as
  remaining work in the task hand-off, not fixed by demoting the moved rows
  (which would break the "control center"/"quick settings" pane results).
- The `menu-bar` map section title `Menu Bar Controls` now appears for the
  status-item list, the footer actions, and the sub-pane rows; the surface, not
  the title, distinguishes them.
