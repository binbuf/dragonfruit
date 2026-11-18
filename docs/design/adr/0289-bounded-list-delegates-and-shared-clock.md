# 0289 — Bounded list delegates and one shared menu-bar clock

Status: accepted (2026-11-17)

## Context

Two popup families built one object per model row. `DockStackPopover` and
`DockAppPicker` put a `Repeater` inside a `Column` under a bounded viewport, so
a folder with thousands of entries or a full application inventory instantiated
a complete row delegate for every entry even though only a handful fit. A live
snapshot reassigned the array, which recreated every delegate and discarded the
hover, press, and highlight state along with it.

The menu bar had a similar duplication in its measurement path. The status
row's width budget was derived from a hidden row of complete `StatusItem`
delegates. Each delegate embedded a `MenuBarClock`, so the hidden row ran a
second clock boundary timer and built the full hover, tooltip, focus, and tap
tree just to read an implicit width. The visible clock ran the same timer again.

## Decision

- **One reusable virtualized viewport.** `ListViewport` owns a `ListView`, so
  delegate count follows the visible rows plus a two-row cache margin. It
  supplies the same token-styled scrollbars, click behavior, keyboard paging,
  and accessible list role as `ScrollView`, so a popup list and a settings page
  scroll identically.
- **Reconcile snapshots by stable key.** `IncrementalListModel` extends
  `ListModel` and diffs the owner's array by `keyRole` (a folder item's `path`,
  a picker row's `desktopId`). Unchanged rows stay, changed roles emit
  `dataChanged`, and inserted, removed, or reordered rows emit the matching
  row signals. The model is never reset for a live update.
- **Build the model when the popup opens.** `ListViewport.live` is false while
  the owner is closed; opening syncs once. A closed popup does not pay for a
  large folder or inventory it cannot show.
- **Keep selection identity, not just the numeric row.** The stack popup
  follows the selected `path` and the picker follows the selected `desktopId`
  across a live update or a filter change, falling back to the nearest surviving
  row when the identity disappears.
- **Measure status slots passively.** `StatusItemMeasure` uses the same token
  spacing and implicit text layout widths as `StatusItem`, including the
  clock's tabular numerals. The measuring items are invisible and carry
  no input handlers, clock, or wallpaper effects.
- **Share one clock source.** `MenuBar` owns one `MenuBarClock`; the visible
  clock slot and the measurement probe read its formatted text. A status slot
  no longer embeds a clock, so an idle bar has one boundary wakeup.
- **Leave exported menus on the current renderer.** `ContextMenu` and
  `MenuBarMenu` keep their `Repeater` rows. Their measured width, variable row
  heights, and nested level navigation need a different virtualization
  contract; the change is deferred until large-menu measurements justify it.

## Alternatives

- **Keep the `Repeater` and only reduce row cost.** Still one object per row;
  a 10,000-entry folder would remain a 10,000-delegate startup.
- **Reset the `ListModel` on every snapshot.** Simple, but it recreates every
  delegate and loses hover/press/highlight state, which is exactly the problem.
- **Publish a C++ `QAbstractListModel` from the shell.** Correct for a very
  large inventory, but the shell already publishes whole snapshots and the QML
  adapter keeps the popup components testable with plain arrays. Revisit if
  profiling shows the diff cost on the shell thread.
- **Keep the full `StatusItem` measurement row.** Matches widths by
  construction, but keeps the second clock timer and the complete status tree
  for every entry.
- **Format the clock separately in the slot and the probe.** Two formatting
  paths drift on a locale, 24-hour, or seconds change.

## Consequences

- The visible delegate count of a folder stack and the application picker
  follows the viewport, not the model length.
- A live folder or inventory update keeps the delegates for unaffected rows and
  the selected identity, so a watcher burst does not drop hover or press state.
- The measured status budget now depends on `StatusItemMeasure` matching
  `StatusItem`. `tst_menubar` pins the parity for icon, label, icon-plus-label,
  clock with and without the date, unavailable, and file-backed icon rows.
- The status row has one clock boundary timer. The clock text still changes on
  the minute (or second) boundary, and the measurement follows it.
- Exported menus still pay one delegate per row. That remains the next
  virtualization step if the large-menu measurements show it matters.

## Verification

- `tst_design_system` covers the incremental model's role, insert, remove, and
  move signals, the live deferral, the viewport's bounded delegate count, and
  delegate reuse across a role change.
- `tst_dock` opens a 10,000-entry stack and a 10,000-row picker and asserts the
  live delegate count stays bounded; it also inserts a file ahead of the
  selected row and checks that activation still names the same path, and filters
  the picker while checking the same desktop id stays selected.
- `tst_menubar` checks the passive probe against every visible slot shape and
  checks that the clock slot and the measurement probe share one source and own
  no clock child.
