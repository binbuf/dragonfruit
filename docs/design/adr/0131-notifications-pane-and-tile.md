# 0131 — The Notifications and Focus panes and tile

## Status

accepted

## Context

T-15.7a landed the Notifications and Focus projection adapter
(`dragonfruit-notify-adapter`) over the existing notification service, and left
two seams for this task: a bridge-host view and the four global presentation
preferences the macOS reference shows have no owner. The reference
(`docs/reference/macos/SystemSettings_Notifiations.md`, ADR
[0122](0122-tahoe-interface-language-across-chrome.md)) shows a `Notifications`
pane (header card, a `Notification Center` group, and per-app rows) and a
sibling `Focus` pane that shares the same service state. The Control Center
already carries a Focus/DND tile from T-11.3b.

The decision needed: where the global preferences live, and how the pane, the
Focus pane, and the tile stay one functional unit without reimplementing the
queue, the history, or the Focus admission rule (ADR
[0058](0058-focus-dnd-policy-semantics.md), [0130](0130-notifications-focus-adapter.md)).

## Decision

- **The Focus mode and the per-app allow list ride the adapter, not settingsd.**
  The bridge host grows `services/system-status/src/notifications.rs`
  (`NotificationsHost` + `notifications_view`) and the
  `org.dragonfruit.SystemStatus1.Notifications` interface with `State()`,
  `Refresh()`, `SetFocusMode(mode)`, and `SetFocusAllowList(apps)`, mirroring
  the Bluetooth/storage/battery halves. The Settings app binds it through
  `NotificationsClient` (`DF_NOTIFICATIONS_FIXTURE` for headless tests) and the
  `Settings` singleton's `notifications`/`setFocusMode`/`setFocusApp`. A write
  invents no snapshot: the service pushes `Changed` and the host re-reads.
- **The four global presentation preferences are settingsd keys** (schema
  revision 14): `notifications.showPreviews` (text:
  `always`/`when-unlocked`/`never`), `notifications.showWhenSleeping`,
  `notifications.showWhenLocked`, `notifications.showWhenMirroring` (bools).
  They are desktop preferences, not notification-service policy, so they belong
  with the other settingsd keys; the pane writes them through `Settings.set`.
- **Two panes ship together.** `NotificationsPane.qml` owns the header-card
  content (the shell's `PaneHeader` draws it), the `Notification Center` group,
  and the read-only per-app inventory; `FocusPane.qml` owns the mode selector
  and the per-app allow list. Both bind the same adapter view, so either pane
  reflects the other's writes and the Control Center Focus tile. The per-app
  allow list is a whole-list replace in the service, so `setFocusApp` reads the
  current list, changes one entry, and writes it back.
- **The tile is the existing T-11.3b Focus tile.** It already reflects the
  notification service's mode and suppressed batch; since the panes write the
  same service, no second tile is added.
- **Absence is layered and normal.** Host absent or a foreign notification
  daemon hides the inventory/Focus controls with a one-line note; the four
  settingsd preferences stay live on the schema defaults. The per-app rows are
  an inventory, not a half-built disclosure sheet: the macOS per-app detail
  has no Linux provider, so the rows carry no chevron.

## Consequences

- The four preferences are **stored policy**: they round-trip through
  settingsd and the pane reflects them, but the notification service does not
  yet consult them to gate a banner. That enforcement is a follow-up recorded
  in `docs/PROGRESS.md`; the rows are still the capture's controls and are not
  dead (they have an owner and a stored effect).
- The interface table, the settings-keys doc, and the absent-daemon matrix now
  include the Notifications/Focus surface (`tst_settings_notifications.qml`,
  the new cases in `tst_settings_absence.qml`).
- A future per-app notification detail sheet would extend
  `NotificationsSnapshot`/`AppNotifications` in the service and the adapter,
  not in the pane.