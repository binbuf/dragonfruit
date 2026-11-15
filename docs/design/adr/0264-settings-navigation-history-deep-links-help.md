# 0264 — Settings history is pane + sub-pane + row; one Help action

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-14 |
| **Extends** | ADR [0231](0231-executable-settings-map.md), [0232](0232-settings-search-and-deep-links.md), [0235](0235-general-group-host-adapters.md), [0263](0263-system-settings-window-ia-and-aliases.md) |

## Context

T263 gave the shell a measured window, grouped sidebar, and alias links, but
navigation still recorded bare pane ids. General's in-place sub-panes were not
history steps, a deep link could not reach a row inside a closed sub-pane, the
`?` control used the bare pane id as its fragment, and the View menu listed
four hard-coded panes. The menu model also broadcast unknown actions.

## Decision

1. **History entries are `{ paneId, subPaneId, rowId }`.** `SettingsShell`
   records one entry per navigation state. Back closes a General sub-pane
   before it changes pane; Forward restores the sub-pane and the row. Clicking
   the already-selected General sidebar row closes the sub-pane. The entry is
   the one place the sub-pane/row survives the pane `Loader` swap.
2. **A deep link is accepted only when its row is shipped and landable.**
   `SettingsMap.subPaneForRow()` maps a General row to its in-place sub-pane;
   `rowSurface()` returns `subpane`, `dialog`, or `pane`; `isRowLandable()`
   gates both the deep link and the search index. A row in the Software Update
   dialog opens the dialog first. A row the resolver cannot open is rejected,
   not indexed and not guessed.
3. **The stable id wins.** A rendered row with a `settingsId` is matched only
   by that id; the label/synonym fallback is reserved for rows that predate the
   map wiring.
4. **One Help action.** `SettingsShell.openHelp()` builds
   `Settings.helpUrl("pane-" + id)`, so the fragment is the map's
   `#pane-<id>` anchor. The bottom-right `?` and Help → Settings Help call it;
   without the document the control disables and carries the reason.
5. **The View menu is generated.** `SettingsMenu` derives the Settings Pane
   submenu from `SettingsPanes.shippedPanes` in sidebar order, checks the
   current pane, and marks Back/Forward from the shell's history state.
   `activate()` returns false for an action it cannot name.
6. **Every published action dispatches.** `SettingsWindow.dispatchMenuAction`
   owns the window verbs, pane jumps, About panel, edit verbs on the focused
   text control, Bring All to Front, and Help. The About panel and the in-window
   About page hand the pre-About geometry back when they close.
7. **Focus and scroll are navigation state.** A pane change resets
   `detailScroll.flickable.contentY` and focuses the first keyboard stop in the
   new body; focus is moved off the outgoing body before it is destroyed.

## Consequences

- T265 builds search results on `isRowLandable()`; a new pane or row that needs
  a surface must teach `rowSurface()` or it is not deep-linkable/searchable.
- T269–T281 add `settingsId` to completed rows; the label fallback shrinks as
  panes land.
- A new sub-pane joins `SettingsMap.generalRowSubPanes` and
  `GeneralPane.openRowSurface()`; history needs no second branch.
- The View submenu grows automatically with `SettingsPanes.shippedPanes`; no
  menu edit is needed when T272 ships VPN.

## References

- [24-system-settings.md](../24-system-settings.md) §3.3
- [08-settings.md](../08-settings.md) help/deep-link section
- [settings-business-logic-map.md](../settings-business-logic-map.md) §4
