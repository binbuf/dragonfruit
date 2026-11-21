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

## Amendment (2026-11-21)

The first pass gated the root list only. Two paths still moved content under
a stationary pointer:

- `ContextMenu` scheduled `submenuPanel.keepHighlightVisible()` from
  `onOpenSubmenuIndexChanged` and `onSubmenuHighlightedIndexChanged` without
  reading `highlightFromPointer`. A tall submenu therefore still scrolled on
  hover. Because Qt re-evaluates hover when items move, the scroll changed
  the row under the cursor, which selected another row and scrolled again:
  the feedback loop read as a flicker and a wrong selection.
- `MenuBarMenu` sized its dropdown viewport from
  `rowColumn.implicitHeight`. After a live model update the delegates still
  reported the previous model's height for a frame, so the viewport and its
  scrollbar changed while the pointer was over the menu.
- The menu-bar popup rectangle and material regions were mapped into the
  bar's *local* coordinates. That dropped the bar's reveal `Translate`, so
  the popup surface was placed a strip away from the QML popup while the bar
  slid, and the row under the pointer was wrong. The mappings now target
  window coordinates (`mapToItem(null, …)`), matching the surface placement
  and the pointer translation.

Both owners now read the pointer flag when they *schedule* the deferred
reveal (the deferred call runs after `setHighlightFromPointer` restores it),
and the menu-bar viewport derives its height from `desiredPopupHeight` and
the entry list.

The sibling crossing was also too broad. `hover()` kept the opener selected
for the whole 150 ms dwell whenever *any* sibling was hovered, so sweeping
across a menu with a child open left the highlight on the opener and never
on the row under the cursor. The corridor is now geometric: `MenuRow`
reports the pointer's x within the row, and the owner keeps the opener only
on the strip beside the child (`component.contextMenu.rowHeight` wide, on
the side the child opened). A pointer over the row body selects that row
immediately; the child still stays open until the dwell commits, so a
diagonal pass that reaches the child is unchanged.

`tst_design_system` adds a partly visible child row that must not scroll on
hover, and a sibling-body hover that must select the sibling while the child
stays open; `tst_menubar` adds row-hover selection and a bounded dropdown
that must not scroll on hover.

## Amendment (2026-11-21, render-target origins)

Content-sized popup targets render the same root item from a different origin.
The menu bar translates its item to the dropdown origin for the
`menubar-popup` view, and the Dock translates its item to the strip origin, the
popover origin, and then back to the input scene origin. Qt flushes a
frame-synchronous hover at the last injected pointer position during polish
(`QQuickDeliveryAgentPrivate::flushFrameSynchronousEvents`), so a pointer left
at the previous window position was resolved against the translated tree. The
popup origin was effectively counted twice: hovering a row selected a row
several rows below it, and the Dock's strip and popup views could disagree by
the popover headroom.

Both owners now rebase the injected pointer whenever the render origin changes,
and restore it with the item. The pointer is always the same item-local point
(`window = item-local + item-origin`), so the strip render, the content-sized
popup view, and the restored input scene all see the row under the cursor.
`tst_chromepointerui` reproduces the translated-item hover jump and verifies
that rebasing the injected pointer keeps the selected row stable.
