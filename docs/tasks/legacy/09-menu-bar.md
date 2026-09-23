# T-09 — Menu Bar (Shell Bootstrap)

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `shell/menubar/` (and shell process bootstrap) |
| **Depends on** | [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-20](20-system-service-adapters.md) (status items) · [T-22](22-global-menu-broker.md) (app menu, may stub initially) |
| **Blocks** | [T-10](10-dock.md) · [T-11](11-mission-control-workspace-ux.md) (Mission Control entry point) · [T-14](14-hot-corners-desktop-background.md) · [T-21](21-control-center.md) · [T-25](25-notifications-and-osd.md) · [T-30](30-compatibility-bridges.md) (tray bridge) · [T-34](34-mvp-vertical-slice-gate.md) |
| **Estimate** | L |
| **Design docs** | [04-shell.md](../design/04-shell.md) · [06-global-menu.md](../design/06-global-menu.md) |

## Summary

The shell process foundation and the top menu bar: anchored chrome surface
with reserved zones, hosting from left to right the always-present system menu
(the dragonfruit mark), the always-present application menu (the focused app's
name and its About/Settings/Hide/Quit items; Files on the desktop), the active
application's exported menus (menu-broker), system status items (Wi-Fi,
Bluetooth, volume, battery, clock, Focus/DND, accessibility), the Control
Center entry point, and a Mission Control button/gesture target. This ticket
also bootstraps the shell process itself (Wayland client of our compositor,
systemd-restartable, independent animation curves).

## Background

Menu bar, Dock, Control Center, banners, and OSD are Wayland surfaces the
shell creates through the private shell protocol; chrome renders
independently of client content — during a workspace switch, client
surfaces shrink or slide while the chrome follows its own animation curves
([04-shell.md](../design/04-shell.md)). The shell is crashable and
restartable without taking down the compositor.

## MVP slice (for T-34)

