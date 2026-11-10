# 0248 — One status-item registry; the clock becomes a Date & Time extra

Status: accepted

## Context

The menu bar's right side had grown by accretion: Wi-Fi, Bluetooth, battery,
Sound and Focus were `StatusItem` delegates built in C++, while the clock,
Quick Settings and Mission Control were fixed siblings in `MenuBar.qml`. The
tray items were appended to the same list but had no ordering contract, and
there was no overflow rule. The clock doubled as the Notification Center entry
(T204), which left no pointer surface for a calendar.

T231 requires one model for system and third-party extras, a Date & Time item
with format settings and a calendar panel, and deterministic overflow.

## Decision

1. **One normalized entry shape** (`id`, `icon`/`iconSource`, `label`,
   `tooltip`, `menu`, `priority`, `visible`, `available`, `enabled`,
   `selected`, `state`, `accessibleName`, `kind`, `thirdParty`). The shell
   controller produces it in `applyStatusItems`; the QML registry consumes it.
2. **`StatusItemRegistry.qml` owns order and overflow.** System items sort by
   priority; every third-party item sorts after the system block. A finite
   budget removes third-party items first, then optional system items;
   `essential` items (clock, Quick Settings, Overview) stay. Removed items are
   listed in one overflow `ContextMenu`. T233 feeds the budget from measured
   width and owns notch/multi-display refinement.
3. **The clock is a registry item.** `StatusItem` embeds the formatting-only
   `MenuBarClock` and owns the slot's hover, focus ring, tooltip, tap and
   accessibility node. Its click opens `DateTimePanel`, a `Popup` using the
   `statusMenu` material preset, not a menu.
4. **Notifications is the Center entry point.** The clock no longer toggles
   the Notification Center; the registry's bell item does. Search opens the
   Applications drawer's search field.
5. **`menu.control.*` remains the durable visibility contract.** Revision 24
   adds `menu.control.clock`, `menu.control.notifications` and
   `menu.control.controlCenter`; `menu.control.search` gains its consumer.

## Consequences

- A third-party item can be hidden by overflow but is never silently dropped:
  it appears in the overflow menu.
- An absent service keeps a hidden zero-width slot, so later indices do not
  shift (the existing `StatusItem.available` contract).
- The clock's click no longer opens the Notification Center. Any keyboard
  shortcut/hot-corner path is unchanged; the bell item is the pointer entry.
- The Menu Bar pane's rows for the three new keys are T233's; the keys are
  live in the shell today.
- The shared `Popup.anchorPoint` now reads the anchor's live geometry, so a
  popup created before the status row's first layout still anchors to its
  item (previously it stayed at x=0).
