# T-09 — Menu Bar (Shell Bootstrap)

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `shell/menubar/` (and shell process bootstrap) |
| **Depends on** | [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-20](20-system-service-adapters.md) (status items) · [T-22](22-global-menu-broker.md) (app menu, may stub initially) |
| **Blocks** | [T-10](10-dock.md) · [T-21](21-control-center.md) · [T-25](25-notifications-and-osd.md) |
| **Estimate** | L |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [06-global-menu.md](../design/06-global-menu.md) |

## Summary

The shell process foundation and the top menu bar: anchored chrome surface
with reserved zones, hosting from left to right the active application's
menu (menu-broker), system status items (Wi-Fi, Bluetooth, volume, battery,
clock, Focus/DND, accessibility), the Control Center entry point, and a
Mission Control button/gesture target. This ticket also bootstraps the
shell process itself (Wayland client of our compositor, systemd-restartable,
independent animation curves).

## Background

Menu bar, Dock, Control Center, banners, and OSD are Wayland surfaces the
shell creates through the private shell protocol; chrome renders
independently of client content — during a workspace switch, client
surfaces shrink or slide while the chrome follows its own animation curves
([04-shell.md](../design/04-shell.md)). The shell is crashable and
restartable without taking down the compositor.

## Scope

### In scope

1. **Shell process bootstrap**:
   - Connects to the compositor over the private protocol using its launch
     token (T-07).
   - Creates the menu-bar chrome surface (top edge, reserved zone, layer,
     exclusive keyboard mode off except for open menus).
   - Restart-safe: state re-syncs from compositor broadcasts on restart;
     windows unaffected (T-04 FR-9).
   - The nested dev workflow runs the full shell
     (`dragonfruit dev --nested`).
2. **Menu bar layout** (left → right, per
   [04-shell.md](../design/04-shell.md)):
   - Active application's menu via **menu-broker** (T-22; until it lands,
     render the app name from focus broadcasts).
   - System status items: **Wi-Fi, Bluetooth, volume, battery, clock,
     Focus/DND, accessibility** — consuming the system-service adapters
     (T-20). Each degrades to hidden/disabled when its daemon is absent
     (graceful-degradation principle).
   - **Control Center entry point** (opens T-21's panel).
   - **Mission Control button/gesture target** (drives T-11).
3. **App menu rendering**: macOS-style menu with click-to-open, drag-through
   submenus with delayed hover, Escape/focus-loss dismissal, open menu
   tracks focused-window switches underneath
   ([06-global-menu.md](../design/06-global-menu.md) interaction rules).
4. **Clock** with locale/region formatting.
5. **Status item slots**: sizing, hover, dark/light treatment unified so
   first-party and (later, T-30) third-party StatusNotifier items render
   identically.
6. **Reserved-zone bookkeeping**: menu bar reserves its zone via the chrome
  protocol so Zoom fills "Space minus menu bar and Dock" (T-04) correctly,
   including auto-hide Dock interplay.

### Out of scope

- Dock (T-10), Control Center panel contents (T-21), notifications (T-25).
- Menu *models and broker logic* (T-22) — this ticket renders what the
  broker resolves.
- Third-party tray bridge (T-30).

## Requirements

- FR-1: Menu bar anchors on every output, reserves its zone, and follows
  output hotplug (appears on new outputs).
- FR-2: App menu shows: broker-resolved menu when available; application
  name only when the app doesn't export one (Tier 3)
  ([06-global-menu.md](../design/06-global-menu.md)).
- FR-3: Menu interaction: click-to-open, drag-through submenus with delayed
  hover, Escape and focus-loss dismissal, live switch when focus changes
  under an open menu.
- FR-4: Status items reflect adapter state within one adapter event; absent
  daemon → item hidden or "unavailable," never an error, never blocks
  session start.
- FR-5: Clock formats per locale; menu bar survives restart in place.
- FR-6: Idle menu bar contributes **zero wakeups** to the idle-desktop
  budget (Phase perf budget — no polling; everything is event-driven).
- FR-7: Reduced-motion variant for menu open/close per design-system rules.

## Acceptance criteria

- [ ] Full menu bar renders in nested mode with placeholders and live
      status items as adapters land.
- [ ] Kill-and-restart of the shell process: menu bar returns, windows
      untouched (Phase-1 exit test reused).
- [ ] Idle trace: zero polling from the menu bar.
- [ ] Menu interaction walkthrough passes (drag-through, dismiss, focus
      switch) on keyboard and pointer.

## Test plan

- Nested-session UI tests for layout, hotplug, restart.
- Adapter-absent matrix: mask each systemd unit in a VM (Phase-4 exit
  criterion pattern) and verify per-item degradation.
- Accessibility: menu bar keyboard navigation + AT-SPI roles.

## Risks / open questions

- Clock/status item density on small displays — spec a collapsed-overflow
  rule now, implement later.
- Menu drag-through timing constants live in design-system motion tokens;
  tune there, not here.
