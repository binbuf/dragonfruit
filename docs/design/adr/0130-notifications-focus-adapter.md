# 0130 — The Notifications and Focus adapter projects the notification service

- **Status:** Accepted (T-15.7a)
- **Date:** 2026-09-28
- **Context:** [07-system-integration.md](../07-system-integration.md),
  [15-system-services-breadth.md](../tracks/15-system-services-breadth.md),
  ADR [0056](0056-notification-service-surface-and-shell-banner.md),
  ADR [0058](0058-focus-dnd-policy-semantics.md),
  ADR [0024](0024-system-adapter-contract.md)

## Context

T-11.1a/T-11.2a shipped the notification service (`services/notifications`):
it owns the queue, the bounded history, and the Focus/DND policy, and it
serves the standard `org.freedesktop.Notifications` interface plus a
shell-facing `org.dragonfruit.Notifications1` interface
(`FocusPolicy()`, `Banners()`, `History()`, `SetFocusMode`,
`SetFocusAllowList`, `Changed`) at `/org/freedesktop/Notifications`. The shell
already reads that surface directly for banners and the Focus menu/Control
Center tile.

T-15.7 asks for Notifications and Focus breadth: an adapter (T-15.7a), then the
Settings pane and Control Center tile (T-15.7b). Unlike the other T-15
adapters, there is no host daemon to wrap — the service is ours. The design
risk is duplication: a naive adapter could re-own the queue, the history, or
the Focus admission rule and drift from the one source of truth
(ADR [0058]).

## Decision

Add `services/notify-adapter` (`dragonfruit-notify-adapter`) as a **projection
adapter** over the notification service, not a second service.

1. **Reuse the service's vocabulary and views.** The crate depends on
   `dragonfruit-notifications` for `FocusMode` (and `Urgency`), and its live
   source reads the service's own shell-facing JSON views over the session bus.
   It owns no queue, no history, and no admission rule.
2. **One read, three-way presence.** `NotificationsSource::read` returns
   `Ok(Some(NotificationsData))` when the service answered, `Ok(None)` when it
   is absent (no session bus, no owner, or a daemon that does not serve our
   shell interface), and `Err` when the service owns its name but is
   unreadable or returns malformed JSON. `Ok(None)` is a normal hidden state,
   never an error, and never a startup blocker (ADR [0024]).
3. **Two explicit Focus writes, snapshot-neutral.** `set_focus_mode` and
   `set_focus_allow_list` call the service once; a successful write invents no
   snapshot and the host re-reads after the service's `Changed`. An absent
   write resyncs the adapter to `Unavailable`.
4. **A typed snapshot and a pure change stream.** `NotificationsSnapshot`
   carries `FocusSnapshot` (mode, allow list, suppressed batch), the active
   banners, the history, and a derived per-app list (`AppNotifications`);
   `NotificationsSnapshot::changes` is the event diff (mode, allow list, batch,
   banner and history counts) the adapter queues in `drain_changes`.
5. **New slot ids.** `AdapterId::NOTIFICATIONS` (`"notifications"`) and
   `AdapterId::FOCUS` (`"focus"`) join the contract; the adapter reports
   `NOTIFICATIONS`.

Rejected: reimplementing policy/queue in a Rust adapter; a combined service
(no second daemon); routing the shell's existing banner path through a new
adapter (the shell already reads the service and T-15.7b does not change it).

## Consequences

- T-15.7b binds the Settings pane and Control Center tile to this adapter's
  view (likely through a `dragonfruit-system-status` host interface, mirroring
  Bluetooth/storage/battery), and adds the global notification preferences
  (`Show previews`, sleeping/locked/mirroring) as settingsd keys — those have
  no owner in the service today and are deliberately not adapter state.
- The service stays the only writer of the queue and the policy. If a richer
  per-app policy is later needed, it grows on the service and the adapter's
  read view; the adapter never grows its own policy.
- `OwnedValue`/zbus never leak above the crate; the mock drives all states in
  CI, and the live source is proven over a private `dbus-daemon`.