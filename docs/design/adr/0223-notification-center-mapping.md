# 0223 — Notification Center mapping and action ownership

## Status

accepted

## Context

The notification service already owns the queue, history, expiry, Focus/DND
and banner policy ([0056](0056-notification-service-surface-and-shell-banner.md),
[0058](0058-focus-dnd-policy-semantics.md)). `NotificationCenter.qml` rendered a
history list in tests but had no shell surface, and the clock, keyboard action
and hot-corner action all existed without a common route. The Center also has
to distinguish a still-open notification from history that merely remains in
the bounded list; invoking an action on an expired or DND-suppressed row would
be a stale-action bug.

## Decision

- **One trusted overlay, one entry route.** The Center is a top-right
  `notification-center` `overlay` layer surface with its own panel-sized input
  region and OnDemand keyboard interaction, exclusive while open. The menu-bar
  clock, the compositor's `notification-center` action and any configured hot
  corner all call the same shell toggle. The surface has no explicit output;
  the compositor's existing per-output overlay rule anchors it on the invoking
  output, and the shell never creates a hidden second layer.
- **One normalized history controller.** `shell/src/notificationcenter.{h,cpp}`
  owns the decoded history rows, the selection and the action validation. A row
  is active only when the service has not set `reason`/`closedAt` and Focus/DND
  did not suppress it. Every row keeps a stable key (the service id); inactive
  rows keep their display text and status but expose no actions. Selection is
  keyed by that identity, not by list index.
- **The service keeps the notification semantics.** Banner delivery, expiry,
  replacement, urgency, Focus policy and badge state are unchanged. The Center
  only calls the existing `Invoke`/`Dismiss` methods. Service absence clears the
  stale view into an accessible unavailable state; the existing
  `availableChanged`/`History()` refresh repopulates it on reconnect.
- **Close is explicit.** Escape, outside focus loss, lock, output teardown and
  a competing ambient overlay close the Center, clear its input region and
  release keyboard focus back to the window that had it. T205 owns the visual
  material calibration; this decision fixes routing and ownership.

## Consequences

- The Center can never invoke an action that is no longer active, even if a
  malformed or late history payload carries one.
- The shell has no second notification store: the service remains the single
  source of truth and the QML view is a projection of the normalized rows.
- A service restart is observable but non-destructive: the open Center shows
  the unavailable empty state until the reconnect refresh arrives.
- Future notification-center material work (T205) can declare regions on the
  same surface without changing entry routing or action ownership.
