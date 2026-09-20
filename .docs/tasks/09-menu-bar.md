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
      status items as adapters land. *(Verified by a live nested capture:
      the bar renders at the top, correctly oriented, with the app-name
      fallback and locale clock. Status items are placeholders until
      T-20.)*
- [x] Kill-and-restart of the shell process: menu bar returns, windows
      untouched (Phase-1 exit test reused). *(`shell_restart_reanchors_chrome_
      and_preserves_windows` in `compositor/tests/shell_protocol_conformance.rs`:
      an independent client's window survives a shell crash; the reserved zone
      clears and returns when a second shell authenticates with a fresh token
      from the up-front `DRAGONFRUIT_LAUNCH_TOKENS` set.)*
- [x] Idle trace: zero polling from the menu bar. *(`shell_idle_trace.rs`
      attaches a mapped menu-bar chrome surface, lets it go idle, and asserts
      `frames_rendered` stays flat across a second. The shell-side half is that
      the clock is a single minute-aligned one-shot timer.)*
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
- **Compositor chrome rendering**: `DfState::chrome_surfaces` +
  `render::chrome_render_elements` composite the shell's layer surfaces
  above the window space in the nested and DRM backends (previously they
  were placed but never drawn, so the bar was invisible). The nested
  backend's pre-existing vertical-flip bug (winit/EGL needs
  `Transform::Flipped180`) is fixed. A live nested capture confirms the bar
  renders at the top, correctly oriented.
- **Status-icon style pass**: original geometry polished to read as a
  macOS-style family (uniform thin optical stroke, Wi-Fi arcs + dot,
  Bluetooth rune, outline/fill/nub battery, toggle-pill Control Center),
  plus the locale short date in the clock. The real macOS screenshots in
  `.docs/reference/macos/` are a **style reference only** — per
  [14-risks.md](../design/14-risks.md) we do not copy Apple's SF Symbols.
- **Dev workflow**: `dragonfruit dev --nested --shell` (and `make dev`)
  launches the shell with the provisioned token and owns it in `ChildGuard`.

Second session (T-09 continuation):

- **Scripted shell restart** (`shell_restart_reanchors_chrome_and_preserves_
  windows` in `compositor/tests/shell_protocol_conformance.rs`): a window
  mapped by an independent client is announced to an observer manager; shell
  #1 reserves the menu-bar zone; dropping the shell connection clears the
  reserved zone while the window survives (never closed, same title); shell #2
  authenticates with a *second* up-front token and the bar returns at
  1280×28. `DRAGONFRUIT_LAUNCH_TOKENS` provisions both tokens, standing in
  for T-24's per-start mint.
- **Scripted idle trace** (`compositor/tests/shell_idle_trace.rs`, in
  `make e2e`): a mapped menu-bar chrome surface commits one frame, then
  idles; `frames_rendered` is flat across a second and `direct_scanouts` never
  advances. This is the compositor-side half of the FR-6 budget.

Third session (T-09 continuation — interactive chrome):

- **Compositor input routing to chrome surfaces.** `input::chrome_under`
  hit-tests `df_layer_surface`s above the window space, topmost layer first,
  using `under_from_surface_tree`; `input::surface_under` now checks chrome
  before windows and returns the surface origin in **global** space (it
  previously returned it window-relative, which mis-placed pointer/touch
  coordinates for any window not at the output origin). Click-to-focus routes
  to a chrome surface only when its `KeyboardInteraction` is `OnDemand` or
  `Exclusive`; `Exclusive` takes focus on `set_keyboard_interaction`; focus
  returns to the active window when the chrome surface closes or the shell
  crashes. `focus_changed` preserves `active_window` while chrome holds the
  keyboard so the window can be re-focused. New headless conformance test
  `chrome_surface_receives_pointer_and_keyboard` proves pointer enter/motion/
  button and keyboard focus/key delivery to a mapped bar.
- **Shell input bridge + dropdown.** `ShellProtocol` binds `wl_seat` and
  owns a `wl_pointer`/`wl_keyboard`, emitting `pointerMoved`/`pointerButton`/
  `keyEvent`/`keyboardFocused`; `ShellController` synthesizes Qt mouse/key
  events into the offscreen `QQuickWindow` so the existing QML
  hover/tap/key handlers drive the bar unchanged. When a menu opens,
  `MenuBar.dropdownBottom` reports the popup's extent and the shell grows the
  chrome surface (`df_layer_surface.set_size`, exclusive zone unchanged) so
  the compositor reveals the dropdown below the 28 px bar; Escape/click-away/
  activation shrink it back. A new `tst_menubar` case clicks a menu title and
  asserts it opens and stays open (the bar's click-away handler used to fire
  on the same tap). Verified live nested: the File menu renders below the bar
  with its rows and shortcut labels, and the shell logs the 1280×28 →
  1280×124 → 1280×28 configure round-trip.
- **HITL fixes (first human nested session).** Nested pointer input was dead:
  `PointerMotionAbsolute` treated winit's window-pixel `CursorMoved` as
  device-normalized, so every location was off-output and the shell never got
  pointer events; it now uses `event.position_transformed(size)`. The first
  menu title also drew its `FocusRing` on launch (the offscreen window
  auto-focused the first `activeFocusOnTab` item); the shell clears the
  active focus on the first event-loop turn.

Hand-off (T-09 continuation):

1. **True overlay layer for the dropdown (optional polish).** The dropdown
   currently rides the `top` menu-bar surface, grown while a menu is open
   (see PROGRESS.md for the rationale and trade-offs). The design doc places
   menus on the `overlay` layer; a separate `df_layer_surface` sized to the
   popup would need a second `QQuickWindow`/item for the dropdown content
   (the design-system `Popup` is a child of the bar item). Not required for
   correctness today because the bar is the only `top` surface.
2. **Output hotplug re-anchoring.** The compositor already reconfigures every
   chrome surface in `on_output_added`/`on_output_removed`, and the shell
   re-renders on every `configure`; the remaining work is a scripted hotplug
   case. That needs a runtime "add an output" hook on the headless backend
   (there is only one static output today).
3. **T-20 status adapters and T-22 menu-broker** replace the placeholders and
   the name-only app menu. The shell's `--placeholders` demo app menu
   (`ShellController::demoAppMenu`) is a T-22 stand-in so the dropdown is
   exercisable now; T-22 should delete it.

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
