---
verify: make gates
---
# T204 — Map the Notification Center and route its actions

## Goal

Make Notification Center usable from the menu-bar clock and existing compositor action, using live notification history.

## Context (read first)

- Depends on T197 and T201. `shell/notifications/NotificationCenter.qml` is currently a tested but unmapped QML list.
- `shell/menubar/MenuBar.qml::clockActivated`, `compositor/src/input/{action,shortcuts,hot_corners}.rs`, and the existing shell overlay paths in `shell/src/{shellcontroller,shellprotocol}.cpp`.
- `shell/src/{notificationclient,notificationmodel}.*` and `services/notifications/src/{dbus,view}.rs` define history, banner and action ownership.

## Scope

- [ ] Host the existing Center in a normal trusted shell overlay with its own panel/input region. Connect the clock and declared Notification Center keyboard/hot-corner action through existing action dispatch. Anchor on the invoking output and close cleanly on Escape, outside press, lock, output loss or competing overlay.
- [ ] Bind the live history model with stable identity, an accessible empty state, and keyboard traversal. Route supported active notification actions/dismissal through the existing service contract. Treat expired, closed, or DND-suppressed history as history with no stale active actions.
- [ ] Keep banner delivery, expiry, replacement, urgency, Focus policy and existing badge semantics with the service. Handle service absence and reconnect. Restore previous focus and clear the backdrop and input region when the Center closes.

## Out of scope

- Notification material calibration (T205), new widget/calendar/gallery services and new actions on historical entries.

## Design notes

Reuse the shell's overlay configure/commit/pacing infrastructure and existing notification service. Do not duplicate history in QML or duplicate active banner actions. Add only the shell action or route missing from the existing private interface. Keep lock isolation and avoid allocating a hidden active layer.

## Done when

- [ ] Integration coverage proves clock, keyboard and configured hot-corner open/toggle; live notifications and DND-suppressed history; Escape/outside close and focus restore; service restart; lock and output teardown. Exercise empty and populated states.
- [ ] `cargo test -p dragonfruit-notifications`, `tst_notificationmodel`, `tst_notifications`, and `make gates` pass. Document the entry points in `docs/design/04-shell.md` and fill in Hand-off.

## Hand-off

_(Implementation session records routing, surface ownership and service behavior.)_
