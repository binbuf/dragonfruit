# T-25 — Notifications and OSD Services

| | |
|---|---|
| **Phase** | 5 · Desktop infrastructure |
| **Area** | `shell/notifications/` (service) + OSD component |
| **Depends on** | [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-09](09-menu-bar.md) · [T-20](20-system-service-adapters.md) |
| **Blocks** | Daily-driver bar (notifications + OSD) · [T-21](21-control-center.md) Focus/DND · [T-27](27-portal-backend.md) (notification portal) |
| **Estimate** | L |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [01-architecture.md](../design/01-architecture.md) |

## Summary

Independent notification and on-screen-display components per the
modern-desktop pattern: a notification service owning the queue and
**Focus/DND state** (one source of truth for menu bar + Control Center),
and an OSD service for volume/brightness changes, caps lock, and battery
warnings.

## Background

A contemporary desktop ships independent idle, OSD, notifications,
screenshot, portal, and settings-daemon components
([ROADMAP.md](../ROADMAP.md)). Focus/DND lives with the
notification service so Control Center and the menu bar share one source
of truth ([04-shell.md](../design/04-shell.md)). Both are restartable;
transient state may be lost ([01-architecture.md](../design/01-architecture.md)).

## Scope

### In scope — notification service

1. **Spec**: implements the freedesktop `org.freedesktop.Notifications`
   D-Bus API so every existing Linux app's notifications work.
2. **Queue + history**: notification center (accessible from menu bar and
   the notification-center hot corner, T-14), grouped per app, actions
   (inline reply buttons where the caller supplies them), persistence of
   history across service restarts (best-effort).
3. **Focus / DND**: owned here — modes (off / Focus / DND), scheduling
   hooks, per-app overrides (Settings Notifications pane in T-16 binds
   here); menu bar and Control Center both render this state via
   subscription.
4. **Banners**: transient chrome surfaces (design-system motion rules,
   reduced-motion variants), click-through activation, dismiss gestures.
5. **Our own emissions**: battery warnings (UPower adapter), system
   messages.

### In scope — OSD service

1. **Triggers**: volume change (PipeWire adapter), brightness change
   (compositor), caps lock, battery warnings.
2. **Behavior**: brief, non-interactive overlay per design-system motion
   tokens; never steals focus; suppressed while in a fullscreen Space
   except critical warnings (policy decision recorded in-repo).
3. **Feedback rule**: Screenshot and OSD feedback follow the same
   design-system motion rules as the rest of the chrome
   ([04-shell.md](../design/04-shell.md)).

### Out of scope

- Idle/lock policy (T-26) — the notification service is *not* the idle
  service; the compositor owns idle thresholds
  ([11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md)).
- Screenshot feedback UI (T-28).

## Requirements

- FR-1: Spec conformance test: standard notify-send matrix (icons,
  actions, urgency, persistence hints, body markup subset) passes.
- FR-2: Focus/DND single-source test: toggling in Control Center, menu
  bar, and Settings all agree within one event (three-way consistency).
- FR-3: Banners + center + history survive service restart (transient loss
  acceptable only for in-flight banners).
- FR-4: DND suppresses banners but history still records (per-app override
  honored).
- FR-5: OSD: appears within one frame of trigger, auto-dismisses;
  concurrent triggers coalesce (volume + brightness simultaneously).
- FR-6: Idle cost: zero wakeups (no polling; events only).
- FR-7: AT-SPI + keyboard access for the notification center; reduced
  motion respected.

## Acceptance criteria

- [ ] `notify-send` conformance matrix green.
- [ ] Three-way Focus/DND consistency test green (Phase-5 contribution).
- [ ] OSD trigger matrix (volume, brightness, caps lock, battery) green.
- [ ] Restart drill: history preserved, in-flight transient lost by design.

## Test plan

- D-Bus conformance suite; timing tests for OSD appearance/dismissal;
  restart drills; fullscreen-space suppression policy test.

## Risks / open questions

- `org.freedesktop.Notifications` is a de-facto spec with dialect variance
  (GNOME/KDE extensions) — implement the core, tolerate extras, log
  unknown hints.
- Decide notification-action trust boundaries (actions run in the
  caller's context — never execute arbitrary commands ourselves).
