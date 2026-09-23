# T-22 — Global Menu Broker

| | |
|---|---|
| **Phase** | 4 · System integration (broker core); DBusMenu bridge itself is Phase 6 ([T-30](30-compatibility-bridges.md)) |
| **Area** | `services/menu-broker/` |
| **Depends on** | [T-07](07-private-shell-protocols.md) (focus tracking) · [T-09](09-menu-bar.md) (rendering) · [T-08](08-design-system.md) (`MenuBarMenu` component) |
| **Blocks** | [T-16](16-settings-app.md) / [T-18](18-files-app.md) menu publication · Phase-6 exit (DBusMenu bridge) |
| **Estimate** | L |
| **Design docs** | [06-global-menu.md](../design/06-global-menu.md) · [04-shell.md](../design/04-shell.md) |

## Summary

`menu-broker`: the service that resolves which menu model belongs to the
focused window, with a strict three-tier priority order (our native API →
DBusMenu/AppMenu → no exporter), macOS menu interaction conventions,
shortcut **dispatch** (not mere display), and the two hard rules (never
break an app's internal menu; the feature is a toggle).

## Background

A global menu is not a fundamental Wayland capability — an application has
to export a meaningful menu model somehow
([06-global-menu.md](../design/06-global-menu.md)). We build the broker
with a priority order and a hard never-break-apps rule rather than forcing
Unity-era injection modules into every toolkit.

## Scope

### In scope

1. **Priority order** (from
   [06-global-menu.md](../design/06-global-menu.md)):
   ```text
   Active window
        │
        ▼
   menu-broker
        │
   ┌───┴───────────────┬──────────────────┐
   our native API     DBusMenu          no exporter
        │               │                  │
        ▼               ▼                  ▼
   perfect menu    compatible menu    app name only
   ```
   1. **Our native API** — Settings and Files publish declarative menu
      models via the design system's `MenuBarMenu`; essentially perfect
      macOS-like behavior.
   2. **DBusMenu / AppMenu** — third-party exporters get a compatible
      global menu (the bridge itself lands in T-30; this ticket defines
      the broker-side mapping target).
   3. **No exporter** — the menu bar shows just the application name.
2. **Focus tracking**: the broker tracks the focused window via the
   compositor's private protocol and resolves which menu model (if any)
   to display.
3. **Interaction conventions** (macOS): click to open; drag through
   submenus with delayed hover; Escape or focus loss dismisses; an open
   menu **tracks the focused window switching underneath**.
4. **Shortcut dispatch**:
   - First-party menu models carry actionable accelerators executed by
     the app itself.
   - DBusMenu accelerator strings are **parsed by the broker**, which asks
     the compositor to register the keybindings **while the owning window
     is focused** (via T-03's engine).
   - **Conflicts resolve by focus**: the active window's menu wins; system
     shortcuts (workspace switching, Mission Control, app switcher) take
     precedence over application accelerators.
5. **Hard rules**:
   - **Never remove a third-party application's internal menu merely
     because global menu mode is enabled.** If the app exports
     successfully → show globally; otherwise leave its menu alone.
   - **The feature is a toggle.** When off, our first-party applications
     restore their local menu presentation immediately.
6. **Restart behavior**: restartable; while absent the menu bar falls
   back to app-name display ([01-architecture.md](../design/01-architecture.md)).
7. **Known limitation** handled by design: GTK apps frequently don't
   export DBusMenu under Wayland — that's an application-cooperation
   problem; we surface Tier 3 and never fight it.

### Out of scope

- The DBusMenu protocol bridge implementation for third parties
  (T-30/Phase 6).
- Menu rendering (menu bar shell, T-09).
- The `MenuBarMenu` QML component itself (T-08).

## Requirements

- FR-1: Focus→menu resolution latency within one interaction beat; the
  open-menu-tracks-focus-change convention works (switch apps under an
  open menu).
- FR-2: First-party publication: both flagship apps' full menus (per
  [09-files.md](../design/09-files.md) for Files) round-trip through the
  broker with live enable/disable.
- FR-3: Accelerator registration/unregistration follows focus; system
  shortcuts always win (test: register a menu accelerator that collides
  with a system shortcut — system shortcut keeps firing).
- FR-4: The toggle off→on: first-party apps restore local menus
  immediately, and re-enable globally without restart.
- FR-5: Broker death: menu bar degrades to app-name display; restart
  restores (no session impact).
- FR-6: Third-party rule enforcement is testable in the broker's contract
  tests (mock exporters): export-success → shown globally; export-fail →
  untouched.

## Acceptance criteria

- [ ] Tier resolution matrix (native / DBusMenu-mock / none) passes.
- [ ] Settings' Global-menu toggle demonstration (On/Off with both apps).
- [ ] Shortcut-dispatch conflict test passes.
- [ ] Kill/restart of menu-broker during use: graceful fallback + recovery.

## Test plan

- Contract tests with mock exporters of each tier; malformed exporter
  (never crash).
- Focus-switch stress with an open menu.
- Integration with T-09 (rendered) in the nested session.

## Risks / open questions

- Accelerator parsing (DBusMenu strings) has dialect variance; keep the
  parser tolerant + logged.
- Decide the native publication channel: D-Bus (`org.dragonfruit.*`) vs
  Wayland-adjacent — D-Bus fits the "restartable service" model; document
  the interface as versioned/additive.
