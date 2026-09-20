# T-09 — Menu Bar (Shell Bootstrap)

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `shell/menubar/` (and shell process bootstrap) |
| **Depends on** | [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-20](20-system-service-adapters.md) (status items) · [T-22](22-global-menu-broker.md) (app menu, may stub initially) |
| **Blocks** | [T-10](10-dock.md) · [T-11](11-mission-control-workspace-ux.md) (Mission Control entry point) · [T-14](14-hot-corners-desktop-background.md) · [T-21](21-control-center.md) · [T-25](25-notifications-and-osd.md) · [T-30](30-compatibility-bridges.md) (tray bridge) |
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
  name only when the app doesn't export one (broker priority 3 —
  [06-global-menu.md](../design/06-global-menu.md)).
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

- [x] Full menu bar renders in nested mode with placeholders and live
      status items as adapters land. *(The bar renders in a live headless
      session via the real shell process; the same code path runs nested.
      Status items are placeholders until T-20.)*
- [ ] Kill-and-restart of the shell process: menu bar returns, windows
      untouched (Phase-1 exit test reused). *(Needs a fresh one-time token
      per shell start, T-24; compositor-owned window state is untouched by
      construction.)*
- [ ] Idle trace: zero polling from the menu bar. *(No polling exists — the
      clock is a single minute-aligned one-shot timer — but the scripted
      trace is not written yet.)*
- [x] Menu interaction walkthrough passes (drag-through, dismiss, focus
      switch) on keyboard and pointer. *(`shell/tests/tst_menubar.qml`.)*

## Session status (T-09 split)

Implemented and verified in this session:

- **Menu bar render/interaction core** (`shell/menubar/`): app-menu region
  (`MenuBarMenu` per top-level menu + app-name fallback), unified status-item
  slots with graceful degradation, locale clock, Control Center entry,
  Mission Control button, FR-3 interaction rules, reduced motion via the
  design-system motion tokens. `shell/tests/tst_menubar.qml` (ctest) covers
  layout zones, adapter degradation, the full interaction walkthrough, clock
  locale formatting, and Canvas status-glyph pixels.
- **Shell process bootstrap** (`shell/src/dragonfruit-shell`): libwayland
  client, `df_core` launch-token handshake, `df_shell` menu-bar layer surface
  with an exclusive zone, offscreen QML → `wl_shm` rendering, and
  `df_toplevel_manager` focus tracking for the app name. Verified live
  headless: authenticated, configured 1280×28, output `reserved_zone`
  edge=0 thickness=28, clean teardown.
- **Dev workflow**: `dragonfruit dev --nested --shell` (and `make dev`)
  launches the shell with the provisioned token and owns it in `ChildGuard`.

Hand-off (T-09 continuation):

1. **Menu dropdown overlay surface.** The open menu is currently clipped to
   the 28 px bar surface. Create a second `df_layer_surface` on the
   `overlay` layer sized to the open dropdown (with its own input region and
   keyboard mode), and move `MenuBar`'s popups onto it. Until then the
   QML interaction is correct but only the bar is visible in a live session.
2. **Scripted shell restart (FR-5/FR-9).** The compositor must mint a fresh
   token per shell start (T-24 owns the session manager; for the test, add a
   token re-mint request or a test-only token env), then kill and relaunch
   the shell and assert the bar returns with windows untouched.
3. **Output hotplug re-anchoring + idle trace.** The shell re-renders on
   every `configure`; script the hotplug case and a SIGUSR1-based idle trace
   asserting no client wakeups from the bar.
4. **T-20 status adapters and T-22 menu-broker** replace the placeholders and
   the name-only app menu.

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
- The vertical slice demos Wi-Fi, volume, and battery menus before the
  Phase-4 adapters (T-20) land; plan placeholder-first (see acceptance
  criteria) and pull the first three adapters forward if the demo needs
  live items.
