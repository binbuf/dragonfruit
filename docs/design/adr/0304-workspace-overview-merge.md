# ADR 0304 — Workspace Overview is one Desktop & Dock section plus one advanced sheet

- **Status:** accepted (T292b)
- **Date:** 2026-11-20

## Context

The mockup's Desktop & Dock ends with a single **Workspace Overview** section:
a description and five toggles (Automatically rearrange Workspaces, Switch to an
app's Workspace, Group windows by application, Displays have separate
Workspaces, Drag
windows to the top to enter Workspace Overview). Its footer carries
**Shortcuts…** and **Hot Corners…**.

The desktop still needed a home for the former Mission Control pane's real
rows: the workspace count (`workspaces.count`), the gesture keys
(`gestures.missionControl`, `gestures.spaceSwitch`), and the four corner
assignments with their modifiers (`overview.hotCorner*`). T285 had already
removed the top-level Overview pane and mapped `mission-control`/`overview` to
Desktop & Dock → Workspace Overview; T292a left a temporary disclosure entry
in its place.

## Decision

1. The section renders the five mockup toggles with their existing owners and
   keys: `overview.switchToAppSpace` is live; `overview.autoRearrange`,
   `overview.groupByApp`, and `overview.dragToOpen` stay visible and disabled
   with a user-facing reason; per-display Spaces is the read-only compositor
   fact (`overview.perDisplay`). No toggle is invented for a runtime that does
   not exist.
2. The real workspace count, the two gesture keys, and the four corner
   assignments (action + modifier popups) live in the **Advanced Workspace
   Overview** sheet — an organization change, not new behavior. Every row
   keeps its owner, key, order, and control.
3. The footer's **Hot Corners…** opens that sheet on this pane; **Shortcuts…**
   keeps handing off to Keyboard → Shortcuts. Neither control is duplicated
   inline, and there is no second Overview pane or hot-corner list.
4. `MissionControlPane.qml`, its dedicated tests, its CMake entries, and the
   shell's unused pane component are deleted. The `mission-control` id survives
   only as the deep-link alias and as the stored corner-action value.
5. `SettingsMap.rowSurface()` answers `dialog` for the seven sheet rows
   (workspace count, two gestures, four corners) so a deep link opens the sheet
   and lands on the row.

A two-popup corner row does not fit the 480 pt dialog default, so the sheet
asks for a measured 560 pt card through a new `Dialog.preferredWidth` property
(capped to the window); the change is additive and every other dialog keeps
the content-derived width.

## Consequences

- The map has one `desktop-dock` inventory and no `mission-control` rows; the
  removed-pane record and the aliases are unchanged and still documented.
- Capture scripts that reached the removed pane (`t276-*`, `t281-*`) now land on
  the merged section; the T292b capture target writes the section, the footer,
  the advanced sheet, and the absence pair in light and dark.
- A later task that needs inline hot-corner editing re-opens this decision; the
  sheet is the one place the corner list may appear.