The gate needs the live status items from the **T-20 MVP slice** (Wi-Fi,
volume, battery; clock already done) and the fixed system + application
menus. The focused-app exported menu (T-22) may remain a stub until after the
gate — the app name fallback already exists. The Control Center entry point
may open a stub panel (T-21 is post-gate). Nothing else in this ticket may
block T-34.

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
   - **System menu** (dragonfruit mark, always present): About This System,
     System Settings, App Store, Sleep, Restart, Shut Down, Lock Screen,
     Log Out <user>.
   - **Application menu** (always present, bold app name; Files on the empty
     desktop): About <App>, Settings, Hide <App>, Hide Others, Show All,
     Quit <App>.
   - The active application's exported menus via **menu-broker** (T-22;
     until it lands, only the fixed system + application menus render).
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
- FR-2: Menu bar always shows the **system menu** (dragonfruit mark) and the
  **application menu** (focused app's name, or Files on the empty desktop),
  then the broker-resolved menu when available. An app that exports nothing
  still gets both fixed menus (broker priority 3 —
  [06-global-menu.md](../design/06-global-menu.md)).
- FR-3: Menu interaction: click-to-open, drag-through submenus with delayed
  hover, Escape and focus-loss dismissal, live switch when focus changes
  under an open menu. Drag-through spans the fixed menus and the app's
  exported menus as one row.
- FR-4: Status items reflect adapter state within one adapter event; absent
  daemon → item hidden or "unavailable," never an error, never blocks
  session start.
- FR-5: Clock formats per locale; menu bar survives restart in place.
- FR-6: Idle menu bar contributes **zero wakeups** to the idle-desktop
  budget (Phase perf budget — no polling; everything is event-driven).
- FR-7: Reduced-motion variant for menu open/close per design-system rules.
- FR-8: Fixed-menu items carry an `action` that the shell dispatches (Quit
  works today; Settings/About/Sleep/Restart/Shut Down/Log Out/Lock Screen/
  App Store are logged stubs until their owning tickets land — see hand-off).

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
- [x] Always-present system menu (dragonfruit mark) and application menu
      (focused app, Files on the desktop) render before the app's exported
      menus; fixed-menu items dispatch their `action`.
      *(`shell/tests/tst_menubar.qml`:
      `test_system_and_application_menu_always_present`,
      `test_app_menu_renders_after_fixed_menus`,
      `test_system_menu_item_carries_action`, `test_logo_renders_pixels_and_tints`.)*

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
  `docs/reference/macos/` are a **style reference only** — per
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

Fourth session (T-09 continuation — output hotplug, FR-1):

- **Chrome surfaces target every output by default.** `df_shell.get_layer_surface`
  no longer resolves a `None` output to the primary; `None` now means *all*
  outputs (matching `wlr-layer-shell`), so the bar anchors on any display
  attached after the shell starts. `LayerSurfaceState::matches_output` is the
  single predicate `chrome_surfaces` filters on, and a unit test pins it.
- **Headless synthetic-output harness** (`compositor/src/backend/
  synthetic_output.rs`, `DRAGONFRUIT_SYNTHETIC_OUTPUT`): a `UnixDatagram` line
  protocol (`add <name> <w> <h> <x> <y> [scale]`, `remove <name>`) drives the
  same `on_output_added`/`on_output_removed` path as DRM hotplug. Opt-in,
  headless-only test plumbing, socket removed at teardown.
- **Scripted hotplug test** `shell_output_hotplug_reanchors_chrome` (in
  `shell_protocol_conformance.rs`): a shell creates the bar on all outputs; a
  second output is attached; the manager announces it with its geometry, three
  fresh Spaces, the bar's reserved zone, and a chrome reconfigure; detaching
  removes the Spaces. Closes the last host-closable T-09 FR-1 item.

Fifth session (T-09 continuation — true overlay dropdown):

- **The dropdown is a real `overlay` chrome surface** (`menubar-popup`),
  placed by top/left margins at the open menu's rectangle, `exclusive_zone
  = -1`, `keyboard = none`. The bar stays a constant 28 px `top` surface
  (it no longer grows); `ShellController` splits one offscreen render into
  the bar strip and the popup rectangle and commits them to their two
  surfaces, unmapping the overlay after the close animation. Pointer
  coordinates from the popup surface are translated into window coordinates
  so the existing QML interaction logic is unchanged. `MenuBar` exposes
  `dropdownX/Y/Width/Height`; `tst_menubar` pins them.
- **Scripted compositor test**
  `overlay_popup_sits_above_the_bar_and_reserves_nothing`: a top bar and an
  overlay popup are mapped on the same output; the popup configures to its
  requested rectangle, contributes no reserved zone, and a synthetic pointer
  over it (below the bar) is hit-tested in popup-local coordinates.
- Verified: `make lint`, `make e2e` (incl. the new test), and a clean
  headless `dragonfruit dev --headless --shell` startup. The live nested
  open/close walkthrough was **not** re-run this session (no input automation
  in the sandbox); the compositor half and the QML geometry contract are
  scripted.

Sixth session (T-09 continuation — system + application menus):

- **Always-present system menu.** `MenuBarMenu` gained `showLogo` and renders
  the new `DragonfruitLogo` (the brand mark from `dragonfruit.svg`, reduced to
  a tintable `Shape`/`PathSvg` path — no bitmap, no baked-in fill, so it reads
  in light and dark chrome). `ShellController::systemMenu` builds the fixed
  items (About This System, System Settings, App Store [disabled], Sleep,
  Restart, Shut Down, Lock Screen, Log Out <user>) once per session; the user
  name comes from `$USER`.
- **Always-present application menu.** `MenuBar.qml` now renders one combined
  top-level row: system menu (index 0), application menu (index 1), then the
  app's exported menus. The application menu is the focused app's name (bold),
  or **Files** on the empty desktop (the Finder-owns-the-desktop model,
  `design/09-files.md`), with the standard About / Settings / Hide / Hide
  Others / Show All / Quit items. The old "app name only" fallback text is
  gone: the name is now the application menu itself, so a real broker menu no
  longer makes it disappear.
- **Action dispatch.** Fixed-menu items carry an `action`; `MenuBar` passes it
  through `appMenuTriggered` and `ShellController::onAppMenuTriggered`
  dispatches it. `quit` closes the focused app; the rest are logged stubs
  until T-16/T-24/T-26/T-04 land (see hand-off 5).
- **Tests.** `tst_menubar` updated for the two fixed menus (indices shift by
  two) and extended with always-present/logo/action-routing cases; all 21 pass
  headless. `make lint`-equivalent gates (`qmllint`, design-token,
  token-freshness, desktop-name, capture-grab) and the gallery visual
  regression pass.

Hand-off (T-09 continuation):

1. ~~**True overlay layer for the dropdown (optional polish).**~~ **Done
   (fifth session).** The dropdown is now a separate `df_layer_surface` on the
   `overlay` layer (`namespace "menubar-popup"`), placed at the open menu's
   window-space rectangle via top/left margins, with `exclusive_zone = -1`
   (it never enlarges the reserved zone) and `keyboard = none` (the bar
   surface keeps Escape/menu navigation). The bar surface is a constant 28 px
   again — it no longer grows. `ShellController` renders the one offscreen
   QML window, commits the bar strip to the `top` surface and the dropdown
   rectangle to the `overlay` surface, and unmaps the overlay on close after
   the animation. Pointer coordinates delivered relative to the popup are
   translated back into window coordinates. `MenuBar` exposes the dropdown
   rectangle (`dropdownX/Y/Width/Height`); `tst_menubar` pins it.
   `overlay_popup_sits_above_the_bar_and_reserves_nothing`
   (`shell_protocol_conformance.rs`) proves the compositor places the overlay
   popup, that it reserves nothing, and that a pointer over it is hit-tested
   in popup-local coordinates.
2. ~~**Output hotplug re-anchoring.**~~ **Done (fourth session).** A chrome
   surface created without an explicit output now targets *every* output
   (`wlr-layer-shell` semantics), so the bar anchors on a display attached
   after the shell started. The headless backend gained an opt-in synthetic
   output harness (`DRAGONFRUIT_SYNTHETIC_OUTPUT`) to script the hotplug;
   `shell_output_hotplug_reanchors_chrome` attaches and detaches a second
   output through it and asserts the new `df_output`, its three Spaces, its
   geometry, the bar's reserved zone on it, and the chrome reconfigure (see
   PROGRESS.md).
3. **T-20 status adapters and T-22 menu-broker** replace the placeholders and
   the app-exported menus. The shell's `--placeholders` demo app menu
   (`ShellController::demoAppMenu`) is a T-22 stand-in so the dropdown is
   exercisable now; T-22 should delete it.
4. **T-22 owns the application menu.** Today the shell synthesizes the fixed
   application menu (About/Settings/Hide/Hide Others/Show All/Quit) from the
   app name in `ShellController::applicationMenu`. T-22 should move that
   synthesis into the broker so per-app enable/disable and state are live, and
   so the Hide/Show All items can map onto compositor window state. The
   system menu stays shell/session-owned.
5. **System-menu action wiring.** `ShellController::onAppMenuTriggered`
   dispatches by `action`; only `quit` is functional. The rest are logged
   stubs to be wired by their owners:
   - `settings` / `about` / `about-system` → T-16 (Settings app; About panel).
   - `sleep` / `restart` / `shut-down` / `log-out` → T-24 (logind adapter A11).
   - `lock-screen` → T-26.
   - `hide` / `hide-others` / `show-all` → T-04/T-24 compositor window state.
6. **App Store item is disabled.** There is no app-store equivalent in the
   project; the item ships disabled so the system menu matches the macOS
   concept without implying a store. Decide later whether to repurpose it as a
   distro/package UI entry or drop it (see [04-shell.md](../design/04-shell.md)).

### Deferred polish (not blocking T-10+)

The interaction works end to end, but the shell's on-demand snapshot renderer
leaves rough edges: a launch highlight on the first title (leading theory:
host-cursor hover; the focus-ring cause is ruled out), choppy popup
open/close, and hover/drag-through timing. The durable fix — commit from
`QQuickWindow::afterRendering` while the scene is dirty — is shared with
T-10's Dock animation and should land there. Details and diagnostics:
[PROGRESS.md](../PROGRESS.md), "T-09 deferred polish backlog".

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
