# T-11 — Control Center, Notifications, and OSD

> **Track, not a single slice.** This file is the design reference. It is executed as 8 one-session units: [T-11.1a](units/073-t-11.1a-notification-service-core.md) · [T-11.1b](units/074-t-11.1b-notification-actions-and-dock-badge.md) · [T-11.2a](units/075-t-11.2a-dnd-focus-policy.md) · [T-11.2b](units/076-t-11.2b-dnd-reflection-and-dock-failure.md) · [T-11.3a](units/077-t-11.3a-control-center-panel-and-tiles.md) · [T-11.3b](units/078-t-11.3b-control-center-focus-dark-a11y.md) · [T-11.4a](units/079-t-11.4a-osd-overlay.md) · [T-11.4b](units/080-t-11.4b-osd-a11y-and-captures.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 11 of 17 — the ambient chrome |
| **Area** | `shell/control-center/` · `shell/notifications/` · notification service |
| **Depends on** | T-07, T-08 |
| **Blocks** | T-15 (tiles), T-17 |
| **Legacy detail** | [legacy/21-control-center.md](legacy/21-control-center.md) · [legacy/25-notifications-and-osd.md](legacy/25-notifications-and-osd.md) · [04-shell.md](../design/04-shell.md) · [07-system-integration.md](../design/07-system-integration.md) |

## Demo

```
click the Control Center item → a panel of tiles: Wi-Fi, Bluetooth, volume,
brightness, Focus/DND, dark mode; toggling a tile applies live
→ change volume/brightness → an OSD appears and fades
→ an app sends a notification → a banner appears; click opens the app;
  the notification center lists history; DND suppresses banners
→ launch failure from the Dock now raises a real notification instead of a
  transient badge
```

Capture: `docs/captures/t11-control-center.*`, plus an OSD and DND still.

## Why now

The Control Center item and the Dock's failure notices already exist as
stubs; T-07 gave them live data. This slice makes the ambient layer real,
which is a large share of the "premium" feel and removes the remaining
"logs only" paths in the shell. It is also the consumer that justifies the
T-15 tiles later.

## Inherited and reused

- The Control Center entry point and its click signal (currently logs).
- T-07's A1/A3/A4 adapters.
- The Dock's launch-failure path (currently a transient badge) — replace with
  the notification service.
- Design-system `Popover`, `Toggle`, `SegmentedControl`, `ScrollView`, `Icon`.
- T-04's blur pass for the panel material.

## Scope

### In

1. **Notification service**: a session D-Bus service implementing
   `org.freedesktop.Notifications`; banners, history, actions, DND/Focus;
   replace the Dock's badge-only failure path.
2. **Control Center panel**: tiles for Wi-Fi, Bluetooth (state from T-07/T-15),
   volume, brightness (compositor-owned), Focus/DND, dark mode; live apply.
3. **OSD**: volume and brightness changes show a brief overlay on the active
   output; respects reduced motion and fullscreen.
4. **Do Not Disturb / Focus**: suppresses banners, keeps history; the menu bar
   item reflects state.
5. **Keyboard**: Control Center on a shortcut, Escape/click-away dismissal,
   accessible roles.

### Out / explicitly deferred

- The remaining adapter tiles (Bluetooth device list, storage, etc.) — T-15.
- Notification grouping/rich media beyond the MVP.
- Notification center sync across devices.

## Acceptance

- [ ] The demo runs and the captures are committed.
- [ ] A third-party app's `Notify` call produces a banner, history entry, and
      action round-trip.
- [ ] DND suppresses banners but keeps history; the menu-bar item reflects it.
- [ ] OSD appears on the active output and respects fullscreen/reduced motion.
- [ ] Dock launch failure raises a real notification.
- [ ] `make e2e` and `make soak` stay green; previous demos still pass.

## Test plan

- Unit: notification model and DND policy.
- Integration: D-Bus `Notify`/`CloseNotification`/`GetCapabilities` with a
  test client; OSD state machine.
- Nested: capture review; fullscreen and reduced-motion variants.
- Regression: T-07 status items.

## Risks

- **Notification policy** (timeouts, grouping, actions) is easy to overbuild;
  ship the MVP and keep the model additive.
- **Focus/DND semantics** must match the menu-bar item and Settings later.

## Hand-off

- T-15 fills the remaining tiles from the full adapter roster.
- T-17 checks the OSD/banner in the loop's reduced-motion pass.
