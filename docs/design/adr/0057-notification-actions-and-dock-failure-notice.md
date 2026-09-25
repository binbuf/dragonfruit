# 0057 — Notification actions round-trip and the Dock failure notice

## Status

accepted

## Context

T-11.1a shipped the notification queue, the shell-facing JSON views, and the
top-right banner surface, but actions were recorded and not advertised, and a
banner was display-only ([0056](0056-notification-service-surface-and-shell-banner.md)).
Two gaps remain in the T-11 track: an app's inline notification actions never
reach the app, and the Dock still replaces a failed launch with a transient
tile badge (T-10 section 8.5) that leaves no durable record
([11-control-center-notifications.md](../tracks/11-control-center-notifications.md)).

The freedesktop spec has the *server* emit `ActionInvoked(id, action_key)` when
the user invokes an action. The shell is the only component that knows a click
happened, and it is a D-Bus client of the service, not the service itself.

## Decision

- **The shell-facing interface gains `Invoke(id, action_key)`.** The shell
  calls it when the user activates an action; the service emits the
  freedesktop `ActionInvoked` to the originating app and then dismisses the
  banner (reason 2), emitting `NotificationClosed` and `Changed`. The service
  now advertises the `actions` capability. The shell never talks to the app
  directly; the service remains the sole bus owner.
- **The banner surface becomes interactive.** The `notification` layer surface
  keeps `KeyboardInteraction::None`, but the shell sets an input region sized
  to the card (not the full surface), so the transparent strip below a
  short banner still passes clicks through. The shell forwards banner-surface
  pointer events into the offscreen banner scene, exactly as it does for the
  Dock. The card renders an inline action row when the app supplied actions
  and grows to fit it (96 px without actions, 132 px with); the surface is
  created at the larger height. A body click invokes the app's `default`
  action when present, otherwise dismisses.
- **The Dock launch failure is a real notification.** The shell's
  `NotificationClient` gains `notify(...)` (freedesktop `Notify`) and
  `invoke(...)`; `failDockLaunch` and the launch-timeout path call
  `raiseDockLaunchFailure`, which sends `Dock` / "Could not launch <app>" /
  `<reason>`. The transient tile mark stays as immediate feedback, but the
  durable record is the banner and history entry.

## Consequences

- Apps that register actions work unchanged, and the shell's own Dock-failure
  notice is an ordinary notification, so it lands in the notification center.
- `org.dragonfruit.Notifications1` is now a write surface too; the lockstep
  shell/protocol contract is unchanged (a method on the existing interface,
  no new version).
- One banner still shows at a time; stacking and the notification-center entry
  point remain T-11.2b/T-11.3a. DND suppression is unaffected (T-11.2a owns
  policy).
- `DF_DOCK_FAIL_FIXTURE` is a capture/demo seam that raises the failure once
  at startup; it is never set in a normal session.