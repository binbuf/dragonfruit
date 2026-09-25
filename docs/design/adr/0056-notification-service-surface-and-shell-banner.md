# 0056 — Notification service surface and the shell banner

## Status

accepted

## Context

The Track-11 design puts the notification queue and Focus/DND state in one
service and the banners in the shell ([04-shell.md](../04-shell.md),
[11-control-center-notifications.md](../tracks/11-control-center-notifications.md)).
Apps must keep working through the standard freedesktop API, but the shell
needs to read the active banners and the history, and nothing in
`org.freedesktop.Notifications` lists the live queue. The shell never links a
Rust service ([01-architecture.md](../01-architecture.md): D-Bus is the seam),
so a shell-facing interface is required, and the chrome needs a surface to
paint banners on.

## Decision

- **One process, one path, two interfaces.** `services/notifications`
  (`dragonfruit-notifications`) owns `org.freedesktop.Notifications` at
  `/org/freedesktop/Notifications` and adds the shell-facing
  `org.dragonfruit.Notifications1` at the *same* path. Both share one queue
  behind a mutex. The standard interface serves apps (`Notify`,
  `CloseNotification`, `GetCapabilities`, `GetServerInformation`,
  `NotificationClosed`); the shell interface serves flat JSON
  (`Banners()`/`History()`), the shell-driven `Dismiss`/`Expire`,
  `DoNotDisturb`/`SetDoNotDisturb`, and a single `Changed` signal. The shell
  never parses the freedesktop interface.
- **The service owns expiry.** An event-driven background thread sleeps until
  the earliest banner deadline (woken early by a new `Notify`), closes the
  banner, and emits `NotificationClosed` (reason 1) plus `Changed`. The shell
  renders and unmaps on `Changed`; it keeps no timer.
- **The shell paints one banner into a new top-right overlay surface.** A
  `notification`-namespace `df_layer_surface` (overlay layer, exclusive zone
  `-1`, no keyboard, empty input region) is anchored `top|right` with a margin
  below the menu bar. `NotificationClient` (live D-Bus or `DF_NOTIFY_FIXTURE`
  mock) feeds `NotificationModel`, and the shell maps the newest active banner
  there; an empty queue unmaps it. Actions are recorded but not advertised or
  round-tripped (T-11.1b), and one banner shows at a time (T-11.1b adds
  stacking and activation).
- **Do Not Disturb is a service-owned state bit.** It suppresses the banner
  while still recording history; T-11.2a grows the policy and the menu-bar
  reflection around it.

## Consequences

- Existing Linux apps work unchanged; the shell's client contract is a small,
  versioned `org.dragonfruit.Notifications1` rather than the freedesktop
  dialect.
- The shell chrome surface is additive and needs no compositor change; the
  banner is drawn like the Dock/switcher chrome and is captured by the live
  visual check.
- A service whose name is already taken on the bus (a host desktop's own
  daemon) cannot start; the shell degrades to no banner. The real session
  starts this service first.
- Deferred to T-11.1b: action round-trip and the Dock launch-failure notice.