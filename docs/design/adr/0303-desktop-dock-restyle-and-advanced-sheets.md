# 0303 — Desktop & Dock: one mockup surface, advanced sheets, and a live Overview entry

- **Status:** Accepted (T292a)
- **Date:** 2026-11-20
- **Context:** [25-system-settings-visual-parity.md](../25-system-settings-visual-parity.md)
  §5.6, [24-system-settings.md](../24-system-settings.md) §4.20–4.21,
  [settings-business-logic-map.md](../settings-business-logic-map.md) §5.12,
  ADR [0277](0277-desktop-dock-overview-and-windows-rows.md),
  ADR [0287](0287-system-settings-visual-parity.md)

## Context

T276 built Desktop & Dock as one long pane: a twelve-row Dock group (ten
settingsd rows plus two Linux-only rows), Desktop & Stage, Widgets, Windows
(including a disabled window-rules row), Overview links into the separate
`mission-control` pane, a default-browser card, and the global application-menu
toggle. The visual-parity document asks for the mockup's five sections and
footers, and T292b will merge the former Mission Control rows into a Workspace
Overview section. T292a is the rest of the pane.

## Decision

- **One main surface in mockup order:** Dock, Desktop & Workspaces, Widgets,
  Windows, Window Overview, footer. Labels, the Workspace Focus rename, the
  Super tiling modifier, and the default-browser placement in Widgets follow
  the mockup. The default browser keeps `defaults.browser` and its delegate.
- **Linux-only rows keep their owners but leave the main pane.** Advanced
  Dock… (hover chooser, minimize reaction) and Advanced Windows… (window rules)
  are footers that open `Dialog`s. Their keys still apply immediately when
  changed in the sheet; the window-rules row stays disabled with its reason.
- **The map answers `dialog` for the three sheet rows.** A deep link opens the
  sheet through `openRowSurface`/`rowTargetFor` and lands on the row, the path
  Appearance established in T290f.
- **The menu-bar toggle is gone from this pane.** `menu.global` belongs to the
  Menu Bar pane (T293a); keeping it here would be a second owner surface.
- **The Window Overview section is a live disclosure**, not the merged
  section. Its one row opens the existing `mission-control` surface; the alias
  `settings://pane/mission-control` still lands on the section. T292b replaces
  the entry with the merged rows and deletes that pane; nothing here may become
  a dead action in the meantime.
- **Shortcuts… and Hot Corners… are footer actions.** Hot Corners… opens the
  existing hot-corner surface; Shortcuts… is a real, stable footer action that
  reports the shortcut editor as unavailable. Neither control is duplicated
  inline. The footer is therefore an action surface, not a renamed row: the
  map keeps the `shortcuts`/`hotCorners` ids and their absence kinds, but the
  renderer test classifies them as footer actions beside the rows (the
  Appearance footer uses the same rule).

## Consequences

- `SettingsMap.rowsFor("desktop-dock")` follows the new section titles; the
  map's section titles equal the rendered group titles.
- Every rendered row carries search synonyms, including the three sheet rows
  (with an "advanced dock"/"advanced windows" term) and the Overview entry.
- `make t292a-settings-desktop-dock-capture` is the repeatable visual evidence;
  T292b reuses the advanced-sheet and footer plumbing when the Overview rows
  move in.
