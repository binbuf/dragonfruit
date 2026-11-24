# 0266 — Settings component contracts: overlay layer, row anatomy, and scroll modes

## Status

Accepted (2026-11-15, T266).

## Context

Every System Settings pane builds from the same `SettingsRow` / `SettingsGroup`
/ `Select` / `Dialog` / `ScrollView` components. The 2026-11-14 review found
the shared layer itself was the defect: a `SettingsRow` binding loop, no row
focus ring, clipped Selects and dialogs inside the detail `ScrollView`, long
strings squeezing labels, double group gaps, final separators, a dead
horizontal scroll path, indistinguishable scroll modes, and resize grips over
the scrollbar corner. Per-pane workarounds (the `popupParent` opt-in) made the
same problems reappear in every new pane.

## Decision

1. **One window overlay.** `WindowOverlay` registers with the `OverlayLayer`
   singleton for its window. `Select`/`ContextMenu`, `Popup`, and `Dialog`
   adopt that layer themselves; panes declare no parent and no `popupParent`
   property. The overlay covers the window above the resize grips and has no
   pointer handlers of its own.
2. **`SettingsRow` is sized from the child, not from layout.** The control
   slot reads the child's implicit size and caps at 60% of the row. The row
   owns a `FocusRing`, activates a disclosure from anywhere on the row, makes
   the chevron/help Tab stops, and fills a control's `rowLabel`/accessible name
   from the row label when the pane sets none.
3. **`SettingsGroup` owns one gap.** Titles wrap or elide, the final row's
   separator is suppressed, and the group's own bottom margin is opt-in so a
   pane's spaced Column supplies the single gap.
4. **Dialogs fit and trap.** A dialog is bounded by the window, scrolls only
   its body, keeps the action row pinned, traps Tab, closes on Escape, and
   always offers a visible Close action (unless it is not dismissible). Its
   width reads implicit content sizes only, never its own laid-out width.
5. **Scroll modes are distinct.** `automatic` shows for overflow while the
   view is focused, hovered, or moving; `when-scrolling` only while moving or
   dragging a thumb; `always` for any overflow at rest. The component exposes
   `scrollBarValue`/`scrollBarOrientation` and both macOS track-click
   behaviours. The horizontal path is live behind `horizontalScroll`.
6. **The resize corner is reserved.** The Settings shell insets its scroll
   bars by the resize-corner size, so a grip and a scrollbar thumb never share
   a pointer target.

## Consequences

- Panes get one popup/dialog behavior and no opt-in parent wiring. A new pane
  cannot reintroduce clipping by forgetting a property.
- Qt 6.11's QML `Accessible` attached object has no value/range slots. `Slider`
  exposes the conventional `value`/`minimumValue`/`maximumValue`/`stepSize`
  properties for the accessible value interface and publishes the human range
  in `Accessible.description`; `Select` publishes its current label the same
  way. A future C++ accessibility object can replace the description without
  changing the component API.
- `automatic` now follows focus/hover/motion rather than always painting an
  overflowing bar; this is the reference behavior and is covered by
  `tst_settings_components`.
- The compositor's live-position ray-test remains the stationary-click fix;
  the QML tests pin the row/tile/select/dialog and scroll-under-pointer half.
- A closed `Dialog`/`Sheet` is input-disabled (`enabled: open`); its card is
  render-only during the close fade, so it can never shadow a pane control
  (ADR [0341](0341-closed-modal-is-input-disabled.md)).
