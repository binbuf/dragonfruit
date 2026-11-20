# 0278 — Quick Settings and Menu Bar are management surfaces over one registry

## Status

Accepted (T277).

## Context

T227 shipped Quick Settings as a flat 37-row Up/Down list and Menu Bar as a
visibility list. Both drifted from the measured shell: the Quick Settings pane
kept its own module catalogue instead of the T235 `ControlCenterRegistry`, the
empty list silently restored the defaults, Edit Controls was a no-op, the
per-display row claimed a surface that does not exist, and the Menu Bar pane
had no per-control options, no third-party StatusNotifier list, no Add Controls,
no Reset, and no Linux-only overflow/workspace rows.

T235 already made `ControlCenterRegistry` the one source for the panel grid,
the 22 categories, the spans, the pages, and the `menu.control.*` mapping.
`EditControls.qml` already writes the same `quickSettings.*` keys. A second
catalogue in the Settings app would be a second truth.

## Decision

1. **The Settings Quick Settings pane consumes `ControlCenterRegistry`.** It
   renders the registry's 37 modules grouped by the same 22 categories, with a
   category filter and the measured 4-column preview. It does not keep a
   module list, a label table, or span math of its own.
2. **An explicit empty module list is a real state.** Only an absent
   `quickSettings.modules` falls back to the measured default; an empty list
   renders only the Edit Controls footer. Reset is the documented way back.
3. **A module with no provider is never addable.** The registry carries the
   reason; the Add Controls sheet lists the module disabled with that reason.
   Removing an older persisted entry stays possible.
4. **Pages are durable and named.** `quickSettings.pageNames` joins
   `page2`/`page3`/`pageCount`/`activePage`; add/rename/delete/select all write
   through the one settingsd path. `EditControls.qml` shares the names and the
   reset.
5. **Reset owns the whole surface.** It restores modules, pages, names, spans,
   menu-bar-only, per-display, and the `menu.control.*` marks the surface
   writes. A reset never leaves a control hidden with no visible reason.
6. **The per-display row is disabled with the exact reason.** The shell ships
   one Quick Settings surface; per-display panels need a compositor overlay per
   output.
7. **Menu Bar per-control options have real consumers.** Battery percentage is
   the existing `power.showPercentage`; Focus `menu.control.focusAlways` keeps
   the crescent while Focus is off; Sound `menu.control.volumeWhenActive` hides
   the mark while muted. Display and Screen Casting are disabled rows with the
   exact "no menu-bar item" reason.
8. **Third-party items are a live list with a durable allowance.** The pane
   reads app-index's `TrayItems` view; `menu.tray.hidden` is the hidden-name
   list the shell reads in `applyStatusItems`.
9. **The Linux-only rows have owners.** `menu.overflow` (`automatic`/`always`)
   drives `StatusItemRegistry.forceOverflow`; `menu.control.workspaces` adds
   the active-Space projection from `ShellProtocol::overviewWorkspaces()`;
   `menu.global` is the existing global-menu toggle.

## Consequences

- The panel, Edit Controls, and the Settings pane cannot drift: one registry,
  one key family, one reset.
- `SCHEMA_VERSION` is 34; the new keys are additive and documented in
  `docs/design/settings-keys.md`.
- A future per-display panel must remove the disabled row and give the policy
  a real surface; the key is already durable.
- A future menu-bar Display or Screen Casting item must replace the disabled
  row with a live `menu.control.*` consumer, not a second settings key.
