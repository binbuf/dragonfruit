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

Implemented on the replacement rail.

**Routing and surface ownership.** The Center is a top-right
`notification-center` `overlay` layer surface created by `ShellProtocol` with
its own panel-sized input region and a modal keyboard grab while open. It is
created without an explicit output, so the compositor's per-output overlay rule
anchors it on the output that produced the invoking interaction. The menu-bar
clock (`MenuBar.clockActivated`) and the compositor's `notification-center`
input action (Control-Shift-N and any configured hot corner, including the
default top-right corner) both call
`ShellController::toggleNotificationCenter()`; the view's close path calls
`hideNotificationCenter()`. Opening the Center closes the
Control Center, Applications drawer, Mission Control, chooser, screenshot,
ScreenCast or polkit overlay. Escape, outside focus loss, lock, output removal,
compositor surface closure or a competing overlay close it, clear the input
region and release keyboard focus back to the active window.

**History and actions.** `shell/src/notificationcenter.{h,cpp}` is the
Wayland-free normalized state. It keys every row by the service id, follows
that key across history refreshes, marks a row active only when `reason` and
`closedAt` are unset and `suppressed` is false, and strips the action list from
every history-only row. Up/Down/Home/End traverse, Return invokes an active
row's `default` action or dismisses a plain active row, and Delete/Backspace
dismiss the selection. Pointer action buttons and the row dismiss control route
through the existing `NotificationClient::invoke`/`dismiss` service contract.
The QML view is `shell/notifications/NotificationCenter.qml`; it renders the
stable rows, accessible empty/unavailable states and the active-only action
controls. Expired, closed, dismissed and DND-suppressed rows remain visible
history with no active actions.

**Service behavior.** Banner delivery, expiry, replacement, urgency, Focus/DND
policy and the existing badge semantics stay with `services/notifications`; the
Center is a projection plus the two existing write calls. Absence clears the
stale rows into an accessible "Notifications unavailable" state while the
Center stays mapped, and service re-registration refreshes the same Center.
The mock client now mirrors the service's Focus/DND admission rule so the
headless round-trip covers suppressed history. T205 owns the notification-stack
and lower-panel material calibration; T204 uses the shared Popup role until
then.

**Checks (final tree).**

- `cargo test -p dragonfruit-notifications` — 24 unit + 12 session-bus tests
  passed.
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor hot_corners`
  — 5 passed, including the configured top-right Notification Center corner.
- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance
  synthetic_input_drives_shortcuts_hot_corners_and_gestures` — passed, including
  the configured hot-corner `notification-center` action.
- `ctest --test-dir build -R '^tst_notificationmodel$|^tst_notificationcenter$|^tst_notifications$|^tst_menubar$'`
  — 4/4 passed.
- `make gates` — passed: glass self-test, preset checks, menu-bar veil, menu
  families and Control Center material checks, e2e, and 84/84 ctest
  (including `tst_notificationcenter`, `tst_notificationmodel`,
  `tst_notifications` and `tst_menubar`).

Integration coverage maps to the acceptance rows: `tst_menubar` clicks the
clock and asserts `clockActivated`; the compositor hot-corner unit/conformance
tests assert the configured top-right corner dispatches `notification-center`;
`tst_notificationcenter` drives the mock service through live, DND-suppressed,
expired/dismissed and reconnect states; `tst_notifications` covers populated
and empty/unavailable views plus keyboard traversal, action invocation,
dismissal and Escape; the controller's close reasons cover Escape/outside/lock/
output/competing, with `releaseKeyboardFocus()` restoring the prior window.

Docs: `docs/design/04-shell.md` T204 status,
`docs/private-protocols.md` layer/namespace note, and ADR
`docs/design/adr/0223-notification-center-mapping.md`. Reusable notes and
follow-ups are in `docs/tasks/glass/plan/progress/T204.md`.

No blockers. Remaining work is T205's material calibration and the optional
follow-ups listed in the progress note.