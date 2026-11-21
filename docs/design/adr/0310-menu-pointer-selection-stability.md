# 0310 — Menu pointer selection stability

Status: accepted (2026-11-20)

## Context

Pointer hover is the primary way a user selects a menu row. The shared menu
components still had two ways to leave the highlight on a different item than
the one under the cursor:

- The owner called `keepRowVisible()` for every highlight change, including a
  hover. Hovering a partially visible row near the viewport edge scrolled the
  list to reveal that row fully. The content moved under the stationary
  pointer, so the row that was selected was no longer the row under the
  cursor.
- `MenuLevels.hover()` cleared the child-panel claim when the pointer moved
  onto a row inside a submenu. A late leave from the opener row could then
  clear the parent highlight while the pointer was still in the child card.

## Decision

- **Pointer selections never scroll the menu.** `MenuLevels` exposes
  `highlightFromPointer` for the duration of a hover or dwell commit.
  `ContextMenu`, `MenuBarMenu`, and `MenuLevelPanel` skip their
  `keepRowVisible()` pass for that change. Keyboard moves, type-ahead,
  identity reconciliation, and the initial open highlight still reveal their
  row. Hovering a partly visible row selects it in place.
- **A hovered nested row keeps its child-card claim.** `hover(level, index)`
  sets the pointer's child-card claim to `level - 1` for nested levels, so the
  opener stays selected while the pointer is in the submenu and a late opener
  leave cannot retire it.
- **The shared row model remains the single selection owner.** The change
  does not add a second hover path or a per-menu copy of the row geometry.

## Consequences

Hovering a row always highlights that row, including at the viewport edge and
inside a submenu. The menu still scrolls for keyboard traversal and for a
model update that moves the selected identity. A pointer that rests on a
partially visible row no longer causes the list to move away from it.

## Verification

`tst_design_system` covers scene-coordinate hover for context menus and menu
bar dropdowns, scroll-under-a-stationary-pointer selection, a clamped submenu
above its owner, the opener highlight while moving through a submenu, and a
partly visible row that must not scroll on hover. `tst_menubar` and `tst_dock`
continue to cover the shell popup geometry, material regions, and dismissal.
