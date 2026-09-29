# Progress Notes

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

_(160 earlier sections omitted)_

- **T124 — T-15.7b Notifications and Focus pane and tile**: **State: done.** The Settings Notifications and Focus panes and the Control; `services/system-status/src/notifications.rs` (new) — `NotificationsHost`
- **T125 — T-15.8a Lock Screen policy adapter**: **State: done.** New workspace crate `dragonfruit-lock-adapter`; `services/lock-adapter/` (new crate, workspace member) —
- **T126 — T-15.8b Lock Screen policy pane and tile**: **State: done.** The Settings Lock Screen pane and the Control Center Lock; `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 14 → 15; new
- **T127 — T-15.9a Menu Bar configuration adapter**: **State: done.** New workspace crate `dragonfruit-menubar-adapter`; `services/menubar-adapter/` (new crate, workspace member) —
- **T128 — T-15.9b Menu Bar configuration pane and tile**: **State: done.** The Settings Menu Bar pane and the Control Center Menu Bar; `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 15 → 16; new
- **T129 — T-15.10a General, About, and Updates adapter**: **State: done.** New workspace crate `dragonfruit-update-adapter`; `services/update-adapter/` (new crate, workspace member) —
- **T130 — T-15.10b General, About, and Updates pane and tile**: **State: done.** The Settings `General` pane and the Control Center `Software; `services/system-status/src/updates.rs` (new) — `UpdatesHost<S>` (refresh/
- **T131 — T-15.11a Users and Groups adapter**: **State: done.** New workspace crate `dragonfruit-account-adapter`; `services/account-adapter/` (new crate, workspace member) —
- **T132 — T-15.11b Users and Groups pane and tile**: **State: done.** The Settings `Users & Groups` pane and the Control Center; `services/system-status/src/accounts.rs` (new) — `AccountsHost<S>` (refresh/
- **T133 — T-15.12a Printers and Scanners adapter**: **State: done.** New workspace crate `dragonfruit-printer-adapter`; `services/printer-adapter/src/source.rs` — `PrinterState` (CUPS `3`/`4`/`5` +
- **T134 — T-15.12b Printers and Scanners pane and tile**: **State: done.** The Settings `Printers & Scanners` pane and the Control Center; `services/system-status/src/printers.rs` (new) — `PrintersHost<S>` (refresh/
- **T135 — T-15.13a Privacy and Security adapter**: **State: done.** New workspace crate `dragonfruit-privacy-adapter`; `services/privacy-adapter/src/source.rs` — `AppPermissionData {app,
- **T136 — T-15.13b Privacy and Security pane and tile**: **State: done.** The Settings `Privacy & Security` pane and the Control Center; `services/system-status/src/privacy.rs` (new) — `PrivacyHost<S>` (refresh/
- **T137 — T-15.14a Accessibility adapter**: **State: done.** New workspace crate `dragonfruit-accessibility-adapter`; `services/accessibility-adapter/src/source.rs` — `AccessibilityData
- **T138 — T-15.14b Accessibility pane and tile**: **State: done.** The Settings `Accessibility` pane and the Control Center; `services/system-status/src/accessibility.rs` (new) — `AccessibilityHost<S>`
- **T139 — T-15.15a Network advanced (VPN) adapter**: **State: done.** The Network advanced (VPN) adapter landed as a **second; `services/networkmanager/src/vpn/mod.rs` (new) — module docs + re-exports.
- **T140 — T-15.15b Network advanced (VPN) pane and tile**: **State: done.** The Settings `Network` pane and the Control Center `VPN` tile; `services/system-status/src/vpn.rs` (new) — `VpnHost<S>` (refresh/view/state/
- **T141 — T-15.16 Absent-daemon matrix and breadth capture**: **State: done.** The T-15 track is closed. The absent-daemon masking matrix is; `docs/design/08-settings.md` — new "The absent-daemon masking matrix
- **T142 — T-16.1a Per-output chrome sizing and reserved zones**: **State: done.** Reserved zones are now **per output**: `aggregate_reserved_for`; `compositor/src/shell/layer.rs` — `aggregate_reserved_for(output_name,
- **T143 — T-16.1b Per-output window placement**: **State: done.** New windows now open on the **focused output** and inside; `compositor/src/state.rs` — new `focused_output()` + `output_geometry_named()`;
- **T144 — T-16.2 Hotplug under load and lockstep**: **State: done.** Output hotplug now preserves lockstep and loses no windows.; `compositor/src/workspace/mod.rs` — `add_output` mirrors the existing
- **T145 — T-16.3a Integer-scaled Xwayland**: **State: done.** The integer-scale half of `docs/xwayland-scaling.md` is built.; `compositor/src/xwayland.rs` — `XwaylandState.integer_scale`; computed and
- **T146 — T-16.3b Viewport downscale and chrome sizing**: **State: done.** Fractional-scale chrome is sized per output and the nested; `compositor/src/backend/mod.rs` — `ENV_NESTED_SCALE` = `DRAGONFRUIT_NESTED_SCALE`.
- **T147 — T-16.6a AT-SPI and keyboard-only audit**: **State: done.** The live AT-SPI dump/walkthrough and the keyboard-only; `scripts/t16-a11y-audit.sh` + `scripts/t16-a11y-audit.py` (new;
- **T148 — T-16.6b Magnifier and reduced-motion sweep**: **State: done.** Two units landed: a compositor-owned screen magnifier and an; `compositor/src/magnifier.rs` (new) — the pure `Magnifier` policy: zoom
- **T149 — T-16.7 Localization and i18n**: **State: done.** The shell and first-party apps are translatable and a locale; `libs/i18n/` (new static lib `dragonfruit-i18n`) — `i18n.cpp`/`i18n.h`:
- **T150 — T-16.8a Crash/kill matrix**: **State: done.** The crash/kill matrix covers every restartable shipped; `services/session/tests/kill_matrix.rs` (5 → 9 tests) — the stand-in plan is
- **T151 — T-16.8b Compositor-death behavior and restart-policy docs**: **State: done.** Compositor death is documented once as session-ending and the; `services/session/tests/restart_policy_matrix.rs` (new; 3 tests) — the
- **T151a — T-16.12 Synthetic chrome pointer injection timestamps**: **State: done.** Every chrome surface that re-injects the compositor's pointer; `shell/src/chromepointer.{h,cpp}` (renamed from `dockpointer.*`, still in
- **T152 — T-17.1a Nested window loop verification**: **State: done.** The nested window loop is verified with a committed capture.; `scripts/capture-t17-window-loop.sh` + `scripts/t17-window-loop-driver.py`
- **T153 — T-17.1b Workspace, Mission Control, and app-switch verification**: **State: done.** The T-17 premium gate's navigation verification unit lands; `scripts/capture-t17-navigation.sh` + `scripts/t17-navigation-driver.py`
- **T154 — T-17.1c Flatpak/browser end-to-end verification**: **State: done.** A real Flatpak browser (`org.mozilla.firefox`) file-chooses,; `scripts/capture-t17-flatpak-browser.sh` + `scripts/t17-flatpak-browser-driver.py`
- **Follow-ups**: **Live PipeWire screencast producer.** The ScreenCast portal flow returns the; **Stable Mission Control window-card locator.** So a future capture can
- **T-14.7aa — Dock hover reference polish: bar geometry, zoom profile, and label tail**: **State: done.** The Dock's hover state is retuned to; **The background geometry.** macOS keeps the dock background a constant
- **T176a — T-19.1a Vendor Phosphor, QML resource plumbing, glyph primitive**: **State: done.** Track 19's foundation lands: the Phosphor icon set is vendored; `assets/icons/phosphor/` (new) — Phosphor **2.0.8** (`v2.0.8`,
- **T176b — T-19.1b System Settings category style**: **State: done.** System Settings now draws its category icons as; `design-system/components/SettingsCategoryIcon.qml` (new) — props `source`,
- **T176c — T-19.1c Menu-bar icon migration to Phosphor**: **State: done.** Every menu-bar status mark now renders from Phosphor via; `shell/menubar/StatusGlyph.qml` — rewritten. A `glyphName` switch maps each
- **T176d — T-19.1d Dock Files tile and first-party app icons**: **State: done.** Our two first-party apps now carry Phosphor-derived artwork:; `assets/icons/apps/org.dragonfruit.Files.svg` / `org.dragonfruit.Settings.svg`
- **T177 — T-19.2 Applications drawer**: **State: done.** The shell's launcher landed: a full-output `apps-drawer` overlay; `shell/src/appsdrawer.{h,cpp}` (new) — the pure list model:
<!-- symphony:digest:end -->

Working notes for the plan in [ROADMAP.md](ROADMAP.md). The harness maintains the
"Key facts" digest near the top; each session appends a `## TNN` section with
environment quirks, decisions that go beyond the design docs, and follow-ups the
next task needs. Keep newest details last within a section.

The legacy (phase-based) plan's notes are archived at
[tasks/legacy/PROGRESS.md](tasks/legacy/PROGRESS.md); T-xx numbers there are
legacy numbers and do not match this plan.

## How this plan runs

- 171 one-session tasks; execution order is [ROADMAP.md](ROADMAP.md). Task ids
  `T01`–`T171` are positional; the stable design ids `T-01.1a` etc. are in each
  task file title and in [design/tracks/](design/tracks/).
- 161 nested/human tasks run first; the 10 `[hw]` tasks are Phase 18 (hardware
  rail) and need a seat, spare GPU or clean VM. T169–T171 (T-12.6, the
  real-session dev harness) are Phase 19 and are validated on that rail.
- Independent verify after every task: `make e2e`. Full gate: `make check`
  (lint + test + 100-cycle soak).
- Design references live in [design/](design/) and [design/tracks/](design/tracks/);
  a task's reference is inlined into its prompt.
- Harness install: `.symphony/` (gitignored). Config `.symphony/symphony.config.json`
  (provider `opencode`, model `openrouter/deepseek/deepseek-v4.1-flash`, jev enabled).
- Definition of done per task: building, headless test green in `make e2e`,
  reduced-motion/degrade variant where relevant, a capture under `docs/captures/`,
  and an explicit deferred list. Human sign-off is batched at track boundaries.

## Environment / toolchain

- Qt/CMake toolchain at `~/.local/df-toolchain/usr` (Qt 6.11, CMake 4.3,
  Ninja 1.13); the Makefile auto-discovers it. Pass
  `PATH=~/.local/df-toolchain/usr/bin:$PATH` and
  `LD_LIBRARY_PATH=~/.local/df-toolchain/usr/lib64` when driving cmake/ctest
  directly.
- Host is Fedora 44 with a KDE Wayland session (`wayland-0`); the nested backend
  works against it.
- `libxkbcommon.so` (linker name) is missing; symlink at
  `~/.local/lib/libxkbcommon.so` and `RUSTFLAGS="-L ~/.local/lib"` cover it.
  `~/.local/df-devroot/lib64` supplies libgbm/libseat/libinput/libudev for the
  DRM build.
- rustfmt/clippy are installed for the pinned toolchain.
- Smithay `=0.7.0`, calloop `=0.14.4`, wayland-server `=0.31.10` are exact pins;
  upgrades are deliberate events (design/14-risks.md). `df-ipc` is std-only by
  policy (docs/licensing.md).
- `make e2e` and `make check` are the scripted gates; `make demo` (T-01.6a)
  launches the nested loop for capture work.

## Landed foundation

Compositor core (nested/DRM/headless, calloop, damage-driven rendering), input
and shortcut engine, window model, Spaces, Xwayland, private shell protocols,
design system + gallery goldens, menu bar, Dock, overview state machine, dev
workflow (`make dev`/`e2e`/`soak`). Details and follow-ups are in
[tasks/legacy/PROGRESS.md](tasks/legacy/PROGRESS.md); the new plan builds on this
rather than rebuilding it.
## T01 — T-01.1 Titlebar render element

**State: done.** The SSD titlebar element exists, is sized from the generated
tokens, and carries assertable geometry/insets; Tier-3 (CSD) windows get none.

What landed:

- **`compositor/src/window/decoration.rs`** — `TitlebarElement`,
  `WindowInsets`, `TrafficLightKind`/`TrafficLightButton`, `ColorScheme`.
  `TitlebarElement::for_window(window_id, content, tier, state, focused,
  hovered)` returns `None` for `ClientSide`, for fullscreen, and for minimized
  windows. The titlebar rect is directly *above* the content
  (`y = content.y - TITLEBAR_HEIGHT`), so the client area sits below it. The
  flat fill and the three left-side traffic lights are emitted as
  `SolidColorRenderElement`s by `render_elements(scale, output_origin)`
  (hover reveals axis-aligned glyph marks; disabled draws the muted fill with
  no glyph). `TITLEBAR_HEIGHT == 40` from `component.titlebar.height`.
- **`compositor/src/render.rs::titlebar_render_elements`** — custom elements
  above the window `Space`, one list per output.
- **Backends** — `NestedOutputElements` (nested) and `DrmOutputElements`
  gained a `Decoration = SolidColorRenderElement` variant; both compose the
  titlebar elements. Headless has no renderer and is unaffected.
- **Tier wiring** — Wayland toplevels get `DecorationTier::ServerSide` by
  default and `ClientSide` only for an explicit `xdg-decoration` CSD request
  (`state.rs::toplevel_decoration_tier`; runtime changes via
  `apply_decoration_mode`). X11 already set the tier (unchanged).
- **Geometry** — `DfState::insets_for` / `titlebar_element`. `zoom_window`
  insets its usable target by the titlebar, so a zoomed client is configured
  `usable - 40` tall through the one `configure_window_size` path.
- **Test plumbing** — the T-03 synthetic-input socket accepts
  `query decorations` and replies with `decoration <id> <ssd> <tbx> <tby>
  <tbw> <tbh> <cx> <cy> <cw> <ch>` lines + `end`. The sender must bind its
  socket (an unbound datagram has no address to reply to); `SyntheticInput`
  in both test files now binds a `.reply` path.

Commands that work (from the repo root, Makefile sets the env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` (137 unit
  tests, 10 new in `window::decoration`).
- `cargo test -p dragonfruit-compositor --test window_conformance` (5 tests;
  `ssd_toplevel_carries_a_titlebar_and_csd_does_not` is the T-01.1 gate).
- `cargo test -p dragonfruit-compositor --test xwayland_conformance`
  (`x11_window_carries_the_ssd_titlebar`).
- `make e2e` (all green).

Gotchas for later tasks:

- `window_conformance::toplevel_zoom_and_fullscreen_round_trip_over_protocol`
  now expects the maximize configure height `OUTPUT_H - TITLEBAR_HEIGHT`; the
  floating restore and fullscreen sizes are unchanged (fullscreen hides the
  titlebar and reserves no inset).
- The titlebar is *above* the client geometry, so placement/zoom must keep the
  total decorated bounds in mind; only zoom applies the inset so far.
- The traffic-light glyphs are placeholder solid marks — T-04 owns the real
  material/icon pass; `ColorScheme` defaults to `Dark` until T-08 settings.
- Titlebars are custom elements composited above the whole window `Space`, so
  with stacked windows a lower window's titlebar can draw over a higher
  window's content. Correct interleaving needs the per-window scene-element
  refactor T-04 is already due to do; note it there.
- Hover is always `false` in `DfState::titlebar_element` until T-01.2; the
  element already exposes button rects and `button_at` for its hit-test.

## T02 — T-01.2 Traffic-light actions

**State: done.** Close/minimize/zoom are clickable through the titlebar
hit-test and wired to the existing window state machine; hover reveals the
glyphs. No new window state or second owner of truth.

What landed:

- **`compositor/src/window/decoration.rs`** — `TitlebarElement::cluster_rect()`
  (the union of the three button rects that drives glyph reveal; hovering the
  bar elsewhere does not reveal). `button_at` already skipped disabled
  buttons.
- **`compositor/src/state.rs`** — new field `DfState::hovered_titlebar:
  Option<WindowId>`. `titlebar_element` now sets `hovered` from it;
  `titlebar_hover_at` / `update_titlebar_hover` recompute it from a pointer
  point; `titlebar_button_at` returns the topmost `(Window, TrafficLightKind)`
  under a point; `traffic_light_action` maps Close→`close_window`,
  Minimize→`minimize_window`, Zoom→`zoom_window`/`unzoom_window`; new
  `close_window` handles Wayland (`toplevel.send_close()`) and X11
  (`X11Surface::close()`, i.e. `WM_DELETE_WINDOW` or destroy). The window menu
  Close arm and the shell protocol `df_toplevel.close` now call the same
  `close_window`.
- **`compositor/src/input.rs`** — `PointerButton` (left, pressed): chrome wins
  first; otherwise the titlebar is consulted only when `surface_under` is
  `None`, then a click on a light is consumed (`return`, never forwarded);
  a left click on a client surface still focuses as before. `PointerMotion`
  and `PointerMotionAbsolute` call `update_titlebar_hover` and schedule a
  redraw on change.
- **`compositor/src/input/synthetic.rs`** — `query decorations` now appends
  the window state (`floating|zoomed|minimized|fullscreen`) as an 11th field,
  so minimize/zoom are observable over the harness. Backward compatible
  (existing parsers ignore trailing tokens). `decoration_report` doc updated.
- **Tests** — `window_conformance::traffic_lights_drive_zoom_minimize_and_close`
  (Wayland: green zooms to `OUTPUT_W × OUTPUT_H - TITLEBAR_HEIGHT`, green
  again unzooms to 200×150, yellow reports `minimized` with no titlebar, a
  fresh window's red delivers `xdg_toplevel.close`). `xwayland_conformance::
  x11_traffic_lights_drive_zoom_minimize_and_close` (same four actions for an
  X11 client; close removes the window from `_NET_CLIENT_LIST`).
- **Docs** — `docs/design/05-window-decorations.md` implementation status
  updated (reduced motion recorded N/A: no animation in this slice).

Commands that work (from the repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor`
  (139 unit tests; +2 in `window::decoration`).
- `cargo test -p dragonfruit-compositor --test window_conformance` (6 tests).
- `cargo test -p dragonfruit-compositor --test xwayland_conformance` (4 tests).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` clean,
  `cargo fmt --all -- --check` clean.

Gotchas for later tasks:

- Traffic-light geometry comes from `component.trafficLights` tokens:
  diameter 12, gap 8, inset 12; centers are at
  `tx + 12 + 6 + index*(12+8)`, `ty + h/2` (0=close, 1=minimize, 2=zoom).
  Tests hardcode these; if the tokens move, update
  `window_conformance.rs` / `xwayland_conformance.rs`.
- The titlebar interception deliberately requires `surface_under(...).is_none()`.
  With stacked windows a top window's titlebar button under a lower window's
  content will lose to that content — the same per-window scene-element
  limitation T-04 owns (titlebars are custom elements above the whole `Space`).
- Clicking a traffic light does **not** focus/raise the window in this slice
  (T-01.3 drag/activation can add it). Actions work on an unfocused window;
  the lights draw muted but still respond.
- Minimize leaves `active_window` pointing at the hidden window (pre-existing
  `minimize_window` behavior); a later task may want focus hand-off.
- X11 close is new here: `close_window` is the one place Wayland and X11 close
  converge; use it instead of `toplevel.send_close()` directly.

## T03 — T-01.3 Titlebar drag, double-click, fullscreen reveal

**State: done.** A floating window's titlebar drags to move (existing
`MoveGrab`), a double-click dispatches the configured
`dock.titlebarDoubleClick`, and a fullscreen window reveals an overlay
titlebar on top-strip hover. All geometry still flows through
`move_window`/`configure_window_size`.

What landed:

- **`compositor/src/window/decoration.rs`** — `TitlebarDoubleClick`
  (`Zoom` default | `Minimize` | `None`, with `parse`/`name`),
  `DoubleClickTracker`/`TitlebarClick` (400 ms, 4 px slop),
  `fullscreen_reveal_rect(content)` (the top `TITLEBAR_HEIGHT` strip), and
  `TitlebarElement::for_window` now lays out a fullscreen titlebar as an
  overlay on the content (`titlebar.loc == content.loc`, `insets == NONE`)
  only while `hovered`. Floating/zoomed geometry is unchanged.
- **`compositor/src/state.rs`** — `DfState::titlebar_double_click` +
  `titlebar_clicks` fields; `set_titlebar_double_click`,
  `register_titlebar_click`, `titlebar_double_click_action`,
  `titlebar_window_at` (topmost titlebar under a point),
  `activate_window(window, serial)`, `begin_titlebar_move(window, serial)`.
  `titlebar_hover_at` now returns a fullscreen window's id when the pointer
  is in its reveal strip. `focus_window` already existed on `DfState` in
  `xwayland.rs`, hence the name `activate_window` for titlebar activation.
- **`compositor/src/input.rs`** — left press on compositor chrome now: finds
  the topmost titlebar window; a floating/zoomed titlebar still requires
  `surface_under(..).is_none()` (popup/client priority), a revealed
  fullscreen titlebar wins even when a client surface is under it; then it
  activates the window and dispatches light action → double-click →
  `MoveGrab` (floating only). Drag is not offered on zoomed/fullscreen.
- **`compositor/src/input/synthetic.rs`** — new
  `set titlebar-double-click zoom|minimize|none` command (test plumbing).
- **Tests** — `window_conformance::titlebar_drag_and_double_click_move_and_zoom`
  (drag moves content by the pointer delta; default double-click zooms to
  `OUTPUT_W × OUTPUT_H - TITLEBAR_HEIGHT`; after setting `minimize` a
  double-click minimizes). `window_conformance::fullscreen_hover_reveals_the_titlebar`
  (fullscreen hides the bar, top-strip hover reveals it at the content top
  with no inset, moving away hides it). Decoration unit tests cover the
  fullscreen overlay, the reveal rect, the tracker, and the setting parse.
- **Docs / ADR** — `docs/design/05-window-decorations.md` behavior spec +
  status updated. ADR `0001-titlebar-double-click-setting-owner.md`: the
  compositor holds the setting until T-08 settingsd pushes it.

Commands that work (from the repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor`
  (144 unit tests; +5 in `window::decoration`).
- `cargo test -p dragonfruit-compositor --test window_conformance` (8 tests).
- `cargo test -p dragonfruit-compositor --test xwayland_conformance` (4 tests).
- `make e2e` (all green). `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo fmt --all -- --check` clean.

Gotchas for later tasks:

- T-01.2's "clicking a light does not focus" is superseded: any titlebar
  press now calls `activate_window` first, so the window takes keyboard
  focus. The old `titlebar_element`'s `focused` flag follows it. Activation
  only sets keyboard focus; it does not raise the window in the `Space`
  (existing click-to-focus does not raise either).
- `dock.titlebarDoubleClick` is not yet wired to the shell; the compositor
  default is `Zoom` and the synthetic command sets it. T-08 must call
  `DfState::set_titlebar_double_click` (ADR 0001).
- Double-click pairs on `(window, time, location)`; a drag press leaves a
  pending click for 400 ms. A synthetic test that drags then double-clicks
  must sleep > 400 ms (or use a different window) to avoid a false pair.
- Fullscreen reveal zone == the titlebar overlay rect (top
  `TITLEBAR_HEIGHT` of the content). Because the overlay covers client
  pixels, it intercepts presses there while revealed; hidden, client input
  is untouched.
- Zoomed/fullscreen windows do not start a titlebar move (grab module's
  contract); if a later task wants drag-to-unzoom, add it deliberately.
- The titlebar is still a custom element above the whole window `Space`; the
  stacked-window interleaving limitation from T-01.1/T-01.2 is unchanged
  (T-04's per-window scene-element refactor).

## T04 — T-01.4 Window menu

**State: done.** A right/Control-click on the SSD titlebar opens a
compositor-owned menu (Move to Space, Minimize, Zoom, Close); each row routes
through `DfState::window_menu_command`, the same primitives the traffic
lights and shell protocol use. Escape / click-away / focus-loss dismiss;
keyboard navigation mirrors the design-system `ContextMenu`.

What landed:

- **`compositor/src/window/menu.rs`** — `WindowMenu`, `WindowMenuRow`,
  `MenuRow`, `MenuKey`/`MenuKeyOutcome`, `MenuActivation`. Geometry from
  `component.contextMenu` (padding 6, rowHeight 26, minWidth 180) +
  `component.window.borderWidth`; panel flips/clamps inside the output.
  Move to Space is a submenu of the window output's Spaces (snapshotted at
  open time). Keyboard/dismissal mirror `ContextMenu.qml` (Escape closes the
  submenu first; Up/Down/Home/End move highlight; Right/Left submenu;
  Enter/Space activates). Rendering is a flat token fill (surface elevated +
  accent highlight); labels are T-04.
- **`compositor/src/state.rs`** — `DfState::window_menu: Option<WindowMenu>`;
  `open_window_menu`, `close_window_menu`, `window_menu_open`,
  `window_menu_click`, `window_menu_key`. `focus_changed` dismisses on focus
  loss. `ColorScheme` gained `surface_elevated`/`border`/`accent` and
  `decoration::color_from_rgba` is now `pub(crate)`.
- **`compositor/src/input.rs`** — right-click (`BTN_RIGHT`) or
  Control-left-click opens on a titlebar (same client-surface priority as a
  left press; revealed fullscreen titlebar wins). While open the menu is
  modal: pointer presses inside are consumed, outside dismiss; every key is
  intercepted before the shortcut engine.
- **`compositor/src/render.rs` + backends** — `window_menu_render_elements`
  composited above titlebars in nested and DRM.
- **`compositor/src/input/synthetic.rs`** — `query window-menu` (open,
  window, panel rect, highlighted, submenu open, submenu rect) and a trailing
  assigned-Space id on `query decorations` lines.
- **Tests** — `window_conformance::{window_menu_opens_dismisses_and_is_keyboard_operable,
  window_menu_runs_zoom_minimize_close_and_move_to_space}` and
  `xwayland_conformance::x11_window_menu_runs_all_four_commands`. 9 new unit
  tests in `window::menu`.
- **Docs / ADR** — `docs/design/05-window-decorations.md` updated; ADR
  `0002-window-menu-ownership.md`: the menu is compositor-owned and
  shell-independent.

Commands that work (from the repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor`
  (153 unit tests).
- `cargo test -p dragonfruit-compositor --test window_conformance` (10 tests).
- `cargo test -p dragonfruit-compositor --test xwayland_conformance` (5 tests).
- `make e2e` (all green). `cargo clippy --workspace --all-targets -- -D warnings`
  and `cargo fmt --all -- --check` clean.

Gotchas for later tasks:

- The scope said "design-system menu components"; the compositor has no QML
  or text renderer, so it reproduces the `ContextMenu` geometry and rules in
  Rust. The shell QML component is not involved (ADR 0002).
- A Control-left-click now opens the menu instead of dragging the window.
- The menu snapshots Spaces at open; a workspace change while open leaves it
  stale until dismissal.
- Menu rendering is flat and label-less (T-04 draws labels/materials, T-14
  reuses the model). The current Space is not marked in the submenu.
- `window_menu_render_elements` draws only on the output containing the menu
  anchor (the panel is clamped to that output).

## T05 — T-01.5 Decoration tier policy and X11 correctness

**State: done.** The three decoration classes are honored and the X11
configure flows the titlebar inset through the one geometry path.

What landed:

- **`compositor/src/state.rs`** — new `DfState::set_decoration_tier(window,
  tier)` (pub(crate)) is the single entry point for a mapped tier change: sets
  the model and, when the reserved inset changes, reflows a zoomed window via
  `reapply_insets` (same `usable_geometry_for` + `insets_for` +
  `configure_window_size` math as `zoom_window`). `apply_decoration_mode`
  routes through it. Floating/fullscreen windows reserve no inset and are not
  resized.
- **`compositor/src/xwayland.rs`** — `_MOTIF_WM_HINTS` property updates call
  `set_decoration_tier`, so runtime X11 tier flips reflow. Tier mapping is
  unchanged: motif decorations=0 → ClientSide (no titlebar), else ServerSide.
- **Tests** — `window_conformance::{decoration_tier_matrix_default_explicit_ssd_and_csd_side_by_side,
  runtime_decoration_tier_change_reflows_a_zoomed_window}` (2 new, 12 total);
  `xwayland_conformance::x11_decoration_tier_follows_motif_hints_and_configures_insets`
  (1 new, 6 total). `cargo test -p dragonfruit-compositor --bin
  dragonfruit-compositor` 153 unit tests unchanged.
- **Docs** — `docs/design/05-window-decorations.md` T-01.5 section added. No
  ADR (applies the policy already recorded in 05 and ADR 0001/0002).

Commands that work (from the repo root; `make` sets the toolchain env;
`PKG_CONFIG_PATH` must be `$HOME/.local/df-devroot/lib64/pkgconfig` when
driving cargo directly):

- `make e2e` (exit 0, 7 suites green).
- `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check` clean.

Gotchas for later tasks:

- Smithay reparents managed X11 windows into a frame window, so
  `x11rb`'s `get_geometry(client)` returns origin (0,0) and only the size is
  meaningful; the content offset lives in the compositor's report/frame.
  Verify X11 geometry by size, not by the client window's x/y.
- A zoomed window's inset is only re-applied on a *tier change*. Other
  inset-changing events (reserved zones, output mode) still need the same
  reflow — `zoom_window` with a changed target updates the stored zoomed
  geometry but `changed == false`, so it neither remaps nor reconfigures.
  A later task should fix or reuse `reapply_insets` there.
- `_NET_FRAME_EXTENTS` is not published to X11 clients; they cannot see the
  40 px titlebar strip.
- Floating windows add/remove the titlebar above their content with no
  resize; only zoomed windows reflow.

## T06 — T-01.6a `make demo` harness

**State: done.** One `make demo` target builds the tree and runs the T-01 loop
demo; the same target runs headless as the CI scripted half and is wired into
`make e2e`.

What landed:

- **`Makefile`** — `demo: build` → `cargo run -p dragonfruit-dev -- dev --demo
  $(DEMO_ARGS)`; new `DEMO_ARGS` variable; `e2e` now depends on `build` and
  ends with `$(MAKE) demo DEMO_ARGS=--headless`. `demo` added to `.PHONY` and
  `help`.
- **`tools/dragonfruit-dev/src/demo.rs`** (new) — `qt_app()`, `x11_app()`,
  `qml_import_path()`, `checklist()`, `SCRIPTED_SETTLE` (3000 ms). Overrides:
  `DF_DEMO_QT_APP`, `DF_DEMO_X11_APP`, `DF_QML_IMPORT_PATH`. 3 unit tests.
- **`tools/dragonfruit-dev/src/main.rs`** — `dev --demo` subcommand and a
  refactor of the session lifecycle into `start_session` / `launch_program` /
  `launch_shell` / `teardown_session` / `wait_for_compositor_exit`, shared by
  `run_dev_session` and `run_demo_session`. Demo backend: explicit flag wins,
  else nested when `WAYLAND_DISPLAY` is set, else headless (scripted). Launches
  shell + Qt/Wayland app (`QT_QPA_PLATFORM=wayland`, `QML_IMPORT_PATH=build/qml`,
  software rendering when headless) + X11 app (`xmessage`). Exit non-zero on a
  launch failure, an early child death in the scripted half, or a dirty
  teardown. 3 new unit tests (parser).
- **CI** — `.github/workflows/ci.yml` installs `xwayland`/`x11-utils` and runs
  `make demo` after the Qt build.
- **Docs** — `docs/design/11-session-and-dev-workflow.md` "The demo harness"
  section; `README.md` dev-workflow section.

Commands that work (from the repo root; `make` sets the toolchain env):

- `make demo DEMO_ARGS=--headless` → exit 0; prints the checklist, launches
  shell + Qt + X11, `all children alive after settle`, clean teardown.
- `make demo` (host Wayland session) → nested; SIGTERM/window close → exit 0,
  no socket leak.
- `make e2e` → exit 0 (7 conformance suites + demo smoke).
- `cargo test -p dragonfruit-dev` (6 tests); `make clippy`; `cargo fmt --all
  -- --check`; `make soak SOAK_CYCLES=3` — all clean.

Gotchas for later tasks:

- **Fixed a pre-existing `--launch` parser bug**: each `--launch` overwrote the
  previous group, so only one extra app could launch. Repeated `--launch` now
  accumulates.
- First-party Qt apps need `QML_IMPORT_PATH=build/qml` at runtime (the shell
  bakes an equivalent define; the apps do not). The demo exports it.
- `make e2e` now builds the Qt/CMake side (it depends on `build`), because the
  demo needs the shell and apps.
- X11 half is skipped, with a note, when Xwayland or `xmessage` is absent; a
  Wayland-only `make demo` is valid and exits 0.
- The demo auto-launches both clients (scope says launch them); the checklist
  still directs the human to the Dock. The Dock/menu-bar integration capture is
  T-01.6b.

## T07 — T-01.6b Loop integration walkthrough and capture

**State: done.** The live nested walkthrough of the T-01 loop is scripted and
captured; the Dock-entry transitions over the shell protocol are asserted
headlessly.

What landed:

- **`scripts/capture-demo.sh`** (new) + **`scripts/capture-demo-driver.py`**
  (new) — launch `make demo` nested with the synthetic-input harness bound,
  drive focus → window menu → zoom → minimize → restore-from-Dock → close
  through the real input router, screenshot each step with Spectacle, and
  assemble `docs/captures/t01-loop-v0.mp4`. Not in `make e2e` (needs a host
  session + Spectacle + ffmpeg + Pillow).
- **`docs/captures/t01-loop-v0.*`** — `t01-loop-v0.png` (whole loop), plus
  `-wayland`, `-x11`, `-titlebar`, `-dock`, `-dock-minimized`, `-menu`,
  `-zoomed`, `-minimized`, `-restored`, `-closed`, and the `.mp4` (~312 KB
  total).
- **`compositor/src/backend/nested.rs`** — the nested backend now installs
  `crate::input::synthetic::install` when `DRAGONFRUIT_SYNTHETIC_INPUT` is
  set, mirroring headless. Test plumbing; unset in a real session.
- **`compositor/tests/shell_protocol_conformance.rs`** — new
  `dock_entry_lifecycle_focus_running_minimize_restore_close`: name
  (title/app id) → running → focus → minimize → restore → close events
  (31 → 32 tests).
- **Docs** — `docs/design/11-session-and-dev-workflow.md` "Capturing the
  walkthrough"; `docs/captures/README.md` names the stills. No ADR (the
  synthetic-input harness is existing test plumbing; extending it to nested
  is not an architecture decision).

Commands that work (from the repo root):

- `bash scripts/capture-demo.sh` → stills + `docs/captures/t01-loop-v0.mp4`;
  observed states: focus/menu/zoom/minimize/restore/close all took; the
  window's Dock entry resolved after close.
- `make e2e`, `cargo test --workspace` (shell_protocol_conformance 32 tests),
  `make soak SOAK_CYCLES=3`, `make lint`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all -- --check` — all green.

Gotchas for later tasks:

- The synthetic-input socket is a `UnixDatagram`; replies (to `query
  decorations` / `query window-menu`) go back to the sender's bound path, so
  a driver must `bind()` its own path and drain `end\n`.
- The nested output is 1920x1200 and the host window lands at a fixed
  position; `capture-demo-driver.py` finds it by the Space-0 wallpaper color
  `(33,13,41)` and crops from there. Re-check that color if the default
  wallpaper changes.
- The `dock entry` restore in the live capture is found by diffing the Dock
  band before/after minimize and clicking the changed blob; it is layout-
  dependent but resolved deterministically.
- The capture guest is a *nested* window on the host; a host KWin context
  menu can occasionally appear if the host pointer sits on the nested window
  decoration. The step stills crop to the nested output, so it is not in the
  committed stills.
- This slice has no motion (T-02), so the clip is the ordered states, not
  continuous animation. The CSD live still is absent (no CSD client in the
  demo); the matrix is covered by `window_conformance`.

## T08 — T-02.1a Animation clock and frame discipline

**State: done.** One shared compositor animation clock exists; the overview's
discrete slide and registered lifecycle animations run on it, one compositor
frame per animation frame, zero damage when idle.

What landed:

- **`compositor/src/animation.rs`** (new) — `FRAME_INTERVAL` (16 ms),
  `ease()` cubic-bezier solver, `Tween` (`new`, `from_motion(start, Motion,
  reduced_motion)`, `raw_progress`, `progress`, `is_done`, `finish_ms`),
  `Animation<S>` trait (blanket impl for `FnMut(&mut S, u64) -> bool`),
  `TweenAnimation`, and `AnimationClock<S>` (running set, reduced-motion
  flag, `frames_stepped`/`animations_started`/`animations_completed`).
  6 unit tests.
- **`compositor/src/state.rs`** — `animation_clock: AnimationClock<DfState>`
  and a single reusable `animation_timer` replace `overview_timer`;
  `animations_active()`, `start_animation()`, `step_animations()`,
  test-only `start_dummy_animation()`; `set_reduced_motion` now writes both
  the overview machine and the clock; `dump_stats` appends
  `animation_frames_stepped=` to the render-stats line and emits an
  `animation stats` line.
- **`compositor/src/input.rs`** — `schedule_animation_timer` arms the one
  calloop timer only while `animations_active()`; `poll_animations` advances
  the overview discrete pipeline and every registered animation once, applies
  `apply_overview_scene`, applies a commit, requests exactly one redraw, and
  re-arms. `schedule_overview_timer`/`poll_overview_animation` are gone.
- **`compositor/src/input/synthetic.rs`** — `animate-dummy <ms>` starts a
  scene-free `TweenAnimation` (test plumbing).
- **Tests** — `compositor/tests/animation_clock.rs` (new): 200 ms dummy →
  `frames_rendered` delta == `animation_frames_stepped` delta (observed +13
  = +13), both flat after it settles; zero-duration → one step.
  `idle_trace.rs`/`shell_idle_trace.rs` also assert the animation counter is
  flat. `Makefile` `e2e` runs `--test animation_clock`.
- **Docs** — ADR `0003-shared-animation-clock.md`;
  `docs/design/02-compositor.md` "Animation clock".

Commands that work (from the repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 164 bin unit + all suites green
  (animation_clock 2/2, shell_protocol_conformance 32/32, idle_trace,
  shell_idle_trace).
- `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo fmt --all -- --check` — clean.
- `make e2e` — green (includes the new suite).

Gotchas for later tasks:

- Use `DfState::start_animation` to begin a transition and
  `DfState::animations_active` as the only liveness predicate; never arm a
  second timer or add a discrete instant path.
- The clock is stepped via `DfState::step_animations`: it swaps the clock out
  so an animation may mutate state. Avoid starting an animation from inside
  another's `advance` (it is absorbed, but keeping the rule avoids relying on
  it).
- `Tween::from_motion(..., reduced_motion)` is the reduced-motion hook; the
  token `reduced_duration_ms` values are all 0 today, so the transition
  completes on its first step.
- `frames_rendered` and `animation_frames_stepped` are on the same
  `SIGUSR1`/exit render-stats line; the added token does not break the
  existing three-field parse.
- T-02.1b: store the per-window appear tween in window/scene state and
  register an `Animation<DfState>`; the Dock tile-origin hand-off stays
  additive/optional (centered origin headless).

## T09 — T-02.1b Window appear transition

**State: done.** A newly mapped window scales/fades in from its Dock tile on
the shared clock; with no shell it degrades to a centered origin; reduced
motion takes one step through the same commit path.

What landed:

- **`compositor/src/window/appear.rs`** (new) — `AppearTransition`
  (`new`, `centered_origin`, `progress`, `is_done`, `frame`), `AppearFrame`
  (`rect`, `scale`, `offset`, `alpha`), `APPEAR_MIN_SCALE = 0.8`; 4 unit
  tests. A transition carries `frames` and `completed` so the reduced-motion
  assertion is `frames == 1`.
- **`compositor/src/window/mod.rs`** — `WindowEntry.appear`;
  `WindowModel::{set_appear, appear, appear_frame, step_appearances,
  appearances}`.
- **`compositor/src/state.rs`** — `pending_appear_origins` (bounded 64,
  keyed by `app_id`), `set_launch_origin`, private `begin_window_appear`
  (called from `map_pending_windows`), `window_appear_frame`,
  `step_window_appearances`. Model geometry and the `Space` location stay
  the final geometry — the appear is render-only, so input/layout never move.
- **`compositor/src/render.rs`** — `window_render_elements` replaces the
  window half of `space::render_output` (same order/locations); an appearing
  window's surface elements are wrapped in `RescaleRenderElement` +
  `RelocateRenderElement` and faded through the surface-tree alpha.
  `titlebar_render_elements` passes the frame to
  `TitlebarElement::render_elements`, which scales/fades its rects.
- **`compositor/src/backend/nested.rs`, `backend/drm.rs`** — new `Appear`
  render-element variant; both render the manual list through their damage
  tracker (`window_render_elements`), so `space::render_output` is no longer
  used for windows.
- **Protocol** — `df_toplevel_manager.set_launch_origin(app_id, x, y, w, h)`
  (`since=4`, manager 3→4); compositor dispatch + `MANAGER_INTERFACE_VERSION
  = 4`; `ShellProtocol::setLaunchOrigin` and shell bind at
  `min(version, 4)`.
- **Synthetic hooks** — `query appear`, `set reduced-motion on|off`,
  `set launch-origin <app-id> <x> <y> <w> <h>`.
- **Tests** — `window_conformance.rs`:
  `window_appear_plays_from_the_dock_tile_origin_and_commits_the_target`,
  `reduced_motion_appear_takes_a_single_frame`;
  `shell_protocol_conformance.rs` manager-version assertion → 4.
- **Docs** — ADR `0004-window-appear-origin-and-transform.md`;
  `docs/design/02-compositor.md` "Window appear (T-02.1b)";
  `docs/private-protocols.md`.

Commands that work (from the repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 168 bin unit + all suites green
  (window_conformance 14/14 incl. the 2 new appear tests;
  shell_protocol_conformance 32/32).
- `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo fmt --all -- --check`; `make cmake-build` (shell rebuilds with the
  v4 protocol) — clean.
- `make e2e` — green; the `--headless` demo logs `animations_started=1
  animations_completed=1 animation_frames_stepped=10` for the launched
  window's appear. `make soak SOAK_CYCLES=3` — clean.

Gotchas for later tasks:

- The Dock does **not** call `setLaunchOrigin` yet: the Dock tile rect is not
  threaded from QML to `ShellController::launchDockAppWithFiles`, and
  `desktopId` → compositor `app_id` mapping is T-23's resolver. Real launches
  currently use the centered fallback. Wire it with T-02.2, which needs the
  same tile lookup.
- `window_appear_frame` returns `None` once completed; the transform is
  identity. T-02.2/T-02.4/T-04 should reuse `AppearFrame` and
  `window_render_elements` rather than add a second effect path.
- Backends no longer call `space::render_output`; if `space`-owned layers are
  ever introduced, `window_render_elements` must grow that handling.
- The appear keys on `app_id`; a window whose `app_id` arrives after mapping
  gets the centered origin.

## T10 — T-02.2 Minimize and restore motion

**State: done.** Minimize shrinks a window into its Dock entry's tile and
restore grows it back out on the shared clock; the window leaves the layout
and input path immediately and is held as a model-rendered ghost until the
motion settles, so the sequence leaves no orphan and no stale state.
Reduced motion (and `minimizedAnimation=none`) collapses each motion to one
step through the same commit path.

What landed:

- **`compositor/src/window/motion.rs`** (renamed from `appear.rs`) —
  `WindowMotionKind` (`Appear`/`Minimize`/`Restore`), `WindowMotion`
  (`new`, `appear`, `centered_origin`, `progress`, `is_done`, `frame`),
  `MotionFrame`, `APPEAR_MIN_SCALE = 0.8`; 6 unit tests. Appear/restore
  interpolate `origin → target` (alpha `0 → 1`); minimize is the exact
  reverse. The render transform is always relative to `target`, so input and
  layout never move.
- **`compositor/src/window/mod.rs`** — `WindowEntry.motion`;
  `WindowModel::{set_motion, motion, motion_frame, step_motions, motions,
  active_motions}`.
- **`compositor/src/state.rs`** — `minimize_window` unmaps + broadcasts state
  immediately, then starts a `Minimize` motion (origin = tile, target =
  restore geometry); `restore_window` maps immediately and starts a `Restore`
  motion; `minimize_window_by_id`/`restore_window_by_id`; `dock_tiles` (was
  `pending_appear_origins`) is now **remembered, not consumed**, so minimize/
  restore reuse the launch tile (`motion_origin_for`, centered fallback);
  `titlebar_element_for_motion`; `step_window_motions`; `window_motion_driver`
  keeps exactly one clock closure stepping every motion once per frame.
- **`compositor/src/render.rs`** — `window_render_elements` and
  `titlebar_render_elements` walk `WindowModel::active_motions` in addition to
  `Space`, drawing a minimizing ghost (unmapped) with the same transform;
  `push_motion_elements` is the shared helper.
- **`compositor/src/shell/mod.rs`** — `broadcast_window_events` no longer
  drains the window outbox when no trusted manager is connected (headless
  observability; a late-bound shell replays the scene). `set_launch_origin`
  dispatch comment updated.
- **Synthetic hooks** — `query motion` (`query appear` is kept as an alias),
  `query events` (peek `WindowDispatch`), `minimize <id>`, `restore <id>`.
- **`WindowDispatch::events()`** peek accessor.
- **Tests** — `window_conformance.rs`:
  `minimize_and_restore_play_through_the_dock_tile_and_resolve_cleanly`,
  `reduced_motion_minimize_and_restore_take_a_single_frame`; the two T-02.1b
  appear tests moved to `query motion`/`parse_motion`.
- **Docs** — ADR `0005-minimize-restore-motion-and-ghost.md`;
  `docs/design/02-compositor.md` "Window lifecycle motion";
  `docs/private-protocols.md`; `protocols/dragonfruit-toplevel.xml`
  `set_launch_origin` description.

Commands that work (from the repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 170 bin unit + all suites green
  (window_conformance 16/16 incl. the 2 new minimize tests; animation_clock
  2/2; shell_protocol_conformance 32/32; idle traces).
- `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo fmt --all -- --check` — clean.
- `make e2e` — green (headless demo still logs `animations_started=1
  animations_completed=1`).

Gotchas for later tasks:

- Use `WindowMotion`/`MotionFrame` for every per-window effect; never add a
  second path. Minimize is the only kind that is a *ghost* (unmapped); appear
  and restore stay in `Space`.
- `window_motion_driver` must guard any future per-window motion that arms the
  clock, or the whole set will be stepped once per registered closure.
- A minimizing window is never in `Space`, so hit-testing, hover, workspace
  layout, and `take_presentation_feedback` skip it automatically; the ghost is
  rendered only from `active_motions`.
- `Tween::from_motion(..., reduced_motion)` is the reduced-motion hook;
  `dock.minimizedAnimation=none` is the same collapse, so no protocol request
  exists for it.
- The shell still does not send `setLaunchOrigin`: the Dock tile rect is not
  threaded from QML and `desktopId` → `app_id` is the T-23 resolver; real
  launches use the centered fallback.
- T-02.3: zoom/fullscreen are render-only (a window must stay mapped and not
  change state mid-motion unless the state machine says so); reuse
  `MotionFrame` and the same clock.

## T11 — T-02.3 Zoom and fullscreen transitions

**State: done.** Zoom/unzoom and fullscreen/unfullscreen animate between the
old and new geometries on the shared clock; the state change is immediate
(the window stays mapped and input-correct) and the motion is render-only, so
a re-entrant request retargets from the current interpolated rect without
waiting. Reduced motion collapses each to one step.

What landed:

- **`compositor/src/window/motion.rs`** — `WindowMotionKind::{Zoom,
  Fullscreen}`; `WindowMotionKind::fades()` (false for zoom/fullscreen);
  `frame()` pins alpha at `1.0` for them; `MotionFrame::scale_for(size)` maps
  a surface of the given size onto the interpolated `rect`; 4 new unit tests
  (10 total).
- **`compositor/src/state.rs`** — `begin_window_motion_from(window, kind,
  fallback_origin, geometry)` (the explicit-origin variant; a live motion
  retargets from its current frame). `zoom_window`/`unzoom_window`/
  `fullscreen_window`/`unfullscreen_window` capture the geometry being left,
  apply the state change immediately, then start a `Zoom`/`Fullscreen` motion;
  `*_window_by_id` wrappers for the harness/shell seam.
- **`compositor/src/render.rs`** — `push_motion_elements` scales client
  surfaces by `frame.scale_for(window.geometry().size)` (the live buffer
  size); the target-relative `frame.scale` stays for the SSD titlebar.
- **Synthetic hooks** — `zoom|unzoom|fullscreen|unfullscreen <window-id>`;
  `query motion` kinds now include `zoom`/`fullscreen`.
- **Tests** — `window_conformance.rs`: `zoom_and_unzoom_...`,
  `fullscreen_and_unfullscreen_...`, `zoom_retargets_mid_flight_without_waiting`,
  `reduced_motion_zoom_and_fullscreen_take_a_single_frame` (20/20).
  `SyntheticInput::query_batch` sends `cmd\nquery motion` in one datagram so
  the just-started motion is observed before the clock's first tick.
- **Docs** — ADR `0006-zoom-fullscreen-geometry-motion.md`;
  `docs/design/02-compositor.md` "Window lifecycle motion".

Commands that work (repo root; `make` sets the toolchain env; the direct
cargo path needs `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`
and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"`):

- `cargo test -p dragonfruit-compositor` — 174 bin unit + all suites green.
- `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo fmt --all -- --check` — clean.
- `make e2e` — green; `make soak SOAK_CYCLES=3` — 3 clean cycles.

Gotchas for later tasks:

- Zoom/fullscreen must stay render-only: never move window state or the model
  geometry into the tween; the window is at its destination geometry from the
  first frame (focus/input/configure depend on it).
- Client surfaces use `MotionFrame::scale_for(current_size)`, not
  `frame.scale`. T-04's clip/blur pass must compose on the former; the
  titlebar/other chrome uses `frame.scale`.
- A fullscreen entry hides the SSD titlebar immediately (the state is
  Fullscreen); only content geometry animates. Unfullscreen re-shows it at
  once and shrinks it with the content.
- Keep new transitions on `begin_window_motion_from`; do not register a second
  clock driver (`window_motion_driver` already guards this).
- `reapply_insets` (a runtime decoration-tier change on a zoomed window) still
  jumps; only explicit zoom/fullscreen transitions animate today.

## T12 — T-02.4a Close ghost

**State: done.** A closing window shrinks/fades out into its app's Dock tile
(or the centered fallback) as a ghost that leaves the input path and the
layout at once; it stays in the model until the motion settles and is then
removed exactly once with a `Closed` broadcast. A client that destroys its
surface mid-ghost does not double-remove (the destroy is deferred to the
motion).

What landed:

- **`compositor/src/window/motion.rs`** — `WindowMotionKind::Close` (reverse
  like minimize, `motion::WINDOW_CLOSE`), `is_ghost()` (`Minimize | Close`),
  doc "six motions"; 2 new unit tests (12 total).
- **`compositor/src/window/mod.rs`** — `WindowModel::is_closing(window)`
  (live `Close` motion, not completed).
- **`compositor/src/window/events.rs`** — `WindowEventKind::Closed` ("closed");
  the shell treats it like `Unmapped` (drop the toplevel resource, `closed()`).
- **`compositor/src/state.rs`** — `close_window` now: no-op while already
  closing; dismisses popups/its window menu; unmaps; `begin_window_motion(
  Close, geometry)` (origin = Dock tile/centered); then sends the client close
  request. `close_window_by_id(id)`. New single teardown path
  `remove_window(window, kind)` (idempotent via `WindowModel::remove`), used by
  `toplevel_destroyed` (`Unmapped`), `destroy_x11_window` (`Unmapped`), and
  `step_window_motions` (`Closed`). `toplevel_destroyed`/`destroy_x11_window`
  return early while `is_closing`. `apply_workspace_layout` never remaps a
  closing window.
- **`compositor/src/render.rs`** — the ghost walk in
  `window_render_elements`/`titlebar_render_elements` uses
  `motion.kind.is_ghost()` and skips dead windows (`IsAlive`).
- **`compositor/src/xwayland.rs`** — `destroy_x11_window` defers/uses
  `remove_window`.
- **Synthetic hook** — `close <window-id>`; `query motion` kind `close`,
  `query events` kind `closed`.
- **Tests** — `window_conformance.rs`:
  `close_fades_out_inert_and_commits_removal_exactly_once` (21/21): asserts the
  close record's tile origin/target, that the window stays tracked while the
  ghost is live, that a click over its old rect reaches no client, that it
  disappears from `query decorations`, and that exactly one `event closed` is
  broadcast.
- **Docs** — ADR `0007-close-ghost-deferred-removal.md`;
  `docs/design/02-compositor.md` "Window lifecycle motion".

Commands that work (repo root; `make` sets the toolchain env):

- `make cargo-test` — 176 bin unit + all suites green (window_conformance
  21/21; animation_clock, shell_protocol_conformance 32/32, xwayland 6/6).
- `make clippy`; `make fmt-check` — clean.
- `make e2e` — green (headless demo logs `animations_started=1
  animations_completed=1`).

Gotchas for later tasks:

- There is exactly **one** teardown path now: `DfState::remove_window`. Never
  remove a `WindowModel` entry by hand or broadcast `Unmapped`/`Closed`
  anywhere else; the `WindowModel::remove` return is the exactly-once guard.
- The close ghost's removal is committed by the **motion** (`step_window_motions`),
  not by the client destroy; `toplevel_destroyed`/`destroy_x11_window` must
  keep the `is_closing` early-return or a real client will double-remove.
- A closing window is a ghost (`is_ghost()` is `Minimize | Close`); render from
  `WindowModel::active_motions`, never from `Space`; guard dead windows.
- `close_window` sends the client close request itself; do not also send it
  from the shell/protocol.
- Veto/interrupt/retarget and the idle trace are T-02.4b; do not register a
  second motion driver.

## T13 — T-02.4b Close interruptibility and idle trace

**State: done.** A live close ghost reverses mid-flight without waiting: a
restore/`interrupt-close` replaces the `Close` with a `Restore` from the
ghost's *current* interpolated rect, re-enters the window in the layout and
input path at once, and returns focus; the pending removal is dropped with
the replaced motion, so no `Closed` is broadcast. A close ghost now releases
keyboard focus the moment it becomes a ghost (no phantom focus target). After
the reverse→close loop settles, the render/clock counters are flat.

What landed:

- **`compositor/src/state.rs`** — `close_window` clears keyboard focus when it
  starts the ghost and **mirrors** the unset in the model (`active_window =
  None`, `Unfocused` broadcast, shortcut scope cleared); Smithay does not call
  `focus_changed` when focus is unset, so the compositor must. New
  `interrupt_close(window)` / `interrupt_close_by_id(id)`: a live close is
  replaced by a `Restore` from the current frame (via
  `begin_window_motion_from`), the window is re-mapped when its state is
  visible and its Space active, and it is re-focused. `restore_window`
  delegates a closing target to `interrupt_close`.
- **`compositor/src/window/motion.rs`** — `WindowMotionKind::is_close()`; 1 new
  unit test (13 total) proving a close reversed mid-flight restores from the
  partial rect.
- **`compositor/src/input/synthetic.rs`** — `interrupt-close <id>` (alias
  `reopen <id>`); parse test.
- **`compositor/tests/window_conformance.rs`** — `CompositorProcess` now pipes
  stdout and parses the SIGUSR1 render stats. New
  `close_reverses_mid_flight_and_the_idle_trace_stays_flat` (22/22): asserts
  the close releases focus, the reverse starts strictly between the Dock tile
  and the geometry (no jump), no `closed` while the ghost is live, the reverse
  resolves back to the window, a later real close broadcasts `closed` exactly
  once, and `frames_rendered`/`animation_frames_stepped` are flat after the
  loop.
- **`scripts/capture-motion.sh`** (new) + `capture-demo-driver.py` (`--prefix`,
  `--reduced`) — runs the nested lifecycle walkthrough twice (normal and
  `accessibility.reduceMotion`) and writes `docs/captures/t02-lifecycle-motion*`
  stills plus `t02-lifecycle-motion.mp4` and
  `t02-lifecycle-motion-reduced.mp4`.
- **Docs** — ADR `0008-close-interruption-and-ghost-focus.md`;
  `docs/design/02-compositor.md` "Window lifecycle motion".

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 177 bin unit + all suites green
  (window_conformance 22/22).
- `make clippy`; `make fmt-check` — clean.
- `make e2e` — green (headless demo clean teardown).
- `SETTLE=8 bash scripts/capture-motion.sh` — needs a host Wayland session,
  spectacle, ffmpeg and Pillow (not CI).

Gotchas for later tasks:

- A close ghost **drops focus by hand**; do not rely on `focus_changed(None)`.
  `after_workspace_change` still has this latent gap (it calls
  `set_focus(None)` but never mirrors `active_window`/`Unfocused`).
- The reverse goes through `begin_window_motion_from`, which reads the live
  close frame; keep using it so interruptions never jump.
- Never remove the replaced close motion's window: `step_window_motions`
  commits removal only while the motion kind is still `Close`.
- A well-behaved Wayland client destroys itself on `send_close`, so a live
  close cannot be visually interrupted in the demo; the reversal is drivable
  through the `interrupt-close` hook only when the client stays mapped.
- The T-02 capture is assembled from **settled stills** (no scriptable Wayland
  screen recorder in this environment); live motion is T-03.1a's frame trace.

## T14 — T-03.1a Nested idle trace and animation frame budget

**State: done.** The idle/animation frame trace is now a real instrument (a
direct `client_wakeups` counter, not an inference from the render count) and
the 60 s acceptance window is flat. Raw numbers from `bash
scripts/idle-trace.sh` (headless backend, `DF_IDLE_TRACE_SECS=60`; the raw log
is committed as `docs/captures/t03-idle-trace.txt`):

- **60.0 s idle window**: `frames_rendered=1` (flat, +0),
  `animation_frames_stepped=0` (flat, +0), `direct_scanouts=0` (flat, +0),
  `client_wakeups=0` (flat, +0); `frames_skipped_no_damage` 8 → 9 (one per
  `SIGUSR1`, expected).
- **animation frame budget**: `frames_rendered +20`,
  `animation_frames_stepped +20` (a 320 ms dummy animation at 16 ms/frame) —
  exactly one compositor frame per animation frame; `client_wakeups +0` (no
  window attached, so no callback batch).
- **after settle**: `frames_rendered=21`, `animation_frames_stepped=20`,
  `client_wakeups=0` — all flat.
- **Verdict: pass** for zero idle damage / zero client wakeups and
  one-frame-per-animation-frame. This is the headless trace the task's test
  plan names; the live nested/DRM runs are T-03.2/T-03.4.

What landed:

- **`compositor/src/state.rs`** — `RenderStats::client_wakeups`; the
  `dump_stats` render-stats line appends `client_wakeups=` (append-only
  positional contract, ADR 0009).
- **`compositor/src/render.rs`** — `post_repaint` / `post_repaint_headless`
  count one frame-callback batch per live window; new `count_output_windows`
  helper (the DRM frame callbacks are queued by `DrmCompositor` itself).
- **`compositor/src/backend/headless.rs`** — `post_repaint_headless` takes
  `&mut DfState` now (the counter lives on `state.stats`).
- **`compositor/src/backend/drm.rs`** — increments `client_wakeups` via
  `count_output_windows`, so the budget is measurable on the DRM rail.
- **`compositor/tests/idle_trace.rs`** — extended with the synthetic-input
  socket and a `DF_IDLE_TRACE_SECS` window (default 10 s in-suite): asserts
  `frames_rendered`/`animation_frames_stepped`/`direct_scanouts`/`client_wakeups`
  flat, drives `animate-dummy 320` and asserts `rendered == stepped`, then
  asserts flat after settle. Prints the raw line.
- **`Makefile`** — `idle-trace` target (`IDLE_TRACE_SECS ?= 60`);
  **`scripts/idle-trace.sh`** runs it and records
  `docs/captures/t03-idle-trace.txt`.
- **Docs** — ADR `0009-idle-trace-instrumentation.md`;
  `docs/design/02-compositor.md` "Idle and animation frame trace (T-03.1a)";
  `docs/captures/README.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `make idle-trace` — 60 s trace green (raw log above); `IDLE_TRACE_SECS=10`
  for a quick run.
- `make cargo-test`; `make clippy`; `make fmt-check`; `make e2e` — green.

Gotchas for later tasks:

- The render-stats line is a **positional, append-only** contract (ADR 0009).
  T-03.1b appends latency/scanout fields; never reorder or rename existing
  ones — tests and the capture log parse them positionally.
- `client_wakeups` counts frame-callback **batches offered to mapped
  windows**, not delivered `wl_callback.done` events; it is an upper bound and
  is `0` with no window attached. On DRM it is the
  `count_output_windows` bound until the `DrmCompositor` callback path is
  hooked (T-03.2).
- `post_repaint_headless` takes `&mut DfState` now; do not revert it to `&`.
- The in-suite idle window is 10 s for speed; the 60 s acceptance is
  `make idle-trace` / `scripts/idle-trace.sh`.

## T15 — T-03.1b Latency instrument and direct-scanout template

**State: done.** The input-to-photon latency instrument is real and the
direct-scanout counter template is in place. Raw nested numbers from
`bash scripts/latency-trace.sh` (host Wayland session, nested backend 60 Hz;
the raw report is committed as `docs/captures/t03-latency-nested.txt`):

- **nested latency**: n=50, min=3501 us, median=13878 us, p95=15223 us,
  max=15853 us; one 60 Hz frame = 16666 us → **pass** (max under one frame);
  skipped(no-damage)=0, dropped(stale)=0.
- **headless** (`compositor/tests/latency_trace.rs`, in `make e2e`): one
  sample of 102 us from a real Ctrl+Up through the input router, within one
  frame; `direct_scanouts` flat at 0 and the template's `considered` =
  rendered + skipped; the instrument is flat while idle.

What landed:

- **`compositor/src/instrument.rs`** (new) — `LatencyInstrument` (earliest
  input per presented frame, 250 ms stale guard, bounded recent ring,
  `within_one_frame`, `summary`) and `ScanoutCounter` (template:
  `read(&RenderStats)`, `advanced_since`, `engaged_since`, `report`); 5 unit
  tests.
- **`compositor/src/state.rs`** — `RenderStats.latency`; `dump_stats` appends
  `latency_us_last`/`latency_us_max`/`latency_samples`/`latency_dropped` to
  the render-stats line (after `client_wakeups`) and emits `latency stats` +
  `scanout stats`.
- **`compositor/src/input.rs`** — every input (except device hotplug) calls
  `note_input` before routing.
- **`compositor/src/render.rs`**, **`compositor/src/backend/drm.rs`** — the
  presenting frame calls `note_present` (nested/headless/DRM).
- **`compositor/src/input/synthetic.rs`** — `query latency`, `query scanout`.
- **`compositor/tests/latency_trace.rs`** (new); `Makefile` e2e list +
  `latency-trace` target; **`scripts/latency-trace.sh`** +
  **`scripts/latency-probe.py`**.
- **Docs** — ADR `0010-latency-and-scanout-instruments.md`;
  `docs/design/02-compositor.md` "Input-to-photon latency and direct scanout
  (T-03.1b)"; `docs/captures/README.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 182 bin unit + all suites green
  (latency_trace 1/1, idle_trace 1/1, window_conformance 22/22).
- `make e2e`; `make clippy`; `make fmt-check` — green.
- `LATENCY_SAMPLES=50 bash scripts/latency-trace.sh` — needs a host Wayland
  session and the built Qt/CMake tree (not CI).

Gotchas for later tasks:

- The render-stats line is **positional, append-only** (ADR 0009); the latency
  fields are appended after `client_wakeups`. Never reorder or rename.
- Latency is **input router → presented frame**, not display scanout/vblank.
  Nested is winit `submit`, DRM is `queue_frame` (still unexercised), headless
  is a synthetic present.
- A stale input (>250 ms before a frame) increments `latency_dropped`, not a
  sample; a rising drop count means inputs are not producing damage.
- The DRM call site in `backend/drm.rs` is wired but untested (no hardware);
  T-03.2/T-03.4 confirm the number and the `direct_scanouts` engagement under
  a fullscreen client.

## T16 — T-04.1a Real shadows

**State: done.** Elevation-token shadows exist on both sides of the process
boundary from one source. The compositor draws a soft drop shadow behind
every decorated (non-fullscreen) window; the QML `Shadow`/`AppWindow` consume
the same token group, so the two agree by construction (FR-2).

What landed:

- **Tokens** — new `component.elevation.{low,med,high,overlay}` in
  `design-system/tokens/tokens.json`, each resolving `blur`/`offsetY`/`layers`
  from `primitive.elevation.*` + `component.shadow.*`; regenerated
  `design-system/Theme.qml` and `compositor/src/design_tokens.rs`
  (`component::elevation::*`). `make check-tokens` green.
- **`compositor/src/window/shadow.rs`** (new) — `ShadowLevel`
  (`Low`/`Med`/`High`/`Overlay`), `ShadowSpec::spec(scheme)`,
  `shadow_layers` (the `Shadow.qml` layered-rectangle formula in logical px),
  `shadow_bounds`, `shadow_elements` (front-to-back solid elements, output
  origin + scale + `MotionFrame`). 7 unit tests. `ShadowLevel::High` is the
  floating/SSD window level; dark is the default scheme until T-08.
- **`compositor/src/render.rs`** — `window_shadow_render_elements(state,
  output, scale)`: shadows every non-fullscreen window (whole decorated rect
  via `WindowInsets::outset`) plus minimizing/closing ghosts, behind the
  window surfaces.
- **`compositor/src/window/decoration.rs`** — `WindowInsets::outset` (inverse
  of `inset`) and `motion_transform` extracted to a `pub(crate)` helper shared
  by the titlebar and the shadow; the titlebar output is byte-identical.
- **`compositor/src/backend/nested.rs`**, **`backend/drm.rs`** — shadow
  elements appended **after** `window_render_elements` (backmost in the
  front-to-back list) so they composite below their window. DRM is wired but
  untested (no hardware), like the rest of that rail.
- **QML** — `Shadow.qml` gained `level` (default `"high"`), deriving
  `blur`/`offset`/`layers` from `Theme.controls.elevation[level]`;
  `AppWindow.qml` gained `shadowLevel` (default `"high"`). Existing window /
  popup / menu geometry is unchanged (defaults resolve to the old values).
- **Gallery** — `GalleryContent.qml` `WindowPage` now shows the four
  elevation levels through `Shadow`; **only**
  `design-system/gallery/snapshots/window_{light,dark,dark_reduced}.png`
  changed.
- **Tests** — `tst_design_system.qml`
  `test_shadow_geometry_comes_from_elevation_tokens`; `shadow.rs` geometry
  tests; `decoration.rs` `outset_is_the_inverse_of_inset`.
- **Docs** — ADR `0011-elevation-shadow-tokens.md`;
  `docs/design/02-compositor.md` "Window shadows (T-04.1a)";
  `docs/design/05-window-decorations.md`; `docs/design/10-design-system.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 190 bin unit + all suites green
  (window_conformance 22/22, idle_trace 1/1, latency_trace 1/1).
- `make qml-test` (tst_design_system + all ctest targets), `make clippy`,
  `make fmt-check`, `make check-tokens`, `make check-design-tokens` — green.
- `./scripts/check-gallery-snapshots.py --strict` — 66/66 (the committed
  goldens, with only the 3 `window_*` files updated).

Gotchas for later tasks:

- **Shadow geometry is token-only.** Do not hardcode a blur/offset/layer in
  either side; add/extend `component.elevation` in `tokens.json` and
  regenerate. ADR 0011 is the contract.
- The compositor shadow is **rectangular and unblurred** (rounded corners and
  blur are T-04.1b/T-04.2). It is a stack of translucent
  `SolidColorRenderElement`s, not a shader; `Shadow.qml` remains the app-side
  fallback.
- Shadow elements must stay **after** `window_render_elements` in the
  per-backend front-to-back list (last = backmost). Moving them before the
  window surface draws the shadow over the window.
- `motion_transform` (in `decoration.rs`) now owns the lifecycle transform for
  both chrome and shadows; reuse it rather than re-deriving scale/offset. Its
  signature is `(rect, color, target, Option<MotionFrame>)`.
- The gallery goldens in this tree: `make gallery-snapshot` rewrites **all**
  66 and shows every page as modified (the committed bytes differ from a fresh
  render by <1%; `--strict` passes). To update goldens for a change, render to
  a temp dir and copy only the affected `*.png` (see
  `DRAGONFRUIT_GALLERY_SNAPSHOT` in `gallery/main.cpp`), or accept the churn.
- `AppWindow.shadowLevel`/`Shadow.level` default to `"high"`; changing the
  default changes the window goldens.

## T17 — T-04.1b Rounded-corner clipping

**State: done.** Rounded corners are a token-derived geometry mask on the
compositor side. The SSD titlebar clips its top corners and every shadow layer
rounds, both from the generated tokens and matching the QML
`TitleBar`/`Shadow`; the client surface's own pixels are clipped by the T-04.3
reusable pass (see deviations). All headless/geometry tests, e2e, QML tests,
and the gallery strict goldens are green.

What landed:

- **`compositor/src/window/corner.rs`** (new) — `CornerMask`
  (`window()`/`titlebar()`), `RoundedCorners` (`All`/`Top`/`Bottom`),
  `rounded_rect_spans` (non-overlapping horizontal spans; a rounded rect for
  the flat renderer), `corner_squares` (the four radius×radius cutouts), and
  the consts `WINDOW_RADIUS`/`TITLEBAR_RADIUS`. 7 unit tests: span tiling (one
  span per row, in-bounds), subset-area, monotonically widening corner rows,
  titlebar bottom stays square, cutout squares equal the token radius, and
  zero/tiny/degenerate clamps.
- **`compositor/src/window/decoration.rs`** — `TitlebarElement::render_elements`
  draws the chrome fill as `CornerMask::titlebar()` spans (top corners rounded
  to `component.titlebar.cornerRadius`, bottom square), the QML `TitleBar`
  decomposition exactly. Tests updated for the multi-span fill.
- **`compositor/src/window/shadow.rs`** — `ShadowSpec.radius` (window token);
  `ShadowLayer.radius = window.radius + blur * spread`; `shadow_elements`
  emits the rounded spans per layer, matching `Shadow.qml`'s per-layer
  `radius: root.radius + root.blur * spread`. Tests rewritten around the
  merged element bounds (the rounded union equals `shadow_bounds`).
- **`compositor/src/window/mod.rs`** — exports (with `#[allow(unused_imports)]`
  for T-04.2/T-04.3).
- **Docs** — ADR `0012-rounded-corner-mask.md`; `docs/design/02-compositor.md`
  "Rounded-corner clipping (T-04.1b)"; `docs/design/05-window-decorations.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 197 bin unit tests + all suites green
  (window_conformance 22/22, idle_trace 1/1, latency_trace 1/1).
- `make e2e`; `make qml-test` (15/15); `make clippy`; `make fmt-check`;
  `make check-tokens`; `make check-design-tokens` — green.
- `./scripts/check-gallery-snapshots.py --strict` — 66/66 (QML untouched, so
  the goldens are byte-identical).

Gotchas for later tasks:

- **Radii are token-only.** `WINDOW_RADIUS`/`TITLEBAR_RADIUS` come from
  `component.window.radius` / `component.titlebar.cornerRadius`; do not
  hardcode a radius. ADR 0012 is the contract.
- `rounded_rect_spans` emits ~`2*radius + 1` non-overlapping spans per rounded
  rect. The shadow is now ~590 solid elements for one `high` window at scale 1;
  T-04.4a's degrade tier should shrink `CornerMask.radius`/layers rather than
  add a new shape model.
- **Client surfaces are not clipped per-window here.** Smithay owns the surface
  elements and keys them by `Id` for presentation feedback; per-window cropping
  would duplicate ids and risk frame-callback/direct-scanout. T-04.3's reusable
  pass owns live-surface `clip`; consume `CornerMask`. Third-party windows keep
  square *client* bottom corners until then; first-party QML windows round
  themselves.
- `ShadowLayer.radius` and `ShadowSpec.radius` are `f32`; `shadow_elements`
  rounds to `i32` only at span generation.
- The titlebar fill is now multiple elements (spans); tests that assumed a
  single backmost fill element were updated (`titlebar_fill_count`).

## T18 — T-04.2 Backdrop blur pass

**State: done.** The chrome material pass exists and is token-driven, applied
at most once per `(frame, output)` on the real backends. The blur itself is a
**flat-tone approximation** — a stack of translucent rounded feather layers
drawn below the chrome Wayland surface — not a texture-sampling gaussian; the
flat solid-color renderer has no sampler/offscreen path yet. Same
approximation status as the T-04.1a shadow; the real sampler is swappable
behind the tokens the pass already reads. ADR 0013 is the contract.

What landed:

- **`compositor/src/window/backdrop.rs`** (new) — `MaterialRole`
  (`Chrome`/`Popup` from `df_shell.layer`), `BackdropSpec` (per-scheme
  `material.{chrome,popup}Blur`/`Opacity` + `component.{menu_bar,popup}.radius`),
  `backdrop_layers`/`backdrop_bounds`/`backdrop_elements` (rounded via
  `CornerMask`), and `BackdropPass` (once-per-`(frame, output)` guard with
  `applications`/`skipped`/`damage`). 8 unit tests.
- **`compositor/src/render.rs`** — `chrome_backdrop_render_elements(state,
  output, scale)`: union of the top/overlay chrome bands (output-local logical)
  is the pass damage; emits the frosted elements.
- **`compositor/src/state.rs`** — `DfState.material_pass`, `render_serial`,
  `begin_render_frame()`; `dump_stats` prints
  `material stats: backdrop_passes=… backdrop_skipped=…` (`skipped` must stay 0).
- **`compositor/src/shell/mod.rs`** — `ChromeSurface.geometry` (output-local
  logical rect; `location` unchanged).
- **`backend/nested.rs`** — `begin_render_frame()` + backdrop elements appended
  after `chrome_render_elements` (below chrome, above windows).
  **`backend/drm.rs`** — same wiring, untested (no hardware); that rail's
  pre-existing window-before-chrome order is noted inline.
- **Docs** — ADR `0013-backdrop-blur-pass.md`; `02-compositor.md` "Backdrop
  blur pass (T-04.2)"; `10-design-system.md` material note.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 205 bin unit tests + all suites
  green. Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- `make e2e`; `make qml-test` (15/15); `make clippy`; `make fmt-check`;
  `make check-tokens`; `make check-design-tokens` — green.
- `./scripts/check-gallery-snapshots.py --strict` — 66/66 (QML untouched).

Gotchas for later tasks:

- **Tokens only** for blur/opacity/radius. ADR 0013 is the contract.
- The backdrop is **not a live-scene sample**: the legacy FR-1 "video under the
  bar keeps updating" bar needs a real sampler that replaces `backdrop_layers`;
  keep the `BackdropPass` contract.
- `BackdropPass` keys on `(render_serial, output)`; every new backend/render
  path must call `DfState::begin_render_frame()` once per rendered frame.
- Element order: backdrop goes **after** chrome surfaces and **before** the
  windows in the front-to-back list.
- Feather layers never extend past the chrome band (asserted); the pass never
  forces a redraw, so idle stays zero-damage (`shell_idle_trace` green).

## T19 — T-04.3 Reusable scene-transform pass

**State: done.** One reusable scene transform exists (scale/translate + optional
token `CornerMask` clip + optional token `BackdropSpec` blur) and is exercised
by the window lifecycle render path and by unit tests. One effect pass per
frame is guarded by the shared `FramePass`; the pass is instrumented on the
render-stats line and the idle trace asserts it flat. ADR 0014 is the contract.

What landed:

- **`compositor/src/window/pass.rs`** (new) — `FramePass`: the single shared
  once-per-frame/per-output guard + damage recorder. 2 unit tests.
- **`compositor/src/window/scene_transform.rs`** (new) — `SceneTransform`
  (`source`→`target` scale/translate; `clip: Option<CornerMask>`; `blur:
  Option<BackdropSpec>`; `new`/`identity`/`from_motion`/`with_clip`/`with_blur`/
  `scale`/`offset`/`map_point`/`map_rect`/`clip_spans`/`blur_layers`/
  `blur_elements`/`is_identity`) and `SceneTransformPass`. `from_motion` uses
  `MotionFrame::scale_for(surface_size)` (committed-surface scale), **not**
  `frame.scale`. 7 unit tests.
- **`compositor/src/render.rs`** — `push_motion_elements` wraps through
  `SceneTransform::from_motion` (same `Rescale`+`Relocate` pair, no behavior
  change); `window_render_elements` now takes `&mut DfState` and opens the pass
  once per output with the union of transformed window rects (output-local
  logical), via `merge_region`.
- **`compositor/src/state.rs`** — `DfState.scene_pass`; `begin_render_frame`
  opens it with `render_serial`; render-stats line appends `scene_transforms`
  and `scene_transform_skipped` after the T-03.1b latency fields (ADR 0009
  append-only).
- **`compositor/src/window/backdrop.rs`** — `BackdropPass` re-expressed on
  `FramePass`; public API and tests unchanged.
- **`compositor/tests/idle_trace.rs`** — parses the two new fields; asserts the
  transform pass is flat while idle and the skip guard stays `0`.
- **Docs** — ADR `0014-reusable-scene-transform-pass.md`; `02-compositor.md`
  "Reusable scene-transform pass (T-04.3)".

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — 214
  unit tests green (9 new).
- `make e2e` green (window_conformance 22/22, shell_protocol_conformance 32/32,
  idle_trace, shell_idle_trace, milestone_e2e, xwayland_conformance,
  latency_trace, animation_clock, protocol_surface + scripted demo).
- `make lint` green (workspace clippy `-D warnings`, qmllint, qml tests 15/15,
  token/design-token/desktop-name/capture-grab gates);
  `./scripts/check-gallery-snapshots.py --strict` 66/66.

Gotchas for later tasks:

- **T-05/T-06/T-11 consume `SceneTransform`/`SceneTransformPass`; do not add a
  second transform.** Future effects wrap `FramePass`, not a new guard.
- T-05's grid builds the transform from its layout rects; the lifecycle motion
  already renders through the same `from_motion` mapping.
- **Clip is geometry only right now**: the transform carries the `CornerMask`,
  but live client surface elements are still not cropped per window (Smithay
  keys them for presentation feedback; T-04.1b's limitation). T-05 owns the
  element wrap (`CropRenderElement`/`constrain_render_elements` exist in
  Smithay 0.7).
- Headless never renders, so `scene_transforms` stays `0` there; the counters
  move only on nested/DRM renders.
- Folding the T-04.2 chrome backdrop element generation into this pass is still
  open; the guard is already shared, so only the element builder moves.

## T20 — T-04.4a Material degrade tiers and instrumentation

**State: done.** One ordered material-quality ladder (`Full` → `Reduced` →
`Minimal`) and one budget selector exist; the backdrop and shadow passes render
through the selected tier, a tier can be pinned over the synthetic harness, and
the selection is reported on a new `degrade stats` line. The window lifecycle
motion is unchanged by the tier (it changes material geometry, not the scene
mapping). ADR 0015 is the contract.

What landed:

- **`compositor/src/window/degrade.rs`** (new) — `DegradeTier` (`Full`
  unchanged; `Reduced` halves radius/blur/layers; `Minimal` backdrop blur
  **off**, small tight shadow), `DegradeTier::{backdrop,shadow}` pure geometry
  scaling (opacity/tone/offset preserved, layers never below 1),
  `DegradeController` (EMA + hysteresis: downgrade after 12 over-budget frames,
  recover after 180 clearly-under-budget frames; `force`/`release`;
  `budget_us` default `animation::FRAME_INTERVAL`=16000, env
  `DRAGONFRUIT_FRAME_BUDGET_US`), and `summary()`. 9 unit tests.
- **`compositor/src/render.rs`** — `chrome_backdrop_render_elements` maps each
  `MaterialRole` spec through `state.degrade.tier()`; at `Minimal` it emits no
  elements and records **no** backdrop damage/application.
  `window_shadow_render_elements` maps the `ShadowLevel::High` spec through the
  tier.
- **`compositor/src/state.rs`** — `DfState.degrade`; `set_degrade_tier` (pins +
  requests a redraw), `set_degrade_budget_us`; `dump_stats` prints
  `degrade stats: tier=… forced=… budget_us=… samples=… over_budget=…
  downgrades=… upgrades=… tiers=full:…,reduced:…,minimal:…`.
- **`compositor/src/session.rs`** — each rendered frame's duration is fed to
  `degrade.observe` (the same `Duration` `RenderStats` records); inert while
  idle, never forces a redraw.
- **`compositor/src/input/synthetic.rs`** — `query degrade`,
  `set degrade-tier full|reduced|minimal`, `set degrade-budget <us>`.
- **`compositor/tests/window_conformance.rs`** — new
  `material_degrade_tiers_select_and_transitions_stay_correct`: default/forced
  tiers round-trip, the budget is selectable, and an appear/minimize/restore
  runs at `Minimal` with the same origin/target geometry and counted frames.
- **Docs** — ADR `0015-material-degrade-tiers.md`; `02-compositor.md`
  "Material degrade tiers (T-04.4a)".

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 223 bin unit tests + all suites
  green (window_conformance now 23/23). Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- `make e2e` green (exit log shows
  `degrade stats (exit): tier=full forced=0 budget_us=16000 samples=33 …`);
  `make lint` green; `make fmt-check`; `make clippy` — green.

Gotchas for later tasks:

- **T-05.1b/T-05.6 consume `DegradeTier`/`DegradeController`; do not add a
  second degrade ladder.** Route a new material's token spec through
  `DegradeTier` and read `DfState::degrade.tier()`.
- `Minimal` is *blur off*, not shadow off: `DegradeTier::shadow` keeps a small
  tight ring so windows stay legible. `DegradeTier::backdrop` returns `None` at
  `Minimal`, so the backdrop pass owns no damage there.
- The controller is **fed by rendered-frame durations** (`session.rs`); it is
  flat and zero-sample on headless (nothing renders), so only nested/DRM move
  it. `force()` pins a tier for deterministic tests.
- `budget_us` default is 16000 (the shared `FRAME_INTERVAL`); override with
  `DRAGONFRUIT_FRAME_BUDGET_US` or `set degrade-budget`.
- The degrade tier does **not** touch the T-04.3 `SceneTransform`/`FramePass`
  guards or the lifecycle mapping; keep it that way (T-02 correctness at every
  tier).

## T21 — T-04.4b Light/dark, reduced motion, and sign-off package

**State: done.** One live `ColorScheme` on `DfState` now drives every
compositor-drawn material (SSD titlebar, window menu, chrome backdrop, window
shadow); both schemes and reduced motion are exercised headless and captured
nested against the gallery goldens. ADR 0016 is the contract.

What landed:

- **`compositor/src/window/decoration.rs`** — `ColorScheme::name`/`parse`
  (`light`/`dark`), the settings spelling T-08 will mirror. 1 unit test.
- **`compositor/src/state.rs`** — `DfState.color_scheme` (default dark);
  `set_color_scheme` (marks the output dirty); `titlebar_element` /
  `titlebar_element_for_motion` stamp it onto the element; `open_window_menu`
  passes it; `dump_stats` emits `scheme stats (label): scheme=…`.
- **`compositor/src/render.rs`** — `chrome_backdrop_render_elements` and
  `window_shadow_render_elements` read `state.color_scheme` (was
  `ColorScheme::default()`); `ColorScheme` import removed there.
- **`compositor/src/input/synthetic.rs`** — `query material` (scheme plus the
  resolved chrome/elevated/border/accent tones) and
  `set color-scheme light|dark`; wire-format docs and parse tests.
- **`compositor/tests/window_conformance.rs`** — new
  `material_schemes_select_and_reduced_motion_stays_single_frame`: dark default,
  light token round-trip, a full appear at light, and a reduced-motion minimize
  at dark with the same committed geometry.
- **Docs** — ADR `0016-live-color-scheme-owner.md`; `02-compositor.md`
  "Light/dark and reduced motion (T-04.4b)".
- **Captures** — `scripts/capture-materials.sh` (new) + `--scheme/--tier/
  --materials-only` in `scripts/capture-demo-driver.py`; artifacts in
  `docs/captures/t04-materials*` (dark, light, dark+reduced, and a
  pre-material baseline). Review sheets:
  `t04-materials-gallery-side-by-side.png` (compositor SSD titlebar beside
  `ssd_light`/`ssd_dark`) and `t04-materials-before-after*.png` (degrade
  `minimal` blur-off vs full).

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — 224 bin unit tests + all suites
  green (`window_conformance` now 24/24). Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- `./scripts/check-gallery-snapshots.py --strict` — 66/66 snapshots green
  (light, dark, dark+reduced are already in the gallery gate's `SCHEMES`).
- `bash scripts/capture-materials.sh` — needs the host Wayland session
  (`WAYLAND_DISPLAY`), `spectacle`, `ffmpeg`, Pillow; ran green and wrote the
  `docs/captures/t04-materials*` set.

Gotchas for later tasks:

- **T-08 owns the live scheme**: mirror `appearance.colorScheme` into
  `DfState::set_color_scheme`. Do not add a second scheme field or a second
  token source. `ColorScheme::name`/`parse` are the wire spelling.
- **The nested desktop background is scheme-dependent** (it is light under
  `light`, dark under `dark`). `scripts/capture-demo-driver.py` therefore
  detects the nested window under dark first, then applies the requested
  scheme; `WALL`/`WALL_TOL` and the row-run detector in `detect_rect` are
  tuned to that. If the background token changes, update `WALL`.
- Shell-rendered chrome (Dock/menu-bar QML) does **not** follow the
  compositor scheme — the shell is not notified. Only compositor-drawn
  materials (SSD titlebar, window menu, backdrop, shadow) switch, which is why
  `t04-materials-*-dock.png` is identical across schemes while the titlebar
  crop differs.
- `query material` reports the resolved tones but no wallpaper; the compositor
  `scheme stats` line is the human-readable mirror.
- Captures are nested-only; CI (`make e2e`) never renders, so the scheme
  conformance test uses `query material` + lifecycle geometry instead of pixels.

## T22 — T-05.1a Live-surface transform into the grid

**State: done.** Mission Control now transforms the **live** window surfaces
into the documented grid through the reusable T-04 `SceneTransform`/`MotionFrame`
— never a thumbnail. The transform is render-time only: committed geometry,
focus, and Space assignment are unchanged (hit-testing ownership is T-05.2).
ADR 0017 is the contract.

What landed:

- **`compositor/src/overview/grid.rs`** (new) — `grid_layout` (candidates
  ordered by active Space → strip → recency; near-square columns starting at
  `ceil(sqrt(n))` and adjusted between `floor`/`ceil(sqrt(n))` toward the
  output aspect; **one uniform scale** so the largest window fits its cell
  minus `GRID_MARGIN` (= `component.overview.stripMargin`); centered;
  non-overlapping), `GridCandidate`/`GridPlacement`/`GridLayout`,
  `GridPlacement::frame(progress)` (the `MotionFrame` lerp from committed
  `source` to grid `target`, alpha 1.0), and `MIN_GRID_SCALE` (exposed for the
  future pager, not enforced). 7 unit tests.
- **`compositor/src/state.rs`** — `overview_grid_progress` (`None` closed;
  `p` while opening, `1-p` while closing, `1.0` settled), `grid_candidates`
  (visible, non-closing, non-fullscreen active-Space windows, recency order),
  `overview_grid_layout(output) -> Option<(f64, GridLayout)>`,
  `overview_grid_frame(window)`, and `window_render_frame(window, now)`
  (grid frame while shown, else lifecycle motion).
- **`compositor/src/render.rs`** — `window_render_elements`,
  `window_shadow_render_elements`, and `titlebar_render_elements` all use
  `state.window_render_frame`, so the client surface, SSD titlebar, and shadow
  carry the same grid mapping (the T-04.3 one-transform invariant).
- **`compositor/src/window/mod.rs`** — `WindowModel::window_by_id` (project the
  recency-ordered grid membership back onto live `Window` handles).
- **`compositor/src/input/synthetic.rs`** — `query grid` (wire format line
  added): `grid none`, else `grid progress=<t>` + one `grid output …` and one
  `grid window <id> <source> <target> <cell>` line per placed live surface,
  then `end`. The `target` is the **interpolated** frame at the current
  progress (equals the cell target once settled).
- **`compositor/tests/window_conformance.rs`** — new
  `overview_grid_transforms_live_surfaces_into_the_grid`: no grid before the
  overview; 3 live surfaces; mid-gesture the transform interpolates; settled
  one uniform scale, centered non-overlapping cells from each committed
  200x150 rect, all inside the output; the surfaces stay mapped; a second
  toggle reverses to no grid. (`parse_grid`/`wait_for_grid` helpers.)
- **Docs** — ADR `0017-overview-grid-render-transform.md`; `02-compositor.md`
  "Mission Control live-surface grid (T-05.1a)".

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — **231** bin unit tests + all suites
  green (`window_conformance` now 25/25). Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- The grid is headless-asserted via `query grid`; no nested capture was added
  (T-05.1b/T-05.6 own the visual capture and frame budget).

Gotchas for later tasks:

- **Reuse the one transform**: `window_render_frame` is the seam. Do not add a
  second grid/transform; route T-05.2 hit-testing through
  `DfState::overview_grid_layout` (transformed on-screen rects) and T-05.3 drag
  through the same `GridPlacement`.
- `overview_grid_progress` returns the *effective* grid progress (closing is
  reversed), so callers do not special-case direction. It is `None` once
  settled-closed; a settled-open overview is `Some(1.0)`.
- `grid_candidates` currently includes **only the active Space**; neighbor-Space
  reveal needs the renderer to draw surfaces that are unmapped from the
  `Space` and is a later T-05 slice. `GridCandidate::space_index` already
  models it.
- Paging at `MIN_GRID_SCALE` and per-output chrome insets are **not** wired;
  the grid uses the full output geometry and shrinks freely.
- The renderer is flat solid-color, so the grid is geometry-asserted, not
  pixel-asserted; `query grid` is the deterministic seam.

## T23 — T-05.1b Live video at scale and degrade

**State: done.** A committing "video" client keeps advancing at the reduced
grid scale and the T-04.4a material degrade tier is selectable and applied to
the Mission Control grid. The transform is render-time only, so the surface
stays mapped and keeps its frame callbacks while scaled; the grid never
resolves its own material.

What landed:

- **`compositor/src/overview/grid.rs`** — `GridMaterial { tier, shadow, blur }`
  and `GridMaterial::resolve(tier, scheme)`: the high-elevation token shadow
  and the material blur mapped through the active degrade tier. 2 unit tests.
- **`compositor/src/state.rs`** — `DfState::overview_grid_material()` (`None`
  when the grid is closed): the one accessor the renderer and `query grid` both
  use.
- **`compositor/src/render.rs`** — `window_shadow_render_elements` resolves its
  shadow through `overview_grid_material` while the overview is open, else the
  global tier.
- **`compositor/src/input/synthetic.rs`** — `query grid` gained
  `grid material tier=<name> blur=<0|1> shadow_layers=<n> shadow_radius=<r>
  shadow_opacity=<o>` (wire format documented). `minimal` → `blur=0`.
- **`compositor/tests/window_conformance.rs`** — new
  `overview_grid_keeps_live_video_advancing_and_applies_degrade_tier`: three
  fresh buffer commits advance `frames_rendered` while the grid is settled, the
  grid still lists the one live surface at its committed geometry, and
  `set degrade-tier reduced|minimal|full` moves the reported grid material
  (blur flag, shadow layers/radius) with the surface staying mapped.
  `parse_grid` parses the material line.
- **Docs** — `02-compositor.md` "Live video at scale and the grid material
  (T-05.1b)". No new ADR (extends ADR 0015/0017).

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — 233
  bin unit tests green (`overview::grid` now 9). Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- `cargo test -p dragonfruit-compositor --test window_conformance` — 26/26
  green.

Gotchas for later tasks:

- **The grid material is the degrade seam.** Read
  `DfState::overview_grid_material`; do not add a second grid shadow/blur
  resolver in a T-05 slice. `blur` is the tier's blur state (the overview
  chrome backdrop also reads it), not a per-window texture blur.
- **T-05.2 hit-testing** should use `overview_grid_layout`'s transformed rects
  and must not unmap surfaces: the live-video test asserts the surface stays in
  `query decorations` across commits and tier changes (a thumbnail substitution
  would break it).
- The reusable `SceneTransform`'s `clip`/`blur` attachments are still **not**
  composed onto third-party client surface elements (the renderer has no clip
  element); the grid material stays in the solid-fill pass. The T-04.3 clip
  follow-up remains open.
- Headless never renders, so the tier's grid effect is asserted through
  `query grid` (`grid material …`) and `frames_rendered`; the pixel capture is
  T-05.6.

## T24 — T-05.2 Hit-testing and selection on live representations

**State: done.** A left click on a live Mission Control representation
hit-tests the **interpolated** grid transform (never the committed geometry)
and routes through the existing selection round-trip: the choice is recorded,
the overview leaves, the window's Space activates, and the window is raised and
focused. The selected window is the real surface and stays mapped.

What landed:

- **`compositor/src/overview/grid.rs`** — `GridLayout::window_at(point,
  progress)`: finds the placement whose `GridPlacement::frame(progress).rect`
  (the renderer's exact rect) contains the point; placements are
  most-recently-used first, so the MRU/topmost window wins overlapping hits.
  3 new unit tests. No new transform.
- **`compositor/src/state.rs`** — `DfState::overview_window_at(point)` (output
  under the point + its interpolated grid layout) and
  `DfState::select_overview_window(id)` (the one round-trip, shared with the
  shell request).
- **`compositor/src/shell/mod.rs`** — `select_overview_toplevel` now calls
  `select_overview_window`, so keyboard and pointer selection share one path.
- **`compositor/src/input.rs`** — on `ButtonState::Pressed`, inside the
  no-chrome branch, a left press while `InputOwner::Overview` hit-tests the
  grid first and returns on a hit; titlebar/menu handling is never reached
  with committed geometry in the overview.
- **`compositor/tests/window_conformance.rs`** — new
  `overview_click_selects_and_focuses_the_live_representation`.
- **Docs** — `02-compositor.md` "Pointer selection on live representations
  (T-05.2)". No new ADR.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — **235**
  bin unit tests green (`overview::grid` now 14).
- `cargo test -p dragonfruit-compositor --test window_conformance` — **27/27**
  green. Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` clean;
  `cargo fmt -p dragonfruit-compositor -- --check` clean.

Gotchas for later tasks:

- **One hit-test seam**: `overview_window_at` / `GridLayout::window_at`. T-05.3
  drag must reuse `GridPlacement`; do not add a second transform or hit test.
- Selection is immediate (no reverse animation), matching the existing
  `select_overview_toplevel` round-trip. `overview_grid_progress` returns `None`
  the moment `overview.select` clears `overview_active`, so the grid is gone on
  the next frame.
- The test picks a representation whose `target` center is outside its own
  `source` rect, proving the transformed hit (a committed-geometry hit would
  not fire that focus). The pixel/nested capture is T-05.6.
- Neighbour-Space reveal and per-output insets remain unwired; `window_at`
  covers only the placements `overview_grid_layout` produces (active Space).

## T25 — T-05.3 Drag a live representation between Spaces

**State: done.** Pressing on a live Mission Control representation and dragging
it onto another Space's workspace-strip card moves the window there through the
one `move_to_workspace` primitive. A press that never crosses the drag
threshold is still the T-05.2 selection. The overview stays open after a drop
and every live surface stays mapped.

What landed:

- **`compositor/src/overview/grid.rs`** — `GridDrag { window, start, current }`
  with `moved()` / `offset()`, `DRAG_THRESHOLD = 8.0`, and the pure strip
  geometry `strip_card_rect` / `strip_space_at` (the shell's centered layout
  reproduced from `component::overview` + `component::menu_bar` tokens,
  `STRIP_TOP = 28 + 16`). 3 new unit tests (`overview::grid` now 17). No new
  transform — reuses `GridPlacement`.
- **`compositor/src/state.rs`** — `DfState::grid_drag` +
  `overview_begin_drag` / `overview_update_drag` / `overview_end_drag` /
  `overview_drag` / `overview_drop_space_at`. `overview_grid_frame` applies the
  drag offset so the live surface, SSD titlebar, and shadow follow the pointer.
  `overview_end_drag` reuses `select_overview_window` (click) and
  `move_window_to_space` (drop).
- **`compositor/src/input.rs`** — left press in `InputOwner::Overview` begins
  the drag; `PointerMotion` updates it; a left release ends it.
- **`compositor/src/input/synthetic.rs`** — `query grid` gained
  `grid drag <window> <x> <y> moved=<0|1> target=<index|-1>`; new
  `query spaces` (`space <output> <index> <id> active=<0|1> windows=<n>`).
  Parser + unit test updated.
- **`compositor/tests/window_conformance.rs`** — new
  `overview_drag_moves_the_live_representation_between_spaces` (28/28
  conformance green): press a live target, drag to the Space-1 card
  (`(640, 86)` on 1280x720), release, assert it left the active grid, Space 1
  holds it, the decoration's `space` equals Space 1's id, the overview stayed
  open, and all three surfaces stayed mapped.
- **Docs** — `02-compositor.md` "Dragging a live representation between Spaces
  (T-05.3)"; new ADR `0018-overview-drag-drop-target.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — **238**
  bin unit tests green.
- `cargo test -p dragonfruit-compositor --test window_conformance` — **28/28**
  green. Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` clean;
  `cargo fmt -p dragonfruit-compositor -- --check` clean.

Gotchas for later tasks:

- **One drag seam**: `overview_begin/update/end_drag` in `DfState`; the pure
  geometry (`GridDrag`, `strip_*`) in `overview::grid`. Do not add a second
  transform or hit test. Neighbour-Space reveal only widens
  `overview_drop_space_at`.
- The drop target is the **strip card under the pointer**, reproduced from the
  shared tokens; a drop off the strip or on the source Space is cancelled.
  Multi-monitor drag edges stay T-16.
- In a nested session the shell's transparent overview surface still captures
  pointer input, so the shell title-card drag remains the user path; the
  compositor drag is the same headless/test seam as T-05.2. Nested capture is
  T-05.6.
- `query spaces` reports `SpaceId.0` (what `query decorations`' `space` field
  carries), so tests can prove the destination index, not just that an
  assignment changed.

## T26 — T-05.4 Image wallpaper and per-Space slide

**State: done.** A Space's wallpaper can be an image (`source` + `fit`) decoded
once and cached, rendered behind the windows per Space, and slid horizontally
with its Space during a workspace switch. No image (or an undecodable source)
keeps the solid color fallback.

What landed:

- **`compositor/src/wallpaper.rs`** (new) — `sample_wallpaper` (pure
  fill/fit/stretch/center mapping: source crop + output-local dest),
  `WallpaperCache` (decode-once via the `image` crate into a Smithay
  `MemoryRenderBuffer`; misses are cached so a broken path is not retried per
  frame), `slide_offset` / `slide_slots` (the per-Space slide, **shared** with
  `DfState::apply_overview_scene`), `WallpaperSlot`. 8 unit tests.
- **`compositor/src/render.rs`** — `WallpaperRenderElement` (`Image` / `Solid`)
  and `wallpaper_render_elements`: image sampled to its fitted dest over a
  solid fallback, appended **last** (bottom-most) in each backend's
  front-to-back list.
- **`compositor/src/state.rs`** — `wallpaper_cache` field; `wallpaper_slide`;
  `wallpaper_slots`; `apply_overview_scene` refactored to `slide_offset`.
- **`compositor/src/backend/{nested,drm}.rs`** — `Wallpaper` element variant,
  appended after the shadows.
- **`compositor/src/input/synthetic.rs`** — new `query wallpaper` (slide
  direction/progress + one `wallpaper slot index= offset= fit= source=` line
  per drawn Space) + parser test.
- **`compositor/tests/window_conformance.rs`** — new
  `wallpaper_follows_the_active_space_and_slides_with_it` (29/29 green).
- **Dependency** — `image = 0.25.10`, `default-features = false`, features
  `png` + `jpeg`, pinned in the workspace deps.
- **Docs** — `02-compositor.md` "Image wallpaper and the per-Space slide
  (T-05.4)", `03-workspaces.md` wallpaper bullet, ADR
  `0019-image-wallpaper-decode-and-slide.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — **246**
  bin unit tests green (`wallpaper` 8).
- `cargo test -p dragonfruit-compositor --test window_conformance` — **29/29**
  green. Direct cargo needs
  `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` clean;
  `cargo fmt -p dragonfruit-compositor -- --check` clean.

Gotchas for later tasks:

- **One wallpaper seam**: `render::wallpaper_render_elements` +
  `WallpaperRenderElement`; the slide reuses `wallpaper::slide_offset` (do not
  add a second transform). Append the wallpaper **last** in the element list.
- The base clear color is still `wallpaper_color_for` (active Space fallback);
  the solid element duplicates it and covers the neighbour's letterbox mid-slide.
- `to_rgba8()` bytes are uploaded as `Fourcc::Abgr8888` (GL `RGBA8`); keep that
  pairing.
- `WallpaperCache` is per source path and caches misses. A replaced file at the
  same path needs `clear()`; `set_wallpaper` does **not** invalidate it yet
  (T-08/T-16).
- Nested capture of the live slide is T-05.6 (this session verified headlessly
  via `query wallpaper`).

## T27 — T-05.5 Desktop Reveal

**State: done.** Ctrl+Down routes `DesktopReveal` through the one overview
state machine and the one reusable scene transform. The live window surfaces
slide out of their nearest horizontal edge (exposing the compositor-drawn
wallpaper), a second trigger restores, and the reduced-motion variant fades
the surfaces in place instead of translating. Headless conformance covers the
full open/restore/reduced-motion path; the nested capture stays with T-05.6.

What landed:

- **`compositor/src/overview/reveal.rs`** (new) — `escape_rect` (the off-screen
  target past the nearest edge) and `reveal_frame(source, area, progress,
  reduced_motion)`. Reduced motion keeps the rect and fades `1 → 0`; full
  motion translates and stays opaque. 4 unit tests.
- **`compositor/src/overview/mod.rs`** — `OverviewMachine::desktop_reveal_progress`
  (mirrors `overview_grid_progress`: `1 - progress` while closing, `1.0`
  settled, `None` hidden). `input_owner` now keeps pointer hit-testing with the
  overview while `desktop_revealed`, so committed geometry is never hit-tested
  against the transformed scene. 2 new unit tests.
- **`compositor/src/state.rs`** — `DfState::desktop_reveal_progress` /
  `desktop_reveal_frame`; `window_render_frame` becomes
  `overview_grid_frame → desktop_reveal_frame → window_motion_frame`, so the
  surface, SSD titlebar, and shadow share the one mapping.
- **`compositor/src/input/synthetic.rs`** — new `query reveal`
  (`reveal none` or `reveal progress=<t> reduced=<0|1>` + one `reveal window
  <id> <sx> <sy> <sw> <sh> <tx> <ty> <tw> <th> <alpha>` line per live surface)
  + parser test.
- **`compositor/tests/window_conformance.rs`** — new
  `desktop_reveal_translates_live_surfaces_and_respects_reduced_motion`
  (30/30 conformance green): Ctrl+Down (`key 29`/`key 108`) reveals both live
  surfaces off-screen and opaque, they stay mapped, the wallpaper is still
  drawn, a second Ctrl+Down restores, and `set reduced-motion on` fades in
  place (`target == source`, `alpha == 0`).
- **Docs** — `02-compositor.md` "Desktop Reveal (T-05.5)"; `04-shell.md`
  desktop-background paragraph. No ADR: this reuses the T-04/T-05 pipeline and
  adds no new schema/library/protocol.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — **252** bin unit tests + every
  integration suite green (`window_conformance` 30/30, `shell_protocol` 32/32).
  Direct cargo needs `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`; the conformance tests need
  `XDG_RUNTIME_DIR`.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` clean;
  `cargo fmt -p dragonfruit-compositor -- --check` clean.

Gotchas for later tasks:

- **One reveal seam**: `overview::reveal::reveal_frame` is pure geometry;
  `DfState::desktop_reveal_frame` is the only place the scene resolves it, and
  `window_render_frame` is the only consumer. Do not add a second transform or
  a second progress function.
- The reveal uses the **render-time** transform (like the grid), not the
  `apply_overview_scene` `space.map_element` mutation the workspace slide uses.
  Do not route reveal through `apply_overview_scene`.
- `desktop_reveal_progress` reports `None` at rest and `Some(1.0)` settled;
  `input_owner` stays `Overview` while revealed, so a left press currently does
  nothing (no click-to-restore). Escape/click/timeout restore is *not*
  implemented; only a second Ctrl+Down (toggle) restores. Legacy T-14 listed
  those; a later task can widen `overview_end_drag`/press handling.
- `query reveal` iterates `windows.windows()` and skips a window whose
  `desktop_reveal_frame` is `None`; a window on an inactive Space is not drawn
  by the reveal.
- Nested capture of the live reveal + gesture frame trace is **T-05.6** (this
  session verified headlessly via `query reveal`).

## T28 — T-05.6 Overview frame budget and capture

**State: done.** The overview gesture now has its own **gesture-scoped** 60 Hz
frame-budget instrument, fed from the same per-rendered-frame seam as the T-03
trace and the T-04.4a degrade controller. The nested capture is scripted and
committed, and the honest shortfall on the dev iGPU is recorded (the T-04
ladder downgrades the grid material under pressure). `make e2e` and `make soak`
are green.

What landed (partial work from attempt 3 was already in `06bbd94`; this session
verified, completed and documented it):

- **`compositor/src/instrument.rs`** — `GestureBudgetTrace`: render-duration
  (over-budget) and presented-interval (dropped, `> 1.5×budget`) kept distinct;
  `held()` is the honest verdict; default budget `animation::FRAME_INTERVAL`
  (16 ms). 2 unit tests.
- **`compositor/src/state.rs`** — `DfState::gesture_trace`;
  `observe_rendered_frame(duration)` is the one per-frame seam (global trace +
  `degrade.observe` + gesture record/finish); `overview_frame_budget_active()`.
  `dump_stats` appends a `gesture budget` line.
- **`compositor/src/session.rs`** — the render loop now calls
  `state.observe_rendered_frame(frame_duration)` instead of the two separate
  calls.
- **`compositor/src/input/synthetic.rs`** — new `query gesture`
  (`gesture tracing=… frames=… over_budget=… dropped=… max_frame_us=…
  max_interval_ms=… budget_us=… held=… tier=<full|reduced|minimal>`) + parser
  test.
- **`compositor/tests/window_conformance.rs`** — new
  `overview_gesture_frame_budget_is_traced_and_reports_the_degrade_tier`
  (31/31 conformance green): idle is an empty trace; a full Ctrl+Up open/close
  produces a live trace, a sub-frame verdict, and reports the live tier as it
  is pinned minimal then full.
- **`scripts/capture-overview.sh`** + **`scripts/capture-overview-driver.py`**
  (new) — drive the nested overview (4-finger open, Ctrl+Down reveal, 3-finger
  Space swipe, reduced-motion run), screenshot with Spectacle, assemble an
  mp4 per mode, and write `query gesture` samples + session exit stats.
- **`docs/captures/`** — `t05-mission-control-live.{png,mp4}`,
  `-reveal.png`, `-slide.png`, `-slide-settled.png`, the `-reduced*` set, and
  `t05-gesture-budget-nested.txt`.
- **Docs** — `02-compositor.md` "Gesture frame budget and capture (T-05.6)";
  ADR `0020-gesture-frame-budget-trace.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `make e2e` — green (all compositor integration suites + headless demo).
- `make soak` — **100 clean cycles, zero strays**.
- `cargo test -p dragonfruit-compositor` — 254 bin unit tests + every
  integration suite green (`window_conformance` 31/31).
- `bash scripts/capture-overview.sh` — needs a host Wayland session, spectacle,
  ffmpeg and Python Pillow; not part of `make e2e`. At 4K the host screenshot is
  3840x2160 and the nested window is a 1920x1200 region; `detect_rect` finds it
  from the wallpaper clear color and crops.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` clean;
  `cargo fmt -p dragonfruit-compositor -- --check` clean.

Gotchas for later tasks:

- **One per-frame seam**: `DfState::observe_rendered_frame`. Do not feed the
  gesture trace or `degrade.observe` directly from a backend. A new overview
  transition must be covered by `overview_frame_budget_active` or it silently
  drops out of the trace (ADR 0020).
- The gesture trace auto-begins on the first recorded active frame and
  auto-finishes when the gesture settles; counters are retained after `finish`
  for the next `query gesture`.
- **Nested gesture shortfall is real**: e.g. `mission-control` →
  `over_budget=1 dropped=2 max_frame_us=18273 held=0`, settling at
  `tier=reduced`. That is the accepted "shortfall recorded" outcome, not a bug.
- **Known nested pointer gap**: the shell overview chrome is an offscreen
  scene-graph surface that owns pointer focus but does not forward it to QML, so
  a synthetic pointer neither begins the compositor's `overview_begin_drag`
  (bypassed by `chrome_under`) nor fires the QML card handlers. Pointer
  selection/drag stay headlessly verified; the nested path needs the shell
  overview input region/forwarding wired (T-05.2/T-05.3 polish or T-11). The
  capture therefore shows the scene *transforms*, not a pointer drag.
- The reduced-motion settled overview is pixel-identical to the normal settled
  overview (as expected); the difference is in the transition/clip, not the
  settled state.

## T29 — T-06.1 App-switcher state machine

**State: done.** The compositor now owns a real Cmd-Tab app switcher: open on
the chord, Tab/Shift+Tab and the arrows cycle/reverse, Command release commits
exactly once through `activate_window_id`, Escape cancels with no focus change.
The chord is resolved by the shortcut engine (not a client), the order is
`WindowModel` recency, and the private `app_switcher` event projects the one
machine. The shell overlay is T-06.2.

What landed:

- **`compositor/src/app_switcher.rs`** (new) — `AppSwitcher` pure machine
  (`open`/`step`/`commit`/`cancel`, `selected_app`/`selected_window`/
  `direction`), `SwitchStep`, `SwitcherApp { app_id, window }`. Open's first
  selection steps one app away from the focused app (wraps; a single app
  selects itself; empty list stays closed). 7 unit tests.
- **`compositor/src/state.rs`** — `DfState::app_switcher: AppSwitcher`;
  `app_switcher_entries` (one per app in recency order, most recent window),
  `focused_app_id`, `app_switcher_key`, `app_switcher_modifier` (commits on
  `!command_held`), `app_switcher_commit`, `app_switcher_cancel`.
- **`compositor/src/input.rs`** — the keyboard filter resolves Cmd+Tab /
  Cmd+Shift+Tab via the shortcut engine (`app_switcher_key`), consumes Tab /
  arrows / Escape while open, and calls `app_switcher_modifier(mods.logo)` on
  every key release.
- **`compositor/src/input/shortcuts.rs`** — added the Cmd+Shift+Tab →
  `AppSwitcher` binding.
- **`compositor/src/shell/mod.rs`** — removed the duplicate shell-side
  `AppSwitcherState`; `broadcast_app_switcher` projects `DfState::app_switcher`;
  `cycle_app_switcher` drives the same machine with the shell trigger.
- **`compositor/src/input/synthetic.rs`** — new `query switcher`
  (`switcher active=.. app=.. direction=.. selected=.. count=.. focus=..` + one
  `switcher app <i> <app_id> <window>` line per entry) + parser test.
- **`compositor/tests/window_conformance.rs`** — new
  `app_switcher_opens_cycles_commits_once_and_escape_cancels` (32/32 green).
- **Docs** — `02-compositor.md` "App-switcher state (T-06.1)"; `04-shell.md`
  app-switcher paragraph; ADR `0021-app-switcher-state-machine.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — **261** bin unit tests + every
  integration suite green (`window_conformance` 32/32, `shell_protocol` 32/32).
  Direct cargo needs `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig` and
  `RUSTFLAGS=-L ~/.local/df-devroot/lib64`; conformance needs `XDG_RUNTIME_DIR`.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` clean;
  `cargo fmt -p dragonfruit-compositor -- --check` clean.

Gotchas for later tasks:

- **One machine**: `DfState::app_switcher`. The shell must render the
  `app_switcher` projection and never re-derive recency or selection.
- Cycling never focuses; only commit activates (one `activate_window_id`:
  cross-Space, restore-if-minimized, focus).
- The machine snapshots the app list at open; a window closing mid-hold is not
  reaped until the next open. T-06.2 can widen this if the overlay needs live
  membership.
- The initial selection skips the focused app, but headless tests with two
  fresh windows have no focused window yet, so the first selection is the most
  recent app (integration test relies on this).
- Within-app Cmd+` cycling stays T-06.2b and must extend this machine, not add
  a second binding path.

## T30 — T-06.2a Switcher overlay and live previews

**State: done.** The Cmd-Tab overlay is live: the compositor scales the
**real** window surfaces of the recency entries through the existing T-04 grid
transform, and the shell draws the centered cards, scrim, selection highlight,
accessible names, and reduced-motion variant on a full-output `app-switcher`
overlay. The shell never re-derives recency — the compositor streams one
`app_switcher_entry` per app in a new additive event. Commit, Cmd+` cycling,
and interruptibility remain T-06.2b.

What landed:

- **Protocol** — `df_toplevel_manager.app_switcher_entry(index, app_id)` since 4,
  emitted between `app_switcher` and the batch `done`. Additive: appended last
  so `since` stays non-decreasing and pre-existing opcodes are unchanged.
- **`compositor/src/overview/switcher.rs`** (new) — `preview_area` / `CARD_STRIP`
  (from `component.overview` tokens); 2 unit tests.
- **`compositor/src/state.rs`** — `switcher_candidates`, `switcher_material`,
  `switcher_frame` (entry windows via `grid_layout` over `preview_area`; other
  visible windows `alpha = 0`), `switcher_preview_placements`; `window_render_frame`
  prefers the switcher frame.
- **`compositor/src/shell/mod.rs`** — emits the entry batch before `done`.
- **`compositor/src/render.rs`** — shadow material falls back to `switcher_material()`.
- **`compositor/src/input/synthetic.rs`** — `query switcher` appends
  `switcher preview <window> <x> <y> <w> <h> selected=<0|1>` lines.
- **`shell/src/shellprotocol.{h,cpp}`** — `createSwitcherSurface` /
  `commitSwitcherImage` / `hideSwitcher`, `switcherConfigured`,
  `appSwitcherChanged(active, entries, selectedAppId, direction)`; the entry
  batch accumulates until `onManagerDone`.
- **`shell/src/shellcontroller.{h,cpp}`** — fourth offscreen scene
  (`Dragonfruit.Switcher`), `onAppSwitcherChanged` resolves `selectedIndex` by
  matching the selected app id, `renderSwitcher`.
- **`shell/switcher/AppSwitcher.qml`** + CMake module; **`shell/tests/tst_switcher.*`**
  (8 QML cases).
- **Tests** — `window_conformance` parses/asserts the preview rects (32/32);
  `shell_protocol_conformance` records/asserts the entry batch (32/32).
- **`scripts/capture-switcher.{sh,driver.py}`** + `docs/captures/t06-app-switcher*.png`.
- **Docs** — `02-compositor.md` "App-switcher live previews (T-06.2a)";
  `04-shell.md`; ADR `0022-app-switcher-overlay-and-previews.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — **266** bin unit tests + all suites
  green. Needs `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig`,
  `RUSTFLAGS="-L ~/.local/df-devroot/lib64"`, `XDG_RUNTIME_DIR`.
- `make e2e` green; `make soak` 100 clean; `make lint` clean; ctest 17/17
  (incl. `tst_switcher`).
- `bash scripts/capture-switcher.sh` — host Wayland + spectacle + Pillow; not
  in `make e2e`.

Gotchas for later tasks:

- **One recency source**: render `AppSwitcher`'s `app_switcher` +
  `app_switcher_entry` projection; never sort in the shell.
- The entry event has **no window handle**; T-06.2b must append it additively
  to address windows.
- Non-entry visible windows are hidden with `alpha = 0` while the switcher is
  active; there is no open/close transition (instant), and reduced motion is
  the same settled frame.
- The compositor preview grid and the shell card row share the strip height
  (tokens) but not per-card geometry; alignment/per-card labels are polish.
- The live check was programmatic (this model cannot view images); a human
  should eyeball `docs/captures/t06-app-switcher.png`.

## T31 — T-06.2b Switcher commit, Cmd+` cycling, interruptibility

**State: done.** T-06 is complete. Cmd+` / Cmd+Shift+` cycle windows within the
selected app on the *same* `AppSwitcher` machine (no second binding path,
ADR 0023); Command release commits the cursor-selected window through the same
activation path as the Dock's `activate_app`; a pointer press on a live preview
commits that app and a press off every preview cancels; Escape is unchanged.
The live preview follows the window cursor, so Cmd+` swaps the surface.

What landed:

- **`compositor/src/app_switcher.rs`** — `SwitcherApp` carries `windows`
  (most-recent first) + `window`; `AppSwitcher` gained `window_cursor`,
  `step_window` (wraps; resets on an app step), `open_focused`, `select_window`,
  `window_direction`; `commit` returns the cursor-selected window. 6 new unit
  tests (13 total).
- **`compositor/src/input/action.rs`** — `InputAction::AppSwitcherWindow`
  (`"app-switcher-window"`).
- **`compositor/src/input/shortcuts.rs`** — Cmd+` and Cmd+Shift+` bind to it
  (keysym `KEY_grave`), resolved by the one shortcut engine; unit test.
- **`compositor/src/input.rs`** — the keyboard arm routes `AppSwitcherWindow`
  to the machine; a left press while the switcher is active is intercepted
  *before* chrome routing (`switcher_window_at` hit -> `app_switcher_commit_window`,
  miss -> `app_switcher_cancel`).
- **`compositor/src/state.rs`** — `app_switcher_entries` groups all windows per
  app; `most_recent_window_of_app` + `activate_app` (shared); `app_switcher_window_key`;
  `app_switcher_commit` (app default via `activate_app`, cycled window via
  `activate_window_id`); `app_switcher_commit_window`; `switcher_window_at`;
  `switcher_candidates`/`switcher_frame` preview the cursor window.
- **`compositor/src/shell/mod.rs`** — the `activate_app` request calls the
  shared `DfState::activate_app`.
- **`compositor/src/input/synthetic.rs`** — `query switcher` adds `window=` and
  `window-direction=` to the state line and a window count to each `switcher app`
  line (all additive; existing readers ignore them).
- **`compositor/tests/window_conformance.rs`** — new
  `app_switcher_cycles_windows_and_pointer_commits` (34/34 green). The parser
  gained `selected_window`/`window_direction`.
- **Captures** — `scripts/capture-switcher-driver.py` adds the Cmd+` step;
  regenerated `docs/captures/t06-app-switcher{,-reduced}{,-cycled,-window-cycled}.png`.
- **Docs** — `02-compositor.md` "App-switcher commit, Cmd+` cycling, and
  interruptibility (T-06.2b)"; `04-shell.md`; ADR `0023`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor` — **273** bin unit tests + every suite
  green (`window_conformance` 34/34, `shell_protocol_conformance` 32/32). Direct
  cargo needs `PKG_CONFIG_PATH=~/.local/df-devroot/lib64/pkgconfig`,
  `RUSTFLAGS="-L ~/.local/df-devroot/lib64"`, `XDG_RUNTIME_DIR`.
- `make e2e` green; `make soak` 100 clean; `make lint` clean; ctest 17/17.
- `bash scripts/capture-switcher.sh` — host Wayland + spectacle + Pillow; not in
  `make e2e`.

Gotchas for later tasks:

- **One machine**: Cmd+` drives `DfState::app_switcher` via
  `app_switcher_window_key`; the shell must never add a binding or re-derive.
- Cmd+` with the switcher closed seeds on the *focused* app (`open_focused`,
  no skip) and opens the overlay; the app highlight never moves on a window
  cycle.
- Only the selected entry's cursor window previews; the app's other windows are
  `alpha = 0` while the overlay owns the scene.
- Pointer commit is compositor-side (the shell overlay is an offscreen image
  with no pointer handling); `switcher_window_at` hit-tests the preview rects.
- The entry protocol event still has **no window handle** and did not need one.
- This model cannot view images; a Pillow check confirms the stills are
  non-blank, have the scrim band and the card highlight (`#fbe1ec`); a human
  should eyeball `docs/captures/t06-app-switcher-window-cycled.png`.

## T32 — T-07.1a Adapter contract, states, and mock

**State: done.** The one adapter contract and its test mock landed in a new
dependency-free library crate; no concrete adapter or subscription yet (those
are T-07.1b–T-07.4).

What landed:

- **`services/system-adapters/`** (new crate `dragonfruit-system-adapters`,
  workspace member, no deps): `src/lib.rs` (`Adapter`, `StatusSource`,
  `status_slots`), `src/state.rs` (`AdapterState<T>` =
  Available/Unavailable/Error, `AdapterError`, `AdapterId`, `StatusSlot`, the
  `slot()` projection), `src/mock.rs` (`MockAdapter`).
- **`Makefile`** — `make e2e` now runs
  `cargo test -p dragonfruit-system-adapters` before `make demo`.
- **Tests** — 8 lib + 2 integration (`tests/status_states.rs`), exercising all
  three states and the consumer slot projection.
- **Docs** — `docs/design/07-system-integration.md` "The adapter contract
  (T-07.1a)"; ADR `0024`.
- **Capture** — `docs/captures/t07-adapter-contract.png` (nested demo; this
  unit has no surface of its own).

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-system-adapters` — 10 tests green.
- `make e2e` — exit 0; the adapters tests run before the scripted demo.
- `cargo fmt --all -- --check` and
  `cargo clippy -p dragonfruit-system-adapters --all-targets -- -D warnings`
  clean.

Gotchas for later tasks:

- **One contract, one projection**: implement `Adapter` (snapshot type, id,
  `state()`); consumers use `AdapterState::slot`/`status_slots` rather than
  re-deriving hidden/inert/live themselves.
- `AdapterId` constants already match the shell `StatusItem` ids (`wifi`,
  `bluetooth`, `volume`, `battery`); add new ids here, not in the shell.
- **No poll seam** on purpose. T-07.1b owns the subscription/event API; keep
  `AdapterState` the stable base and do not add a `tick()`.
- The crate has **no dependencies** so a native/D-Bus adapter can depend on it
  without leaking its stack; concrete adapters own their daemon deps.
- No process hosts the Rust adapters for the (C++) shell yet; the `StatusSlot`
  projection is the seam T-07.1b/T-07.2/T-07.5 must bridge.

## T33 — T-07.1b Event subscription, restart re-subscribe, absence

**State: done.** The subscription/event seam landed in the same
dependency-free crate; no concrete adapter yet (T-07.2 is next).

What landed:

- **`services/system-adapters/src/subscription.rs`** (new) — `ConnectionState`
  (`Absent`/`Subscribed`), `AdapterEvent` (`Subscribed { resubscribe }`,
  `Disconnected`, `Changed`), `SubscribeError` (`Absent` vs `Failed`, with
  `to_state()`), `Subscription` (lifecycle + bounded event outbox,
  `subscribed()`/`absent()`/`changed()`/`record_subscribe()`).
- **`src/lib.rs`** — `Adapter` gained `connection()` and
  `drain_events(&mut self) -> Vec<AdapterEvent>`; exports extended.
  `AdapterState` and the `slot()` projection are unchanged.
- **`src/mock.rs`** — `MockAdapter` simulates its daemon: `kill()` (absence →
  `Unavailable`, slot hidden), `restart()` (re-subscribe + re-sync to the last
  snapshot), plus `connection()`, `subscriptions()`, `events()`.
- **`tests/subscription.rs`** (new) — 4 headless tests: kill hides the slot
  (not an error), restart re-subscribes & re-syncs; absent-at-startup is
  hidden; repeated restarts keep counting; a data push is an event not a poll.
- **Docs** — `07-system-integration.md` "Event subscription and restart
  re-subscribe (T-07.1b)"; ADR `0025`.
- **Capture** — `docs/captures/t07-subscription-restart.png` (nested demo; this
  unit has no surface of its own).

Commands that work (repo root):

- `cargo test -p dragonfruit-system-adapters` — 16 lib + 2 + 4 integration
  tests green.
- `make e2e` — exit 0.
- `cargo fmt --all -- --check` and
  `cargo clippy -p dragonfruit-system-adapters --all-targets -- -D warnings`
  clean.

Gotchas for later tasks:

- **One lifecycle**: a concrete adapter owns a `Subscription` and calls
  `subscribed()`/`absent()`/`changed()` (or `record_subscribe`). Do not add a
  second resubscribe counter; the host drains `AdapterEvent`s via
  `drain_events`.
- **Events carry no snapshot** (ADR 0025): after any event, re-read
  `Adapter::state`. The event stream and state cannot disagree.
- **`Absent` ≠ error**: `SubscribeError::Absent` → `Unavailable` (hidden),
  `Failed` → `Error` (visible inert); neither blocks startup. A restart is
  `absent()` then `subscribed()` → `Disconnected` then
  `Subscribed { resubscribe: true }`, subscribe count 2.
- Redundant lifecycle calls are no-ops, so a busy loop cannot inflate counts.
- `MockAdapter::restart()` with no prior snapshot leaves the adapter
  subscribed but `Unavailable` (present daemon, no data yet) — the slot hides
  until the first push. Intentional; tests rely on it.
- `MockAdapter::new`/`with_available` are no longer `const` (the outbox holds a
  `VecDeque`); no call site needed const.

## T34 — T-07.2a NetworkManager read path

**State: done.** The NetworkManager read path landed in a new concrete-adapter
crate; no write path (T-07.2b) and no shell wiring (T-07.5) yet.

What landed:

- **`services/networkmanager/`** (new crate `dragonfruit-networkmanager`,
  workspace member; deps: `dragonfruit-system-adapters`, `zbus` blocking,
  `serde`): `src/source.rs` (raw `NetworkManagerData`/`WifiDeviceData`/
  `AccessPointData`, `NetworkManagerSource` seam, `MockNetworkManager`),
  `src/model.rs` (`WifiSnapshot` + `WifiState`/`Connectivity`/`Security`/
  `AccessPoint`/`Band` decoding), `src/adapter.rs`
  (`NetworkManagerAdapter<S>` over the shared `Adapter`/`Subscription`),
  `src/dbus.rs` (`DbusNetworkManager`, zbus blocking proxies; no libnm).
- **Transport seam**: `NetworkManagerSource::read` → `Ok(Some(data))`
  present, `Ok(None)` absent, `Err(AdapterError)` present-but-failed.
  `refresh()` is the only daemon read and drives `Subscription`: data →
  `Subscribed`/`Changed`; absent → `Disconnected` + `Unavailable`; error →
  `Subscribed`/`Changed` + `Error`. `MockNetworkManager::reads()` proves no
  hidden polling.
- **Tests** — 16 lib + 5 integration (`tests/read_path.rs`, fixture
  `tests/fixtures/nm-office.json`); acceptance:
  `the_fixture_renders_state_and_the_access_point_list`,
  `an_absent_networkmanager_hides_the_item_and_never_errors`.
- **Makefile** — `make e2e` now runs `cargo test -p dragonfruit-networkmanager`
  after the system-adapters tests.
- **Docs** — `07-system-integration.md` "The NetworkManager read path
  (T-07.2a)"; ADR `0026`.
- **Capture** — `docs/captures/t07-networkmanager.png` (nested demo; this unit
  has no surface of its own).

Commands that work (repo root):

- `cargo test -p dragonfruit-networkmanager` — 16 lib + 5 integration green.
- `make e2e` — exit 0 (networkmanager tests included).
- `make clippy` and `cargo fmt --all -- --check` — clean.

Gotchas for later tasks:

- **Crate per adapter** (ADR 0026): T-07.2b extends
  `dragonfruit-networkmanager`; T-07.3/T-07.4 add `services/audio` /
  `services/power`. Do not move NM types into `dragonfruit-system-adapters`.
- **Read only**: `DbusNetworkManager` reads; no activate/join (T-07.2b). There
  is no background signal thread yet — the host (T-07.5) owns the bridge and
  must call `NetworkManagerAdapter::refresh()` on `PropertiesChanged`/
  `DeviceAdded`/`DeviceRemoved`, not poll.
- **Absence vs failure**: no system bus or `NameHasOwner` false → `Ok(None)`
  (hidden, never an error); a name owned but a property read failing → `Err`
  (visible, inert).
- **Snapshot shape**: `WifiSnapshot.access_points` is one entry per SSID
  (strongest BSSID wins; the active flag survives a stronger neighbour),
  sorted strongest-first; `signal_strength()` is the active AP's; `glyph()` /
  `label()` are the menu-bar strings T-07.5 should render.
- The live D-Bus source is compile-checked, not exercised in CI (no bus).
- The shell still shows `--placeholders` fake Wi-Fi until T-07.5b; this task
  wires nothing into QML.

## T35 — T-07.2b NetworkManager join and polkit degradation

**State: done.** The write path (join/activate) and the polkit read-only
degradation landed in the same concrete-adapter crate; no shell wiring yet
(T-07.5).

What landed:

- **`services/networkmanager/src/source.rs`** — the transport seam gained
  `NetworkManagerSource::activate(&ActivateRequest) -> ActivateOutcome`
  (`Accepted` / `Denied(note)` / `Absent` / `Failed(AdapterError)`).
  `ActivateRequest` (ssid + optional secret + optional AP path) has a manual
  `Debug` that redacts the secret. `MockNetworkManager` gained
  `deny_joins(note)` / `fail_joins(message)` / `allow_joins()` and
  `activations()`.
- **`services/networkmanager/src/adapter.rs`** — `NetworkManagerAdapter::join`
  maps the source outcome to `JoinResult::{Accepted, Denied{note}, Absent,
  Failed}`. A denial sets `WifiAccess::ReadOnly{note}` (queryable via
  `access()` / `is_read_only()` / `degradation_note()`); a later join short-
  circuits locally (no second daemon call). A successful join does **not**
  write a snapshot: the host refreshes on the daemon's push. `JoinRequest` also
  redacts its secret in `Debug`.
- **`services/networkmanager/src/dbus.rs`** —
  `DbusNetworkManager::activate` resolves the Wi-Fi device + strongest matching
  BSSID, calls `AddAndActivateConnection(a{sa{sv}}, o device, o specific)`, and
  classifies a polkit refusal by error name
  (`org.freedesktop.NetworkManager.PermissionDenied`,
  `org.freedesktop.DBus.Error.AccessDenied`,
  `org.freedesktop.PolicyKit1.Error.NotAuthorized`) or message. It stays
  compile-checked only (no bus in CI).
- **Tests** — 32 lib (join accept/deny/fail/absent, read-only persistence,
  secret redaction, error classification, settings shape); acceptance
  `tests/join_path.rs` (4 tests):
  `a_join_against_a_mocked_networkmanager_is_accepted`,
  `a_polkit_denial_leaves_a_read_only_item_with_a_recorded_note`,
  `a_denied_join_does_not_drop_the_network_list`,
  `an_absent_daemon_reports_absence_and_never_errors`.
- **Docs** — `07-system-integration.md` "The NetworkManager join path and
  polkit degradation (T-07.2b)"; ADR `0027`.
- **Capture** — `docs/captures/t07-networkmanager-join.png` (nested demo window,
  `spectacle -b -n -a`; this unit has no surface of its own).

Commands that work (repo root):

- `cargo test -p dragonfruit-networkmanager` — 32 lib + 4 + 5 integration green.
- `make e2e` — exit 0.
- `cargo clippy -p dragonfruit-networkmanager --all-targets -- -D warnings`
  and `cargo fmt --all -- --check` — clean.

Gotchas for later tasks:

- **Read-only is adapter-level, not snapshot-level** (ADR 0027): the network
  list keeps rendering; the status slot stays visible **and enabled**. Only the
  join affordance is off. A refresh does not clear the degradation.
- **Denied ≠ Failed ≠ Absent**: `Denied` is a polkit refusal (read-only
  degradation); `Failed` is any other write error and leaves `WifiAccess`
  untouched; `Absent` hides the item and marks `Unavailable`.
- `join` while read-only returns `Denied` with the recorded note **without**
  calling the source (`activations()` stays put). A test can assert that.
- The recorded note is the daemon's own message, prefixed `NetworkManager: `.
- The live `AddAndActivateConnection` path and its `a{sa{sv}}` settings builder
  are compile-checked only; no system bus in CI. NM normalises the missing
  id/uuid in the `connection` setting.
- **T-07.5a**: build the Wi-Fi menu against the concrete adapter and gate the
  join rows on `adapter.access()`; surface `degradation_note()` where useful.
  A generic contract-level read-only concept was deliberately not added — lift
  it into `dragonfruit-system-adapters` only if T-07.3/T-07.4 gain a gated
  write.

## T36 — T-07.3 Audio adapter (PipeWire/WirePlumber)

**State: done.** The audio adapter landed in a new concrete-adapter crate; no
shell wiring yet (T-07.5 renders the slider).

What landed:

- **`services/audio/`** (new crate `dragonfruit-audio`, workspace member; deps:
  `dragonfruit-system-adapters` + `serde_json`; no zbus, no libpipewire):
  `src/source.rs` (raw `AudioData`/`SinkData`, `AudioSource` trait, `SetOutcome`,
  `MockAudio`), `src/pw_dump.rs` (`AudioData::from_pw_dump`, `CommandAudio`,
  `PW_DUMP_BIN`/`WPCTL_BIN`/`DEFAULT_SINK_TARGET`), `src/model.rs`
  (`AudioSnapshot` + `Sink`), `src/adapter.rs` (`AudioAdapter<S>`).
- **Transport seam**: `AudioSource::read` → `Ok(Some(data))` present,
  `Ok(None)` absent, `Err(AdapterError)` present-but-failed. `refresh()` is the
  only daemon read and drives the shared `Subscription`. Writes are
  `set_volume(f32)` / `set_mute(bool)` → `SetOutcome::{Applied, Absent,
  Failed}`; a write does **not** invent a snapshot (the daemon push + host
  refresh is the single source of truth, as in T-07.2b).
- **Live path** (ADR 0028): there is no stable D-Bus volume API and no
  `libpipewire` dev headers in the pinned toolchain, so `CommandAudio` runs
  WirePlumber's tools: `pw-dump` (JSON) for the read, `wpctl set-volume`/
  `set-mute @DEFAULT_AUDIO_SINK@` for the writes. The `pw-dump` schema is
  pinned in `from_pw_dump`, which un-cubes WirePlumber's stored channel volume
  (`linear = cbrt(stored)`) to the 0..=1 value the menu shows.
- **Tests** — 27 lib + `tests/read_path.rs` (6) + `tests/volume_path.rs` (6),
  fixture `tests/fixtures/pw-dump-office.json` (captured from the real
  `pw-dump`, plus one synthesized second sink). Acceptance:
  `a_volume_change_reflects_within_one_event`,
  `a_mute_change_reflects_within_one_event`, `toggle_mute_reflects_within_one_event`.
  `the_live_wireplumber_reads_when_a_session_is_present` exercises the real CLI
  path and skips (absence) on CI.
- **Makefile** — `make e2e` now runs `cargo test -p dragonfruit-audio` after
  the networkmanager tests.
- **Docs** — `07-system-integration.md` "The audio path (T-07.3)"; ADR `0028`.
- **Capture** — `docs/captures/t07-audio.png` (nested demo, `spectacle -b -n -f`;
  this unit has no surface of its own). Vision check: menu bar + Dock + windows
  render, no stray artifacts.

Commands that work (repo root):

- `cargo test -p dragonfruit-audio` — 27 lib + 6 + 6 integration green.
- `make e2e` — exit 0 (audio tests included).
- `cargo clippy -p dragonfruit-audio --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.

Gotchas for later tasks:

- **CLI source, not a native link** (ADR 0028): the live read/write shells out
  to `pw-dump`/`wpctl`. The adapter/model/shell see only `AudioSnapshot`. If
  T-15 adds libpipewire to the toolchain, drop a native source in behind the
  same `AudioSource` trait.
- **Volume is linear 0..=1** in `AudioSnapshot` (`volume()`/`level()`), with
  `volume_percent()` for the slider label. WirePlumber stores volume cubed;
  `from_pw_dump` does the `cbrt`. Do not re-cube in the shell.
- **Default sink is resolved by `node.name`**, falling back to the first sink
  when WirePlumber names none; the default sorts first in `sinks`.
- **Glyph names** are the shell's (`StatusGlyph.qml`): `volume` / `volume-muted`
  (`glyph()` returns `volume-muted` when muted or volume is 0).
- **Absence**: a `pw-dump` that cannot run or exits non-zero (no PipeWire core)
  → `Unavailable` (hidden); malformed JSON → `Error` (visible, inert).
- **T-07.5** owns the event bridge: it must call `AudioAdapter::refresh()` on a
  WirePlumber change (`pw-mon` / `pactl subscribe`), not poll. Writes target the
  default sink only; per-sink writes and routing are T-15.

## T37 — T-07.4 Power adapter (UPower)

**State: done.** The read-only power adapter landed in a new concrete-adapter
crate; no shell wiring yet (T-07.5 renders the battery item).

What landed:

- **`services/power/`** (new crate `dragonfruit-power`, workspace member; deps:
  `dragonfruit-system-adapters` + `zbus` (blocking-api) + `serde`; dev
  `serde_json`): `src/source.rs` (raw `PowerData`/`PowerDeviceData`,
  `PowerSource` trait, `MockPower`, `DEVICE_TYPE_BATTERY`), `src/model.rs`
  (`ChargeState` / `BatteryLevel` / `Battery` / `PowerSnapshot`),
  `src/upower.rs` (`DbusUPower`, `UPOWER_SERVICE`), `src/adapter.rs`
  (`PowerAdapter<S>`).
- **Transport seam**: `PowerSource::read` → `Ok(Some(data))` present,
  `Ok(None)` absent, `Err(AdapterError)` present-but-failed. `refresh()` is the
  only daemon read and drives the shared `Subscription`. **There is no write**:
  the battery item is read-only (power profiles deferred to T-15).
- **Live path**: UPower over the **system bus** via `zbus`
  (`org.freedesktop.UPower`): `OnBattery` + `EnumerateDevices`, then each
  device's `org.freedesktop.UPower.Device` properties (`Type`, `IsPresent`,
  `PowerSupply`, `Percentage`, `State`, `BatteryLevel`, `TimeToEmpty`,
  `TimeToFull`). `NameHasOwner` distinguishes absence from a present-but-broken
  daemon. No new dependency class — zbus is already in
  `dragonfruit-networkmanager` (ADR 0026 named `services/power`).
- **Presence semantics**: the model reports the first device with
  `kind == 2 (Battery)` and `IsPresent`; if none, `PowerSnapshot::present()` is
  false. The **adapter** stays `Available` when UPower answers with no battery
  (a desktop/VM); the consumer hides the item on `!present()`. UPower absent is
  `AdapterState::Unavailable` (hidden). Both paths never error.
- **Model** — `ChargeState::{Unknown,Charging,Discharging,Empty,FullyCharged,
  PendingCharge,PendingDischarge}` (`is_charging`/`is_plugged`/`is_discharging`),
  `BatteryLevel`, `Battery` (percentage clamped 0–100, `percent()`, `fill()`),
  `PowerSnapshot` (`present`, `percentage_percent`, `level` = 0..=1,
  `charging`, `plugged`, `on_battery`, `time_to_empty`, `time_to_full`). Glyph
  is `"battery"` (the existing `StatusGlyph.qml` case); charging is carried by
  `charging()` + the label, not a distinct glyph.
- **Tests** — 22 lib + `tests/read_path.rs` (7), fixture
  `tests/fixtures/upower-laptop.json` (line power + one 82% charging battery).
  Acceptance includes `the_fixture_renders_the_battery_level_and_charge_state`,
  `an_absent_upower_hides_the_item_and_never_errors`,
  `a_present_daemon_with_no_battery_has_no_item_to_show`,
  `a_daemon_restart_resubscribes_and_resyncs`. `the_live_upower_reads_when_a_session_is_present`
  exercises the real D-Bus path (this host has UPower with line power only) and
  skips on CI.
- **Makefile** — `make e2e` now runs `cargo test -p dragonfruit-power` after the
  audio tests.
- **Docs** — `07-system-integration.md` "The power path (T-07.4)".
- **Capture** — `docs/captures/t07-power.png` (nested demo, `spectacle -b -n -f`;
  this unit has no surface of its own). Vision check: menu bar + Dock + windows
  + desktop render, no stray artifacts.

Commands that work (repo root):

- `cargo test -p dragonfruit-power` — 22 lib + 7 integration green.
- `make e2e` — exit 0 (power tests included).
- `cargo clippy -p dragonfruit-power --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.

Gotchas for later tasks:

- **Read-only, no write half.** `PowerAdapter` has only `refresh()` and
  `snapshot()`. Power profiles (`power-profiles-daemon`) and control are T-15;
  do not add a `set_*` here without a task.
- **Two hides, one item**: UPower absent → `AdapterState::Unavailable`
  (`slot.visible == false`); UPower present but no battery → `Available` with
  `snapshot.present() == false`. T-07.5b must check BOTH before drawing the
  item, otherwise desktops/VMs show "No battery".
- **Glyph**: `PowerSnapshot::glyph()` returns `"battery"` only. The charging
  bolt is not yet in `StatusGlyph.qml`; T-07.5b either adds a `battery-charging`
  case + changes `glyph()` or uses `charging()`/`label()`.
- **`level()` is 0..=1** for the `StatusGlyph` fill; `percentage_percent()` is
  the 0–100 label. Do not pass the 0–100 value as `level`.
- **`Percentage` is 0–100** (double) in UPower, unlike PipeWire's 0..=1. The
  model clamps; do not re-scale in the shell.
- **T-07.5** owns the event bridge: call `PowerAdapter::refresh()` on a UPower
  `PropertiesChanged`/`DeviceAdded`/`DeviceRemoved` signal (or `NameOwnerChanged`
  for restart), never a poll. The live source is compile-checked + exercised on
  a host with UPower, not in CI.
- **UPower on this host has no battery** (line power only), so the live smoke
  test reads `Ok(Some)` with `present() == false`; the fixture covers the
  battery path.

## T38 — T-07.5a Wi-Fi and volume status menus

**State: done.** Wi-Fi list/join and volume slider/mute render in
design-system popovers, served by a new session-bus bridge host over the
T-07.2/T-07.3 adapters.

What landed:

- **`services/system-status/`** (new crate `dragonfruit-system-status`,
  workspace member; deps: the three adapter crates + `serde_json` + `zbus`):
  `src/lib.rs` (`StatusHost<N, A>` adapter-only core, `wifi_view`/`audio_view`
  JSON, `join`/`set_volume`/`set_mute`/`toggle_mute` reports,
  `DBUS_NAME`/`DBUS_PATH`/interface consts), `src/dbus.rs` (blocking zbus
  service `org.dragonfruit.SystemStatus1` at `/org/dragonfruit/SystemStatus1`,
  interfaces `…Wifi`/`…Audio`), `src/main.rs` (`--print-wifi`/`--print-audio`
  smoke modes). `tests/host.rs` (4) + 7 lib tests.
- **Shell side**: `shell/src/systemstatusmodel.{h,cpp}` (dockcore lib, JSON →
  QVariantMap menu model, request signals), `shell/src/systemstatusclient.{h,cpp}`
  (`DbusSystemStatusClient` + `MockSystemStatusClient`),
  `shell/menubar/WifiMenu.qml` + `VolumeMenu.qml`, and `MenuBar.qml` popover
  wiring + `ShellController` bridge + `StatusGlyph.qml` `wifi-*` variants.
  `shell/tests/tst_statusmodel.cpp` (new, 9 cases) and 10 new cases in
  `tst_menubar.qml`.
- **Makefile** — `make e2e` now runs `cargo test -p dragonfruit-system-status`.
- **Docs** — `07-system-integration.md` "The status bridge host (T-07.5a)";
  ADR `0029`.
- **Capture** — `scripts/capture-status-menus.sh` +
  `capture-status-menus-driver.py`; `docs/captures/t07.5a-wifi.png` /
  `t07.5a-volume.png`. Vision check: Wi-Fi popover lists
  home/dragonfruit-guest/NeighbourNet with signal bars; Sound popover shows a
  60% slider, "Built-in Speakers", Mute.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 7 lib + 4 integration green.
- `make qml-test` — includes `tst_statusmodel` (9) and the new `tst_menubar`
  cases; `make e2e` exit 0; `cargo fmt --all -- --check` clean.
- Smoke: `cargo run -q -p dragonfruit-system-status -- --print-wifi` (and
  `--print-audio`) prints the live JSON view.

Gotchas for later tasks:

- **The bridge host is a distinct process.** Nothing launches
  `dragonfruit-system-status` yet; the shell constructs
  `DbusSystemStatusClient` whenever `--placeholders` is off. T-07.5b removes
  the flag, so it must also start the host (session bus) or the items hide.
- **`--placeholders` now drives `MockSystemStatusClient`** (the demo data).
  Keep that fixture client when the flag goes; the headless tests and the
  capture script depend on it.
- **Popover geometry**: status popovers reuse the existing menu `overlay`
  popup surface. `MenuBar._dropdownRect` returns the open status popover's
  rect; `ShellController::onStatusMenuOpened/Closed` set `m_menuOpen`. A new
  status popover must set `openStatusItem` (only "wifi"/"volume" are wired).
- **Tap handling**: the bar's click-away `TapHandler` must ignore taps inside
  `statusRow` (as it does for `appMenuRow`), otherwise the tap that opens a
  status popover closes it in the same event. Keep that guard for battery.
- **Glyphs**: `StatusGlyph.qml` now handles `wifi-off`/`wifi-disabled`
  (dimmed), `wifi-secure`, `wifi-connecting`, and `wifi-error` (small cross)
  in addition to `wifi`; volume uses `volume`/`volume-muted`.
- **No daemon-signal refresh yet**: the host re-reads on startup, on the
  shell's explicit `Refresh()` (menu open), and after an action reply. The
  daemon subscriptions (NM signals / `pw-mon` / UPower `PropertiesChanged`)
  are T-07.6 and must not become a poll loop.
- **`SystemStatusModel::parseView` rejects a mismatched `kind`**, so the two
  interfaces cannot cross-pollute; `visible`/`enabled` are derived from
  `state` (`unavailable` hides, `available` enables, `error` is visible/inert).
- **Status item measured centers** (placeholder demo, 1920-wide output,
  right-anchored): wifi x≈1615, bluetooth≈1644, volume≈1673, battery≈1702,
  y=14. The capture driver hard-codes wifi/volume from-right (305/247 px); if
  T-07.5b adds/removes visible placeholder slots, re-measure.

## T39 — T-07.5b Battery menu, placeholder removal, keyboard a11y

**State: done.** The battery item is live over the bridge host, `--placeholders`
is gone, and the status row has a keyboard path; the demo app menu is kept for
T-14.

What landed:

- **`services/system-status`** — `StatusHost<N, A, P>` gained a `PowerAdapter`;
  `new(wifi, audio, battery)`, `refresh_battery()`, `battery_view()` /
  `battery_state()`, `battery()`/`battery_mut()`. New const
  `BATTERY_INTERFACE = "org.dragonfruit.SystemStatus1.Battery"`, served by a
  `BatteryInterface` with `State()`/`Refresh()` only (read-only, no write).
  `dbus::LiveHost` is now `StatusHost<DbusNetworkManager, CommandAudio,
  DbusUPower>`; `interface_names()` returns 3. `main.rs` adds `--print-battery`.
  `battery_view` JSON: `{kind:"battery", state, present, glyph, label, percent,
  level, charging, plugged, onBattery, timeToEmpty, timeToFull}`; `present` is
  `false` on a machine with UPower but no battery.
- **Shell model/client** — `SystemStatusModel` gained `battery()`,
  `batteryVisible()`, `applyBattery[Json]`, `requestRefreshBattery()`. Its
  `normalize` hides the battery on the second hidden case (`present:false`).
  `SystemStatusClient` gained `refreshBattery()` + `batteryState`; the D-Bus
  client calls `…Battery.State()`, the fixture client emits an 82%-charging
  view.
- **Shell QML** — new `shell/menubar/BatteryMenu.qml` (design-system `Popup`,
  read-only: percentage, charge label, time to full/empty; raises
  `refreshRequested`/`closed`). `MenuBar.qml` wires the `batteryMenu` model,
  the third popover, `_dropdownRect`, and the click-away guard; `StatusItem`
  gained a `keyboardFocus` `FocusRing`; `StatusGlyph` gained `battery-charging`
  (a bolt). The shell controller hides the Bluetooth/Focus/Accessibility
  placeholders (their adapters are later), keeps `demoAppMenu()` always, and
  wires `onBatteryState`.
- **Keyboard a11y** — the bar root keeps focus; Left/Right move
  `keyboardStatusIndex` across available status slots (hidden slots skipped),
  Return/Space opens the selected slot's popover (or raises
  `statusItemActivated`), Escape closes. `WifiMenu` adds Up/Down highlight +
  Return, `VolumeMenu` adds arrow-key volume steps + Return mute,
  `BatteryMenu` is read-only. Each menu exposes `closed()` so the bar clears
  `openStatusItem` when a popup closes via Escape.
- **`--placeholders` removed** — gone from `shell/src/main.cpp`,
  `ShellController::start`, and `tools/dragonfruit-dev`'s `launch_shell`. The
  fixture client is now selected by `DF_STATUS_FIXTURE=1` (headless/capture
  only). The demo app menu is no longer gated on the flag.
- **Tests** — `cargo test -p dragonfruit-system-status`: 11 lib + 6 integration
  (battery view, present-but-no-battery, absent UPower, read failure). Qt:
  `tst_statusmodel` (battery decode/hide + refresh request) and
  `tst_menubar.qml` (battery popover, no-battery opens nothing, arrow
  selection, Return-opens/Escape-closes, `battery-charging` pixel test).
- **Capture** — `scripts/capture-status-menus.sh` now exports
  `DF_STATUS_FIXTURE=1` and captures `docs/captures/t07.5b-battery.png`
  (wifi/volume still `t07.5a-*`). Vision check: popover shows "82% charging",
  a bolt glyph, "1:30 until full", read-only, no artifacts.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 11 lib + 6 integration green.
- `make e2e` — exit 0 (system-status included; scripted demo still clean).
- `make qml-test` — 18/18, including `tst_menubar`, `tst_statusmodel`,
  `qmllint_shell-menubar`.
- Capture: `bash scripts/capture-status-menus.sh`.

Gotchas for later tasks:

- **Visibility needs the bridge host running.** Nothing launches
  `dragonfruit-system-status` yet; the dev tool does **not** start it. A live
  dev session has every live item hidden (the honest absent-daemon state).
  Start the host (or set `DF_STATUS_FIXTURE=1`) to see wi-fi/volume/battery.
  T-07.6a validates absence; packaging/session ownership starts the host.
- **`DF_STATUS_FIXTURE`** selects `MockSystemStatusClient`; it is the only
  remaining fixture switch after `--placeholders`. Keep it for headless and
  the capture script; it is read once at `ShellController::start`.
- **Battery hides on two conditions**: `state != available` (UPower absent /
  error handling) and `present == false` (no cell). The model owns both; the
  QML only reads `visible`.
- **Measured status centers** (fixture demo, 1920-wide): wifi 1644, volume
  1673, battery 1702, y=14; from-right 276/247/218 (Bluetooth hidden). The
  capture driver hard-codes these.
- **Read-only**: the Battery interface has no write; do not add `Set*` here
  (power profiles are T-15).
- **Keyboard**: the bar root must hold active focus for arrows; opening a
  popover moves focus into the `Popup`, which handles its own keys. `closeStatusMenu`
  now guards on `openStatusItem` (not the popup `open` flags) to avoid a
  double `statusMenuClosed` when a popup's own `closed` signal re-enters.
- **Glyph**: charging picks `battery-charging` from `charging` in the
  controller; the battery fill is `level` (0..=1), the label is `percent`
  0–100.

## T40 — T-07.6a Absent-daemon masking matrix

**State: done.** The absent-daemon masking matrix is asserted headlessly at
all three seams; no production behavior changed. Capture and the idle trace
stay with T-07.6b.

What landed:

- **`services/system-status/tests/host.rs`** —
  `the_absent_daemon_masking_matrix_hides_only_the_masked_item` (kill + refresh
  each of Wi-Fi/audio/power in turn, then all three; neighbours stay
  `available`, no slot ever `error`, join/set_volume/set_mute report `absent`)
  and `masking_one_daemon_never_turns_a_neighbour_into_an_error` (present but
  unreadable → `error`, neighbours untouched). Added `bar_state()` helper.
- **`shell/tests/tst_statusmodel.cpp`** —
  `theAbsentDaemonMaskingMatrixHidesOnlyTheMaskedItem` and
  `anUnreachedBridgeHostLeavesEveryItemHidden` (fresh model / malformed payload
  → hidden, never a visible error). 14 → 16 cases.
- **`shell/tests/tst_menubar.qml`** —
  `test_absent_daemon_matrix_hides_and_refuses_each_item` (masked slot hidden
  and `openStatusMenu` refuses; every other slot still opens). 38 → 39 cases.
- **`docs/design/07-system-integration.md`** — new section "The absent-daemon
  masking matrix (T-07.6a)" with the matrix table, the negative-space cases,
  and the test locations. No ADR (proves ADR 0024/0025/0029, adds no decision).

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-system-status` — 11 lib + 8 host green.
- `make qml-test` — 18/18 (`tst_menubar` 39, `tst_statusmodel` 16).
- `make e2e` exit 0; `make lint` exit 0.

Live visual check: `/tmp/opencode/t076a-capture.sh` launched the nested demo
with `DRAGONFRUIT_SYNTHETIC_INPUT` set and **no** `DF_STATUS_FIXTURE` and no
bridge host (the honest absent state), captured
`/tmp/opencode/t076a-full.png`. Vision verdict: menu bar shows only the clock +
dot-grid (Control Center) + squares (Mission Control) — no Wi-Fi/volume/battery
items — desktop, Dock, and both demo windows render correctly. No capture
committed (T-07.6b owns `docs/captures/t07-*`).

Gotchas for later tasks:

- **This task changed no runtime behavior.** If the matrix fails later, the
  regression is in the adapter/host/model decode, not here.
- The absent state in the live nested demo is just "no bridge host + no
  fixture": all three live slots hide. T-07.6b's absent-daemon still is that
  same run.
- Real `systemctl stop` masking needs a VM and is recorded as manual in the
  design doc; CI uses the mock `kill()`/`refresh` path only.
- No `docs/captures/` file may be added by this task.

## T41 — T-07.6b Menu-bar idle trace and capture

**State: done.** The menu-bar idle trace is flat at both seams and the T-07
track captures are committed. No runtime behavior changed.

What landed:

- **`compositor/tests/shell_idle_trace.rs`** — the bar-live idle trace now
  parses and asserts `client_wakeups` flat too (was frames / direct-scanout /
  animation only). Result: `frames_rendered=3`, `client_wakeups=0` (both flat).
- **`Makefile`** — new `menubar-idle-trace` target runs that test; raw output
  is `docs/captures/t07-shell-idle-trace.txt`.
- **`services/system-status/tests/host.rs`** — new
  `the_live_status_items_never_poll_the_daemons`: startup sync = exactly one
  read per daemon, 50 view renderings add zero reads, one `refresh()` adds
  exactly one. A poll loop above an adapter would move these counters.
- **`scripts/capture-live-menubar.sh`** (new) — regenerates
  `docs/captures/t07-live-menubar.png` (Wi-Fi + volume + battery live over the
  `DF_STATUS_FIXTURE=1` bridge fixture) and
  `docs/captures/t07-live-menubar-absent.png` (no fixture/host; all live items
  hidden), plus the idle-trace log. Both stills are 1920x1200 nested-output
  crops.
- **`docs/design/07-system-integration.md`** — new section "The menu-bar idle
  trace (T-07.6b)"; no ADR (proves ADR 0009/0029, adds no decision).

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-system-status` — 11 lib + 9 host green.
- `make menubar-idle-trace` — 1 green, `client_wakeups=0` flat.
- `make e2e` exit 0; `make lint` exit 0.
- Capture: `bash scripts/capture-live-menubar.sh`.

Gotchas for later tasks:

- **Host signal wiring is still absent.** `StatusHost` reads on startup, an
  explicit `Refresh()`, and after an action; it does **not** yet subscribe to
  NetworkManager / `pw-mon` / UPower `PropertiesChanged`. The design doc
  (07-system-integration.md) still calls that wiring "T-07.6"; it did not land
  in T-07.6a/b and remains a real follow-up (see Follow-ups). The idle trace
  passes precisely because nothing polls.
- The capture script trims the host window decoration by cropping to the
  bottom-centred 1920x1200 output (the nested output size, same hard-coding as
  the other capture drivers). It best-effort raises the nested window with a
  KWin D-Bus script before `spectacle -a` so a host window cannot crop the bar
  out; on a non-KWin host the window must already be in front.
- The idle-trace files are `docs/captures/t07-shell-idle-trace.txt` (bar-live)
  and the T-03 `t03-idle-trace.txt` (no clients). Do not overwrite the latter.

## T42 — T-08.1a settingsd config model and D-Bus API

**State: done.** `services/settingsd` is no longer a stub: the desktop-settings
model and the `org.dragonfruit.Settings1` surface land. Persistence is
deliberately T-08.1b.

What landed:

- **`services/settingsd/src/schema.rs`** (new) — `KEYS`: 20 named keys
  (11 Dock, 1 workspaces, 3 gestures, 2 appearance, 1 animation, 2 input), each
  with its D-Bus type, default, range/enum, and owner/consumer pair;
  `SCHEMA_VERSION = 1`; `spec()`/`key_names()`; a frozen v1 key-manifest test.
- **`src/value.rs`** (new) — `Value` (`b`/`d`/`x`/`s`/`as`) with
  `to_owned_value`/`from_owned_value`; `SettingsError`.
- **`src/model.rs`** (new) — `Settings` store: validate-on-write, change
  detection (repeat writes are silent no-ops), `snapshot()`/`reset()`.
- **`src/dbus.rs`** (new) — `Settings1` object at
  `/org/dragonfruit/Settings1`: `Get(key)->v`, `Set(key,v)`, `GetAll()->a{sv}`,
  `ListKeys()->as`, the `SchemaVersion` property, and `Changed(key,value)`
  (emitted only on a real change). Rejections are typed under
  `org.dragonfruit.Settings1.Error.{UnknownKey,TypeMismatch,OutOfRange,
  NotAllowed}`. `dbus::run()` parks on the session bus (no poll loop).
- **`src/lib.rs` / `src/main.rs`** — lib+bin; `--print-keys` / `--print-schema`
  debug flags; the old stub test moved into the lib.
- **`Cargo.toml`** — lib target `dragonfruit_settingsd`; dep `zbus`
  (blocking-api), dev-dep `serde`.
- **`tests/session_bus.rs`** (new) — 3 integration tests that stand up a
  private `dbus-daemon`, serve the interface, and drive it from a second
  connection: every key round-trips Get/Set/Changed; GetAll/ListKeys/
  SchemaVersion; typed rejections leave the store untouched.
- **`Makefile`** — `make e2e` now also runs `cargo test -p
  dragonfruit-settingsd`.
- **`docs/design/tracks/08-settingsd-live-settings.md`** — new "The key schema
  and D-Bus surface (T-08.1a)" section with the full key table.
- **`docs/design/adr/0030-settingsd-schema-and-dbus-surface.md`** (new).

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-settingsd` — 16 lib + 3 session-bus green.
- `make e2e` exit 0; `make lint` exit 0.

Live visual check: `/tmp/opencode/t42-capture.sh` launched the nested demo
(no fixture), raised the window via a best-effort KWin D-Bus script, captured
the active window and trimmed it to the 1920x1200 nested output:
`/tmp/opencode/t42-nested.png`. Vision read: top bar present, two app windows
(a large settings window and a small X11 demo window), bottom Dock with five
icons, solid dark background, "no rendering glitches". No `docs/captures/`
file added — the track owns `t08-settingsd.*`.

Gotchas for later tasks:

- **No persistence.** Values are in-memory only; a settingsd restart resets to
  schema defaults. T-08.1b writes `$XDG_CONFIG_HOME/dragonfruit/settings.json`
  in the same `{"schema":N,"keys":{…}}` shape; the key names/defaults already
  match the shell's interim file, so it adopts without migration.
- **No consumer is migrated.** The shell still owns `DockSettings`/`DockPins`
  and their `QFileSystemWatcher`; the compositor still uses its own defaults.
  T-08.2a must delete those shell classes in the same change it wires the
  daemon, or there will be two writers.
- **Values are typed D-Bus variants, not JSON strings** (ADR 0030). `dock.pinned`
  defaults to `[]`; the shell still seeds installed-app defaults.
- `appearance.accent` is free text (`""` = design-system token default); it is
  not hex-validated yet.
- Adding a key is safe; renaming or removing one fails the frozen v1 manifest
  test in `schema.rs`.

## T43 — T-08.1b settingsd persistence and migrations

**State: done.** `settingsd` now owns
`$XDG_CONFIG_HOME/dragonfruit/settings.json` in the shell's existing
`{"schema":N,"keys":{…}}` shape, with atomic writes and a startup migration.
T-08.2 still migrates consumers.

What landed:

- **`services/settingsd/src/persist.rs`** (new) — the persisted format,
  `default_path()` (`$XDG_CONFIG_HOME` → `$HOME/.config`, subdir `dragonfruit`,
  file `settings.json`), atomic `save` (hidden sibling + `sync_all` + `rename`
  + best-effort directory fsync), and `load` → `Loaded { settings,
  persistence, schema, migrated, found }`. Unknown **root** fields and unknown
  **key** entries are preserved verbatim across a save (a schema key wins on a
  name clash); a value that fails validation falls back to its default; a
  malformed/unreadable file is a typed `LoadError`.
- **Migration.** A file with no `schema` field is revision 0; `load` fills
  every key the older file predates with its default and `migrated=true`;
  `main` then writes the upgraded file once. Keys carry `since`, so adding a
  key later is the same mechanism.
- **`dbus.rs`** — `Settings1` has an optional `Arc<Persistence>`
  (`with_persistence`/`persistence`); a real `Set` saves the store **before**
  emitting `Changed`. `dbus::run(settings, Option<Persistence>)`.
  `new`/`from_shared` stay in-memory.
- **`main.rs`** — loads and migrates on startup, runs from defaults on a
  malformed/unreadable file, and gained `--config-path`.
- **`Cargo.toml`** — runtime dep `serde_json = "=1.0.151"`.
- **ADR [0031](../design/adr/0031-settingsd-persistence-format-and-atomic-writes.md)**;
  track doc 08 gained a persistence/atomic-writes/migrations section. The
  `043-…` task file's acceptance box is ticked.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-settingsd` — 26 lib (8 new persist) + 4
  session-bus (1 new: Set→disk→restart reload) green.
- `make e2e` exit 0; `make lint` exit 0.
- Binary smoke test (private `dbus-daemon`, revision-0 fixture): startup logs
  `migrated … to schema 1`, the file gains `"schema":1` and every default while
  keeping `dock.size:0.8` and an unknown root `future:true`;
  `gdbus … Set dock.autohide "<true>"` writes the file and `Get` returns
  `<true>`.

Gotchas for later tasks:

- **T-08.2a must still delete `DockSettings`/`DockPins` + the
  `QFileSystemWatcher`** in the same change it binds the daemon. Until then the
  shell is a second writer of the same file; settingsd preserves unknown
  entries so neither clobbers the other, but do not run both durably for long.
- **`Set` is persist-then-signal.** A save failure is logged and the in-memory
  value stays authoritative (the daemon never dies on disk errors). The file
  path is resolved on every startup; no path is cached across runs.
- **`dbus::run` now takes a second argument** (`Option<Persistence>`); only
  `main` calls it.
- The `--config-path` flag prints the resolved file (or a no-config-home
  notice). The dev tool / demo still does **not** start `dragonfruit-settingsd`
  or create the file; the live visual check therefore only confirms the desktop
  renders (capture `/tmp/opencode/t43-nested.png`: top bar, settings window,
  five-icon Dock, dark background; no artifacts). No `docs/captures/` file was
  added — the T-08 track owns `t08-settingsd.*` for the scripted flip.

## T44 — T-08.2a Shell migration to settingsd

**State: done.** The shell no longer owns Dock settings: `DockSettings`/
`DockPins` and their `QFileSystemWatcher` are deleted; every `dock.*` key is
read/written through `settingsd`.

What landed:

- **`shell/src/settingsclient.{h,cpp}`** (new) — `SettingsClient` (typed key
  store over `QVariantMap`, signals `changed`/`availableChanged`/`refreshed`),
  `DbusSettingsClient` (live `org.dragonfruit.Settings1`: `GetAll` resync,
  `Changed` subscription, `Set` mirror, service add/remove tracking),
  `MockSettingsClient` (tests), and `settingsSchemaDefaults()` mirroring all
  20 v1 keys with their D-Bus types. No polling anywhere.
- **`shell/src/dockmodel.{h,cpp}`** — new `DockConfig`, `dockConfigFromValues`,
  `dockIconSize`, and `resolveDefaultDockPins` (moved from the deleted
  `DockPins`).
- **`shell/src/shellcontroller.{h,cpp}`** — `m_settings`/`m_pins`/watcher
  replaced by `m_settingsClient` + `m_dockConfig`; `writeDockSetting()` is the
  one write path (optimistic local apply, daemon `Set` mirror, echo
  de-duplicated by `m_localSettingsWrite`); `onSettingsChanged()` applies a
  daemon-originated change; `seedDefaultDockPins()` seeds the installed pins
  once when `dock.pinned` is empty. `applyDockSettings`/`dockPosition`/
  `computeDockOverflow` read the typed view.
- **`shell/tests/tst_dockcore.cpp`** — file-persistence tests removed; new
  typed-view / icon-size / default-pin / re-layout tests.
- **`shell/tests/tst_settingsclient.cpp`** (new) — mock store + a fake
  `org.dragonfruit.Settings1` service on a private bus
  (`dbus-run-session`); asserts `GetAll`, `Changed`, `Set` and the
  de-duplication; skips the bus half if no bus.
- **`Cargo`/`CMake`** — `dockpins.cpp`/`docksettings.cpp` removed from
  `dragonfruit-shell-dockcore`, `settingsclient.cpp` added; dockcore links
  `Qt6::DBus`.
- **ADR [0032](design/adr/0032-shell-settings-client.md)**; track doc
  `08-settingsd-live-settings.md` gained a "shell consumes settingsd" section.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build` — 19/19 green (incl. `tst_dockcore` 49+1 and the
  new `tst_settingsclient` under `dbus-run-session`).
- `make lint` exit 0.

Live visual check (`/tmp/opencode/t44-capture.sh`): started the built
`dragonfruit-settingsd` on the session bus with a scratch `XDG_CONFIG_HOME`,
launched the nested demo, captured at `dock.size=0.5`
(`/tmp/opencode/t44-before.png`) then `gdbus … Set dock.size "<0.9>"`
(`/tmp/opencode/t44-nested.png`). Evidence: shell log
`Dock settings changed (live: dock.size)`; settingsd persisted `dock.size:
0.9` and the seeded `dock.pinned`; the light Dock panel measured 335x65 ->
413x78 px. Dock crop `/tmp/opencode/t44-after-dock.png`; vision read a
centered rounded Dock with six icons (pinned Konsole/Firefox, running apps,
Downloads, Trash). No `docs/captures/` file — the T-08 track owns
`t08-settingsd.*`.

Gotchas for later tasks:

- **Absent daemon is graceful.** The dev/demo tool still does not start
  settingsd; the shell then runs from the mirrored schema defaults + in-memory
  writes. The demo Dock seeds and resizes without a daemon.
- **`dock.pinned` re-seeds on empty.** There is no "file existed" check via
  D-Bus, so an intentionally emptied pin set comes back next launch. T-08.3
  can add a marker if that matters.
- **New schema key => update `settingsSchemaDefaults()`** (and
  `dockConfigFromValues` if the Dock consumes it). Renames still fail the
  frozen v1 manifest test in `settingsd`.
- **Daemon `Changed` for `dock.size` takes the full reconfigure path**, not
  the size-only divider path; a live settings-app drag would rebuild entries
  until made smarter.
- **T-08.2b**: bind `Theme.dark`/`Theme.reducedMotion` on the same
  `SettingsClient` (`appearance.colorScheme`, `appearance.accent`,
  `accessibility.reduceMotion`); do not add a second bus connection or timer.

## T45 — T-08.2b Design-system Theme binding

**State: done.** The design-system `Theme` singleton's `dark`/`reducedMotion`
are bound to settingsd through the same `SettingsClient` the Dock uses. The
acceptance — "Theme flips live from a settingsd change" — is green headlessly
and visually.

What landed:

- **`shell/src/themebinding.{h,cpp}`** (new) — `ThemeBinding` is the one
  writer of `Theme.dark`/`Theme.reducedMotion`. It consumes the shell's
  `SettingsClient` (no second bus connection/watcher/timer), reacts to
  `changed`/`refreshed`, and maps `appearance.colorScheme`
  (`light`/`dark`/`auto`) and `accessibility.reduceMotion`. `auto` follows the
  host `QStyleHints` color scheme and stays live via `colorSchemeChanged`; the
  pure resolver is `ThemeBinding::darkForScheme(scheme, hostDark)`.
- **`shell/src/shellcontroller.{h,cpp}`** — creates one `ThemeBinding` right
  after the QML engine and calls `apply()`; the Dock reconfigure path no
  longer writes `Theme.reducedMotion` (single writer). The compositor mirror
  (`ShellProtocol::setReducedMotion`) stays.
- **`shell/tests/tst_themebinding.cpp`** (new) — `MockSettingsClient` +
  `ThemeBinding` + a real `QQmlEngine`: scheme flip asserts `Theme.dark` and
  `Theme.color.surface`; reduced motion asserts the collapsed token duration;
  and a `GalleryContent` pixel grab proves the gallery variant re-renders
  (dark/light sunken background).
- **`Cargo`/`CMake`** — `themebinding.cpp` added to `dragonfruit-shell` and to
  the new `tst_themebinding` target (which pulls the design-system + gallery
  plugins); env `QML2_IMPORT_PATH` for the Dragonfruit module.
- **ADR [0033](design/adr/0033-theme-binding-single-writer.md)**; track
  doc `08-settingsd-live-settings.md` gained a "design-system Theme follows
  settingsd" section; the `045-…` acceptance box is ticked.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R tst_themebinding --output-on-failure` — 6 passed.
- `ctest --test-dir build` — 20/20 green (was 19).
- `make lint` exit 0; `make demo DEMO_ARGS=--headless` exit 0 (clean teardown).

Live visual check (`/tmp/opencode/t45-capture.sh`): settingsd on the session
bus with a scratch `XDG_CONFIG_HOME`, seeded `appearance.colorScheme=dark`,
nested demo, then `gdbus … Set appearance.colorScheme "<'light'>"`. Menu-bar
background pixel went `(45,37,52)` (dark `chrome`) → `(255,255,255)` (light
`chrome`); the Dock panel changed too. Captures
`/tmp/opencode/t45-before-dark.png` / `t45-after-light.png`; vision on a tight
menu-bar strip pair read dark top / white bottom, identical content, no
artifacts. The compositor backdrop stayed dark (T-08.2c).

Gotchas for later tasks:

- **One writer.** Do not assign `Theme.dark`/`Theme.reducedMotion` anywhere in
  shell QML/`ShellController`; `ThemeBinding` owns both. A new appearance key
  means a branch in `ThemeBinding::apply()` plus the mirrored default in
  `settingsSchemaDefaults()`.
- **`appearance.accent` is unconsumed.** `Theme.color.accent` is a read-only
  scheme token, so an override is a design-system change; T-09.2 (Appearance
  pane) is the natural owner.
- **Compositor still dark.** `appearance.colorScheme` reaches only the shell
  Theme today; T-08.2c must mirror it into `DfState::set_color_scheme` (the
  existing `setReducedMotion` protocol path is the template).
- **`auto` and the host.** The binding replaces the singleton's
  `Application.styleHints` binding, so host-following now depends on the
  `QStyleHints::colorSchemeChanged` connection in `ThemeBinding`; keep it when
  touching the class.
- **No `docs/captures/` file.** The T-08 track owns `t08-settingsd.*` for the
  scripted flip; T45's evidence is under `/tmp/opencode/`.

## T46 — T-08.2c Compositor motion/input policy migration

**State: done.** The compositor now consumes the settingsd motion/input policy;
no local settings file remains on either side. The shell is the only forwarder
from the one `SettingsClient`; the compositor is the only applier.

What landed:

- **`protocols/dragonfruit-toplevel.xml`** — `df_toplevel_manager` is v5 with
  two additive requests: `set_motion_policy(color_scheme,
  titlebar_double_click, minimized_animation)` (strings, `allow-null`) and
  `set_input_policy(repeat_delay_ms, repeat_rate_hz, gestures_enabled,
  gesture_space_switch, gesture_mission_control)`. `set_reduced_motion` (v3)
  is unchanged.
- **`compositor/src/shell/mod.rs`** — `MANAGER_INTERFACE_VERSION = 5`; the two
  requests dispatch to `DfState::set_motion_policy`/`set_input_policy`.
- **`compositor/src/state.rs`** — `set_motion_policy` (color scheme, titlebar
  double-click, minimized animation) and `set_input_policy` (clamped repeat +
  gesture flags via `apply_input_settings`), a `minimized_animation` field, and
  the minimize/restore collapse in `begin_window_motion_from`.
- **`compositor/src/window/motion.rs`** — `MinimizedAnimation`
  (`genie`/`scale`/`none`); `none` collapses the tween, `genie` renders as
  `scale`.
- **`compositor/src/input/gestures.rs`** — `GestureConfig` gained `enabled`,
  `space_switch`, `mission_control` and an `admits()` gate used by
  `update_swipe`/`update_pinch`.
- **`compositor/src/input/synthetic.rs`** — `query policy` reports the live
  scheme, titlebar double-click, minimized animation, repeat delay/rate and
  gesture flags (the headless end-to-end observation).
- **`shell/src/compositorpolicy.{h,cpp}`** (new) — the pure `CompositorPolicy`
  view + `compositorPolicyFromValues`/`resolveCompositorColorScheme` (`auto`
  follows the host).
- **`shell/src/shellprotocol.{h,cpp}`** — `setMotionPolicy`/`setInputPolicy`,
  guarded on `m_managerVersion >= 5`. **Bug caught live:** the manager was
  bound with a hard-coded `std::min(m_managerVersion, 4u)`; it now binds
  through `df_toplevel_manager_interface.version`, so a lockstep bump needs no
  second cap. (The conformance suite bound the advertised version directly and
  did not catch this; only the nested demo did.)
- **`shell/src/shellcontroller.{h,cpp}`** — `applyCompositorPolicy()` on
  `SettingsClient::changed`/`refreshed`, on host `colorSchemeChanged`, and once
  after authenticate; the first apply always sends. The old `setReducedMotion`
  call in `applyDockSettings` was removed (single forwarder).
- **Tests** — `compositor/tests/shell_protocol_conformance.rs::motion_and_input_policy_requests_apply_live`
  (drives v5, reads `query policy`, proves a disabled gesture family stops
  firing and re-enabling restores it); `gestures::tests::gesture_policy_gates_each_family`;
  `shell/tests/tst_compositorpolicy.cpp` (new).
- **Docs** — ADR [0034](design/adr/0034-compositor-policy-via-shell-bridge.md);
  `docs/private-protocols.md` v5 list; track doc
  `08-settingsd-live-settings.md` T-08.2c section; `02-compositor.md` scheme
  wording.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance` —
  33/33 green (the pre-existing v4 assertion was updated to v5).
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor gesture`
  — 12/12 green.
- `ctest --test-dir build` — 21/21 green (incl. `tst_compositorpolicy`).
- `make e2e` green (all suites + clean `make demo --headless`). `make lint` exit 0.

Live visual check (`/tmp/opencode/t46-capture.sh`): settingsd on the session bus
with a scratch `XDG_CONFIG_HOME` seeded `appearance.colorScheme=dark`, nested
demo, then `gdbus … Set appearance.colorScheme "<'light'>"`. Captures
`/tmp/opencode/t46-before-dark.png` / `t46-after-light.png`. Shell log applied
`scheme=dark` then `scheme=light`; 265617 pixels differ. The compositor-drawn
SSD titlebar (traffic lights) of both the Wayland settings window and the X11
`xmessage` dialog read dark (#333-ish) → white; vision on tight before/after
titlebar pairs reported identical traffic lights with only the background
changing, no artifacts. Menu bar (45,37,52)→(255,255,255) and Dock
(42,33,50)→(241,240,241) followed too.

Gotchas for later tasks:

- **Extending the policy:** add the key to the `SettingsClient` schema defaults,
  `compositorpolicyFromValues`, the compositor's `set_*_policy` + a new
  `since` protocol request, and the `query policy` report. Never add a second
  D-Bus connection or a local file (ADR 0034).
- **The shell's manager bind version is now `df_toplevel_manager_interface.version`.**
  A new protocol request no longer needs a matching `std::min(..., Nu)`.
- **`dock.minimizeIntoTileIcon` is Dock entry visibility, not a compositor
  key** — it is not forwarded.
- **`genie` minimize** is accepted but renders as `scale` (material-pass
  follow-up).
- **`workspaces.count`** still has no compositor owner; it is a workspace-model
  key, out of T-08.2c's motion/input scope.
- **T-08.3** owns restart/resync: `refreshed` already re-applies the policy, but
  a `kill -9`/reappear test and the scripted `t08-settingsd.*` capture remain.

## T47 — T-08.3 Restart, resync, and key-schema documentation

**State: done.** `settingsd` is restartable with no lost write and the shell
re-syncs on reappearance; every key's owner/consumer is documented in-repo and
guarded by a test; the scripted `t08-settingsd.*` capture is committed.

What landed:

- **`docs/settings-keys.md`** (new) — the human-facing key table (type,
  default, constraints, owner, consumer, summary), a consumer map, and the
  restart/resync contract. `services/settingsd/tests/schema_doc.rs` (new)
  parses the table and fails on any drift from `services/settingsd/src/schema.rs`
  (key/type/default/owner/consumer).
- **`services/settingsd/tests/restart.rs`** (new) — spawns the real
  `dragonfruit-settingsd` binary (`env!("CARGO_BIN_EXE_dragonfruit-settingsd")`)
  on a private `dbus-daemon`, sets two keys, `kill -9`s it, asserts the
  well-known name is released, writes a value straight to the durable file
  while it is down, restarts it, and asserts `GetAll` returns both pre-kill
  writes plus the while-down change.
- **`shell/src/settingsclient.{h,cpp}`** — **real bug fixed.** The live client
  now watches `org.dragonfruit.Settings1` with a `QDBusServiceWatcher` instead
  of `QDBusConnectionInterface::serviceRegistered`/`serviceUnregistered`.
  Those interface signals only fire for a connection's **unique** name, so the
  shell never re-synced after a real restart of an external settingsd (it
  resynced once via the constructor's [`GetAll`] and then never). ADR 0032
  gained a consequence bullet.
- **`shell/tests/tst_settingsclient.cpp`** — the restart/resync case now serves
  the fake daemon from a **separate bus connection** (`QDBusConnection::
  connectToBus`), making it remote exactly like a real process; it fails
  without the watcher.
- **`scripts/capture-settingsd.sh`** (new) + `make settingsd-capture` —
  produces `docs/captures/t08-settingsd.{png,mp4,txt}` and
  `-before-dark/-after-light/-down/-restart.png`.
- **Docs** — track doc `08-settingsd-live-settings.md` gained a T-08.3 section;
  `docs/captures/README.md` describes the capture; ADR 0032 notes the watcher.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-settingsd` — 26 unit + 1 restart + 1 schema_doc +
  4 session_bus green.
- `ctest --test-dir build -R tst_settingsclient --output-on-failure` — green.
- `make lint` — all 21 ctest tests green incl. `tst_settingsclient`; clippy,
  fmt, token/design/desktop-name gates green.
- `make e2e` green (all Rust suites + clean `make demo --headless`).

Live capture (`bash scripts/capture-settingsd.sh`): settingsd on the session
bus with a scratch `XDG_CONFIG_HOME`, nested demo, seed dark/0.5, flip
`appearance.colorScheme=light` + `dock.size=0.75`, `kill -9`, write dark/0.35
to the file while down, restart. Raw pixels: menu bar `(45,37,52)` (before) →
`(255,255,255)` (after flip) → unchanged after kill → `(45,37,52)` on restart
(the while-down value); Dock region differs between the 0.75 and 0.35 frames.
`t08-settingsd.txt` is the D-Bus transcript + persisted file. Vision on tight
menubar/dock crops reported no artifacts; the raw pixel values are the
load-bearing evidence (a stacked-sheet vision pass got the strip order's colors
wrong).

Gotchas for later tasks:

- **Use `QDBusServiceWatcher`, not `QDBusConnectionInterface` signals, for the
  well-known name.** The latter cover unique names only.
- **Adding a key:** `schema::KEYS` (+ bump `SCHEMA_VERSION`, set `since`),
  the mirrored `settingsSchemaDefaults()` in `shell/src/settingsclient.cpp`,
  and `docs/settings-keys.md` — the new doc test enforces the doc.
- **`tst_settingsclient`'s restart case is in-process** (separate connection,
  not a separate process); the separate-process half is the Rust `restart.rs`.
- **Do not rename/remove a v1 key**; the frozen manifest and the doc test both
  guard the documented v1 set.

## T48 — T-09.1a Settings app shell

**State: done.** The `apps/settings` stub is now a real shell: frameless
Tier-1 window with design-system titlebar/traffic lights, sidebar + local pane
search, back/forward history, and a pane header card. The four Wave-1 pane
rows are advertised; their bodies land in T-09.2…T-09.5.

What landed:

- **`apps/settings/`** is a reusable QML module `Dragonfruit.Settings` (static
  lib `dragonfruit-settings-ui` + plugin) and the thin `dragonfruit-settings`
  executable — the gallery/module split, so the app and the QML test render the
  same surfaces.
- **`apps/settings/SettingsPanes.qml`** (new, singleton) — the ordered catalog
  from `docs/reference/System_Preferences.md`:
  `{ id, title, icon, description, shipped }`; `shippedPanes`/`filter(query)`.
  Wave-1 `shipped` ids (reference order): `appearance`, `desktop-dock`,
  `displays`, `wallpaper`.
- **`apps/settings/SettingsShell.qml`** (new) — `AppWindow` + `TitleBar`,
  `Sidebar` + `SearchField`, `Toolbar` back/forward, history
  (`selectPane`/`back`/`forward`, `canGoBack`/`canGoForward`), `PaneHeader`,
  and a placeholder note per unfilled pane. Alt+Left/Right history, Ctrl+F
  search focus.
- **`apps/settings/PaneHeader.qml`**, **`SettingsWindow.qml`** (new) — header
  card; frameless window mapping shell intent to `Window` state
  (`startSystemMove` for drag).
- **Design system** — `Icon.qml`: `chevron-left` + Canvas pane glyphs
  (`appearance`, `wallpaper`, `dock`, `displays`); `Button.qml`:
  `accessibleName`; `Sidebar.qml`: untitled sections render no header row.
- **Tests** — `apps/settings/tests/tst_settings_shell.{cpp,qml}` (new, 11
  cases): opens, search filter, keyboard nav + Enter, history, AT-SPI roles,
  close forwarding, selected-row accent pixel check.
- **Docs** — ADR [0035](design/adr/0035-settings-shell-and-pane-catalog.md);
  track doc shell section; task hand-off.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R tst_settings_shell --output-on-failure` — 11/11.
- `ctest --test-dir build` — 22/22 (no regression; the design-system suite
  still passes after the `Icon`/`Button`/`Sidebar` additions).
- `make lint` green; `make e2e` green (all Rust suites + clean
  `make demo --headless`).
- Live capture (`/tmp/opencode/t48-capture.sh`): nested demo with
  `dragonfruit-settings` as the Qt/Wayland client. The selected row is accent
  #b32a66 (5162 accent px in the sidebar); vision on a tight crop reported
  titlebar "Settings", "Search" + magnifier, the four rows, magenta selected
  row, header card, and no artifacts.

Gotchas for later tasks:

- **Add a pane by flipping its `SettingsPanes` entry to `shipped: true`**,
  writing its body in `apps/settings/`, and (if new) adding its `Icon` glyph.
  Never add a row before the pane works — that is the no-half-panes gate
  (ADR 0035).
- **The sidebar's selection highlight animates over `motion.hover` (100 ms).**
  Pixel-sampling tests must `wait()` it out (see
  `test_selected_sidebar_row_is_accent_tinted`).
- **The shell is a module, not an executable body**: reuse `SettingsShell` in
  tests; `SettingsWindow` only maps window-control intent.
- `SettingsPanes.catalog` holds the full reference IA; only `shippedPanes`
  render. Search is local over shipped panes (id/title/description,
  case-insensitive).
- The app's `Theme.dark` follows the host style hint, not the compositor
  scheme (existing T-04.4b/T-08 follow-up), so a nested light window can sit on
  a dark compositor default.

## Follow-ups

- **T-19.2 Applications drawer: the deferred Spotlight-equivalent search pane.**
  The drawer's search field filters its own `buildAppsDrawerList` corpus only;
  app-index-backed *global* search (apps + files + settings) is still deferred
  and must not replace the drawer's local filtering. The drawer's category
  pills, the `appsDrawer` token block, and the `show-apps` shortcut are reusable
  by it.
- **T-19.2 `show-apps` (F4) is not rebindable through Settings yet.** It is a
  default in `default_system_bindings()`; the Settings Keyboard pane (T-16)
  owns rebinding and has not been extended for the new action.
- **T-15.11a has no live distro group provider yet.**
  `dragonfruit-account-adapter` ships the `GroupProvider` seam and the mock;
  the concrete provider belongs with packaging (T-16.9/T-16.10) because
  AccountsService has no group API. `HostAccounts` therefore runs with
  `groups: None` (a normal layered absence) until one is attached, so the
  Users & Groups pane shows a live user list, disabled group controls, and the
  "no group provider" note. Attach a `Box<dyn GroupProvider>` to the
  `AccountsHost` in `services/system-status/src/main.rs` to light it up.
- **T-15.11b drops `Network account server` from the reference pane.** There is
  no enterprise directory join provider, so the row (and its `Edit…` button)
  is omitted rather than shipped as a dead control. A future directory-join
  task can add the provider and the row together.
- **T-15.11b Control Center panel is at its nested-output height ceiling.**
  The fourteenth (Users) tile fit by compacting every tile's vertical padding
  from `spacing.xs` to `spacing.xxs` (content ~1108 px inside 1140 px). A
  fifteenth tile needs a taller nested output: a scrolling panel is not
  reachable because `ShellProtocol::onPointerAxis` is a no-op and the
  compositor forwards no pointer-axis events to overlay surfaces.

- **T-15.10a has no live distro update provider yet.**
  `dragonfruit-update-adapter` ships the `UpdateProvider` seam and the mock;
  the concrete provider (the `SystemProvider` of
  `docs/design/08-settings.md`) belongs with packaging (T-16.9/T-16.10) and is
  not implemented here. `HostSystem` therefore runs with `updates: None`
  (a normal layered absence) until T-15.10b or packaging attaches one. The
  adapter itself is fully tested through `MockSystem`/`MockUpdateProvider`.
- **T-15.9b auto-hide mode and recent-items count are stored policy.**
  `menu.autoHide` (revision 16) and `menu.recentItems` round-trip live and the
  Menu Bar pane reflects them, but the shell does not consume them: the menu-bar
  chrome surface stays pinned and no recent-items consumer exists. Applying
  auto-hide needs the compositor input-region/hover contract; the recent count
  needs a recents source. The clock options, the background material, and the
  per-control `menu.control.*` visibility all apply live.
- **T-15.9b/T-15.10b Control Center is at its nested-output height ceiling.**
  The panel is now 360×1160; T-15.10b fit a thirteenth tile (Software Update)
  by tightening every tile's internal padding to the `spacing.xs` token (the
  inter-tile gap was already `xs`), leaving the content at ~1097 px inside the
  1140 px the surface gives it. There is no room for a fourteenth tile: the
  nested demo output is 1920×1200 and leaves only 1164 px below the bar. The
  next Control Center tile task must make the panel scroll (or grow the nested
  output), not just bump the constant or shave more padding.
- **T-15.10b has no live distro update provider yet (inherits T-15.10a).**
  The `Updates` bridge interface and both consumers are live, but
  `dragonfruit-system-status` attaches no provider to `HostSystem`, so a real
  session shows the About rows live and `updatesAvailable: false` (the update
  controls disabled, the tile subtitle `Software Update Unavailable`). The
  concrete `SystemProvider` lands with packaging (T-16.9/T-16.10); attaching it
  to `HostSystem` in `services/system-status/src/main.rs` is the only wiring
  change then.
- **T-15.9a richer clock options and Apple-only controls are not modelled.**
  `dragonfruit-menubar-adapter` models the two clock options the shell's
  `MenuBarClock` actually renders (`showDate`, `showSeconds`) and the seven
  controls whose slots exist in `shell/menubar/MenuBar.qml`. The macOS
  `Clock Options...` rows (day of week, 24-hour, flashing separators) and the
  Apple-only `AirDrop`/`Screen Mirroring`/`Display` controls are deliberately
  omitted rather than shipped as dead controls. Adding a clock option means
  teaching `MenuBarClock` to render it first, then extending `ClockOption`.
- **T-15.8b lock-screen display keys are stored policy.** The four `lock.*`
  display keys (revision 15) round-trip live and the Lock Screen pane reflects
  them, but `shell/lock/LockScreen.qml` does not yet read them: it always
  shows the user name/photo and has no custom-message path (its `message`
  property is the auth status line, a different concept). Wiring the keys into
  the lock-screen renderer (and hiding the name/photo, hints, message, and
  power buttons accordingly) is a follow-up. The `idle.blank`/`idle.lock`
  timing keys already apply live through the session idle engine.
- **T-15.7b notification presentation prefs are stored policy.** The four
  settingsd keys (`notifications.showPreviews`, `notifications.showWhenSleeping`,
  `notifications.showWhenLocked`, `notifications.showWhenMirroring`, revision
  14) round-trip live and the Notifications pane reflects them, but the
  notification service does not yet read them to gate a banner. Enforcement
  (the service consulting settingsd, or the shell banner path) is a follow-up.
- **T-15.7b per-app notification detail.** The macOS per-app disclosure sheet
  has no Linux provider: the Application Notifications rows are a read-only
  inventory (`status` + count) with no chevron, and per-app notification policy
  is reduced to the Focus allow list. A richer per-app model would extend
  `NotificationsSnapshot`/`AppNotifications` in the service and adapter.
- **T-18.1a live-network coverage.** The provider's catalogue/download path is
  tested through `MockSource`; CI never hits Wikimedia (no network, and a full
  first run downloads up to 60 × 3840 px images). The live `UreqHttp` path was
  smoke-tested manually (empty cache + `HTTPS_PROXY=http://127.0.0.1:1` →
  immediate `offline`); if a live test is wanted, gate one category behind an
  `--ignored` test.
- **T-18.1a provider launch in the dev harness (resolved).**
  `launch_services` in `tools/dragonfruit-dev/src/main.rs` now starts
  `dragonfruit-wallpaperd` alongside app-index and menu-broker for the nested
  human demo and `dev --shell`, so opening Settings → Wallpaper shows the
  Featured catalogue. The `--headless` scripted demo (`make demo
  DEMO_ARGS=--headless`, hence `make e2e`) deliberately skips it, so CI never
  warms the provider's cache or hits the network. Selected by the pure
  `service_names(wallpaper)` helper (covered by a unit test).
- **T-18.1a cache size.** The provider caps 10/category and downloads
  sequentially, but does not cap the total cache size or evict old items; add
  an LRU/size cap if the install footprint becomes a concern.
- **T-18.1a default path canonicalization.** `DefaultResolver::resolve` returns
  the in-tree path with its `../..` segments (absolute, not canonicalized); the
  compositor's loader handles it, but a future task could canonicalize.
- **T-14.7f drop capture follow-up.** The committed `t14-dock-drops.png` drives
  the Dock's production presentation through the `DF_DOCK_DROP_FIXTURE` seam
  because the harness has no reusable external drag source. A small scripted
  DnD source client (offer `text/uri-list` + `application/x-dragonfruit-app`,
  `start_drag`, synthetic motion) would exercise the enter-time read and the
  real `wl_data_offer` path end-to-end for the capture; the compositor
  conformance test already covers the target half.
- **T-14.7f enter-mime ordering.** `ShellProtocol::beginDndRead` picks the mime
  from the offer's advertised set at enter and reads it once. A compositor that
  sends `enter` before the offer's `mime` events would read nothing (the drop
  then falls back to no-op); the real compositors send offer → mimes → enter, so
  this is only a hardening note.
- **T-14.5 XDnD follow-up (the connection/runtime half).** T-14.5 landed the
  XDnD protocol model (`compositor/src/xdnd.rs`) and a live-Xwayland
  conformance test but not the bridge itself. Remaining: (a) a bridge X11
  connection to Xwayland (its own `x11rb` client; Smithay's `X11Wm` connection
  and atoms are private), (b) a bridge window carrying `XdndAware = 5` and an
  event loop watching `ClientMessage`/`SelectionNotify`, (c) serving
  `XdndSelection` as `text/uri-list`, and (d) calling Smithay's `start_dnd`
  (or feeding a server DnD grab) on an accepted `XdndDrop`. This is the
  X11-source → Wayland-target direction; the Wayland-source → X11-target
  direction additionally translates `ClientDndGrabHandler::started` into
  `XdndEnter`/`Position`/`Drop` towards an Xwayland window. Recorded as a
  known, waived gap at T-17 (ADR [0099](design/adr/0099-xdnd-bridge-model-and-documented-gap.md)).
- **T-14.3 follow-ups.** (a) Live SNI `NewIcon`/`NewToolTip`/`NewStatus` signals
  are not subscribed: the shell re-reads `TrayItems` on a 2 s timer, so icon
  updates and attention animation lag or are missed. (b) A pixmap-only item
  (`IconPixmap`, no `IconName`) falls back to the `application` glyph, and the
  item's `IconThemePath` is not consulted; resolve both when a real app needs
  them. (c) Left-click always opens the DBusMenu; a primary `Activate`
  (and middle-click/scroll) path is not wired. (d) `dragonfruit-app-index` is
  not auto-started by `dragonfruit dev`/`make demo`, so the live tray needs the
  service started by hand (the same gap as settingsd/menu-broker).
- **T-14.2b follow-ups.** (a) The shell still computes its accelerator table
  from the menu it renders (`fixedApplicationMenu` + `demoAppMenu`); it does not
  read the broker's `Resolve`/`FocusedAccelerators` over D-Bus, and Settings
  still does not call `Publish` (the T-14.2a transport follow-up). (b) An
  accelerator action that is not a shell-owned verb (e.g. `edit.undo`) is
  routed to `ShellController::dispatchAppAction` but only logs; the app-facing
  action channel is still to come, so the compositor→shell half of dispatch is
  real and tested while the shell→app half is not. (c) The accelerator table is
  registered per focused app in `applyFocusedApp`; when focus leaves a
  published app its table stays registered in the compositor until the next
  registration (harmless — only the focused app matches).
- **T-14.2a follow-ups.** (a) The shell does not consume
  `org.dragonfruit.MenuBroker1` yet: it computes the fixed application menu's
  live state locally (`shell/src/menubrokerpolicy`) and still uses the interim
  `demoAppMenu()` for the exporting app's menus. T-14.2b should push
  `SetWindowStates`/`SetFocusedApp` and read `Resolve`, and Settings should
  call `Publish` with its `SettingsMenu.publishedModel` over the native
  channel. (b) Hide/Hide Others/Show All dispatch their `action` but only log;
  actually hiding an app's windows needs a new compositor window-state request.
  (c) The `DbusMenu` tier is defined but unpopulated (T-14.4).
- **T-14.7 follow-ups (the remaining global-menu halves).** (a) The shell
  consumes `org.dragonfruit.MenuBroker1` now (T-14.7), but a clicked bridged
  `dbusmenu:<id>` row still only logs: route it back through app-index's
  `WindowMenuEvent(windowId,id)` (the window→registration mapping lives in
  app-index). (b) Settings still does not call `Publish` with its
  `SettingsMenu.publishedModel`, so the `native` tier never lights; add a
  publish client (the shell's `MenuBrokerClient` is in dockcore and not linked
  by `apps/settings`, so either link it or add a small shared client). The
  DBusMenu tier (T-14.4) is already pushed by app-index and now renders.
  (c) `menu.global=false` suppresses exported menus in the bar but the broker
  still resolves them; no functional gap, just a redundant resolve.
- **T-14.1c follow-ups.** (a) The service subscription API is live but the shell
  does not consume it: `shell/src/appindexclient.{h,cpp}` has no `Subscribe` and
  `ShellController::start` still loads the index once at startup. Wire the shell
  to call `Subscribe("all")` on the session bus and reload
  `DesktopEntryIndex` on the directed `Changed` signal so installs/uninstalls
  reach the Dock without a restart (and drop the startup `Enumerate`). (b) The
  `identity,recency,icons` kinds are noted by the service mutation paths only;
  a consumer that subscribes must still call `Running`/`Recent`/`Enumerate`
  itself — the signal carries kinds, not a snapshot (by design).
- **T-14.1b follow-ups.** (a) The shell does not yet forward window activity:
  the service exposes `WindowOpened`/`WindowClosed`/`NoteActivity` but nothing
  calls them live, so the Dock's running projection still comes from the
  compositor; wire the forwarder when a consumer needs the registry. (b) Recency
  is in-memory per session; persistence across restarts is future work. (c) The
  inotify watcher is Linux-only (non-Linux is a no-op), matching the shipping
  target.
- **T-14.1a follow-ups.** (a) `Enumerate` resolves and serializes every entry
  on each call; the icon resolver now memoizes and caches directory listings
  (cold ~0.2s, was ~8s), but T-14.1c's subscription should let the shell stop
  calling `Enumerate` at all. (b) The shell keeps its local `.desktop` scan
  (`DesktopEntryIndex::scan`) as an absent-service fallback; T-14.7 deletes it
  once the app-index launch API lands. (c) The in-repo `org.dragonfruit.*`
  desktop files are not installed, so on a host session the Dock's first-party
  pins resolve from the system's Terminal/Browser categories; packaging
  (T-16) installs them.
- **T-13.7 follow-ups.** (a) Clipboard has no portal (ADR 0082); the Flatpak
  capture proves FileChooser/Screenshot/ScreenCast but not a Flatpak↔native
  clipboard round-trip in the nested session (T-13.5a covers the compositor
  data device with native clients). A Flatpak-vs-native clipboard still would
  need a small Wayland clipboard client in the sandbox. (b) A live PipeWire
  ScreenCast producer is still the named stills fallback; that is T-13.4b's
  documented gap, not new here.
- **T-13.5b follow-ups.** (a) The Control Center clipboard section shows the
  five most recent entries only; a dedicated popover with image thumbnails,
  entry removal, search, and the pinned/clear-all controls from legacy T-29 is
  a presentation follow-up that needs no store or protocol change. (b) There is
  no headless test of the C++ `wlr-data-control` client (it needs a live
  compositor); `tst_clipboardhistory` covers the store and the Control Center
  tests cover the view. A Rust data-control conformance client would close that
  gap. (c) Copying an entry re-serves it but does not move it to the top of the
  history until the compositor's own selection echo is observed; harmless but
  worth normalizing.
- **T-13.3b follow-ups.** (a) The compositor still-shot capture is implemented
  on the nested (GL) backend only; headless and DRM explicitly reply
  `screenshot_failed`. The T-13.4b PipeWire screencast path, or a later pass,
  should serve the real-DRM session. (b) The capture crop treats coordinates as
  output-local pixels at scale 1 with a Normal transform; fractional-scale and
  rotated outputs need the transform-aware crop. (c) The shell's copy uses
  `QGuiApplication`'s clipboard, not the compositor's data device; T-13.5 owns
  the real Wayland round-trip and can reuse `ScreenshotWriter::copy`.
- **T-12.4a follow-ups.** (a) The idle engine (ADR 0070) has no production
  caller yet: no idle service binds the compositor's `ext-idle-notify` global,
  and the compositor has no dim/blank path, so T-12.4b/T-12.5a must wire the
  engine to a real activity source and action sink. (b) The `idle.*` policy
  keys exist only as the `IdlePolicy::from_keys` contract; T-12.5b must
  register them in settingsd (schema `since` bump) and keep
  `docs/settings-keys.md` in lockstep.
- **T-12.3c follow-ups.** (a) The shell types passwords from a fixed US/ASCII
  evdev table (`shell/src/lockinput.cpp`) because it is not an xkb client; a
  later task that installs an xkb state in the shell should replace it. (b)
  Killing the shell while locked keeps the session locked but Smithay's
  `SessionLockManagerState.locked_outputs` is not cleared, so a restarted shell
  cannot create a new lock surface for the same outputs without a compositor
  restart; the restart/kill matrix (T-12.5b) should decide whether to clear
  `locked_outputs` on lock-client disconnect while `LockModel.locked` stays set.
  (c) The input-capture integration test drives the lock client only; a
  second non-lock client asserting zero keyboard events would harden it.
- **T-12.3b follow-ups.** (a) Input capture is still T-12.3c: the lock surface
  is not reachable, and `ShellController::submitLockPassword(password)` is the
  slot T-12.3c must call once it routes keys to the lock UI. (b) Packaging must
  install `dragonfruit-pam-helper` on `PATH` and may install
  `/etc/pam.d/dragonfruit`; without the service file the helper falls back to
  `login`. (c) The helper runs only the PAM auth+account phases, so session
  modules (keyring/systemd/selinux) do not run during unlock — intentional.
- **T-12.3a follow-ups.** (a) `ext_session_lock_manager_v1` has an open client
  filter (`|_| true` in `DfState::new`); restrict it to the trusted shell during
  T-12.3c hardening. (b) While locked the compositor drops *all* input, so the
  lock surface is not yet reachable; T-12.3c specifies capture and routes
  keyboard/pointer to the lock surface. (c) The shell's lock object is left
  alive on shell shutdown while locked so a crash keeps the session locked;
  T-12.3b should ensure the PAM success path calls `unlock_and_destroy` before
  any shell restart. (d) The lock UI shows a fixed `$USER` and a static clock;
  T-12.3b replaces the inert password field with PAM and T-12.4 adds idle
  auto-lock.
- **T-12.1b follow-ups.** (a) The shipped units are not yet installed by a
  package; T-12.2/packaging must place `services/session/units/*` under
  `~/.config/systemd/user/` (or `/usr/lib/systemd/user/`) and add the
  `.desktop` session entry that runs `--print-env` + `import-environment` +
  `systemctl --user start dragonfruit-session.target`. (b) The compositor is
  `Type=simple`; a systemd `Type=notify` (`sd_notify` after the private socket
  exists) would make the gate native instead of the `--wait-socket`
  `ExecStartPre`. (c) `--wait-socket` tests file existence, not a connect;
  revisit with T-12.2's logout teardown. (d) `Supervisor::shutdown` still
  SIGKILLs only direct children; T-12.2 extends it to process groups.
- **T-10.4a follow-ups.** (a) The files-core bridge is deliberately not
  introduced (ADR 0048); T-10.4b adds it and attaches the listing to
  `FilesBrowser.currentUri`. (b) The trailing view-options dropdown, sort
  controls, and selection are deferred with the views (T-10.4b). (c) The local
  search field updates `FilesShell.searchText` but there is no result set until
  the listing exists; scope it with the views. (d) Sidebar volumes are a
  `QStorageInfo` block-device read; network/GVfs volumes, mount-on-demand, and
  eject wait for the volume monitor (track item 5 / T-10.6). (e) Sidebar
  favorites drag-reorder and Add to Sidebar are not implemented. (f) The Files
  app does not consume settingsd yet, so its `Theme` is the build default until
  a later binding task (the shell and Settings sync through `ThemeBinding`).
  (g) The Favorites `Recents` row is omitted because `recent://` needs the GVfs
  backend; add it with the volume monitor (T-10.6).
- **T-10.4b follow-ups.** (a) `FilesDirectoryModel` resets from a **full
  ordered snapshot** on every poll; correct but O(n²) over a large directory —
  T-10.5 must switch the C ABI to incremental/windowed delivery (ADR 0049).
  (b) The folder watcher is not wired, so external changes do not repaint.
  (c) The search field still has no result set. (d) Per-location sort
  persistence and the trailing view-options dropdown are not done (sort is
  global for the session). (e) The live visual check used
  `DF_FILES_START_VIEW` because nested synthetic pointer clicks reach the
  compositor but not a Qt client surface; a nested-client click driver is
  future capture-harness work.
- **T-10.4c follow-ups.** (a) Rubber-band (marquee) selection in icon view is
  not implemented; modifier clicks and Cmd+A are. (b) `Open` is enabled for
  folders only; file opening, Open With, Get Info, Copy, Duplicate, Compress,
  and Make Alias are deferred, so an item menu shows only Open / Rename /
  Move to Trash. (c) A confirmed trash is not undoable (Put Back is a later
  task); the row simply disappears. (d) The operation worker stays
  synchronous per call; a batch is a loop of single-item optimistics, so a
  large multi-select trashes one worker round-trip per row — fold into one
  batch in T-10.5. (e) The inline rename editor commits on Return and cancels
  on Escape but does not pre-select the base name sans extension (Finder
  behaviour) or validate illegal names before the worker reports the error.
  (f) Multi-select uses the QML `FilesShell.selectedIds` set; the Rust
  `Selection` is unused by the app (fine for one window, revisit for the
  portal chooser).
- **T-09.3 follow-ups.** (a) `apps/settings/AppearancePane.qml` (T50) places
  its controls as plain children of `SettingsRow`; `SettingsRow` has no
  `default` property, so they go to `Item.data` and can overlap the label.
  Wrap each in `controlData:` (the gallery form T-09.3 uses). (b) Wallpaper
  per-Space distinct persistence is last-selection-only; a stable Space
  identity exposed to settingsd (T-16) is needed to persist a map. (c) The
  `WallpaperCache` is not cleared when a source path is reused with new bytes
  (T-05.4 note); a replaced file at the same path still shows the old raster.
- **T-09.4 follow-ups.** (a) `design-system/Select.qml`'s menu is a plain
  `Item` child of the `Select` and is clipped by the pane's `ScrollView`
  (`clip: true`); popup rows must stay in the initial viewport until a
  window-level overlay hosts them (the existing design-system `Popup` has the
  same limitation). (b) The Desktop & Dock pane's reference "Desktop & Stage
  Manager" group is omitted — `Show items` / `Click wallpaper to show desktop`
  need Desktop Reveal / hot-corner keys, which land with T-15.5. (c) Gallery
  goldens were not regenerated (matches T-50/T-51): `make gallery-snapshot`
  rewrites all 72 because of host font differences; `make visual-test`
  (non-strict) only checks the freshly generated set.
- **T-09 Settings Wave 1 (T-09.1a done in T48).** The shell, sidebar, local
  search, history, header card, and the four Wave-1 sidebar rows landed in
  `apps/settings` (`Dragonfruit.Settings` module, ADR 0035). Remaining:
  T-09.1b live-apply plumbing; T-09.2–T-09.5 fill the Appearance, Wallpaper,
  Desktop & Dock, and Displays pane bodies and flip each `SettingsPanes` entry
  to `shipped: true`; T-09.6a publishes the app menu model; T-09.6b records the
  absence matrix and wave captures.
- **T-08.2 consumer migration (T-08.2a done in T44, T-08.2b done in T45,
  T-08.2c done in T46).** T44 deleted the shell's `DockSettings`/`DockPins`
  and `QFileSystemWatcher`; the Dock now reads/writes through the
  `SettingsClient` seam (ADR 0032). T45 bound `Theme.dark`/`reducedMotion` to
  `appearance.colorScheme`/`accessibility.reduceMotion` via `ThemeBinding`
  (ADR 0033). T46 forwards the motion/input keys over the private
  `df_toplevel_manager` v5 `set_motion_policy`/`set_input_policy`; the
  compositor is the sole applier (ADR 0034). T-08.3 (done in T47) made the
  shell re-sync on a settingsd restart (QDBusServiceWatcher) and documented the
  key schema. Remaining: `appearance.accent` was consumed in T-09.2
  (`Theme.accentOverride`, ADR 0037); `workspaces.count` has no compositor
  owner yet (workspace-model key, not
  motion/input). Nothing starts
  `dragonfruit-settingsd` (the dev tool deliberately does not); the shell runs
  from the mirrored schema defaults then, which is intended.

- **T-07.6 signal wiring (not done).** The bridge host
  (`services/system-status`) reads an adapter only on startup, an explicit
  `Refresh()` (menu open), and after an action; it never subscribes to
  NetworkManager / WirePlumber (`pw-mon`) / UPower `PropertiesChanged`. So a
  daemon change is not reflected until the user opens the menu (or acts). The
  design doc calls this wiring "T-07.6" but it did not land in T-07.6a/b; it
  is the remaining piece of the track's "within one event" acceptance. The
  idle-trace guarantee (T41) deliberately relies on the absence of polling, so
  any wiring must be event-driven, not a timer.
- T-07.5b (done in T39): battery menu + `--placeholders` removal + keyboard
  a11y landed. Remaining: (a) no process starts `dragonfruit-system-status`
  yet — the dev tool deliberately does not; the session/systemd unit (or a
  `dragonfruit dev` flag) must, or live items hide; (b) T-07.6 wires the
  daemon signals (`PropertiesChanged`/`pw-mon`) and validates absence, then
  T-07.6b records the track capture.
- T-07.2a (done in T34): NetworkManager **read** path + mock/fixture landed
  (`services/networkmanager`, ADR 0026). Remaining: (a) T-07.2b (done in T35,
  ADR 0027) added activate/join + polkit read-only degradation to the same
  crate; (b) the D-Bus source needs a signal subscription
  (`PropertiesChanged`, `DeviceAdded/Removed`) and a host to call `refresh()` —
  no process hosts the Rust adapters for the C++ shell yet (decide the bridge
  in T-07.5); (c) the live source (read and write) is compile-checked only.
- T-07.1a (done in T32): the adapter contract + mock landed
  (`services/system-adapters`, ADR 0024); T-07.1b (done in T33, ADR 0025) added
  the subscription/re-subscribe API. Remaining: (a) no process yet hosts the
  Rust adapters for the C++ shell — decide the bridge (D-Bus service vs.
  embedded) in T-07.2/T-07.5; (b) `--placeholders` stays until T-07.5b; (c)
  T-07.2–T-07.4 must classify connect failures as `Absent` vs `Failed` and
  implement `connection()`/`drain_events()`; (d) T-07.5 drains `AdapterEvent`s
  and must not add a poll loop.
- T-06.1/T-06.2a/T-06.2b (done in T29/T30/T31): the switcher machine (ADR
  0021), overlay + live previews (ADR 0022), and commit/Cmd+`/pointer (ADR
  0023) landed. Remaining: (a) the switcher snapshots the app list at open, so a
  window closing mid-hold is not reaped until the next open — widen if the
  overlay needs live membership; (b) the shell card row is a non-wrapping
  `Row`, so many apps need paging/wrapping; (c) the preview grid and the shell
  cards agree on the strip height but not per-card geometry — align them
  (labels under each preview) as polish; (d) only one live surface per app
  previews (the cursor window); per-window cards are later polish.
- T-05.1a (done in T22): the live-surface grid landed as a render-time
  transform (ADR 0017). Remaining: (a) T-05.1b (done in T23) keeps a committing
  client ("video") playing at the scaled target and composes the degrade tier
  through `overview_grid_material`;
  (b) neighbor-Space reveal — draw the adjacent Spaces' surfaces (unmapped
  from `Space`) as extra `GridCandidate`s; (c) T-05.2 (done in T24) transfers
  hit-testing to the interpolated `GridPlacement` rects via
  `overview_window_at`; (d) T-05.3 (done in T25) drags the live representation
  onto the workspace-strip card under the pointer and reuses `GridPlacement`
  (ADR 0018); the drop target widens to the revealed Space regions when (b)
  lands; (e) per-output chrome insets for the grid area; (f) paging at
  `MIN_GRID_SCALE`; (g) compose the reusable `SceneTransform`'s clip/blur
  attachments onto third-party client surface elements (the renderer still has
  no clip element).
- T-04.4b (done in T21): the live scheme is on `DfState` (ADR 0016) and
  `window/degrade.rs` already scales the scheme-resolved spec. Remaining: (a)
  T-08 must mirror `appearance.colorScheme` into `set_color_scheme` (today only
  the synthetic `set color-scheme` and the dark default drive it); (b) the
  QML shell chrome (Dock/menu bar) does not follow the compositor scheme, so a
  real light session will show light compositor SSD and dark QML chrome until
  T-08/T-17 wire one scheme to both sides; (c) a *real* texture-sampling blur
  (T-04.2 follow-up) would make the before/after capture visibly sharper than
  the current feather-stack proxy.
- T-04.3 (done in T19): the reusable `SceneTransform`/`SceneTransformPass`
  landed and the lifecycle motion renders through it. Remaining: (a) fold the
  T-04.2 chrome backdrop **element generation** into the pass (the `FramePass`
  guard is already shared; `BackdropPass`/`MaterialRole` are the seam); (b)
  apply the carried `CornerMask` clip to live third-party client surface
  elements when T-05 composes the grid (the T-04.1b gap remained by design).
- T-04.2 follow-up: replace the flat-tone backdrop with a **real
  texture-sampling blur** (Kawase vs dual-pass Gaussian) behind the same
  tokens; the legacy FR-1 live-content test (video under the bar) needs it.
- T-04.4a (done in T20): the degrade ladder/selector landed
  (`window/degrade.rs`, ADR 0015); `DegradeTier` is the knob for backdrop and
  shadow, `degrade stats` is the instrument. Remaining: (a) T-04.4b wires the
  light/dark scheme into the degraded passes (render still uses
  `ColorScheme::default()`); (b) a *real* measurement on nested/DRM should
  record a frame trace with the tier moving under load (headless never renders).
- T-04.1a/2: chrome surfaces (menu bar, Dock, popovers, OSD) still get no
  compositor shadow; they rely on QML `Shadow`. The shared
  `component.elevation` group is ready for the compositor chrome pass and for
  the `Overlay` level.
- T-08: `render::window_shadow_render_elements` uses `ColorScheme::default()`
  (dark); wire the live scheme when settingsd owns it.
- Add the QML import-dir define (`DF_QML_IMPORT_DIR`) to the first-party apps'
  CMake so they run without `QML_IMPORT_PATH`; today only the shell has it and
  the demo harness supplies the env.
- T-16: publish `_NET_FRAME_EXTENTS` for Tier-2 X11 windows so clients can
  position menus/tooltips relative to the compositor titlebar.
- Zoom (and any future re-layout) on an inset change should reuse
  `DfState::reapply_insets`; today only a decoration-tier change reflows.
- T-04: draw the window-menu labels/icons and the material pass (replaces
  `WindowMenu::render_elements` only; keep the geometry/routing).
- T-04: compose clip/blur on `MotionFrame::scale_for(current_size)` for client
  surfaces (not `frame.scale`, which is target-relative and chrome-only).
- T-04 (or later): animate `DfState::reapply_insets` geometry changes with the
  same `WindowMotion`; a decoration-tier flip on a zoomed window still jumps.
- T-14: reuse `WindowMenu`/`WindowMenuCommand` for decoration themes; add the
  current-Space check glyph to the Move to Space submenu.
- T-01.6b (done in T07): the nested window-menu/walkthrough capture landed as
  `scripts/capture-demo.sh` + `docs/captures/t01-loop-v0.*`.
- Capture a live CSD still when a CSD client is available; the demo only
  launches an SSD Wayland app and an X11 app, so the CSD matrix is headless
  only today.
- Per-window scene-element refactor (T-04) still owns the stacked-titlebar /
  menu interleaving limitation.
- T-02.2/T-10: the compositor now remembers the app-keyed Dock tile
  (`DfState::dock_tiles`) and reuses it for appear/minimize/restore; the shell
  still must thread the Dock entry's tile rectangle from QML through
  `ShellController::launchDockAppWithFiles` to `ShellProtocol::setLaunchOrigin`
  (needs the T-23 `desktopId`→`app_id` resolver). Until then real launches use
  the centered fallback.
- T-02.4b (done in T13): close interruption/reversal and the idle trace
  landed. Remaining follow-ups: (a) the T-07 shell/Dock "activate a closing
  window" path should call `DfState::interrupt_close` once it exists; (b)
  `after_workspace_change` should mirror `active_window`/`Unfocused` when it
  drops focus (same Smithay unset gap as close); (c) T-04 composes
  scale/clip/blur onto the same `MotionFrame`.
- T-03.1b (done in T15): the latency fields are appended after
  `client_wakeups` and the `ScanoutCounter` template landed (ADR 0010);
  do not reorder existing fields.
- T-03.4: fill the DRM half of the template on hardware — record the
  on-hardware input-to-photon latency (the instrument is backend-agnostic)
  and prove `direct_scanouts` engages for an unobstructed fullscreen client;
  state that the measurement stops at `queue_frame`, not vblank.
- T-03.2/T-03.4: hook `DrmCompositor`'s frame-callback delivery so
  `client_wakeups` is exact on DRM instead of the `count_output_windows`
  upper bound; record the DRM 60 s trace alongside the nested one.
- T-05.4 (done in T26): image wallpaper decode/cache + per-Space slide
  (ADR 0019). Remaining: (a) `DfWorkspace::SetWallpaper` does not invalidate
  `DfState::wallpaper_cache` (add `clear()`/per-path invalidation when
  settingsd/T-16 drives wallpaper changes); (b) nested capture of the live
  slide is T-05.6; (c) per-output wallpaper animation polish is T-16.
- T-05.6 (done in T28): gesture frame budget (ADR 0020) + nested captures land.
  Remaining: (a) the nested shell overview chrome does not forward pointer to
  QML, so live-window click selection / drag-between-Spaces in the *shell*
  session is unverified beyond the headless conformance tests — wire the
  overview layer's input region/forwarding (T-05.2/T-05.3 polish or T-11), then
  re-drive a pointer drag in `scripts/capture-overview-driver.py`; (b) the
  shell's QML window cards are a centered `Flow` that does not align with the
  compositor's `GridPlacement` live surfaces — align them (or route the card
  tap/drag through `overview_begin_drag`) so pressing the visible live surface
  works; (c) DRM gesture trace beyond the T-03 smoke is T-16 (the instrument is
  backend-agnostic); (d) the nested gesture is over budget on the dev iGPU, so
  a later perf pass (real texture blur, fewer shadow layers under `Reduced`) is
  the mitigation path.
- T-07.3 (done in T36): audio adapter landed (`services/audio`, ADR 0028).
  Remaining: (a) T-07.5 owns the event bridge — it must call
  `AudioAdapter::refresh()` on a WirePlumber change (`pw-mon`/`pactl subscribe`)
  and render the slider from `AudioSnapshot`; (b) the live source is the
  WirePlumber CLI (`pw-dump`/`wpctl`), not a native link — T-15 can swap a
  `libpipewire` client in behind `AudioSource`; (c) writes target the default
  sink only (per-sink/routing is T-15); (d) the absent-daemon matrix across all
  three adapters is T-07.6a.
- T-07.4 (done in T37): power adapter landed (`services/power`). Remaining:
  (a) T-07.5b renders the read-only battery item — it must hide it both when
  `AdapterState::Unavailable` and when `PowerSnapshot::present()` is false, and
  optionally add a charging bolt to `StatusGlyph.qml`; (b) T-07.5 owns the
  event bridge — it must call `PowerAdapter::refresh()` on a UPower
  `PropertiesChanged`/device signal, never poll; (c) power profiles
  (`power-profiles-daemon`) and any writes are T-15.
- **T-10.1b follow-ups.** (a) files-core name collation is natural/numeric but
  **not locale-aware**; the design's one-collation rule and the cross-locale
  matrix (legacy FR-5) need a collation implementation in a later task.
  (b) A symlink's folders-first/sort kind is its own `Symlink` kind, not its
  target's kind (the design says a broken symlink "sorts by its target's
  kind"); that needs a follow-stat at list time in the source.
- **T-10.3b follow-ups.** (a) The watcher is non-recursive and watches one
  directory (correct for "one monitor per visible directory"), but a **self
  delete/move of the watched folder** (`IN_DELETE_SELF`/`IN_MOVE_SELF`) is not
  surfaced — the watcher just goes quiet; a later task should turn it into an
  error state (the model already has `ListingState::Failed`). (b) A GIO/GVfs
  `GFileMonitor` backend is still owed behind `FolderWatcher` (removes
  `SANCTIONED_WATCHER_FALLBACK_MARKER`, ADR 0047) and will carry remote
  `GVfs` change events. (c) Optimistic reconciliation by the watcher is
  wired, but there is no timeout policy: a pending op whose filesystem change
  never arrives stays pending forever (the `*_via` path resolves
  synchronously, so today this only matters for an async T-10.4 bridge).
- **T-11.4b follow-ups.** (a) Hardware volume/brightness media keys are still
  not wired to the OSD (no compositor input action); route them into
  `ShellController::showOsd`. (b) The live cross-process AT-SPI tree dump and
  keyboard-only walkthrough are T-16.6a; T-11.4b only asserts the per-item
  roles and the Escape dismissal. (c) `MockNotificationClient` does not model
  DND banner suppression, so the DND capture dismisses the seeded banner by
  pointer; the real suppression is the Rust policy.
- **T-14.7a follow-up — the Dock divider's cross-axis placement is wrong.**
  `Dock.qml`'s `layout` computes a divider's along-axis position with the
  entry height (`sizes[j] = dividerWidth = 1`) but its object height is
  `barThickness - 2 * padding`, so on a bottom Dock the divider is drawn well
  below the plate (mostly clipped by the surface); on a left/right Dock its
  width/height are also swapped (it draws a short vertical hairline instead of
  a horizontal separator). This is pre-existing and outside T-14.7a's
  spacing/plate scope; the divider artwork and orientation are T-14.7j. The
  T-14.7a containment tests skip dividers for that reason.

- **T-14.7c follow-ups.** (a) The Dock's drag-gap *close* only springs for the
  cancel/snap-back path; a successful reorder still resets the Repeater model
  through the shell (`onDockPinnedOrderChanged` → `rebuildDockEntries`), so the
  gap closes on the fresh delegates. Making the shell apply the pin order
  optimistically (no model reset) would close that. (b) The pre-sized popover
  buffer keeps the whole Dock window larger (`dock + 2*320` by
  `dock + 320`), so every `grabWindow()` readback is ~4x the resting area; the
  T-14.7c nested trace shows no degrade-tier downgrade, but a real-hardware
  trace that shows one should move the popover into its own fixed-size
  offscreen window (the task's documented fallback).

- **T-16.3b shell-side fractional chrome (sharpness).** The compositor now
  advertises each output's fractional scale to its chrome surfaces
  (`render::update_chrome_preferred_scale`), but the shell's hand-rolled
  Wayland client (`shell/src/shellprotocol.cpp`) renders QML offscreen at a
  device pixel ratio of 1 and binds neither `wp_fractional_scale` nor
  `wp_viewporter`. At a fractional output scale the shell ignores the
  preferred-scale event, so its chrome surfaces are 1x rasters the compositor
  upscales (visibly soft in `/tmp/opencode/t146-frac-active.png`). Fixing it
  needs the shell to render each chrome `QImage` at the output scale and map
  it back with `wl_surface.set_buffer_scale` (integer) or
  `wp_viewport.set_destination` (fractional); all chrome commits already funnel
  through `ShellProtocol::commitTo`, so that is the one choke point. The
  compositor half and the per-output chrome *sizing* are done (ADR 0153).

## T49 — T-09.1b Settings live-apply plumbing

**State: done.** The Settings app is a real settingsd consumer: a QML `Settings`
singleton over the shared C++ client lets panes bind to keys and write them
live, and a headless test round-trips a stock design-system `Toggle` through a
fake service and the real `dragonfruit-settingsd`. The `--placeholders`
Settings window was already replaced by the T-09.1a shell.

What landed:

- **`libs/settings-client/`** (new; `libs/CMakeLists.txt`) — the former
  `shell/src/settingsclient.{h,cpp}` moved here verbatim and built as the
  static library `dragonfruit-settings-client` (ADR 0036). One client, one
  D-Bus dialect, one schema-default table for the shell and the app.
  `dragonfruit-shell-dockcore` no longer compiles it; it links the library
  `PUBLIC` (its include dir propagates, so `shellcontroller.h` /
  `themebinding.cpp` / the shell tests are unchanged otherwise).
- **`apps/settings/SettingsBridge.{h,cpp}`** (new) — a C++ QML singleton
  registered as **`Settings`** in `Dragonfruit.Settings`. Surface:
  `values` (reactive `QVariantMap`, NOTIFY `valuesChanged`), `available`,
  `value(key, fallback)`, `set(key, value)`, `refresh()`, `keys()`, and a
  per-key `changed(key, value)`. `DF_SETTINGS_FIXTURE` selects
  `MockSettingsClient`; else `DbusSettingsClient` (defaults + in-memory writes
  when no daemon).
- **`apps/settings/tests/tst_settings_live.cpp`** (new) — binds a stock
  `Toggle` to `accessibility.reduceMotion` via the two-way pattern
  (`onToggled -> Settings.set`, `Binding -> Settings.values[...]`) and runs
  three cases: the fixture (no bus), a fake `org.dragonfruit.Settings1` on the
  private bus, and the **real `target/debug/dragonfruit-settingsd`** over
  `dbus-run-session` with a scratch `XDG_CONFIG_HOME`. Each asserts the write
  reaches the owner and an external `Changed` flips the control without a
  restart.
- **`apps/settings/CMakeLists.txt`** — `SOURCES SettingsBridge.*` on the
  `Dragonfruit.Settings` QML module + link `dragonfruit-settings-client`; tests
  CMake locates `dragonfruit-settingsd` (`target/debug|release`) and passes
  `DF_SETTINGSD_BIN`.
- **Docs** — ADR [0036](design/adr/0036-shared-settings-client-and-qml-singleton.md);
  track 09 gained a "Live-apply plumbing (T-09.1b)" section with the binding
  pattern; track 08's T-08.2a section points at the shared library.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R tst_settings_live --output-on-failure` — 3 cases
  pass (fixture, fake service, real settingsd) in ~0.17 s.
- `make qml-test` — 23/23; `make lint` green; `make e2e` green (all Rust
  suites + clean `make demo --headless`).
- Live capture (`/tmp/opencode/t49-capture.sh`,
  `/tmp/opencode/t49-settings-live.png`): nested `make demo`, Settings is the
  Qt/Wayland client. The window renders titlebar "Settings" + traffic lights,
  "Search" + magnifier, the four rows, the accent-selected Appearance row, and
  the pane placeholder card; vision reported no clipping/overlap/stray
  artifacts. No new visual surface exists this slice, so the capture is the
  shell/app rendering check, not a live-apply demo.

Gotchas for later tasks:

- **Panes use the `Settings` singleton, never D-Bus.** `Settings.values[key]`
  (bracket the dotted key) is the reactive read; `Settings.set(key, value)` is
  the write. The shipped two-way pattern is in track 09 and
  `tst_settings_live.cpp`; a bare `checked: Settings.values[...]` breaks on the
  first user tap, so restore with a `Binding` element or a `Connections` slot.
- **`DF_SETTINGS_FIXTURE` is the deterministic test/capture switch** (mirrors
  `DF_STATUS_FIXTURE`). Without it a QML test talks to whatever is on the
  session bus — always set it or run under `dbus-run-session`.
- **The shared client is at `libs/settings-client`**; `shell/src/settingsclient.*`
  no longer exists. Add a key in three places as before: `schema.rs`, the
  mirrored `settingsSchemaDefaults()` (now in the lib), and
  `docs/settings-keys.md`.
- `appearance.accent` is consumed as of T-09.2 (`Theme.accentOverride`,
  ADR 0037); `workspaces.count` still has no compositor owner.
- The Settings app now mirrors `appearance.colorScheme`/`appearance.accent`/
  `accessibility.reduceMotion` onto its own `Theme` (T-09.2), so it follows
  settingsd, not the host style hint (the earlier T-04.4b/T-08 gap is closed
  for this app; other first-party apps still follow the host until they add
  the same bindings).

## T50 — T-09.2 Appearance pane

**State: done.** The Appearance pane is real and live: a Light/Dark/Auto
segmented control bound to `appearance.colorScheme` and an accent swatch row +
custom hex picker bound to `appearance.accent`, all through the `Settings`
singleton. The accent is now consumed across the desktop — the design-system
`Theme` gained a writable `accentOverride`, written by the shell's
`ThemeBinding` and mirrored onto the Settings app's own `Theme`.

What landed:

- **`apps/settings/AppearancePane.qml`** (new) — `SettingsGroup`/`SettingsRow`
  with a `SegmentedControl` (Light/Dark/Auto) and an accent swatch row
  (`Repeater` of circular swatches; Default = token accent) plus a `Custom…`
  `Popup` with a `#rrggbb` `TextInput` and live preview. Every control uses the
  two-way pattern: `onActivated`/`onTapped` -> `Settings.set(key, value)`, and
  a `Binding` back to `Settings.values[...]` (the scheme selector) or a
  `selected` binding (the swatches).
- **`apps/settings/SettingsShell.qml`** — a `paneComponent(id)` registry and a
  `Loader` (`paneBody`) render the body; the placeholder shows only when no
  body is registered. Three `Binding`s mirror `appearance.colorScheme`,
  `appearance.accent`, and `accessibility.reduceMotion` onto the app-local
  `Theme` (the shell's `ThemeBinding` is another process). Exposes
  `paneBody` for tests.
- **`design-system/Theme.qml`** (generated) — `property string accentOverride`
  (empty = token accent) plus `hasAccentOverride`/`accentOverrideColor`; the
  accent family (`accent`, `accentHover`, `accentMuted`, `accentContent`) of
  both schemes is emitted as bindings over it. `scripts/gen-tokens.py` owns the
  emission; `compositor/src/design_tokens.rs` is unchanged.
- **`shell/src/themebinding.{h,cpp}`** — `appearance.accent` -> 
  `Theme.accentOverride`, reacted to on `changed`; `tst_themebinding.cpp`
  gained `accentOverrideFlipsTheme`.
- **Tests** — `apps/settings/tests/tst_settings_appearance.{cpp,qml}` (new, 5
  cases, `DF_SETTINGS_FIXTURE=1`): pane loads with the wired controls; scheme
  selection flips the key and `Theme.dark`; each swatch sets the key and
  `Theme.accentOverride`; the custom hex picker applies/validates; reduced
  motion mirrors onto `Theme.reducedMotion`.
- **Docs** — ADR [0037](design/adr/0037-accent-override-and-app-local-theme-sync.md);
  track 09 "Appearance pane (T-09.2)"; `docs/settings-keys.md` consumer map.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R tst_settings_appearance --output-on-failure` —
  5/5 (fixture, ~0.12 s).
- `make qml-test` — 24/24; `make lint` green; `make check-tokens` current;
  `make visual-test` — 66 gallery snapshots unchanged; `make e2e` exit 0.
- Live captures: `/tmp/opencode/t50-capture.sh` (nested demo, Appearance pane)
  and `/tmp/opencode/t50-accent-capture.sh` (real `settingsd` on the session
  bus, accent flipped to `#4a7dff` mid-session). Raw observation: the Dock
  running-app indicator pixel was `#b32a66` before and `#4a7dff` after, and the
  Settings window's accent region changed with it — shell + app applied the
  accent live. On a tight crop of the pane, vision reported titlebar
  "Settings", sidebar with Appearance selected, header, Light/Dark/Auto, six
  swatches + `Custom…`, and no clipping/artifacts.

Gotchas for later tasks:

- **Register a pane body in `SettingsShell.paneComponent(id)`**, add the QML
  file to `apps/settings/CMakeLists.txt` (`QML_FILES` + `df_qml_lint`), and
  leave/put its catalog `shipped` true. The placeholder shows when the registry
  returns `null`.
- **The app must mirror Theme itself.** `ThemeBinding` runs in the shell
  process; a first-party app's `Theme` is per-process. Copy the three
  `Binding`s in `SettingsShell.qml` (or a shared helper) into any new app that
  renders settings controls (ADR 0037).
- **`Theme.accentOverride` is the accent knob** (string `#rrggbb`, empty =
  token). Deriving `accentHover`/`accentMuted`/`accentContent` is done in the
  generator; do not set `Theme.color.accent` anywhere (it is read-only).
- **Pane tests set `DF_SETTINGS_FIXTURE=1` and reset keys in `init()`** — the
  mock store is process-global, so state leaks between cases otherwise.
- **Omitted reference rows are deliberate** (Highlight color, Sidebar icon
  size, wallpaper tinting, scroll bars: provider T-15.x; reduced motion stays
  Accessibility T-15.14). Add them only with real keys — no-half-panes.
- Settings app still follows the host style hint only when the three bindings
  are absent; with the shell loaded it follows settingsd.

## T51 — T-09.3 Wallpaper pane

**State: done.** The Wallpaper pane is real and live: our own gradient
collections plus a portal "Add Photo…", a "Show on all Spaces" toggle, and a
fit control, all bound to settingsd; the shell forwards the selection to the
compositor's per-Space T-05 model. Selecting a wallpaper changes the Space
and the compositor background within one beat, and it persists in settingsd.

What landed:

- **Schema (since 2)** — `wallpaper.source` (s, empty = solid),
  `wallpaper.fit` (s enum fill/fit/stretch/center), `wallpaper.showOnAllSpaces`
  (b, default true). `services/settingsd/src/schema.rs` gained a `Wallpaper`
  key group and `SCHEMA_VERSION` is now `2`; the migration is additive and the
  v1 frozen-key test still passes. Mirrored in
  `libs/settings-client/settingsclient.cpp` and `docs/settings-keys.md`.
- **Shell forwarder** — `shell/src/wallpaperpolicy.{h,cpp}` is the pure
  `WallpaperSettings` mapping (`CompositorPolicy` analogue, unit-tested by
  `shell/tests/tst_wallpaperpolicy.cpp`). `ShellProtocol::setWallpaper` sends
  `df_workspace.set_wallpaper` to every Space, or only the active one when
  `showOnAllSpaces` is false; it remembers the selection and re-applies on
  `df_toplevel_manager.done` so a policy that predates the Space list is not
  lost. `ShellController::applyWallpaperPolicy` runs on settingsd
  `changed`/`refreshed` and once at startup.
- **Compositor** — `df_workspace.set_wallpaper` now changes only the image
  `source`/`fit` (`WorkspaceModel::set_wallpaper_image`), keeping the Space's
  solid `color`, so a NULL source returns the Space to its default color. New
  unit test in `compositor/src/workspace/mod.rs`.
- **`apps/settings/WallpaperPane.qml`** (new) — hero preview (the exact file
  the compositor decodes), current name, "Show on all Spaces" toggle, fit
  `SegmentedControl`, two original gradient collections, and "Add Photo…".
  `SettingsBridge` renders six original gradient PNGs to
  `$XDG_DATA_HOME/dragonfruit/dragonfruit-settings/wallpapers/` and exposes
  `Settings.wallpaperPresets`; the portal FileChooser call and URI→path
  conversion live there (`wallpaperChooserAvailable` gates the button).
  `Settings.startPane` (`DF_SETTINGS_START_PANE`) opens a pane at startup for
  captures. Registered in `SettingsShell.paneComponent`; `wallpaper` was
  already `shipped: true`.
- **Tests** — `apps/settings/tests/tst_settings_wallpaper.{cpp,qml}` (7 cases,
  `DF_SETTINGS_FIXTURE=1`): presets load; tile select / all-Spaces / fit apply
  live; an external `Settings.set` converges; the photo path applies; the row
  controls are right-aligned.
- **Docs** — ADR [0038](design/adr/0038-wallpaper-pane-settingsd-and-shell-forwarder.md);
  track 09 "Wallpaper pane (T-09.3)"; `02-compositor.md` set_wallpaper
  semantics; `docs/settings-keys.md` rows + consumer map.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R "tst_settings_wallpaper|tst_wallpaperpolicy" --output-on-failure`
  — 7 + 5 cases pass.
- `make qml-test` — 26/26; `make lint` green; `make e2e` exit 0.
- Live capture (`/tmp/opencode/t51-capture.sh`): real `dragonfruit-settingsd`
  on the session bus, nested demo with `DF_SETTINGS_START_PANE=wallpaper`, then
  `wallpaper.source` set to a preset mid-session. Raw observation: the
  compositor background pixel was `(33, 13, 41)` before and `(37, 97, 100)`
  after; vision on the window crop read sidebar rows Appearance/Desktop &
  Dock/Displays/Wallpaper, header "Wallpaper", "Current wallpaper", "Meadow",
  "Show on all Spaces" (toggle right, label fully visible), Fill/Fit/Stretch/
  Center, "Dragonfruit", no clipping.

Gotchas for later tasks:

- **`SettingsRow` has NO `default` property.** A control placed as a plain
  child goes to `Item.data` (direct child at x0) and overlaps the label. The
  correct usage is `controlData: Control { ... }` (the gallery pattern); ids
  inside `controlData:` work. `AppearancePane.qml` (T50) still uses the
  plain-child form — a latent left-overlap bug worth the same one-line fix
  (not done here: out of T51 scope).
- **Add a wallpaper key in three places**: `schema.rs`, the mirrored
  `settingsSchemaDefaults()`, `docs/settings-keys.md` (the doc test enforces
  the table). Bump `SCHEMA_VERSION` and set `since`.
- **`df_workspace.set_wallpaper` keeps the Space color**; the wire `color`
  arg is retained for the lockstep contract but unused. Per-Space distinct
  persistence is still last-selection-only (one source/fit pair); a stable
  Space identity exposed to settingsd is the T-16 follow-up.
- **Built-in wallpaper files are stable paths** under
  `QStandardPaths::AppDataLocation` (`.../dragonfruit-settings/wallpapers/`);
  settingsd persists the absolute path, and the app regenerates the same files
  if missing. An app that is never launched leaves the compositor with the
  solid fallback.
- `DF_SETTINGS_START_PANE=<id>` opens a shipped pane for captures/tests; an
  unknown id is ignored.

## T52 — T-09.4 Desktop & Dock pane

**State: done.** The Desktop & Dock pane is real and live: the `Dock` group
ships all ten `dock.*` control rows through the `Settings` singleton, and the
shell updates the Dock from the settingsd `Changed` on the next beat (its
existing T-08.2a applier). Two new design-system components, `Slider` and
`Select`, carry the slider and popup rows (ADR 0039).

What landed:

- **`apps/settings/DesktopDockPane.qml`** (new) — `SettingsGroup` "Dock" with
  `Slider` (Size `Small`/`Large`; Magnification `Off`/`Small`/`Large`),
  `Select` (position / minimized animation / titlebar double-click), and
  five `Toggle` rows. Two-way T-09.1b pattern throughout. Registered in
  `SettingsShell.paneComponent("desktop-dock")`; the catalog row was already
  `shipped: true`.
- **`design-system/components/Slider.qml`** (new) — `[from,to]` value,
  drag/tap/arrow/Home/End, optional `minLabel`/`midLabel`/`maxLabel`, `moved`
  + `committed` signals, `FocusRing`, AT-SPI `Slider`. Tokens
  `component.slider` (`trackHeight`, `knob`, `height`, `minWidth`, `captionGap`,
  `step`).
- **`design-system/components/Select.qml`** (new) — current label + chevron
  opening a `ContextMenu`; `activated(index)`/`selected(value)`, AT-SPI
  `ComboBox`. Tokens `component.select`. Both registered in the module +
  `df_qml_lint`, with gallery pages `Slider`/`Select`.
- **Tests** — `apps/settings/tests/tst_settings_desktop_dock.{cpp,qml}` (10
  cases, `DF_SETTINGS_FIXTURE=1`): wiring, every control applies live, external
  convergence, right-alignment. `design-system/tests/tst_design_system.qml`
  gained 5 Slider/Select cases (39→40 pass).
- **Docs** — ADR
  [0039](design/adr/0039-slider-and-select-design-system-components.md);
  track 09 "Desktop & Dock pane (T-09.4)"; `10-design-system.md` component list.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R "tst_settings_desktop_dock|tst_design_system"`
  — both pass.
- `make qml-test` 27/27; `make lint` green; `make visual-test` — 72 snapshots;
  `make e2e` exit 0.
- Live capture (`/tmp/opencode/t52-capture.sh`, stills
  `/tmp/opencode/t52-{before,scrolled,after}.png`): real
  `dragonfruit-settingsd` on the session bus, nested demo with
  `DF_SETTINGS_START_PANE=desktop-dock`, then `dock.size`/`dock.magnification`/
  `dock.showIndicators` changed mid-session. Raw observation: the pane's slider
  knobs moved from ~0.5 to near "Large" in step with the daemon. Vision on the
  maximized window read all ten rows and the header "Desktop & Dock" with no
  clipping/overlap. The 900x620 window cuts the last row off until the window
  is zoomed (double-click titlebar), so the zoomed still is the complete one.

Gotchas for later tasks:

- **`Select`'s menu is clipped by a pane `ScrollView`** (same as the
  design-system `Popup`): keep popup rows above the fold or add a window-level
  overlay. The three Desktop & Dock popups sit near the top.
- **A `Slider` writes per drag step via `moved`.** Bind `value` with a
  `Binding` element (T-09.1b), not a bare property binding, and write the key
  in `onMoved`. No local preview channel exists, so a drag emits a settingsd
  write per step; the shell's divider-preview is the precedent if damping is
  needed.
- **No new applier.** `ShellController::applyDockSettings` already reacts to
  `dock.*` `Changed`; the pane only writes keys. `dock.pinned` has no row.
- **Desktop & Stage Manager rows are omitted** pending T-15.5 keys (the
  no-half-panes rule); `10-design-system.md` now lists Slider/Select as
  library components, so later panes use them.
- The appearance pane's `controlData:` bug (T-09.3 follow-up (a)) is still
  open; T-09.4 uses `controlData:` everywhere.

## T53 — T-09.5 Displays-basic pane

**State: done.** The Displays-basic pane is real and live: the `Built-in
Display` preview, the five scaled-resolution tiles, and a rotation `Select`,
all bound to settingsd. The shell forwards `display.scale`/`display.rotation`
to the compositor's private output API (`df_output.set_scale` /
`df_output.set_transform`); the selection persists in settingsd (schema 3).

What landed:

- **Schema (since 3)** — `display.scale` (d, 0.5–2.0, default 1.0),
  `display.rotation` (s enum normal/90/180/270, default normal). New
  `KeyGroup::Displays`; `SCHEMA_VERSION` is now 3. Mirrored in
  `libs/settings-client/settingsclient.cpp` and `docs/settings-keys.md`.
- **Shell forwarder** — `shell/src/displayspolicy.{h,cpp}` is the pure
  `DisplaySettings` mapping (unit-tested by `shell/tests/tst_displayspolicy.cpp`).
  `ShellProtocol::setDisplayPolicy` stores the policy and remembers the
  announced `df_output` handles (`m_outputs`), applying it in `onManagerDone` /
  `onManagerOutput` so a policy that predates the output list is not lost.
  `ShellController::applyDisplayPolicy` runs on settingsd `changed`/`refreshed`
  and once at startup.
- **`apps/settings/DisplaysPane.qml`** (new) — `SettingsGroup` "Built-in
  Display" (our own preview artwork), "Rotation" (`Select`), and "Resolution"
  (five tiles `Larger Text`/`Large`/`Default`/`More Space`/`Most Space` over
  `display.scale`, plus the reference footer). Rotation is deliberately above
  Resolution so its `Select` popup stays above the pane `ScrollView` fold.
  Registered in `SettingsShell.paneComponent("displays")`.
- **Tests** — `apps/settings/tests/tst_settings_displays.{cpp,qml}` (4 cases,
  `DF_SETTINGS_FIXTURE=1`): wiring, every tile applies live, rotation applies
  live, right-alignment. `shell/tests/tst_displayspolicy.cpp` (5 cases).
  `compositor/tests/shell_protocol_conformance.rs` gained a `set_transform`
  round-trip assertion next to the existing `set_scale` one.
- **Docs** — ADR
  [0040](design/adr/0040-displays-config-via-settingsd-and-shell-forwarder.md);
  track 09 "Displays pane (T-09.5)"; `docs/settings-keys.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R "tst_settings_displays|tst_displayspolicy"`
  — 4 + 5 cases pass.
- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance
  handshake_chrome_and_control_conformance` — passes.
- `make qml-test` 30/30; `make lint` green; `make e2e` exit 0.
- Live capture (`/tmp/opencode/t53-final.sh`, stills
  `/tmp/opencode/t53f-{before,scale,rotated}.png`): real
  `dragonfruit-settingsd` on the session bus, nested demo with
  `DF_SETTINGS_START_PANE=displays`, window zoomed. Raw observation: the
  pane shows the preview, five tiles with `Default` selected, and Rotation
  `Standard`; no clipping. The shell logged `display applied (scale=1.000
  rotation=normal)` then `scale=1.250` then `rotation=90` as the daemon keys
  changed.

Gotchas for later tasks:

- **The nested backend does not faithfully re-render an output scale/transform
  change.** Its damage tracker is fixed at the host window size with
  `Transform::Flipped180`, so changing `output.current_scale()`/transform
  either leaves the frame unchanged (no repaint) or misrenders the desktop.
  The shell→compositor path is real and asserted by the conformance test; a
  DRM output applies it. Nested scaling is a T-16 item.
- **Displays config is settingsd-persisted**, not owned by the compositor
  (ADR 0040): the shell is the forwarder and the compositor the applier, the
  same shape as wallpaper (T-09.3) and motion/input (T-08.2c). A display
  change is applied to every announced output; per-display targeting is T-16.
- **`df_output.set_mode` is unused.** The pane's `Resolution` rows are the
  compositor's scaled-resolution model (`display.scale`), matching the
  reference UI. Exposing a real supported-mode list needs a `df_output` modes
  event (T-16).
- **Omitted rows are deliberate** (no-half-panes): Brightness, auto-brightness,
  True Tone, Preset, Refresh rate, Night Shift, Advanced, `Arrange…`.
- **The schema doc test enforces `docs/settings-keys.md`**: adding a key means
  `schema.rs` + the client mirror + the doc table; bump `SCHEMA_VERSION` and
  set `since`.

## T54 — T-09.6a Settings menu-model publication

**State: done.** The Settings app publishes its native menu model and the
shell's `MenuBar` consumes it unchanged. The cross-process transport
(focus → app → model) is the menu-broker's job (T-14.2a) and is deliberately
not built here — the task's "fixed app menu is enough until then."

What landed:

- **`apps/settings/SettingsMenu.qml`** (new; QML singleton) — single source of
  Settings' menus in the design-system normalized entry shape: fixed
  `applicationMenuItems` (About/Settings/Hide/Hide Others/Show All/Quit) and
  `menus` (File/Close Window; Edit/undo-redo-cut-copy-paste-select-all +
  separator; View/Enter Full Screen + a `Settings Pane` submenu over the four
  shipped panes; Window/Minimize/Zoom/Bring All to Front; Help/Settings Help).
  Each row carries an `action` string. `publishedModel` is the JSON-serializable
  wrapper; `actionFor(menuIndex,itemIndex)` follows the `MenuBarMenu`
  `triggered` contract; `activate(action,item)` emits `activated`.
- **`SettingsShell.qml`** exposes `menuModel: SettingsMenu.publishedModel`.
  **`SettingsWindow.qml`** wires `SettingsMenu.activated` to the window verbs
  (close/minimize/zoom/fullscreen) and `pane.*` jumps; `edit.*` are the standard
  editing verbs the broker routes to the focused text input (T-14.2b).
- **Consumption** — the shell's `MenuBar` takes the model as
  `applicationMenuItems` + `appMenuModel` with no translation. ADR
  [0041](design/adr/0041-native-menu-model-publication-shape.md) fixes the
  payload shape and the consumption surface, and leaves the channel to T-14.2a.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R tst_settings_menu --output-on-failure` — 1/1
  passed; direct run reports `Totals: 10 passed, 0 failed, 0 skipped`.
- `make lint` green (30/30 ctest incl. `qmllint_app-settings`); `make e2e`
  exit 0.
- Live capture (`/tmp/opencode/t54-capture.sh`; stills `t54-desktop.png`,
  `t54-window.png`, `t54-menubar.png`): nested demo with Settings open on
  Appearance. Raw observation: the bar renders brand mark + application menu +
  File/Edit/View and the clock/status icons; the Settings window renders the
  sidebar, header, Light/Dark/Auto segmented control, and accent swatches with
  no clipping. Vision on the window crop read the same rows and controls. The
  published Settings menus (File/Edit/View/Window/Help) do **not** appear in the
  bar — expected until T-14.2a.

Gotchas for later tasks:

- **The published model is the design-system entry shape** (`MenuBarMenu`'s
  `menuModel` input), not a shell-private shape. Rows carry `action` strings;
  no callbacks. Do not add a second model shape.
- **Transport is T-14.2a.** The app owns no well-known bus name (settingsd owns
  `org.dragonfruit.Settings1`); focus → bus-name resolution and dispatch belong
  to the menu-broker. `tst_settings_menu.qml` links
  `dragonfruit-shell-menubarplugin` to prove the round-trip headlessly, so the
  settings test target now depends on the shell menubar target.
- **Action dispatch is duplicated by design**: `SettingsMenu.activated` drives
  the app locally, and the broker will route the same action back. `edit.*` is
  not wired in-app (the focused text input handles the standard keys today).
- **`SettingsMenu` is a singleton**: add a new menu row by editing only this
  file; `SettingsShell.menuModel` and the global menu pick it up together.
- **Pre-existing tree fix**: T53 left `compositor/tests/shell_protocol_conformance.rs`
  unformatted and tripping `clippy::manual_contains` (new in rust 1.98), which
  made `make fmt-check`/`make lint` red before this task. Fixed to
  `state.output_transforms.contains(&1)` so the gate is green.

## T55 — T-09.6b Settings absence matrix and wave captures

**State: done.** The T-09 Settings wave is signed off: the absent-provider
matrix is documented and asserted headlessly, the wave captures are committed,
and the required live check's one real finding (the Appearance pane's
overlapping controls) is fixed.

What landed:

- **`docs/design/08-settings.md`** — new "The absent-provider matrix (T-09.6b)"
  section: the no-half-panes rule, the per-pane provider-absence table
  (settingsd and the xdg-desktop-portal FileChooser are the only observable
  providers), and the test/capture locations. No ADR: this documents the
  contract ADR 0035 already set.
- **`apps/settings/tests/tst_settings_absence.{cpp,qml}`** (new; 9 cases) —
  run under `dbus-run-session` with no settingsd, no portal, and no
  `DF_SETTINGS_FIXTURE`, so `Settings.available` and
  `Settings.wallpaperChooserAvailable` are both false. Asserts the four
  shipped panes stay live and write in memory, the no-half-panes catalog/body
  pairing, and the Wallpaper portal row is the only disabled control
  ("No file chooser is available.").
- **`apps/settings/AppearancePane.qml`** — fixed the documented T-09.3
  follow-up (a): the scheme `SegmentedControl` and accent `Row` are now
  `controlData:` (they were plain `SettingsRow` children and overlapped the
  labels). Added `schemeRow`/`accentRow` aliases and a right-alignment
  regression case in `tst_settings_appearance.qml`; `WallpaperPane.photoRow`
  alias added for the absence test.
- **Captures** — `scripts/capture-settings-wave-1.sh` (new; `make
  settings-wave-1-capture`) writes `docs/captures/t09-settings-wave-1.{png,
  light,dark,reduced}.png`, `t09-settings-wave-1-<pane>.png` for appearance/
  wallpaper/desktop-dock/displays, and `t09-settings-wave-1.mp4`; documented in
  `docs/captures/README.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `ctest --test-dir build -R "tst_settings_appearance|tst_settings_absence"`
  — 7 + 9 cases pass.
- `make lint` — 31/31 ctest, all checks green. `make e2e` — exit 0.
- `make settings-wave-1-capture` — host Wayland session + scratch settingsd;
  finished in ~4 min, all stills 1920x1200.

Vision raw observations (detail-pane crops of the committed stills): the
Appearance pane reads "Appearance" + Light/Dark/Auto (Light selected) and
"Accent color" with six swatches + "Custom…" right-aligned and not overlapping
the label after the fix; Wallpaper shows "Solid color" preview, Dragonfruit/
Landscape collections, Fit, and Add Photo…; Desktop & Dock lists the ten rows;
Displays shows the Built-in Display preview, Rotation, and five resolution
tiles with Default selected.

Gotchas for later tasks:

- **Absence tests must run under `dbus-run-session`** (the CMake target does);
  otherwise the host settingsd/portal make the absence assertions pass
  vacuously. Same shape as `tst_settings_live`.
- **No "settings unavailable" banner** is correct: panes use schema defaults.
  T-15.16's breadth matrix should document the same.
- **`make settings-wave-1-capture`** uses the `# df-allow-desktop-name` KWin
  scripting raise and the synthetic-input nudge, copied from
  `scripts/capture-settingsd.sh` / the T-05 harness; it is not part of
  `make e2e`.
- The per-pane stills show the window maximized with the scroll fold near the
  last row (same as T-52's note); a pane that scrolls is not a capture defect.

## T56 — T-10.1a files-core streaming listing and model

**State: done.** `files-core` now exists as a headless Rust library and a
directory streams incrementally into its model on a worker thread. T-10.1b
adds sorting and finalizes GIO-vs-fallback; T-10.2 adds operations.

What landed:

- **`services/files-core/`** (new crate `dragonfruit-files-core`, workspace
  member; no deps, `tempfile` dev-only):
  - `src/location.rs` — `Location` (URI-addressed; `file://` resolves,
    foreign schemes carried). Raw-byte-safe percent-encode/decode so spaces
    and invalid-UTF-8 names round-trip.
  - `src/node.rs` — `Node` (name `OsString`, `NodeId` stable per session,
    `NodeKind`, size, modified, symlink target) + lossy `display_name()`.
  - `src/source.rs` — `DirectorySource`/`DirectoryReader` seam + `SourceError`.
  - `src/fallback.rs` — `StdFsSource`, the sanctioned `std::fs` fallback,
    marked by `SANCTIONED_FALLBACK_MARKER` ("replace with GIO/GVfs (T-10.1b)");
    resolves only `file://`, else `UnsupportedScheme`.
  - `src/listing.rs` — worker thread + `ListingHandle` + `ListingEvent`
    (`DEFAULT_BATCH = 256`).
  - `src/model.rs` — `DirectoryModel` (`begin`/`apply`/`drain`), generation-
    keyed so stale listings are ignored; `check_consistent()` asserts the
    id→index invariant.
  - `src/mock.rs` — `MockSource` for headless tests.
- **Tests** — `cargo test -p dragonfruit-files-core`: 15 unit + 6 integration
  (`tests/streaming.rs`) + 1 doctest = 22 pass. The 4k-file temp tree streams
  a first batch `<` total and reaches `Complete` with all 4000 nodes and a
  consistent index; `MockSource` proves arrival order; non-UTF-8 name test.
- **Gate** — `Makefile` `e2e` now runs `cargo test -p dragonfruit-files-core`.
  `make lint` 31/31, `make e2e` exit 0.
- **Docs** — ADR
  [0042](design/adr/0042-files-core-streaming-listing-and-fallback.md);
  `docs/design/01-architecture.md` services tree; a T-10.1a status paragraph
  in `docs/design/09-files.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-files-core` — 22 pass.
- `make lint` green; `make e2e` exit 0.

Live capture (`/tmp/opencode/t56-capture.sh`; still `/tmp/opencode/t56-desktop.png`,
1920x1200): nested demo on the host session. Raw vision observation: menu bar
+ dock render, wallpaper is the solid default, no clipping/artifacts. Vision is
a supporting check; the crate has no surface of its own.

Gotchas for later tasks:

- **`files-core` is at `services/files-core`** (`dragonfruit-files-core`), a
  library — do not add a binary or a daemon.
- **The model is generation-keyed**: always use the `ListingHandle` returned
  by `begin`; `apply` silently ignores retired generations. `begin` cancels
  the previous worker and clears the model.
- **IDs are assigned by `DirectoryModel::apply`**, not the source; a source
  emits `Node` with `NodeId::UNSET`. Rename survival is T-10.3b's watcher job.
- **GIO is absent on this host** (`pkg-config --exists gio-2.0` fails);
  `StdFsSource` is the only backend. T-10.1b decides GIO vs. hardened
  fallback behind the unchanged `DirectorySource` seam.
- **No sort order is applied** — nodes are in arrival order; T-10.1b sorts.
- Adding a key/`Node` field: raw name bytes are load-bearing; keep `OsString`
  and percent-encoding, never a lossy string.

## T57 — T-10.1b files-core sorting and platform fallback

**State: done.** `files-core` now sorts its streamed model and the
GIO-vs-fallback decision is final: GIO headers are absent from the pinned
toolchain, so `StdFsSource` stays the shipping backend, explicitly marked for
replacement. T-10.2a adds operations on top of this model.

What landed:

- **`services/files-core/src/sort.rs`** (new module) — `SortKey`
  (Name/Kind/Size/Modified), `SortDirection` (Ascending/Descending), and
  `SortSpec` (key + direction + `folders_first`). `natural_cmp` is
  dependency-free natural/numeric (`file2` < `file10`), case-insensitive with
  a case-sensitive tie-break. `SortKey`/`SortDirection` have
  `as_str`/`parse` ("modified" also accepts "date"). Re-exported from
  `lib.rs`.
- **`services/files-core/src/model.rs`** — `DirectoryModel` keeps arrival
  order in `nodes()` and maintains a separate sorted projection:
  `ordered()`, `ordered_indices()`, `ordered_node(rank)`,
  `ordered_position_of(id)` (O(n) scan), `sort_spec()`, `set_sort(spec)`.
  Each `apply` batch is stable-sorted and merged into the projection in O(n)
  (`merge_sorted`); `set_sort` stable-sorts in place. Equal keys keep arrival
  order; node ids are never reassigned, so selection survives a re-sort.
  `check_consistent()` now also asserts the projection is a permutation and is
  stable-sorted. Default spec: name, ascending, folders first.
- **`services/files-core/src/fallback.rs`** — `SANCTIONED_FALLBACK_MARKER`
  reworded to name GIO/GVfs, the `DirectorySource` seam, and ADR 0043. Still
  resolves only `file://`, else `UnsupportedScheme`.
- **Tests** — `cargo test -p dragonfruit-files-core`: 39 pass (27 unit + 5
  `tests/sorting.rs` + 6 `tests/streaming.rs` + 1 doctest). New
  `tests/sorting.rs`: real temp tree natural order with folders first, a
  600-file stream whose projection is sorted after *every* batch, re-sort
  preserving ids, folders-first toggle + descending not reversing it, and the
  fallback's kinds/scheme refusal/marker.
- **Docs** — ADR
  [0043](design/adr/0043-files-core-fallback-is-the-shipping-backend.md);
  `docs/design/09-files.md` status paragraph and collation bullet updated;
  crate `Cargo.toml` description/comment.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-files-core` — 39 pass.
- `make lint` — 31/31 ctest plus all checks green.
- `make e2e` — exit 0.

Live capture (`/tmp/opencode/t57-capture.sh`; still
`/tmp/opencode/t57-desktop.png`, 1920x1200): nested demo on the host session.
Raw vision observation: menu bar, Dock, and the Settings window render with no
artifacts; the only reported oddity was the pre-existing X11 demo window's
title clipped at its right edge. files-core has no surface of its own, so this
only confirms the session still renders. Vision is a supporting check.

Gotchas for later tasks:

- **Sorting is model state, not view state.** Call `set_sort`; consume
  `ordered()`/`ordered_indices()`. Do not sort in the bridge/QML or a second
  collation will appear.
- **`nodes()` is arrival order; `ordered()` is the sort.** Ids are assigned
  in arrival order and never change on re-sort.
- **`folders_first` is applied before the key and is not reversed by
  descending.**
- **Size/modified `None` sorts last ascending** (directories carry no size).
- **GIO is still absent** (`pkg-config --exists gio-2.0` fails). Trash/Recents/
  volumes return `UnsupportedScheme`; T-10.3a must add a GIO reader or
  explicitly degrade. The replacement point is the `DirectorySource` seam plus
  the marker.
- **Locale-aware collation is not done** (see Follow-ups): the design's
  locale-aware numeric rule is currently natural/numeric only.

## T58 — T-10.2a files-core operations

**State: done.** `files-core` gained the one operations seam: rename, new
folder, move, copy, and permanent delete against a real temp tree, with typed
errors. Optimistic semantics, conflict policy, undo, progress, the journal,
and `trash://` are later tasks.

What landed:

- **`services/files-core/src/ops.rs`** (new module):
  - `FileOps` — the seam: `rename`, `create_dir`, `copy`, `move_to`,
    `delete`, `exists`, and a provided `new_folder`. `Send + Sync + 'static`;
    synchronous, to be driven off the UI thread.
  - `StdFsOps` — the sanctioned `std::fs` backend (same degradation as
    listing, ADR 0043); resolves only `file://`, else `UnsupportedScheme`.
    Copy is recursive and recreates symlinks with their raw target (never
    follows); cross-device move is copy-then-delete; delete is permanent and
    recursive.
  - `generated_name(base, exists)` — the one next-available-name helper
    (`untitled folder`, `untitled folder 2`, …) shared by new-folder later
    with duplicate/paste/compress; preserves base raw bytes. `NEW_FOLDER_BASE`.
  - `OperationError` — `UnsupportedScheme`, `NotFound`, `PermissionDenied`,
    `AlreadyExists`, `NotADirectory`, `DirectoryNotEmpty`, `InvalidName`,
    `Io`, with `from_io(error, context)`.
- **`services/files-core/src/location.rs`** — added `Location::parent()`
  (file paths and foreign-scheme URI prefixes). Fixed `Location::child` for an
  authority-only foreign root: `trash:///` used `trim_end_matches('/')`, which
  collapsed it to `trash:` and made `scheme()` panic; now strips at most one
  trailing slash.
- **Tests** — `services/files-core/tests/operations.rs` (new; 17 cases): the
  full operation matrix against temp dirs, including recursive dir copy,
  symlink recreation, non-UTF-8 name preservation, directory-into-itself
  refusal, conflicts, missing targets, broken-symlink `exists`, and all
  operations refusing `trash://`. Plus unit tests for `generated_name`, name
  validation, `from_io`, and `Location::parent`.
- **Docs** — ADR
  [0044](design/adr/0044-files-core-operations-seam.md); a T-10.2a
  implementation-status paragraph in `docs/design/09-files.md`; crate
  description in `Cargo.toml`.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-files-core` — 66 pass (36 unit + 17 operations +
  5 sorting + 6 streaming + 2 doctests).
- `make lint` — 31/31 ctest plus all checks green.
- `make e2e` — exit 0.

Live capture (`/tmp/opencode/t58-capture.sh`; still
`/tmp/opencode/t58-desktop.png`, 1920x1200): nested demo on the host session.
Raw vision observation: no clipping, blurring, or missing elements; the
Settings window, X11 demo window, and Dock/taskbar render. files-core has no
surface of its own, so this only confirms the session still renders. Vision is
a supporting check.

Gotchas for later tasks:

- **Use `make`, or `CARGO_NET_OFFLINE=true`.** A bare `cargo test` (and
  `cargo test -p dragonfruit-files-core`) blocked for minutes here with no
  output before the tests even started; the offline flag makes it instant
  once built. The `make` targets are unaffected.
- **`FileOps` is the one mutation path.** T-10.2b wraps it for optimistic
  rendering/reconciliation; never call `std::fs` from the bridge/QML and never
  bypass the seam.
- **Destination exists = `OperationError::AlreadyExists`.** Conflict policy
  (Keep Both / Stop / Replace / Merge) is not implemented; a primitive never
  guesses.
- **`delete` is permanent.** Trash is T-10.3a; `trash://` still returns
  `UnsupportedScheme`. `delete` on a directory is recursive
  (`remove_dir_all`).
- **Cross-device `move_to` is copy-then-delete, not journaled.** A SIGKILL in
  between leaves a duplicate (safe direction); temp-then-rename, partial-copy
  journaling, and orphan cleanup are still owed (design crash suite).
- **Generated names preserve raw bytes** via `OsString::push`; if you add
  duplicate/paste, pass the item's `Node::name()` to `generated_name` — do not
  lossy-decode.
- **`Location::child` of a foreign root** now keeps the scheme (`trash:///a`);
  if you touch scheme handling, keep `strip_suffix('/')`, not
  `trim_end_matches('/')`.

## T59 — T-10.2b Optimistic semantics and state preservation

**State: done.** `files-core` now applies rename / new-folder / delete to the
model optimistically (visible before any confirmation) and reconciles via
confirm/revert, preserving node ids, the sort spec, and selection. T-10.3a
(trash) reuses the optimistic delete path; T-10.3b's watcher will drive
reconciliation.

What landed:

- **`services/files-core/src/optimistic.rs`** (new) — `OptimisticModel`
  wraps a `DirectoryModel` + `Selection`:
  - `begin_rename`, `begin_new_folder`, `begin_delete` mutate the model
    synchronously and return an `OpId`; the change is painted on the next
    frame with no worker/await.
  - `confirm(op)` retires the pending record and keeps the painted result;
    `revert(op)` restores the captured node, arrival position, and selection
    membership/position.
  - `rename_via` / `new_folder_via` / `delete_via` run the real `FileOps`
    call and confirm on `Ok` / revert on `Err` in one synchronous call
    (off-UI-thread, like `FileOps`). A successful create/rename is retargeted
    to the `Location` the real op returned.
  - `begin` / `apply` / `drain` forward to the underlying model; `set_sort`
    and `sort_spec` pass through. `new_folder` generates the name from the
    model's own rows with the shared `generated_name`.
- **`services/files-core/src/selection.rs`** (new) — `Selection`, an
  insertion-ordered set of `NodeId` with O(1) membership: `select`/`deselect`/
  `toggle`/`insert_at`/`position_of`/`replace_all`/`retain`/`prune`/`clear`.
  `select` ignores `NodeId::UNSET`.
- **`services/files-core/src/model.rs`** — new mutation API used by the
  optimistic layer: `insert_node`, `restore_node`, `remove_node`,
  `replace_node`, `rename_node` (updates name+URI), all re-sorting the
  projection via a private `rebuild_order` (stable `sort_by`). Ids are never
  renumbered; `next_id` only grows.
- **`services/files-core/src/node.rs`** — crate-private `set_name`/`set_uri`.
- **Tests** — `cargo test -p dragonfruit-files-core`: 86 pass (44 unit + 17
  operations + **11 new `tests/optimistic.rs`** + 5 sorting + 6 streaming + 3
  doctests). New cases: optimistic rename visible then confirm and revert,
  new-folder folders-first + reverts + generated numbering, selection survives
  delete/revert/re-sort, and `*_via` against a real temp tree (rename confirm,
  conflict revert, delete confirm, missing-delete revert, two new folders,
  `trash://` unsupported revert).
- **Docs** — ADR
  [0045](design/adr/0045-files-core-optimistic-layer.md);
  `docs/design/09-files.md` T-10.2b status paragraph; crate `Cargo.toml`
  description.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-files-core` — 86 pass.
- `make lint` — 31/31 ctest plus all checks green.
- `make e2e` — exit 0.

Live capture (`/tmp/opencode/t59-capture.sh`; still
`/tmp/opencode/t59-desktop.png`, 1920x1200): nested demo on the host session.
Raw vision observation: Settings window (sidebar Appearance/Desktop & Dock/
Displays/Wallpaper, accent swatches), menu bar, and Dock render fully; the
only reported oddity is the pre-existing X11 demo window whose title/content
is clipped at its edge (same as T57/T58, unrelated to files-core). files-core
has no surface of its own, so this only confirms the session still renders.
Vision is a supporting check.

Gotchas for later tasks:

- **`OptimisticModel` is the one optimistic path.** Call `begin_*` and then
  `confirm`/`revert` (or the `*_via` helpers). Do not call `std::fs` or
  `DirectoryModel` mutations directly from the bridge/QML.
- **Node ids are never renumbered** by any optimistic edit, so selection is
  keyed by id and survives rename/new-folder/re-sort. Only a confirmed delete
  drops an id; revert re-selects it at its old position.
- **`next_id` only grows.** A removed id can be restored with `restore_node`;
  a fresh insert never reuses it.
- **Reverting several pending removals** should be done in reverse order for
  exact arrival positions; the list is clamped and safe otherwise.
- **`begin_new_folder` names from model rows only**, not the filesystem; the
  real op may pick a different free name, and `*_via` retargets the row to the
  returned `Location` before confirming.
- **Trash is still absent** (`trash://` → `UnsupportedScheme`); T-10.3a
  reuses `begin_delete`. `delete` (and the optimistic delete) is permanent.
- **The watcher is not wired.** Today `confirm`/`revert` are driven by the
  operation result; T-10.3b must guarantee each pending op is resolved exactly
  once.

## T60 — T-10.3a files-core trash

**State: done.** `files-core` now speaks the freedesktop Trash spec and the
trash seam exists. Because GIO/GVfs is not linked, `FreedesktopTrash` is the
sanctioned fallback, but it is **not** a behavioral degradation: it reads and
writes the same on-disk store GVfs owns (same layout, same `.trashinfo`
format), so trash stays one source of truth. T-10.3b (watcher) and T-10.6a
(Dock `trash://` source) build on the enumeration here.

What landed:

- **`services/files-core/src/trash.rs`** (new) — the trash engine:
  - `TrashOps` trait — the seam (sibling of `FileOps`): `trash`, `restore`,
    `empty`, `entries`, `home_trash`. `FileOps::delete` stays **permanent**.
  - `FreedesktopTrash` — home trash `$XDG_DATA_HOME/Trash` else
    `$HOME/.local/share/Trash`; `files/` + `info/*.trashinfo`; percent-encoded
    absolute `Path`; de-duplicated names via the shared `generated_name`;
    per-volume `.Trash/$UID` (sticky shared) or `.Trash-$UID` (`0700`) when the
    target is on another filesystem. `new()`/`with_home_trash(path)` (tests
    inject a temp root) and `marker()`.
  - `TrashedItem` — stored name, original path/location, `DeletionDate`,
    `file_path`, `info_path`. `parse_trash_info` and `format_deletion_date` are
    public so a spec golden round-trips without a store.
  - `SANCTIONED_TRASH_FALLBACK_MARKER` names GIO/GVfs and the `TrashOps` seam.
- **`services/files-core/src/optimistic.rs`** — `OptimisticModel::trash_via`
  reuses `begin_delete` exactly as ADR 0045 promised: row disappears within a
  frame, reverts on failure.
- **`services/files-core/src/ops.rs`** — `OperationError::MalformedTrashInfo`
  added; `path_of`/`lexists`/`copy_entry`/`remove_entry` made `pub(crate)`.
- **`services/files-core/src/location.rs`** — `encode_path` extracted (keeps
  `/`), used by both `file://` URIs and the trash `Path=`; `decode_path` now
  `pub(crate)`.
- **Tests** — 13 new in `tests/trash.rs`: file round-trip (content + info
  format + restore), recursive directory, duplicate de-dup, empty, empty on a
  missing store, foreign scheme, missing source, restore-occupied,
  restore-missing-parent, malformed info skipped, non-UTF-8 round-trip, and
  `trash_via` confirm/revert. Plus unit tests for info parse/format and the
  ISO 8601 formatter.
- **Docs** — ADR
  [0046](design/adr/0046-files-core-trash-seam-and-spec-fallback.md);
  `docs/design/09-files.md` T-10.3a implementation paragraph and Trash bullet;
  crate `Cargo.toml` description.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — 106 pass (50
  unit + 17 operations + 11 optimistic + 5 sorting + 6 streaming + 13 trash + 4
  doctests).
- `make lint` — 31/31 ctest plus all checks green.
- `make e2e` — exit 0.

Live capture (`/tmp/opencode/t60-capture.sh`; still
`/tmp/opencode/t60-desktop.png`, 1920x1200): nested demo on the host session.
Raw vision observation: menu bar, Settings window, Dock (with the Trash item)
render; the only artifact reported is the pre-existing X11 demo window's
clipped title (same as T57–T59, unrelated to files-core). files-core has no
surface of its own, so this only confirms the session still renders. Vision is
a supporting check.

Gotchas for later tasks:

- **`TrashOps` is the one trash path.** `trash`/`restore`/`empty`/`entries`;
  never call `std::fs` or `FreedesktopTrash` internals from the bridge/QML.
- **`trash()` takes a `file://` target** and returns a `TrashedItem` (the
  caller keeps it to offer Put Back). `restore()` takes that item back.
- **`entries()` enumerates the home trash only.** Per-volume trashes are
  written correctly but not aggregated; `trash://` listing is still
  `UnsupportedScheme` (T-10.6a).
- **`DeletionDate` is UTC**, not local (spec default is local). Display-only;
  restore never reads it. Documented in ADR 0046.
- **`MalformedTrashInfo`** is the new `OperationError` variant; a malformed
  `.trashinfo` is skipped by `entries`, never fatal.
- **MSRV trap:** `std::io::ErrorKind::CrossesDevices` is Rust 1.85, but MSRV
  is 1.80. Reference it **only in a `match` pattern** (clippy's
  `incompatible_msrv` flags `==`). ops.rs and trash.rs both do this.
- **Tests inject the trash root** with
  `FreedesktopTrash::with_home_trash(dir.join("Trash"))`; targets must be in
  the same tempdir or the per-volume branch engages.

## T61 — T-10.3b files-core folder watcher

**State: done.** `files-core` now has the one change monitor: one watch per
visible directory, event-driven (no polling), folding external changes into the
model incrementally — never by re-listing. Because GIO/GVfs is not linked,
`InotifyWatcher` is the sanctioned fallback (speaking Linux inotify, the local
mechanism `GFileMonitor` wraps) and is **marked for replacement**. The watcher
also reconciles pending optimistic edits: a matching event confirms, a
contradicting one reverts. T-10.4 renders the model; T-10.4b/c need no
re-listing path.

What landed:

- **`services/files-core/src/watch.rs`** (new) — the watch seam and fallback:
  - `FolderWatcher` / `WatchReader` — the seam (sibling of `DirectorySource`):
    `FolderWatcher::watch(&Location)` opens the folder on the caller's thread
    (foreign scheme / missing folder is an immediate `Err`), returning a reader
    whose `next_batch(timeout)` blocks for changes. A named
    `files-core-watcher` worker forwards `WatchEvent`s over an `mpsc` channel;
    `WatchHandle` has `try_recv`/`recv_timeout`/`recv`/`cancel`, and drop
    cancels. `WATCH_POLL_INTERVAL` (200 ms) is the cancellation poll, not a
    directory poll.
  - `WatchEventKind` — `Created(Node)`, `Modified(Node)`, `Removed { uri }`,
    `Renamed { from_uri, to }`; `WatchEvent` tags the generation. Events are
    incremental: creates/modifies carry the one freshly stat'd `Node`.
  - `InotifyWatcher` (Linux) — `inotify_init1` + `inotify_add_watch` over
    `libc`, non-recursive, watches create/delete/moved/modify/attrib and pairs
    `IN_MOVED_FROM`/`IN_MOVED_TO` by cookie into a `Renamed`; an unmatched
    move-from flushes as a `Removed`. It stats only the named entry (via the
    shared `fallback::node_for_path`), so no directory is re-listed.
  - `SANCTIONED_WATCHER_FALLBACK_MARKER` names GIO/GVfs and the seam.
- **`services/files-core/src/fallback.rs`** — extracted
  `pub(crate) node_for_path(parent, name, path)` (symlink-aware, no follow) and
  reused it for both the listing and the watcher.
- **`services/files-core/src/model.rs`** — `DirectoryModel::begin_watch`
  (shares the listing generation; cancels any previous watch; `begin` also
  cancels the active watch), `apply_watch`, `drain_watch`, and
  `node_id_for_uri`. `apply_watch` folds one event: creates/modifies dedupe by
  URI and refresh, removals drop the row, a rename keeps the existing node id
  (selection survives). Stale generations are ignored.
- **`services/files-core/src/optimistic.rs`** — `OptimisticModel::begin_watch`,
  `apply_watch`, `drain_watch`. `apply_watch` decides each pending op against
  the event (`pending_outcome`): matching target confirms, a vanished create
  or a returning removed item reverts, then the event folds. `confirm`/`revert`
  are idempotent-safe, so a resolved `*_via` op is never resolved twice.
- **`services/files-core/src/mock.rs`** — `MockWatcher` fixture (scripted event
  batches then blocks) and `MockSource::opens()` so a test can prove no
  re-listing. (Adding `opens` is additive; existing tests unaffected.)
- **Tests** — `services/files-core/tests/watcher.rs` (12 cases): scripted
  create+delete folds with no re-list; scripted rename keeps the node id;
  modify refreshes instead of duplicating; stale generation ignored; optimistic
  create confirm/revert, optimistic delete confirm/revert; failing watcher open
  reports `UnsupportedScheme`; and real-inotify cases (external create+delete
  update the model, external rename keeps the id, `new_folder_via` stays
  confirmed with a live watch). Plus unit tests in `watch.rs` for the marker,
  mask classification, foreign scheme, and missing folder.
- **Docs** — ADR
  [0047](design/adr/0047-files-core-folder-watcher-seam.md);
  `docs/design/09-files.md` T-10.3b implementation paragraph + change-monitor
  bullet; crate `Cargo.toml` description and a Linux-only `libc` dependency.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — 122 pass (54
  unit + 17 operations + 11 optimistic + 5 sorting + 6 streaming + 13 trash +
  **12 watcher** + 4 doctests).
- `CARGO_NET_OFFLINE=true cargo clippy -p dragonfruit-files-core
  --all-targets -- -D warnings` — clean.
- `make lint` — 31/31 ctest plus all checks green.
- `make e2e` — exit 0.

Live capture (`/tmp/opencode/t61-capture.sh`; `/tmp/opencode/t61-desktop.png`,
1920x1200): nested demo on the host session. Raw vision observation: menu bar,
Settings window (sidebar + Appearance controls + accent swatches), X11 demo
window, and the Dock (K/F/X/D app icons, Downloads folder, Trash) all render;
artifacts reported are the pre-existing X11 demo window clipped at the right
edge and the Dock "Downlo…" label truncation (same as T57–T60, unrelated to
files-core). files-core has no surface of its own, so this only confirms the
session still renders. Vision is a supporting check.

Gotchas for later tasks:

- **`FolderWatcher` is the one watch path.** Call `begin_watch` after
  `begin` (it shares the listing generation), then drain with
  `DirectoryModel::drain_watch` / `OptimisticModel::drain_watch` in the event
  loop. Never poll `read_dir` or `std::fs` from the bridge/QML.
- **A watch belongs to the current generation.** `begin` cancels the active
  watch and stale `WatchEvent`s are ignored; drop `WatchHandle` to cancel too.
  The worker notices cancellation at the next 200 ms poll.
- **Events are incremental.** Do not expect a `Renamed` when inotify splits a
  move across reads: you may get `Removed` then `Created`, which is correct but
  gives a new node id. Within one read the rename keeps the id.
- **`apply_watch` dedupes by URI** (`node_id_for_uri`, a linear scan). A
  watcher event for an unlisted entry inserts it. Watch events are rare, so no
  second index is maintained.
- **Optimistic + watcher:** `OptimisticModel::apply_watch` confirms a matching
  pending edit or reverts a contradicting one, then folds. Reverting a pending
  remove restores the row, and the same event then refreshes it — order matters
  and is handled internally; do not call `confirm`/`revert` around a drain.
- **`libc` is Linux-only here** (`[target.'cfg(target_os = "linux")']`);
  `InotifyWatcher` is exported only on Linux. The seam and `MockWatcher`
  compile anywhere.
- **A self delete/move of the watched folder is silent** (see T-10.3b
  follow-ups). Re-watch on navigation; an error state is a later task.

## T62 — T-10.4a Files window, toolbar, and sidebar

**State: done.** The Files window, toolbar, and sidebar are real. `apps/files`
is now a reusable QML module `Dragonfruit.Files` (a thin executable only loads
the window), mirroring Settings. The browsing model is QML (`FilesBrowser`);
the platform locations are a Qt singleton (`Files` / `FilesBridge`); the
directory listing and the views are explicitly T-10.4b's, attached to
`FilesBrowser.currentUri`. See ADR
[0048](design/adr/0048-files-app-shell-location-provider.md).

What landed:

- **`apps/files/FilesBridge.{h,cpp}`** — `QML_SINGLETON`, `QML_NAMED_ELEMENT(Files)`:
  `favorites` (real XDG dirs via `QStandardPaths`; Desktop/Documents/Downloads/
  Pictures/Music/Videos, only directories that exist; Home is a Location, and
  Recents is omitted pending `recent://`/GVfs), `volumes`
  (`QStorageInfo::mountedVolumes()`, root/system mounts
  excluded, `/dev/` devices only), `homeUri`/`computerUri`/`trashUri`,
  `userName`, `startUri`, `breadcrumb(uri)`, `displayName(uri)`,
  `isBrowsable(uri)`. `DF_FILES_FIXTURE=1` selects fixed paths
  (`/home/tester/...`, one `Data` volume); `DF_FILES_START_URI` opens a
  specific location.
- **`apps/files/FilesBrowser.qml`** — current URI, per-window history
  (`navigate` truncates the redo tail; `reset` seeds without a step), and
  per-URI view state (`viewFor`/`setView`, default `"icon"`).
- **`apps/files/FilesShell.qml`** — `AppWindow` + `TitleBar` (centered location
  title) + `Toolbar` (back/forward, icon/list `SegmentedControl` kept in step
  via a `Binding`, right-aligned `SearchField`) + `Sidebar` (Favorites = the
  user folders; Locations = Home, Computer, volumes, `Trash` last) + footer
  `PathBar`. Design-system components only.
  Keyboard: sidebar Up/Down/Home/End/Return, Alt+Left/Right and
  Cmd+[ / Cmd+] history, Ctrl+F search.
- **`apps/files/PathBar.qml`** — breadcrumb of ghost `Button`s + chevron
  `Icon`s; segments are history anchors.
- **`apps/files/FilesWindow.qml`** — frameless first-party window (traffic
  lights forward close/minimize/zoom/move, like SettingsWindow).
- **`design-system/components/Icon.qml`** — new canvas glyphs `home`,
  `documents`, `downloads`, `music`, `movies`, `trash`, `computer`, `volume`,
  `folder`, `icon-view`, `list-view` (Pictures reuses `wallpaper`, Desktop
  reuses `displays`).
- **`design-system/components/SegmentedControl.qml`** — an entry may carry
  `icon`; the segment draws the glyph (Accessible.name stays the label).
  Existing string/object usages are unchanged.
- **`design-system/components/Sidebar.qml` + `SourceList.qml`** — selection is
  an instant second fill layer; only hover animates. A freshly opened window
  previously painted a half-faded selection when the client was idle and no
  frames were delivered (seen in the live capture).
- **Tests** — `apps/files/tests/` (new; `tst_files_shell.cpp` + 14 QML cases).
- **Docs** — ADR 0048; `docs/design/09-files.md` T-10.4a status paragraph.

Commands that work (repo root; `make` sets the toolchain env):

- `make qml-test` — 32/32 ctest green; `tst_files_shell` 14 pass.
- `make visual-test` — 72 gallery snapshots pass.
- `make lint` — fmt/clippy/qmllint/tokens/desktop-name gates green.
- `make e2e` — exit 0.

Live capture (`/tmp/opencode/t62-capture.sh`; `/tmp/opencode/t62-files-home.png`
and `t62-files-documents.png`, 1920x1200) with
`DF_DEMO_QT_APP=build/apps/files/dragonfruit-files`. Raw vision observation on a
tight window crop: toolbar chevrons + two-segment view switch + `Search` field;
sidebar `FAVORITES` (Desktop/Documents/Downloads/Pictures/Music/Videos) and
`LOCATIONS` (user/Computer/Trash, Trash last); footer `Computer > user`;
selected `user` row highlighted with white glyph/text. Raw pixel observation: sidebar
background `(248,246,250)`, selected row `(179,42,102)` exact accent (1363 px).
Vision is a supporting check.

Gotchas for later tasks:

- **`Files` is a location provider, not the filesystem backend.** It never
  lists or mutates; do not call `std::fs` from the app. The listing is the
  T-10.4b bridge into `files-core`.
- **`FilesBrowser` owns navigation/history/view state only.** T-10.4b attaches
  the listing to `currentUri` and must not reimplement history or view
  persistence. `viewState` is keyed by URI, matching `09-files.md#views`.
- **Fixture envs are read once at singleton construction.** `DF_FILES_FIXTURE`
  and `DF_FILES_START_URI` must be set before the process starts (the CMake
  test ENVIRONMENT does).
- **Sidebar entries drop custom item fields.** `Sidebar.entries` keeps only
  label/icon/badge and adds `section`/`itemIndex`; `FilesShell.uriForEntry()`
  maps back through `sidebarSections`. Keep that mapping when adding rows.
- **The content area is a placeholder.** `FilesShell.contentArea` holds
  `emptyState`; replace it with the list/icon views. The trailing view-options
  dropdown, sort controls, and search results are deferred to T-10.4b.
- **Selection must not depend on a hover animation.** The Sidebar/SourceList
  two-layer fill is deliberate; do not collapse it back to one animated
  rectangle.
- **`QStorageInfo` volumes are block devices only**; network/GVfs volumes and
  eject wait for the volume monitor (track item 5 / T-10.6).

## T63 — T-10.4b Files list and icon views

**State: done.** The Files icon and list views render the `files-core` listing.
The bridge ADR 0048 deferred is a **hand-written C ABI** in `files-core`
(crate-type `lib` + `staticlib`) behind a thin `QAbstractListModel` facade,
`FilesDirectoryModel`; there is no cxx-qt (offline toolchain, no dependency).
Selection is the stable node id and lives in `FilesShell`, so switching views
cannot lose it. See ADR
[0049](design/adr/0049-files-core-c-abi-bridge.md).

What landed:

- **`services/files-core/src/ffi.rs`** (new) — the C ABI: `df_files_begin`,
  `df_files_poll` (blocks for one event and returns the current ordered
  snapshot), `df_files_snapshot`, `df_files_set_sort`, `df_files_event_free`,
  `df_files_free`. `Cargo.toml`: `crate-type = ["lib", "staticlib"]`;
  `lib.rs`: `pub mod ffi`. Status/kind constants and `#[repr(C)]` structs are
  mirrored in `apps/files/ffi/files_core.h`.
- **`apps/files/FilesDirectoryModel.{h,cpp}`** (new) — `QML_ELEMENT`
  `QAbstractListModel`: `location`, `count`, `state`
  (`idle`/`streaming`/`complete`/`error`), `errorMessage`, `sortKey`,
  `sortAscending`, `foldersFirst`, `Q_INVOKABLE sortBy`. A 16 ms `QTimer`
  polls `df_files_poll(session, 0)` on the Qt thread and resets from the
  snapshot; the timer stops on complete/error, so idle = no polling. Roles:
  `nodeId`, `name`, `uri`, `isDir`, `kind`, `kindText`, `icon`, `size`,
  `hasSize`, `sizeText`, `modified`, `hasModified`, `modifiedText`,
  `symlinkTarget`. Size/date formatting is in C++ (`QLocale`), the icon is
  chosen from kind + suffix.
- **`apps/files/FilesIconView.qml`** (new) — `GridView` of tiles: glyph over a
  two-line label, accent-tinted selection, hover; emits `selected(nodeId)` /
  `activated(uri, isDir)`.
- **`apps/files/FilesListView.qml`** (new) — `ListView` with a fixed header and
  sortable `Name` / `Date Modified` / `Size` / `Kind` cells; header click calls
  `directory.sortBy`; full-row selection.
- **`apps/files/FilesShell.qml`** — `contentArea` holds `FilesDirectoryModel`
  and both views (only the active one visible); `selectedId` + `selectNode` /
  `activateNode`; double-click opens a folder. The `emptyState` now also shows
  the listing error.
- **`apps/files/FilesBridge.{h,cpp}`** — `viewFixtureUri`
  (`DF_FILES_VIEW_FIXTURE`) and `startView` (`DF_FILES_START_VIEW`).
- **`design-system/components/Icon.qml`** — a generic `file` glyph.
- **`apps/files/CMakeLists.txt`** — `cargo build -p dragonfruit-files-core`
  custom target, imported staticlib with `pthread;dl;m`, linked into the QML
  module. A standalone `make cmake-build` now also builds the Rust staticlib.
- **Tests** — `apps/files/tests/tst_files_shell.qml` (4 new cases) and a
  custom `main` in `tst_files_shell.cpp` that materializes a fixture directory
  and exports `DF_FILES_VIEW_FIXTURE`.
- **Docs** — ADR 0049; `09-files.md` T-10.4b status paragraph.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — 125 pass (57
  unit incl. 3 ffi + 17 operations + 11 optimistic + 5 sorting + 6 streaming +
  13 trash + 12 watcher + 4 doctests).
- `make qml-test` — 32/32 ctest green; `tst_files_shell` 18 pass.
- `make lint` — green (fmt/clippy/qmllint/tokens/desktop-name).
- `make e2e` — exit 0.

Live capture: `/tmp/opencode/t63-files-icon.png` and
`/tmp/opencode/t63-files-list.png` (1920x1200), nested demo with
`DF_DEMO_QT_APP=build/apps/files/dragonfruit-files`,
`DF_FILES_START_URI=file:///tmp/opencode/t63-files-fixture`, and
`DF_FILES_START_VIEW=icon|list` (scripts `/tmp/opencode/t63-still.sh`,
`t63-still.py`). Raw vision observation on tight crops: grid of glyph tiles
with labels (`Archive`, `Photos`, `Projects`, `budget.xlsx`, `Notes.txt`,
`photo.png`, `Report.txt`); list with `Name`/`Date Modified`/`Size`/`Kind`
headers, a sort indicator on `Name`, and sizes (`8.00 KiB`, `6 bytes`) and
dates (`9/25/26`). Vision is a supporting check.

Gotchas for later tasks:

- **`FilesDirectoryModel` is the one listing seam.** QML never reads the
  filesystem; extend the facade (and the C ABI) for operations, never add
  `QFile`/`std::fs` in the app.
- **Whole-snapshot per poll.** `df_files_poll` returns the model's full ordered
  list, and the facade resets on every batch. T-10.5 owns replacing it with
  incremental/windowed delivery; the QML roles are the stable contract.
- **One collation.** Sorting is `files-core`'s `SortSpec` via
  `df_files_set_sort`; do not re-sort in C++/QML.
- **Node ids are per-listing** (they restart at 1 on each `begin`).
  `FilesShell` clears `selectedId` on navigation for that reason.
- **The timer stops on complete/error**; an idle window does not poll. A
  `TIMEOUT` event keeps it running.
- **The folder watcher is not wired.** External changes do not repaint until a
  later task drains `WatchHandle`.
- **`DF_FILES_VIEW_FIXTURE` / `DF_FILES_START_VIEW` / `DF_FILES_START_URI` are
  read at singleton construction** — set before launch.
- **The CMake Rust target always runs cargo.** It is a no-op once built;
  `target/debug/libdragonfruit_files_core.a` is the linked artifact.
- **Nested synthetic pointer clicks reach the compositor (titlebar, shell
  layer) but not a Qt client surface.** Use `DF_FILES_START_VIEW` for captures
  instead of clicking the toolbar.

## T64 — T-10.4c Files context menus, multi-select, optimistic UI

**State: done.** Context menus, multi-select, and optimistic
rename/trash/new-folder are real. The `files-core` C ABI now carries the
optimistic operations: the session owns an `OptimisticModel` plus one
`files-core-ops` worker; `df_files_begin_*` paints synchronously and the
worker's outcome is confirmed or reverted on the next poll. Selection is the
shell's set of node ids, and the one `ContextMenu` follows the Finder rule.
See ADR [0050](design/adr/0050-files-core-ffi-optimistic-ops.md).

What landed:

- **`services/files-core/src/ffi.rs`** — `FfiSession` now wraps
  `OptimisticModel` (still a `DirectoryModel` underneath) and owns the op
  worker. New ABI: `df_files_begin_rename(session, node_id, new_name)` →
  op id, `df_files_begin_new_folder(session, parent_uri)`,
  `df_files_begin_trash(session, node_id)`, `df_files_pending_ops(session)`,
  `df_files_take_error(session)` + `df_files_string_free`. `df_files_poll`
  drains op outcomes first (confirm via `retarget`+`confirm`, or `revert` +
  record the message) and reports a drained outcome as a `BATCH` snapshot;
  then it polls the listing. The session is no longer freed on completion.
  `optimistic.rs` — `retarget` is now `pub(crate)`.
- **`apps/files/ffi/files_core.h`** — the new functions, mirrored.
- **`apps/files/FilesDirectoryModel.{h,cpp}`** — `Q_INVOKABLE rename(id,
  name)`, `newFolder(parentUri)`, `trash(id)`; `pendingOps`, `lastError`
  properties; `rowForNodeId`, `nodeIdAt`, `uriAt`, `isDirAt`, `allNodeIds`.
  Each begin calls `repaint()` (synchronously resets from the snapshot) then
  `ensurePolling()`. `poll()` keeps the timer while `pendingOps > 0` or the
  listing is streaming, and **stops the timer without freeing the session**
  once settled (this also fixes sort-after-load, which silently no-op'd
  before because `stop()` freed the session on `DONE`). `stop()` (destroy /
  navigation) frees the session and its worker.
- **`apps/files/FilesShell.qml`** — `selectedIds` (multi), `anchorId`,
  `renamingId`, `notice`; `selectNode(id, modifiers)` (Cmd toggles, Shift
  ranges via `rangeTo`), `selectAll`, `clearSelection`, `beginRename`/
  `commitRename`/`cancelRename`, `trashSelection`, `makeFolder`,
  `nodeMenu`/`backgroundMenu`/`contextAction`, `openNodeMenu`/
  `openBackgroundMenu`; keyboard Cmd+A, Shift+Cmd+N, Return (rename), Delete
  (trash), Escape. One `ContextMenu` instance; a `Connections` shows
  `directory.lastError` as a transient banner. Capture seams `startMenu` /
  `startRename` / `startSelect` (from env) applied once the listing settles.
- **`apps/files/FilesIconView.qml` / `FilesListView.qml`** — `selectedIds`
  prop, context-menu signal (coordinates fixed: emitted in *view* coords via
  `mapFromItem`), right-click, and an inline rename `TextInput`.
- **`apps/files/FilesBridge.{h,cpp}`** — `mutationFixtureUri`
  (`DF_FILES_MUTATION_FIXTURE`), `startMenu` (`DF_FILES_START_MENU` =
  `item`|`background`), `startRename` (`DF_FILES_START_RENAME`), `startSelect`
  (`DF_FILES_START_SELECT`).
- **Tests** — Rust `ffi::tests`: rename commits, rename reverts on conflict
  (with `lastError` taken once), new-folder commits and creates on disk, plus
  the existing cases. QML: multi-select modifiers/range/Select All, item and
  background menu models, real right-click opens the menu, capture seams,
  optimistic rename immediate + revert, new-folder immediate, trash immediate.
- **`apps/files/tests/tst_files_shell.cpp`** — materializes a second
  `DF_FILES_MUTATION_FIXTURE` tree and points `XDG_DATA_HOME` at a temp dir so
  the trash tests never touch the real user trash.
- **Docs** — ADR 0050; `09-files.md` T-10.4c status paragraph.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — 128 pass
  (60 unit incl. 6 ffi + 17 operations + 11 optimistic + 5 sorting + 6
  streaming + 13 trash + 12 watcher + 4 doctests).
- `make qml-test` — 32/32 ctest green; `tst_files_shell` 26 pass.
- `make lint` — green (fmt/clippy/qmllint/tokens/desktop-name/capture-grab).
- `make e2e` — exit 0.

Live capture (`/tmp/opencode/t64-capture.sh <mode>`; modes
`multiselect|context|background|rename`; driver `/tmp/opencode/t64-still.py`,
fixture `/tmp/opencode/t64-fixture`, PNGs `/tmp/opencode/t64-files-*.png`).
Raw vision observations on tight window crops: three rows highlighted
(`Archive`, `Projects`, `budget.xlsx`); item menu reading `Open` /
`Open in New Window` / `Rename Return` / `Move to Trash Delete`; background
menu reading `New Folder Shift+Cmd+N` / `Select All Cmd+A`; inline editor with
`Archive` on the first row. Vision is a supporting check.

Gotchas for later tasks:

- **The files-core session outlives the completed listing.** `stop()` frees
  it (navigation/destroy); `poll()` only stops the timer. Do not reintroduce a
  free-on-`DONE` path or sort/operations on a loaded folder break.
- **One op worker per open location.** Navigation drops the session → drops
  `op_tx` → the worker exits. No process, no daemon.
- **Optimistic begin + later poll is the contract.** `rename`/`trash`/
  `newFolder` repaint the model before returning; `pendingOps` tells the
  facade to keep polling; `lastError` carries a snapped-back message (taken
  once). Keep the timer alive while `pendingOps > 0`.
- **The op worker uses the real `FreedesktopTrash`**, whose store comes from
  `XDG_DATA_HOME`/`HOME` at worker spawn. Tests set `XDG_DATA_HOME` to a temp
  dir; do the same for any test that trashes.
- **Coordinate mapping:** a delegate's `mouse.x/y` is delegate-relative; map
  with `view.mapFromItem(delegate, …)` before emitting, and the shell maps the
  view point into the shell with `view.mapToItem(root, …)`.
- **`ContextMenu` opens with an opacity animation.** In the nested demo an
  idle client can leave it invisible; the capture driver sends a few
  `motion-abs` events first to wake the frame loop. The headless test uses
  `tryCompare(menu, "visible", true)` for the same reason.
- **Synthetic pointer input still cannot hold a modifier or right-click a Qt
  surface reliably.** The capture seams (`DF_FILES_START_MENU` /
  `_START_RENAME` / `_START_SELECT`) exist for the live check; the real
  gestures are covered headlessly.
- **Still deferred (T-10.4c+):** rubber-band selection, file opening /
  Open With, Get Info, Copy/Duplicate/Compress/Make Alias, the folder watcher,
  and the search result set.

## T65 — T-10.5 Files performance budgets

**State: done.** Files meets both budgets with incremental/windowed delivery.
The C ABI no longer re-sends the whole listing per batch: `df_files_poll_delta`
returns only the newly arrived nodes (at their final ranks), and
`df_files_snapshot_delta` returns a full reset for the first paint / sort /
optimistic edits. The Qt facade merges insertions into its display order in one
O(n+k) pass and emits `beginInsertRows` + `dataChanged` instead of resetting
the view. See ADR [0051](design/adr/0051-files-core-incremental-delta-abi.md).

What landed:

- **`services/files-core/src/ffi.rs`** — `df_files_row` / `df_files_delta`,
  `df_files_poll_delta`, `df_files_snapshot_delta`, `df_files_delta_free`,
  `df_files_begin_synthetic`. `FfiSession` keeps a `reported` `HashSet<NodeId>`
  so a streamed node is serialized exactly once; a vanished reported id falls
  back to a reset. The old `df_files_poll`/`df_files_snapshot` snapshot
  functions remain for the Rust unit tests (they do not touch `reported`).
- **`services/files-core/src/mock.rs`** — `SyntheticSource`: a disk-free
  `DirectorySource` fabricating `file-0000000.txt…` with sizes/mtimes, batches
  driven by `next_batch(max)`. Exported from `lib.rs`.
- **`services/files-core/src/listing.rs`** — the worker doubles its batch each
  event up to `MAX_BATCH = 8192`, keeping the first batch at `DEFAULT_BATCH`
  (first frame unchanged) while cutting a 100k merge from ~390 to ~20 passes.
  `a_large_directory_streams_incrementally` still sees a first batch ≤ 64.
- **`apps/files/ffi/files_core.h`** — the delta structs/functions mirrored.
- **`apps/files/FilesDirectoryModel.{h,cpp}`** — `m_nodes` is now
  `std::vector<Node>`; `applyDelta`/`applyReset`/`applyInsertions`/`nodeFromFfi`.
  `poll`/`repaint`/`applySort` use the delta functions. `start()` calls
  `df_files_begin_synthetic` for a location ending in `/synthetic` when
  `DF_FILES_SYNTHETIC_COUNT > 0`; `DF_FILES_SYNTHETIC_BATCH` sets the batch.
- **`apps/files/FilesIconView.qml` / `FilesListView.qml`** — `gridView` /
  `listView` aliases so the windowed-delegate check can read the viewport.
- **Tests** — `services/files-core/tests/performance.rs` (4 tests: warm 1k
  first frame, real-fallback 1k, 100k delivered-once, 100k viewport sweep);
  `ffi::tests::incremental_poll_sends_each_streamed_node_once` and
  `a_sort_change_delivers_a_reset_delta`; QML
  `test_large_list_paints_fast_and_stays_windowed` and
  `test_large_list_scroll_sweep` (runner sets `DF_FILES_SYNTHETIC_COUNT=100000`,
  batch 512).
- **Docs** — ADR 0051; `09-files.md` T-10.5 status paragraph; raw numbers in
  `docs/captures/t10-files-perf.txt`.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — 134 pass
  (62 unit incl. 8 ffi + 17 operations + 11 optimistic + 5 sorting + 6
  streaming + 13 trash + 12 watcher + 4 doctests + 4 performance).
- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core --test
  performance -- --nocapture --test-threads=1` — prints the budgets.
- `make qml-test` — 32/32 ctest green; `tst_files_shell` 28 pass.
- `make lint` green; `make e2e` exit 0.

Raw numbers (debug tree; full text in `docs/captures/t10-files-perf.txt`):

- warm 1k first frame 0.57 ms (budget < 50 ms); real 1k 1.80 ms.
- 100k stream: 100000 rows delivered exactly once in ~1.06 s.
- 100k scroll: 40-row viewport read 2.2 µs (budget < 16.6 ms / frame).
- Qt (offscreen/software): 100k first frame 21 ms; icon delegates 70 (48/49
  after scrolling to end/home); list delegates 24; 100 list jumps 999 ms.

Live capture (`/tmp/opencode/t65-capture.sh top|scroll`;
`/tmp/opencode/t65-still.py`; PNGs `/tmp/opencode/t65-files-100k-*.png`).
Launch env: `DF_DEMO_QT_APP=build/apps/files/dragonfruit-files`,
`DF_FILES_START_URI=file:///synthetic`, `DF_FILES_START_VIEW=list`,
`DF_FILES_SYNTHETIC_COUNT=100000`, `DF_FILES_SYNTHETIC_BATCH=512`. Raw vision
observation on the window: title `synthetic`; sidebar FAVORITES/LOCATIONS;
list headers `Name` (sort arrow) / `Date Modified` / `Size` / `Kind`; rows
`file-0000000.txt` … `file-0000016.txt`; breadcrumb `Computer > synthetic`.
Vision is a supporting check.

Gotchas for later tasks:

- **The incremental delta contract is "existing rows never reorder".** If a
  delta is not strictly-ascending ranks, or `reported.len() > model.len()`, the
  Rust side or the facade resets. Optimistic begin/confirm/revert and any
  watcher removal must go through `df_files_snapshot_delta` (reset), not the
  insertion path.
- **Old-outside, new-inside:** the Rust `reported` set is only updated by the
  delta functions. Do not mix `df_files_poll` and `df_files_poll_delta` on one
  session in production code (tests may use the snapshot functions on their own
  sessions).
- **`df_files_poll_delta` folds ops first.** A pending operation outcome always
  yields a reset delta; keep draining until `df_files_pending_ops == 0`.
- **The Qt model declares insertions appended** (`beginInsertRows(oldCount,
  newCount-1)`) then `dataChanged(0, oldCount-1)`. It is correct because
  delegates are index-based and re-query `data`; do not switch to a per-rank
  insert without re-checking the layoutChanged/scroll behavior.
- **The synthetic source is test-only**, gated to `/synthetic`; production
  listings still go through `StdFsSource` (GIO headers absent, ADR 0043).
- **Batches grow: first 256, then double to 8192.** A test that asserts a
  specific non-first batch size will break; assert bounds instead.
- **Synthetic mtimes start at the Unix epoch**, so the list shows `12/31/69`.
  Cosmetic only.
- **Still deferred (T-10.6a+):** the Dock Trash source, wiring the folder
  watcher to the facade (via the delta path), and network-mounted performance.

## T66 — T-10.6a Dock trash source

**State: done.** The Dock's Trash state comes from `files-core` over the one
freedesktop store, and the shell's interim home-trash watcher is deleted. The
shell links the `files-core` staticlib (the same artifact Files links) and reads
it through a small trash-only C ABI. See ADR
[0052](design/adr/0052-dock-trash-state-from-files-core.md).

What landed:

- **`services/files-core/src/trash_source.rs`** (new) — `TrashSource`
  (`DirectorySource` over `trash://`; items are `Node`s at `trash:///<name>`),
  `TrashMonitor` (count + `available`; change events on the T-10.3b
  `FolderWatcher` over the store's `info/`), and `TrashState`. Exported from
  `lib.rs`.
- **`services/files-core/src/ffi.rs`** — `df_files_trash_state` and
  `df_files_trash_monitor_{new,free,state,wait,refresh,trash,empty,take_error}`.
  `df_files_begin` now dispatches `trash://` to `TrashSource` (every other
  scheme stays `StdFsSource`). New Rust test
  `trash_monitor_abi_reads_trashes_and_empties`.
- **`shell/src/files_core_trash.h`** (new) — the mirrored trash ABI (kept
  separate from `apps/files/ffi/files_core.h` so the shell does not depend on
  the listing/model ABI).
- **`shell/src/trashbridge.{h,cpp}`** (new) — `TrashBridge` (same public shape
  as the old `TrashMonitor`: `start/stop/refresh/isFull/itemCount/
  isAvailable/lastError/trash/empty/changed`). A worker thread blocks on
  `df_files_trash_monitor_wait` and posts `changed()` to the UI thread.
- **Deleted** `shell/src/trashmonitor.{h,cpp}`; shellcontroller now owns a
  `TrashBridge`. Drop/Empty still work because they went through the same
  method names and are now routed through files-core.
- **CMake** — the `dragonfruit-files-core` imported staticlib + its cargo
  custom target moved to the top-level `CMakeLists.txt`, linked by
  `apps/files` and `dragonfruit-shell-dockcore`.
- **Tests** — `tst_dockcore`: four `trashBridge*` tests (they `setenv`
  `XDG_DATA_HOME` to a temp dir before constructing the bridge): read
  empty/full, third-party watch wake, trash+empty through files-core, and
  creating a missing store on demand.
- **Docs** — ADR 0052; `09-files.md` T-10.3a note + T-10.6a status paragraph.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — 142 pass
  (70 unit incl. 7 trash_source + 9 ffi, plus integration suites).
- `make qml-test` — 32/32 ctest green; `tst_dockcore` 46 pass.
- `make lint` green; `make e2e` exit 0.

Live capture: `/tmp/opencode/t66-capture.sh` (driver `t66-still.py`; PNGs
`/tmp/opencode/t66-dock.png`, `t66-dock-strip.png`, `t66-dock-tight.png`).
Launch: `DF_DEMO_QT_APP=build/apps/files/dragonfruit-files`,
`DF_FILES_START_URI=file:///tmp`, default socket. Raw vision on the tight Dock
crop: tiles `K` `F` `X` `D`, a blue Downloads folder labeled `Downloa…`, and a
red-stroked trash can with no count badge (the `trashFull` accent; the real
home trash held 24 items). Vision is a supporting check.

Gotchas for later tasks:

- **`TrashBridge` is the shell's only Trash seam.** It wraps
  `df_files_trash_monitor_*`; QML never sees the ABI. The method names match the
  deleted `TrashMonitor`, so `shellcontroller.cpp` call sites are unchanged.
- **The shell now links the `files-core` staticlib.** `make cmake-build` builds
  it via the top-level `dragonfruit-files-core-ffi` target; the artifact is
  `target/debug/libdragonfruit_files_core.a`. Both Files and the shell share
  the one imported target.
- **`TrashMonitor` reports a missing store as available** and creates
  `files/`+`info/`; only a store whose dirs cannot be created reads
  unavailable. The old "unreadable existing root" test was dropped (the new
  source follows the freedesktop "create on demand" contract).
- **The monitor watches only the home trash.** A delete on another volume
  lands in that volume's `.Trash-$UID` and does not move the badge.
- **`TrashSource` is now reachable by `df_files_begin("trash://…")`** so Files
  can list the Trash; Put Back / Delete Immediately over those nodes is
  T-10.6b/c.
- **Still deferred:** T-10.6b (Drop/Empty UX + `trash://` navigation),
  T-10.6c (Show in Files / Downloads / `.desktop` identity), wiring the folder
  watcher to the Files facade via the delta path, and network-mounted
  performance.

## T67 — T-10.6b Drop-to-trash, Empty Trash, trash://

**State: done.** The Dock's drop and Empty Trash already routed through
`files-core` since T-10.6a; this task added Files' own Empty Trash and the
`trash://` open path. Files opens the location the Dock passes, the Trash
lists through the shared store, and Empty Trash is a confirmation that calls
`files-core` and clears the listing's model.

What landed:

- **`services/files-core/src/optimistic.rs`** — `PendingKind::Empty` +
  `RemovedNode`; `OptimisticModel::begin_empty_trash()` removes every listed
  node in one edit and `revert` restores them in ascending arrival position;
  `empty_trash_via(&dyn TrashOps)`. Two unit tests:
  `empty_trash_clears_every_row_and_reverts_them_in_order`,
  `confirm_empty_trash_keeps_the_cleared_model`.
- **`services/files-core/src/ffi.rs`** — `OpRequest::EmptyTrash` /
  `OpOutcome::EmptyTrash` on the one op worker (calls `TrashOps::empty`);
  `df_files_begin_empty_trash(session)` returns 0 unless the session's
  `Location::scheme() == "trash"` and the model is non-empty. Test
  `empty_trash_abi_rejects_a_non_trash_session`.
- **`apps/files/ffi/files_core.h`** — `df_files_begin_empty_trash` mirrored.
- **`apps/files/FilesDirectoryModel.{h,cpp}`** — `Q_INVOKABLE bool
  emptyTrash()` (same optimistic contract as `trash`: repaint + keep polling).
- **`apps/files/FilesShell.qml`** — `inTrash` (`currentUri === Files.trashUri`),
  `confirmEmptyTrash()`/`emptyTrashNow()`, a design-system `Dialog` (title
  "Empty Trash?", message, Cancel + primary Empty Trash), an Empty Trash entry
  in the Trash background menu, and `Shift+Cmd+Delete`. Capture seam
  `DF_FILES_START_EMPTY_TRASH` opens the dialog once a non-empty Trash listing
  settles. `startEmptyTrash` is exposed on `Files` (`DF_FILES_START_EMPTY_TRASH`).
- **`apps/files/FilesArguments.h`** (new, header-only) — `filesLocationArgument`
  accepts the first URI-with-scheme or absolute path, skipping Qt options.
- **`apps/files/main.cpp`** — maps that argument onto `DF_FILES_START_URI`
  when the env var is unset, so the Dock's `openTrashInFiles`
  (`dragonfruit-files trash://`) opens the Trash.
- **`apps/files/tests/tst_files_shell.cpp`** — adds an `Empty/` mutation
  fixture (x.txt, y.txt) and keeps pointing `XDG_DATA_HOME` at a temp dir.
- **Tests** — Rust totals: 73 unit (was 70) + integration suites, all pass.
  QML: `tst_files_shell`
  `test_trash_location_lists_and_empty_clears_the_model` (trashes real files,
  opens `trash://`, checks the menu + confirmation, empties, asserts the model
  clears); `apps/files/tests/tst_files_arguments.cpp` (new C++ test target).
- **Docs** — `09-files.md` T-10.6b status paragraph. No ADR: the ABI addition
  extends ADR 0050/0051 and the CLI contract is recorded in `09-files.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — 73 unit +
  integration, all pass.
- `make qml-test` — 33/33 ctest green (new `tst_files_arguments`).
- `make lint` green; `make e2e` exit 0.

Live capture (`/tmp/opencode/t67-capture.sh`; still `/tmp/opencode/t67-still.py`;
PNGs `/tmp/opencode/t67-files-empty-trash.png`, `-window.png`, `t67-card.png`).
Launch env: `XDG_DATA_HOME=<temp trash with report.pdf + notes.txt>`,
`DF_DEMO_QT_APP=build/apps/files/dragonfruit-files`,
`DF_FILES_START_URI=trash://`, `DF_FILES_START_EMPTY_TRASH=1`. Raw vision on
the tight card crop: title `Empty Trash?`; message `The items in the Trash will
be deleted immediately. This cannot be undone.`; buttons `Cancel` and
`Empty Trash`; the background Trash list (dimmed by the scrim) shows
`notes.txt` and `report.pdf`; window title `Trash`. Vision is a supporting
check.

Gotchas for later tasks:

- **Empty Trash is one optimistic op, not N deletes.** `PendingKind::Empty`
  carries every removed node and restores them in ascending position order;
  do not model it as a batch of `begin_delete` (the pending count / outcome
  fold would drift).
- **`df_files_begin_empty_trash` is gated on `trash://`** and a non-empty
  model; other locations return 0. The files-core worker's `FreedesktopTrash`
  reads `XDG_DATA_HOME`/`HOME` at worker spawn, so any test that empties must
  point `XDG_DATA_HOME` at a temp dir (the QML runner does).
- **Files accepts a positional location argument.** `main.cpp` only maps it
  when `DF_FILES_START_URI` is empty; the capture harness still wins.
- **`org.dragonfruit.Files.desktop` / `Files1` are not installed yet.**
  `openTrashInFiles` resolves the `.desktop` through the interim index and
  calls `buildLaunchCommand(files, {"trash://"})`; on a host without the
  identity it logs and returns. T-10.6c installs it.
- **Files does not watch the Trash.** A third-party delete (or the Dock's Empty
  Trash) is not pushed into an open Trash listing; re-open/reload to see it.
  Wiring the T-10.3b watcher to the facade via the delta path is still open.
- **Capture seam:** `DF_FILES_START_EMPTY_TRASH=1` only opens when the listing
  reaches `complete` with `count > 0`; `DF_FILES_START_URI=trash://` is
  required.

## T68 — T-10.6c Show in Files, Downloads, and .desktop identity

**State: done.** The Files identity is installed and the Dock's two navigation
paths route through Files: Show in Files reveals an app's executable, a
Downloads-stack row opens the file's folder with the file selected. Files
resolves a file-valued argument to its parent plus a reveal target; a directory
or `trash://` browses itself. See ADR
[0054](design/adr/0054-files-identity-and-reveal-argument.md).

What landed:

- **`apps/files/org.dragonfruit.Files.desktop`** (new) — `Exec=dragonfruit-files
  %U`, `DBusActivatable=true`, `MimeType=inode/directory;`,
  `StartupWMClass=dragonfruit-files`.
- **`apps/files/org.dragonfruit.Files1.service`** (new) — D-Bus activation
  `Name=org.dragonfruit.Files1`, `Exec=/usr/bin/dragonfruit-files`.
- **`apps/files/CMakeLists.txt`** — `install(FILES …)` to
  `${CMAKE_INSTALL_DATADIR}/applications` and `/dbus-1/services`; the app links
  `Qt6::DBus`.
- **`apps/files/main.cpp`** — owns `org.dragonfruit.Files1` on the session bus
  (`registerService`; a second instance only warns).
- **`apps/files/FilesArguments.h`** — `FilesOpenTarget` + inline
  `filesOpenTarget(argument)`: an existing file → `{parent, file://file}`, a
  directory / other scheme → `{itself, ""}`.
- **`apps/files/FilesBridge.{h,cpp}`** — `revealUri` property
  (`DF_FILES_START_REVEAL`, or derived from a file-valued `DF_FILES_START_URI`);
  invokable `resolveOpenTarget(argument)`.
- **`apps/files/FilesShell.qml`** — overridable `startUri`/`revealUri`, and
  `applyReveal()` selects + scrolls to the matching row once the listing is
  `complete`. Exposed `scrollToNode` on both `FilesListView.qml` and
  `FilesIconView.qml`.
- **`shell/src/filestarget.{h,cpp}`** (new, in `dragonfruit-shell-dockcore`) —
  `revealExecutable(entry)`: first token of the launch command, absolute as-is
  or `QStandardPaths::findExecutable`; empty for a terminal wrapper/miss.
- **`shell/src/shellcontroller.{h,cpp}`** — `launchFiles(argument)` is the one
  Files launch path (Trash, reveal, Downloads). `showDockAppInFiles` uses
  `revealExecutable`; `onDockDownloadActivated` launches Files at the file
  instead of `QDesktopServices::openUrl`.
- **Tests** — `shell/tests/tst_dockcore.cpp`: 3 `revealExecutable` cases;
  `apps/files/tests/tst_files_arguments.cpp`: 4 `filesOpenTarget` cases;
  `apps/files/tests/tst_files_shell.qml`: `test_open_target_reveals_a_file_in_
  its_parent`, `test_reveal_selects_the_matching_row`.
- **Docs** — ADR 0054; `09-files.md` T-10.6c status paragraph.

Commands that work (repo root; `make` sets the toolchain env):

- `CARGO_NET_OFFLINE=true cargo test -p dragonfruit-files-core` — all pass.
- `make qml-test` — 33/33 ctest green. If cmake regenerates, export
  `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64` (its cmake needs
  `librhash.so.1`).
- `make e2e` exit 0; `make lint` pieces (fmt, clippy, tokens, design tokens,
  no-capture-grab, check-desktop-names) green.

Live capture (`/tmp/opencode/t68-capture.sh`; still `/tmp/opencode/t68-still.py`;
PNG `/tmp/opencode/t68-files-reveal-window.png`). Launch env:
`DF_DEMO_QT_APP=build/apps/files/dragonfruit-files`,
`DF_FILES_START_URI=<fixture>/notes.txt`, `DF_FILES_START_VIEW=list`. Raw
vision on the crop: breadcrumb `Computer > tmp > dragonfruit-t68-capture.… >
reveal`; rows `Photos`, `archive.tar.gz`, `notes.txt`, `report.pdf`; the
`notes.txt` row is highlighted. That argument is exactly what
`ShellController::launchFiles` passes. Vision is a supporting check.

Gotchas for later tasks:

- **The reveal contract is "a file argument reveals".** `filesOpenTarget` only
  treats an *existing* file as reveal; a missing path browses as given so the
  app shows its own error. A directory browse has `revealUri` empty. Non-ASCII
  names rely on Files and files-core encoding the same `file://` URI.
- **`revealUri` is compared as the exact URI** (`directory.uriAt(i) ===
  root.revealUri`) in `FilesShell.applyReveal`. It runs once
  (`revealApplied`); a reload of the same folder does not re-select.
- **`FilesShell.startUri`/`revealUri` are overridable properties** (defaulting
  to the `Files` singleton) so headless tests can drive reveal without env.
- **The identity is shipped and installable, but the demo does not yet put it
  on `XDG_DATA_DIRS`.** `launchFiles` still resolves
  `org.dragonfruit.Files.desktop` through the interim index and logs/returns
  when absent; installing the CMake files or pointing `XDG_DATA_DIRS` at
  `apps/files` makes the Dock paths live. Running-instance `OpenPaths`/
  `RevealItems` over `org.dragonfruit.Files1` and `DBusActivatable` honoring are
  still T-18.
- **Downloads rows reveal, they do not open with the registered handler.**
  Files has no open-with yet; the demo's "open a file from the Downloads stack"
  is therefore a reveal-into-Files until T-18.
- **Terminal apps have no reveal target**: `buildLaunchCommand` wraps them in an
  emulator, so `revealExecutable` returns empty for `Terminal=true`.

## T69 — T-10.7 Files capture and acceptance walkthrough

**State: done.** T-10 (Files MVP) is captured at the track boundary and the
whole gate is green. The capture is committed under `docs/captures/t10-files.*`
and the large-directory scroll trace is
`docs/captures/t10-files-scroll-trace.txt`.

What landed:

- **`scripts/capture-files.sh`** (new) — `make files-capture` (new target in
  the Makefile). Runs the normal nested demo (`make demo`, `DF_DEMO_QT_APP` →
  `build/apps/files/dragonfruit-files`) once per scene over a scratch fixture
  tree and a scratch freedesktop trash store, and writes the stills + clip.
  `ONLY=name1,name2` re-runs a subset; `KEEP_SCRATCH=1` keeps the temp tree.
- **`scripts/capture-files-driver.py`** (new) — raises the nested Dragonfruit
  window via the KWin scripting D-Bus (`# df-allow-desktop-name`), nudges the
  frame loop over the synthetic-input harness, screenshots the active window
  with `spectacle -b -n -a`, trims to 1920x1200, and optionally crops the Dock
  band.
- **`docs/captures/`** — `t10-files-list.png` (representative `t10-files.png`),
  `-icon.png`, `-context-menu.png`, `-multiselect.png`, `-rename.png`,
  `-trash-empty.png`, `-dock-trash.png`, `-reveal.png`, `-large-directory.png`,
  `-large-directory-scrolled.png`, `t10-files.mp4` (mpeg4, 1280 wide),
  `t10-files-scroll-trace.txt`.
- **Docs** — `docs/captures/README.md` T-10.7 paragraph;
  `docs/design/09-files.md` T-10.7 status note. No ADR (no new decision).

Commands that work (repo root; `make` sets the toolchain env):

- `make e2e` — exit 0.
- `make soak` — `soak passed — 100 clean cycles, zero strays`.
- `make lint` — green; 33/33 ctest (incl. `tst_files_shell`).
- `make files-capture` — regenerates the slice capture (host Wayland,
  `spectacle`, `ffmpeg`, `gdbus`, Pillow; not part of e2e).

Raw numbers in `t10-files-scroll-trace.txt`: warm 1k first frame 0.73 ms
(budget < 50); 100k stream 100000 rows once in ~1.09 s; 40-row viewport read
2.2 µs (budget < 16.6 ms); QML 100k first frame 22 ms, 70 icon / 24 list
delegates, 100 list jumps ~983 ms.

Live capture: raw under `/tmp/opencode/t69/`. Vision on tight crops:
list view (sidebar, toolbar, `Name`/`Date Modified`/`Size`/`Kind`, `Documents`
fixture rows), icon view (8 items), item context menu, `Empty Trash?` dialog
(Cancel / Empty Trash) over a dimmed `Trash` listing, `notes.txt` selected in
its parent after reveal, Dock strip with K/F/X/D tiles + blue Downloads folder
+ red-stroked Trash. No clipping/blur/artifacts inside the window. Vision is a
supporting check.

Gotchas for later tasks:

- **`make files-capture` deliberately does not override `HOME`** (only
  `XDG_CONFIG_HOME`/`XDG_CACHE_HOME`/`XDG_DATA_HOME` are scratch). Changing
  `HOME` breaks the build: the Makefile derives `PKG_CONFIG_PATH` from
  `$(HOME)/.local/df-devroot`, and cargo would rebuild into a fresh
  `$HOME/.cargo`.
- **Capture seams used per scene:** `DF_FILES_START_MENU=item`,
  `DF_FILES_START_SELECT=3`, `DF_FILES_START_RENAME=1`,
  `DF_FILES_START_EMPTY_TRASH=1` (needs `DF_FILES_START_URI=trash://` and a
  non-empty scratch trash), and `DF_FILES_START_URI=<file>` for reveal.
- **Active-window capture can race the KWin raise**: a still may catch the
  host window instead of the nested one (it came out 1685x1163 once). Re-run
  `ONLY=<scene> bash scripts/capture-files.sh` to retake.
- **The "scrolled" 100k still is nearly identical to the top still** — the
  nested synthetic pointer axis cannot scroll a Qt client surface (known
  T-10.4 limitation). The QML `test_large_list_scroll_sweep` is the real
  scroll evidence.
- **Unrelated working-tree changes are present** from another session:
  `docs/design/02-compositor.md`, `docs/design/08-settings.md`,
  `docs/licensing.md`, `docs/design/adr/0055-*`, `docs/design/tracks/18-*`,
  `docs/tasks/172..175-*`. Left untouched.
- **What remains for T-10:** none in this task; the deferred items are the
  files-core folder watcher → facade delta wiring, network-mounted perf, and
  the T-10.6c note that the demo does not yet put the Files `.desktop` on
  `XDG_DATA_DIRS` (T-18 owns the running-instance `Files1` path).

## T70 — T-11.1a Notification service core

**State: done.** The `org.freedesktop.Notifications` service and the shell
banner + history landed. A client `Notify` shows a banner in the nested
session and is recorded. See ADR
[0056](design/adr/0056-notification-service-surface-and-shell-banner.md).

What landed:

- **`services/notifications/`** (new crate `dragonfruit-notifications`,
  workspace member; `df-ipc`, `serde_json=1.0.151`, `zbus=5.19.0`) — the
  service: `model.rs` (queue, bounded history, `replaces_id`, DND bit, lazy
  `expire_due`), `view.rs` (flat JSON), `dbus.rs` (both interfaces at
  `/org/freedesktop/Notifications`), `main.rs`; 13 unit tests.
- **`tests/session_bus.rs`** — 8 integration tests on a private
  `dbus-daemon` (Notify round-trip + model, `Changed`, `CloseNotification`,
  dismiss/expire, `replaces_id`, service expiry, DND).
- **`shell/src/notificationmodel.{h,cpp}`** (dockcore) — the decode seam;
  `banner()` is the newest active banner.
- **`shell/src/notificationclient.{h,cpp}`** — `DbusNotificationClient` +
  `MockNotificationClient` (`DF_NOTIFY_FIXTURE`).
- **`shell/src/shellprotocol.{h,cpp}`** — top-right `notification` overlay
  surface (exclusive `-1`, no keyboard, empty input region).
- **`shell/src/shellcontroller.{h,cpp}`** — a fifth QML scene
  (`NotificationBanner`), client/model wiring, map/unmap; ignores the
  full-output pre-layout configure.
- **`shell/notifications/`** — `NotificationBanner.qml` and a real
  `NotificationCenter.qml` (history list, unmapped).
- **Tests** — `tst_notificationmodel` (5), `tst_notifications` QML (4);
  ctest 35/35.
- **Docs** — ADR 0056; `04-shell.md` T-11.1a status.

Commands that work (repo root; `make` sets the toolchain env; if cmake
regenerates, export `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64`):

- `cargo test -p dragonfruit-notifications` — 13 unit + 8 integration pass.
- `make qml-test` — 35/35; `make lint` green; `make e2e` exit 0.

Live visual check (real path, `/tmp/opencode/t70-real-capture.sh`): private
`dbus-daemon` + `dragonfruit-notifications` + `make dev` on the same bus, a
real `gdbus ... Notify "Files" ... "Copy finished"`. Raw observations: the
shell-facing `Banners` read returned the notification (`id:1`); the captured
card `/tmp/opencode/t70-real-card.png` reads `Files`, `Copy finished`,
`12 items copied to Documents` on a white card with a magenta stripe, no
clipping/artifacts. Fixture: `/tmp/opencode/t70-dev-capture.sh` +
`/tmp/opencode/t70-banner-clean.png` reads `Mail`, `New message`,
`Ada Lovelace — Notes on the Analytical Engine`. Vision is a supporting check.

Gotchas for later tasks:

- **Both interfaces live at `/org/freedesktop/Notifications`.** The shell
  reads `org.dragonfruit.Notifications1` there; the service must own the
  freedesktop name. If a host desktop daemon already owns it the service exits
  and the shell renders no banner (the dev harness does not start the
  service; `make e2e` covers the absent path).
- **Actions are recorded but not advertised or emitted.** `ActionInvoked` is
  declared only; `GetCapabilities` omits `actions`. T-11.1b wires it.
- **One banner at a time** (`NotificationModel::banner()` = newest); older
  active banners stay in `banners()`/history. Stacking/activation are T-11.1b.
- **The banner surface is display-only** (empty input region, no keyboard) so
  clicks pass through; T-11.1b adds an input region for activation.
- **Expiry is service-owned and event-driven** (background thread to the
  earliest deadline, emits `NotificationClosed(1)` + `Changed`); the shell
  keeps no timer.
- **The dev capture reads white** because the shell's Theme follows the host
  `styleHints.colorScheme` when settingsd is absent.
- **`DF_NOTIFY_FIXTURE=1`** seeds one 60 s banner for demo/capture; the live
  D-Bus client is the default.
- **Pre-layout configure is ignored** (`onBannerConfigured` only accepts the
  requested 380x96), the same rule as the menu bar and Dock.

## T71 — T-11.1b Notification actions and Dock badge replacement

**State: done.** Notification actions round-trip to the originating app, the
banner is interactive with an inline action row, and the Dock's transient
launch-failure badge is replaced by a real notification. See ADR
[0057](design/adr/0057-notification-actions-and-dock-failure-notice.md).

What landed:

- **`services/notifications/src/dbus.rs`** — `GetCapabilities` adds `actions`;
  `org.dragonfruit.Notifications1.Invoke(id, action_key)` emits the
  freedesktop `ActionInvoked` to the app then dismisses the banner (reason 2).
  `emit_action_invoked` added.
- **`services/notifications/tests/session_bus.rs`** — capability assertion
  now requires `actions`; new `invoking_an_action_round_trips_action_invoked_and_dismisses`.
- **`shell/src/notificationclient.{h,cpp}`** — client seam gained
  `notify(...)` and `invoke(...)` plus `notified(id)`; Dbus + Mock implement
  both. `MockNotificationClient(parent, seedFixture=false)` for tests. The file
  moved from the shell executable into `dragonfruit-shell-dockcore` so the
  write half is unit-testable.
- **`shell/src/launchfailure.{h,cpp}`** (new, dockcore) —
  `raiseDockLaunchFailure(client, appName, reason)` formats
  `Dock` / "Could not launch <app>" / `<reason>` and calls `notify`.
  `ShellController::failDockLaunch` and the `onDockLaunchTick` timeout call it.
- **`shell/src/shellprotocol.{h,cpp}`** — `setBannerInputRegion(w,h)`,
  `bannerPointerMoved/Button/Left`, and banner tracking in the pointer
  enter/leave/motion/button handlers.
- **`shell/src/shellcontroller.{h,cpp}`** — banner pointer forwarding into the
  offscreen scene (`m_bannerId`, `m_bannerButtons`), `actions` property,
  dynamic input region, `onBannerActionInvoked`/`onBannerActivated` (body click
  = `default` action or dismiss). `DF_DOCK_FAIL_FIXTURE` raises the failure
  once at startup (capture/demo seam).
- **`shell/notifications/NotificationBanner.qml`** — inline action-row Repeater,
  `actionInvoked`/`activated` signals, card grows to 132 with actions.
- **Tests** — `tst_dockcore.cpp`: launch-failure summary + fallback,
  `notify` actions, `invoke` dismissal; `tst_notifications.qml`: action row
  renders/invokes, body click activates. ctest 35/35.
- **Docs** — ADR 0057; `04-shell.md` T-11.1b status.

Commands that work (repo root; `make` sets the toolchain env; if cmake
regenerates, export `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64`):

- `cargo test -p dragonfruit-notifications` — 13 unit + 9 integration pass.
- `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64 make cmake-build` then
  `make qml-test` — 35/35; `make lint` green; `make e2e` exit 0.

Live capture (`/tmp/opencode/t71-actions-capture.sh`,
`/tmp/opencode/t71-seam-capture.sh`; PNGs under `/tmp/opencode/t71-*`):
private `dbus-daemon` + `dragonfruit-notifications` + `make dev` on one bus.
A real `Notify` with actions renders `Files` / `Copy finished` /
`12 items copied to Documents` plus `Reply`/`Archive` buttons. With
`DF_DOCK_FAIL_FIXTURE=1` the failure banner reads `Dock` /
`Could not launch Files` / `The app did not start.`. Vision is a supporting
check.

Gotchas for later tasks:

- **`Invoke` dismisses the banner before the app replies.** The service emits
  `ActionInvoked` then closes with reason 2; an app that wants to keep it open
  must re-`Notify` with `replaces_id`.
- **A missing `Exec` is not an immediate launch failure.** `QProcess::startDetached`
  forks before `exec`, so a nonexistent program still returns true; the Dock
  failure surfaces through the 8 s launch-timeout path, not synchronously.
- **`DF_DOCK_FAIL_FIXTURE`** is a capture seam (raises
  `org.dragonfruit.Files.desktop` 1.5 s after startup); never set normally.
- **The banner surface is 380x132 always** (card 96 or 132); only the card
  region is clickable. `onBannerConfigured` accepts 380x132 now, not 380x96.
- **The launch-failure banner uses the service default 5 s deadline**; a live
  screenshot must land inside that window (the seam script polls `Banners`).
- **`notificationclient.cpp` is now in `dragonfruit-shell-dockcore`**, not the
  shell executable; `tst_dockcore` links it.
- **T-11.2a** owns DND/Focus policy; the service bit and the
  "suppress banner, keep history" rule are unchanged.

## T72 — T-11.2a DND/Focus policy

**State: done.** The notification service now owns a three-mode Focus/DND
policy and the admission rule; semantics are frozen in ADR
[0058](design/adr/0058-focus-dnd-policy-semantics.md). The DND bool is gone.

What landed:

- **`services/notifications/src/policy.rs`** (new) — `FocusMode` (`off` /
  `focus` / `dnd`) and `FocusPolicy` (`admits`, `set_allow_list`,
  `note_batched`, `batched_count`); 6 unit tests.
- **`services/notifications/src/model.rs`** — `Queue` holds one `FocusPolicy`
  (replacing `do_not_disturb: bool`); `HistoryEntry.suppressed`;
  `focus_policy`/`focus_mode`/`set_focus_mode`/`set_focus_allow_list`, plus
  compat `do_not_disturb`/`set_do_not_disturb` (`true`=`dnd`, `false`=`off`).
- **`services/notifications/src/view.rs`** — `history_json` gains
  `suppressed`; new `focus_policy_json` (`mode`, `allowList`, `batchedCount`).
- **`services/notifications/src/dbus.rs`** — `org.dragonfruit.Notifications1`
  adds `FocusPolicy()`, `SetFocusMode(name) -> bool` (unknown rejected,
  previous mode kept), `SetFocusAllowList(apps)`; both setters emit `Changed`.
  The T-11.1a `DoNotDisturb`/`SetDoNotDisturb` pair is kept as a compat
  mapping.
- **`services/notifications/src/main.rs`** — `--print-capabilities` now
  matches `CAPABILITIES` (added the missing `actions`).
- **Tests** — `cargo test -p dragonfruit-notifications`: 24 unit + 12
  integration (3 new session-bus tests) pass.
- **Docs** — ADR 0058; `04-shell.md` T-11.2a status.

The rules (frozen): `off` banners everything; `focus` banners allow-listed
apps and `critical` urgency; `dnd` banners allow-listed apps only (silences
`critical` too). A suppressed notification is recorded in history with
`suppressed: true` and counted in the batch, which clears when the mode
returns to `off`. No synthetic summary banner is ever emitted.

Commands that work (repo root; `make` sets the toolchain env):

- `cargo test -p dragonfruit-notifications` — 24 unit + 12 integration pass.
- `cargo clippy -p dragonfruit-notifications --all-targets -- -D warnings`,
  `cargo fmt -p dragonfruit-notifications -- --check` — green.

Gotchas for later tasks:

- **Mode names on the wire are `off`/`focus`/`dnd`** (`do-not-disturb` is
  accepted as an alias); `SetFocusMode` returns `false` for anything else and
  leaves the mode unchanged.
- **`History()` entries now carry a `suppressed` boolean** — additive for the
  shell decoder in `apps/`/`shell/`.
- **The batch is cleared only by returning to `off`**, not by switching
  between `focus` and `dnd`; `FocusPolicy().batchedCount` is the current
  suppression session.
- **`SetDoNotDisturb(false)` forces `off`** (and clears the batch); it is the
  T-11.1a compat surface, not a way to get back to `focus`.
- **No shell source changed in T-11.2a.** The shell's existing
  `setDoNotDisturb` still works through the compat method. T-11.2b must add a
  `FocusPolicy()` reader to `shell/src/notificationclient.{h,cpp}` and reflect
  `mode` + `batchedCount` in the bar via `Changed`.
- **Scheduling Focus windows and per-app silencing were not built**; the
  policy value is additive if a later task needs them.

## T73 — T-11.2b DND/Focus menu-bar reflection and Dock failure path

**State: done.** The menu bar reflects the notification service's Focus/DND
policy; the Dock launch-failure path is the T-11.1b real notification
(unchanged). The reflection contract is frozen in
[0059](design/adr/0059-menu-bar-focus-reflection.md).

What landed:

- **`shell/src/notificationclient.{h,cpp}`** — the seam gains
  `setFocusMode(mode)` and `focusPolicyChanged(json)`; `refresh()` now also
  reads `FocusPolicy()`. `DbusNotificationClient` calls
  `FocusPolicy`/`SetFocusMode`; `MockNotificationClient` keeps the mode
  (rejects unknown names, clears the batch on `off`) and mirrors the T-11.1a
  `setDoNotDisturb` bool.
- **`shell/src/notificationmodel.{h,cpp}`** — `applyFocusPolicyJson` and
  `focusPolicy()`; a malformed/missing payload clears the policy (never stale).
- **`shell/src/focusstatus.{h,cpp}`** (new, dockcore) —
  `focusStatusItem(policy)` maps the policy map to the `focus` status item:
  hidden for `off`/absent, crescent for `focus`, `selected` (accent) for
  `dnd`; label = `batchedCount` when > 0; accessible name always carries
  mode/count.
- **`shell/src/shellcontroller.{h,cpp}`** — `onNotificationFocusPolicy`
  decodes and rebuilds the status row; the `focus` item now comes from
  `focusStatusItem` instead of a hardcoded hidden slot; the service-absent
  path clears the policy. `DF_FOCUS_FIXTURE=<mode>` starts the
  `DF_NOTIFY_FIXTURE` mock in that mode (capture/demo only).
- **Tests** — `tst_notificationmodel` policy decode + safe default;
  `tst_dockcore` `focusStatusItem` mapping + mock `setFocusMode`/compat
  round-trip; `tst_menubar.qml` renders the three states. ctest 35/35.
- **Docs** — ADR 0059; `04-shell.md` T-11.2b status.

Commands that work (repo root; `make` sets the toolchain env):

- `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64 ctest --test-dir build
  -R "tst_notificationmodel|tst_dockcore|tst_menubar"` — 3/3.
- `make qml-test` — 35/35; `make lint` and `make e2e` green.

Live capture (`/tmp/opencode/t73-capture.sh` + `/tmp/opencode/t73-driver.py`;
PNGs `/tmp/opencode/t73-focus.png`, `t73-menubar.png`): `make demo` with
`DF_NOTIFY_FIXTURE=1 DF_FOCUS_FIXTURE=dnd`; the nested-bar crop reads an
accent-tinted crescent immediately left of the clock, then the Control Center
and Mission Control marks. No clipping/artifacts. Vision is a supporting check.

Gotchas for later tasks:

- **The menu-bar item is read-only.** It reflects the policy; clicking it is
  the generic `statusItemActivated("focus")` no-op. The Control Center
  (T-11.3a) owns the active Focus controls. `selected`=DND is the frozen
  visual convention.
- **The shell learns the policy on `Changed`** (the client's `refresh()` reads
  `Banners`/`History`/`FocusPolicy` together). `amend` the service only through
  `SetFocusMode`/`SetFocusAllowList`; it emits `Changed`.
- **`focusPolicyChanged` is NOT emitted by the mock when the mode is
  unchanged** (and unknown names are ignored), so a toggle test must set a
  different mode.
- **`DF_FOCUS_FIXTURE` requires `DF_NOTIFY_FIXTURE`** — it only seeds the mock
  client. Values are `off`/`focus`/`dnd` (alias `do-not-disturb`).
- **No pixel assertion in the headless suites**; the accent-tinted crescent
  was checked only by the live capture above.

Continuation (attempt 2) — the "failed" verdict was a build-environment bug,
not a T73 code defect:

- **Real cause:** the harness verify ran bare `make e2e`; `make cmake-build`
  failed at `Automatic MOC and UIC` with
  `/home/user/.local/df-toolchain/usr/bin/cmake: error while loading shared
  libraries: librhash.so.1`. `build/build.ninja` records the toolchain cmake
  (`CMAKE_COMMAND=/home/user/.local/df-toolchain/usr/bin/cmake`), and ninja's
  autogen rule calls it even though a different cmake
  (`~/.local/bin/cmake`, uv-installed) is first on `PATH`. The Makefile only
  exported `LD_LIBRARY_PATH=$DF_TOOLCHAIN/lib64` when cmake was *absent* from
  `PATH`, so that loaded lib was not found.
- **Fix (`Makefile`):** the `ifneq ($(wildcard $(DF_TOOLCHAIN)/lib64),)` block
  now always prepends `$(DF_TOOLCHAIN)/lib64` to `LD_LIBRARY_PATH`; the
  PATH-fallback guard only adds the toolchain `bin/` when cmake/ctest are not
  on `PATH`. No shell/T73 source changed.
- **Verified:** `make e2e` exit 0 (bare, no manual env); `make qml-test` 35/35;
  `make cmake-build` exit 0. Live capture re-inspected via
  `./.symphony/symphony vision /tmp/opencode/t73-focus.png` — accent-tinted
  crescent left of the clock, sharp, no artifacts.
- **Gotcha for every later task:** a bare `make <target>` must work; do not
  assume the caller exported the toolchain `LD_LIBRARY_PATH`.

## T74 — T-11.3a Control Center panel and core tiles

**State: done.** The Control Center panel opens (menu-bar item or
Control-Option-C) and shows Wi-Fi / Sound / Display tiles; Sound and Display
apply live; Escape and click-away dismiss. Contract frozen in ADR
[0060](design/adr/0060-control-center-panel-and-brightness.md).

What landed:

- **`shell/control-center/ControlCenter.qml`** (rewritten) — the panel scene
  with a normalized `tiles` model (headless test seam) and signals
  `volumeSetRequested` / `brightnessSetRequested` / `muteToggleRequested` /
  `wifiToggleRequested` / `wifiSettingsRequested` / `closed`. The module is
  now `qt_add_library(... STATIC)` linked to `dragonfruit-design-systemplugin`
  and to the shell executable.
- **`shell/src/shellprotocol.{h,cpp}`** — top-right `control-center` overlay
  layer surface (anchors `top|right`, `exclusive_zone -1`, keyboard
  `ON_DEMAND`, whole-panel input region) plus pointer/keyboard classification
  and `onOutputBrightness`.
- **`shell/src/shellcontroller.{h,cpp}`** — the offscreen panel scene,
  `toggle/show/hideControlCenter`, `renderControlCenter`, pointer + keyboard
  routing, the `control-center` input action, Escape dismissal, and the tile
  gestures. Brightness writes settingsd `display.brightness`; volume/mute
  reuse the T-07 status client.
- **`protocols/dragonfruit-toplevel.xml`** — `df_output` v2 adds
  `set_brightness` + the `brightness` event, appended *after* `removed` so
  the scanner's `since` stays non-decreasing and existing opcodes are stable.
- **`compositor/src/shell/mod.rs`** — `DF_OUTPUT_INTERFACE_VERSION = 2`, a
  per-output `output_brightness` map, the `SetBrightness` clamp/store arm, and
  the readback event. **`compositor/src/input/{action,shortcuts}.rs`** — the
  `ControlCenter` `InputAction` (`control-center`) bound to Control-Option-C.
- **`services/settingsd/src/schema.rs`** + `docs/settings-keys.md` —
  `display.brightness` (d, 0.0–1.0, default 1.0, since 4; `SCHEMA_VERSION`
  4). **`libs/settings-client/settingsclient.cpp`** mirrors the default.
- **`shell/src/displayspolicy.{h,cpp}`** — `DisplaySettings.brightness`.
- **`design-system/components/Icon.qml`** — `wifi`, `bluetooth`, `brightness`
  painted glyphs.

Commands that work (repo root; `make` sets the toolchain env):

- `make e2e` exit 0; `make lint` exit 0 (ctest 36/36, includes
  `tst_controlcenter` 10/10 and `qmllint_shell-control-center`).
- `cargo test -p dragonfruit-settingsd` — schema/doc/bus green.
- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance`
  33/33; `--bin dragonfruit-compositor` 275/275.

Live capture: `scripts/capture-control-center.sh` (`DF_STATUS_FIXTURE=1`,
Control-Option-C) writes `docs/captures/t11-control-center.png` and
`t11-control-center-context.png`. Vision check: Wi-Fi tile (toggle +
`Wi-Fi Settings…`), Sound at 60% + Mute, Display brightness slider, rounded
tiles, no artifacts.

Gotchas for later tasks:

- **Wi-Fi is the one non-live core tile.** The T-07 NetworkManager adapter has
  no radio-write method, so the tile toggle reflects `radioEnabled` and is
  inert (`wifiWritable=false`; the tile model already carries `enabled`).
  T-15 adds the write and flips the property.
- **Brightness is stored/clamped by the compositor but applied best-effort.**
  Headless/nested keep the value only; DRM is a later task. `df_output` is v2
  and the `brightness` event is `fixed` (24.8) — compare with ~1/256
  tolerance.
- **`shell/control-center` now depends on `dragonfruit-design-systemplugin`**
  and is linked into `dragonfruit-shell` + `qt_import_qml_plugins`.
- **Panel dismissal:** Escape is handled both in the QML panel and in
  `ShellController::onKeyEvent`; click-away is `keyboardFocused(false)`
  deferred one event-loop turn, plus `controlCenterKeyboardFocused(false)`.
  T-11.3b must keep the deferral (bar→panel focus movement is not a dismissal).
- **The Wi-Fi settings link logs only** — Settings-on-a-pane launch is T-16.
- **Add `since`-versioned protocol members at the END** of the interface
  (after events/enums), as `df_toplevel_manager` already does, or
  wayland-scanner warns "since version not increasing".

## T75 — T-11.3b Focus/DND, dark mode, and Control Center a11y

**State: done.** The Control Center panel has five tiles now: Wi-Fi, Focus,
Sound, Display, Dark Mode. Focus/DND and dark mode apply live; accessible
roles are on every tile and link. Contract frozen in ADR
[0061](design/adr/0061-control-center-toggles-and-a11y.md).

What landed:

- **`shell/control-center/ControlCenter.qml`** — Focus and Dark Mode tiles, a
  `TextLink` component for the trailing `… Settings…` links, five-tile `tiles`
  model, and the new signals `focusToggleRequested(bool)` /
  `focusSettingsRequested()` / `darkModeToggleRequested(bool)` /
  `appearanceSettingsRequested()`. New properties `focusPolicy` (the
  notification-service view; the property is not named `focus` because
  `QQuickItem.focus` already exists) and `dark` (effective scheme). External
  switch state is applied through `Binding` elements, not `checked:`
  bindings.
- **`shell/src/controlcenterpolicy.{h,cpp}`** (new, in dockcore) —
  `focusModeForToggle` (`dnd`/`off`) and `colorSchemeForDarkToggle`
  (`dark`/`light`).
- **`shell/src/shellcontroller.{h,cpp}`** — `applyControlCenterData` pushes
  `focusPolicy` and the effective `dark` (via
  `ThemeBinding::darkForScheme`); four new slots write `setFocusMode(...)` to
  the notification client and `appearance.colorScheme` to settingsd;
  `onNotificationFocusPolicy` refreshes an open panel. Panel surface is now
  `kControlCenterHeight = 520` (was 420).
- **`design-system/components/Toggle.qml`** — optional `accessibleName`
  (falls back to `text`).
- **`design-system/components/Icon.qml`** — new `focus` crescent glyph.
- **Tests** — `tst_controlcenter.qml` 18/18; `tst_dockcore` mapping tests.
  `make qml-test` 36/36; `make lint` and `make e2e` exit 0.
- **Docs** — ADR 0061; `04-shell.md` T-11.3b status; `settings-keys.md`
  `appearance.colorScheme` note.

Commands that work (repo root; `make` sets the toolchain env):

- `make qml-test` — 36/36; `make lint` exit 0; `make e2e` exit 0.
- `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64 ctest --test-dir build
  -R tst_controlcenter` — 18/18.

Live capture: `scripts/capture-control-center-focus-dark.sh` +
`...-driver.py` (run with `DF_STATUS_FIXTURE=1 DF_NOTIFY_FIXTURE=1
DF_FOCUS_FIXTURE=dnd`) writes `docs/captures/t11-control-center.png` (five
tiles, Focus `Do Not Disturb`), `t11-control-center-focus-dark.png` (Focus
clicked off), `t11-control-center-dark-toggle.png` (Dark Mode on, whole panel
dark), and `t11-control-center-context.png`. Vision confirmed all five tiles
and both live flips; no clipping or artifacts.

Gotchas for later tasks:

- **Focus tile is Do Not Disturb only.** On=`dnd`, off=`off`; the middle
  `focus` mode is not selectable from the tile but still lights it (and the
  menu-bar crescent) when set elsewhere. Mode/count always come from
  `FocusPolicy()`.
- **Dark Mode writes absolute `dark`/`light`, never `auto`.** It goes through
  `SettingsClient::set`; `ThemeBinding` consumes the `changed` echo and flips
  the Theme. The tile shows the effective `auto`-resolved state.
- **The Focus/Appearance settings links log only** (Settings-on-a-pane is
  T-16), like the Wi-Fi link.
- **`shadowing`: do not name a panel property `focus`.** Use `focusPolicy`;
  `focus` collides with `Item.focus` ("Property value set multiple times").
- **The notification fixture seeds a banner at the panel's top-right corner.**
  The capture driver dismisses it (click the banner body) before opening the
  panel, or the banner occludes the Wi-Fi tile.
- **`Toggle` writes its own `checked`**; bind external state with a `Binding`
  element (the Settings panes' pattern), not `checked:`.
- **The panel is 360×520**; the capture driver's switch centres
  (panel-relative 320,131 and 320,419) come from the QML layout and must be
  updated if the tile order/heights change.

## T76 — T-11.4a OSD overlay

**State: done.** A volume/brightness change presents a brief centered OSD card
on the active output, fades, and dismisses; a fullscreen surface suppresses it
and reduced motion collapses the fade to an immediate 1.0. Contract frozen in
ADR [0062](design/adr/0062-osd-overlay.md).

What landed:

- **`shell/src/osdmodel.{h,cpp}`** (new, dockcore) — the pure `OsdModel`:
  `present` (last change wins, so concurrent volume+brightness coalesce),
  `tick` auto-dismiss at `kDismissMs = 1400`, `hide`, `setFullscreen`
  (suppresses and hides at once), and `fade(now, reducedMotion)` (1.0 for
  reduced motion; ramp-in 120 ms / ramp-out 220 ms otherwise). No QObject, no
  timer.
- **`shell/osd/Osd.qml`** (new module `Dragonfruit.Osd`) — a pure view driven
  by `kind`/`value`/`muted`/`fade`: the design-system glyph, a level track
  (accent fill, empty when muted), and the percentage/`Muted` label, plus an
  `Accessible.Alert` name.
- **`shell/src/shellprotocol.{h,cpp}`** — the centered `overlay` surface
  (`namespace "osd"`, 220×220, no anchors, `exclusive_zone -1`, keyboard NONE,
  empty input region), `commitOsdImage`/`hideOsd`, the `osdConfigured` signal,
  and `fullscreenOverlayActive()` (active Space marked fullscreen, with the
  focused toplevel's fullscreen bit as a fallback).
- **`shell/src/shellcontroller.{h,cpp}`** — the offscreen OSD scene, the
  `showOsd`/`renderOsd`/`hideOsd` path, a 16 ms timer that runs only while the
  OSD is visible, and the triggers: `onVolumeSetRequested`,
  `onMuteToggleRequested`, `onBrightnessSetRequested`. `DF_OSD_FIXTURE`
  (`volume`/`brightness`) presents one OSD after startup for the live check.
- **`design-system/tokens/tokens.json`** — `component.osd` (180×180 card,
  radius/padding/iconSize/gap/trackHeight/trackRadius/fontSize) and `motion.osd`;
  `Theme.qml` and `compositor/src/design_tokens.rs` regenerated.
- **Tests** — `tst_dockcore`: five `OsdModel` tests (kind/value/clamp,
  deadline, fullscreen, reduced-motion fade, coalescing). `tst_osd` (new QML
  test): glyph, level track, muted, fade→opacity, accessible role. `make
  qml-test` 38/38; `make lint` and `make e2e` exit 0.
- **Docs** — ADR 0062; `04-shell.md` T-11.4a status.

Commands that work (repo root; `make` sets the toolchain env):

- `make qml-test` — 38/38; `make lint` exit 0; `make e2e` exit 0.
- `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64 ctest --test-dir build
  -R "tst_dockcore|tst_osd"` — 2/2.

Live check (`/tmp/opencode/t76-osd-live.sh` + `t76-osd-driver.py`): `make demo`
with `DF_STATUS_FIXTURE=1`, open Control Center (Control-Option-C), drag the
Sound slider → `/tmp/opencode/t76-osd.png` (and `t76-osd-context.png`). Vision:
a centered rounded card with the speaker glyph, an accent level bar at 100%,
and the percentage, sharp, no clipping or artifacts. The still is intentionally
not committed — the capture set is T-11.4b.

Gotchas for later tasks:

- **The OSD is suppressed while a fullscreen Space is active.** That includes
  the Control Center shortcut over a fullscreen window; volume/brightness are
  not treated as critical warnings.
- **`fullscreenOverlayActive()` is derived from the workspace projection**
  (active && fullscreen), with the focused toplevel's `state & 0x4` as a
  fallback. If a future task changes when the fullscreen Space is marked
  active, revisit it.
- **The OSD surface is unanchored and centered by the compositor**; it targets
  the chrome-focus output (per-output overlay rule) and falls back to every
  output before any chrome focus. Multi-output "active output" pinning is a
  later enhancement.
- **Only the shell's own gestures trigger the OSD** (Control Center sliders,
  menu-bar volume menu, mute). There is no compositor media-key input action
  yet; route one into `ShellController::showOsd` later. External volume pushes
  (`onAudioState`) deliberately do not present the OSD to avoid double-showing
  after our own write.
- **`DF_OSD_FIXTURE` requires the shell to start with it; values are
  `volume` (default) and `brightness`.** It fires once, 1500 ms after startup.
- **The OSD card color is `surfaceElevated`.** In a live capture the level bar
  is the only reliable accent marker; the demo app behind it can share the card
  color, so detect the accent bar, not the card fill.

## T77 — T-11.4b OSD keyboard/a11y and captures

**State: done.** The OSD is keyboard/AT-SPI accessible and the T-11 capture
set is committed. Contract frozen in ADR
[0063](design/adr/0063-osd-keyboard-atspi.md).

What landed:

- **`shell/osd/Osd.qml`** — `Accessible.role: Alert` + value-derived
  `Accessible.name` (unchanged) plus `Accessible.description` ("Volume 60
  percent. Press Escape to dismiss." / "Volume is muted. Press Escape to
  dismiss."), `Accessible.focusable`, `Accessible.onPressAction`, a
  `dismissed()` signal, and `Keys.onEscapePressed`.
- **`shell/src/shellcontroller.cpp`** — `onKeyEvent` hides a visible OSD on
  Escape before the Control Center check (the OSD window is never focused);
  the OSD object's `dismissed()` connects to `hideOsd()`.
- **`design-system/components/Icon.qml`** — `volume` is now a speaker
  (body + cone + two waves); the T-11.4a `volume` case drew a drive slab that
  the capture read as a battery. Shared by the OSD and the Control Center
  Sound tile; the gallery icon snapshot renders other glyphs, so it is
  unaffected.
- **Tests** — `tst_osd.qml` 7 cases (role/name/description, muted wording,
  Escape -> `dismissed`). `make qml-test` 38/38; `make lint` and `make e2e`
  exit 0. `ctest -R tst_osd` 9/9.
- **Captures** — `scripts/capture-osd-dnd.sh` + `scripts/capture-osd-dnd-driver.py`
  (new; `make osd-dnd-capture`) write `docs/captures/t11-osd.png`,
  `t11-osd-context.png`, `t11-dnd.png`; re-ran
  `scripts/capture-control-center-focus-dark.sh` to refresh
  `t11-control-center*.png` with the speaker glyph.
- **Docs** — ADR 0063; `04-shell.md` T-11.4b status; `docs/captures/README.md`.

Commands that work (repo root; `make` sets the toolchain env):

- `make qml-test` — 38/38; `make lint` exit 0; `make e2e` exit 0.
- `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64 ctest --test-dir build
  -R tst_osd` — 9/9.
- `make osd-dnd-capture` — writes the three new stills (host Wayland +
  `spectacle` + Pillow).

Capture verdict (vision): `t11-osd.png` — centered card, speaker glyph, accent
bar, `100%`, sharp; `t11-osd-context.png` — card centered over the Settings
window, Control Center open, DND crescent in the bar; `t11-dnd.png` — accent
crescent immediately left of the clock. No clipping/artifacts.

Gotchas for later tasks:

- **The fixture mock seeds a banner even in DND.** The capture driver clicks it
  away before the OSD drag; the real service's suppression is the Rust policy.
- **The OSD never takes focus; Escape is shell-level.** A live screen-reader
  announcement depends on the AT-SPI tree, whose dump/walkthrough is T-16.6a.
  Hardware media keys are still unwired.
- **`Icon.qml` `volume` is shared** by the OSD and the Control Center Sound
  tile; do not reintroduce the drive-slab drawing.

## T78 — T-12.1a Session manager and restart policy

**State: done.** `services/session` is a real session manager: the composition
is data (`SessionPlan`/`ServiceSpec`) and a runtime-free `Supervisor` starts
services by stage and restarts them per policy. Contract frozen in ADR
[0064](design/adr/0064-session-manager-plan-and-restart-policy.md).

What landed:

- **`services/session/src/plan.rs`** (new) — `RestartPolicy` (`always` /
  `on-failure` / `never`), `ServiceSpec` (name, program, args, env, policy,
  stage, `ends_session`, `gate`, builders), `SessionPlan`
  (`default_session`, `stages`, `services_in_stage`, `anchor`, `validate`),
  `PlanError`. Default plan mirrors `11-session-and-dev-workflow.md`: stage 0
  `compositor` (Never, anchor, gate); stage 1 `shell` (Always) + `settingsd` /
  `menu-broker` / `app-index` / `notifications` (OnFailure); stage 2 `portal`.
- **`services/session/src/supervisor.rs`** (new) — `Supervisor::start`,
  `tick` (reap + policy + stage advance), `set_ready`, `kill`, `shutdown`;
  `ExitOutcome`, `ServiceState`, `SessionState`, `SupervisorEvent`. The anchor
  is never restarted; its exit stops survivors and ends the session.
- **`services/session/src/main.rs`** — `--print-plan` (default) and
  `--exec PROG [ARGS...] [--policy P]`; SIGINT/SIGTERM teardown.
- **Tests** — `tests/restart.rs` 8 cases + 5 unit tests; `Makefile` `e2e`
  runs `cargo test -p dragonfruit-session`.

Commands that work (repo root):

- `cargo test -p dragonfruit-session` — 13/13.
- `cargo clippy -p dragonfruit-session --all-targets -- -D warnings` and
  `cargo fmt -p dragonfruit-session -- --check` — exit 0.
- `cargo run -p dragonfruit-session --bin dragonfruit-session -- --print-plan`
  prints stage 0 `compositor never … anchor` first.
- `cargo run -p dragonfruit-session -- --exec sh -c 'exit 0' --policy always`
  prints repeated `restarted exec (…, #N, after exit 0)`.

Live visual check: no surface of its own; the nested session was launched
(`make demo`, host Wayland) and captured at `/tmp/opencode/t78-demo.png` —
menu bar, Dock, Settings window, and X11 demo window all render cleanly, no
artifacts. `make demo DEMO_ARGS=--headless` (e2e) also launches the unchanged
composition and tears down cleanly; `make e2e` and `make lint` are green. The
nested session capture set is T-12.2 / track-boundary.

Gotchas for later tasks:

- **T-12.1b attaches env via `ServiceSpec::env` (empty in T-12.1a) and calls
  `Supervisor::set_ready("compositor")`** once the private socket exists; do
  not change stages/policies.
- **The production supervisor is T-12.1b's systemd user units.** The
  in-process `Supervisor` is the headless/testable contract the units must
  agree with; the binary defaults to `--print-plan`, not run.
- **`shutdown` SIGKILLs direct children only**; T-12.2's logout teardown
  should extend it to process groups.
- **A `gate` service that has already exited does not block its stage** (no
  deadlock); the anchor is `ends_session` and must be `RestartPolicy::Never`
  or `validate()` rejects the plan.
- **`on-failure` treats a signal as failure** (`ExitOutcome::Signaled`), so a
  killed service restarts; a clean `exit 0` does not.

## T79 — T-12.1b Session environment, systemd units, second-VT

**State: done.** The session environment is data and reaches every child; the
systemd user units in `services/session/units/` are the production supervisor,
one per `ServiceSpec`, gated on the compositor's socket; the dedicated-user
second-VT workflow is documented. Contract frozen in ADR
[0065](design/adr/0065-session-environment-and-units.md).

What landed:

- **`services/session/src/env.rs`** (new) — `SessionEnvironment` (`new`,
  `default_socket`, `with_x11_display`, `with_launch_token`,
  `with_generated_token`, `base`, `trusted`, `base_value`,
  `x11_display_from_handoff`), `generate_launch_token`, and the variable-name
  constants. Base = `XDG_CURRENT_DESKTOP=dragonfruit`,
  `XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY=<socket>`, `DISPLAY=<:N>` when
  Xwayland is up; trusted = base + `DRAGONFRUIT_LAUNCH_TOKEN`.
- **`services/session/src/plan.rs`** — `ServiceSpec` gained `trusted` (true for
  compositor + shell); `SessionPlan::with_environment` /
  `default_session_for` attach the environment. Stages/policies unchanged.
- **`services/session/units/`** (new) — `dragonfruit-session.target` plus the
  seven service units. Compositor `Restart=no` (`--backend drm`), shell
  `Restart=always`, others `Restart=on-failure`. Every non-compositor unit has
  `ExecStartPre=/usr/bin/dragonfruit-session --wait-socket dragonfruit-wayland`,
  `Environment=` for the three static variables, and
  `PassEnvironment=DISPLAY DRAGONFRUIT_LAUNCH_TOKEN`.
- **`services/session/src/main.rs`** — `--print-env [--socket-name NAME]
  [--x11-display DISPLAY]` and `--wait-socket NAME [--timeout SECS]`.
- **Tests** — `tests/environment.rs` 6 cases (trusted child sees all five
  variables; untrusted child sees no token; `--wait-socket` success/timeout;
  `--print-env` contract + 64-hex token) and `tests/units.rs` 6 cases (unit
  set, target pulls the composition, anchor `Restart=no`, per-service
  policies, environment/gate on every service, dependency closure).
- **Docs** — `11-session-and-dev-workflow.md` (environment contract + shipped
  units + units-based second-VT), `testing-ladder.md` rung 2, ADR 0065.

Commands that work (repo root):

- `cargo test -p dragonfruit-session` — 30/30.
- `cargo clippy -p dragonfruit-session --all-targets -- -D warnings` and
  `cargo fmt -p dragonfruit-session -- --check` — exit 0.
- `make lint` — exit 0 (`qml-test` 38/38); `make e2e` — exit 0.
- `cargo run -p dragonfruit-session -- --print-env` — five `KEY=VALUE` lines,
  64-hex token.

Live check: `./target/debug/dragonfruit dev --demo --nested --socket-name
t79-demo`, captured at `/tmp/opencode/t79-demo.png`; vision confirms the nested
Dragonfruit window renders its menu bar, Dock, Settings, and the X11 demo
window with no clipping/artifacts. The task adds no surface of its own.

Gotchas for later tasks:

- **T-12.2 owns the `.desktop` session entry and unit installation.** The
  entry runs `eval "$(dragonfruit-session --print-env --socket-name
  dragonfruit-wayland)"`, imports `XDG_CURRENT_DESKTOP XDG_SESSION_TYPE
  WAYLAND_DISPLAY DISPLAY DRAGONFRUIT_LAUNCH_TOKEN`, then
  `systemctl --user start dragonfruit-session.target`.
- **The fixed production socket is `dragonfruit-wayland`**
  (`env::DEFAULT_SOCKET_NAME`); the units and `WAYLAND_DISPLAY` must stay in
  lockstep. The nested dev tool keeps `dragonfruit-dev-<pid>`.
- **The launch token is minted once by `--print-env` and passed through the
  user manager** (`PassEnvironment=`). The compositor pre-mints it
  (`shell::provision` already reads `DRAGONFRUIT_LAUNCH_TOKEN`) and writes
  `<socket>.launch-token`; the shell reads it from the environment. Never log
  the value.
- **The compositor is `Type=simple`.** The readiness gate is the
  `--wait-socket` `ExecStartPre`, not systemd `Type=notify`. A native
  `sd_notify` would remove the helper (follow-up).
- **`--wait-socket` tests file existence, not a connect** — it can pass on a
  stale socket path; teardown correctness is the compositor's job.

## T80 — T-12.2 Display-manager entry and logout teardown

**State: done.** The session now has a display-manager `.desktop` entry, an
entry script that owns startup and logout teardown, and a supervisor that
tears down whole process groups. Contract frozen in ADR
[0066](design/adr/0066-display-manager-session-entry.md).

What landed:

- **`services/session/dragonfruit.desktop`** (new) — Wayland session
  descriptor: `Name=Dragonfruit`, `DesktopNames=dragonfruit`,
  `Type=Application`, `Exec=/usr/bin/dragonfruit-session-entry`. Install target
  `share/wayland-sessions/`.
- **`services/session/dragonfruit-session-entry`** (new, 0755) — startup +
  logout owner: `--print-env` → `systemctl --user import-environment` (5 vars)
  → `start dragonfruit-session.target` → wait for
  `dragonfruit-compositor.service` inactive → `stop` + `reset-failed` target →
  remove `$XDG_RUNTIME_DIR/<socket>{,.lock,.x11-display,.launch-token}`.
- **`services/session/src/entry.rs`** (new) — contract constants,
  `include_str!`-embedded `DESKTOP_ENTRY`/`ENTRY_SCRIPT`/`UNIT_FILES`, and
  `install_into(prefix)` writing `bin/`, `share/wayland-sessions/`,
  `lib/systemd/user/` (entry at 0755).
- **`services/session/src/main.rs`** — `--install-session DIR` prints the 10
  written paths.
- **`services/session/src/supervisor.rs`** — services spawn as process-group
  leaders (`Command::process_group(0)`); `shutdown`, anchor-exit, `Drop`, and
  `kill` signal the group (`SIGTERM`, then `SIGKILL` after 500 ms).
- **Tests** — `tests/session_entry.rs` 4 (embedded/shipped drift guard, desktop
  contract, install layout + exec bit, real entry-script run against fake
  `systemctl`/`dragonfruit-session`); `tests/logout.rs` 3 (shutdown,
  anchor-exit, `kill` each reap a wrapper's `sleep` grandchild).
- **Docs** — ADR 0066; `11-session-and-dev-workflow.md` entry + teardown
  section; `12-packaging.md` package contents; `testing-ladder.md` rung 2.

Commands that work (repo root):

- `cargo test -p dragonfruit-session` — 40/40 (13 unit + 27 integration).
- `cargo clippy -p dragonfruit-session --all-targets -- -D warnings`; `cargo
  fmt -p dragonfruit-session -- --check` — exit 0.
- `cargo run -p dragonfruit-session -- --install-session /tmp/prefix` —
  writes the entry, desktop file, and 8 units.
- `make lint` — exit 0 (`qml-test` 38/38); `make e2e` — exit 0.

Live check: `./target/debug/dragonfruit dev --demo --nested --socket-name
t80-demo`, captured at `/tmp/opencode/t80-demo.png`; vision confirms the nested
window renders its menu bar, Dock, Settings, and X11 demo window with no
clipping/artifacts; the dev tool reported a clean teardown. The task adds no
surface of its own.

Gotchas for later tasks:

- **The entry script is the only startup/logout owner.** T-12.6a selects
  `dragonfruit.desktop` by name (`entry::SESSION_FILE`); T-12.6b/c reuse the
  runtime hand-off cleanup list (`<socket>{,.lock,.x11-display,.launch-token}`).
- **T-32 packaging installs with `dragonfruit-session --install-session
  "$RPM_BUILD_ROOT/usr"`** (or `entry::install_into`); it does not hand-copy
  `services/session/units/`.
- **`entry.rs` embeds the units at compile time**, so a unit edit that skips
  the installer is caught by `tests/session_entry.rs`'s drift guard.
- **Teardown waits up to 500 ms per service** on `SIGTERM` before `SIGKILL`;
  a service that ignores `SIGTERM` costs that grace at logout.
- **The `.desktop` `Exec` is the absolute `/usr/bin/dragonfruit-session-entry`.**
  A non-standard prefix needs the entry edited or a symlink; T-12.6's harness
  should use the installed path.
- **`process_group(0)` is Unix-only** (`#[cfg(unix)]`); the crate already
  assumes Unix throughout.

## T81 — T-12.3a Lock protocol and lock UI

**State: done.** `ext-session-lock-v1` is enforced end to end and the shell is
the first-party lock UI. The compositor owns the fail-secure state and drops
client input while locked; the shell binds the standard protocol and paints a
`Dragonfruit.Lock` scene into one lock surface per output. Contract frozen in
ADR [0067](design/adr/0067-session-lock-protocol-and-ui.md).

What landed:

- **`compositor/src/lock.rs`** (new) — `LockModel`: the one `locked` flag (with
  the pre-lock focused window for restore), `surfaces: HashMap<output,
  LockSurface>`, `lock/unlock/insert_surface/surface/retain_live/
  surface_count/covers`, `covered_by`, `lock_render_elements` (renders the live
  lock surface at the output origin), `lock_surface_size`. `LockSurface` has an
  inherent `alive()`, not `IsAlive`; `alive` on `Window` needs the `IsAlive`
  trait in scope.
- **`compositor/src/state.rs`** — `DfState.lock: LockModel`; `SessionLockHandler`
  enters the lock and clears keyboard focus *before* confirming, configures each
  lock surface to `lock_surface_size`, logs coverage, restores focus on unlock.
  `delegate_session_lock!` unchanged.
- **`compositor/src/input.rs`** — `process_input_event` returns before routing
  any user input (device add/remove still flows) while locked.
  `compositor/src/shell/mod.rs` — `activate_window_id` refuses while locked and
  `sync_outputs` calls `lock.retain_live`; `state.rs` `request_activation`
  refuses while locked.
- **`compositor/src/backend/nested.rs` / `drm.rs`** — `lock_render_elements`
  prepended to the front-to-back custom-element list (above cursor/windows/
  chrome); headless renders nothing.
- **`shell/src/shellprotocol.{h,cpp}`** — binds
  `ext_session_lock_manager_v1` + `wl_output`; `lockSession()` creates a lock
  surface per output, `commitLockImage(id, image)` paints one, `unlockSession()`
  calls `unlock_and_destroy`; signals `sessionLocked` / `sessionFinished` /
  `lockSurfaceConfigured`; `isSessionLocked()` (the signal is `sessionLocked()`
  — do not name a method the same). Lock surface id is the opaque
  `ext_session_lock_surface_v1*` as `quintptr`.
- **`shell/src/shellcontroller.{h,cpp}`** — offscreen `Dragonfruit.Lock` scene,
  `showLockScreen/applyLockData/renderLockSurface`, `lock-screen` input action
  handler, `DF_LOCK_FIXTURE` / `DF_LOCK_UNLOCK_MS` capture seams.
- **`shell/lock/LockScreen.qml`** (new) + `shell/lock/CMakeLists.txt`; module
  linked as `dragonfruit-shell-lockplugin`.
- **`protocols/wayland-protocols/ext-session-lock-v1.xml`** (new, vendored MIT);
  `shell/src/CMakeLists.txt` runs `wayland-scanner` over it and adds the C to the
  shell target.
- **Tests** — `compositor/tests/session_lock_conformance.rs` (compositor
  headless: `locked` event, full-output configure per output, clean unlock +
  re-lock); `shell/tests/tst_lock.{cpp,qml}` (lock view + a11y). `Makefile`
  `e2e` runs `session_lock_conformance`.

Commands that work (repo root):

- `cargo test -p dragonfruit-compositor --test session_lock_conformance` — 1/1.
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — 279/279
  (includes `lock::tests`).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings`; `cargo
  fmt -p dragonfruit-compositor -- --check` — exit 0.
- `make lint` — exit 0 (`qml-test` 40/40, includes `tst_lock` and
  `qmllint_shell-lock`); `make e2e` — exit 0.

Live check: `DF_LOCK_FIXTURE=3000 ./target/debug/dragonfruit dev --demo --nested
--socket-name t81-lock`, captured at `/tmp/opencode/t81-lock.png`; vision
confirms the lock screen covers the nested output — large clock, date, avatar
(initial of `$USER`), name, rounded password field ("Press Enter to unlock"),
drawn lock glyph — no artifacts, no menu bar/Dock visible.

Gotchas for later tasks:

- **All user input is dropped while locked.** The lock surface is not yet
  reachable; T-12.3c routes input to it. Do not assume the shell can
  click/type on the lock screen yet.
- **Unlock is protocol-only.** The shell exposes `ShellProtocol::unlockSession`
  (`ext_session_lock_v1.unlock_and_destroy`); T-12.3b calls it from PAM success.
  There is no user-facing unlock.
- **The manager filter is open** (`SessionLockManagerState::new(..., |_| true)`,
  TODO(T-26) in `state.rs`). T-12.3c restricts it to the trusted shell.
- **The shell's lock object is left alive while locked** (destroy is a protocol
  error while locked); a shell crash therefore keeps the session locked. The
  compositor crash path ends the session.
- **Signals and methods cannot share a name** in `ShellProtocol` (moc/Qt
  overload resolution): the getter is `isSessionLocked()`, the signal
  `sessionLocked()`.
- **`DF_LOCK_FIXTURE` / `DF_LOCK_UNLOCK_MS` are capture-only** and never set in
  a real session; the real trigger is Cmd+Ctrl+Q → `InputAction::LockScreen` →
  `df_toplevel_manager.input_action`.

## T82 — T-12.3b Lock PAM authentication

**State: done.** Unlock is now real PAM authentication through a small helper;
the shell keeps no credential store and reads back only the helper's exit
status. Contract frozen in ADR
[0068](design/adr/0068-lock-pam-helper.md).

What landed:

- **`services/lock-auth/`** (new crate `dragonfruit-lock-auth`) —
  `PamAuthenticator` (`new`, `with_service`, `with_confdir`, `service`,
  `confdir`), the `Authenticator` trait, and `AuthResult`
  (`Success`/`Denied`/`Error`, `exit_code`, `from_exit_code`). libpam is
  `dlopen`ed (`libpam.so.0` → `libpam.so`) and only
  `pam_start_confdir`/`pam_authenticate`/`pam_acct_mgmt`/`pam_end` are called;
  the PAM ABI is declared locally, so no `pam-devel` is needed. Default service
  `dragonfruit`, fallback `login`; `--confdir` is the `pam_start_confdir` test
  seam.
- **`services/lock-auth/src/main.rs`** — `dragonfruit-pam-helper --user USER
  [--service S] [--confdir DIR]`: password on stdin (one line; terminal echo
  suppressed), exit `0` authenticated / `1` rejected / `2` unavailable. Never
  prints, logs, or writes the password.
- **`shell/src/lockauth.{h,cpp}`** (new, in the Wayland-free dockcore) —
  `LockAuthenticator`: `resolveHelperPath` (`DF_PAM_HELPER` → `PATH` →
  shell-sibling / `target/{debug,release}`), async `QProcess`,
  `authenticate(user, password)` (clears its own copy), `succeeded()` /
  `failed(message)`, `cancel()`, one run at a time.
- **`shell/src/shellcontroller.{h,cpp}`** — creates `m_lockAuth` (service from
  `DF_PAM_SERVICE`); `showLockScreen` sets `authEnabled=true`; new slot
  `submitLockPassword` (T-12.3c calls it), `onLockAuthSucceeded` (calls
  `ShellProtocol::unlockSession` then tears down the lock scene),
  `onLockAuthFailed`, `repaintLockSurfaces`, `teardownLockScreen`,
  `lockUserName`.
- **`tools/dragonfruit-dev/src/main.rs`** — `pam_helper_path()` and
  `DF_PAM_HELPER` passed to the shell.
- **Tests** — `services/lock-auth/tests/helper.rs` 7 (permit success, deny
  rejection, `login` fallback, missing-service error, missing `--user`,
  no-leak/no-disk-write, library seam); `src/lib.rs` 4 unit (exit codes,
  status classification, defaults, conversation returns the exact password
  including spaces); `shell/tests/tst_lockauth.cpp` 6 (`DF_PAM_HELPER`
  override, success/denied/unavailable mapping, busy guard, missing helper).
- **Docs** — ADR 0068; `11-session-and-dev-workflow.md` auth paragraph;
  `testing-ladder.md` rung 1 bullet.

Commands that work (repo root):

- `cargo test -p dragonfruit-lock-auth` — 11/11 (4 unit + 7 integration).
- `cargo clippy -p dragonfruit-lock-auth --all-targets -- -D warnings` and
  `cargo fmt -p dragonfruit-lock-auth -- --check` — exit 0.
- `make qml-test` — 41/41 (adds `tst_lockauth`).
- `make e2e` — passed (adds `cargo test -p dragonfruit-lock-auth`).
- `make clippy` — exit 0.

Live check: `DF_LOCK_FIXTURE=3000 ./target/debug/dragonfruit dev --demo
--nested --socket-name t82-lock`, captured `/tmp/opencode/t82-lock.png`; vision
confirms the lock scene (clock 17:25, date, magenta avatar "U", username
`user`, password field placeholder "Enter Password", lock glyph, no menu
bar/Dock, no artifacts) and the dev tool reported a clean teardown.

Gotchas for later tasks:

- **Input capture is still T-12.3c.** The lock surface is not reachable;
  `ShellController::submitLockPassword(password)` is the slot T-12.3c must call
  once it routes keys to the lock UI. The lock scene has no `TextInput` yet.
- **PAM service selection**: helper default `dragonfruit`, falling back to
  `login` when the service is absent. `DF_PAM_SERVICE` (shell) / `--service`
  (helper) override. Packaging installs `dragonfruit-pam-helper` on `PATH` and
  may install `/etc/pam.d/dragonfruit`.
- **`--confdir` is the only test seam** (`pam_start_confdir`); the headless
  suite never touches `/etc/pam.d` and needs no root.
- **The helper path is `DF_PAM_HELPER` first**; the dev tool sets it to the
  cargo sibling. A missing helper (or libpam) is exit `2` and keeps the session
  locked — never a false unlock.
- **Only the PAM auth+account phases run**; the session phase (keyring,
  systemd, selinux) does not run for a lock unlock.

## System font — Inter (post-T82, before T83)

**State: done (first-party half).** The desktop's type is now Inter 4.001
(SIL OFL 1.1), bundled under `fonts/Inter/` and declared as the
`primitive.font.family` token, replacing the host Qt/fontconfig default every
first-party surface silently inherited. No roadmap unit covered the swap, so
it landed out-of-band between T82 and T83; the remaining units and the T-17
visual floor therefore bake in the final font instead of needing a later
re-capture sweep.

What landed:

- **`fonts/Inter/`** (now tracked; T82's `/fonts/` .gitignore entry is gone) —
  the roman and italic `opsz,wght` variable faces plus a README with
  provenance.
- **`libs/system-font/`** (new static library) — `Dragonfruit::installSystemFont()`
  registers both faces from Qt resources (`Q_INIT_RESOURCE`, needed because a
  static library's resource object is not linked unless referenced) and swaps
  only the family on `QGuiApplication::font()`; idempotent, and it warns and
  keeps the host default when a face is missing.
- **Entry points** — `shell/src/main.cpp`, `apps/settings/main.cpp`,
  `apps/files/main.cpp`, and `design-system/gallery/main.cpp` call it after
  `QGuiApplication` exists and before QML loads. The gallery matters because
  it renders the art-direction goldens.
- **`design-system/tokens/tokens.json`** — `primitive.font.family: "Inter"`;
  `Theme.qml` and `compositor/src/design_tokens.rs` regenerated (the
  compositor draws no text; the Rust constant is for later SSD work).
- **Tests** — `libs/system-font/tests/tst_systemfont.cpp` 7/7: the family
  registers, the app default is Inter, weight/italic variants resolve without
  falling back, the QML `Application.font` is Inter, and repeat calls are
  no-ops.
- **Docs** — `10-design-system.md` typography section; `NOTICE` and
  `licensing.md` OFL entries; packaging-track scope note.

Commands that work (repo root):

- `make qml-test` — 42/42 (adds `tst_systemfont`).
- `make visual-test` and `./scripts/check-gallery-snapshots.py --strict` —
  72/72; `make gallery-snapshot` regenerated all 72 goldens, including the six
  `slider_*`/`select_*` goldens that had been missing since T-50/T-51.

Live check: `DF_STATUS_FIXTURE=1 make demo DEMO_ARGS="--socket-name
dragonfruit-font-check"`, captured `/tmp/opencode/font-check.png`; vision
confirms the menu bar, Settings window, and Dock render in Inter with no
clipping, and the dev tool reported a clean teardown.

Gotchas for later tasks:

- **Third-party apps do not get Inter yet.** Only first-party Qt processes
  install it in-process. GTK/Flatpak/XWayland apps still follow the host
  fontconfig; T-16.9/T-16.10 must install `fonts/Inter/` system-wide with a
  `sans-serif` alias (noted in the T-16 track) and ship
  `LICENSES/OFL-1.1.txt` for OFL condition 2.
- **QML component tests intentionally use the host font.** They are designed
  to be font-rasterization-independent; only the gallery app (and its
  goldens) installs Inter. A test that needs the desktop font can link
  `dragonfruit-system-font` and call `installSystemFont()`.
- **The family name lives in two places** by construction:
  `primitive.font.family` (QML/Rust tokens) and `kFontFamily` in
  `libs/system-font/systemfont.cpp`. Change both together.
- **New first-party processes must opt in**: link `dragonfruit-system-font`
  and call `installSystemFont()` in `main` before QML loads. The screenshot
  UI, when T-13.3a gives it a process of its own, is the first such case.
- **Goldens no longer follow the host's installed font** — all 72 use the
  bundled Inter faces. Host hinting/rasterization can still differ, so the
  default gate stays non-strict; a strict cross-host diff is not promised.

## T83 — T-12.3c Lock input capture and kill-resistance

**State: done.** The locked session now captures input in the lock UI instead
of dropping all of it, and it stays locked if the lock UI dies. Contract frozen
in ADR [0069](design/adr/0069-locked-input-capture-and-kill-resistance.md).

What landed:

- **`compositor/src/lock.rs`** — `LockModel::input_surface()` (first live lock
  surface) and `is_lock_surface()`; `retain_live` still never clears `locked`.
- **`compositor/src/input.rs`** — while locked, `route_locked_keyboard` focuses
  the lock surface and forwards keys to it with no shortcut filter; pointer,
  touch, and gestures stay dropped. With no live lock surface, even keys are
  dropped.
- **`compositor/src/state.rs`** — `SessionLockHandler::new_surface` focuses the
  lock surface; `unlock` always resets focus. `lock` refuses untrusted clients:
  it checks the T-07 launch-token trust model and drops the `SessionLocker`
  (`finished`, no lock) for anyone but the shell.
- **`compositor/src/input/synthetic.rs`** — `query lock` reports
  `lock locked=<0|1> surfaces=<n> lock-focus=<0|1>`; read-only.
- **`shell/src/shellprotocol.{h,cpp}`** — `lockKeyEvent(uint32_t key, bool
  pressed, bool shift)`; `onKeyboardEnter`/`Leave` set `m_keyboardOnLock` via
  `isLockSurface`, `onKeyboardKey` routes lock keys away from the chrome, and
  `onKeyboardModifiers` tracks Shift.
- **`shell/src/shellcontroller.{h,cpp}`** — `onLockKeyEvent` fills a private
  `m_lockPassword` (Escape clears, Backspace chops, Return submits through the
  existing `submitLockPassword`, characters append); only `passwordLength` is
  exposed to the scene. Password cleared on show/teardown/failure/success.
- **`shell/src/lockinput.{h,cpp}`** (new, dockcore) — pure US/ASCII
  `lockKeyFromEvdev(key, shift)`.
- **`shell/lock/LockScreen.qml`** — `passwordLength` + a bullet mask
  (`lockPasswordMask`) that replaces the placeholder while typing.
- **Tests** — `compositor/tests/session_lock_conformance.rs` 3/3 (output
  coverage; untrusted client cannot lock; locked keys reach the lock surface and
  `query lock` still reports `locked=1` after the lock client's socket is
  closed); `lock::tests` 4/4; `shell/tests/tst_lockinput.cpp` 4/4;
  `tst_lock.qml` mask test.
- **Docs** — ADR 0069; `11-session-and-dev-workflow.md` lock section.

Commands that work (repo root):

- `cargo test -p dragonfruit-compositor --test session_lock_conformance` — 3/3.
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — 279/279.
- `make qml-test` — 43/43 (adds `tst_lockinput`).
- `make e2e`, `make lint` — exit 0.

Live check: `DF_LOCK_FIXTURE=3000 DRAGONFRUIT_SYNTHETIC_INPUT=/tmp/opencode/
t83-synth.sock ./target/debug/dragonfruit dev --demo --nested --socket-name
t83-lock`, with `key 30/48/46` (a/b/c) injected through the synthetic harness,
captured `/tmp/opencode/t83-lock.png`; vision confirms the nested lock window is
opaque with the clock/date/avatar/user/lock glyph and exactly three password
bullets (no placeholder), then the dev tool reported a clean teardown.

Gotchas for later tasks:

- **Input capture route is keyboard-only.** Pointer/touch/gesture are swallowed,
  not delivered to the lock surface; the lock UI is keyboard-driven.
- **Untrusted lock requests are refused** in `SessionLockHandler::lock`, but the
  `ext_session_lock_manager_v1` global is still *advertised* to everyone (the
  Smithay global filter is `|_| true`); the request gate, not the global, is the
  boundary. Moving to the `can_view` filter needs the trust check at global-bind
  time.
- **The US/ASCII mapping and the no-dead-lock-UI-recovery limitation** are
  recorded under `## Follow-ups`.
- **The shell's lock scene is a pure view**: it never holds the password; the
  buffer lives in `ShellController` and goes straight to the PAM helper stdin.

## T84 — T-12.4a Idle timers

**State: done.** The dim → blank → lock → suspend chain is a pure,
clock-injected engine in `dragonfruit-session`, driven from `idle.*` policy
keys. Contract frozen in ADR
[0070](design/adr/0070-idle-timer-engine-and-policy.md).

What landed:

- **`services/session/src/idle.rs`** (new) — `IdleStage`
  (`Active`/`Dim`/`Blank`/`Lock`/`Suspend`, ordered, `as_str`, `is_idle`,
  `next`); `IdlePolicy` (`new` defaults dim 150 s / blank 300 s / lock 600 s /
  no suspend, `disabled`, `delay`, `set_delay`, `with_delay`, `normalized`,
  `from_keys`); `KEY_DIM`/`KEY_BLANK`/`KEY_LOCK`/`KEY_SUSPEND`,
  `DEFAULT_*`; `IdleTimers` (`new`, `policy`, `stage`, `last_activity`,
  `idle_for`, `activity`, `stage_at`, `poll`, `next_deadline`, `set_policy`).
- **`services/session/src/lib.rs`** — exports `idle`, re-exports
  `IdlePolicy`/`IdleStage`/`IdleTimers`.
- **Tests** — `idle::tests` 10 unit (each stage at its delay, late-tick jump,
  activity reset/wake, disabled stages, disabled policy, out-of-order clamping,
  `next_deadline`, live policy change, `from_keys`, key/name round-trip);
  `services/session/tests/idle.rs` 3 integration (fake clock drives every
  stage; wake resets; a zero key disables its stage).
- **Docs** — ADR 0070; `11-session-and-dev-workflow.md` new "Idle timers
  (T-12.4a)" subsection and the logind bullet.

Commands that work (repo root):

- `cargo test -p dragonfruit-session` — 23 unit + 3 integration.
- `cargo clippy -p dragonfruit-session --all-targets -- -D warnings`,
  `cargo fmt -p dragonfruit-session -- --check` — exit 0.
- `make e2e` — passed; `make lint` — 43/43 QML tests, exit 0.

Live check: `make demo DEMO_ARGS="--socket-name t84-idle"` (nested), captured
`/tmp/opencode/t84-idle.png`; vision confirms the nested desktop renders
(menu bar `dragonfruit-settings File Edit View`, Dock tiles, Settings and the
X11 demo window) with no artifacts, and the dev tool reported a clean teardown.

Gotchas for later tasks:

- **No production consumer yet.** Nothing calls `IdleTimers`; the compositor
  still has no dim/blank path and no idle service binds `ext-idle-notify`.
  T-12.4b is the first consumer (a held inhibitor simply skips `poll`; wake
  calls `activity`).
- **Key names are fixed by ADR 0070**: `idle.dim`, `idle.blank`, `idle.lock`,
  `idle.suspend`, whole seconds, `0`/negative/`never` disables. T-12.5b must
  register exactly these in settingsd and call `IdlePolicy::from_keys`.
- **`normalized()` clamps enabled delays into chain order** (a stage can never
  fire before an earlier enabled stage); disabled stages are skipped.
- The engine is in `dragonfruit-session`, not the compositor (ADR 0070); the
  compositor keeps serving `ext-idle-notify`/`idle-inhibit` for the future
  idle service.

## T85 — T-12.4b Idle inhibitors and wake restore

**State: done.** `dragonfruit-session` now wraps the T-12.4a idle engine with
inhibitors and wake restore. Contract frozen in ADR
[0071](design/adr/0071-idle-inhibitors-and-wake-restore.md).

What landed:

- **`services/session/src/idle.rs`** — `IdleController` (owns `IdleTimers` +
  `IdleInhibitors`, returns `IdleEvent`), `IdleInhibitors` (`new`, `acquire`,
  `release`, `is_inhibited`, `count`, `contains`, `clear`, `iter`),
  `InhibitorId`, `IdleEvent` (`Enter(stage)` / `Restore(stage)`, `stage`).
  Controller: `new`, `timers`, `policy`, `stage`, `last_activity`, `idle_for`,
  `inhibitors`, `is_inhibited`, `inhibitor_count`, `acquire_inhibitor`,
  `release_inhibitor`, `clear_inhibitors`, `activity`, `poll`, `next_deadline`,
  `set_policy`. Added `IdleTimers::replace_policy` (store without evaluating).
- **Semantics** — while any inhibitor is held, `poll` is a no-op and
  `next_deadline()` is `None`; acquiring an inhibitor forces the chain to
  `Active` and reports `Restore(prior)`; `activity` wakes and reports
  `Restore(prior)`; releasing an inhibitor does not move the chain or reset the
  clock, so an overdue chain catches up on the next poll; waking from `Lock`
  reports `Restore(Lock)` but never unlocks.
- **`services/session/src/lib.rs`** — re-exports `IdleController`, `IdleEvent`,
  `IdleInhibitors`, `InhibitorId`.
- **Tests** — `idle::tests` 6 new unit; new
  `services/session/tests/idle_inhibitors.rs` 4 integration.

Commands that work (repo root):

- `cargo test -p dragonfruit-session` — 29 unit + 4 `idle_inhibitors` (plus
  the existing suites).
- `cargo clippy -p dragonfruit-session --all-targets -- -D warnings`,
  `cargo fmt -p dragonfruit-session -- --check` — exit 0.
- `make e2e`, `make lint` — exit 0.

Live check: `make demo DEMO_ARGS="--socket-name t85-idle"` (nested), captured
`/tmp/opencode/t85-idle.png`; desktop renders with no artifacts, clean
teardown. No UI changed, so this is a regression check.

Gotchas for later tasks:

- **No production consumer yet.** Nothing constructs `IdleController`; no idle
  service binds `ext-idle-notify`. T-12.5a drives suspend from
  `Enter(Suspend)` and resume/restore from `Restore(Suspend)`; T-12.5b calls
  `set_policy(IdlePolicy::from_keys(...))`.
- **`InhibitorId` is registry-scoped**; across an idle-service restart create a
  new controller or `clear_inhibitors` instead of reusing a handle.
- **`Restore(Lock)` is not an unlock** — the lock UI/PAM path owns unlocking.
- The compositor's `idle-inhibit` state is protocol-level; the idle service is
  the adapter that maps surfaces to `InhibitorId` handles.

## T86 — T-12.5a Suspend/resume cycle

**State: done.** One suspend/resume round trip recovers outputs, input, and
clients without a restart. Contract frozen in ADR
[0072](design/adr/0072-suspend-resume-cycle.md).

What landed:

- **`services/session/src/suspend.rs`** (new) — `SuspendState`
  (`Awake`/`Requested`/`Asleep`), `SuspendRequest` (`Suspend`/`Cancel`),
  `SuspendBackend` trait, `SuspendCycle` (`new`, `state`, `is_awake`,
  `is_requested`, `is_asleep`, `cycles`, `request`, `prepare_for_sleep`,
  `activity`), `SuspendController<B>` (`new`, `cycle`, `state`, `is_awake`,
  `is_asleep`, `cycles`, `backend`, `last_error`, `on_idle_event`,
  `prepare_for_sleep`, `activity`), and `MockSuspend`. `on_idle_event` maps
  `Enter(Suspend)` → request and `Restore(Suspend)` → cancel; every other
  stage is ignored. `prepare_for_sleep(true)` also accepts `Awake` (external
  lid/power suspend); waking counts one cycle.
- **`compositor/src/suspend.rs`** (new) — pure `SuspendModel`
  (`is_suspended`, `cycles`, `suspend`, `resume`, idempotent).
- **`compositor/src/state.rs`** — `DfState.suspend`; `suspend_session`
  (disarm animation timer, close cadence, drop redraw) and `resume_session`
  (repaint every output, `notify_activity`, re-arm animations).
- **`compositor/src/input.rs`** — user input is dropped while suspended
  (device hotplug still routes); `RenderStats.input_events` counts input that
  reached the router, exposed as `received=` in `query latency`.
- **`compositor/src/session.rs` / `backend/drm.rs`** — the shared render hook
  and DRM `render_surface` skip frames while suspended; DRM
  `PauseSession`/`ActivateSession` drive the model on a VT switch.
- **`compositor/src/input/synthetic.rs`** — `suspend`, `resume`, and
  `query session` (`session suspended=<0|1> cycles=<n> outputs=<n>
  windows=<n>`).
- **Tests** — `suspend::tests` 8 unit + `services/session/tests/suspend.rs` 3
  integration (a real `sleep` child survives the cycle with zero restarts);
  new `compositor/tests/suspend_resume_conformance.rs` 1 integration (mapped
  client, dropped-then-restored input, pending frame callback delivered on
  resume, two cycles). Added to `make e2e`.
- **Docs** — ADR 0072; `11-session-and-dev-workflow.md` "Suspend/resume
  (T-12.5a)" subsection and the logind bullet.

Commands that work (repo root):

- `cargo test -p dragonfruit-session` — 29 unit + 3 `suspend` (plus existing).
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` —
  280/280.
- `cargo test -p dragonfruit-compositor --test suspend_resume_conformance` —
  1/1.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings`,
  `cargo clippy -p dragonfruit-session --all-targets -- -D warnings`,
  `cargo fmt -p ... -- --check` — exit 0.
- `make e2e`, `make lint` — exit 0.

Live check: nested demo driven over the synthetic socket
(`/run/user/1000/t86-suspend-live.synth`): `query session` went
`suspended=0 cycles=0` → `suspended=1 cycles=0` → `suspended=0 cycles=1` →
`cycles=2`, always `outputs=1 windows=1`; captures
`/tmp/opencode/t86/t86-suspend-{awake,suspended,resumed}.png` (3840x2160),
byte-identical across the cycle (suspend freezes frames, resume restores the
same scene — the intended no-visual-change outcome), and the dev tool
reported a clean teardown. The vision tool was rate-limited (HTTP 429), so the
stills were checked with PIL (not black, ~46k distinct colors, same
distribution as the known-good `t84/t85-idle.png`).

Gotchas for later tasks:

- **The concrete logind backend is not wired.** `SuspendBackend` has only the
  CI `MockSuspend`; no session process constructs `SuspendController` yet. The
  real logind `Manager.Suspend` client and `PrepareForSleep` forwarding to the
  compositor are the T-12.6/hardware step. Until then `suspend`/`resume` are
  driven by the synthetic harness (and DRM VT pause/activate).
- **Suspending is not locking.** A suspend while locked stays locked; the
  lock UI/PAM path still owns unlocking (the controller never touches
  `LockModel`).
- **`resume_session` is the only repaint source.** A real sleep wake must call
  it (or the DRM `ActivateSession` path) or the screen stays on its last
  frame; the compositor does not poll for sleep.
- **`query latency` gained a `received=` field before `within_one_frame_60hz`**
  (ADR 0009 append-only ordering is respected for the `dump_stats` line, not
  this query report).

## T87 — T-12.5b Session policy keys, kill matrix, capture

**State: done.** The session policy keys are registered in settingsd, the
kill matrix is a headless suite, and the T-12 track capture is committed.
Contract frozen in ADR
[0073](design/adr/0073-session-policy-keys-and-kill-matrix.md).

What landed:

- **`services/settingsd/src/schema.rs`** — schema v5, new `Session` key group:
  `idle.dim` (150), `idle.blank` (300), `idle.lock` (600), `idle.suspend`
  (0), all `KeyType::Integer` (`x`) whole seconds, range 0–86400, `0`
  disables. Exactly the names/spellings ADR 0070 freezes. Owner
  `apps/settings`, consumer `session/idle engine`.
- **`docs/settings-keys.md`** — four rows + `session/idle engine` consumer
  map entry (gated by `tests/schema_doc.rs`).
- **`libs/settings-client/settingsclient.cpp`** — `settingsSchemaDefaults()`
  mirrors v5 (settingsd-absent path).
- **`services/session/tests/session_policy.rs`** (new, 5 tests) — the keys
  drive the chain headlessly: defaults reproduce `IdlePolicy::new()`; a
  snapshot walks dim/blank/lock/suspend on a fake clock and round-trips
  `SuspendController`; `0` disables one stage; a live key change moves the
  chain; a held inhibitor freezes it.
- **`services/session/tests/kill_matrix.rs`** (new, 5 tests) — shell/lock UI
  restarts (`always`); settingsd and notifications restart (`on-failure`);
  compositor death ends the session and stops the rest, never restarted. The
  fail-secure "never unlocked" half stays with the compositor
  (`session_lock_conformance.rs`, already in `make e2e`).
- **Capture** — `scripts/capture-session.sh` + `capture-session-driver.py` +
  `make session-capture` write `docs/captures/t12-session.png`,
  `t12-session-lock.png`, `t12-session.mp4`, `t12-session.txt`,
  `t12-session-kill-matrix.txt`.
- **Docs** — ADR 0073; `11-session-and-dev-workflow.md` "Session policy keys
  and the kill matrix (T-12.5b)"; `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-settingsd` — 27 unit + restart/schema_doc/bus.
- `cargo test -p dragonfruit-session` — 37 unit + `kill_matrix` 5 +
  `session_policy` 5 (plus existing).
- `cargo clippy -p dragonfruit-settingsd -p dragonfruit-session
  --all-targets -- -D warnings`, `cargo fmt ... -- --check` — exit 0.
- `make e2e`, `make lint` — exit 0.
- `make session-capture` — writes the artifacts above.

Live check: nested demo with the synthetic harness; `query lock` went
`locked=0 surfaces=0 lock-focus=0` → `locked=1 surfaces=1 lock-focus=1` after
the real Cmd+Ctrl+Q chord; `query session` stayed `suspended=0 outputs=1
windows=2`. Vision confirms the desktop still (menu bar, Settings Appearance
pane, X11 demo window, Dock) and the lock card (clock, date, avatar, password
field), no artifacts. Stills are 1920x1200 crops at (960,467).

Gotchas for later tasks:

- **No production idle service yet.** The keys are registered and
  `IdlePolicy::from_keys` is proven, but nothing binds `ext-idle-notify` or
  calls `set_policy` at runtime; the real consumer is T-12.6/portals.
- **Idle keys are `x` int64 whole seconds, 0–86400**, `0` disables. Convert
  with `to_string()` into `IdlePolicy::from_keys`; "Never" is `0`, not the
  `never` parser leniency.
- **`SCHEMA_VERSION` is now 5**; older settings files migrate by filling the
  four defaults.
- The lock UI death/lock invariant is the compositor's
  (`session_lock_conformance.rs`); the session kill matrix asserts supervision
  only. T-16.8a/T-17.5a add the VM/DRM no-live-under-lock drills.

## T88 — T-13.1a Portal backend and session service

**State: done.** `portal/` is now a real session-bus portal backend plus its
registration, and the frontend-absent path is proven headlessly. Contract
frozen in ADR
[0074](design/adr/0074-portal-backend-registration-and-frontend-degradation.md).

What landed:

- **`portal/src/`** (new `[lib]` + existing binary) — `model` (pure:
  `BackendStatus`, `FrontendPresence` `Unknown`/`Absent`/`Present{owner}`,
  `FrontendTracker`, `BACKEND_INTERFACES`), `data` (the three discoverability
  files + parsers + `install_into`), `dbus` (`serve(address)`/`run()`,
  `probe_frontend`, `name_has_owner`/`name_owner`, the `NameOwnerChanged`
  watch).
- **Identity** — owns `org.freedesktop.impl.portal.desktop.dragonfruit`,
  serves `/org/freedesktop/portal/desktop`; backend name in portals.conf is
  `dragonfruit`. Diagnostic interface `org.dragonfruit.Portal1` at the same
  path reports identity, interfaces, and frontend presence
  (`BackendName`/`DbusName`/`ObjectPath`/`Version`/`Interfaces`/`Frontend`/
  `FrontendPresent`/`FrontendOwner`, signal `FrontendChanged`).
- **`portal/data/`** (new) — `dragonfruit.portal` (empty `Interfaces=`
  until T-13.1b), `dragonfruit-portals.conf` (`[preferred] default=
  dragonfruit;gtk`), `org.freedesktop.impl.portal.desktop.dragonfruit.service`
  (D-Bus activation). Installed under `share/...` by `install_into(prefix)`.
- **Degradation** — the `xdg-desktop-portal` frontend is only *observed*:
  one `NameHasOwner` probe at startup plus a live `NameOwnerChanged` watch.
  Absent/unknown never blocks startup; the backend keeps serving.
- **Binary flags** — `--print-interfaces`, `--print-identity`,
  `--check-frontend` (exit 0 even with no bus), `--install-data PREFIX`.
- **Tests** — 11 unit (model/data/dbus) + `portal/tests/session_bus.rs` 4
  integration on a private `dbus-daemon`: name registration + introspection,
  absent frontend, present frontend at startup, live appear/disappear via the
  `FrontendChanged` signal. Added to `make e2e`.
- **Docs** — ADR 0074; `07-system-integration.md` "The backend service and
  its registration (T-13.1a)".

Commands that work (repo root):

- `cargo test -p xdg-desktop-portal-dragonfruit` — 11 unit + 4 `session_bus`.
- `cargo clippy -p xdg-desktop-portal-dragonfruit --all-targets -- -D warnings`,
  `cargo fmt -p xdg-desktop-portal-dragonfruit -- --check` — exit 0.
- `make e2e`, `make lint` — exit 0.

Live check: nested demo (`make demo`) renders the desktop with no artifacts;
T-13.1a adds no surface, so this is a regression check.

Gotchas for later tasks:

- **`BACKEND_INTERFACES` is `[]` and `dragonfruit.portal` has an empty
  `Interfaces=`.** T-13.1b (Settings, GlobalShortcuts) and later tasks append
  their standard `org.freedesktop.impl.portal.*` names to both; the data
  test asserts the descriptor equals the model.
- **The backend name/path/data-file identities are frozen**; only the
  interface list is additive.
- **Production data-file install is packaging's job (T-16)**; nothing in the
  session yet calls `install_into`.
- **`org.dragonfruit.Portal1` is diagnostic only** — the portal frontend
  never calls it; `df_ipc::is_valid_dbus_name` applies to it but not to the
  standard `org.freedesktop.impl.portal.*` interfaces.

## T89 — T-13.1b Settings and GlobalShortcuts portals

**State: done.** The backend now serves the first two standard interfaces at
`/org/freedesktop/portal/desktop`, advertises them in `dragonfruit.portal` /
`model::BACKEND_INTERFACES`, and answers real D-Bus clients. Contract frozen in
ADR [0075](design/adr/0075-settings-and-globalshortcuts-portals.md).

What landed:

- **`portal/src/settings.rs`** (new) — the pure projection:
  `SettingsSnapshot` (namespace → key → `OwnedValue`, `new`,
  `from_entries`, `read`, `read_all`, `namespaces`, `desktop_key`, `diff`,
  `refresh_appearance`), `SettingsStore` (`store`, `lock`), `fetch`, `sync`,
  `spawn_settings_sync` (+ name watch + `Changed` loop). Namespaces
  `org.freedesktop.appearance` (`color-scheme` from
  `appearance.colorScheme`: auto=0/dark=1/light=2; `contrast`=0;
  `accent-color` `a(ddd)` only when `appearance.accent` is `#rrggbb`) and
  `org.dragonfruit.desktop` (every settingsd key, read-only).
- **`portal/src/shortcuts.rs`** (new) — `PortalShortcut`,
  `ShortcutSession`, `SessionError` (`Duplicate`/`Unknown`),
  `ShortcutRegistry` (`create`, `bind`, `list`, `close`, `session`,
  `contains`, `has_shortcut`, `len`, `is_empty`), `SharedRegistry`
  (`registry`, `lock`). Constants `GLOBAL_SHORTCUTS_INTERFACE`,
  `SESSION_INTERFACE`, versions, response/close codes.
- **`portal/src/interfaces.rs`** (new) — the zbus objects `SettingsPortal`
  (`Read`, `ReadAll`, `Version`, `SettingChanged`) and `GlobalShortcuts`
  (`CreateSession`, `BindShortcuts`, `ListShortcuts`, `Version`,
  `Activated`/`Deactivated`/`ShortcutsChanged`) plus
  `ShortcutSessionObject` (`Close`, `Closed`). `SETTINGS_INTERFACE`.
- **`portal/src/dbus.rs`** — `initialize` serves `SettingsPortal` and
  `GlobalShortcuts` at `DBUS_PATH` and spawns the settings sync;
  `Backend` gained the shared `shortcuts` registry, `with_shortcuts`,
  `shortcuts()`, and the diagnostic `ActivateShortcut`/`DeactivateShortcut`
  which emit the standard signals.
- **`portal/src/model.rs` / `portal/data/dragonfruit.portal`** —
  `BACKEND_INTERFACES` is now
  `["org.freedesktop.impl.portal.Settings", "org.freedesktop.impl.portal.GlobalShortcuts"]`;
  the descriptor `Interfaces=` matches (data test enforces).
- **Tests** — `portal/tests/portals.rs` (new, 3 integration on a private
  bus: standard interfaces introspect; Settings answers appearance/desktop
  reads, follows a real settingsd `Set` live and emits `SettingChanged`;
  GlobalShortcuts create/bind/list/close with `ShortcutsChanged`,
  `Activated`, and `Closed`). Unit tests added in `settings`/`shortcuts`/
  `interfaces`; `session_bus.rs` now expects the two advertised interfaces.
  Dev-dependency `dragonfruit-settingsd` (test-only) serves a real settingsd
  object beside the backend.
- **Docs** — ADR 0075; `07-system-integration.md` "Settings and
  GlobalShortcuts (T-13.1b)".

Commands that work (repo root):

- `cargo test -p xdg-desktop-portal-dragonfruit` — 23 unit + 4 `session_bus`
  + 3 `portals`.
- `cargo clippy -p xdg-desktop-portal-dragonfruit --all-targets -- -D
  warnings`, `cargo fmt -p xdg-desktop-portal-dragonfruit -- --check` — exit 0.
- `make lint`, `make e2e` — exit 0.

Live check: nested demo over the synthetic socket rendered the desktop,
menu bar, Dock, the Settings Appearance pane, and the X11 demo window with no
artifacts (capture `/tmp/opencode/t89/t89-desktop.png`, vision-checked). This
task adds no surface, so the check is a regression pass.

Gotchas for later tasks:

- **The compositor is not wired to GlobalShortcuts.** Nothing feeds bound
  shortcuts into `ShortcutEngine`; the diagnostic `ActivateShortcut` /
  `DeactivateShortcut` on `org.dragonfruit.Portal1` is the only emitter of
  `Activated`/`Deactivated`. T-13.7 (or the shell bridge) drives it from the
  engine.
- **The Settings portal never owns a value.** It reads settingsd over
  `org.dragonfruit.Settings1` (`GetAll` + `Changed` + name watch) and answers
  with the appearance defaults when settingsd is absent. Do not add a second
  writer.
- **`BACKEND_INTERFACES` / `dragonfruit.portal` are additive.** T-13.2a must
  append `org.freedesktop.impl.portal.FileChooser` to both (the data test
  keeps them in lockstep).
- **The portal→settingsd mapping is string-keyed** (`appearance.colorScheme`,
  `appearance.accent`); `org.dragonfruit.desktop` forwards key names verbatim.
- A missing Settings key/namespace is
  `org.freedesktop.DBus.Error.InvalidArgs`; a duplicate session path or an
  unknown session is `org.freedesktop.DBus.Error.Failed`.

## T90 — T-13.2a FileChooser portal

**State: done.** The backend now serves the standard FileChooser interface
(version 3) at `/org/freedesktop/portal/desktop`, advertises it in
`dragonfruit.portal` / `model::BACKEND_INTERFACES`, and a D-Bus test client
drives OpenFile/SaveFile to a normalized path. Browsing and URI normalization
go through files-core. Contract frozen in ADR
[0076](design/adr/0076-filechooser-portal-and-presenter-seam.md).

What landed:

- **`portal/src/chooser.rs`** (new) — the pure model: `ChooserKind`
  (`OpenFile`/`SaveFile`/`SaveFiles`), `ChooserOptions` (decodes `multiple`,
  `directory`, `modal`, `accept_label`, `current_name`, `current_folder`,
  `current_file`, `files`; the `ay`/`aay` arrays are null-terminated),
  `ChooserRequest` (also keeps `raw_options`), `ChooserResponse`
  (`success`/`cancelled`/`other`, `uris()`), `normalize_uri` (files-core
  `Location`), `ChooserRegistry` (`begin`/`complete`/`cancel`/`handles`/
  `request`; duplicate handle refused), the one-shot `ChooserCompletion`/
  `ChooserCompleter` future, and `list_directory`/`DirectoryListing`/
  `ChooserEntry` over `DirectoryModel` + `StdFsSource`.
- **`portal/src/interfaces.rs`** — `FileChooserPortal` +
  `FILE_CHOOSER_INTERFACE`; `OpenFile`/`SaveFile`/`SaveFiles` register, emit
  `FileChooserOpened`, and await; `Version` = 3.
- **`portal/src/dbus.rs`** — `Backend` shares the `chooser` registry
  (`with_services`, `chooser()`); `initialize` serves `FileChooserPortal`.
  Presenter half on `org.dragonfruit.Portal1`: `PendingFileChoosers() ->
  a(ss)`, `CompleteFileChooser(handle, uris) -> b`, `CancelFileChooser(handle)
  -> b`, `ListDirectory(uri) -> a(ssbu)`, signal
  `FileChooserOpened(handle, kind, app_id, parent_window, title, options)`.
- **Dependency** — `portal` links `dragonfruit-files-core` (library, never a
  process) for `Location` and listing; `portal/Cargo.toml` gained the path
  dep.
- **Identity / data** — `BACKEND_INTERFACES` and `dragonfruit.portal` now
  list `org.freedesktop.impl.portal.FileChooser` (data test keeps lockstep).
- **Tests** — `portal/tests/filechooser.rs` (new, 5 integration on a private
  bus; the client acts as the presenter). Unit tests in `chooser.rs` and
  `interfaces.rs`; `session_bus.rs` expects the third interface;
  `portals.rs` introspects it.

Commands that work (repo root):

- `cargo test -p xdg-desktop-portal-dragonfruit` — 35 unit + 5
  `filechooser` + 4 `session_bus` + 3 `portals`.
- `cargo clippy -p xdg-desktop-portal-dragonfruit --all-targets -- -D
  warnings`, `cargo fmt -p xdg-desktop-portal-dragonfruit -- --check` — exit 0.
- `make e2e` (exit 0) and `make lint` (exit 0), re-run after the change.

Live check: nested demo over the synthetic socket
(`DRAGONFRUIT_SYNTHETIC_INPUT=$XDG_RUNTIME_DIR/dragonfruit-t90-capture.synth
make demo DEMO_ARGS="--socket-name dragonfruit-t90-capture"`) rendered the
desktop; full-screen still `/tmp/opencode/t90/t90-desktop.png`, vision-checked
— menu bar, Dock band, Settings Appearance pane, and the X11 demo window all
present, no black regions/clipping/tearing. This task adds no surface, so this
is a regression pass.

Gotchas for later tasks:

- **T-13.2b owns the picker.** No dialog exists yet: a request stays pending
  until a presenter calls `CompleteFileChooser`/`CancelFileChooser`. The shell
  picker should watch `FileChooserOpened`, browse with `ListDirectory` (or its
  own files-core bridge), and complete with local paths or `file://` URIs —
  `normalize_uri` accepts either and discards foreign schemes.
- **`uris` is the only success payload** (plus `writable=false` for open).
  Filters/choices are not interpreted in T-13.2a; the raw options reach the
  presenter through `FileChooserOpened` / `ChooserRequest::raw_options`.
- **`ListDirectory` is diagnostic and blocks** while files-core lists; it is
  the seam, not the final transport.
- The impl-portal method is **synchronous** (blocks until the presenter
  answers); the one-shot completion future is the async seam. The `handle` is
  an object path on the wire — a test client must send an `ObjectPath`, not a
  `String`.
- T-13.2b adds the picker UI; T-13.7 adds the real frontend routing check.

## T91 — T-13.2b FileChooser picker UI

**State: done.** The FileChooser portal now has its dialog: a design-system
picker rendered by the shell into a centred `file-chooser` overlay surface,
fed by a presenter bridge that watches `FileChooserOpened` and returns the
selection over `CompleteFileChooser`. Browsing is files-core, in-process.
Contract frozen in ADR
[0077](design/adr/0077-filechooser-picker-seat.md).

What landed:

- **`shell/screenshot/FileChooser.qml`** (new) — the pure view: title, an Up
  control and folder breadcrumb, the entry list (folders-first, natural order
  from files-core), a SaveFile name field, a status line, and Cancel plus the
  caller's `accept_label`. Emits `selectionChanged`/`entryActivated`/
  `browseRequested`/`upRequested`/`nameEdited`/`accepted`/`cancelled`; no D-Bus.
- **`shell/src/chooserbridge.{h,cpp}`** (new, dockcore) — `ChooserBridge`:
  subscribes to `org.dragonfruit.Portal1.FileChooserOpened`, decodes the
  request's options (`current_folder`/`current_file`, `multiple`, `directory`,
  `accept_label`), lists folders through files-core, and answers with
  `CompleteFileChooser`/`CancelFileChooser`. A missing portal is a normal
  state. `begin()` is shared by the live signal and the fixture.
- **`shell/src/files_core_list.h`** (new) — the listing slice of the
  files-core C ABI, mirroring `files_core_trash.h`; keeps the shell's two
  translations units in lockstep with `ffi.rs`.
- **`shell/src/shellprotocol.{h,cpp}`** — `createChooserSurface`/
  `commitChooserImage`/`hideChooser`/`setChooserInputRegion` (overlay layer,
  namespace `file-chooser`, unanchored = centred, `ON_DEMAND` keyboard) and
  pointer/keyboard routing signals. No compositor change: layer namespaces are
  generic. `teardown()` destroys `m_chooserLayer`/`m_chooserSurface` like every
  sibling overlay (fixed in a later session; the first pass leaked them to the
  display disconnect).
- **`shell/src/shellcontroller.{h,cpp}`** — owns the bridge and the offscreen
  QML scene, maps/unmaps the surface on `started`/`finished`, routes pointer,
  keyboard (Escape cancels, Return accepts) and the view's signals, and
  presents a fixture under `DF_CHOOSER_FIXTURE` (a folder to browse; empty/`1`
  = home).
- **Tests** — `shell/tests/tst_chooser.cpp` (listing, folders-first selection,
  SaveFile join, go-up, cancel, directory mode, and a real round-trip against a
  fake `org.dragonfruit.Portal1` on the private bus) and
  `shell/tests/tst_chooserui.{cpp,qml}` (the view).

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 45/45 pass, including
  `tst_chooser` (needs `dbus-run-session`; skips the live half without a bus)
  and `tst_chooserui`.
- `make lint`, `make e2e` re-run after the change.

Live check (re-run and image-inspected in a later session):
`DF_CHOOSER_FIXTURE=/tmp/opencode/t91 make demo DEMO_ARGS="--socket-name
dragonfruit-t91-capture"` mapped the picker over the desktop; full-screen still
`/tmp/opencode/t91-chooser.png` captured with `spectacle -b -n -f` and analysed
with `.symphony/symphony vision` — centred 640×440 card titled "Open File",
breadcrumb `/tmp/opencode/t91`, folders-first rows `alpha`, `beta`, then
`notes.md`/`report.pdf`/`zeta.txt` with sizes, and `Cancel`/`Open` footer
buttons, no clipping or overlap. Log confirms `FileChooser scene-graph commit
path active`.

Gotchas for later tasks:

- **`ChooserBridge` lists in-process through files-core, not the portal's
  diagnostic `ListDirectory`.** That is deliberate: the fixture renders a real
  listing with no portal. `ListDirectory` remains the backend's own seam for
  `portal/tests/filechooser.rs`.
- **The portal normalizes the final selection.** The bridge sends the entry's
  canonical `file://` URI (or a SaveFile local path); anything files-core
  cannot address locally is the portal's to discard.
- **The surface is created at startup but unmapped until a request.** Absence
  of the portal or a presenter is a silent, supported state.
- **`accept_label` is honoured verbatim** when non-empty, else falls back to
  Open/Save/Choose from `kind`.
- T-13.7 owns the real `xdg-desktop-portal` routing and Flatpak walkthrough;
  T-13.3a/T-13.4a reuse the same `shell/screenshot` overlay module.

## T92 — T-13.3a Screenshot portal and selection UI

**State: done.** The backend serves `org.freedesktop.impl.portal.Screenshot`
(version 2) beside the other three interfaces, and the shell has the
region/window/fullscreen selection overlay, driven both by portal requests and
by the Cmd+Shift+3/4 shortcut. Contract frozen in ADR
[0078](design/adr/0078-screenshot-portal-and-selection-overlay.md).

What landed:

- **`portal/src/screenshot.rs`** (new) — the pure model: `CaptureMode`
  (`Fullscreen`/`Region`/`Window`, `as_str`/`parse`/`default_for`),
  `ScreenshotOptions` (`modal`, `interactive`, the `mode` extension),
  `ScreenshotRequest` (resolves its mode; `success(uri)` normalizes through
  files-core and refuses a foreign scheme), `ScreenshotResponse`
  (`success`/`cancelled`/`other`, `uri()`), `ScreenshotError`, the one-shot
  `ScreenshotCompletion`/`ScreenshotCompleter`, `ScreenshotRegistry`
  (`begin`/`request`/`handles`/`len`/`is_empty`/`complete`/`cancel`), and the
  `SharedScreenshot`/`registry`/`lock` plumbing.
- **`portal/src/interfaces.rs`** — `SCREENSHOT_INTERFACE` and the
  `ScreenshotPortal` zbus object (`Screenshot`, `Version`); the request
  registers, emits the diagnostic `ScreenshotOpened`, and awaits.
- **`portal/src/dbus.rs`** — `Backend` gained the shared `screenshot`
  registry (`with_services` now takes it, `screenshot()` accessor);
  `initialize` serves `ScreenshotPortal`; diagnostic `PendingScreenshots`,
  `CompleteScreenshot(handle, uri)`, `CancelScreenshot(handle)`, signal
  `ScreenshotOpened(handle, mode, app_id, parent_window, options)`.
- **Identity / data** — `model::BACKEND_INTERFACES` and
  `portal/data/dragonfruit.portal` now list
  `org.freedesktop.impl.portal.Screenshot` (data test keeps lockstep).
- **`shell/screenshot/SelectionOverlay.qml`** (new; the old
  `ScreenshotOverlay.qml` placeholder is deleted) — the pure view: a
  full-output scrim, a mode badge, a region drag with live `w × h`, a window
  crosshair, Escape cancel and Return accept. Emits `accepted(x,y,w,h)` /
  `cancelled()`; no D-Bus.
- **`shell/src/screenshotbridge.{h,cpp}`** (new, dockcore) —
  `ScreenshotBridge`: watches `ScreenshotOpened`, opens the overlay in the
  request's mode, emits `captureRequested(mode,x,y,w,h)`, and answers the
  portal with `CompleteScreenshot`/`CancelScreenshot`. `begin` (live),
  `beginLocal` (the shortcut), `accept`, `complete`, `cancel`. A missing portal
  is a normal state.
- **`shell/src/shellprotocol.{h,cpp}`** — a full-output `screenshot` overlay
  layer surface (annotated all four edges, `exclusive_zone = -1`, `ON_DEMAND`
  keyboard) plus pointer/keyboard routing signals and `teardown`. No
  compositor change.
- **`shell/src/shellcontroller.{h,cpp}`** — owns the bridge and the offscreen
  QML scene, maps/unmaps the surface on `started`/`finished`, routes pointer,
  keyboard (Escape cancels via the bridge), and presents a fixture under
  `DF_SCREENSHOT_FIXTURE=<mode>` (empty/`1` = region). `onInputAction` handles
  `screenshot` (fullscreen) and `screenshot-region`.
- **`compositor/src/input/action.rs` / `shortcuts.rs`** — split
  `InputAction::Screenshot` (`screenshot`, Cmd+Shift+3) from the new
  `InputAction::ScreenshotRegion` (`screenshot-region`, Cmd+Shift+4); both
  already flowed through the one engine.
- **Tests** — `portal/tests/screenshot.rs` (new, 4 integration on a private
  bus: the interface introspects; *each mode* registers, the presenter sees
  the mode, `CompleteScreenshot` returns the normalized URI; cancel answers 1;
  a foreign URI answers 2), unit tests in `screenshot.rs`/`interfaces.rs`,
  `session_bus.rs` expects the fourth interface, `portals.rs` introspects it;
  `shell/tests/tst_screenshot.cpp` (bridge lifecycle + a fake
  `org.dragonfruit.Portal1`) and `tst_screenshotui.{cpp,qml}` (the view).

Commands that work (repo root):

- `cargo test -p xdg-desktop-portal-dragonfruit` — 4 `screenshot` + 5
  `filechooser` + 3 `portals` + 4 `session_bus` + unit.
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor input::`
  — 31 pass.
- `ctest --test-dir build --output-on-failure` — 47/47 (`tst_screenshot` needs
  `dbus-run-session`; skips the live half without a bus).
- Build note: the compositor needs the dev sysroot —
  `export PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH`
  and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).
- `make lint`, `make e2e`, `make check-no-capture-grab` — exit 0.

Live check (vision-inspected): `DF_SCREENSHOT_FIXTURE=region make demo
DEMO_ARGS="--socket-name dragonfruit-t92-region"`, full-screen still
`/tmp/opencode/t92/region.png` — dimmed overlay, centered "Screenshot · Region"
badge with "Drag to select · Esc to cancel", no clipping. Fullscreen/window
stills at `/tmp/opencode/t92/{fullscreen,window}.png` show the matching badges
and the full-output accent border.

Gotchas for later tasks:

- **The capture bytes are T-13.3b.** The shell emits `captureRequested` and
  leaves a portal request with the presenter seam; T-13.3b turns the rectangle
  into a saved image and calls `ScreenshotBridge::complete(uri)`. No portal
  change is needed. The desktop shortcut (`beginLocal`) closes the overlay on
  selection.
- **The `mode` option is a Dragonfruit extension.** A standard frontend never
  sends it: absent, a non-interactive request is fullscreen and an interactive
  one is region. The diagnostic `ScreenshotOpened` carries the resolved mode.
- **Window selection has no window geometry.** The shell is not given window
  rects, so a window click emits the pointer position with a 1×1 box; the
  capture seam (T-13.3b) must resolve the window under that point.
- **The screenshot surface is full-output and created at startup, unmapped
  until a request.** Absence of the portal or a presenter is a silent,
  supported state.
- T-13.4a (ScreenCast picker) reuses the same `shell/screenshot` module and
  overlay pattern; T-13.7 owns the real frontend routing/Flatpak walkthrough.

## T93 — T-13.3b Screenshot save/copy and portal-only gate

**State: done.** A capture is now actually produced, saved, and copied. The
compositor renders the still itself on the private protocol (`df_toplevel_manager`
v6) and the shell writes it into `Pictures/Screenshots` and puts it on the
clipboard. No client-facing grab path exists; `make check-no-capture-grab` is
green. Contract frozen in ADR
[0079](design/adr/0079-screenshot-capture-delivery-and-save-copy.md).

What landed:

- **`protocols/dragonfruit-toplevel.xml`** — `df_toplevel_manager` v6 adds
  `capture_screenshot(x, y, width, height, mode, path)` and the
  `screenshot_saved(path)` / `screenshot_failed(reason)` events.
  `MANAGER_INTERFACE_VERSION` in `compositor/src/shell/mod.rs` is 6.
- **`compositor/src/state.rs`** — `PendingCapture`; `DfState::request_capture`
  (zero size = full output, `mode` `window` resolves the topmost window under
  the point) and `take_capture`.
- **`compositor/src/shell/mod.rs`** — the request arm and
  `broadcast_screenshot_saved`/`broadcast_screenshot_failed`.
- **`compositor/src/backend/nested.rs`** — `capture_frame` renders the frame's
  already-built element list into a `GlesTexture` (`create_buffer`/`bind`),
  reads it back (`copy_framebuffer`/`map_texture`), crops, and `resolve_capture`
  writes PNG to the shell's path. It runs **before** the window render so the
  winit EGL surface is restored for the submit. `headless.rs`/`drm.rs` reply
  `screenshot_failed` explicitly.
- **`shell/src/screenshotwriter.{h,cpp}`** (new, dockcore) — `save` (PNG into
  `DF_SCREENSHOT_DIR` or `<Pictures>/Screenshots`), `copy` (image + saved
  `file://` URI via `QGuiApplication`'s clipboard), `deliver`.
- **`shell/src/shellprotocol.{h,cpp}`** — `captureScreenshot(...)` and the
  `screenshotSaved`/`screenshotFailed` signals (listener order updated for the
  new events).
- **`shell/src/shellcontroller.{h,cpp}`** — `onScreenshotCaptureRequested`
  unmaps the overlay, writes a temp path, and requests capture;
  `onScreenshotSaved` loads the temp PNG, saves/copies, removes the temp, and
  completes a waiting portal request with the saved URI. `onScreenshotFailed`
  cancels a waiting request.
- **Dependency** — `dragonfruit-shell-dockcore` now links `Qt6::Gui` (clipboard).
- **Tests** — `shell/tests/tst_screenshotwriter.cpp` (save writes a PNG, copy
  sets the clipboard incl. the file URI, unique names); a new
  `screenshot_capture_requests_are_answered_portal_only` in
  `compositor/tests/shell_protocol_conformance.rs` (headless answers
  `screenshot_failed`); two existing assertions bumped from manager v5 to v6.

Commands that work (repo root):

- `make e2e` — exit 0. `make lint` — exit 0.
- `./scripts/check-no-capture-grab.sh` — exit 0.
- `ctest --test-dir build --output-on-failure` — 48/48.
- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance
  screenshot_capture` — passes.
- Build note (unchanged): `export PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH`
  and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Live check (vision-inspected): `DF_SCREENSHOT_FIXTURE=fullscreen
DF_SCREENSHOT_DIR=/tmp/opencode/t93/shots make demo DEMO_ARGS="--socket-name
dragonfruit-t93"`, then a synthetic click accepted the overlay. The compositor
wrote a temp PNG, the shell saved
`Screenshot_2026-09-26_02-13-48.png` (66515 bytes) and logged
`screenshot saved … (copy=yes)`. Vision: upright, menu bar, Dock (five icons),
dark wallpaper, the Settings "Appearance" window and the X11 demo window all
present, no scrim/overlay and no black regions.

Gotchas for later tasks:

- **Nested only.** DRM/headless reply `screenshot_failed`; the shell treats
  that as a cancel. T-13.4b's PipeWire path should serve real DRM.
- **Coordinates are output-local pixels at scale 1** with a Normal transform.
- **The shell passes a temp path; the final save is `ScreenshotWriter`'s.**
  The compositor's reply path is the one the shell gave it.
- **The `mode` string is a Dragonfruit extension** on the capture request; the
  portal backend is unchanged from T-13.3a.
- **Copy is Qt's clipboard**, not the Wayland data device yet (T-13.5).
- T-13.4a (ScreenCast picker) reuses the `shell/screenshot` module and this
  capture seam for its stills fallback.

## T94 — T-13.4a ScreenCast portal and source picker

**State: done.** The backend serves `org.freedesktop.impl.portal.ScreenCast`
(version 3): `CreateSession` serves a standard Session object, `SelectSources`
opens the source picker and awaits a one-shot completion, and `Start` returns
the chosen sources as `streams`. The shell has the monitor/window picker,
driven by portal requests. The PipeWire stream is T-13.4b. Contract frozen in
ADR [0080](design/adr/0080-screencast-portal-and-source-picker.md).

What landed:

- **`portal/src/screencast.rs`** (new) — the pure model: `SourceType`
  (`Monitor`/`Window`/`Virtual`, `bit`/`parse`/`as_str`), `ScreenCastOptions`
  (`types`, `multiple`, `cursor_mode` with spec defaults), `ScreenCastRequest`,
  `ScreenCastSelection`, `ScreenCastStream` (`to_wire`/`from_wire`),
  `ScreenCastResponse` (`started` builds the `a(ua{sv})` array explicitly),
  `ScreenCastSession`, `ScreenCastError`, the one-shot
  `ScreenCastCompletion`/`ScreenCastCompleter`, and `ScreenCastRegistry`
  (`create_session`/`begin_select`/`complete`/`cancel`/`start`/
  `close_session`/`handles`). `AVAILABLE_SOURCE_TYPES = 3`,
  `AVAILABLE_CURSOR_MODES = 3`.
- **`portal/src/interfaces.rs`** — `ScreenCastPortal` (`CreateSession`,
  `SelectSources`, `Start`, `Version`, `AvailableSourceTypes`,
  `AvailableCursorModes`) and `ScreenCastSessionObject` (the standard Session
  `Close`). `SelectSources` emits the diagnostic `ScreenCastOpened` and awaits.
- **`portal/src/dbus.rs`** — the shared screencast registry (new
  `with_services` argument), diagnostic `PendingScreenCasts`,
  `CompleteScreenCast(handle, a(su))`, `CancelScreenCast(handle)`, signal
  `ScreenCastOpened(handle, session_handle, app_id, types, multiple, options)`.
  (`cursor_mode` rides in the raw options to keep the signal under clippy's
  7-arg limit.)
- **`portal/data/dragonfruit.portal`** + `model::BACKEND_INTERFACES` list
  `org.freedesktop.impl.portal.ScreenCast` (lockstep test green).
- **`shell/src/screencastbridge.{h,cpp}`** (new, dockcore) —
  `ScreenCastBridge`: watches `ScreenCastOpened`, `begin`/`setSources`/
  `select`/`accept`/`cancel`; answers the portal with `CompleteScreenCast`
  (`a(su)`, via a registered `ScreenCastSelection` metatype) /
  `CancelScreenCast`. A missing portal is a normal state.
- **`shell/screenshot/ScreenCastPicker.qml`** (new) — the pure view:
  "Share your screen", the app subtitle, `ListView` with `section.property`
  (Screens/Windows), per-row icon/label/detail and a checkbox (multiple) or
  radio (single) indicator, the hint, and Cancel/Share.
- **`shell/src/shellprotocol.{h,cpp}`** — the centred `screencast` overlay
  layer surface, its configure/pointer/keyboard signals, and
  `screencastSources()` (monitors from announced outputs with name/size;
  windows from the toplevel projection).
- **`shell/src/shellcontroller.{h,cpp}`** — owns the bridge and offscreen QML
  scene, filters `screencastSources()` by the request's `types`, maps/unmaps
  the surface, routes pointer/keyboard, and presents a deterministic fixture
  under `DF_SCREENCAST_FIXTURE` (`1`/`both`/`monitors`/`windows`).
- **Tests** — `portal/tests/screencast.rs` (4 integration on a private bus:
  interface + properties introspect; a full CreateSession→SelectSources(open
  the picker)→CompleteScreenCast→Start round trip returns both source handles;
  cancel answers 1 and a follow-up selection works; an empty selection answers
  2). `shell/tests/tst_screencast.cpp` (bridge lifecycle + a fake
  `org.dragonfruit.Portal1`) and `tst_screencastui.{cpp,qml}` (the view).

Commands that work (repo root):

- `cargo test -p xdg-desktop-portal-dragonfruit` — 52 lib + 4 `screencast` +
  others, all pass.
- `ctest --test-dir build --output-on-failure` — 50/50, including
  `tst_screencast` (needs `dbus-run-session`) and `tst_screencastui`.
- `make lint` — exit 0 (fmt, clippy, qmllint, token/desktop-name gates).
- `make e2e` — exit 0.
- Build note (unchanged): `export PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH`
  and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Live check (vision-inspected): `DF_SCREENCAST_FIXTURE=1 make demo
DEMO_ARGS="--socket-name dragonfruit-t94"`, full-screen still
`/tmp/opencode/t94/screencast.png` (spectacle). Vision: centred "Share your
screen" card with the "org.example.App wants to record…" subtitle, "Screens"
(Built-in Display 1920 × 1080) and "Windows" (Settings, Files) sections with
empty selection boxes, "Choose one or more sources", and Cancel / Share; on top
of the X11 demo window, no clipping. Log confirms `ScreenCast scene-graph
commit path active`.

Gotchas for later tasks:

- **The stream is a handle, not bytes.** `Start` returns `node_id = 0` and a
  Dragonfruit `id` property with the chosen source handle; T-13.4b replaces the
  node id and adds geometry/cursor. No session, picker, or diagnostic change is
  needed.
- **The backend advertises version 3.** Persistence (`persist_mode` /
  `restore_data`, v4) and virtual monitors are not advertised; bump the version
  with the restore-data format when streaming supports them.
- **Source ids are `monitor:<name>` and `window:<toplevel-windowId>`** (the
  shell's spelling); the type bit (`1` monitor, `2` window) travels with each
  selection and becomes the stream's `source_type`.
- **The picker surface is created at startup, unmapped until a request.**
  Absence of the portal or a presenter is a silent, supported state.
- T-13.4b (PipeWire stream/fallback) and T-13.7 (real `xdg-desktop-portal`
  routing/Flatpak walkthrough) are the follow-ons; the `shell/screenshot`
  module is the shared home for the portal pickers.

## T95 — T-13.4b ScreenCast stream and stills fallback

**State: done.** The ScreenCast stream now goes through one transport seam, and
this build ships the *named* stills fallback (no in-process PipeWire producer).
A standard client cannot yet receive live frames; the gap is explicit on the
wire, on the diagnostic, and in the picker. Contract frozen in ADR
[0081](design/adr/0081-screencast-stream-negotiation-and-stills-fallback.md).

What landed:

- **`portal/src/stream.rs`** (new) — `StreamMode` (`pipewire`/`stills`),
  `FallbackReason` (`pipewire-producer-unavailable`/`source-unavailable`),
  `StreamSource`, `NegotiatedStream`, the one `StreamTransport` trait,
  `StillsTransport`, `StreamNegotiator`, and `stream_properties`. Properties
  `df_stream_mode` / `df_fallback` are Dragonfruit extensions.
- **`portal/src/screencast.rs`** — `ScreenCastStream` carries `mode`/`fallback`;
  `from_wire` defaults a missing mode to `stills`; `ScreenCastRegistry` holds a
  `StreamNegotiator`, tracks live node ids per session, releases them on
  `close_session`, and exposes `with_transport`/`stream_mode`.
- **`portal/src/dbus.rs`** — diagnostic method `ScreenCastStreamMode` (spoken
  before a source is chosen). `Start` negotiates.
- **`shell/src/screencastbridge.{h,cpp}`** — `streamMode`/`streamNote`; reads the
  diagnostic asynchronously (missing service/method leaves `stills`).
  `shell/screenshot/ScreenCastPicker.qml` shows `streamNote`; `shellcontroller`
  forwards it.
- **Tests** — 5 `stream.rs` units, `screencast.rs` live-node/release units,
  `portal/tests/screencast.rs` wire+diagnostic assertions, `tst_screencast.cpp`,
  `tst_screencastui.qml`.

Commands that work (repo root):

- `cargo test -p xdg-desktop-portal-dragonfruit` — 4 `screencast` + others pass.
- `ctest --test-dir build --output-on-failure` — 50/50.
- `make lint` — exit 0; `make e2e` — exit 0.
- Build note: `export PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH`
  and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Live check: `DF_SCREENCAST_FIXTURE=1 make demo DEMO_ARGS="--socket-name
dragonfruit-t95b"` + `spectacle` (`/tmp/opencode/t95/capture2.sh`), still
`/tmp/opencode/t95/screencast2.png`. Vision: the picker shows the exact note
"Live streaming is not available in this build — the app will receive still
images.", the Screens/Windows list, Cancel/Share; no binding loops; desktop
renders normally.

Gotchas for later tasks:

- **No live producer is built.** This is the timeboxed, named fallback the
  track design allows. A real producer only implements
  `portal::stream::StreamTransport`; session, picker, and diagnostic surfaces do
  not change. `node_id` stays `0` in fallback.
- **`df_stream_mode`/`df_fallback` are Dragonfruit extensions**; the standard
  frontend ignores them. Geometry/cursor metadata can be added by a producer
  without a backend version bump.
- T-13.5 (clipboard) and T-13.7 (real frontend routing/Flatpak) are the
  follow-ons; T-13.7's browser walkthrough will exercise this fallback.

## T96 — T-13.5a Clipboard text/image/uri-list round-trips

**State: done.** The clipboard round-trip matrix is proven on the headless
compositor through the existing data-device bridge, and the ownership contract
is frozen. No compositor behaviour changed: Smithay's `delegate_data_device`
already carries every MIME type. Contract in ADR
[0082](design/adr/0082-clipboard-data-device-bridge-ownership.md).

What landed:

- **`compositor/tests/shell_protocol_conformance.rs`** — new
  `clipboard_round_trips_text_image_and_uri_list`: two independent Wayland
  clients, the source focused via `xdg-activation` sets a `wl_data_source`
  offering `text/plain;charset=utf-8`, `image/png`, and `text/uri-list`; the
  target takes focus, receives the offer, and reads every payload back
  byte-for-byte. The `TestClient` harness gained
  `source_payloads: HashMap<String, Vec<u8>>`, `clipboard_offer`,
  `clipboard_mimes`, the `wl_data_device.selection` child registration, and
  `read_pipe_to_bytes`.
- **`docs/design/07-system-integration.md`** — a "Clipboard (T-13.5a)" section.
- **ADR 0082** — `wl_data_device` is the one owner; `wlr-data-control` is the
  manager half; a history manager must forward, not replace, the source.

Commands that work (repo root):

- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance
  clipboard_round_trips_text_image_and_uri_list` — passes.
- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance` —
  35/35.
- `make lint` — exit 0 (fmt, clippy, qmllint, gates; ctest 50/50).
- `make e2e` — exit 0.
- Build note (unchanged): `export PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH`
  and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Live check (no surface of its own): `make demo DEMO_ARGS="--socket-name
dragonfruit-t96"`, raised via the KWin scripting D-Bus (as
`scripts/capture-live-menubar.sh` does), then `spectacle -b -n -a`, trimmed to
1920x1200 → `/tmp/opencode/t96/desktop.png`. Vision: menu bar
(`dragonfruit-settings` + File/Edit/View, clock), Dock, dark wallpaper,
Settings "Appearance", the X11 demo window; no black regions or clipping.

Gotchas for later tasks:

- **`wl_data_device` selection is focus-gated.** Smithay denies `set_selection`
  from an unfocused client and only offers the selection to the focused
  client's data device. Tests must activate/focus both sides; a background
  copier must use `wlr-data-control`.
- **`wlr-data-control` is the manager path** (advertised, open filter, no
  focus). T-13.5b's history observes and forwards through it; it must not
  become a second owner.
- The shell's screenshot copy (T-13.3b) uses Qt's clipboard, which is the same
  Wayland data device when the shell is a Wayland client.
- T-13.5b adds clipboard history if the design calls for it (legacy T-29 says
  yes); T-13.6 is the polkit agent.

## T97 — T-13.5b Clipboard history (if specified)

**State: done.** The design calls for history (ADR 0082's accepted consequences
name T-13.5b's history with clear-on-lock and size caps; legacy T-29 specifies
it), so it is implemented, not declined. The shell observes the selection over
the vendored `wlr-data-control` client protocol, feeds a pure bounded store,
renders it in the Control Center, and forwards a chosen entry. Contract frozen
in ADR [0083](design/adr/0083-clipboard-history-store-and-observer.md).

What landed:

- **`shell/src/clipboardhistory.{h,cpp}`** (new, dockcore) — `ClipboardHistory`:
  classifies `text`/`image`/`files`/`other`, dedups by content hash, caps at
  50 entries / 1 MiB each / 8 MiB total (evicting oldest unpinned), refuses the
  common password-manager secret hints, clears unpinned on lock (pinned
  survive), searches, and round-trips best-effort JSON.
- **`protocols/wayland-protocols/wlr-data-control-unstable-v1.xml`** (new,
  vendored MIT) + client-binding generation in `shell/src/CMakeLists.txt`.
- **`shell/src/shellprotocol.{h,cpp}`** — binds
  `zwlr_data_control_manager_v1`, gets a seat device, reads the supported MIME
  payloads on `selection`, emits `clipboardObserved`, and re-serves an entry
  with `offerClipboard` (source + `set_selection`). Idle observation never
  replaces the selection (ADR 0082).
- **`shell/src/shellcontroller.{h,cpp}`** — owns/persists the store
  (`~/.local/share/dragonfruit/dragonfruit-shell/clipboard-history.json`),
  clears unpinned on lock, feeds the panel, handles copy/pin/clear.
- **`shell/control-center/ControlCenter.qml`** — a Clipboard section (5 entries
  max) with previews, Pin/Unpin, and Clear; the five-tile model is unchanged.
  The panel grew 520 → 780 px.
- **Tests** — `shell/tests/tst_clipboardhistory.cpp` (13 cases) + 2 Control
  Center QML cases.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 51/51.
- `make lint` — exit 0; `make e2e` — exit 0.
- Build note (unchanged): `export PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH`
  and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Live check: `DF_STATUS_FIXTURE=1 make demo DEMO_ARGS="--socket-name
dragonfruit-t97-capture"`; opened the Control Center via the real
Control-Option-C shortcut over `DRAGONFRUIT_SYNTHETIC_INPUT`; captured with
`spectacle -b -n -a`. A seeded history file exercised the load path. Vision
(`/tmp/opencode/t97/panel-wide.png`): five tiles above a Clipboard section with
the pinned text entry ("Unpin"), the uri-list entry "shot.png, notes.txt"
("Pin"), the text entry "hello from the clipboard observer" ("Pin"), and a
fully visible "Clear Clipboard" link; no clipping.

Gotchas for later tasks:

- **The store owns nothing while idle.** `offerClipboard` is called only when
  the user copies an entry; that is the one ownership hand-off.
- **Primary selection is ignored** (its offer is destroyed); only the clipboard
  is stored.
- **Control Center is 780 px tall now**; a taller section should scroll rather
  than grow the surface without bound.
- The C++ data-control client has no headless test (needs a live compositor);
  the store and view are covered. A Rust conformance client would close that.

## T98 — T-13.6 polkit authentication agent

**State: done.** A privileged polkit request now raises a Dragonfruit
design-system dialog. The shell hosts a real polkit authentication agent; the
host's `polkit-agent-helper-1` still owns the PAM conversation and reports to
the authority, so we never verify a credential or escalate privilege. Contract
frozen in ADR [0084](design/adr/0084-polkit-authentication-agent.md).

What landed:

- **`shell/src/polkitagent.{h,cpp}`** (dockcore) — `PolkitAgent` exports
  `org.freedesktop.PolicyKit1.AuthenticationAgent` as a `QDBusVirtualObject` at
  `/org/dragonfruit/PolicyKit1/AuthenticationAgent` on the **system bus** and
  registers with `org.freedesktop.PolicyKit1` as the shell's session agent
  (`unix-session` from `XDG_SESSION_ID`; process-subject overrides
  `DF_POLKIT_SUBJECT_SESSION` / `DF_POLKIT_SUBJECT_PID` and setters). It parses
  the `(sa{sv})` identities by hand (the attr is `uid`, not `name`), resolves
  the username with `getpwuid`, queues concurrent requests, and registers
  **asynchronously** so a missing/slow authority never blocks startup. Absent
  authority or an existing agent for the subject is a normal degraded state.
- **`shell/src/polkitsession.{h,cpp}`** (dockcore) — `PolkitHelperSession`: the
  host helper boundary. Spawns `polkit-agent-helper-1 <user>` (override
  `DF_POLKIT_HELPER`; also `QLocalSocket` to `/run/polkit/agent-helper.socket`
  on polkit 127+ hosts), writes the cookie on stdin, relays the escaped PAM
  lines, and forwards the response. The helper, not the agent, calls
  `AuthenticationAgentResponse`.
- **`shell/screenshot/PolkitDialog.qml`** — pure view: title, identity, action
  message, masked field, expandable details, Cancel/Authenticate.
- **`shell/src/shellprotocol.{h,cpp}`** — a centered `polkit` overlay surface
  (namespace `polkit`, on-demand keyboard, pointer + shift-aware key routing).
- **`shell/src/shellcontroller.{h,cpp}`** — maps the dialog, owns the response
  buffer (`dragonfruit::lockKeyFromEvdev`, same as the lock screen), submits on
  Return, cancels on Escape/lost keyboard, and presents `DF_POLKIT_FIXTURE`.
- **Tests** — `tst_polkitagent.cpp` (11 cases incl. a **real `pkcheck` request**
  against the host polkitd) and `tst_polkitui.qml` (8 cases). `shell/src`
  dockcore now links `Qt6::Network` (for `QLocalSocket`).

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53.
- `make lint` — exit 0; `make e2e` — exit 0 (the polkit surface is created and
  configured in the scripted demo).
- Run the agent test directly:
  `dbus-run-session -- build/shell/tests/tst_polkitagent` (the fake-authority
  half needs the private session bus; the real-polkit half uses the system bus
  and skips when absent).
- Build note (unchanged): `export PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH`
  and `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Live check: `DF_POLKIT_FIXTURE=1 make demo DEMO_ARGS="--socket-name
dragonfruit-t98"`, captured with `spectacle -b -n -f` →
`/tmp/opencode/t98/polkit.png`. Vision: the centered dialog shows
"Authentication Required", identity "user", "Authentication is required to
manage system services or other units.", the "Enter your password" field,
"Show details", Cancel/Authenticate; the menu bar and Dock render normally; 0
binding loops.

Gotchas for later tasks:

- **A dev host's own agent (here the KDE agent) owns the session subject.**
  Dragonfruit's session registration is refused and logs a registration error —
  the intended degradation; the desktop keeps working. The real-request test
  and a dev session avoid it by registering for a process subject
  (`DF_POLKIT_SUBJECT_PID`); in a Dragonfruit session the session agent
  registers cleanly.
- **The helper is the only credential path.** Never call
  `AuthenticationAgentResponse` from the shell and never add a second privilege
  scheme; a future auth method (T-15) extends `PolkitHelperSession`.
- The password field reuses the lock screen's fixed US/ASCII evdev mapping
  (T-12.3c); a non-US layout is the same known limitation.
- `DF_POLKIT_FIXTURE` resolves locally (`presentLocal`) with no helper.
- Registering the agent object on the system bus requires no root; the agent
  path is fixed and one per process.

## T99 — T-13.7 Flatpak validation and capture

**State: done** (one honest deviation: no Flatpak↔native clipboard still; see
below). A real Flatpak browser (`org.mozilla.firefox`) file-chooses,
screenshots, and screen-shares through the **real** `xdg-desktop-portal`
frontend into the Dragonfruit backend, and the shell picker it raises is
committed as a capture. Running against the real frontend found and fixed two
wire-contract bugs the private-bus tests could not. Contract frozen in ADR
[0085](design/adr/0085-real-frontend-portal-compatibility.md).

What landed:

- **`scripts/capture-portals.sh`** (new; `make portals-capture`) — private
  session bus + real frontend + backend + `settingsd` + notifications; a host
  presenter (the shell's role) and a driver executed *inside* `flatpak run
  org.mozilla.firefox`. Writes `docs/captures/t13-portals.txt` (round-trips)
  and `t13-portals.png` (the picker raised by the Flatpak FileChooser, the
  private bus exported into `make demo` so the shell is the presenter). Not in
  `make e2e`.
- **`scripts/flatpak-portal-driver.py`** (new) — `presenter` / `client` /
  `trigger` roles; one file for host and sandbox.
- **`docs/captures/t13-portals.txt`** — FileChooser `uris` exported into the
  sandbox document portal, Screenshot `uri`, ScreenCast `streams`
  (`df_stream_mode=stills`, `df_fallback=pipewire-producer-unavailable`), and
  the frontend-reported ScreenCast/Screenshot versions (3 / 2).
- **`docs/captures/t13-portals.png`** — 1920×1200 active-window still; vision:
  menu bar + Dock render; centered picker titled "Open a file from the Flatpak
  browser", breadcrumb `/home/user`, folders-first rows, Cancel/Open.
- **Real-frontend fixes:**
  1. `portal/data/dragonfruit.portal` and `dragonfruit-portals.conf` used `;`
     comments; GLib key files need `#` (these edits were already uncommitted in
     the tree; now validated by the frontend loading them).
  2. Impl interfaces exported `Version` (zbus default); the standard is
     lower-case `version`. The frontend read 0 and disabled ScreenCast cursor
     modes (so `SelectSources` with `cursor_mode` was rejected), the Screenshot
     `uri` result, and persistence. Fixed with
     `#[zbus(property, name = "version")]` on Settings/Screenshot/ScreenCast/
     GlobalShortcuts; FileChooser's non-standard `Version` removed. Pinned by
     `the_standard_interfaces_expose_the_lowercase_version_property`
     (`portal/tests/portals.rs`) and `filechooser.rs` (asserts absence).
- **Docs** — ADR 0085 and a "Flatpak validation (T-13.7)" section in
  `07-system-integration.md`.

Commands that work (repo root):

- `cargo test -p xdg-desktop-portal-dragonfruit` — 58 lib + 5 filechooser + 4
  screenshot + 4 screencast + 4 portals + 4 session_bus, all pass.
- `cargo clippy -p xdg-desktop-portal-dragonfruit --all-targets -- -D
  warnings` — exit 0; `cargo fmt -p ... -- --check` — exit 0;
  `./scripts/check-no-capture-grab.sh` — green.
- `bash scripts/capture-portals.sh` (or `make portals-capture`) — exit 0;
  `CLIENT: RESULT: PASS`.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Deviation: clipboard. There is no clipboard portal by design (ADR 0082); a
Flatpak app's clipboard is the Wayland data device through the sandbox proxy.
T-13.5a proves the device with native clients; a nested Flatpak↔native
clipboard still is recorded as a follow-up above. Everything else the task
names passed.

Gotchas for later tasks:

- **The private portal bus must also run `settingsd`**, or the shell stops
  before mapping its chrome (no menu bar/Dock) even though it authenticates
  with the compositor.
- **`version`, not `Version`.** New impl portal `version` properties must name
  the wire property `version`; the discoverability files must use `#` comments.
- **Run `spectacle` on the host bus** (`DBUS_SESSION_BUS_ADDRESS=unix:path=$XDG_RUNTIME_DIR/bus`),
  never the private bus.
- The ScreenCast path is still the named stills fallback; a live producer only
  extends `portal::stream::StreamTransport`.
- The flatpak client uses `--filesystem=<repo>/scripts` and needs distinct
  `handle_token`s per request (same token + same sender = same request path,
  which aliased the client's response map).

## T100 — T-14.1a app-index identity resolution and icons

**State: done.** `org.dragonfruit.AppIndex1` is real: identity resolution for
Wayland `app_id` and X11 `WM_CLASS`, themed icon resolution to files, and a
flat-JSON session-bus surface. The compositor's interim `.desktop` resolver is
deleted (it now publishes raw identity) and the Dock loads its identity corpus
from the service and renders themed icons. Contract frozen in ADR
[0086](design/adr/0086-app-index-identity-ownership.md).

Real paths:

- `services/app-index/src/index.rs` — pure `AppIndex` (scan, `resolve`,
  `resolve_window`, `lookup`, `records`, `misses`, counts) and `AppRecord`.
- `services/app-index/src/icons.rs` — `IconTheme` freedesktop lookup, with a
  memoized `lookup` and cached size-dir/listing caches.
- `services/app-index/src/dbus.rs` — `org.dragonfruit.AppIndex1` at
  `/org/dragonfruit/AppIndex1`; `slice/main.rs` serves it with debug flags.
- `services/app-index/tests/session_bus.rs` — private `dbus-daemon` fixtures.
- `compositor/src/identity.rs` — **deleted**; `compositor/src/xwayland.rs`
  `resolve_x11_identity` returns the raw `WM_CLASS` class.
- `shell/src/appindexclient.{h,cpp}` — the shell's client; `DesktopEntry` grew
  `iconPath`; `shell/src/desktopentry.{h,cpp}` `loadFromRecords` /
  `loadFromAppIndex`; `shell/src/dockmodel.cpp` carries `iconPath`; `shell/dock/
  DockGlyph.qml` renders it.

Commands that work (repo root):

- `cargo test -p dragonfruit-app-index` — 18 unit + 5 session-bus cases.
- `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D
  warnings`; `cargo fmt --all -- --check` — all green.
- `ctest --test-dir build --output-on-failure` — 53/53 (adds a `tst_dockcore`
  `iconPath` case and a `tst_dock.qml` themed-icon case).
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **`Enumerate` was ~8s cold** resolving icons for all 265 entries; caching in
  `IconTheme` brought it to ~0.2s. The shell's blocking client timeout is
  `AppIndexClient::kCallTimeoutMs` (2000); keep the cache or raise it.
- **No svg imageformat plugin in the toolchain.** The Dock renders `.svg`
  icon paths through `QtQuick.VectorImage` and raster through `Image`.
- **The shell fallback local scan remains** for an absent service; T-14.7
  deletes it. Launch command building still uses the shell's `Exec` expansion.
- **The compositor no longer records the miss set**; app-index does. T-14.1b's
  events/launch registry/recency and T-14.1c's subscription extend this
  additively.
- A live check needs `dragonfruit-app-index` running on the session bus
  (`target/debug/dragonfruit-app-index &`) before `make demo`; otherwise the
  Dock falls back to initial tiles.

## T101 — T-14.1b app-index events, launch registry, recency

**State: done.** `org.dragonfruit.AppIndex1` is now live: the index re-scans
and diffs into install/uninstall/update events (driven by a recursive inotify
watcher, no idle polling), and the service owns a launch registry and a
most-recent-first recency order fed by window activity. Contract frozen in ADR
[0087](design/adr/0087-app-index-events-launch-registry-recency.md).

Real paths:

- `services/app-index/src/index.rs` — `IndexEvent`/`IndexEventKind`; `AppIndex`
  remembers its dirs, `refresh()` diffs (installed/updated/uninstalled),
  `events()`/`drain_events()`, `revision()`; `resolve_activity()` resolves a
  window without touching the counters/miss set.
- `services/app-index/src/registry.rs` (new) — pure `LaunchRegistry`,
  `RunningApp`, `ActivityKind`/`ActivityEvent`, recency ring (`RECENT_CAPACITY`
  32), ordered by insertion so same-ms events order correctly.
- `services/app-index/src/watch.rs` (new) — `DirectoryWatcher` (recursive
  inotify, `DEBOUNCE_MS` 150, watches established in `new()`), `watch_dirs`.
- `services/app-index/src/dbus.rs` — additive methods `WindowOpened`,
  `WindowClosed`, `NoteActivity`, `Running`, `Recent`, `Refresh`,
  `IndexEvents`, `ActivityEvents`; signals `AppRunning`, `AppExited`,
  `IndexChanged`. `run()` starts the watcher and emits `IndexChanged` per diff.
- `services/app-index/src/{view,lib,main}.rs` — JSON views
  (`running_json`/`recent_json`/`index_events_json`/`activity_events_json`);
  `--refresh` flag.
- `services/app-index/Cargo.toml` — adds `libc` (inotify).
- `docs/design/02-compositor.md` "Application identity" updated.

Commands that work (repo root):

- `cargo test -p dragonfruit-app-index` — 26 unit + 8 session-bus cases.
- `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D
  warnings`; `cargo fmt --all -- --check` — all green.
- `ctest --test-dir build --output-on-failure` — 53/53; `make e2e` — exit 0.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **The index refresh never touches the resolution audit.** Window activity
  uses `resolve_activity` (no miss, no counter); the shell's `Resolve` calls
  remain the only source of `Stats`/`Misses`.
- **Recency is insertion-ordered, not timestamped.** Do not re-sort by `activeMs`
  (two events can share a millisecond).
- **The watcher adds watches synchronously in `DirectoryWatcher::new`**; keep
  that if you refactor, or a write can race the first `add_watch` and be
  missed. Use `DirectoryWatcher` directly for a test that needs determinism.
- **The shell does not call the registry yet.** `WindowOpened`/`WindowClosed`/
  `NoteActivity` are exercised by the private-bus tests; wiring the forwarder
  remains (see Follow-ups).
- T-14.1c: the signals (`AppRunning`, `AppExited`, `IndexChanged`) are already
  emitted; it only needs the subscription bookkeeping and coalescing.

## T102 — T-14.1c app-index subscription API

**State: done.** `org.dragonfruit.AppIndex1` now has a subscription surface:
`Subscribe`/`Unsubscribe` register a consumer by its unique bus name for the
`identity`/`recency`/`icons` categories, and a coalescer thread delivers **one
directed `Changed` signal per 100 ms burst** instead of a signal per change.
Consumers no longer need to re-query or poll. Contract frozen in ADR
[0088](design/adr/0088-app-index-subscription-coalescing.md).

Real paths:

- `services/app-index/src/subscription.rs` (new) — pure `ChangeKind`
  (`Identity`/`Recency`/`Icons`), `Interests` (bitmask, `parse`/`as_str`/
  `includes`/`union`/`intersection`), `ChangeNotice` (destination + union of
  kinds), `Subscriptions` (`subscribe`/`unsubscribe`/`note`/`due`/
  `next_deadline_ms`/`flush`), `COALESCE_WINDOW_MS` 100.
- `services/app-index/src/dbus.rs` — `AppIndex1` carries
  `Arc<Mutex<Subscriptions>>` + a `Wake`; methods `Subscribe(interests) ->
  String`, `Unsubscribe() -> bool`, `Flush() -> u32`, `SubscriberCount() ->
  u32`, signal `Changed(interests)`; `note_change`/`note_change_at`;
  `spawn_coalescer` (condvar-woken, sleeps until the deadline, no polling).
- `services/app-index/src/lib.rs` — exports the new module and types.
- `services/app-index/tests/session_bus.rs` — new headless tests
  `a_subscriber_receives_one_coalesced_signal_per_burst` (subscribe → install +
  two window opens + focus → `Flush` delivers exactly one `Changed` carrying
  `identity,recency,icons`; none after `Unsubscribe`) and
  `the_coalescer_thread_delivers_without_a_flush` (the production thread wakes
  to the deadline and delivers).
- `docs/design/02-compositor.md` "Application identity" updated.

Commands that work (repo root):

- `cargo test -p dragonfruit-app-index` — 35 unit + 10 session-bus cases.
- `cargo clippy -p dragonfruit-app-index --all-targets -- -D warnings` — exit 0;
  `cargo fmt -p dragonfruit-app-index -- --check` — exit 0.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **Coalescing is a throttle, not a debounce.** The window opens at the first
  change and is *not* extended by later changes, so a continuous stream still
  delivers at ~10 Hz. The union of kinds is per subscriber; `interests` on the
  wire is the canonical subset string (`identity,icons`, etc.).
- **A change kind is consumed by `note` entries in `dbus.rs`, not by the pure
  model.** `Refresh` and the inotify watcher note `Identity` + `Icons`; window
  open/close/focus note `Recency`. If you add a new mutation path, call
  `AppIndex1::note_change` or subscribers never hear it.
- **An empty/unknown interest spec means `all`**, so a consumer asking for a
  category that does not exist yet goes silent rather than silent-by-default.
- **Signals are directed to the subscriber's unique name.** A consumer must
  emit `Subscribe` from the same connection it listens on; a separate
  `bus.connect()` listener will not receive them.
- **The shell does not subscribe yet** and the window-activity forwarder is
  still unwired (T-14.1b follow-up). Shell-side wiring remains for a later task.

## T103 — T-14.2a menu-broker export model and fixed menu

**State: done.** `services/menu-broker` is a real service and the fixed
application menu's Hide/Hide Others/Show All carry live state. Contract frozen
in ADR [0095](design/adr/0095-menu-broker-resolution-and-fixed-menu.md).

Real paths:

- `services/menu-broker/src/model.rs` (new) — pure `Broker`: `PublishedModel`
  parse (ADR-0041 shape), `Visibility`/`WindowState`/`HideVerbs`, the
  synthesized fixed menu, the published-menu rewrite, and
  `resolve`/`resolve_focused`/`fixed_menu`. `default_app_name` capitalizes the
  last reverse-DNS segment; empty is `Files`.
- `services/menu-broker/src/dbus.rs` (new) — `org.dragonfruit.MenuBroker1` at
  `/org/dragonfruit/MenuBroker1`: `Publish`/`Withdraw`, `SetFocusedApp`/
  `SetWindowStates`, `Resolve`/`ResolveFocused`/`Policy`,
  `PublisherCount`/`Revision`, signal `Changed(reason, appId)`.
- `services/menu-broker/src/{lib,main}.rs`, `Cargo.toml` — lib+bin; debug flags
  `--fixed <appId>` / `--resolve <appId>`.
- `services/menu-broker/tests/session_bus.rs` (new) — private `dbus-daemon`.
- `shell/src/menubrokerpolicy.{h,cpp}` (new) — the shell's pure live-state
  fallback over `dockStateChanged` entries.
- `shell/src/shellcontroller.cpp` — `applyFocusedApp` publishes the live fixed
  menu, `onDockStateChanged` re-publishes; the static `applicationMenu` helper
  is deleted.
- `shell/tests/tst_dockcore.cpp`, `shell/tests/tst_menubar.qml` — live-state
  policy and rendering cases.
- `docs/design/adr/0095-*.md` (new), `docs/design/06-global-menu.md` updated.

Commands that work (repo root):

- `cargo test -p dragonfruit-menu-broker` — 13 unit + 4 session-bus cases.
- `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D
  warnings`; `cargo fmt --all -- --check` — all green.
- `ctest --test-dir build --output-on-failure` — 53/53; `make e2e` — exit 0.

Gotchas for later tasks:

- **The shell does not talk to the service yet.** `ShellController` computes the
  live flags locally (`menubrokerpolicy`); the broker's D-Bus surface is only
  covered by its private-bus tests. T-14.2b should push `SetWindowStates` +
  `SetFocusedApp` and read `Resolve`, and wire Settings' `publishedModel` to
  `Publish`.
- **The `resolved` JSON shape is** `{appId, appName, tier, applicationMenuItems,
  menus}`; `tier` is `native`/`dbusmenu`/`none`. The fixed application menu is
  the same design-system entry shape, with `enabled` always present on the hide
  rows.
- **`SetWindowStates` takes** `[{appId, windows, minimized}]` where `minimized`
  is true only when **all** the app's windows are minimized (the Dock
  projection's reading). Malformed rows are skipped, not fatal.
- **Actions still log.** Hide/Hide Others/Show All reflect live enabled state
  but do not yet minimize windows; that needs a compositor window-state request.
- Live check: `make demo` + a synthetic click at the app-menu title (x≈64,
  y≈14) captured `/tmp/opencode/t103-menu-open.png` (idle bar:
  `/tmp/opencode/t103-bar-idle.png`). `Show All` renders dimmed while `Hide
  dragonfruit-settings`/`Hide Others` are bright (two apps running). Not
  committed to `docs/captures` (that is the track-boundary artifact).

## T104 — T-14.2b menu-broker accelerators and toggle

**State: done.** The menu-broker now parses and dispatches focus-scoped
accelerators, the compositor admits them over a new additive protocol request,
and `menu.global` is the Settings-owned global-menu toggle. Contract frozen in
ADR [0096](design/adr/0096-focus-scoped-accelerators-and-global-menu-toggle.md).

Real paths:

- `services/menu-broker/src/accelerators.rs` (new) — pure `Mods`/`Chord`
  (parse + canonical `Super+Shift+H`), `Accelerator` (+`to_json`), `extract`
  (walks `applicationMenuItems` + `menus`, recursively through `submenu`; a row
  needs both `shortcut` and `action`), and the focus-scoped `AcceleratorTable`
  (`register`/`clear`/`set_focused`/`resolve`; `Dispatch::System` wins over
  `Application`/`None`).
- `services/menu-broker/src/model.rs` — `Broker` owns an `AcceleratorTable`;
  publish/withdraw update it, `set_focused` scopes it; `resolve()` carries an
  `accelerators` array; new `set_system_accelerators`, `accelerators_for`,
  `focused_accelerators`, `resolve_accelerator`.
- `services/menu-broker/src/dbus.rs` — additive methods `Accelerators(appId)`,
  `FocusedAccelerators()`, `Dispatch(spec)` (returns
  `{"kind":"application"|"system"|"none",…}`), `SetSystemAccelerators(json)`.
- `services/menu-broker/tests/session_bus.rs` — new private-bus case
  `registrations_are_focus_scoped_and_dispatch_over_the_bus`.
- `protocols/dragonfruit-toplevel.xml` — `df_toplevel_manager` v7 adds
  `set_app_accelerators(app_id, accelerators)` (one `action<TAB>chord` per
  line, empty clears).
- `compositor/src/input/shortcuts.rs` — `ShortcutEngine::set_app_accelerators`
  and the pure `parse_accelerator_table` (skips malformed lines; uses
  `keymap::parse_binding`).
- `compositor/src/state.rs` — `DfState::set_app_accelerators`.
- `compositor/src/shell/mod.rs` — `MANAGER_INTERFACE_VERSION = 7` and the
  request handler.
- `shell/src/menubrokerpolicy.{h,cpp}` — pure `MenuAccelerator`,
  `menuAccelerators`, `publishedAccelerators`, `acceleratorWireTable`.
- `shell/src/shellprotocol.{h,cpp}` — `setAppAccelerators` and the
  `appAccelerator(appId, action, source, serial)` signal (the previously inert
  `app_accelerator` event is now forwarded).
- `shell/src/shellcontroller.cpp` — `applyFocusedApp` registers the focused
  app's table; `onAppAccelerator` routes a matched chord through the new
  `dispatchAppAction` (shared with the menu-click path); `applyMenuBarPolicy`
  reads `menu.global`; the demo menu rows gained `action` strings.
- `shell/menubar/MenuBar.qml` — `globalMenuEnabled` suppresses only the app's
  exported menus.
- `services/settingsd/src/schema.rs` — schema v6, `KeyGroup::Menu`,
  `menu.global` (bool, default true); `libs/settings-client/settingsclient.cpp`
  seeds it; `docs/settings-keys.md` documented.
- `apps/settings/DesktopDockPane.qml` — a "Menu Bar" group with the
  `globalMenuToggle` (the dedicated pane is T-15.9b).
- Tests: `shell/tests/tst_dockcore.cpp` (accelerator flattening/wire form),
  `shell/tests/tst_menubar.qml` (toggle hides/shows exported menus),
  `apps/settings/tests/tst_settings_desktop_dock.qml` (toggle applies and
  converges).

Commands that work (repo root):

- `cargo test -p dragonfruit-menu-broker` — 25 unit + 5 session-bus cases.
- `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D
  warnings`; `cargo fmt --all -- --check` — all green.
- `ctest --test-dir build --output-on-failure` — 53/53; `make e2e` — exit 0.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **The broker never installs a grab.** It maps chord→action; the compositor's
  `ShortcutEngine` matches and emits `app_accelerator`; the shell routes it via
  `dispatchAppAction`. System shortcuts still win in the engine, and the broker
  can be told the reserved set with `SetSystemAccelerators`.
- **Chords are canonicalized in the broker** (`Super`/`Alt`/`Control`/`Shift`,
  command first) but the key token keeps its published case: the compositor's
  `parse_binding`/`keysym_from_name` is case-sensitive for letters, so do not
  lowercase keys.
- **A row registers only with both `shortcut` and `action`**; submenus are
  walked, separators skipped.
- **The accelerator table is per app and replaced atomically** on re-publish;
  `withdraw` clears it. `Dispatch` returns `system` even with no focus (a
  reserved chord needs no app).
- **The shell sends the table for `m_appId` on every `applyFocusedApp`;** the
  desktop (empty app id) clears it. This is the shell's own menu, not the
  broker's `Resolve` (transport still pending — see Follow-ups).
- `df_toplevel_manager` is now **v7**; the two conformance version assertions
  were bumped. The request is appended after the v6 messages so opcodes are
  stable.
- Live check: `make demo` + a synthetic click captured
  `/tmp/opencode/t104b-idle.png` (bar with app menu title + File/Edit/View) and
  with `menu.global=false` (settingsd + gdbus) `/tmp/opencode/t104c-off.png`
  (app menu title only, no File/Edit/View). Not committed to `docs/captures`
  (track-boundary artifact).

## T105 — T-14.3 StatusNotifier/AppIndicator tray

**State: done.** StatusNotifier/AppIndicator tray items render in the menu
bar and their DBusMenu works end to end. `dragonfruit-app-index` is now also
the tray host: it serves `org.kde.StatusNotifierWatcher`, keeps a pure
registration/DBusMenu model, and exposes it to the shell over
`org.dragonfruit.AppIndex1`. Contract frozen in ADR
[0097](design/adr/0097-statusnotifier-tray-host-and-dbusmenu-projection.md).

Real paths:

- `services/app-index/src/tray.rs` (new) — pure `Registration`
  (`parse`: bus name → `/StatusNotifierItem`; path → `sender:/path`),
  `TrayRegistry` (`register`/`remove`/`remove_sender`/`remove_owner`), and the
  DBusMenu projection: `MenuNode` (`to_json`), `parse_layout`
  (`GetLayout (i,a{sv},av)` → rows), mnemonic stripping, toggle/checked state,
  shortcut strings.
- `services/app-index/src/dbus.rs` — `WATCHER_NAME`/`WATCHER_PATH`/
  `WATCHER_INTERFACE`/`ITEM_INTERFACE`/`DBUSMENU_INTERFACE`; the
  `StatusNotifierWatcher` object sharing `AppIndex1::tray()`; new `AppIndex1`
  methods `TrayItems`/`TrayNames`/`TrayMenu`/`TrayMenuEvent`/`TrayActivate` and
  the `TrayChanged` signal; `spawn_tray_cleaner` (NameOwnerChanged → prune);
  `run()` takes the watcher name best-effort and still serves the watcher
  object when another host owns it.
- `services/app-index/src/main.rs` — `--mock-tray <icon>` debug item + menu.
- `services/app-index/tests/session_bus.rs` — mock SNI item/menu; two cases.
- `shell/src/trayclient.{h,cpp}` (new) — pure `parseItems`/`parseMenu` + the
  D-Bus calls; `TrayItem`.
- `shell/src/shellcontroller.{h,cpp}` — `m_trayClient`, `refreshTrayItems`
  (2 s `QTimer`), `openTrayMenu`, `onTrayMenuTriggered`, `onTrayMenuClosed`;
  tray items appended to `statusItems` as `tray:<name>` with a
  `QUrl::fromLocalFile` `iconSource`.
- `shell/menubar/StatusItem.qml` — `iconSource` (`Image` instead of glyph);
  `MenuBar.qml` — `trayMenu`/`trayMenuOpen`, `openTrayMenu`/`closeTrayMenu`,
  the design-system `ContextMenu`, and the tray branch in `_dropdownRect`.
- `shell/tests/tst_dockcore.cpp`, `shell/tests/tst_menubar.qml` — decode and
  interaction cases.
- `docs/design/06-global-menu.md` "Implementation note (T-14.3)";
  `docs/design/adr/0097-*.md` (new).

Commands that work (repo root):

- `cargo test -p dragonfruit-app-index` — 42 unit + 12 session-bus cases
  (incl. `a_mock_tray_item_registers_and_its_menu_round_trips`).
- `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D
  warnings`; `cargo fmt --all -- --check` — all green.
- `ctest --test-dir build --output-on-failure` — 53/53; `make e2e` — clean.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **The tray is app-index's, not the shell's.** `org.kde.StatusNotifierWatcher`
  is taken best-effort. On a KDE host plasmashell already owns the name, so
  app-index logs "already owned" and serves only the watcher *object*; a mock
  or app can register by calling
  `RegisterStatusNotifierItem` on `org.dragonfruit.AppIndex1` at
  `/StatusNotifierWatcher`.
- **All shell calls are flat JSON.** `TrayItems` returns
  `[{name,destination,path,id,title,iconName,iconPath,tooltip,menuPath,
  itemIsMenu,status,needsAttention,hasPixmap}]`; `TrayMenu` returns the
  design-system row array (nested `submenu`); a miss is `[]`/`false`, never an
  error. `TrayItems` prunes an item whose owner is gone.
- **The ContextMenu's size is content-derived**, so the shell re-commits the
  overlay popup rectangle at 0/60/160 ms after opening a tray menu; without
  that the surface stays at its first (near-empty) size and clicks miss. If
  you change the menu component, keep those delayed `updatePopupGeometry`
  calls.
- **`iconSource` must be a file URL**, not a bare path: QML resolves a plain
  string relative to the QML file (`qrc:`). The controller wraps `iconPath`
  with `QUrl::fromLocalFile`.
- **T-14.4 reuses the projection**: bridge DBusMenu into the menu-broker with
  `tray::MenuNode`/`tray::parse_layout`/`GetLayout`, not a second parser.
- Live check: nested `make demo` with `dragonfruit-app-index` +
  `dragonfruit-app-index --mock-tray firefox` on the session bus —
  `/tmp/opencode/t105/traymenu4.png` shows the Firefox tray icon in the bar and
  its open menu (`Show Window` / separator / `Tools › Preferences`); clicking
  `Show Window` logged `mock tray menu event 1 (clicked)`. The active-window
  capture trim is flaky on this KWin host, so treat the shell/DBus logs as the
  hard evidence. Not committed to `docs/captures` (track-boundary artifact).

## T106 — T-14.4 DBusMenu bridge

**State: done.** A DBusMenu/AppMenu-exporting app's global menu is now bridged
into the menu-broker at the `dbusmenu` tier, with live enable/disable state.
`dragonfruit-app-index` is also the AppMenu.Registrar host. Contract frozen in
ADR [0098](design/adr/0098-dbusmenu-bridge-in-app-index.md).

Real paths:

- `services/app-index/src/menubridge.rs` (new) — pure `MenuRegistration`
  (`parse`: explicit app id, else the owner bus name; absolute path),
  `AppMenuRegistry` (`register`/`unregister`/`remove_owner`/`has_app`/`entries`),
  `menus_from_layout` (DBusMenu root children → the broker's `[{title, items}]`
  menus, invisible rows omitted, separators skipped), `entry_json` (row +
  synthesized `action: "dbusmenu:<id>"`; `enabled`/`checked`/`shortcut`/`id`
  unchanged from `tray::MenuNode::to_json`), `action_for`/`id_from_action`,
  `published_model`, `fallback_app_name`, and the registrar/broker constants.
- `services/app-index/src/dbus.rs` — `APP_MENU_REGISTRAR_{NAME,PATH,INTERFACE}`;
  `AppMenuRegistrar` object (`com.canonical.AppMenu.Registrar`) sharing
  `AppIndex1::menus()` + the index: `RegisterWindow(u,o)`,
  `RegisterWindowForApp(u,s,o)` (additive), `UnregisterWindow(u)`,
  `GetMenuForWindow(u)`, `IsWindowRegistered(u)`, property `RegisteredWindows`;
  new `AppIndex1` methods `AppMenuWindows`/`WindowMenu`/`WindowMenuEvent`;
  `publish_to_broker`/`withdraw_from_broker` best-effort pushes; `run()`
  serves the registrar and takes its name best-effort (like the watcher); the
  `NameOwnerChanged` cleaner now also prunes menu registrations and withdraws
  models whose last window left.
- `services/app-index/src/main.rs` — `--mock-menu [appId]` debug DBusMenu app
  (File › New enabled / Quit disabled, Edit › Cut) that registers with the
  registrar; registers keyed by app id via `RegisterWindowForApp`.
- `services/menu-broker/src/model.rs` — `tiers: BTreeMap<String, Tier>`;
  `publish` delegates to `publish_tiered(_, _, Tier::Native)`;
  `publish_dbusmenu` (tier `DbusMenu`); `withdraw` clears the tier; `tier(app)`;
  `resolve` reads the stored tier.
- `services/menu-broker/src/dbus.rs` — additive `PublishDbusMenu(appId, model)`
  (`#[zbus(name = "PublishDbusMenu")]`; zbus would otherwise emit
  `PublishDbusmenu`), signal reason `publish-dbusmenu`.
- Tests: `services/app-index/src/menubridge.rs` (6 unit cases),
  `services/app-index/tests/session_bus.rs` (2 new: mock app → registrar →
  broker push round-trip incl. enabled, and the standard registration key),
  `services/menu-broker/src/model.rs` (tier unit case),
  `services/menu-broker/tests/session_bus.rs` (bridge publish resolves at
  `dbusmenu` + focus-scoped `dbusmenu:<id>` accelerator).
- Docs: `docs/design/06-global-menu.md` "Implementation note (T-14.4)";
  `docs/design/adr/0098-*.md` (new).

Commands that work (repo root):

- `cargo test -p dragonfruit-app-index` — 48 unit + 14 session-bus cases.
- `cargo test -p dragonfruit-menu-broker` — 26 unit + 6 session-bus cases.
- `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D
  warnings`; `cargo fmt --all -- --check` — all green.
- `ctest --test-dir build --output-on-failure` — 53/53; `make e2e` — exit 0.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).
- Live bridge proof (private bus, real binaries): start
  `dragonfruit-app-index` + `dragonfruit-menu-broker`, then
  `dragonfruit-app-index --mock-menu org.example.MockMenu`; `gdbus call … 
  MenuBroker1.Resolve org.example.MockMenu` returned
  `"tier":"dbusmenu"` with `menus` File(`New` enabled, `Quit` disabled)/Edit and
  `accelerators` `[{"action":"dbusmenu:1","chord":"Control+N"}]`.

Gotchas for later tasks:

- **The registrar body is `(u, o)`, not `(u, s)`.** Pass a
  `zvariant::ObjectPath`, never a `&str`, or zbus errors with a signature
  mismatch. `RegisterWindowForApp` is `(u, s, o)`.
- **The bridge is a push.** app-index projects on each registration and calls
  `org.dragonfruit.MenuBroker1.PublishDbusMenu(appId, model)`; an absent broker
  is silent (best-effort). `WindowMenu`/`WindowMenuEvent` on
  `org.dragonfruit.AppIndex1` are the local query/click paths and work without
  the broker.
- **The app key is the registration's `app_id`**: the explicit id from
  `RegisterWindowForApp`, else the caller's unique bus name (`:1.x`) under the
  standard `RegisterWindow`. The shell must map focus to the same key before
  the global menu renders bridged menus.
- **Rows carry `action: "dbusmenu:<id>"`, not a semantic action.** The broker's
  accelerator extraction picks them up (a shortcut string from DBusMenu
  `shortcut` + the synthesized action). Routing that action back to the app
  means calling `WindowMenuEvent(windowId, id)`; the shell→app half is not
  wired.
- **Only visible rows survive** (`menus_from_layout` drops invisible and
  top-level separators); an empty projection is not published, so the broker
  stays Tier 3.
- **`PublishDbusMenu` must keep its explicit `#[zbus(name = …)]`** (capital M):
  zbus's default conversion of `publish_dbusmenu` is `PublishDbusmenu`.
- Live check: `make demo` (nested, host Wayland) captured
  `/tmp/opencode/t106-desktop.png` (17:02) — the desktop, menu bar, Dock, and
  settings window render normally, no stray artifacts. The bridge has no shell
  surface yet (the shell still computes its own menu), so the pixel check only
  confirms the demo renders; the D-Bus round-trip above is the hard evidence.
  Not committed to `docs/captures` (track-boundary artifact).

Pre-existing, not introduced by T106: `make lint`'s `check-desktop-names`
gate is red because T-14.3's standard protocol names (`org.kde.
StatusNotifierWatcher` / `org.kde.StatusNotifierItem`) trip the `kde` word
match in `services/app-index/{dbus,tray,lib,main}.rs`,
`services/app-index/tests/session_bus.rs`, `shell/src/trayclient.h`, and
`shell/tests/tst_dockcore.cpp`. The gate supports a `df-allow-desktop-name`
marker; adding it (or exempting the `org.kde.` freedesktop namespace) is a
small separate fix. `cargo fmt`/`clippy` and every task-relevant suite are
green.

Remaining for a later session (none required for this task's acceptance):

- Shell wiring: on focus, map the app/window to a registrar entry, read
  `WindowMenu`/the broker's `Resolve`, render it, and route `dbusmenu:<id>`
  actions through `WindowMenuEvent`. Live re-projection on `about-to-show` /
  DBusMenu `ItemsPropertiesUpdated`/`LayoutUpdated` is deferred (a registration
  projects once).

## T107 — T-14.5 XDnD bridge

**State: done (protocol half + documented gap; the task explicitly allows
documenting the gap at T-17).** Smithay 0.7's XWM implements no XDnD
translation and exposes neither its X connection nor a client-message hook, so
a real bridge needs its own X11 connection plus a Wayland server drag
(`start_dnd`). This session landed and tested the protocol half and recorded
the connection/runtime half as an explicit gap. Contract frozen in ADR
[0099](design/adr/0099-xdnd-bridge-model-and-documented-gap.md).

Real paths:

- `compositor/src/xdnd.rs` (new) — pure XDnD:
  - `ATOM_NAMES` (17 standard atoms), `XDND_VERSION = 5`,
    `URI_LIST_MIME = "text/uri-list"`, and `XdndAtoms` (the interned-id table:
    `names`, `as_array`, `action_atom`, `action_from_atom`,
    `message_type_name`).
  - `XdndAction` (`None`/`Copy`/`Move`/`Link`/`Ask`/`Private`, `atom_name`).
  - `XdndMessage` (`Enter`/`Position`/`Status`/`Leave`/`Drop`/`Finished`) with
    `name`/`encode(&XdndAtoms) -> Option<[u32;5]>`/`decode(name, data, atoms)`.
    `Enter` with >3 types requires the `XdndTypeList` atom and encodes the
    more-types bit; `pack_position`/`unpack_position` handle the 16-bit signed
    root coordinates.
  - The two state machines: `XdndTarget` (`enter`/`position`/`drop`/`finished`/
    `leave`/`offers`; `IncomingDrag`/`DropRequest`/`XdndStatus`/`XdndFinished`)
    and `XdndSource` (`enter`/`position`/`drop`/`leave`/`absorb_status`/
    `accepted_action`).
  - `text/uri-list`: `parse_uri_list`, `uris_to_paths`, `uri_to_path`,
    `path_to_uri`, `format_uri_list` (RFC 2483 comments/blanks, CRLF,
    `file://` percent-coding; remote hosts and non-`file:` schemes dropped).
  - 11 unit tests.
- `compositor/src/lib.rs` (new) — the compositor's library surface, exposing
  `pub mod xdnd;` so `compositor/tests/` can drive the codec without spawning
  the binary. The binary is unchanged (`main.rs` does not declare `xdnd`, so no
  dead-code warnings).
- `compositor/tests/xdnd_conformance.rs` (new) — live conformance: starts the
  headless compositor (Xwayland), interns the atoms, sets `XdndAware = 5`,
  round-trips `Enter`/`Position`/`Status`/`Finished` as real X ClientMessages,
  drives `XdndTarget` (accepts a `text/uri-list` drag, refuses a non-file one).
  Skips if `Xwayland` is absent.
- `Makefile` — `make e2e` now runs `--test xdnd_conformance`.
- Docs: `docs/design/02-compositor.md` "Implementation note (T-14.5)";
  `docs/design/tracks/17-premium-gate.md` "Known compatibility gaps";
  `docs/design/adr/0099-*.md` (new).

Commands that work (repo root):

- `cargo test -p dragonfruit-compositor --lib` — 11 xdnd unit cases.
- `cargo test -p dragonfruit-compositor --test xdnd_conformance` — 1 case.
- `cargo test -p dragonfruit-compositor` — 11 lib + 277 bin + 34 window + 35
  shell-protocol + 6 xwayland + 1 xdnd + the trace suites, all green.
- `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --
  --check` — green.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **There is no live cross-boundary file DnD yet.** `compositor/src/xdnd.rs` is
  a model, not a wired bridge; `services/app-index` and the shell do not
  consume it. Treat it as a known, waived gap at T-14.6/T-17.
- **XDnD wire layouts to respect**: `XdndStatus` puts the action atom in l[4];
  `XdndFinished` puts it in l[2] (success is l[1] bit 0). `XdndEnter` l[1] is
  `version << 24 | more-types-bit`; the positions are 16-bit signed root
  coords. The conformance test is the regression guard.
- **Adding the compositor library target changes nothing for existing tests**:
  they still use `CARGO_BIN_EXE_dragonfruit-compositor`. Pure policy modules
  can now live in the lib for direct integration testing.
- Live check: `make demo` (nested, host Wayland) captured
  `/tmp/opencode/t107-desktop.png` (17:22) — the nested desktop renders the
  wallpaper ground, Dock (6 apps + trash), the Settings window, and the X11
  demo window, no stray artifacts from this change (this task has no UI
  surface of its own). Not committed to `docs/captures` (track-boundary
  artifact).

## T107 — T-14.5 XDnD bridge (attempt 2 — gate repair)

**State: done.** The attempt-1 protocol half and documented gap stand
unchanged. Attempt 1 was marked failed for one reason only: the harness
`make e2e` verify timed out after 30 min.

Root cause (unrelated to XDnD): `portal/tests/screencast.rs` has four tests
that each stand up a private session bus and call
`xdg_desktop_portal_dragonfruit::dbus::serve` **in-process**, then block on
zbus. Run four-up (the default), the shared blocking executor can starve and a
test thread blocks forever in a `call_method` reply — it *hangs*, it does not
fail. Captured with `eu-stack`: `version_property` → `blocking::Connection::
call_method`, while the backend's connection never gets polled.
Reproduction: `target/debug/deps/screencast-*` in a loop → 1/30 hang
(1/114 on a second run). With `--test-threads=1` → 0/80.

Fix this session (Makefile-only, no product code):

- `Makefile` — the `e2e` recipe runs
  `cargo test -p xdg-desktop-portal-dragonfruit -- --test-threads=1`; the
  portal package is the only one whose tests serve D-Bus in-process. Comment
  above the target explains why. (Attempt 1 had already added
  `--test xdnd_conformance` to the same recipe.)

Verified this session (repo root; env from the build note below):

- `make e2e` — **exit 0 in 63 s** (was a 30-min timeout); scripted headless
  demo teardown clean, `xdnd_conformance` green in-suite.
- `cargo test -p dragonfruit-compositor --lib` — 11/11 xdnd unit cases.
- `cargo test -p dragonfruit-compositor --test xdnd_conformance` — 1/1.
- `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --
  --check` — green.

Gotchas for later tasks:

- **Keep `-- --test-threads=1` on the portal `e2e` line.** Without it the gate
  does not merely fail, it wedges indefinitely; the harness's 30-min verify
  timeout then reports the task as failed. Other in-process-D-Bus test
  packages should get the same treatment if they ever grow parallel blocking
  tests.
- The XDnD gap itself is unchanged: `compositor/src/xdnd.rs` is a model, not a
  wired bridge; T-14.6/T-17 treat cross-boundary file DnD as a known, waived
  gap (see ADR 0099 and `docs/design/tracks/17-premium-gate.md`).
- Live check reuse: the attempt-1 nested capture `/tmp/opencode/t107-desktop.png`
  still shows the nested Dragonfruit window (Settings > Appearance, Dock row,
  no stray artifacts); this session changed no UI code, so the visual verdict
  is unchanged.

## T108 — T-14.6a Strange-app zoo run and matrix

**State: done.** The scripted zoo run and its matrix are committed. Six rows,
all passing identity resolution: Firefox (X11), xterm, a Steam-`WM_CLASS`
stand-in, GNOME Calculator (GTK4 via flatpak), an SDL2 sample via Xwayland, and
an Electron client. The run records launch / raw identity / app-index desktop
id / SSD-CSD / menu-broker tier per app. No ADR (the only contract addition is
one read-only synthetic query).

Real paths:

- `scripts/zoo/zoo-run.sh` (new) — orchestrator (`make zoo-run`): private nested
  compositor + synthetic input, Xwayland-readiness wait, app launch, driver,
  teardown. Skips missing apps as "not run".
- `scripts/zoo/zoo-driver.py` (new) — polls `query identity`, reads X11
  `WM_CLASS` with `xprop`, resolves through `dragonfruit-app-index`, reads SSD
  from `query decorations`, menu tier from `dragonfruit-menu-broker`, writes
  `docs/captures/t14-zoo-matrix.{md,json}` and crops `docs/captures/t14-zoo.png`
  (wallpaper-colour bbox discovery, **not** the old hardcoded colour).
- `scripts/zoo/sdl_zoo.c`, `scripts/zoo/x11_zoo.c`, `scripts/zoo/electron/main.js`,
  `scripts/zoo/steam.desktop` (new) — samples/stand-in entries.
- `compositor/src/input/synthetic.rs` — new read-only `query identity`:
  `identity <id> <app_id|-> <title>` per window (Wayland `app_id` or Xwayland
  `WM_CLASS` class). Unit parse test added.
- `services/app-index/src/main.rs` — `--resolve-window <class> [instance]`
  (instance needed for Firefox X11: class `org.mozilla.firefox`,
  instance `Navigator`).
- `compositor/tests/xwayland_conformance.rs` — `identity_query_reports_x11_wm_class`.
- Docs: `docs/design/02-compositor.md` "Implementation note (T-14.6a)";
  `docs/design/tracks/14-...` "Implementation note (T-14.6a)";
  `docs/captures/README.md` zoo paragraph.

Commands that work (repo root):

- `make zoo-run` — exit 0, ~2 min (needs host Wayland, spectacle, Pillow,
  gcc+SDL2/X11, Electron at `ZOO_ELECTRON_DIR`, Calculator flatpak).
- `cargo test -p dragonfruit-compositor --lib`; `--test xwayland_conformance`
  (7 tests); `cargo test -p dragonfruit-app-index` — all green.
- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D
  warnings`; `make e2e` — exit 0.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **Firefox's raw X11 identity is `WM_CLASS = "Navigator", "org.mozilla.firefox"`**
  (the Fedora wrapper sets `MOZ_APP_REMOTINGNAME=org.mozilla.firefox`; launching
  `/usr/lib64/firefox/firefox` directly yields class `firefox`). app-index
  resolves either. An inherited host `WAYLAND_DISPLAY` makes Firefox probe the
  host compositor and fail with "cannot open display"; launch it with
  `env -u WAYLAND_DISPLAY DISPLAY=<nested> GDK_BACKEND=x11 MOZ_ENABLE_WAYLAND=0`
  and a seeded `user.js` to skip the first-run delay.
- **Flatpak needs `--filesystem=$XDG_RUNTIME_DIR/$SOCK`** in addition to
  `--socket=wayland` to reach a *custom-named* nested socket; `--socket=wayland`
  alone only exposes the host default socket.
- **SDL2's Wayland backend does not map a window** on the nested compositor (it
  stops after the registry roundtrip, no xdg_surface); the zoo uses SDL's X11
  driver with `SDL_VIDEO_X11_WMCLASS`. T-14.6b should investigate.
- **Decoration expectations**: GTK apps (Firefox, Calculator) negotiate CSD;
  xterm/Steam stand-in/SDL X11 get SSD; Electron/Chromium negotiates SSD.
- **Steam is a stand-in**, not the real client (raw X11 window with
  `WM_CLASS=Steam`); the matrix says so. Steam cannot be installed here.
- `dnf download xterm` + `rpm2cpio` extracts xterm without root; the script does
  this automatically when xterm is absent.
- The old `capture-demo-driver.py` `WALL=(45,35,51)` constant no longer matches
  the shipped wallpaper (`(33,13,41)`); the zoo driver discovers the colour
  instead. Other capture scripts using `WALL` may need the same fix.

## T109 — T-14.6b Strange-app zoo fixes

**State: done.** The zoo surfaced one fixable failure and one compositor
hardening. The T-14.6a note that "SDL2's Wayland backend does not map a window
(it stops after the registry roundtrip, no xdg_surface)" is wrong: SDL **does**
create the `xdg_surface`/`xdg_toplevel` and negotiates SSD. The sample simply
never presented a frame, and SDL's Wayland backend does not attach its first
`wl_buffer` until the app draws — the compositor only maps a toplevel that has
a buffer. No ADR (a sample fix plus a protocol-callback answer that the design
docs already describe).

Real paths:

- `scripts/zoo/sdl_zoo.c` — the loop now calls `SDL_GetWindowSurface` +
  `SDL_FillRect` + `SDL_UpdateWindowSurface` each iteration. This is what makes
  the Wayland path map.
- `scripts/zoo/zoo-run.sh` — SDL runs on the nested Wayland socket
  (`WAYLAND_DISPLAY="$SOCK" SDL_VIDEODRIVER=wayland SDL_APP_ID=game.zoo.sdl`),
  replacing the X11 fallback.
- `scripts/zoo/zoo-driver.py` — the SDL row is `backend: wayland`,
  `raw: game.zoo.sdl`, `expected_decoration: SSD`.
- `compositor/src/state.rs` — `DfState::send_pending_frame_callbacks` (new,
  called from the `CompositorHandler::commit` pending branch after
  `map_pending_windows`). The zoo's Wayland trace showed SDL requests
  `wl_surface.frame` *before* its first buffer; both normal frame paths
  (`render::post_repaint`, `render::send_frame_callbacks`) only walk `Space`
  members, so a not-yet-mapped toplevel's callback was never answered.
- `compositor/tests/window_conformance.rs` — new
  `sdl_style_pre_map_frame_callback_is_answered_and_then_maps`: reproduces
  SDL's exact sequence (pre-role buffer-less commit → configure → pre-map frame
  request → first buffer) and asserts the callback arrives. Fails without the
  compositor fix (verified by disabling the call).
- `docs/captures/t14-zoo-matrix.{md,json}`, `docs/captures/t14-zoo.png` —
  regenerated by `make zoo-run`; SDL row now Wayland/SSD/pass.
- Docs: `docs/design/02-compositor.md` "Implementation note (T-14.6b)";
  `docs/design/tracks/14-global-menu-app-index-compat.md` T-14.6b note;
  `docs/captures/README.md` zoo paragraph.

Commands that work (repo root):

- `make zoo-run` — exit 0; six rows all pass (needs host Wayland, spectacle,
  Pillow, gcc+SDL2/X11, Electron at `ZOO_ELECTRON_DIR`, Calculator flatpak).
- `cargo test -p dragonfruit-compositor --test window_conformance` — 35 pass.
- `make e2e` — exit 0.
- `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --
  --check` — green.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).
- Live check: `make zoo-run` capture (and a focused nested capture
  `/tmp/opencode/t109-nested.png`) shows the SDL window mapped with an SSD
  titlebar and a solid blue client area.

Gotchas for later tasks:

- **The SDL Wayland gap is fixed by drawing in the sample, not by a compositor
  map change.** A future SDL sample that never presents will not map; that is
  correct Wayland behavior.
- **Frame callbacks for buffer-less toplevels are now answered at commit.**
  They are *not* part of the cadence/`has_pending_frame_callbacks` path (which
  still scans only `Space`); the answer is synchronous on the commit that
  carries the request. If a later client requests `frame` without a
  subsequent commit, that path would need the cadence set widened too.
- The old T108 gotcha bullet ("SDL2's Wayland backend does not map a window …
  the zoo uses SDL's X11 driver") is superseded by this section.
- Decoration expectations are otherwise unchanged: Firefox/Calculator CSD;
  xterm/Steam stand-in/SDL/Electron SSD. Steam is still an X11 stand-in and
  cross-boundary XDnD is still the documented T-14.5 gap.

## T110 — T-14.7 Retire interim paths

**State: done.** The last two interim hacks are gone: the Dock's local
`.desktop` scan/parse and the shell's `demoAppMenu()` stand-in. No ADR (the
menu-broker transport was already the frozen design in ADR 0095).

Real paths:

- `shell/src/desktopentry.{h,cpp}` — `scan`, `parse`, `defaultApplicationDirs`
  and the file-reading helpers are deleted. What remains is the app-index-fed
  record cache (`loadFromAppIndex`/`loadFromRecords`/`resolve`/`byId`) plus the
  launcher (`isLaunchable`/`buildLaunchCommand`/`appLaunchEnvironment`).
- `shell/src/shellcontroller.cpp` — `start()` always
  `m_index.loadFromAppIndex(appIndex)` (no `scan()` fallback);
  `applyFocusedApp()` no longer builds a demo menu.
- `shell/src/menubrokerclient.{h,cpp}` (new, dockcore) — `available`,
  `setFocusedApp`, `setWindowStates`, `resolveFocused`, and the pure
  `parseResolved` (`ResolvedMenu{appId,appName,tier,applicationMenuItems,menus}`).
- `shell/src/menubrokerpolicy.{h,cpp}` — new pure `windowStatesJson(entries)`:
  one `{appId,windows,minimized}` row per `kind == "temporary"` entry (the
  per-window "minimized" entries are skipped).
- `shell/src/shellcontroller.cpp` `applyFocusedApp()` — pushes
  `SetFocusedApp`/`SetWindowStates` and reads `ResolveFocused`; renders the
  broker's fixed menu + exported menus. Fallback when the service is absent is
  the local `fixedApplicationMenu` with no exported menus. One-shot log:
  `shell: global menu resolved by org.dragonfruit.MenuBroker1 tier <t> for <id>`.
- `tools/dragonfruit-dev/src/main.rs` — `launch_services()` starts
  `dragonfruit-app-index` and `dragonfruit-menu-broker` (best-effort; skipped
  without `DBUS_SESSION_BUS_ADDRESS` or a missing binary) into a new
  `ChildGuard.services` list (their early exit is not a demo failure; killed on
  teardown). Called before `launch_shell` in `run_dev_session` and
  `run_demo_session`; a 500 ms settle covers the shell's one-shot identity load.
  `dev_share_env()` dedups the staged `XDG_DATA_DIRS`/`PATH` for shell and
  services.
- `services/app-index/src/index.rs` — new
  `shipped_first_party_entries_are_launchable` (migrated from the deleted
  `tst_dockcore` test): app-index, the only production parser, verifies the
  in-repo first-party `.desktop` entries.
- `shell/tests/tst_dockcore.cpp` — parser/scan tests replaced by record-based
  fixtures (`makeEntry`); new `windowStatesJsonCarriesOneRowPerRunningApp` and
  `resolvedMenuDecodesTheBrokerReply`.
- Docs: `docs/design/06-global-menu.md` and
  `docs/design/tracks/14-global-menu-app-index-compat.md` "Implementation note
  (T-14.7)".

Commands that work (repo root):

- `make e2e` — exit 0; the demo now logs
  `launched dragonfruit-app-index` / `launched dragonfruit-menu-broker` and
  `global menu resolved by org.dragonfruit.MenuBroker1 tier "none" for ""`.
- `ctest --test-dir build --output-on-failure` — 53/53.
- `QT_QPA_PLATFORM=offscreen build/shell/tests/tst_dockcore` — 76 pass.
- `cargo test -p dragonfruit-app-index --lib` — 49 pass;
  `cargo test -p dragonfruit-dev` — 7 pass.
- `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all --
  --check` — green.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Live check: `make demo` (nested), captured `/tmp/opencode/t110-live.png`
(host 3840x2160; nested Dragonfruit window). The Dock renders six resolved
tiles (Files, Settings, Terminal, Firefox, VS Code, Trash) — proof app-index is
the live identity source — the menu bar shows the focused app's application
menu name, and the desktop area has no stray artifacts. Verdict: intended.

Gotchas for later tasks:

- **app-index is now mandatory for a populated Dock.** A session without it
  shows an empty Dock (by design); `dragonfruit dev`/`make demo` start it. If a
  later consumer needs it in another harness, start it there too.
- **The menu-broker is consumed, but two halves remain**: a clicked
  `dbusmenu:<id>` row still only logs (route it back through app-index's
  `WindowMenuEvent`), and Settings still does not `Publish` its
  `SettingsMenu.publishedModel` (so the native tier never lights). Both are in
  Follow-ups.
- **The shell loads the app corpus once at startup** (no subscription yet); the
  dev harness's 500 ms settle covers the service-start race. T-14.1c's
  `Subscribe("all")` is the real fix.
- **`ChildGuard` now has a `services` list** distinct from `launched`: services
  are killed on teardown but an early exit is not a demo failure, so a
  best-effort service can't wedge `make demo`/`make e2e`.
- The `tst_dockcore` fixtures no longer parse `.desktop` files; build records
  with the local `makeEntry(...)` helper. app-index owns parsing.

## T110a — T-14.7a Dock plate geometry and spacing

**State: done.** The Dock plate now floats: token-driven cross-axis and
along-axis padding plus a real `edgeMargin` gap to the screen edge, with the
surface and exclusive zone following the new geometry so windows never underlap
and a hidden Dock leaves no strip. No ADR (ADR 0089/0091 already froze the
model; this realizes it).

Real paths:

- `design-system/tokens/tokens.json` — `controls.dock`: `padding` 10,
  `paddingAlong` 14 (new), `gap` 8, `edgeMargin` 8. `scripts/gen-tokens.py`
  regenerated `design-system/Theme.qml` and `compositor/src/design_tokens.rs`.
- `shell/dock/Dock.qml` — one `plateRect` replaces `barRect` and is the source
  for the plate drawing, the input region, and T-14.7b's panel rect;
  `paddingAlong` insets the ends; `surfaceThickness`
  (`barThickness + magnifyBand + edgeMargin`) and `reservedThickness`
  (`barThickness + edgeMargin`) are derived here. Entry placement is
  plate-relative, so `padding` stays the cross-axis inset on every position.
- `shell/src/shellcontroller.cpp` / `.h` — reads `surfaceThickness` for the
  layer-surface extent and `reservedThickness` for the exclusive zone (all
  three configure paths + creation); `m_dockBarThickness` deleted as dead.
- `shell/tests/tst_dock.qml` — `barRect`→`plateRect`; new plate cases: gap =
  `edgeMargin` on bottom/left/right, `paddingAlong` ends, entries inside the
  plate at `iconSizeMin`/`iconSizeMax`, hidden plate fully clears the surface.
- `scripts/capture-dock-spacing.sh` (new; `make dock-spacing-capture`) — nested
  run + scratch settingsd, flips `dock.position`/`appearance.colorScheme`,
  crops the 1920x1200 output to a 190 px edge strip.
- `docs/captures/t14-dock-spacing-{bottom,left,right}-{light,dark}.png` (new) —
  measured plate 75 px; gap 8 px on each edge (left x=8..83, right x=107..182,
  bottom y=105..180 at 1920x1200).
- Docs: `docs/design/04-shell.md` "Dock plate and materials" wording;
  `docs/design/tracks/14-...md` "Implementation note (T-14.7a)";
  `docs/captures/README.md` T-14.7a paragraph.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock|tst_dockcore"` — 2/2; `make qml-test` —
  53/53; `make e2e` — exit 0 (`Dock configured 1280x151`, reserved `thickness=83`).
- `make clippy`; `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh` — green.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).
- Live: `make dock-spacing-capture` (host Wayland + spectacle + Pillow).

Gotchas for later tasks:

- **`plateRect` is the contract.** T-14.7b must send it as the live
  `set_panel_rect` baseline and grow it under magnification; do not re-derive
  padding math in the shell or compositor.
- **The reserved/exclusive zone is now `barThickness + edgeMargin`** (was
  `barThickness`). At the default icon size that is 83 px; the surface is 151 px.
  Rust `reserved_rect` needs no change — it follows the exclusive zone, and
  `panel_bounds` already lands the panel on the plate (input region ∩ reserved
  strip) so the edge gap is not frosted.
- **Vertical Docks float too**: left plate x=`edgeMargin`, right plate
  `x+ w = width - edgeMargin`; the magnify band is on the interior side.
- **Auto-hide clears the surface**: `hideOffset = barThickness + edgeMargin`,
  and the hidden plate's anchored edge lands exactly on the screen edge; the
  `edgeTrigger` input sliver stays on the screen edge.
- **`check-desktop-names.sh` fails on pre-existing StatusNotifier/zoo lines**
  (untouched by T-14.7a); `make check`'s lint gate was already red for that
  reason.

## T110b — T-14.7b Magnified plate growth and backdrop panel

**State: done.** The Dock plate now grows to wrap the magnified row (both axes)
inside the pre-reserved `magnifyBand`, the compositor's frost follows it exactly,
the reserved zone is unchanged during a sweep, and the pointer is smoothed with
`motion.dockMagnify`. No new ADR — ADR 0089 (dock plate geometry and the live
panel rect) already froze this model; this realized it.

Real paths:

- `protocols/dragonfruit-shell.xml` — `df_shell` v2, `df_layer_surface` v2, new
  additive `set_panel_rect(x, y, w, h)` (surface-local). `LOCKSTEP_VERSION` stays
  1 (lockstep test green).
- `compositor/src/shell/mod.rs` — `SHELL_INTERFACE_VERSION = 2` for the
  `df_shell` global; dispatch arm `SetPanelRect` stores
  `LayerSurfaceState.panel_rect` (new field in `shell/layer.rs`) and marks a
  redraw; `chrome_surfaces` prefers `window::explicit_panel_bounds`, else the old
  `panel_bounds`.
- `compositor/src/window/backdrop.rs` — new `explicit_panel_bounds(geometry,
  explicit)` (translate + intersect; `None` when it leaves the surface); new
  test `explicit_panel_rect_is_honored_and_clipped_to_the_surface`. Re-exported
  from `window/mod.rs`.
- `shell/src/shellprotocol.{h,cpp}` — `setDockPanelRect`; binds `df_shell` at
  `df_shell_interface.version` (no hardcoded cap).
- `shell/src/shellcontroller.cpp` `renderDock()` — sends `plateRect` every
  commit before `commitDockImage`.
- `shell/dock/Dock.qml` — `restingPlateRect` (fixed anchored edge + hide
  translation; entries position against it) and live `plateRect` (union of entry
  rects + padding, clamped, bounce added back); `smoothPointerAlong` snaps on
  entry/mouse-out/reduced-motion and springs otherwise; input region = live plate
  + magnified/bouncing rects; `dockBar` draws the live plate.
- `shell/tests/tst_dock.qml` — replaced
  `test_bar_does_not_grow_with_magnification` with nine T-14.7b cases (plate
  grows, contains every entry, anchored edge fixed, reserved thickness constant,
  returns to baseline, reduced-motion, bounce excluded, smoothing, smoothing
  snap). New cases park the pointer clear first so an earlier case's hover
  position cannot seed `pointerAlong`.
- `scripts/capture-dock-magnify.sh` + `-driver.py` (new;
  `make dock-magnify-capture`) — nested sweep stills and the frame-budget probe.

Commands that work (repo root):

- `cargo test --workspace` — green; `cargo test -p dragonfruit-compositor
  --bin dragonfruit-compositor window::backdrop` — 11 pass.
- `make qml-test` — 53/53; `make e2e` — exit 0 (`Dock configured 1280x151`,
  reserved bottom `thickness=83`); `make clippy`; `cargo fmt --all -- --check`;
  `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh` — green.
- `make dock-magnify-capture` — six stills + `t14-dock-magnify-frame-budget.txt`
  (`+175` frames / 2.21 s ≈ 79 fps, skipped 0).

Live check: the six `docs/captures/t14-dock-magnify-*` stills; a left-third
montage of left/center/right (pointer at left ⇒ leftmost icon largest, plate
tallest at the left) confirms the plate and its frost track the magnified row.
`backdrop_passes=17` in the demo log proves the explicit panel is consumed.

Gotchas for later tasks:

- **The plate top rises by the same peak everywhere** (one hovered icon reaches
  the same max size), so left/center/right stills differ in *where* the tall icon
  sits, not in plate height. Compare the hovered end, not the plate height.
- **`restingPlateRect` is the layout anchor.** The live `plateRect` must not feed
  back into `layout`; entry cross-axis positions reference `restingPlateRect`.
- **Bounce is excluded from the plate** by adding `entryBounce` back out of the
  cross-axis union. T-14.7c's bounce-phase discipline can rely on the plate not
  pumping.
- **`df_shell` and `df_layer_surface` are both v2.** Bumping only the child
  would not let the client use the request: a Wayland `new_id` inherits the
  parent's version.
- The synthetic-input host pointer can seed `pointerAlong`; tests park the
  pointer outside the Dock before asserting geometry.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110c — T-14.7c Dock motion smoothness and frame discipline

**State: done.** Continuous Dock motion no longer rebuilds the entry model:
launch/attention phases ride a per-entry map while the `entries` list (and so
every Repeater delegate and its hover/press state) stays put. Opening a
popover no longer resizes the offscreen window, `dock.size` changes animate,
and the settled Dock still contributes zero frames. ADR
[0100](design/adr/0100-dock-motion-phase-map-and-popover-buffer.md) froze the
contract.

Real paths:

- `shell/src/shellcontroller.cpp` — `withBounce` deleted; `rebuildDockEntries`
  builds the stable entries and calls new `publishDockBouncePhases()`, which
  pushes `entry id -> { phase, attention }` as the QML `bouncePhases` property
  only when it changes. `onDockAnimationTick` advances the map (no model
  rebuild) and stops the 16 ms timer when both clocks are empty;
  `onDockLaunchTick` rebuilds only on a real launch-state change;
  `clearAttention`/`onDockAttention` publish the map. New
  `kDockPopoverHeadroom`/`kDockPopoverGutter` (320) pre-size `m_dockWindow` at
  configure (window = `dock + 2*gutter` by `dock + headroom`); `renderDock`
  uses that constant offset and no longer resizes per popover.
- `shell/dock/Dock.qml` — new `bouncePhases`, `popoverHeadroom`,
  `popoverGutter` properties; `entryPhase`/`entryAttention` +
  `clampPopoverX`/`clampPopoverY`; delegate injects `bouncePhase`/
  `bounceAttention`; root `Behavior on iconSize` gated by `laidOutOnce` (plus a
  new `onWidthChanged/onHeightChanged` 0 ms settle timer); delegate x/y
  Behaviors also cover `dragSettling`; all three popovers clamp to the budget;
  stale comments refreshed.
- `shell/dock/DockEntry.qml` — `bouncePhase`/`bounceAttention` properties;
  `attention` and `bounce` read them (entry-embedded values still win).
- `shell/tests/tst_dock.qml` — new `test_bounce_does_not_rebuild_the_entry_model`,
  `test_attention_phase_rides_the_phase_map`,
  `test_a_size_change_animates_then_settles`,
  `test_a_size_change_snaps_under_reduced_motion`,
  `test_popover_open_stays_inside_the_pre_sized_buffer`.
- `scripts/capture-dock-motion.sh` + `-driver.py` (new;
  `make dock-motion-capture`) — scratch settingsd + synthetic input; writes
  `docs/captures/t14-dock-motion-trace.txt`, `t14-dock-motion-menu-dark.png`,
  `t14-dock-motion-resized-dark.png`.
- Docs: `docs/design/04-shell.md` "Dock motion and frame discipline";
  `docs/design/adr/0100-...md`; `docs/captures/README.md` T-14.7c paragraph.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 129 cases).
- `make e2e` — exit 0; `make soak SOAK_CYCLES=5` — 5 clean cycles, zero strays.
- `make dock-motion-capture` — trace: sweep `over_budget=+0` (~66 fps),
  popover `over_budget=+5`/63 frames, size-change `+0`, idle
  `frames_rendered=+0 over_budget=+0`; whole run `tier=full downgrades=0`.
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).

Gotchas for later tasks:

- **Per-frame entry state goes through `bouncePhases`, not `entries`.** Any
  rebuild of the entry list recreates delegates and drops hover/press state;
  the two new tests guard this.
- **Popover placement must clamp into `popoverHeadroom`/`popoverGutter`**
  (shell-set; 0 = legacy unbounded). The Dock window is already that big; do
  not compute headroom in the shell.
- **`laidOutOnce` snaps the first configure**; everything after springs via the
  root `Behavior on iconSize`. A real drag reorder still resets the model, so
  only the cancel path's gap-close springs (Follow-up).
- The pre-sized buffer is ~4x the resting readback area; the nested trace shows
  no tier downgrade, but the separate-popover-window fallback is the documented
  next step if a hardware trace degrades (Follow-up).
- Running the `tst_dock` binary directly needs
  `QML2_IMPORT_PATH=$PWD/build/qml QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software`;
  `ctest` sets this.

## T110d — T-14.7d Trash entry artwork

**State: done.** The Trash glyph is now a designed, original bin at the token
size, with empty/full/unavailable states that read at a glance in light and
dark. No new ADR — ADR 0091 already froze "original geometry at the token size";
`docs/design/04-shell.md` gains a "Trash entry artwork" subsection.

Real paths:

- `shell/dock/DockGlyph.qml` — `import QtQuick.Shapes`; the `trash` item is a
  `trashSize`-derived box (`root.size * Theme.controls.dock.trashSize /
  Theme.controls.dock.iconSize`) centred in the iconSize box. Anatomy: body
  (gradient + rim + sheen), overhanging lid (rim + top highlight), clean handle;
  no plus ribs. Full adds a crumpled-paper `Shape` behind the lid (`PathSvg`
  jagged silhouette) plus the accent rim; empty is neutral. All colours are
  `Theme.color.trash*` tokens.
- `design-system/tokens/tokens.json` — new semantic colours `trashFillTop`,
  `trashFillBottom`, `trashRim`, `trashHighlight`, `trashPaper`,
  `trashPaperEdge` (light neutral200/300/500/neutral0/neutral50/neutral400;
  dark neutral600/800/400/300/200/600). Regenerated `Theme.qml` +
  `compositor/src/design_tokens.rs`.
- `shell/tests/tst_dock.qml` — helpers `backgroundIs`/`countForeground`/
  `countForegroundAll`/`sumLuma`/`trashBox`/`trashInset`/`contentsBand`/
  `makeGlyphOnBackdrop`; tests `test_trash_glyph_renders_pixels_at_every_dock_size`,
  `test_trash_empty_and_full_differ_above_the_lid`,
  `test_trash_full_and_empty_render_in_both_schemes`,
  `test_trash_unavailable_renders_dimmer`; `init()` now resets `Theme.dark`.
- `scripts/capture-dock-trash.sh` (new; `make dock-trash-capture`) — three
  nested runs (scratch `XDG_DATA_HOME`: empty / two `.trashinfo` records /
  a regular file at `Trash` for unavailable) + scratch settingsd; flips
  `appearance.colorScheme`; writes 2x whole-Dock crops.
- `docs/captures/t14-dock-trash-{empty,full,unavailable}-{light,dark}.png`
  (1690x400) — verified live: empty tidy, full paper + accent rim, unavailable
  dimmed + badge dot.
- Docs: `docs/design/04-shell.md`; `docs/captures/README.md` T-14.7d paragraph;
  task Hand-off.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$"` — 129/129; `make qml-test` — 53/53.
- `make e2e` — green (see flake note); `cargo clippy --workspace --all-targets`;
  `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `check-no-capture-grab.sh` — green.
- `make check` — unchanged: still stops at `check-desktop-names.sh` on the
  pre-existing StatusNotifier/zoo lines (nothing from this task).
- Build note (unchanged): `export
  PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig:$PKG_CONFIG_PATH` and
  `RUSTFLAGS="-L $HOME/.local/df-devroot/lib64"` (or just `make`).
- Live: `make dock-trash-capture` (host Wayland + spectacle + gdbus + Pillow).

Gotchas for later tasks:

- **The Trash is a bin, not a squircle.** T-14.7j's tile/inset work must leave
  the `trash` item's centred `trashSize` box alone; app tiles fill the whole
  iconSize box, the Trash deliberately does not.
- **`grabImage` does not apply the grabbed item's own opacity.** To pixel-test
  the unavailable dimming, grab the `DockEntry` (not the glyph) over a fixed
  dark backdrop; `shell/tests/tst_dock.qml` does this.
- **Testing gotcha:** `grabImage` composites over the offscreen stage, and
  `createTemporaryObject` deletion can overlap siblings within one test; the
  trash tests place each glyph on a dark backdrop at a distinct x to keep the
  foreground sample clean.
- **Flake:** `dragonfruit-files-core`'s
  `trash_source::tests::trash_monitor_watches_and_wakes_on_a_change` can fail
  once under parallel `make e2e` load (inotify wake vs scan); it passes on
  rerun. Not caused by this task.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110e — T-14.7e Add Application picker

**State: done.** The Dock's divider menu now offers **Add Application…**,
opening an anchored overlay popover fed by app-index; a row toggles
`dock.pinned` through the single settings writer and both the Dock and picker
update live. No new ADR — ADR 0090 already froze this model.

Real paths:

- `shell/src/apppicker.{h,cpp}` (new, dockcore) — pure
  `buildAppPickerList(entries, pinnedIds, query)` (drop `noDisplay`/
  non-launchable, dedupe by id, locale-name sort + id tiebreak, name+id
  case-insensitive filter, tag pinned) and
  `toggleAppPickerPin(pinnedIds, desktopId, pinned)`.
- `shell/dock/DockAppPicker.qml` (new) — popover: title, auto-focused
  `SearchField` (local filter mirroring the helper predicate), scrollable
  `ScrollView` rows (`DockGlyph` 28 px, name, "Add"/"In Dock"), disabled
  "No applications found" and "Application index unavailable" rows, keyboard
  Up/Down/Return/Escape, list/listitem a11y.
- `shell/dock/Dock.qml` — `appPickerItems`/`appIndexAvailable`/
  `appPickerOpen`/`appPickerAnchor`; `appPickerRequested()`/
  `appPinToggled(id, pinned)`; `openAppPicker()`; divider "Add Application…"
  item intercepted in the entry menu; picker in `popoverOpen`/`popoverRect`;
  instance mirrors the stack placement.
- `shell/src/shellcontroller.{h,cpp}` — `AppIndexClient m_appIndexClient`;
  subscribes to app-index's coalesced `Changed` (`identity`) and connects the
  directed D-Bus signal; `refreshAppPicker(reloadCorpus)` pushes the helper's
  rows; `onDockAppPickerRequested` (one-shot re-enumerate only when not
  subscribed), `onDockAppPinToggled`, `onAppIndexChanged`.
  `DF_APP_PICKER_FIXTURE` opens the picker for capture (never in a session).
- `shell/src/appindexclient.{h,cpp}` — `subscribe(interests="identity")`.
- Tests: `tst_dockcore` four `appPicker*` cases; `tst_dock.qml` nine picker
  cases.
- `scripts/capture-dock-app-picker.sh` + `make dock-app-picker-capture`;
  `docs/captures/t14-dock-app-picker.png`.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore|qmllint_shell-dock"` —
  green (`tst_dock` 142, `tst_dockcore` 80).
- `make qml-test` — 53/53; `make e2e` — green (one rerun; pre-existing
  lock-auth flake below).
- `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`.
- Live: `make dock-app-picker-capture`.

Gotchas for later tasks:

- **Filtering is in two places by design.** The pure helper's `query` is the
  tested spec; QML `filteredItems` mirrors it so standalone `tst_dock.qml` can
  exercise live typing. Keep them in sync.
- **The controller pushes the full corpus** (no filtering); `appIndexAvailable`
  is `AppIndexClient::available()`, so an available-but-empty corpus is
  "No applications found" and an absent service is "Application index
  unavailable".
- **`appPicker.open` is not bound** to `dock.appPickerOpen` (same as the stack
  popover); `onOpened`/`onClosed` sync the flag.
- **Subscription is one-shot per shell start**; a live app-index restart is not
  re-subscribed (open-time re-enumerate is the fallback). Follow-up if needed.
- **Flake (pre-existing):** `dragonfruit-lock-auth`'s
  `a_missing_user_is_a_usage_error` can fail once under `make e2e` load; passes
  on rerun. Not caused by this task.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines.

## T110f — T-14.7f Dock drag-and-drop identity and feedback

**State: done.** External drags are now read once at drag *enter*, so the Dock
shows the real app identity while hovering, each target states what a drop will
do, a duplicate alias drop pulses the existing entry, and Trash/Downloads
failures raise a notification instead of a log line. No new ADR — ADR 0090
already froze "drops show identity"; `docs/design/04-shell.md` gains a "Drop
identity and feedback" paragraph.

Real paths:

- `shell/src/dockdrops.{h,cpp}` — `DockDropPayloadData` +
  `parseDockDropPayload(mime, data)` (the one decoder, reused by enter and
  drop), `dockDropAffordance(targetKind, payload, targetName, trashAvailable)`,
  and `dockPinnedContains(pinned, id, resolvedId)`.
- `shell/src/shellprotocol.{h,cpp}` — enter-time read: `onDataDeviceEnter`
  calls new `beginDndRead()` (one pipe + `wl_data_offer_receive`, non-blocking
  `QSocketNotifier`); `onDndReadable` reaches EOF → `finishDndRead()` caches the
  parsed payload and emits new `dockExternalDragPayload(payloadIsApp, desktopId,
  paths)`. `onDataDeviceDrop` calls `ensureDndPayloadReady()` (synchronous
  bounded `poll` drain) and reuses the cache — no second receive. `resetDndRead`
  + `resetExternalDrag` clear it; a superseding offer resets it.
- `shell/src/shellcontroller.cpp` — `onDockExternalDragPayload` resolves the
  alias through `m_index` (name + `iconPath`, degrading to the raw id) and calls
  the new Dock `setExternalPayload`. `onDockExternalDropRequested`: duplicate
  alias → `flashDuplicatePin` (no append); Trash unavailable / partial trash /
  partial Downloads move → `raiseDockNotice`; multi-file stays one launch with
  all args. New `flashDuplicatePin`; new `DF_DOCK_DROP_FIXTURE` capture seam
  (`app`/`file-app`/`file-trash`).
- `shell/src/launchfailure.{h,cpp}` — `raiseDockNotice(client, summary, body)`.
- `shell/dock/Dock.qml` — `externalPayloadName/IconPath/FirstName`,
  `duplicateFlashId` + `duplicateFlashTimer` + `flashPin`; `setExternalPayload`;
  `externalAffordance` (mirrors the pure helper), `externalIdentityLabel`,
  `externalAffordanceFor`; the placeholder entry carries the ghost identity; a
  new 2x `externalAffordance` capsule; new `externalHoverEntry` test/capture
  hook.
- `shell/dock/DockEntry.qml` — `externalHasIdentity`, `duplicateFlash`; the
  identity ghost draws the real `DockGlyph` plus `externalGhostName`; the
  `duplicateFlash` highlight.
- Tests: `tst_dockcore` — `parseDockDropPayloadClassifiesAppAliasAndFiles`,
  `dropAffordanceMatchesTheActionPerTarget`,
  `duplicatePinIsDetectedByIdOrResolvedIdentity`,
  `aDockNoticeRaisesThroughTheSameNotificationPath`. `tst_dock.qml` —
  `test_external_app_ghost_shows_the_resolved_identity`,
  `_degrades_without_identity`, `test_external_file_affordances_follow_the_target`,
  `test_external_multi_file_label_counts`, `test_external_affordance_capsule_renders`,
  `test_duplicate_pin_flashes_then_clears`.
- `scripts/capture-dock-drops.sh` + `make dock-drops-capture`;
  `docs/captures/t14-dock-drops.png`; `docs/captures/README.md` T-14.7f
  paragraph.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore"` — green (`tst_dock`
  148, `tst_dockcore` 84).
- `ctest --test-dir build --output-on-failure` — 53/53.
- `make e2e` — green (incl. `client_drag_and_drop_reaches_a_chrome_surface`;
  the compositor is untouched, so the target-half conformance is unchanged).
- Live: `make dock-drops-capture` (host Wayland + spectacle + gdbus + Pillow).

Gotchas for later tasks:

- **The payload is read at enter and cached; the drop must not re-receive.**
  `ensureDndPayloadReady()` is the only drop-time completion path; use
  `parseDockDropPayload` for any new classification so enter/drop cannot drift.
- **`beginDndRead` prefers `text/uri-list` over `application/x-dragonfruit-app`**
  (same order the old drop path used), so a `.desktop` uri-list is classified as
  an app alias once parsed.
- **Identity is resolved shell-side.** QML only ever sees `name`/`iconPath`/
  count; do not add identity lookups to QML.
- **`DF_DOCK_DROP_FIXTURE` is a capture seam, never a session value.** It drives
  `beginExternalDrag`/`setExternalPayload`/`externalHoverEntry` directly.
- **Duplicate feedback is `flashPin(desktopId)`** (a 700 ms Dock highlight);
  `dockDropActionFor` still returns `PinApp`, the controller decides duplicate.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110g — T-14.7g Dock activation and launch correctness

**State: done.** The Dock click tree is now observable end to end: a launch
always names the compositor socket, an activation is answered, and a missing
identity raises a notice. Protocol `df_toplevel_manager` is **v8**.

Real paths:

- `protocols/dragonfruit-toplevel.xml` — manager version 8; new
  `activation_result(app_id, found)` event (append-only, `since="8"`).
- `compositor/src/shell/mod.rs` — the `ActivateApp` request now sends
  `resource.activation_result(app_id, state.activate_app(&app_id) as u32)`.
- `compositor/tests/shell_protocol_conformance.rs` — `TestClient` collects
  `activation_results`; `dock_click_tree_activation_conformance` asserts a
  found reply for a real window and a not-found reply for `GhostApp`.
- `shell/src/desktopentry.{h,cpp}` — `appLaunchEnvironment(base, waylandDisplay
  = {})`: scrubs offscreen QPA and exports `WAYLAND_DISPLAY` from the shell's
  socket; `XDG_RUNTIME_DIR` inherited.
- `shell/src/shellprotocol.{h,cpp}` — listener `onManagerActivationResult`
  (appended last to the manager listener) + `activationResult(appId, found)`.
- `shell/src/shellcontroller.{h,cpp}` — `m_waylandDisplay`;
  `activateAppOrFallback(appId, desktopId)`; `onActivationResult`; missing
  entries raise `raiseDockNotice`; `launchDetachedFromShell` takes the socket;
  `DF_DOCK_ACTIVATION_FIXTURE` capture seam (`launch`/`activate`/`missing`).
- `shell/dock/DockEntry.qml` — `dragSlop` (8) shared by the left `TapHandler`
  (`gesturePolicy: DragThreshold`) and the `DragHandler`.
- `scripts/capture-dock-activation.sh` + `make dock-activation-capture`;
  `docs/captures/t14-dock-activation.png`,
  `docs/captures/t14-dock-activation-missing.png`; captures README paragraph.
- ADR `docs/design/adr/0101-dock-activation-result-and-launch-display.md`;
  `docs/design/04-shell.md` "Dock activation and launch";
  `docs/design/02-compositor.md`; `docs/private-protocols.md`.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore"` — green (`tst_dock`
  152, `tst_dockcore` 88).
- `cargo test --workspace` — green; `make e2e` — green.
- `cargo clippy --workspace --all-targets`; `cargo fmt --all -- --check`;
  `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh` — green.
- Live: `make dock-activation-capture`.

Gotchas for later tasks:

- **One activation entry point.** Use `ShellController::activateAppOrFallback`
  for anything that activates an app, so the `activation_result` fallback
  stays wired. `m_pendingActivations` maps app id -> desktop id (empty =
  notice, not launch).
- **One launch-env helper.** `appLaunchEnvironment(base, waylandDisplay)` with
  the shell's `m_waylandDisplay`; never `systemEnvironment()` alone.
- **Synthetic-tap limitation.** In the shell's synthetic-injection path a
  stationary pointer tap does not land on a Dock entry that also enables a
  `DragHandler` (app/temporary); Trash/minimized/stack tap fine. The live
  capture therefore uses `DF_DOCK_ACTIVATION_FIXTURE`, which calls the real
  `onDockEntryActivated` path. Follow-up: fix the synthetic tap path and
  re-capture without the seam. **Resolved by T-14.7x**: the injected
  `QMouseEvent` had `timestamp() == 0`, which made `QQuickDragHandler` grab the
  press before the sibling `TapHandler`; `DockPointer` now stamps a monotonic
  timestamp, the `DF_DOCK_ACTIVATION_FIXTURE` seam is deleted, and the
  activation/launch-origin captures drive a real synthetic click (see the
  T110x section).
- **`DF_DOCK_ACTIVATION_FIXTURE` is a capture seam, never a session value.**
- Adding a manager event: append it last in the XML (append-only opcodes) and
  append its callback last in `bindTrustedGlobals`' listener initializer.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110h — T-14.7h Dock folder stacks: presentation and clicks

**State: done.** Folder entries read like macOS: a clean folder silhouette with
no text in the artwork, single click opens the stack popover, double-click
opens the folder in Files, and the popover is a real presented list (header,
separator, scrollable rows, overflow/empty). No new ADR — ADR 0092 already
froze this model; `docs/design/04-shell.md` gains the presentation details.

Real paths:

- `shell/dock/DockGlyph.qml` — the `stack` block is `stackArtwork`: back tab
  (`stackFolderTab`), gradient front face (`stackFolderFront`) with rim + inner
  sheen (`stackFolderSheen`); no `Text`. New color tokens `folderTab`,
  `folderFillTop`, `folderFillBottom`, `folderRim`, `folderHighlight`
  (light+dark) in `design-system/tokens/tokens.json`; `Theme.qml` regenerated.
- `shell/dock/DockEntry.qml` — stack-only `TapHandler` (`stackTapHandler`)
  emits the ordinary `activated`; generic tap/DragHandler stay disabled for
  stacks. Accessible name uses the entry name.
- `shell/dock/Dock.qml` — `handleEntryTap(entry)` resolves single vs double
  click with `stackDoubleClickMs` (400 ms): first tap opens the popover, a
  second tap calls `doubleActivateEntry` (close popover +
  `downloadsFolderRequested`). The timer lives on the Dock, not the delegate,
  because clearing the badge can recreate the delegate between taps.
  `downloadsName` property feeds `stackEntry.name` and the popover title; stack
  menu label is now **Open in Files**.
- `shell/dock/DockStackPopover.qml` — rewritten (`FocusScope`): header with
  `Icon "folder"` + elided name + `stackOpenAction` ("Open in Files"),
  `ScrollView` rows with `Icon "folder"/"file"` + names, `stackOverflowRow`
  ("N more…"), `stackEmptyRow`; keyboard `currentIndex` -1 = header; separator
  uses `Theme.color.border`.
- `shell/src/downloadsmonitor.{h,cpp}` — `displayName()` (basename, "Downloads"
  fallback). `shell/src/shellcontroller.cpp` — pushes `downloadsName`; folder
  open routes through `launchFiles(folder)`; `DF_DOCK_STACK_FIXTURE`
  (`open`/`empty`/`long`) capture seam.
- Tests: `tst_dock.qml` single/double click, context-menu Open in Files,
  header/action, keyboard header reach, rows, empty/overflow/scroll, no
  text-in-artwork; `tst_dockcore.cpp` `downloadsMonitorReportsItsDisplayName`.
- `scripts/capture-dock-folder-stack.sh` + `make dock-folder-stack-capture`;
  `docs/captures/t14-dock-folder-stack.png` (resting/open/empty/long);
  captures README.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore|qmllint_shell-dock"` —
  green (`tst_dock` 160 passed, `tst_dockcore` includes the new name case).
- `ctest --test-dir build --output-on-failure` — 53/53.
- `make e2e` — green; `cargo fmt --all -- --check`; `./scripts/gen-tokens.py
  --check`; `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`.
- Live: `make dock-folder-stack-capture`.

Gotchas for later tasks:

- **One double-click source.** `Dock.handleEntryTap()` owns the
  `stackDoubleClickMs` window (`Date.now()`); the entry delegate only emits
  `activated`. Keep it on the Dock — the delegate can be recreated when the
  badge clears. Synthetic `mouseDoubleClickSequence` does not reach a
  TapHandler offscreen, so tests call `handleEntryTap` twice.
- **Folder name is data, not a literal.** `Dock.downloadsName` /
  `DownloadsMonitor::displayName()`; T-14.7i's hover Tooltip and any T-14.7k
  pinned-folder entry must read the entry `name`, never hardcode "Downloads".
- **Folder open is `launchFiles(folder)`**, not `QDesktopServices` (the one
  reveal path).
- **`DF_DOCK_STACK_FIXTURE` and `XDG_DOWNLOAD_DIR` are capture seams**, never
  session values.
- **ScrollView viewport** is capped at `maxItems`; the overflow row sits below
  it (so a 13-item folder shows 8 rows + "5 more…", and the viewport scrolls).
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110i — T-14.7i Dock hover name labels (Tooltip)

**State: done.** The design system has a passive `Tooltip` and the Dock shows a
hovered entry's name plus state in it. No new ADR — ADR 0093 froze the
component and ADR 0092 the label source; `docs/design/04-shell.md` gains a
"Dock hover name label" subsection.

Real paths:

- `design-system/components/Tooltip.qml` (new) — `open`, `anchorItem`,
  `text`, `placement` (above/below/left/right), `dwell`, `bounds`; one elided
  line; no focus, no keys, no pointer blocking (`focus: false`,
  `enabled: false`, `Accessible.ignored`). Motion reuses
  `Theme.motion.popupOpen`/`popupClose` (reduced = instant). `textElided` is a
  test hook.
- `design-system/tokens/tokens.json` — new `component.tooltip` group: dwell
  600, offset 8, radius `radius.md`, paddingH `spacing.sm`, paddingV
  `spacing.xs`, maxWidth 260, fontSize `sizeSm`; `Theme.qml` and
  `design_tokens.rs` regenerated.
- `design-system/gallery/GalleryContent.qml` — `TooltipPage` (above + elided
  below) and the `"Tooltip"` page appended; `scripts/check-gallery-snapshots.py`
  `PAGES` gains `tooltip`; goldens `tooltip_{light,dark,dark_reduced}.png`.
- `design-system/tests/tst_design_system.qml` — 6 `test_tooltip_*` cases.
- `shell/dock/DockEntry.qml` — `hoverBegan(entryItem)`/`hoverEnded(entryItem)`
  on the `HoverHandler`; readonly `tooltipLabel` = name + state (windows,
  folder count, Trash). The one label source.
- `shell/dock/Dock.qml` — `tooltipEntry`/`tooltipAnchor`/`tooltipOpen`/
  `tooltipDwell`/`tooltipPlacement`/`tooltipText`; `tooltipDwellTimer`;
  `entryHoverBegan`/`entryHoverEnded`/`showTooltip`/`hideTooltip`;
  `tooltipRect` unioned into `popoverRect`; one `Tooltip`
  (`objectName: "dockTooltip"`, `bounds: dock`); `showTooltipFor(kind)` capture
  seam; `tooltipItem()` test hook. Hidden on click, drag, resize, external
  drag, pointer leave, and `popoverOpen`.
- `shell/src/shellcontroller.cpp` — `DF_DOCK_TOOLTIP_FIXTURE` seam
  (`app`/`folder`/`trash`).
- `shell/tests/tst_dock.qml` — 11 `test_tooltip_*` cases (dwell/leave/click/
  popover, state text, folder-name-is-data, follow-magnify, clamp, overlay
  rect, reduced motion).
- `scripts/capture-dock-tooltip.sh` + `make dock-tooltip-capture`;
  `docs/captures/t14-dock-tooltip.png` (app / folder / Trash stacked);
  captures README T-14.7i paragraph.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_design_system"` — green
  (`tst_dock` 170, `tst_design_system` 47).
- `ctest --test-dir build --output-on-failure` — 53/53.
- `./scripts/check-gallery-snapshots.py --strict` — 75 snapshots.
- `make e2e`; `make clippy`; `cargo fmt --all -- --check`;
  `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh` — green.
- Live: `make dock-tooltip-capture` (host Wayland + spectacle + gdbus +
  Pillow).

Gotchas for later tasks:

- **The Tooltip anchor must be a sibling (same parent).** It reads
  `anchorItem.x/y/width/height` directly, not `mapToItem`, so it re-binds and
  follows a magnified entry; `mapToItem` alone does not track live geometry.
- **`DockEntry.tooltipLabel` is the only label source.** App window counts,
  the folder's `downloadsName` + item count, and the Trash state are formatted
  there. T-14.7k must reuse it; never hardcode "Downloads".
- **One Tooltip, Dock-owned dwell/suppression.** `entryHoverBegan` starts
  `tooltipDwellTimer`; `hideTooltip` clears the anchor only when the capsule is
  invisible (or never appeared), gated on `!tooltipDwellTimer.running`, so an
  A→B hover cannot wipe B's anchor.
- **`popoverRect` is now the union** of the open popover and `tooltipRect`.
  Anything reading `popoverRect` gets the label too; a new overlay consumer
  must keep the union.
- **`DF_DOCK_TOOLTIP_FIXTURE` and `showTooltipFor` are capture seams**, never
  session values (synthetic dwell hover does not work offscreen/headless).
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110j — T-14.7j Dock Tahoe visual language: floating glass, squircles, states

**State: done.** The Dock is now a layered floating glass plate with a bright
interior rim, hairline border, and soft shadow; app tiles are token squircles
with a themed-icon inset; hover/press/running states share one spec. The
compositor gives the Dock its own material role, selected by namespace, so its
frost is tunable independently of the menu bar. ADR
`0102-dock-material-role-and-qml-glass-layers.md`.

Real paths:

- `design-system/tokens/tokens.json` — semantic colors `dockFill`, `dockRim`,
  `dockBorder`, `dockShadow`, `dockHoverFill`, `dockDivider`, `dockIndicator`
  (light+dark); semantic material `dockOpacity` (light 0.5 / dark 0.42) and
  `dockBlur` (30 / 34); `component.dock.radius` is now `primitive.radius.xl`
  (20); new nested groups `component.dock.plate` (fillOpacity 0.42, rimOpacity
  0.7, rimHeight 1, borderWidth 1, borderOpacity 0.55, shadowBlur 20,
  shadowOpacity 0.3, shadowOffsetY 6), `.icon` (radiusRatio 0.24, inset 0.06),
  `.hover` (fillOpacity 0.18, radiusRatio 0.28), `.indicator` (opacity 0.9),
  `.divider` (opacity 0.5, width 1, heightRatio 0.6). `Theme.qml` and
  `design_tokens.rs` regenerated.
- `compositor/src/window/backdrop.rs` — `MaterialRole::Dock`,
  `from_layer_namespace(layer, namespace)` (`"dock"` on a persistent surface),
  name `"dock"`, `spec()` reads `DOCK_BLUR`/`DOCK_OPACITY` and
  `component::dock::RADIUS`; new tests `the_dock_namespace_selects_the_dock_material_role`
  and `dock_material_degrades_with_the_shared_tiers`.
- `compositor/src/window/decoration.rs` — `ColorScheme::dock_fill()`.
- `compositor/src/shell/mod.rs` — `ChromeSurface` gains `pub namespace: String`
  (filled from `LayerSurfaceState.namespace`).
- `compositor/src/render.rs` — role now
  `MaterialRole::from_layer_namespace(chrome.layer, &chrome.namespace)`.
- `shell/dock/Dock.qml` — the plate is a clipped `Item` `dockPlate` with
  children `dockBar` (fill), `dockRim` (interior highlight), `dockBorder`,
  `dockPlateShadow` (`Shadow`). `plateX`/`plateY`/`plateW`/`plateH` give child
  placement inside the group; the group clips so the shadow falls only toward
  the anchored edge, never into the magnify band. The rim is orientation-aware:
  horizontal on a bottom Dock, the interior vertical edge on left/right. All
  values from `Theme.controls.dock.plate` / `Theme.color.dock*`.
- `shell/dock/DockEntry.qml` — `tileRadius` (`icon.radiusRatio`) and
  `hoverRadius` (`hover.radiusRatio`); hover wash is `dockHoverFill` at
  `hover.fillOpacity`; placeholder/drop/duplicate/lift-shadow/focus-ring use
  `tileRadius`; indicator is `dockIndicator` at `indicator.opacity`; divider is
  the `divider` tokens.
- `shell/dock/DockGlyph.qml` — `appTile` (`objectName`) at `tileRadius`;
  `rasterIcon`/`vectorIcon` inset by `icon.inset`. The Trash stays its own bin
  (never a squircle).
- `shell/tests/tst_dock.qml` — `test_plate_is_a_layered_glass_not_a_flat_slab`,
  `test_plate_glass_follows_the_color_scheme`,
  `test_glyph_tile_is_a_token_squircle_with_inset`,
  `test_plate_rim_is_on_the_interior_edge`,
  `test_entry_states_use_the_squircle_and_state_tokens`.
- `scripts/capture-dock-tahoe.sh` + `scripts/capture-dock-tahoe-driver.py`,
  `make dock-tahoe-capture`; `docs/captures/t14-dock-tahoe-{light,dark,reduced}.png`;
  captures README paragraph.
- Docs: `docs/design/04-shell.md` "Dock plate and materials",
  `docs/design/02-compositor.md`, `docs/design/10-design-system.md`.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_design_system|qmllint_shell-dock"` —
  green (`tst_dock` 175).
- `ctest --test-dir build --output-on-failure` — 53/53.
- `cargo test --workspace`; `make e2e` — green.
- `cargo clippy --workspace --all-targets`; `cargo fmt --all -- --check`;
  `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-tahoe-capture` (host Wayland + spectacle + Pillow).

Gotchas for later tasks:

- **Read `plateRect` for geometry, `dockPlate.plateX/plateY` for child
  placement.** The plate group is clipped and offset differently per position;
  `dockBar`/`dockRim`/`dockBorder`/`dockPlateShadow` live at `plateX/plateY`,
  not `0,0` (except a bottom Dock). The bottom-Dock `plateX` is 0.
- **The rim is orientation-aware.** Never assume a top bar; a left/right Dock
  puts it on the interior vertical edge. Test `test_plate_rim_is_on_the_interior_edge`.
- **No degrade-tier signal reaches QML.** The QML always draws fill/rim/border/
  shadow; the tier only changes the compositor frost (`Reduced` scales it,
  `Minimal` drops it). T-14.7k must not expect a QML "minimal" switch; a shell
  tier binding is a future item (ADR 0102).
- **Themed artwork is inset-and-fitted, not per-pixel squircle-masked.** A
  software-renderer-safe mask needs a GPU pass, deferred with refraction;
  placeholder/hover/focus/badges carry the squircle geometry. `appTile` is the
  placeholder `objectName`.
- **`check-desktop-names.sh` still fails on the pre-existing
  StatusNotifier/zoo lines** (unchanged here).

## T110k — T-14.7k Dock folder pins: any folder as a stack

**State: done.** Any folder can be pinned to the Dock as a stack: a single
folder dropped on the app region pins it, it lists through files-core, opens
its popover listing, opens in Files on double-click / context menu, moves files
into it on drop, and is removed by dragging the tile off the Dock or the context
menu's Remove from Dock. The Downloads stack is the built-in first member of the
same widget and one model. ADR `0104-dock-folder-pins-and-arbitrary-stack-owner.md`.

Real paths:

- `services/settingsd/src/schema.rs` — `dock.pinnedFolders` (`as`, default
  empty, since 7, owner/consumer shell/Dock); `SCHEMA_VERSION` is now 7.
  `docs/settings-keys.md` row added.
- `shell/src/folderstacks.{h,cpp}` (new) — `FolderStacks` (ordered paths →
  `{ path, name, items, count, badge, missing }` maps) lists each folder through
  `files_core_list.h` (`df_files_begin`/`df_files_poll`, the chooser-bridge
  poll), watches with `QFileSystemWatcher` for change notify only, and tracks a
  per-folder new-items badge; `moveFilesIntoFolder` is the one move helper
  (copy+remove across filesystems, `name.N` de-dup);
  `folderDisplayName` is the basename/`Downloads` fallback.
- `shell/src/downloadsmonitor.{h,cpp}` — **deleted**; the shell now lists all
  folder stacks through `FolderStacks`. Its tests moved to `FolderStacks`.
- `shell/src/dockdrops.{h,cpp}` — `DockDropPayload::Folder`, `uriListIsFolder`
  (absolute + existing dir), `DockDropAction::PinFolder`/`MoveToFolder`
  (`MoveToDownloads` renamed), folder affordances ("Pin Folder", "Move to
  <name>").
- `shell/src/dockmodel.{h,cpp}` — `DockConfig::pinnedFolders` read from
  `dock.pinnedFolders`.
- `shell/src/shellcontroller.cpp` — `refreshFolderStackData()` projects the
  Downloads member onto `downloads*` (+ new `downloadsPath`) and the other paths
  onto `folderPins`; `onDockFolderOpenRequested`/`onDockFolderViewed`/
  `onDockFolderPinRemoved`; drop handling pins a folder, moves into the target
  stack path (`folder:<path>` / `__downloads__`), and derives the folder payload
  from the file list (`uriListIsFolder`). `DF_DOCK_FOLDER_PIN_FIXTURE`
  (`<path>` or `<path>:open`) capture seam.
- `shell/dock/Dock.qml` — `folderPins`/`folderEntries`/`stackEntryRef`,
  `openStackFor(entry)` + `openStackById(id)`, per-stack popover
  (`stackPopoverItems`/`stackPopoverTitle`), folder menu (Open in Files +
  Remove from Dock when `canRemove`), drag-out removal (`dragIsFolderPin`),
  external folder gap/affordance.
- `shell/dock/DockEntry.qml` — `isExternalFolder` (stack silhouette for a
  dropped folder), `canRemoveStack`, DragHandler enabled for removable stack
  pins.
- Tests: `tst_dockcore` — folder payload classification, `dockDropActionFor`/
  `dockDropAffordance` folder rows, `FolderStacks` list/badge/missing/dedupe,
  `moveFilesIntoFolder`, `folderDisplayName`. `tst_dock.qml` — 6
  `test_folder_pin_*` cases.
- `scripts/capture-dock-folder-pin.sh` + `make dock-folder-pin-capture`;
  `docs/captures/t14-dock-folder-pin.png` (pinned / open); captures README.
- Docs: ADR 0104, `docs/design/04-shell.md` "Dock folders and stacks" +
  drop-affordance paragraph.

Commands that work (repo root):

- `cargo test -p dragonfruit-settingsd`; `cargo fmt --all -- --check`;
  `cargo clippy -p dragonfruit-settingsd --all-targets` — green.
- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 181,
  `tst_dockcore` includes the new folder cases).
- `make e2e` — green (Rust workspace suites + `make demo --headless`).
- `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh` — green.
- Live: `make dock-folder-pin-capture` (host Wayland + spectacle + gdbus +
  Pillow).

Gotchas for later tasks:

- **One listing owner: `FolderStacks`.** Never add a `QDir` reader to the Dock.
  The `QFileSystemWatcher` is a notify seam only; when files-core exposes its
  `FolderWatcher` over the C ABI, `FolderStacks` must consume it and drop the
  Qt watcher (ADR 0104).
- **`dock.pinnedFolders` is separate from `dock.pinned`.** Folder pins must not
  be folded into the app-pin list; the Downloads path is never stored in the
  key (it is the always-present default member).
- **Downloads is not removable in this unit.** `canRemove` is true only for
  `dock.pinnedFolders` entries; `stackEntry` has `canRemove: false`.
- **Folder pin ids are `folder:<absolute path>`**; the shell resolves a stack
  drop target from the id (`__downloads__` → `downloadsDirectory()`,
  `folder:<path>` → the path).
- **`computeDockOverflow` still counts `fixedCount = 2`** (Downloads + Trash),
  so a very large number of folder pins is not included in the clamp math. A
  later task should pass the live folder-pin count.
- **`DF_DOCK_FOLDER_PIN_FIXTURE` and `openStackById` are capture seams**, never
  session values.
- Live drag-from-Files was **not** exercised end to end; the capture used the
  fixture's settingsd write, which is the production persistence path. Verify a
  real nested drag in the human sign-off.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110l — T-14.7l Dock launch-origin tile hand-off

**State: done.** The Dock now hands the acted-on entry's icon tile to the
compositor before a launch (`df_toplevel_manager.set_launch_origin`, v4), so a
launched window's appear — and the remembered minimize/restore — originates at
the real icon instead of the centered fallback. The protocol, compositor store,
and motion path were already present; this unit added only the shell/QML
plumbing. ADR `0105-dock-launch-origin-tile-handoff.md`.

Real paths:

- `shell/src/dockmodel.{h,cpp}` — `dockLaunchAppId(entry)` (`StartupWMClass`
  trimmed, else the desktop id without `.desktop`); pure `DockTileRects`
  (capacity 32, FIFO eviction, in-place refresh, ignores non-positive rects);
  `clampDockTileRect(rect, output)`.
- `shell/src/shellprotocol.{h,cpp}` — `df_output.geometry` is now stored
  (`OutputInfo.x/y/geometryWidth/geometryHeight`); new
  `primaryOutputGeometry()` returns the first announced output's logical rect,
  invalid until one exists. `onOutputGeometry` is no longer a no-op.
- `shell/src/shellcontroller.{h,cpp}` — `m_dockTiles` (`DockTileRects`),
  slot `onDockEntryTileRect` (clamps to the output), `dockSurfaceOutputOrigin()`
  (primary output + Dock edge + `m_dockThickness`), `sendLaunchOrigin(desktopId)`
  called from `launchDockAppWithFiles` before the child is spawned. `renderDock`
  pushes `outputOriginX/Y` onto the Dock item. `m_dockTiles.clear()` on
  `dock.pinned` changes. The `DF_DOCK_ACTIVATION_FIXTURE` `launch`/`activate`
  paths now invoke the Dock's QML `activateEntry` so the tile fires like a real
  click (`missing` still calls the controller directly).
- `shell/dock/Dock.qml` — `signal entryTileRect(desktopId,x,y,w,h)`;
  `outputOriginX/Y`; `tileEntryId`/`tileDesktopId`/`lastTileRect`;
  `launchIdentity`/`entryTileRectFor`/`publishEntryTile`/`republishEntryTile`;
  `onLayoutChanged: republishEntryTile()` (suppressed while
  magnifying/dragging); `publishEntryTile(entry)` in `activateEntry` for every
  non-trash, non-stack entry.
- Tests: `tst_dockcore` — `dockLaunchAppIdPrefersStartupWmClass`,
  `dockLaunchAppIdFallsBackToTheIdStem`, `dockTileRectsKeepTheLastRectPerIdentityAndStayBounded`,
  `clampDockTileRectBoundsToTheOutput`. `tst_dock.qml` —
  `test_activation_reports_the_entry_tile_in_output_coordinates`,
  `test_tile_is_not_reported_for_trash_or_stacks`,
  `test_settled_relayout_republishes_a_moved_tile`.
- `scripts/capture-dock-launch-origin.sh` +
  `scripts/capture-dock-launch-origin-driver.py` +
  `make dock-launch-origin-capture`;
  `docs/captures/t14-dock-launch-origin.png` (annotated appear origin) and
  `t14-dock-launch-origin-trace.txt`; captures README paragraph.
- Docs: ADR 0105, `docs/design/04-shell.md` "Dock activation and launch"
  paragraph, `docs/private-protocols.md` caller note.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore|qmllint_shell-dock"` —
  green (`tst_dock` 183).
- `ctest --test-dir build --output-on-failure` — full suite.
- Live: `make dock-launch-origin-capture` (host Wayland + spectacle + gdbus +
  Pillow). Recorded `appear origin=(848, 1127, 48, 55)` vs
  `target=(560, 280, 800, 640)`: the origin is a bottom-band icon tile, not the
  centered fallback.

Gotchas for later tasks:

- **One tile channel:** `Dock.qml`'s `entryTileRect` signal is the only
  geometry publisher; the shell's `DockTileRects` is a bounded cache (32), not
  a queue. Do not add a second publisher or a continuous stream.
- **The rect is in output coordinates.** The shell derives the layer surface
  origin from `ShellProtocol::primaryOutputGeometry()` + `m_dockThickness`; keep
  `outputOriginX/Y` on the Dock item in sync if the surface geometry model
  changes. Cross-output is deferred (one primary output; rects are clamped).
- **Key derivation is pure and single-sourced** (`dockLaunchAppId`):
  `StartupWMClass` first, else the desktop-id stem. An empty key or missing rect
  is the documented centered fallback; never send a guess.
- **`DF_DOCK_ACTIVATION_FIXTURE` now goes through QML `activateEntry`** for
  `launch`/`activate`, so it publishes the tile. A future capture seam for a
  launch should do the same or the tile will be absent.
- The compositor's own `DfState::dock_tiles` remains bounded at 64 and is
  independent of the shell cache.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110m — T-14.7m Dock window chooser: per-window actions

**State: done.** Each window row in the Dock's chooser now carries a stateful
**Minimize / Restore** and a destructive **Close**, so a grouped app is managed
from its popover without focusing each window. The chooser asks, never mutates:
the compositor round-trip changes the projection and the next projection
refreshes the rows; the chooser stays open (a close drops the row, a minimize
flips the button) and dismisses when the app's last window closes. No new ADR:
ADR 0103 already decides the chooser is first-class with per-window actions.

Real paths:

- `shell/src/shellprotocol.{h,cpp}` — `setToplevelMinimized(windowId, bool)`
  (true → `df_toplevel.minimize`, false → `unminimize`). No protocol change.
- `shell/src/shellcontroller.{h,cpp}` — slots
  `onDockWindowCloseRequested(QString)` /
  `onDockWindowMinimizeRequested(QString,bool)`, wired to the Dock signals;
  both `scheduleDockRender()` and leave the popover open. New
  `DF_DOCK_CHOOSER_FIXTURE` seam: a 500 ms retry that calls
  `Dock.openChooserFixture()` until a running multi-window app exists.
- `shell/dock/DockWindowChooser.qml` — `windowCloseRequested` /
  `windowMinimizeRequested` signals, `minimizeWindow(index)` /`closeWindow(index)`
  (no `hide()`), `windowAccessibleName(title,focused,minimized)`; the
  `ChooserActionButton` inline component (icon-only, `activeFocusOnTab:false`,
  danger tint for Close); the reserved `chooserRowActions` slot;
  `fixtureHoverIndex`. Row background now keys off `row.hovered`.
- `shell/dock/Dock.qml` — signals `windowCloseRequested` /
  `windowMinimizeRequested`; `onEntriesChanged:
  Qt.callLater(refreshChooserAfterProjection)`; `refreshChooserAfterProjection()`
  re-resolves `chooserEntry`/`chooserAnchor` by id (dismisses on no match);
  `openChooserFixture()`.
- `design-system/components/Icon.qml` — new `restore` glyph (up arrow).
- `shell/tests/tst_dock.qml` — `windowCloseSpy`/`windowMinimizeSpy` and five
  new cases (close stays open, stateful label, destructive + not tab-focusable,
  rows update + dismiss on last close, accessible name state).
- `compositor/tests/shell_protocol_conformance.rs` —
  `toplevel_handle_requests_round_trip` now asserts `df_toplevel.close` reaches
  the client (`toplevel_close_requests`).
- `scripts/capture-dock-chooser-actions.sh` + `make
  dock-chooser-actions-capture`; `docs/captures/t14-dock-chooser-actions.png`
  (light over dark) plus `-light`/`-dark`; captures README paragraph.
- Docs: `docs/design/04-shell.md` "Dock window chooser".

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 189).
- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance` —
  35/35 (needs `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
  `RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`,
  `LD_LIBRARY_PATH=$HOME/.local/df-toolchain/usr/lib64`).
- `cargo fmt --all -- --check`; `cargo clippy -p dragonfruit-compositor
  --all-targets -- -D warnings`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-chooser-actions-capture` (host Wayland + spectacle + gdbus +
  Pillow; launches a second `dragonfruit-settings` via `--launch` so the entry
  groups).

Gotchas for later tasks:

- **The chooser survives a projection with `Qt.callLater`.** Rebind the entry
  and anchor only on the next event-loop turn: inside `onEntriesChanged` the
  derived `items` binding and the entry Repeater still hold the pre-change
  values (a synchronous refresh silently rebinds the old entry).
- **Actions ask; `windowList` is read-only.** `minimizeWindow`/`closeWindow`
  emit and return — they never mutate the model or hide the popover.
- **One wrapper, one state.** `setToplevelMinimized` sends the requested state,
  not a shell-side toggle; `selectToplevel` (activate) still restores + focuses.
- **The action slot is reserved** (`2 * actionButtonSize + actionGap`); a
  T-14.7n scrollbar/badge must not steal it or the popover resizes on hover.
- **`fixtureHoverIndex` / `DF_DOCK_CHOOSER_FIXTURE` are capture seams**, never
  session state. `row.hovered` (pointer or seam) is the single reveal source.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110n — T-14.7n Dock window chooser: row discipline

**State: done.** The window chooser's row list is now bounded: at most
`component.dock.chooser.maxRows` (default 7) rows render, a longer list
flick-scrolls through the design-system `ScrollView`, and the "Show All
Windows" header + separator stay pinned above it. A short list is unchanged
(no scrollbar, no gutter). A scrolling list reserves the scrollbar's width to
the right of the T-14.7m action slot, so the thumb never covers
Minimize/Close. Up/Down/Home/End/Return navigate and scroll the focused row
into view. No new ADR: ADR 0103 already decides the bounded viewport and
ADR 0100's pre-sized buffer still holds.

Real paths:

- `design-system/tokens/tokens.json` — `component.dock.chooser`:
  `maxRows` 7, `scrollbarWidth`/`scrollbarMargin` `$ref`'d from
  `component.scrollView`. `Theme.qml` + `compositor/src/design_tokens.rs`
  regenerated (`./scripts/gen-tokens.py`).
- `shell/dock/DockWindowChooser.qml` — root `Item` → `FocusScope`;
  `maxRows`, `currentIndex`, `visibleRows`, `listHeight`, `scrolls`,
  `scrollbarGutter`; `moveSelection`/`moveSelectionTo`/`keepSelectionVisible`/
  `activateCurrent`; the rows `Column` is inside a `ScrollView`
  (`objectName: "chooserRows"`, `interactive`/`scrollbarVisible: scrolls`);
  `actionSlot.rightMargin = contextMenu.padding + scrollbarGutter`; row
  `active` highlight and `showActions` include the keyboard highlight.
- `shell/dock/Dock.qml` — `scrollChooserFixture()` capture seam.
- `shell/src/shellcontroller.cpp` — `DF_DOCK_CHOOSER_FIXTURE=scroll` opens the
  chooser then scrolls it ~300 ms later; other values keep the T-14.7m path.
- `shell/tests/tst_dock.qml` — 7 new `test_chooser_*` cases (token cap,
  short-list geometry, action gutter, header pinning, keyboard
  scroll-into-view, live keys, Enter/Escape). `tst_dock` 189 → 196.
- `scripts/capture-dock-chooser-scroll.sh` + `make
  dock-chooser-scroll-capture`; `docs/captures/t14-dock-chooser-scroll.png`
  (+ `-light`/`-dark`); captures README paragraph.
- Docs: `docs/design/04-shell.md` "Dock window chooser".

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 196).
- `make e2e` — green (Rust workspace suites + `make demo --headless`).
- `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py` — green.
- Live: `make dock-chooser-scroll-capture` (host Wayland + spectacle + gdbus +
  Pillow; 12 Settings windows via repeated `--launch`).

Gotchas for later tasks:

- **The chooser height is now capped, not content-sized.** Any T-14.7o badge or
  T-14.7q overflow work must not push the viewport past `maxRows` or it will
  grow the popover past ADR 0100's headroom.
- **`chooserRows` is the `ScrollView`**; `chooserRows.flickable` is the
  scroll-position seam. `currentIndex` (-1 = the pinned header) is the single
  keyboard highlight.
- **The gutter is conditional** (`scrollbarGutter` is 0 when short); never
  hard-code the scrollbar width into the row right margin.
- **`DF_DOCK_CHOOSER_FIXTURE=scroll`** is the long-list capture seam;
  `scrollChooserFixture()` is a fixture, never session state.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here).

## T110o — T-14.7o Dock window-count badge

**State: done.** A grouped app now shows a count at its icon's top-right corner
(`2` for two windows, capped at `9+`); zero/one-window and stopped apps show
nothing. The count lives in the pure model. No new ADR: ADR 0103 already
decides the entry gains a window-count badge.

Real paths:

- `shell/src/dockprojection.{h,cpp}` — new `dockWindowCount(entry)` (prefers
  `windowList` length, falls back to the `windows` scalar, else 0);
  `buildDockProjection` inserts `windowCount` on app and minimized entries.
- `shell/src/dockmodel.cpp` — `buildDockEntries` sets `windowCount` on pinned,
  temporary, and recent entries; minimized entries carry it from the projection.
- `shell/dock/DockEntry.qml` — `windowCount`, `isMinimized`, `showStatusBadge`,
  `showWindowBadge`, `windowBadgeSize`/`Label`/`Color`; the `windowBadge`
  Rectangle + `windowBadgeText`. Precedence: status > app `badge` > window
  count. Stacks/Trash/dividers/external/minimized rows never badge. Normal fill
  `Theme.color.accent`, attention `Theme.color.danger`, numeral
  `accentContent`, `chrome` hairline.
- `design-system/tokens/tokens.json` — `component.dock.windowBadge`
  (`sizeRatio` 0.34, `sizeMin` 14, `sizeMax` 22, `fontRatio` 0.56, `fontMin`
  = `primitive.font.sizeXs`, `paddingH` 5, `inset` 2, `borderWidth` 1);
  `Theme.qml` + `compositor/src/design_tokens.rs` regenerated.
- Tests: `tst_dockcore` — `projectionExposesTheWindowCount`,
  `windowCountDerivesFromTheWindowListOrTheScalar` + merge assertions;
  `tst_dock.qml` — `windowedEntry` helper and 8 badge cases. `tst_dock`
  196 → 204.
- `scripts/capture-dock-window-badge.sh` + `make dock-window-badge-capture`;
  `docs/captures/t14-dock-window-badge-{light,dark,min,max}.png` and the
  light-over-dark composite; captures README paragraph.
- Docs: `docs/design/04-shell.md` "Dock window-count badge".

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 204).
- `make e2e` — green (Rust workspace suites + `make demo --headless`).
- `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py` — green.
- Live: `make dock-window-badge-capture` (host Wayland + spectacle + gdbus +
  Pillow; Settings grouped via one `--launch` plus a single Files window).

Gotchas for later tasks:

- **`windowCount` is the field; do not re-derive from `windowList` in QML.**
  The QML fallback is only for direct test/fixture callers.
- **`showStatusBadge` is the single status condition**; extend precedence in
  `showWindowBadge`, not the status Rectangle.
- **Minimized per-window rows never badge** (`isMinimized`): they carry the
  app's full `windowList` for the menu but represent one window.
- **`windowBadge` size scales with the tile**, clamped to `sizeMin`/`sizeMax`;
  T-14.7p overflow work must keep it inside the tile at `iconSizeMin`.
- `check-desktop-names.sh` still fails on the pre-existing StatusNotifier/zoo
  lines (unchanged here); `make check` stops there.

## T110p — T-14.7p Dock hover-open, retargetable chooser and stable anchor

**State: done.** `dock.chooserOnHover` (bool, default off) opts into a
dwell-open window chooser: hovering a grouped running entry (`> 1` window, not
a minimized row) for `component.dock.chooser.hoverDwell` (280 ms) opens the
popover without a click, and dwelling on another grouped entry retargets the
same popover once its own dwell elapses — no close/reopen. Pointer leave starts
`hoverCloseDelay` (260 ms); releasing onto an entry with `<= 1` window closes it.
Drag, resize, keyboard navigation, and any other popover suppress/dismiss the
hover path; a hover-open chooser suppresses the name label. The popover anchors
to `chooserAnchorProxy`, a non-visual snapshot `Item` in `Dock.qml`, not the
live delegate, so a pin-reorder Repeater rebuild cannot orphan it. No new ADR
(ADR 0103/0100 already decide the chooser and its buffer).

Real paths:

- `services/settingsd/src/schema.rs` — `dock.chooserOnHover` (`since: 8`),
  `SCHEMA_VERSION` 7 → 8; a declaration test. `docs/settings-keys.md` row.
- `libs/settings-client/settingsclient.cpp` — seeds `dock.chooserOnHover` false.
- `shell/src/dockmodel.{h,cpp}` — `DockConfig.chooserOnHover` +
  `dockConfigFromValues` read.
- `shell/src/shellcontroller.cpp` — `applyDockSettings` sets the QML property;
  new `DF_DOCK_HOVER_FIXTURE` (`open`/`retarget`) capture seam.
- `apps/settings/DesktopDockPane.qml` — "Open a window chooser by hovering"
  toggle + `chooserOnHoverToggle` alias + Binding; two pane suites' row count
  10 → 11.
- `design-system/tokens/tokens.json` — `component.dock.chooser.hoverDwell` 280,
  `hoverCloseDelay` 260; `Theme.qml` + `compositor/src/design_tokens.rs`
  regenerated.
- `shell/dock/Dock.qml` — `chooserOnHover`; `chooserHoverDwell`/
  `chooserHoverCloseDelay`; `chooserAnchorProxy` (`capture`/`reposition`);
  `chooserHoverTimer`/`chooserCloseTimer`; `chooserEligible`, `stopChooserHover`,
  `openChooserOnHover`, `closeHoverChooser`, `retargetChooser`,
  `hoverChooserFixture`; `openChooser(entry, fromHover)`; `entryHoverBegan`/
  `entryHoverEnded` reworked; `refreshChooserAfterProjection` re-captures;
  `beginKeyboardNavigation`/`closePopovers`/`handleEntryTap`/Dock-leave stop it;
  `windowChooser.onClosed` resets hover state.
- `shell/tests/tst_dock.qml` — 8 `test_chooser_hover_*` /
  `test_chooser_anchor_survives_a_delegate_rebuild`. `tst_dock` 204 → 212.
- `shell/tests/tst_dockcore.cpp` — config default + `chooserOnHover` override.
- `scripts/capture-dock-hover-chooser.sh` + `make dock-hover-chooser-capture`;
  `docs/captures/t14-dock-hover-chooser{,-light,-dark}.png`; captures README.
- Docs: `docs/design/04-shell.md` "Dock window chooser" hover-open paragraph.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 212).
- `make e2e` — green.
- `cargo test -p dragonfruit-settingsd`; `cargo clippy -p dragonfruit-settingsd
  --all-targets -- -D warnings`; `cargo fmt --all -- --check`;
  `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`; `./scripts/check-gallery-snapshots.py
  --strict` — green.
- Live: `make dock-hover-chooser-capture`.

Gotchas for later tasks:

- **The anchor is `chooserAnchorProxy`, a snapshot, never the delegate.**
  `chooserAnchor` must stay the proxy; re-capture via `capture(item)` on open,
  retarget, and after every projection rebuild, then `reposition()`.
- **`chooserHoverOpened` distinguishes hover from click.** Only a hover-opened
  chooser is closed by a pointer leave; a click-opened one persists until an
  explicit dismiss. `windowChooser.onClosed` resets both timers + flags.
- **Hover diversion lives in `entryHoverBegan`.** A grouped entry starts the
  chooser dwell and never the Tooltip; `entryHoverEnded` only restarts the
  close delay (the next `entryHoverBegan` stops it) — do not clear the target
  there or retarget flickers.
- **T-14.7q overflow work** must not grow the popover past ADR 0100's headroom;
  the hover path adds no surface, only a geometry proxy.
- `check-desktop-names.sh` still fails on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110q — T-14.7q Dock overflow cell and More Windows popover

**State: done.** When the running groups do not fit even at the minimum icon,
they are no longer dropped silently (this supersedes legacy T-10 §5.1 for the
running-groups case only; ADR 0103 already records it). The pure planner folds
them into one terminal `kind: "overflow"` app-region entry — the last item
before the divider, never reorderable or pinnable — and the Dock renders a grid
glyph with the hidden-group count badge and opens a "More Windows" list
anchored to the cell. Choosing a single-window group activates its window;
choosing a multi-window group opens the T-14.7m chooser anchored to the cell.
The icon-size clamp remains the outer fallback when even one cell cannot fit
(then the legacy error state stands). No new ADR.

Real paths:

- `shell/src/dockmodel.{h,cpp}` — `applyDockOverflow` now returns a terminal
  overflow entry (`{ id: "__overflow__", kind: "overflow", hiddenCount,
  windowCount, groups: [...] }`; each group carries `overflowGroup: true`);
  `DockOverflowResult.overflowShown`. Pinned/minimized/fixed never in the
  overflow set; recents still dropped silently. Legacy "drop the tail" is the
  `capacity == 0` / `maxVisible < nonDroppable` fallback.
- `shell/src/shellcontroller.{h,cpp}` — `m_dockOverflowShown` participates in
  the Repeater-reset decision; `warnDockOverflow` wording; the
  `DF_DOCK_OVERFLOW_FIXTURE=1` capture seam (8 synthetic groups + a 480 px
  effective-axis cap).
- `shell/dock/DockGlyph.qml` — `kind: "overflow"` grid-of-windows artwork.
- `shell/dock/DockEntry.qml` — `isOverflow`, `overflowCount`,
  `showWindowBadge`/`windowBadgeLabel` overflow branch, accessible name
  "N more window groups", tooltip, drag-handler exclusion.
- `shell/dock/DockOverflowPopover.qml` (new) — the bounded "More Windows" list
  (`component.dock.overflow.maxRows`), signals `groupActivated`.
- `shell/dock/Dock.qml` — `appEntries` includes `overflow`; `openOverflow`,
  `activateOverflowGroup`, `openOverflowGroupChooser`,
  `refreshOverflowAfterProjection`, `refreshPopoversAfterProjection`,
  `openOverflowFixture`; `popoverOpen`/`activePopoverRect`/`closePopovers`
  include the list; `isDraggable`/`openEntryMenu`/`publishEntryTile`/
  `externalInsertionIndex` treat the cell as inert.
- `design-system/tokens/tokens.json` — `component.dock.overflow`
  (`maxRows`, `gridInset`, `gridGap`, `gridCell`); `Theme.qml` +
  `compositor/src/design_tokens.rs` regenerated.
- Tests: `tst_dockcore` — overflow cell carries the hidden groups, pinned
  exclusion, last-app-region placement, no-cell fallback, fits-alone; the
  existing `overflowHidesRecentsBeforeTemporaries` repurposed. `tst_dockcore`
  98 → 103. `tst_dock.qml` — 8 `test_overflow_*` cases. `tst_dock` 212 → 220.
- `scripts/capture-dock-overflow.sh` + `make dock-overflow-capture`;
  `docs/captures/t14-dock-overflow-{light,dark}.png` + the composite.
- Docs: `docs/design/04-shell.md` "Dock overflow cell"; captures README.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 220,
  `tst_dockcore` 103).
- `make e2e` — green.
- `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-overflow-capture` (host Wayland + spectacle + gdbus +
  Pillow).

Gotchas for later tasks:

- **`hiddenTemporary` now counts the groups in the cell**, which is one more
  than the groups that would have been dropped, because the cell itself costs a
  physical slot. `overflowShown` is the cell's count (0 when no cell).
- **Do not re-derive `groups` in QML**; the pure planner owns the overflow set
  and its order. The Dock only renders and routes.
- **A chooser opened from the list is bound to an `overflowGroup` entry**, not a
  live delegate: `refreshChooserAfterProjection` re-resolves it from the fresh
  overflow entry and keeps the anchor on `chooserAnchorProxy`.
- **The overflow entry must stay the last app-region item before the divider**;
  `externalInsertionIndex` breaks on it so an external drop never inserts after
  it.
- `DF_DOCK_OVERFLOW_FIXTURE` is a capture seam, never session state.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110r — T-14.7r Dock Trash empty progress and result

**State: done.** Empty Trash is asynchronous with visible states. Confirming
Empty Trash (the context-menu confirmation stays the entry point) closes the
menu and opens a progress/result popover anchored to the Trash entry: a delayed
indeterminate busy ring, then a check + removed count, or the message + **Try
Again**. The shell UI thread never blocks. No new persistent state, no polling;
the result is a one-shot signal. ADR 0106 records the seam.

Real paths:

- `shell/src/trashbridge.{h,cpp}` — `EmptyState {Idle,Emptying,Succeeded,Failed}`;
  `emptyAsync()` (one-shot worker, `false` when already emptying, immediate
  failure when the monitor is absent); `emptyStarted`/`emptyFinished(ok,removed,
  error)` one-shot signals; `emptyState`/`isEmptying`/`emptyRemoved`/
  `emptyError`; `resetEmptyState()`; `joinEmptyWorker()` joins on start/stop/
  destruct so the monitor is never freed mid-op. Synchronous `empty()` kept.
- `shell/dock/DockTrashEmptyPopover.qml` (new) — the view: `phase`, `busyVisible`,
  `removedCount`, `errorMessage`, `retryRequested`; the spinner
  (`QtQuick.Shapes` `PathAngleArc`, static under reduced motion), success check,
  failure mark + `Button` Try Again; `Accessible.announce` on every phase change;
  focus moves to Try Again on failure. `Dock.qml` registers it.
- `shell/dock/Dock.qml` — `trashEmptyPhase`/`trashBusyVisible`/`trashEmptyRemoved`/
  `trashEmptyError`/`trashEmptyOpen`/`trashEmptyAnchor`; `beginTrashEmpty`,
  `handleTrashEmptyResult`, `retryTrashEmpty`, `resetTrashEmptyState`,
  `announceTrashEmpty`, `trashEmptyFixture`; the menu trigger calls
  `beginTrashEmpty()` then emits `empty_trash`; `popoverOpen`/`activePopoverRect`
  include the popover; `closePopovers()` leaves an in-flight empty alone;
  `trashBusyDelayTimer`.
- `shell/src/shellcontroller.{h,cpp}` — connects `TrashBridge::emptyFinished`
  to `onTrashEmptyFinished` (forwards to `handleTrashEmptyResult` via
  `QMetaObject::invokeMethod`); the `empty_trash` action calls `emptyAsync()`;
  the `DF_DOCK_TRASH_EMPTY_FIXTURE=busy|success|failed` capture seam.
- `design-system/tokens/tokens.json` — `component.dock.trashEmpty`
  (`busyDelay` 350, `spinnerSize` 18, `spinnerStroke` 2, `spinnerSpeed` 900);
  `Theme.qml` regenerated (`design_tokens.rs` carries no component tokens).
- Tests: `tst_dockcore` — `trashBridgeAsyncEmptyReportsTheRemovedCount`,
  `trashBridgeSecondEmptyWhileEmptyingIsIgnored`,
  `trashBridgeEmptyOnAnAbsentBackendFailsWithoutHanging`; 103 → 106.
  `tst_dock.qml` — 7 `test_empty_trash_*` cases (confirmation opens progress,
  delayed busy, fast-success skip, check + count, failure + Try Again + focus,
  second-request ignored, reduced-motion static ring, result-after-dismiss
  dropped); `tst_dock` 220 → 227.
- `scripts/capture-dock-trash-empty.sh` + `make dock-trash-empty-capture`;
  `docs/captures/t14-dock-trash-empty.png` (busy over success composite) and
  `t14-dock-trash-empty-{busy,success}-{light,dark}.png`; captures README.
- Docs: `docs/design/04-shell.md` "Emptying the Trash";
  `docs/design/adr/0106-async-trash-empty-seam.md`.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 227,
  `tst_dockcore` 106).
- `make e2e` — green.
- `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-trash-empty-capture` (host Wayland + spectacle + gdbus +
  Pillow). Composite inspected: busy = spinner + "Emptying the Trash…",
  success = check + "3 items removed".

Gotchas for later tasks:

- **`handleTrashEmptyResult(ok, removed, error)` is the only shell→Dock result
  path** (invoked by name from `onTrashEmptyFinished`). It drops the result when
  `trashEmptyOpen` is false, so a late result after dismiss cannot resurrect the
  popover.
- **One operation at a time lives in the bridge** (`emptyAsync` returns false
  while `Emptying`), not in QML; `retryTrashEmpty` also guards on the Dock's
  phase.
- **The busy indicator is gated by `trashBusyVisible`**, set by
  `trashBusyDelayTimer` (`component.dock.trashEmpty.busyDelay`). A fast empty
  clears it via `handleTrashEmptyResult` before the timer fires — do not show the
  spinner directly from `phase`.
- **`closePopovers()` skips `trashEmptyPopover.hide()` while emptying** so a
  stray dismiss cannot hide an in-flight operation.
- **The worker is a one-shot `std::thread`**, separate from the watch worker
  (which stays blocked on `df_files_trash_monitor_wait`); `joinEmptyWorker()` is
  called before starting a new one and in the destructor, so the monitor is
  never freed mid-op.
- **Deferred:** determinate `N of M` progress needs a files-core progress seam
  (a count/callback streamed from the empty op) before the popover can show it;
  the indeterminate ring is the current fallback.
- `DF_DOCK_TRASH_EMPTY_FIXTURE` is a capture seam, never session state.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110s — T-14.7s Dock minimize-to-icon reaction

**State: done.** With `dock.minimizeReaction` on, a window entering `minimized`
gives its app entry one short acknowledgment hop — vertical (upward) on a bottom
Dock, cross-axis toward the interior on a left/right Dock. Opt-in, default off,
and removed under reduced motion. The pulse rides the existing `bouncePhases`
map, so the Repeater model is never rebuilt; no new surface and no compositor
protocol. ADR 0107 records the seam.

Real paths:

- `shell/src/dockprojection.{h,cpp}` — pure `dockMinimizedCounts(entries)` and
  `dockMinimizedPulses(entries, previous)`; per-window `minimized` rows are
  skipped so the owning app is not double-counted.
- `shell/src/dockmodel.{h,cpp}` — `DockConfig.minimizeReaction` +
  `dockConfigFromValues` read; `kMinimizeReactionMs = 320` and pure
  `dockMinimizeReactionPhase(elapsed)`.
- `shell/src/shellcontroller.{h,cpp}` — `m_dockMinimizedCounts`,
  `m_dockMinimizedCountsSeeded`, `m_minimizeReactionStart`; new
  `detectDockMinimizePulses()` run from `rebuildDockEntries()`;
  `publishDockBouncePhases()` adds `minimize`/`minimizePhase`; tick expiry + the
  timer stop condition; `applyDockSettings` sets the QML property and clears a
  stale pulse when the key goes off; the
  `DF_DOCK_MINIMIZE_REACTION_FIXTURE=<phase>` capture seam.
- `shell/dock/Dock.qml` — `minimizeReaction` property, `entryMinimizePhase`/
  `entryMinimizeActive`, and the minimize branch at the top of `entryBounce`
  (cross-axis placement already points the hop away from the anchored edge).
- `design-system/tokens/tokens.json` — `component.dock.minimizeReaction`
  (`amplitudeRatio` 0.28, `duration` 320, `reducedDuration` 0);
  `Theme.qml` + `compositor/src/design_tokens.rs` regenerated.
- `services/settingsd/src/schema.rs` — `dock.minimizeReaction` (`since: 9`),
  `SCHEMA_VERSION` 8 → 9; declaration test. `libs/settings-client/
  settingsclient.cpp` seed; `docs/settings-keys.md` row.
- `apps/settings/DesktopDockPane.qml` — toggle + `minimizeReactionToggle` alias
  + Binding; Dock group rows 11 → 12 (both pane suites and
  `tst_settings_absence` updated).
- Tests: `tst_dockcore` 106 → 109 (config default/override, phase, detection,
  coalescing, per-window-row skip); `tst_dock` 227 → 232 (off default, lift
  once + bounds + clear, axis/direction, reduced motion, no model rebuild).
- `scripts/capture-dock-minimize-reaction.sh` + `make
  dock-minimize-reaction-capture`; `docs/captures/t14-dock-minimize-reaction.png`
  (bottom-light over right-dark) + the two individual crops; captures README.
- Docs: `docs/design/04-shell.md` "Dock minimize-to-icon reaction";
  `docs/design/adr/0107-dock-minimize-to-icon-reaction.md`.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 232,
  `tst_dockcore` 109).
- `make e2e` — green.
- `cargo test -p dragonfruit-settingsd`; `cargo clippy -p dragonfruit-settingsd
  --all-targets -- -D warnings`; `cargo fmt --all -- --check`;
  `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-minimize-reaction-capture` (host Wayland + spectacle + gdbus
  + Pillow). Composite inspected: exactly one tile offset per panel, correct
  axis/direction, clean render.

Gotchas for later tasks:

- **`minimize`/`minimizePhase` are separate from `phase`/`attention`** on the
  `bouncePhases` map; do not fold them in. Launch/attention keep their own
  amplitude and the reaction stays independently opt-in.
- **Detection is shell-side** (`dockMinimizedPulses` over the merged entries),
  never QML. The first projection only seeds `m_dockMinimizedCounts`, so a Dock
  that starts with minimized windows does not pulse. Rapid minimizes on one
  entry restart the clock (coalesced into the last).
- **`entryBounce` returns 0 before any branch under `Theme.reducedMotion`**, so
  the reaction needs no separate reduced-motion gate.
- `kMinimizeReactionMs` mirrors `component.dock.minimizeReaction.duration`; the
  shell owns the clock, QML owns amplitude (no QML Behavior).
- **Neighbour lateral ripple is deferred**; if added it needs its own
  token-gated field and must not rebuild the model.
- `DF_DOCK_MINIMIZE_REACTION_FIXTURE` is a capture seam, never session state.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110t — T-14.7t Dock keyboard reordering

**State: done.** The Dock's rearrangement affordance is no longer pointer-only:
with the Dock keyboard-focused, `Ctrl+Shift+Arrow` moves the focused pinned
entry one slot and writes the new full order through the existing
`pinnedOrderChanged` → settingsd single-writer path. Axis-aware (Left/Right
bottom, Up/Down vertical), no-op at the ends, pinned region only, focus
retained for chained presses, move announced with the new position. No new
animation (discrete reorder; correct under reduced motion). No new ADR.

Real paths:

- `shell/src/dockmodel.{h,cpp}` — `QStringList movePinnedEntry(const QStringList
  &pinnedIds, int index, int delta)`; off-region/out-of-range/zero-delta are
  no-ops, never wraps.
- `shell/dock/Dock.qml` — `movePinnedEntry` (QML mirror, same semantics),
  `reorderFocusedPinned(delta)`, `reorderChordHint`, `reorderHintFor(entry)`;
  the `Keys.onPressed` Ctrl+Shift branch ahead of the plain arrows; delegate
  `reorderHint: dock.reorderHintFor(modelData)`.
- `shell/dock/DockEntry.qml` — `property string reorderHint` +
  `Accessible.description: root.reorderHint`.
- `shell/src/shellprotocol.{h,cpp}` — `keyEvent` now carries the modifier mask;
  `m_keyboardModifiers` caches the xkb `depressed` bits from
  `onKeyboardModifiers`.
- `shell/src/shellcontroller.{h,cpp}` — `onKeyEvent(key, pressed, modifiers)` +
  `qtModifiersFromXkb`; the chrome `QKeyEvent` is built with real modifiers
  instead of `Qt::NoModifier`.
- Tests: `tst_dockcore` 109 → 112 (helper bounds/no-op/full list);
  `tst_dock` 232 → 238 (six `test_keyboard_reorder_*`: move + focus,
  axis-awareness, ends no-op, not-focused, pinned-only payload, axis-aware
  pinned-only accessible hint).
- `scripts/capture-dock-keyboard-reorder.sh` + `make
  dock-keyboard-reorder-capture`; `docs/captures/t14-dock-keyboard-reorder.png`.
- Docs: `docs/design/04-shell.md` "Dock keyboard navigation and reordering".

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 238,
  `tst_dockcore` 112).
- `make e2e` — green.
- `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-keyboard-reorder-capture` (host Wayland + synthetic input +
  spectacle + gdbus + Pillow). Verified: persisted `dock.pinned` became
  `[Settings, Files, Konsole, Firefox]` from `[Files, Settings, …]`, and the
  inspection sees the focus ring on the moved 2nd tile with no clipping.

Gotchas for later tasks:

- **The Dock QML mirror must stay in sync with `dockmodel::movePinnedEntry`.**
  There is no C++ singleton in the `Dragonfruit.Dock` module (and
  `tst_dock.qml` loads `Dock {}` standalone), so the pure helper is duplicated.
  If a later task introduces a Dock QML singleton, collapse the mirror.
- **`ShellProtocol::keyEvent` now has a third `uint32_t modifiers` argument and
  `ShellController::onKeyEvent` takes it too.** Modifiers enter the chrome
  exclusively through `qtModifiersFromXkb` (Shift 0x1, Ctrl 0x4, Alt 0x8,
  Super 0x40). Any new chrome chord should read `event.modifiers` on the routed
  `QKeyEvent`; do not add a second modifier-tracking path.
- **The live capture needs the compositor to forward Ctrl+Shift+Arrow to the
  Dock.** `Control+Arrow` are system shortcuts, but the shortcut engine matches
  the modifier set exactly (`shortcut.mods == state`), so Ctrl+Shift+Arrow is
  not intercepted and reaches the focused Dock surface. Do not add exact
  Ctrl+Shift+Arrow system bindings without revisiting the Dock chord.
- **`reorderFocusedPinned` only acts on `kind === "pinned"`;** temporary,
  recent, minimized, stack, overflow, and Trash are inert even when focused.
- **The T-14.7c optimistic no-model-reset follow-up still applies** to the
  keyboard path too (the shell `rebuildDockEntries()` resets the Repeater); it
  was not fixed here.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110u — T-14.7u Dock reference metrics: spacing, plate radius, indicator inset

**State: done.** The resting Dock is retuned to the mature reference capture:
wider tile gaps (14), softer cross-axis padding (15), a rounder plate (radius
28), and the running dot moved into the anchored-edge padding (8 below the
artwork) instead of an additive band. No interaction, material, protocol, or
settings-key change. ADR 0108 records the reconciliation.

Real paths:

- `design-system/tokens/tokens.json` — `component.dock`: `padding` 10 → 15,
  `paddingAlong` 14 → 16, `gap` 8 → 14, `indicatorGap` 3 → 8, `radius` moved
  from `$ref primitive.radius.xl` (20) to the literal `28` (≈0.36 × the resting
  `barThickness`). `iconSize` 48 and `indicatorSize` 4 are unchanged, so the
  plate is now `iconSize + 2 * padding` = 78 px. `Theme.qml` +
  `compositor/src/design_tokens.rs` regenerated; the Rust constant stays
  `component::dock::RADIUS` and still drives the compositor Dock backdrop.
- `shell/dock/Dock.qml` — `barThickness = iconSize + 2 * padding` (the
  indicator is no longer additive); the per-frame `layout` now anchors each
  *artwork* on the anchored-edge side: for a bottom Dock
  `y = plate.bottom - padding - sizes[j]`, for a left Dock
  `x = plate.x + padding - indicatorSpace`, for a right Dock
  `x = plate.x + padding + iconSize - sizes[j]`. `draggedX`/`draggedY` updated
  to the same anchors. Comment block on `indicatorSpace`/`barThickness` rewritten.
- `shell/dock/DockEntry.qml` — unchanged; `artworkX`/`artworkY` and the dot at
  `height - indicatorSize` already place the dot `indicatorGap` below the
  artwork once the entry height is `iconSize + indicatorSpace`.
- `compositor/src/window/backdrop.rs` — the Dock-material unit test now asserts
  `light.radius == component::dock::RADIUS` and `== 28.0` (was 20.0). No code
  change to the compositor.
- Tests: `tst_dock.qml` left/right placement expectations updated (the artwork,
  not the entry box, sits `padding` inside the plate) plus two new cases,
  `test_plate_thickness_and_indicator_inset_match_the_reference` and
  `test_indicator_stays_inside_the_plate_at_icon_size_extremes`; `tst_dock`
  238 → 240, `tst_dockcore` 112 (unchanged). `ctest` 53/53.
- Captures refreshed: `docs/captures/t14-dock-spacing-{bottom,left,right}-{light,dark}.png`
  and `t14-dock-tahoe-{light,dark,reduced}.png`.
- Docs: `docs/design/04-shell.md` "Resting proportions (T-14.7u)";
  `docs/design/adr/0108-dock-reference-metrics-indicator-inset.md`;
  `docs/captures/README.md` measured values; the stale spacing-capture strip
  comment.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 240,
  `tst_dockcore` 112).
- `make e2e` — green.
- `make clippy`; `make fmt-check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-spacing-capture` and `make dock-tahoe-capture` (host Wayland
  + spectacle + gdbus + Pillow). Pixel analysis of
  `t14-dock-spacing-bottom-light.png`: plate 78 px tall, radius ≈28 px (measured
  corner inset 8 px at 8 px below the top edge matches r≈28), artwork top inset
  15 px, dot 4 px with an 8 px gap under the artwork and a 3 px margin to the
  plate edge; no clipping.

Gotchas for later tasks:

- **`component.dock.radius` is the single static plate/backdrop radius** shared
  by the shell plate, the compositor Dock backdrop, and the gallery tooltip
  demo. It is *not* recomputed as `ratio × barThickness`; a size-tracking
  radius would need the compositor to know the panel thickness (deferred,
  T-16.1a). Keep the value static or update all three together.
- **`barThickness` no longer includes `indicatorSpace`.** Any geometry that
  used it to place the dot/entry band must anchor on the artwork edge instead
  (`plate.bottom - padding - size` for the cross axis). `surfaceThickness` is
  159 px at the default size (78 plate + 73 band + 8 edge margin).
- **`indicatorSpace` (12) must stay ≤ `padding` (15)** or an entry's dot clips
  the plate at the anchored edge; the new QML test pins this.
- **The QML test's `dock.indicatorGap` does not exist** — use
  `Theme.controls.dock.indicatorGap`; the Dock only exposes `indicatorSpace`.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110v — T-14.7v Dock region dividers: pinned | temporary/recent | stacks and Trash

**State: done.** The Dock projects the reference's region structure: a rule
between the pinned prefix and the temporary/recent tail, the app | right-region
rule, and a rule between the minimized group and the fixed stacks/Trash tail.
Each rule gets the over-sized divider gap and a near-plate-height hairline.
Only the app | right-region rule keeps the T-10 §5 resize handle. ADR 0109
records the projection.

Real paths:

- `shell/src/dockmodel.{h,cpp}` — `DockRegionPlan` + `planDockRegions(entries,
  fixedCount, minimizedVisible)` (regions `pinned`/`tail`/`minimized`/`fixed`;
  `dividerCount()` = adjacent non-empty pairs). `applyDockOverflow` reserves
  `dividerCount()` dividers in `nonDroppable` and its fit formula.
- `design-system/tokens/tokens.json` — `component.dock.divider`: `gap` 22 added,
  `heightRatio` 0.6 → 0.82; `Theme.qml` + `compositor/src/design_tokens.rs`
  regenerated.
- `shell/dock/Dock.qml` — `dividerGap`; `fixedEntries`; `dividerEntry` (id
  `__divider__`, `resizeHandle: true`), `pinnedDividerEntry`
  (`__divider_pinned__`), `minimizedDividerEntry` (`__divider_minimized__`);
  the region-group `items` builder; `appItemIndices`; `gapBetween(i,j)`;
  divider-aware `_baseline` and `scaledGap`; full-plate-cross-axis divider box
  in `layout`; `plateRect` skips dividers; `externalInsertionIndex` skips rules.
- `shell/dock/DockEntry.qml` — `resizeHandle` (default true when absent);
  `ruleIsVertical` (bottom = vertical rule, left/right = horizontal); centered
  rule; `dividerHit.visible` gated on `resizeHandle`.
- Tests: `tst_dockcore` 112 → 116 (region plan counts, orphan suppression,
  recents/overflow as tail, divider-reserving overflow fit); `tst_dock` 240 →
  248 (divider counts/order for two/one/zero rules, minimized-region rule,
  oversized gap on all positions, hairline spans the plate, gap holds under
  magnification). Updated `test_items_order_and_trash_is_last`,
  `test_minimized_entry_menu_lists_windows_and_quit`,
  `test_tooltip_folder_name_is_data_not_a_literal` for the inserted rules.
- `scripts/capture-dock-dividers.sh` + `make dock-dividers-capture`;
  `docs/captures/t14-dock-dividers-{light,dark}.png` (three-region /
  empty-tail / single-region strips). Captures README entry.
- Docs: `docs/design/04-shell.md` "Region dividers (T-14.7v)";
  `docs/design/adr/0109-dock-region-dividers.md`.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 248,
  `tst_dockcore` 116).
- `make e2e`; `make clippy`; `cargo fmt --all -- --check`;
  `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Live: `make dock-dividers-capture` (host Wayland + spectacle + gdbus +
  Pillow). Pixel analysis of `t14-dock-dividers-dark.png`: two rules in the
  three-region strip, one in the empty-tail strip, none in the single-region
  strip; each rule is 64 px tall (0.82 × the 78 px plate). The primary divider
  still drag-resizes icon size.

Gotchas for later tasks:

- **`applyDockOverflow` counts every planned divider but still models the gaps
  as the uniform icon `gap`.** The divider `gap` (22) is not in the fit
  estimate; the clamp stays a worst-case budget. If a later task needs an exact
  overflow, thread `component.dock.divider.gap` through and charge 2 per rule.
- **The Dock QML region builder mirrors `planDockRegions`.** There is no Dock
  QML singleton (same constraint as `movePinnedEntry`); keep the two in sync.
- **`appItemIndices` is required for any drag/gap math that speaks in
  `appEntries` indices:** an inserted rule shifts the tail entries in `items`.
  Do not index `items` with an `appEntries` index.
- **`plateRect` must exclude dividers from its cross-axis union** or the
  full-plate rule grows the plate upward by `padding` on every Dock.
- **A divider delegate's root box is `dividerWidth × barThickness` (bottom) or
  `barThickness × dividerWidth` (left/right),** not `iconSize`; the rule inside
  is oriented from `indicatorEdge`.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110x — T-14.7x Dock activation: taps on entries that carry a DragHandler

**State: done.** A stationary left click on any app/temporary/overflow entry
now activates or launches in a real session. The recorded T110g "synthetic-tap
limitation" is fixed at the injection boundary, not in QML: the shell's
hand-built Dock `QMouseEvent`s carried `timestamp() == 0`, and with a zero
timestamp `QQuickDragHandler` measures a bogus initial movement on the press and
takes the exclusive grab, so the sibling `TapHandler` never sees the tap. The
`DF_DOCK_ACTIVATION_FIXTURE` capture seam is retired. ADR 0110 records the rule;
ADR 0101 gains a pointer to it.

Real paths:

- `shell/src/dockpointer.{h,cpp}` (new, dockcore) — `DockPointer::timestamp()`
  (a shared `QElapsedTimer`, `+1` so the first event is nonzero) and
  `DockPointer::send(window, type, pos, button, buttons)`, which builds the
  `QMouseEvent` and calls `QEvent::setTimestamp` before `sendEvent`.
- `shell/src/shellcontroller.cpp` — `onDockPointerMoved` / `onDockPointerButton`
  / `onDockPointerLeft` route through `DockPointer::send`; the
  `DF_DOCK_ACTIVATION_FIXTURE` block (`launch`/`activate`/`missing`) deleted.
- `shell/tests/tst_dock.cpp` — now `QUICK_TEST_MAIN_WITH_SETUP`; a `DockInject`
  context property wraps the production `DockPointer` using the same button-state
  bookkeeping as the controller.
- `shell/tests/tst_dock.qml` — eight new `test_injected_*` cases (stationary tap
  on app/temporary/minimized/Trash/stack/overflow, right-click menu, slop-drag
  lift) driven through `DockInject`; `tst_dock` 248 → 256.
- `scripts/capture-dock-activation-driver.py` (new) — real synthetic click; scans
  the left Dock half and detects the launched window via `query identity`.
- `scripts/capture-dock-activation.sh` — no fixture; before/launch/active/missing
  through the driver.
- `scripts/capture-dock-launch-origin.sh` + `-driver.py` — no fixture; the driver
  clicks the pinned entry, then reads `query motion`.
- Captures refreshed: `docs/captures/t14-dock-activation{,-before,-after,-active,-missing}.png`
  and `docs/captures/t14-dock-launch-origin.png` + `-trace.txt`.
- Docs: `docs/design/04-shell.md` "Dock activation and launch";
  `docs/design/adr/0110-dock-pointer-injection-timestamps.md`; ADR 0101
  consequences; captures README.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore" --output-on-failure` —
  256 + 116 green.
- The pre-fix probe: comment out `event.setTimestamp(timestamp())`, rebuild
  `tst_dock`, and `test_injected_stationary_tap_activates_{an_app,temporary}_entry`
  fail (254 passed / 2 failed); all other injected cases still pass.
- Live: `make dock-activation-capture` — the driver clicked the pinned Settings
  entry at x=800 and launched it (real pointer); the stacked still reads
  before (no window) / after (window mapped, running dot) / active (focused).
  `make dock-launch-origin-capture` — real click, `appear origin=(800, 1104,
  73, 85)` inside the bottom Dock band.

Gotchas for later tasks:

- **Any chrome surface re-injected into an offscreen window must set a nonzero
  monotonic timestamp** (use `ChromePointer`; `DockPointer` is its
  compatibility alias). **Resolved across all chrome by T-16.12**: every
  injection site (menu bar / main, control center, chooser, screenshot,
  screencast, polkit, overview, banner — and the Dock) now routes through
  `ChromePointer::send`, and no bare `QMouseEvent` construction remains in the
  `ShellController` pointer handlers. An earlier revision of this note said the
  non-Dock paths were unchanged; that is superseded. See the T151a section and
  ADR 0110's amendment.
- **The QML test's `DockInject` wrapper is not the controller**, but it calls
  the same `DockPointer::send`; keep the wrapper's button-state bookkeeping in
  sync with `ShellController::m_dockButtons` if the controller's changes.
- **QtTest's own `mouseClick` stamps timestamps**, so the older `tst_dock.qml`
  activation cases pass on broken injection. A regression that changes the
  pointer synthesis must use `DockInject`, not `mouseClick`.
- **`testCase.window` does not exist** in Qt Quick Test; use
  `stage.Window.window` (the root Item's attached `Window.window`).
- **The capture driver hard-codes the bottom Dock band** (`DOCK_Y=1155`,
  scan x 770–990) and only scans for a launched Settings window; a Dock
  position/token change needs those constants revisited (same convention as
  `capture-dock-magnify-driver.py`).
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110y — T-14.7y Dock magnification tracking: stable pointer and anchor

**State: done.** Hover magnification now tracks the pointer without ringing.
Three couplings were removed in `shell/dock/Dock.qml`:

1. The per-sample pointer tracker no longer uses the overshoot
   `motion.dockMagnify` curve. A new token
   `motion.dockMagnifyTrack` (`primitive.duration.fast` = 100 ms, curve
   `[0.2, 0.0, 0.0, 1.0]`, `reducedDuration` 0) drives
   `smoothPointerAlong`; `motion.dockMagnify` keeps its overshoot only for
   discrete changes (the `iconSize` spring and reveal). Reduced motion tracks
   the raw pointer.
2. `anchorIndex` is computed from the **raw** `pointerAlong` (was the smoothed
   value), so the filter can never flip the anchored tile at a boundary. The
   anchor rule (the tile under the pointer stays put) is unchanged.
3. The plate's magnify edge is a damped peak-hold. `plateTrackRaw` is the
   entry-union edge with launch/attention bounce added back (bottom = topmost
   entry edge; right = interior left edge; left = interior right edge) and
   `smoothPlateTrack` follows a *deeper* peak through `motion.dockMagnifyTrack`,
   holds while the pointer stays on the Dock, ignores a sub-pixel
   `plateTrackDeadband` (0.75 px), and snaps on discrete magnify entry/exit
   (`plateTrackEngaged`) and under reduced motion. `plateRect` reads
   `smoothPlateTrack`. The plate still grows into the magnify band with
   magnification; it just no longer follows the per-tile ripple or sub-pixel
   noise.

Real paths:

- `design-system/tokens/tokens.json` — `motion.dockMagnifyTrack` added;
  `Theme.qml` + `compositor/src/design_tokens.rs` regenerated
  (`Theme.motion.dockMagnifyTrack`, `motion::DOCK_MAGNIFY_TRACK`).
- `shell/dock/Dock.qml` — tracker token swap + comment; `anchorIndex` raw;
  `plateTrackRaw`/`smoothPlateTrack`/`plateTrackDeadband`/`plateTrackEngaged`/
  `plateTrackAnimation`; `plateRect` now uses `smoothPlateTrack` for both axes.
- `shell/tests/tst_dock.qml` — four new cases (below); `tst_dock` 256 → 260.
- Docs: `docs/design/04-shell.md` magnification paragraph;
  `docs/design/10-design-system.md` Motion rule;
  `docs/design/adr/0111-dock-magnification-tracking-stability.md`.
- Captures: `scripts/capture-dock-magnify-sweep.sh` +
  `scripts/capture-dock-magnify-sweep-driver.py` (`make
  dock-magnify-sweep-capture`), `docs/captures/t14-dock-magnify-sweep-{light,dark}.png`
  (before row over after row), `docs/captures/sweep-row-{light,dark}-{before,after}.png`,
  `docs/captures/t14-dock-magnify-sweep-trace.txt`, captures README entry. The
  `make dock-magnify-capture` stills were regenerated once (settled; content
  unchanged).

Measured oscillation (real numbers, pre-fix vs fixed):

- One-tile pointer jump: smoothed-pointer overshoot **11.0 px** pre-fix (635.5
  vs target 624.5), **0.0 px** fixed (`test_smoothed_pointer_never_overshoots_the_target`
  fails pre-fix).
- 40-step slow sweep (QML trace): **19** plate-top reversals, max **1.108 px**
  pre-fix; **0** reversals fixed (`test_slow_sweep_has_a_stable_anchor_and_plate`).
- Captured 6-frame strip, settled crops, plate top local-y range: light
  before `[58,57,57,59,63,58]` range 6 px → after `[58,57,57,57,57,57]` range
  1 px; dark before `[59,58,57,60,63,59]` range 6 px → after all 57 range 0 px.
  The pre-fix row follows the per-tile ripple; the fixed peak-hold holds it.
- `anchorIndex` raw-pointer case and the sweep's anchor check flip on pre-fix
  code.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore|tst_design_system"
  --output-on-failure` — 260 + 116 + design-system green.
- `ctest --test-dir build --output-on-failure` — 53/53.
- `make e2e`; `make cargo-test`; `make soak` (100 clean cycles); `make clippy`;
  `cargo fmt --all -- --check`; `./scripts/gen-tokens.py --check`;
  `./scripts/check-design-tokens.sh`; `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Pre-fix probe: `git show HEAD:shell/dock/Dock.qml > shell/dock/Dock.qml`,
  rebuild, run `tst_dock` — the 4 new cases fail (overshoot, raw anchor, sweep
  anchor flip, plate reversal); restore and rebuild.
- Live: `make dock-magnify-sweep-capture` (host Wayland + spectacle + Pillow).
  To capture the committed before row, revert `Dock.qml` to HEAD, rebuild, run
  `LABEL=before COMPOSE=0 bash scripts/capture-dock-magnify-sweep.sh`, restore,
  rebuild, then run the script normally to compose.

Gotchas for later tasks:

- **The plate edge is peak-held, not released, while the pointer is on the
  Dock.** A slow move toward the Dock's ends does not shrink the plate until
  magnification ends (pointer leaves / popover / drag). That is deliberate: the
  per-tile `min()` ripple is what jittered. If T-14.7z or a later unit needs a
  release, add a release deadband on top of `plateTrackRaw` rather than
  reverting to the raw `min()`.
- **`plateTrackRaw` differs per position.** Bottom = topmost entry edge; right =
  interior (left) edge; left = interior (right) edge. The no-entry fallback was
  fixed to `rest.x + padding` (right) / `rest.x + barThickness - padding`
  (left); keep those in sync if the artwork anchor changes.
- **The tracker and the plate edge share one motion token** (`motion.dockMagnifyTrack`).
  Do not give the plate a second animation on top of the pointer tracker — ADR
  0111's "one smoothing source" rule.
- **The public QML property names are unchanged** (`pointerAlong`,
  `smoothPointerAlong`, `anchorIndex`, `plateRect`, `restingPlateRect`), so the
  shell controller and capture drivers need no change.
- The strip capture driver hard-codes `DOCK_Y=1155` and the sweep range
  `785..1135` (same convention as `capture-dock-magnify-driver.py`); a Dock
  position/token change needs those revisited.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110z — T-14.7z Dock plate rendering: corner-following rim and frost alignment

**State: done.** The plate is now one integer rounded rect in every state. The
QML fill and the compositor frost draw the same snapped edge, the bright rim
follows the corner arcs, and the declared backdrop panel is only committed when
its integer rect changes. ADR 0112 records the single-edge rule.

Real paths:

- `shell/dock/Dock.qml` — new `panelRect` (each edge of the live `plateRect`
  `Math.round`ed); the `dockPlate` group draws `panelRect` (fill, rim, border,
  shadow, clip). `dockRim` is now a `Shape` with a `PathSvg` traced by
  `rimOutlinePath(w, h)` along the interior edge (flat + both interior corner
  arcs), inset by half the stroke; the straight hairline `Rectangle` is gone.
  `import QtQuick.Shapes` added.
- `shell/src/shellcontroller.cpp` `renderDock` — reads `panelRect` (not
  `plateRect`) and commits it only when `m_dockPanelRect` changes and the
  protocol call succeeds.
- `shell/src/shellcontroller.h` — `m_dockPanelRect` / `m_dockPanelRectValid`
  (and `<QRect>`).
- `shell/tests/tst_dock.qml` — `test_panel_rect_is_the_integer_edge_the_qml_draws`,
  `test_panel_rect_never_flips_during_a_sweep`,
  `test_rim_pixels_follow_the_plate_round_rect` (hide-the-rim frame diff: no
  changed pixel outside the plate rounded rect, at least one on the top-left
  arc), `test_artwork_padding_is_even_at_rest_and_magnified`; updated
  `test_plate_is_a_layered_glass_not_a_flat_slab` and
  `test_plate_glass_follows_the_color_scheme` to read the `ShapePath`
  (`dockRimStroke`). `tst_dock` 260 → 264.
- `scripts/capture-dock-plate-corners.sh` +
  `scripts/capture-dock-plate-corners-driver.py` (`make
  dock-plate-corners-capture`); captures
  `docs/captures/t14-dock-plate-corners-{light,dark}.png` (6x corner pair) and
  `docs/captures/t14-dock-plate-magnified-{light,dark}.png` (2x strip). Captures
  README entry.
- Docs: `docs/design/04-shell.md` \"One rounded edge (T-14.7z)\";
  `docs/design/adr/0112-dock-plate-single-rounded-edge.md`.

Commands that work (repo root):

- `ctest --test-dir build --output-on-failure` — 53/53 (`tst_dock` 264).
- `make e2e` — green; `material stats` still `backdrop_passes=0
  backdrop_skipped=0` (no double-blur regression).
- `cargo test --workspace` (needs
  `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`; the Makefile
  exports it) — green.
- `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Pre-fix probe: temporarily make `rimOutlinePath` return `\"M 0 0 L <w> 0\"`
  (the old straight hairline) and rebuild `tst_dock`; the rim-differential case
  fails with **38** changed pixels outside the plate shape (fixed: 0). Restore.
- Live: `make dock-plate-corners-capture` (host Wayland + spectacle + Pillow).
  Detected resting plate nested `top=1085 left=703 right=1216`; pixel scan of
  the raw still shows a clean arc (no straight run past the curve). Vision
  review of the 6x close-up was ambiguous (it mistook the end artwork for a
  line); the deterministic QML differential is the authority.

Gotchas for later tasks:

- **`panelRect` is the drawn edge; `plateRect` is still the logical one.**
  Input region, magnify math, and most tests use the fractional `plateRect`.
  Only the plate group and the declared backdrop use `panelRect`. Do not feed
  `panelRect` back into the layout.
- **The declared panel is deduped by integer rect.** This is safe because
  `configureDockSurface` reuses the leading `m_dockLayer` (no recreate); a
  position change swaps the axes, so a new rect is sent. If a future task ever
  *destroys and recreates* the Dock layer surface with the same rect, reset
  `m_dockPanelRectValid`.
- **The rim is a `Shape`/`PathSvg`; `findChild("dockRim")` is the Shape and the
  stroke colour lives on `dockRimStroke`.** Update tests accordingly; `rim.height`
  is now `plateH - strokeWidth`, not `rimHeight`.
- **`rimOutlinePath` depends on `dock.position`** (`bottom`/`left`/`right`); a
  new top-anchored position would need a fourth branch. The SVG sweep flags are
  picked for the interior edge per position.
- The capture driver uses scheme-specific absolute rim thresholds
  (`RIM_THRESHOLD` dark 430 / light 560) and a broad `DOCK_BAND`; a theme or
  wallpaper change needs those revisited (same convention as the other Dock
  drivers).
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T110w — T-14.7w Dock icon tiles: true squircle masking

**State: done.** Every Dock app tile now clips its themed artwork to the token
squircle. The mask is a `Canvas` 2D clip, which the headless software scene
graph executes through `QPainter`, so `tst_dock.qml` sees real masked pixels —
`MultiEffect`/`OpacityMask`/`ShaderEffect` all no-op there (ADR 0113). Closes the
T-14.7j "inset-and-fitted, not squircle-masked" deviation.

Real paths:

- `shell/dock/DockGlyph.qml` — the themed app artwork is now a `Canvas`
  (`objectName: "artwork"`) that traces the tile rounded rect at
  `component.dock.icon.radiusRatio`, `ctx.clip()`s, and `ctx.drawImage()`s the
  icon from its URL (aspect-preserving). A hidden `Image` (`objectName:
  "iconLoader"`) is the load oracle: `Ready` drives `requestPaint()`, `Error`
  selects the placeholder tile (raster) or the `VectorImage` fallback (SVG).
  `hasThemedIcon` now means "masked artwork or SVG fallback showing".
  `tileW`/`tileH`/`tileRadius` are the single tile geometry; the placeholder
  `appTile` uses the same.
- `design-system/tokens/tokens.json` — `component.dock.icon.inset` 0.06 → 0.0
  (the artwork reaches the tile edge, matching the reference's "no letterbox");
  `radiusRatio` stays 0.24. `Theme.qml` + `compositor/src/design_tokens.rs`
  regenerated.
- `shell/tests/tst_dock.qml` — four new cases + the rewritten
  `test_glyph_tile_is_a_token_squircle_with_inset`; `tst_dock` 264 → 268. New
  helpers `assetPath` (resolves `shell/tests/data`) and `nearColor` (explicit
  artwork-colour checks, so a fully-drawn square cannot masquerade as masked).
- `shell/tests/data/icon-{square,padded,round}.svg` (new) — a full-bleed red
  square, a green padded square (25% padding each side), and a blue circle.
- `scripts/capture-dock-icon-mask.sh` + `-driver.py` (new,
  `make dock-icon-mask-capture`) — scratch app-index corpus + scratch settingsd
  pinning the three test apps, nested demo, per-scheme colour-located 4x crop;
  captures `docs/captures/t14-dock-icon-mask-{light,dark}.png` (748x252).
- Docs: `docs/design/04-shell.md` tile-mask paragraph;
  `docs/design/adr/0113-dock-icon-squircle-canvas-clip.md` (new); ADR 0102
  consequences updated; captures README entry.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_dock$|tst_dockcore|tst_design_system"
  --output-on-failure` — 268 + 116 + design-system green.
- `ctest --test-dir build --output-on-failure` — 53/53.
- `make e2e` — exit 0 (`material stats` still `backdrop_passes=0
  backdrop_skipped=0`).
- `make lint` — fmt/clippy/qml-test(53)/check-tokens/check-design-tokens all
  green; it still stops at `check-desktop-names` on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged baseline, same as T-14.7y/z).
- `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- Pre-fix probe: comment out `ctx.clip()` in `DockGlyph.qml`, rebuild `tst_dock`
  — `test_square_icon_is_masked_to_the_tile` fails ("the square tile corner is
  masked") and `test_placeholder_and_themed_artwork_share_the_tile` fails ("the
  artwork reaches the tile edge"); restore and rebuild.
- Live: `make dock-icon-mask-capture` (host Wayland + spectacle + gdbus +
  Pillow). PIL on the committed light/dark stills: red square bbox
  32..223 (192 px = 48 at 4x) with corner pixels = plate background; green
  padded bbox 80..175 (96 px = the icon's own 24 px square); blue circle bbox
  32..223 with centre blue and the pixel above the top edge background. The
  nested Dock also showed the default pins/Downloads/Trash; the crop frames
  only the three test tiles by colour.

Gotchas for later tasks:

- **The mask is a `Canvas` clip, not a shader.** Do not replace it with
  `MultiEffect`/`OpacityMask`/`ShaderEffect`: those no-op on the software scene
  graph and the `tst_dock.qml` pixel cases would silently stop observing the
  mask (ADR 0113).
- **`Canvas.drawImage(url)` and `drawImage(Image)` differ in software**: the
  element form silently draws nothing under a clip, so the artwork is drawn by
  URL. `imageLoaded` must `requestPaint()` because the first paint may precede
  the decode.
- **The hidden `iconLoader` and the Canvas share one cache entry** only because
  neither sets `sourceSize`; re-adding `sourceSize` would double-decode.
- **`component.dock.icon.inset` is now 0.** A padded theme icon therefore keeps
  its own padding and its opaque content stays its own shape (a sharp inner
  square) — that is the "not double-inset" behaviour, not a failure to mask.
- The capture driver detects the three tiles by their exact fill colours inside
  a fixed bottom-centre band and requires all three; a different pinned corpus
  or a Dock-position change needs `CROP_HALF_W`/`CROP_HEIGHT`/`COLORS` revisited
  (same convention as the other Dock drivers). The capture runner re-asserts
  `dock.pinned` after settle because the shell seeds default pins on the first
  empty settings snapshot.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T172 — T-18.1a Wallpaper provider service and shipped default

**State: done.** `services/wallpaperd` is a real session service. It resolves
the shipped original default, fetches Wikimedia Commons Featured Pictures into
a lazy cache with attribution, selects the deterministic Nature entry, and
serves `org.dragonfruit.Wallpaper1`; T-18.1b consumes the interface.

Real paths:

- `services/wallpaperd/` (new crate, workspace member) —
  `src/model.rs` (`Category` + ADR 0055 map, `Status`, `WallpaperItem`,
  `Catalogue`); `src/wikipedia.rs` (`build_query`, `strip_utm`, `strip_html`,
  fixture-pinned `parse_response`); `src/source.rs` (`HttpClient`, live
  `UreqHttp`, `ContentSource`, `WikipediaSource`, `MockSource`);
  `src/cache.rs` (`$XDG_CACHE_HOME/dragonfruit/wallpapers`, `index.json`,
  `WEEK_SECS`); `src/defaults.rs` (resolution + `install_default`);
  `src/provider.rs` (`Provider`, `fetch`, `ServiceState`, one in-flight
  refresh); `src/dbus.rs`; `src/main.rs`; `tests/read_path.rs` (8);
  `tests/session_bus.rs` (3, private dbus-daemon);
  `tests/fixtures/commons-landscapes.json` (real API response).
- `Cargo.toml` — member added; the only new dependency is
  `ureq = "=3.4.2"` (rustls+gzip, MIT/Apache-2.0).
- `services/session/units/dragonfruit-wallpaperd.service` (new),
  `dragonfruit-session.target` `Wants`, `services/session/src/plan.rs`
  (stage 1), `services/session/src/entry.rs` (`UNIT_FILES` + installs
  `Default.jpg` to `share/dragonfruit/wallpapers/`).
- `services/session/tests/{units,session_entry}.rs` — unit set, policies, and
  the wallpaper install asserted.
- `Makefile` — `e2e` runs `dragonfruit-wallpaperd`; new `install` target
  (`dragonfruit-session --install-session $(DESTDIR)$(PREFIX)`).
- Docs: `docs/design/adr/0114-wallpaper-provider-service-and-shipped-default.md`.
- No deviation from the task's D-Bus contract. The custom `ItemsChanged` /
  `StatusChanged` signals use Rust idents `notify_*` with
  `#[zbus(name = ...)]` because zbus reserves `<property>_changed` for the
  standard `PropertiesChanged`; both are emitted.

Commands that work (repo root):

- `cargo test -p dragonfruit-wallpaperd` — 37 lib + 8 + 3 integration green.
- `cargo test --workspace` (`PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`)
  — green.
- `make e2e` — green (`make demo --headless` scripted half OK).
- `cargo clippy --workspace --all-targets -- -D warnings`; `cargo fmt --all -- --check`
  — green.
- Live smoke: `target/debug/dragonfruit-wallpaperd --print-builtin`
  (override → share → in-tree), `--print-status` (lazy: `status:"idle"`, no
  fetch), `--install-default DIR`; with
  `HTTPS_PROXY=http://127.0.0.1:1 ... --preload` → `status:"offline"` in 11 ms,
  shipped default still resolved.

Gotchas for later tasks:

- **Two defaults, two properties.** `BuiltinDefaultSource` is the shipped
  `Default.jpg` (resolved, read in place, never cached); `DefaultSource` is the
  fetched first-Nature entry and its fallback. T-18.1b's precedence is user →
  builtin → provider → solid colour.
- **The refresh lock is never held across network I/O.** `ServiceState::
  refresh_if_needed` locks only for `warm_decision`/`begin_fetch`/`apply_fetch`;
  `fetch` runs unlocked. `begin_fetch` is the single in-flight guard. Keep that
  split — holding the provider mutex across a fetch would block `Items` reads.
- **Resolution order is `DF_DEFAULT_WALLPAPER` → `$XDG_DATA_DIRS/dragonfruit/
  wallpapers/Default.jpg` → in-tree.** Tests inject the resolver's fields; do
  not add another env var without updating `defaults.rs` tests.
- **`ureq` is the one HTTP dependency; keep the license permissive.** Do not
  swap in a GPL/AGPL client.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T173 — T-18.1b Provider settings, wallpaper API wiring, and effective source

**State: done.** The additive provider keys are declared (schema rev 10), the
shell resolves the effective source (user → shipped default → fetched
fallback → solid), and `SettingsBridge` exposes the provider catalogue and
`Preload` to panes. The out-of-box `Default.jpg` renders with the provider
absent, the provider default reaches the compositor with settingsd absent, and
a user `wallpaper.source` always wins. ADR 0115 records the wiring.

Real paths:

- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 9 → 10; 5 additive
  `wallpaper.*` keys (`provider`, `providerAutoFetch`, `providerLastFetch`,
  `providerSource`, `builtinDefault`), owner `wallpaperd`; new unit test
  `the_wallpaper_provider_keys_are_declared_in_revision_ten`.
- `docs/settings-keys.md` — the five rows + consumer-map/effective-source text.
- `libs/settings-client/settingsclient.{h,cpp}` — defaults for the five keys;
  new shared `shippedDefaultWallpaperPath()` (`DF_DEFAULT_WALLPAPER` →
  `$XDG_DATA_DIRS/dragonfruit/wallpapers/Default.jpg` → in-tree), mirroring
  `services/wallpaperd/src/defaults.rs`.
- `libs/settings-client/CMakeLists.txt` — compile define
  `DF_IN_TREE_DEFAULT_WALLPAPER` = `<repo>/assets/graphics/wallpapers/Default.jpg`.
- `shell/src/wallpaperpolicy.{h,cpp}` — `WallpaperSettings` gains
  `builtinDefault`/`providerSource` + `effectiveSource()`;
  `effectiveWallpaperSource(user, builtin, provider)` is the pure precedence;
  `wallpaperSettingsFromValues` reads the two new keys.
- `shell/src/shellcontroller.{h,cpp}` — `connectWallpaperProvider()` /
  `refreshWallpaperProvider()` subscribe to `org.dragonfruit.Wallpaper1`
  (`Properties.Get` + `PropertiesChanged` + `ItemsChanged`, restart resync);
  `applyWallpaperPolicy` merges settingsd keys, provider props, then the local
  shipped-asset resolver.
- `apps/settings/SettingsBridge.{h,cpp}` — `providerItems`, `providerStatus`,
  `providerDefault`, `wallpaperBuiltinDefault`, `providerChanged()`, and
  `Q_INVOKABLE preloadWallpapers()`; parses the provider JSON, adds `source`
  (local path) + `url` (`file://`) per entry.
- `shell/tests/tst_wallpaperpolicy.cpp` — `userChoiceWins`,
  `bundledDefaultBeatsTheProvider`, `providerIsTheFetchedFallback`,
  `missingEverythingIsTheSolidColor`, `providerKeysMapThrough`,
  `shippedDefaultResolvesTheOverride`.
- `apps/settings/tests/tst_settings_absence.qml` —
  `test_wallpaper_provider_absence_never_surfaces_an_error`.
- `apps/settings/tests/tst_settings_live.cpp` — `FakeWallpaperService` +
  `bridgeExposesTheWallpaperProviderProperties`.
- `docs/design/adr/0115-wallpaper-effective-source-and-absence.md` (new).

Commands that work (repo root):

- `cargo test -p dragonfruit-settingsd` — green (schema/schema_doc/session_bus).
- `cargo run -p dragonfruit-settingsd -- --print-keys | grep wallpaper` — the
  five new keys with owner `wallpaperd`.
- `ctest --test-dir build -R "tst_wallpaperpolicy|tst_settings_absence|tst_settings_live|tst_settingsclient|tst_settings_wallpaper" --output-on-failure` — green.
- `ctest --test-dir build --output-on-failure` — 53/53.
- `cargo test --workspace` (+ `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`) — green.
- `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings` — green.
- `make e2e` — green; demo log line:
  `wallpaper applied (... source=.../assets/graphics/wallpapers/Default.jpg)`.

Live checks (host Wayland, scratch scripts under `/tmp/opencode/`):

- No settingsd, no provider: demo renders
  `source=.../assets/graphics/wallpapers/Default.jpg`; a spectacle still shows a
  rich multicolour background (82k unique colours, σ≈85 in the centre band),
  i.e. an image, not a solid colour. (Vision tool returned HTTP 429; the
  pixel-stat + shell log are the evidence.)
- Scratch settingsd owning `wallpaper.source=/tmp/opencode/user-wallpaper.png`:
  shell log shows first the default, then
  `source=/tmp/opencode/user-wallpaper.png` — user choice wins live.
- No settingsd + scratch provider with `DF_DEFAULT_WALLPAPER=/tmp/opencode/provider-default.png`:
  shell log shows `source=/tmp/opencode/provider-default.png` — the provider's
  `BuiltinDefaultSource` reaches the compositor with settingsd absent.

Gotchas for later tasks:

- **Qt6 `QDBusConnection::connect` here has no functor overload.** Use the
  `SLOT(...)` string form; the handlers must be declared under `private slots:`
  (`onWallpaperProviderPropertiesChanged(QString,QVariantMap,QStringList)`,
  `onProviderPropertiesChanged(...)`). A lambda does not compile.
- **The provider's properties are read via `org.freedesktop.DBus.Properties.Get`**
  on `org.dragonfruit.Wallpaper1` / `/org/dragonfruit/Wallpaper1`; the property
  values arrive wrapped in a `QDBusVariant` and must be unwrapped.
- **`wallpaperd` does not mirror its defaults into settingsd.** The five keys
  are declared (owner `wallpaperd`) and the shell reads them when non-empty, but
  T-18.1a publishes the values only as D-Bus properties. The shell therefore
  prefers a non-empty settingsd key and falls back to the live provider
  property; a future task can add the settingsd mirror without a shell change
  (ADR 0115).
- **The shipped-asset resolver lives in `libs/settings-client`**, not in
  `wallpaperpolicy`, so both the shell and the Settings app share it. Its
  in-tree path comes from the `DF_IN_TREE_DEFAULT_WALLPAPER` compile define; a
  build that drops it still works when the asset is installed under
  `$XDG_DATA_DIRS`.
- **Pane UI is untouched** (`WallpaperPane.qml` still shows the six gradients);
  T-18.2 consumes `providerItems`/`providerStatus`/`preloadWallpapers`.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

## T174 — T-18.2 Wallpaper pane collections, skeleton, and attribution

**State: done.** The Wallpaper pane now has Featured / Built-in / Custom rows;
Featured binds the provider catalogue, shimmers with a new design-system
`Skeleton` while it downloads, shows attribution for the current fetched
picture, and requests an eager `Preload` on open. A guarded `DF_WALLPAPER_FIXTURE`
seam makes every provider state assertable headlessly.

Real paths:

- `design-system/tokens/tokens.json` — semantic `skeletonBase`/`skeletonHighlight`
  (light/dark), `component.skeleton` (`radius`/`width`/`height`/`highlightRatio`),
  `motion.skeleton` (`duration` = primitive.duration.slower, curve, reduced 0).
  `Theme.qml` and `compositor/src/design_tokens.rs` regenerated.
- `design-system/components/Skeleton.qml` (new) — a clipped grey rect with a
  horizontal plateau gradient highlight; `shimmering` is false under
  `Theme.reducedMotion`, which starts no animation and centers the band.
  `progress` + `highlightX` are exposed for tests; `accessibleName` drives
  `Accessible.name` (unnamed = ignored).
- `design-system/gallery/GalleryContent.qml` — new `SkeletonPage` (index 25).
- `scripts/check-gallery-snapshots.py` — `skeleton` page + light/dark base and
  light highlight invariant checks; goldens
  `design-system/gallery/snapshots/skeleton_{light,dark,dark_reduced}.png`.
- `design-system/tests/tst_design_system.qml` — four Skeleton cases (tokens,
  accessible name, shimmer moves, reduced motion static).
- `apps/settings/SettingsBridge.{h,cpp}` — `providerPreloadCount` property;
  `preloadWallpapers()` counts fixture requests; `setWallpaperFixture(status)`
  seeds ready/fetching/offline/error under `DF_WALLPAPER_FIXTURE`;
  `sanitizeHtmlText()` strips tags/decodes entities from `artist`/`description`
  in `applyProviderItems`.
- `apps/settings/WallpaperPane.qml` — three groups; `featuredItems` (provider),
  `builtinItems` (shipped `Default` tile first + 6 gradients), `currentItem`,
  `hasAttribution`; `Component.onCompleted` calls `preloadWallpapers()`;
  attribution = artist (PlainText) + license link + file-page link.
- `apps/settings/tests/tst_settings_wallpaper.qml` — fetching skeleton,
  reduced motion, ready tiles + accessible names, offline/error, selection,
  sanitized attribution, shipped-default tile, preload-on-open, and the old
  fit/all-Spaces/photo cases. `CMakeLists.txt` sets `DF_WALLPAPER_FIXTURE=1`.
- `apps/settings/tests/tst_settings_absence.qml` — absent provider leaves
  Featured empty with the note and the Built-in default visible; portal absent
  disables only Custom.
- `scripts/capture-t18-wallpaper.sh` (new, `make t18-wallpaper-capture`) +
  `docs/captures/t18-wallpaper-{fetching,filled}.png`.
- `docs/design/adr/0116-wallpaper-pane-rows-skeleton-and-fixture-seam.md` (new);
  `docs/captures/README.md` entry.

Commands that work (repo root):

- `ctest --test-dir build -R "tst_design_system|tst_settings_wallpaper|tst_settings_absence|tst_settings_live" --output-on-failure` — green.
- `make qml-test` — 53/53.
- `./scripts/gen-tokens.py --check`; `./scripts/check-design-tokens.sh`;
  `./scripts/check-no-capture-grab.sh`;
  `./scripts/check-gallery-snapshots.py --strict` — green.
- `cargo fmt --all -- --check` — green (no Rust logic changed; only the
  generated token module).
- Live: `make t18-wallpaper-capture` (host Wayland + scratch settingsd +
  scratch-cache wallpaperd through a hanging CONNECT proxy for the fetching
  still, then a real fetch and a selected item for attribution). PIL evidence:
  the Featured row area at (820,430)-(1280,560) is 9 unique colours (uniform
  `#f8f6fa`) in the fetching still and 1639 unique colours (a photo) in the
  filled still.
- `check-desktop-names.sh` still fails only on the pre-existing
  StatusNotifier/zoo/apppicker lines (unchanged here).

Gotchas for later tasks:

- **The skeleton highlight is a plateau (two `#skeletonHighlight` stops at
  0.4/0.6).** A single stop at 0.5 never lands on a pixel centre, so the exact
  token colour would not appear in the golden; the `--strict` gallery invariant
  checks the exact colour. Keep the plateau if you retune the gradient.
- **`DF_WALLPAPER_FIXTURE` is the only provider test seam.** It seeds the
  bridge in-process and makes `preloadWallpapers()` a counted no-op; it is
  inert without the env var. Do not build production behavior on it.
- **Attribution is only for fetched items.** `currentItem.fetched` gates the
  artist/license/file-page panel; built-in and user photos intentionally show
  none (ADR 0094). The artist/description are sanitized in the bridge and
  rendered as `Text.PlainText` (defense in depth).
- **`category` arrives as a lowercase slug**; the pane capitalizes it for the
  tile accessible name via `categoryLabel()`.
- **The pane calls `Preload` in `Component.onCompleted`**; the shell's Loader
  destroys/recreates the body on each pane switch, so this is once per open.
  The provider has no explicit "return to idle" call — leaving the pane simply
  stops asking, and the service stays lazy.
- The `check-gallery-snapshots.py --update` run rewrites the timing-sensitive
  `tooltip_*` goldens; restore them (`git checkout -- ...tooltip_*.png`) unless
  the tooltip actually changed.

## T175 — T-18.3 Provider licensing, absence matrix, and capture

**State: done.** The licensing policy is reviewed and made true (`NOTICE`
now lists the shipped default), the 11-row absence/state matrix is committed
with a headless reproduction, the offline capture still is added, the
T-16.6/T-16.7 hand-off notes are written into the track doc, all eight track
acceptance boxes are ticked, and the package install proves only
`Default.jpg` ships.

Real paths:

- `docs/licensing.md` — "Fetched third-party content (wallpaper)" extended:
  the shipped original default is the only packaged image, fetched pictures
  are cache-only user data, attribution is mandatory, share-alike/FAL are
  never original, and the HTTP dependency is named (`ureq` 3.4.2,
  MIT OR Apache-2.0). Names the mechanical checks.
- `NOTICE` — `Default.jpg` added under "Bundled assets" plus the
  never-bundled-fetched-content note.
- `docs/captures/t18-absence-matrix.md` (new) — the reviewed matrix + live
  halves + verdict; `docs/captures/t18-absence-matrix.txt` (new) — headless
  reproduction transcript.
- `scripts/t18-absence-matrix.sh` (new; `make t18-absence-matrix`) — runs the
  row reproductions (CLI with a dead proxy + named tests + ctest + `make
  install`) into the transcript.
- `docs/captures/t18-wallpaper-offline.png` (new) — cold cache + dead network:
  shipped-default desktop + "available soon" Featured row. Emitted first by
  `scripts/capture-t18-wallpaper.sh` (now takes mode `dead` in
  `start_provider`).
- `docs/design/tracks/18-wallpaper-content-provider.md` — 8 acceptance boxes
  ticked; T-16.7 string list + T-16.6 AT-SPI/keyboard notes added to Hand-off.
- `docs/captures/README.md` — T-18.3 matrix + offline still described.
- `compositor/tests/milestone_e2e.rs` — `xwayland_running_for` now splits argv
  (argv[0] basename `Xwayland`, display as a whole arg) instead of substring
  scanning the flattened cmdline.
- `services/wallpaperd/tests/session_bus.rs` — `preload_round_trips…` waits
  (bounded) for the zbus proxy property cache to be invalidated.

Commands that work (repo root):

- `make e2e` — EXIT 0.
- `make test` — EXIT 0, 53/53 ctest, 78 gallery snapshots.
- `make soak` — EXIT 0, "100 clean cycles, zero strays".
- `make t18-absence-matrix` — transcript all green.
- `make install DESTDIR=<scratch> PREFIX=/usr` — only
  `share/dragonfruit/wallpapers/Default.jpg`, byte-identical to the asset.
- `make t18-wallpaper-capture` (host Wayland) — offline/fetching/filled
  stills. Pixel evidence: offline desktop 172,128 unique colours, σ 70.4,
  identical to the pre-fetch still; offline Featured row 343 colours;
  filled Featured row 43,972 colours. Vision HTTP 429 (pixel stats are the
  evidence, as in T-18.1b).

Gotchas for later tasks:

- **`make check` still fails only on `check-desktop-names`** (pre-existing
  StatusNotifier/zoo/apppicker lines), unchanged. `make lint` otherwise green.
- **The `make soak` "stray processes" failure is a leftover-process gate, not
  a product bug.** An interrupted/long run can leave `dragonfruit dev`
  children; clear them (`pkill -f target/debug/dragonfruit`) before soak.
- **`Default.jpg` provenance cannot be proven inside the repo.** It is a real
  Canon 5D/Lightroom photo with no watermark, added in the early `c138043`
  snapshot and treated as the project's original asset (ADR 0094). The
  licensing claim rests on that prior decision; keep any provenance record
  with the asset.
- **`docs/captures/t18-absence-matrix.txt` is generated**; regenerate with
  `make t18-absence-matrix` if the CLI/tests change.
- The provider reports the in-tree default as a `../../`-relative path; the
  transcript shows it. Canonicalizing would be cosmetic only.

## Dev tooling — wallpaper provider in the dev session (T-18.1a follow-up)

**State: done.** `make demo` (nested) and `make dev --shell` now start
`dragonfruit-wallpaperd`, so the Settings → Wallpaper **Featured** row fills
from `org.dragonfruit.Wallpaper1` instead of always showing "available soon".

Real paths:

- `tools/dragonfruit-dev/src/main.rs` — `launch_services` takes a
  `wallpaper: bool`; a new pure `service_names(wallpaper)` returns app-index,
  menu-broker, and (when set) wallpaperd. `run_demo_session` passes
  `!scripted`, `run_dev_session` passes `true`; the `--headless` scripted path
  (CI / `make e2e`) skips the provider so the gate never warms its cache.
- Unit test `wallpaperd_is_started_for_the_live_session_but_not_the_scripted_demo`.

## T111 — T-15.1a Bluetooth adapter

**State: done.** The BlueZ Bluetooth adapter landed in a new crate,
`dragonfruit-bluetooth` (`services/bluetooth`, workspace member), behind the
shared contract. No shell wiring (T-15.1b renders it; the status-bridge host is
T-15.16).

Real paths:

- `services/bluetooth/` (new crate) — `src/source.rs` (`BluetoothData`,
  `BluetoothOutcome`, `BluetoothSource`, `MockBluetooth`), `src/model.rs`
  (`BluetoothController`, `BluetoothDevice`, `BluetoothSnapshot`),
  `src/bluez.rs` (`DbusBluez`: `GetManagedObjects` at `/` on the system bus +
  `Properties.Set Powered` / `Start|StopDiscovery` / `Pair` / `Connect|Disconnect`),
  `src/adapter.rs` (`BluetoothAdapter<S>`), `src/lib.rs`.
  `tests/read_path.rs` + `tests/fixtures/bluez-office.json`.
- `Cargo.toml` (workspace members) + `Makefile` (`make e2e` now runs
  `cargo test -p dragonfruit-bluetooth`).
- `docs/design/07-system-integration.md` — "The Bluetooth path (T-15.1a)".
- `docs/design/adr/0117-bluetooth-adapter-absence-and-write-outcomes.md` (new).
- `docs/captures/t15-1a-bluetooth-adapter.png` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-bluetooth` — 30 lib + 9 integration green.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`;
  `cargo clippy --workspace --all-targets -- -D warnings` — green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (StatusNotifier/zoo/apppicker), unchanged.

Gotchas for later tasks (T-15.1b / T-15.16):

- **No `BluetoothAccess` read-only degradation.** Unlike Wi-Fi, a polkit denial
  is per-request (`BluetoothOutcome::Denied`); reads stay live and the adapter
  keeps no access mode (ADR 0117). Each refused action surfaces its own note.
- **Two hide rules**: `bluetoothd` absent ⇒ `AdapterState::Unavailable` (slot
  hidden); BlueZ present but no controller ⇒ `Available` with
  `BluetoothSnapshot::present() == false` (consumer hides). A present but
  `Powered = false` controller is shown with an Off switch.
- **Writes invent no snapshot**: after `set_powered`/`set_discovering`/`pair`/
  `set_connected`, BlueZ pushes `PropertiesChanged`/`InterfacesAdded`; the host
  calls `BluetoothAdapter::refresh` and re-reads. The adapter is event-driven;
  do not add a poll.
- **ObjectManager read decodes property maps directly** (`downcast_ref`), so a
  missing optional property (`Name`, `RSSI`, `Alias`) is a default, not an
  error. RSSI 0 is "unknown" (`BluetoothDevice::rssi == None`).
- The live D-Bus path is compile-checked but not run in CI (no bus/daemon); the
  fixture seam is the tested contract.
- `BluetoothAdapter` is the adapter struct; the controller model is
  re-exported as `BluetoothController` to avoid the name clash.

## T112 — T-15.1b Bluetooth pane and tile

**State: done.** The Bluetooth pane and Control Center tile ship as one unit
over the bridge host. The pane applies live (power/discovery/connect), the tile
reflects state and writes the radio, and absence is documented and tested.

Architecture (ADR 0118): the Bluetooth adapter rides a **fourth interface** on
the existing `dragonfruit-system-status` host, not a second service.

Real paths:

- `services/system-status/src/bluetooth.rs` (new) — `BluetoothHost<B>` and
  `bluetooth_view`/`bluetooth_snapshot_view`/`bluetooth_report`. The view is
  `{state,present,powered,discovering,discoverable,pairable,glyph,label,
  adapterName,connectedCount,knownCount,nearbyCount,knownDevices,nearbyDevices}`;
  each device carries `address,name,paired,connected,trusted,blocked,rssi,
  signal`. Four writes return `accepted/denied/absent/failed`.
- `services/system-status/src/lib.rs` — `BLUETOOTH_INTERFACE` constant,
  re-exports.
- `services/system-status/src/dbus.rs` — `LiveBluetooth = BluetoothHost<DbusBluez>`,
  `BluetoothInterface` (`State/Refresh/SetPowered/SetDiscovering/Pair/
  SetConnected`), `run(host, bluetooth)`, `interface_names()` now 4.
- `services/system-status/src/main.rs` — `--print-bluetooth`, constructs the
  BlueZ host.
- `services/system-status/Cargo.toml` — `dragonfruit-bluetooth` dependency.
- `services/system-status/tests/bluetooth.rs` (new) — 4 integration tests.
- `shell/src/systemstatusclient.{h,cpp}` — `refreshBluetooth` + four writes +
  `bluetoothState`/`writeReport`; `MockSystemStatusClient` serves a fixture
  (powered, 2 known devices, nearby when discovering) — `DF_STATUS_FIXTURE`.
- `shell/src/systemstatusmodel.{h,cpp}` — `bluetooth()`/`bluetoothVisible()`,
  `applyBluetooth(Json)`, kind `bluetooth`; the `present: false` second hide
  rule is shared with the battery.
- `shell/src/shellcontroller.{h,cpp}` — wires the signals, pushes the tile
  data and `bluetoothWritable=true`, adds
  `onBluetoothToggleRequested`/`onBluetoothDeviceToggled`/
  `onBluetoothSettingsRequested` (the Settings link logs T-16, like
  Wi-Fi/Focus).
- `shell/control-center/ControlCenter.qml` — a Bluetooth tile (icon, toggle,
  known-device rows with Connected/Not Connected, `Bluetooth Settings…`);
  tile model now 6 entries and the Bluetooth entry carries `visible`.
- `apps/settings/BluetoothClient.{h,cpp}` (new) — abstract seam +
  `DbusBluetoothClient` (`org.dragonfruit.SystemStatus1.Bluetooth`) +
  `MockBluetoothClient` (`DF_BLUETOOTH_FIXTURE`).
- `apps/settings/SettingsBridge.{h,cpp}` — `bluetooth`/`bluetoothAvailable`
  properties + `refreshBluetooth`/`setBluetoothPowered`/
  `setBluetoothDiscovering`/`pairBluetooth`/`setBluetoothConnected`.
- `apps/settings/BluetoothPane.qml` (new) — toggle card + discoverable caption,
  `My Devices` rows with a circular info/connect button, `Nearby Devices`
  (`Searching…` + a `Shape` spinner). Starts discovery on open, stops on close.
- `apps/settings/SettingsPanes.qml` — `bluetooth.shipped: true`;
  `SettingsShell.qml` registers the pane and now defaults to `appearance`
  (the first row is Bluetooth; opening Settings must not auto-start an
  inquiry).
- `apps/settings/CMakeLists.txt` — new sources/QML; tests CMake adds
  `tst_settings_bluetooth` (with `DF_BLUETOOTH_FIXTURE`).
- Tests: `shell/tests/tst_statusmodel.cpp` (3 new), `shell/tests/
  tst_controlcenter.qml` (Bluetooth tile, toggle/device round-trip, absence
  hide, fit-to-surface, six-tile model), `apps/settings/tests/
  tst_settings_bluetooth.{cpp,qml}` (new), `tst_settings_absence.qml`
  (bluetooth shipped + absence), `tst_settings_shell.qml` (5 shipped panes).
- Docs — `docs/design/07-system-integration.md` "The Bluetooth pane and tile
  (T-15.1b)"; ADR `0118-bluetooth-pane-and-tile.md`; `docs/captures/README.md`.
- Captures — `scripts/capture-t15-bluetooth.sh` (new);
  `docs/captures/t15-1b-bluetooth-pane.png`,
  `docs/captures/t15-1b-bluetooth-control-center.png`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 17 lib + 4 bluetooth + 9 host.
- `cmake --build build --target ...`; `ctest --output-on-failure -j4` —
  54/54.
- `make e2e` — EXIT 0 (one flaky `dragonfruit-lock-auth` run first; the
  isolated test passes and the rerun is green).
- `cargo fmt --all -- --check`; `cargo clippy -p dragonfruit-system-status
  --all-targets -- -D warnings` — clean.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (StatusNotifier/zoo/apppicker), unchanged.

Decisions / deviations:

- **Bridge host, not a second service.** A `BluetoothHost<B>` sits beside
  `StatusHost` under one `org.dragonfruit.SystemStatus1`, so the three-adapter
  host and its tests are untouched (ADR 0118).
- **Settings link is a T-16 entry point.** `Bluetooth Settings…` logs, matching
  the existing Wi-Fi/Focus/Appearance links; launching Settings on a named pane
  is T-16.
- **Menu-bar Bluetooth slot stays hidden.** The task owns the Control Center
  tile; a visible slot with no Bluetooth menu would be a dead control.
- **Default Settings pane stays `appearance`.** Shipping Bluetooth (catalog
  row 2) would otherwise make it the default and auto-start discovery on every
  Settings launch.

Live visual check: `bash scripts/capture-t15-bluetooth.sh` on the host Wayland
session. Both stills written (paths above). The vision tool returned HTTP 429,
so the evidence is the pixel statistics plus the headless tests: the pane still
is a rendered Settings window (2088x1410, 430 364 unique colours, luminance
sigma 93.5) and the panel crop is 360x760 (4 942 unique colours, sigma 33.1).
`tst_controlcenter` additionally asserts the panel content fits the 780 px
surface, and `tst_settings_bluetooth` asserts the toggle caption, device
connect round-trip, and searching state.

Gotchas for T-15.16 / later:

- `DF_BLUETOOTH_FIXTURE` is the Settings-pane seam; `DF_STATUS_FIXTURE` now
  also drives the shell's Bluetooth tile. Neither is set in `make e2e`.
- The absent-daemon matrix can drive Bluetooth three ways: mask the bridge host
  (view empty/`unavailable`), mask `org.bluez` (`unavailable`), or drop the
  controller (`present: false`).
- The shell menu-bar Bluetooth item (`shellcontroller.cpp` `applyStatusItems`)
  is still hardcoded hidden; enabling it needs a Bluetooth menu (not this task).

## T113 — T-15.2a Storage and removable media adapter

**State: done.** The UDisks2 storage/removable-media adapter landed in a new
crate, `dragonfruit-storage` (`services/storage`, workspace member), behind the
shared contract. Backend only — no shell wiring (T-15.2b renders it).

Real paths:

- `services/storage/` (new crate) — `src/source.rs` (`StorageData`,
  `StorageDriveData`, `StorageVolumeData`, `StorageOutcome`, `StorageSource`,
  `MockStorage`), `src/model.rs` (`StorageSnapshot`, `StorageDrive`,
  `StorageVolume`), `src/adapter.rs` (`StorageAdapter<S>`), `src/udisks.rs`
  (`DbusUDisks`: ObjectManager `GetManagedObjects` at
  `/org/freedesktop/UDisks2` on the system bus; `Mount`/`Unmount` on the
  block's `Filesystem`, `Eject` on the owning `Drive`), `src/lib.rs`.
  `tests/read_path.rs` + `tests/fixtures/udisks-workstation.json`.
- `services/system-adapters/src/state.rs` — new `AdapterId::STORAGE` (`"storage"`).
- `Cargo.toml` (workspace members) + `Makefile` (`make e2e` now runs
  `cargo test -p dragonfruit-storage`).
- `docs/design/07-system-integration.md` — "The storage path (T-15.2a)".
- `docs/design/adr/0119-storage-adapter-absence-and-mount-outcomes.md` (new).
- `scripts/capture-t15-storage.sh` (new) + `docs/captures/t15-2a-storage-adapter.png`;
  `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-storage` — 30 lib + 10 integration green.
- `cargo test -p dragonfruit-system-adapters` — 4 green.
- `make e2e` — EXIT 0 (first run hit the known flaky `dragonfruit-lock-auth`
  `helper` test; the isolated test passed and the rerun was green).
- `cargo fmt --all -- --check`; `cargo clippy -p dragonfruit-storage
  --all-targets -- -D warnings` — clean.

Gotchas for T-15.2b / T-15.16:

- **UDisks2 is the one consumed stack, not GIO/GVfs.** UDisks2 already gives
  the block enumeration, `HintAuto`/`HintSystem`/`HintIgnore`, and the mount
  ops; the GIO/GVfs volume monitor stays in the Files sidebar (`files-core`).
  ADR 0119 records this.
- **Writes are addressed by UDisks2 object path**, not device node — the
  snapshot carries `StorageVolume::path` / `StorageDrive::path`.
- **Two hide rules**: UDisks2 absent ⇒ `AdapterState::Unavailable` (hidden);
  UDisks2 present but no mountable volume ⇒ `Available` with
  `StorageSnapshot::present() == false` (consumer hides).
- **The model drops `HintIgnore` and non-mountable blocks.** Internal system
  volumes are kept (attached to their drive, `is_system()` true); use
  `removable_volumes()` / `internal_volumes()` / `mounted_volumes()`.
- **Writes invent no snapshot**: after `mount`/`unmount`/`eject`, UDisks2
  pushes the property changes and the host re-reads. Do not add a poll.
- **A polkit refusal is `StorageOutcome::Denied`; a busy device is
  `Failed`**, never a denial. There is no read-only degradation.
- **Locked encrypted volumes have no `Filesystem`**, so they are not listed as
  mountable; unlock/format are out of scope.
- The live D-Bus path is compile-checked but not run in CI (no bus/daemon);
  the fixture seam is the tested contract.
- Live visual check: nested demo + `scripts/capture-t15-storage.sh` (no
  surface; vision tool returned HTTP 429 — evidence is pixel statistics plus
  the headless suites).

## T114 — T-15.2b Storage and removable media pane and tile

**State: done.** The Storage pane and Control Center tile ship as one unit over
a fourth interface on the existing `dragonfruit-system-status` bridge host
(ADR 0120). The pane applies live (mount/unmount/eject), the tile reflects
state and writes it, and absence is documented and tested.

Real paths:

- `services/system-status/src/storage.rs` (new) — `StorageHost<S>` +
  `storage_view`/`storage_snapshot_view`/`storage_report`; view
  `{state,present,glyph,label,mountedCount,volumeCount,removableCount,drives,
  volumes}`; three writes report `accepted/denied/absent/failed`.
- `services/system-status/src/{lib,dbus,main}.rs` — `STORAGE_INTERFACE`,
  `LiveStorage`, `StorageInterface (State/Refresh/Mount/Unmount/Eject)`,
  `run(host, bluetooth, storage)`, `interface_names()` 5, `--print-storage`.
- `services/system-status/Cargo.toml` — `dragonfruit-storage` dep.
- `services/system-status/tests/storage.rs` (new) — 4 integration tests.
- `shell/src/systemstatusclient.{h,cpp}` — `refreshStorage` + mount/unmount/
  eject + `storageState`; mock serves a removable-drive fixture.
- `shell/src/systemstatusmodel.{h,cpp}` — `storage()`/`storageVisible()`,
  `applyStorage(Json)`; `present:false` hide rule now covers storage.
- `shell/src/shellcontroller.{h,cpp}` — tile data + `storageWritable`, four
  `onStorage*` slots.
- `shell/control-center/ControlCenter.qml` — compact Storage tile; tile model
  now 7 entries.
- `design-system/components/Icon.qml` — `storage` painted glyph.
- `apps/settings/StorageClient.{h,cpp}` (new) — `DbusStorageClient` +
  `MockStorageClient` (`DF_STORAGE_FIXTURE`).
- `apps/settings/SettingsBridge.{h,cpp}` — `storage`/`storageAvailable` +
  four writes.
- `apps/settings/StoragePane.qml` (new) — `Volumes` (Mount/Unmount per row) +
  `Removable Media` (Eject) + absence note.
- `apps/settings/SettingsPanes.qml` — top-level `storage` row shipped;
  `SettingsShell.qml` registers the pane.
- Tests — `shell/tests/tst_statusmodel.cpp` (2), `shell/tests/
  tst_controlcenter.qml`, `apps/settings/tests/tst_settings_storage.{cpp,qml}`
  (new), `tst_settings_absence.qml`, `tst_settings_shell.qml`.
- Docs — `docs/design/07-system-integration.md` "The Storage pane and tile
  (T-15.2b)"; `docs/design/adr/0120-storage-pane-and-tile.md`; capture script
  `scripts/capture-t15-storage-pane.sh` + two stills; `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 17 lib + 4 bluetooth + 9 host +
  4 storage green.
- `ctest --output-on-failure -j4` (in `build/`) — 55/55.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; `cargo clippy -p dragonfruit-system-status
  --all-targets -- -D warnings` — clean.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (StatusNotifier/zoo/apppicker), unchanged.

Decisions / gotchas for later tasks:

- **Storage is a top-level Settings pane, not under `General`.** The reference
  has `General > Storage`, but `General` is unshipped (T-15.10); this keeps the
  no-half-panes rule. Move the row into General when T-15.10 lands (ADR 0120).
- **Fourth interface on the bridge host** (Bluetooth precedent, ADR 0118).
- **`DF_STORAGE_FIXTURE`** is the Settings-pane seam; `DF_STATUS_FIXTURE` now
  also drives the shell's Storage tile. Neither is set in `make e2e`.
- **Absence matrix for T-15.16**: mask the bridge host (view empty), mask
  `org.freedesktop.UDisks2` (`unavailable`), or drop the mountable volume
  (`present: false`).
- The shell menu bar has no Storage slot (no dead control); Storage is the
  Control Center tile only.
- **The fixture's eject case is order-sensitive**: the mock cannot restore an
  ejected drive, so `tst_settings_storage` names it `test_zz_…` to run last.
- The Control Center surface is a fixed 360x780; the tile is deliberately
  compact and `tst_controlcenter` asserts the content still fits.
- Live visual check: `scripts/capture-t15-storage-pane.sh`; the vision tool
  confirmed both surfaces render with no clipping.

## T115 — T-15.3a Sound and routing adapter

**State: done.** The T-07.3 audio adapter (`dragonfruit-audio`) grew the input
half and default-device routing. No new crate, no reimplementation: the same
`AudioSource` seam over WirePlumber's `pw-dump`/`wpctl`. Backend only — the
Settings pane and Control Center tile are T-15.3b.

Real paths:

- `services/audio/src/source.rs` — new `SourceData`; `AudioData` gained
  `default_source` + `sources`; `AudioSource` gained `set_default_sink(id)` /
  `set_default_source(id)`; `MockAudio` gained the two writes, the
  `default_sink_writes`/`default_source_writes` counters, and
  `default_sink_name`/`default_source_name`/`source_volume` readers.
- `services/audio/src/model.rs` — new `Source`; `AudioSnapshot` gained
  `sources` + `default_source`; `default_source_device()`/`default_source_name()`
  /`source_count()`; independent default resolution (falls back to the first
  device per list) via a private `Named` trait.
- `services/audio/src/pw_dump.rs` — decodes `Audio/Source` nodes and the
  `default.audio.source` metadata entry; `wpctl set-default <id>` for both
  routing writes.
- `services/audio/src/adapter.rs` — `set_default_sink`/`set_default_source`
  pass through `note_write` (an absent write still hides the item).
- `services/audio/src/lib.rs` — re-exports + module docs.
- `services/audio/tests/routing_path.rs` (new) — 6 integration tests.
- `services/audio/tests/fixtures/pw-dump-office.json` — added one
  `Audio/Source` node (id 150); both defaults were already in the metadata.
- `services/system-status/src/lib.rs`, `tests/host.rs` — fixture constructors
  use `..AudioData::default()` (additive fields).
- Docs — `docs/design/07-system-integration.md` "The audio routing path
  (T-15.3a)"; ADR `0121-sound-routing-adapter.md`.
- Capture — `scripts/capture-t15-audio-routing.sh`;
  `docs/captures/t15-3a-audio-routing.png`; `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-audio` — 38 lib + 6 read_path + 6 routing_path +
  6 volume_path green.
- `cargo test -p dragonfruit-system-status` — 17 lib + 4 bluetooth + 9 host +
  4 storage green.
- `make e2e` — EXIT 0 (first run hit the known flaky `dragonfruit-lock-auth`
  `helper` test; rerun green).
- `cargo fmt --all -- --check`; `cargo clippy -p dragonfruit-audio
  --all-targets -- -D warnings`; `cargo clippy -p dragonfruit-system-status
  --all-targets -- -D warnings` — clean.

Decisions / gotchas for T-15.3b:

- **Routing is default-device switching, by node id.** `set_default_sink` /
  `set_default_source` take the PipeWire node `u32` carried by `Sink.id` /
  `Source.id` and map to `wpctl set-default <id>`. Selecting a device makes it
  the default; the existing `set_volume`/`set_mute` (which target
  `@DEFAULT_AUDIO_SINK@`) then adjust it. There is no per-device volume write.
- **Per-application stream routing is NOT implemented** (out of scope): no
  `Stream/Output/Audio` decoding and no `move-stream`. Add it behind the same
  seam if a consumer needs it.
- **Independent default fallback per list.** If the named default is missing,
  the first sink/source becomes the default (unchanged T-07.3 rule, now applied
  to both lists).
- **Unknown node id is `Failed`, not `Absent`/`Denied`.** The write does not
  invent a snapshot; a failed write leaves the last snapshot live.
- **T-15.3b's bridge work is not done here.** `services/system-status` still
  serialises only `sinks` in `audio_view`; T-15.3b must extend the bridge
  (input list + routing writes) and the shell/Settings clients, following the
  Bluetooth/Storage `*Host` precedent (ADR 0118/0120).
- Live visual check: `bash scripts/capture-t15-audio-routing.sh` produced
  `docs/captures/t15-3a-audio-routing.png` (3840x2160; unique 454 797, full
  luminance sigma 95.4). Vision confirmed the nested desktop, menu bar, Dock,
  and client windows render with no blank areas or clipping.

## T116 — T-15.3b Sound and routing pane and tile

**State: done.** The Settings Sound pane and the Control Center Sound tile ship
as one unit over the T-15.3a audio adapter. Device routing, `Output volume`, and
`Mute` round-trip through the bridge host; the `Sound Effects`/`Balance` rows
are settingsd keys (new `sound` group, schema revision 11), so every control
applies live with no dead toggles. Absence is documented and tested.

Real paths:

- `services/system-status/src/lib.rs` — `audio_view` now carries `sources`,
  `sourceCount`, `defaultSource`; `StatusHost::set_default_sink` /
  `set_default_source` (+ tests).
- `services/system-status/src/dbus.rs` — `Audio` interface adds
  `SetDefaultSink(id)` / `SetDefaultSource(id)`.
- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 10 → 11; new
  `KeyGroup::Sound`; 7 keys (`sound.alertSound`, `sound.playEffectsThrough`,
  `sound.alertVolume`, `sound.playOnStartup`, `sound.uiEffects`,
  `sound.volumeFeedback`, `sound.balance`) + revision-11 test.
- `docs/settings-keys.md` — the 7 rows.
- `libs/settings-client/settingsclient.cpp` — the same 7 defaults (schema
  mirror), comment to revision 11.
- `shell/src/systemstatusclient.{h,cpp}` — the mock audio view now carries
  `sources` + the default output/input names (the shell tile reflects routing;
  it never writes it).
- `shell/src/systemstatusmodel.cpp` — the audio view decode already passes the
  wider payload through, so no model change was needed for routing.
- `shell/src/shellcontroller.{h,cpp}` — `onSoundSettingsRequested`; Control
  Center `soundSettingsRequested` connection.
- `shell/control-center/ControlCenter.qml` — Sound tile subtitle reflects the
  default output device (`audioOutputName`); `Mute` and `Sound Settings…` share
  one row; `soundSettingsRequested` signal.
- `apps/settings/SoundClient.{h,cpp}` (new) — `DbusSoundClient` +
  `MockSoundClient` (`DF_SOUND_FIXTURE`); State/SetVolume/SetMute/
  SetDefaultSink/SetDefaultSource.
- `apps/settings/SettingsBridge.{h,cpp}` — `sound`/`soundAvailable` +
  `refreshSound`/`setSoundVolume`/`setSoundMute`/`setSoundDefaultSink`/
  `setSoundDefaultSource`.
- `apps/settings/SoundPane.qml` (new) — `Sound Effects` (2 popups, alert-volume
  slider, 3 toggles) + `Output & Input` (segmented tabs, Name/Type device table,
  output volume, Mute, Balance) + absence note.
- `apps/settings/SettingsPanes.qml` (sound shipped; icon `volume`),
  `SettingsShell.qml` (sound body), `apps/settings/CMakeLists.txt`.
- Tests — `shell/tests/tst_statusmodel.cpp` (audio view decode),
  `shell/tests/tst_controlcenter.qml` (sound reflect + link; fit test now
  includes Storage and routing), `apps/settings/tests/tst_settings_sound.{cpp,
  qml}` (new), `tst_settings_absence.qml` (sound absence; counts 7),
  `tst_settings_shell.qml` (counts 7).
- Docs — `docs/design/07-system-integration.md` "The Sound pane and tile
  (T-15.3b)" + Audio interface bullet; ADR
  `docs/design/adr/0123-sound-pane-and-tile.md`; capture script
  `scripts/capture-t15-sound-pane.sh` + two stills; `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 25 lib + 9 host + 4 storage green.
- `cargo test -p dragonfruit-settingsd` — 33 lib + schema_doc green.
- `ctest --output-on-failure -j4` (in `build/`) — 56/56.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; clippy on `dragonfruit-system-status` and
  `dragonfruit-settingsd` — clean.

Decisions / gotchas for later tasks:

- **Sound Effects/Balance are settingsd keys, not adapter state.** The task left
  the providers "TBD"; no later T-15 task owns them, and the no-half-panes rule
  forbids dead controls, so a new `sound` key group (revision 11) owns the
  preference. An actual alert/UI-sound playback engine is the one deferred item
  (ADR 0123). Device selection/volume/mute stay on the adapter.
- **Three hide rules for audio:** WirePlumber absent (`unavailable`) or a
  running daemon that names no device hides the `Output & Input` group; the
  `Sound Effects`/`Balance` controls stay live on schema defaults. The tile
  still shows when `state: available` (audio has no `present` flag).
- **Input volume/mute and per-app stream routing are not implemented.** The
  Input tab shows only the device table (selecting routes the default source);
  no inert slider.
- **The Control Center panel is a fixed 360x780 and clips overflow.** The Sound
  settings link shares the `Mute` row to keep the tile one row taller than
  before; `tst_controlcenter`'s fit test now includes the Storage tile and the
  routing subtitle, and the live capture confirms the Clipboard tile is not
  clipped.
- **The Sound pane is taller than the window and scrolls.** The capture shows
  the top (`Sound Effects` + the device table); `Output volume`, `Mute`, and
  `Balance` are below the fold. `tst_settings_sound` exercises them directly.
- `DF_SOUND_FIXTURE` is the Settings-pane seam; `DF_STATUS_FIXTURE` drives the
  shell's Sound tile. Neither is set in `make e2e`.
- Live visual check: `bash scripts/capture-t15-sound-pane.sh` produced
  `docs/captures/t15-3b-sound-pane.png` (2088x1410) and
  `docs/captures/t15-3b-sound-control-center.png` (360x780). Vision confirmed
  both groups, the device table, the sliders, the toggles, the `Built-in
  Speakers` tile subtitle, and an unclipped Control Center bottom.

## T117 — T-15.4a Keyboard, Mouse, and Trackpad adapter

**State: done.** A new workspace crate `dragonfruit-input` (`services/input`)
is the libinput keyboard/mouse/trackpad adapter. It implements the T-07
contract behind an `InputSource` seam with a `MockInput`, reports state and
events, and treats absence as a normal state. It is deliberately **read-only**:
libinput persists nothing and has no setter, and the design's principle 3 / the
legacy FR-4 reserve keyboard settings for the compositor, so the user's choices
stay settingsd-owned and compositor-applied. The adapter is the device
inventory. Backend only — the Settings pane and Control Center tile are
T-15.4b.

Real paths:

- `services/input/src/lib.rs` — crate docs + exports.
- `services/input/src/source.rs` — `InputSource` seam, `InputData` /
  `InputDeviceData`, `MockInput` (`kill`/`restart`/`push`, `reads`).
- `services/input/src/libinput.rs` — `CommandLibinput` (`libinput
  list-devices`) + the total parser `InputData::from_list_devices` (mirrors the
  audio `pw-dump` CLI source).
- `services/input/src/model.rs` — `InputSnapshot`, `InputDevice`,
  `DeviceKind`, `DeviceChange`; classification, ordering, glyph/label, and the
  pure `device_changes(previous)` diff.
- `services/input/src/adapter.rs` — `InputAdapter<S>` over `Adapter` /
  `Subscription`.
- `services/input/tests/fixtures/libinput-list-devices.txt` (new) — captured
  tool fixture with a keyboard, mouse, touchpad, and touchscreen.
- `services/input/tests/read_path.rs` (new) — 5 integration tests.
- `services/system-adapters/src/state.rs` — `AdapterId::INPUT` (`"input"`) +
  its id assertion.
- `services/input/Cargo.toml`, `Cargo.toml` (workspace member), `Makefile`
  (`cargo test -p dragonfruit-input` in the test target).
- Docs — `docs/design/07-system-integration.md` "The input device path
  (T-15.4a)" + an adapters-table row; ADR `0124-input-device-adapter.md`;
  capture script `scripts/capture-t15-input.sh` → 
  `docs/captures/t15-4a-input-adapter.png`; `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-input` — 26 lib + 5 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `make e2e` — EXIT 0 (includes the new test target).
- `cargo fmt --all -- --check`; `cargo clippy -p dragonfruit-input
  --all-targets -- -D warnings`; clippy on `dragonfruit-system-adapters` —
  clean.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (app-index tray, apppicker, zoo), unchanged.

Decisions / gotchas for T-15.4b (and T-15.5a):

- **Read-only adapter.** No write methods exist by design. Keyboard/pointer
  settings are NOT this adapter's state: `input.repeatDelay`/`input.repeatRate`
  already exist in settingsd and are forwarded to the compositor over
  `df_toplevel_manager.set_input_policy` (ADR 0034). T-15.4b must add the
  pointer keys (`Tracking speed`, `Tap to click`, scrolling, …) as settingsd
  keys consumed by the compositor, the same split T-15.3b used for the `sound`
  group, or the pane would have dead controls.
- **Two hide rules**, like the battery item: libinput gone or unable to reach a
  seat → `AdapterState::Unavailable`; a running stack with **no recognized
  device** → `Available` with `InputSnapshot::present() == false`. Neither
  blocks startup.
- **Classification is capability-driven.** A pointer with gesture support or
  `Tap-to-click` is a `Touchpad`; any other pointer is a `Mouse`. There is no
  literal `touchpad` capability token.
- **The parser is total and ignore-unknown.** `libinput list-devices` is
  human-oriented ("may change its output at any time"); the fixture is the
  tripwire and unknown keys are skipped, so a tool change is a one-file fix
  behind the seam. It reports libinput **built-in defaults**, not the desktop's
  applied configuration.
- **The live source on CI/permission-limited hosts** exits 0 with no devices
  (permission denied on `/dev/input`), which projects `Available` +
  `present:false` — a normal state. It does not use the bridge host; T-15.4b
  adds the `*Host`/`*Client` wiring if it projects the inventory.
- Live visual check: `bash scripts/capture-t15-input.sh` produced
  `docs/captures/t15-4a-input-adapter.png` (3840x2160). Vision confirmed the
  nested desktop, menu bar, Dock, and client windows render with no blank
  areas, clipping, or stray artifacts.

## T118 — T-15.4b Keyboard, Mouse, and Trackpad pane and tile

**State: done.** The Settings Keyboard/Mouse/Trackpad panes and the Control
Center Keyboard tile ship as one functional unit (ADR 0125). Every preference
row is a settingsd key (revision 12) and applies live; the device list is the
read-only libinput inventory projected through the system-status bridge host.
Absence is documented and tested.

Real paths:

- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 11 → 12; 10 new
  `input.*` keys (`pointerSpeed`, `naturalScroll`, `tapToClick`, `leftHanded`,
  `scrollMethod`, `keyboardBrightness`, `adjustBrightnessLowLight`,
  `backlightOffAfter`, `keyboardNavigation`, `emojiKeyAction`) + a revision-12
  test. `docs/settings-keys.md` gained the rows and a consumer-map line.
- `libs/settings-client/settingsclient.cpp` — the schema-defaults mirror updated
  to revision 12, plus a new `coerceToSchema` used by `set`/`applyValue`: a QML
  double bound to an integer key is stored and sent as `x`, so the daemon's
  type check accepts it. This is the first pane to write integer keys from QML.
- `services/system-status/src/input.rs` (new) — `InputHost`, `input_view`,
  `input_snapshot_view`, tests.
- `services/system-status/src/lib.rs`, `dbus.rs`, `main.rs` —
  `INPUT_INTERFACE`, the read-only `InputInterface` (`State`/`Refresh`),
  `LiveInput = InputHost<CommandLibinput>`, `--print-input`, `run()` signature,
  `interface_names()` 5 → 6. `Cargo.toml` depends on `dragonfruit-input`.
- `services/system-status/tests/input.rs` (new) — bridge integration tests.
- `apps/settings/InputClient.{h,cpp}` (new) — `DbusInputClient` +
  `MockInputClient` (selected by `DF_INPUT_FIXTURE`), read-only seam.
- `apps/settings/SettingsBridge.{h,cpp}` — `input`/`inputAvailable` +
  `refreshInput`.
- `apps/settings/InputPane.qml` (new shared body) + `KeyboardPane.qml`,
  `MousePane.qml`, `TrackpadPane.qml` (new thin wrappers setting `section`).
- `apps/settings/SettingsPanes.qml` — the three panes `shipped: true` with
  `keyboard`/`mouse`/`trackpad` icons; `SettingsShell.qml` registers the three
  bodies; `CMakeLists.txt` lists the sources/QML.
- `design-system/components/Icon.qml` — original `keyboard`, `mouse`,
  `trackpad` painted glyphs.
- `shell/src/systemstatusmodel.{h,cpp}` — `input()`/`inputVisible()`/
  `applyInputJson`/`refreshInputRequested`; the `present:false` second hide rule
  now includes `input`.
- `shell/src/systemstatusclient.{h,cpp}` — `refreshInput` + `inputState`; the
  mock inventory.
- `shell/src/shellcontroller.{h,cpp}` — `onInputState`, a startup
  `refreshInput()`, the `input` push in `applyControlCenterData`, the
  `onKeyboardSettingsRequested` slot, and `kControlCenterHeight` 780 → 880.
- `shell/control-center/ControlCenter.qml` — the Keyboard tile (inventory
  summary + `Keyboard Settings…`), `keyboardSettingsRequested`.
- Tests — `apps/settings/tests/tst_settings_input.{cpp,qml}` (new);
  `tst_settings_absence.qml` (input absence, counts 10);
  `tst_settings_shell.qml` (shipped/visual counts 10);
  `shell/tests/tst_statusmodel.cpp` (input decode + refresh signal);
  `shell/tests/tst_controlcenter.qml` (8 tiles, fit at 880, keyboard tile).
- Docs — `docs/design/07-system-integration.md` "The Keyboard/Mouse/Trackpad
  pane and tile (T-15.4b)" + the Input interface bullet; ADR
  `docs/design/adr/0125-keyboard-mouse-trackpad-pane-and-tile.md`; capture
  script `scripts/capture-t15-input-pane.sh` + two stills;
  `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 5 lib + 9 host + 4 bluetooth + 4
  storage + 4 input green.
- `cargo test -p dragonfruit-settingsd` — 34 lib + schema_doc green.
- `ctest --output-on-failure -j4` (in `build/`) — 57/57.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; clippy on `dragonfruit-system-status`,
  `dragonfruit-input`, `dragonfruit-settingsd` — clean.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.

Decisions / gotchas for later tasks:

- **The compositor does not apply the pointer keys yet.** The pointer
  preferences map to `PointerSettings` in `compositor/src/input/settings.rs`,
  but `df_toplevel_manager.set_input_policy` carries only keyboard repeat and
  gesture gating. A later task extends the protocol (append-only request,
  version bump) to apply them; until then the panes persist and apply live in
  settingsd but pointer behavior is unchanged in the compositor.
- **Keyboard-brightness and emoji-key rows persist a preference only**; their
  hardware bridges are deferred (same shape as the Sound alert engine).
- **`coerceToSchema`** in the settings client is the general fix for QML integer
  writes; reuse it for any future integer/`x` key a QML control binds to.
- **The Control Center panel is a fixed 360x880 and clips overflow.** Another
  tile needs the fit test revisited.
- **Three panes share one body.** `InputPane.qml` is parameterised by `section`;
  keep new pointer rows there so Mouse and Trackpad cannot drift.
- **Absent-inventory hide rule:** libinput gone or a running stack with no
  recognized device hides the `Devices` group and shows a note; the settingsd
  rows stay live on the defaults. `DF_INPUT_FIXTURE` is the pane seam;
  `DF_STATUS_FIXTURE` drives the shell tile. Neither is set in `make e2e`.
- Live visual check: `bash scripts/capture-t15-input-pane.sh` produced
  `docs/captures/t15-4b-keyboard-pane.png` (2088x1410) and
  `docs/captures/t15-4b-input-control-center.png` (360x880). Vision confirmed
  the pane's `Devices` rows (`AT Translated Set 2 keyboard`, `Logitech USB
  Mouse`, `Synaptics TouchPad`), the `Key repeat rate`/`Delay until repeat`
  sliders, the `Keyboard Brightness` group with nothing cut off, and the
  Control Center Keyboard tile (`1 keyboards, 2 pointing devices`,
  `Keyboard Settings…`) above an unclipped Clipboard tile.

## T119 — T-15.5a Mission Control and hot corners adapter

**State: done.** A new workspace crate `dragonfruit-overview`
(`services/overview`) is the Mission Control and hot corners adapter. Mission
Control and hot corners are compositor-native — there is no external daemon: the
compositor detects corners (`compositor/src/input/hot_corners.rs`) and owns the
one overview machine (`compositor/src/overview/mod.rs`), and the shell mirrors
both over the private `df_toplevel_manager` bridge (`hot_corner`,
`overview_changed`). The adapter projects that path (never re-detects a corner
or re-runs a transition), implements the T-07 contract behind a
`MissionControlSource` seam with `MockMissionControl`, reports state plus two
event streams, and treats a missing bridge as a normal hidden state. Backend
only — the pane and tile are T-15.5b.

Real paths:

- `services/overview/src/source.rs` — `MissionControlSource` seam,
  `MissionControlData` (corner map, dwell/inset, gesture-gating trio, overview
  runtime), `MockMissionControl` (`kill`/`restart`/`push`/`trigger`/`reads`).
- `services/overview/src/model.rs` — `HotCorner` (4, wire order),
  `HotCornerAction` (5, stable ids), `GestureGating`, `OverviewState`,
  `MissionControlSnapshot` (label/glyph/reachability), the pure
  `changes(previous)` diff, `HotCornerTrigger`, `MissionControlChange`.
- `services/overview/src/adapter.rs` — `MissionControlAdapter<S>` over
  `Adapter`/`Subscription`; `drain_changes`/`drain_triggers`.
- `services/overview/tests/read_path.rs` (new) — 7 integration tests.
- `services/system-adapters/src/state.rs` — `AdapterId::MISSION_CONTROL`
  (`"mission-control"`) + its id assertion.
- `services/overview/Cargo.toml`, `Cargo.toml` (workspace member), `Makefile`
  (`cargo test -p dragonfruit-overview` in the test target).
- Docs — `docs/design/07-system-integration.md` "The Mission Control and hot
  corners path (T-15.5a)" + adapters-table row; ADR
  `0126-mission-control-hot-corners-adapter.md`; capture script
  `scripts/capture-t15-overview-adapter.sh` +
  `docs/captures/t15-5a-mission-control-adapter.png`; `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-overview` — 23 lib + 7 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `make e2e` — EXIT 0 (includes the new test target).
- `cargo fmt --all -- --check`; clippy on `dragonfruit-overview` and
  `dragonfruit-system-adapters` — clean.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.

Decisions / gotchas for T-15.5b (and later):

- **No live source ships here.** The production source is the shell's
  `df_toplevel_manager` client (C++); T-15.5b wires it through the status bridge
  host (`*Host`/`*Client`) and adds the hot-corner assignment settings keys the
  pane writes, exactly as ADR 0124 anticipated for input's bridge host.
- **The adapter is read-only.** Runtime state is the compositor's one machine;
  configuration is settingsd-owned and compositor-applied. Applying an
  assignment needs an append-only compositor-policy request, not an adapter
  write (ADR 0126).
- **`HotCornerAction::id()` is the cross-layer spelling** (`mission-control`,
  `notification-center`, `desktop-reveal`, `lock-screen`, `none`) — reuse it for
  settings key values and the wire so the pane popup maps straight on.
- **Absence is one rule** (no second `present()` hide like input): a missing
  bridge is `Unavailable` (hidden); a present-but-unreadable bridge is `Error`
  (visible inert).
- Live visual check: `bash scripts/capture-t15-overview-adapter.sh` produced
  `docs/captures/t15-5a-mission-control-adapter.png` (3840x2160). Vision
  confirmed the nested desktop renders — menu bar, Dock, Settings, and client
  windows, no blank areas, clipping, or stray artifacts.

## T120 — T-15.5b Mission Control and hot corners pane and tile

**State: done.** The Settings Mission Control & Hot Corners pane and the Control
Center Mission Control tile ship as one functional unit (ADR 0127). Four
revision-13 settingsd keys hold the corner assignments and apply live; the
gesture rows reuse the revision-1 `gestures.*` keys the compositor already
applies over `set_input_policy`. The tile is projected by the shell from the
settings values. Absence is documented and tested.

**Deviation from the T-15.5a note/ADR 0126:** no `dragonfruit-system-status`
overview host is added. Mission Control's runtime is compositor-native and the
only reader is the shell's `df_toplevel_manager` client; a services-layer host
would need an invented shell→D-Bus push bridge. The shell therefore projects
the tile locally (`shell/src/controlcenterpolicy.cpp::missionControlView`,
mirroring the Rust `MissionControlSnapshot::label()`), and the pane writes
settingsd keys. The `dragonfruit-overview` adapter stays the contract/model and
is exercised by its own crate tests; it is not linked into the shell or the
host.

Real paths:

- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 12 → 13; new
  `KeyGroup::Overview`; 4 keys `overview.hotCornerTopLeft`/`TopRight`/
  `BottomLeft`/`BottomRight` (Text, values = the `HotCornerAction::id()`
  vocabulary, defaults mirror `HotCornerConfig::default()`) + a revision-13
  test.
- `libs/settings-client/settingsclient.cpp` — the schema-defaults mirror bumped
  to revision 13 and the four defaults added.
- `design-system/components/Icon.qml` — new original `overview` painted glyph
  (a 2x2 window-thumbnail grid).
- `apps/settings/MissionControlPane.qml` (new) — the pane body: a
  `Mission Control` group (gesture toggles) and a `Hot Corners` group (four
  `Select` rows), plus a settings-daemon absence note.
- `apps/settings/SettingsPanes.qml` — `mission-control` catalog row
  (`shipped: true`, icon `overview`), after `desktop-dock`.
- `apps/settings/SettingsShell.qml`, `apps/settings/CMakeLists.txt` — register
  the body/source/QML.
- `shell/src/controlcenterpolicy.{h,cpp}` — pure `missionControlView(values)`
  (state/glyph/label/reachable/gesture/cornerCount).
- `shell/src/shellcontroller.{h,cpp}` — `missionControl` pushed in
  `applyControlCenterData`; `onMissionControlSettingsRequested` slot;
  `kControlCenterHeight` 880 → 980 (content measures 925 with the full tile
  set).
- `shell/control-center/ControlCenter.qml` — `missionControl` property, the
  `info` tile at index 8, and `missionControlSettingsRequested`.
- Tests — `apps/settings/tests/tst_settings_mission_control.{cpp,qml}` (new);
  `apps/settings/tests/tst_settings_shell.qml` and `tst_settings_absence.qml`
  (counts 10 → 11, shipped list, a mission-control absence case);
  `shell/tests/tst_controlcenterpolicy.{cpp}` (new); `shell/tests/`
  `tst_controlcenter.qml` (9 tiles, mission tile + link, fit at 980).
- Docs — `docs/design/07-system-integration.md` "The Mission Control pane and
  tile (T-15.5b)"; ADR `0127-mission-control-pane-and-tile.md`;
  `docs/settings-keys.md` rows + consumer map; capture script
  `scripts/capture-t15-mission-control-pane.sh`; `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-settingsd` — green (incl. schema_doc).
- `ctest --output-on-failure -j4` (in `build/`) — 59/59.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; clippy on `dragonfruit-settingsd` — clean.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (app-index tray, apppicker, zoo), unchanged.

Decisions / gotchas for later tasks:

- **The compositor does not apply the `overview.hotCorner*` assignments yet.**
  The gesture trio already applies over `set_input_policy`; applying a corner
  assignment needs an append-only `df_toplevel_manager.set_hot_corners` request
  (version bump, shell forward from `applyCompositorPolicy`) per ADR 0126/0127.
  Until then the keys persist and apply live in settingsd, exactly like the
  T-15.4b pointer keys.
- **The Mission Control tile is shell-native.** Do not add a system-status
  `MissionControl` interface unless a real services-layer source appears; the
  shell owns the only compositor mirror.
- **Panel height is now 980** (from 880). Another tile needs the fit test and
  every capture crop revisited.
- **`missionControlView` mirrors the Rust label exactly** (`Gesture`,
  `Gesture, n corner(s)`, `n corner(s)`, `No trigger`). Keep the two in step if
  the label format changes.
- **`overview.hotCorner*` values are the `HotCornerAction::id()` strings**
  (`none`, `mission-control`, `notification-center`, `desktop-reveal`,
  `lock-screen`); reuse them for the wire and any popup.
- Live visual check: `bash scripts/capture-t15-mission-control-pane.sh`
  produced `docs/captures/t15-5b-mission-control-pane.png` (2088x1410) and
  `docs/captures/t15-5b-mission-control-control-center.png` (360x980). Vision
  confirmed the pane's two groups and all four corner popups
  (`Mission Control`, `Notification Center`, `Desktop`, `Lock Screen`) and the
  Control Center Mission Control tile (`Gesture, 1 corner(s)`,
  `Mission Control Settings…`) above an unclipped Clipboard tile.

## T121 — T-15.6a Battery and power profiles adapter

**State: done.** `dragonfruit-power` (`services/power`) grew from the T-07.4
read-only battery item into the battery **and** power-profiles adapter. Backend
only — the pane and tile are T-15.6b (ADR 0128).

Real paths:

- `services/power/src/source.rs` — `PowerData.profiles:
  Option<PowerProfilesData>`; `PowerProfilesData` / `PowerProfileData` raws;
  `PowerDeviceData.capacity` (f64, UPower `Capacity`) and `charge_cycles`
  (i32); `ProfileOutcome` + default `PowerSource::set_active_profile`;
  `MockPower.fail_writes`/`profile_writes`.
- `services/power/src/model.rs` — `PowerProfile` (stable ids
  `power-saver`/`balanced`/`performance`), `PowerProfilesSnapshot`,
  `BatteryHealth` (documented 80% threshold), `Battery.capacity`/
  `charge_cycles`, and pure `PowerSnapshot::changes(previous) -> Vec<PowerChange>`.
- `services/power/src/adapter.rs` — previous-snapshot diffing into
  `drain_changes`; `set_active_profile` (snapshot-neutral; resyncs on absence).
- `services/power/src/upower.rs` — live source reads UPower **and**
  power-profiles-daemon, probing `net.hadess.PowerProfiles` then
  `org.freedesktop.UPower.PowerProfiles`; implements the one write.
- Tests — `services/power/tests/read_path.rs` (fixture drives battery health +
  profiles) and crate tests: 45 lib + 12 read_path.
- `services/system-status/src/lib.rs`, `tests/host.rs` — test literals updated
  for the two new `PowerDeviceData` fields + `profiles`.
- Docs — `docs/design/07-system-integration.md` power section rewritten; ADR
  `docs/design/adr/0128-battery-power-profiles-adapter.md`; capture script
  `scripts/capture-t15-power-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-power` — 45 lib + 12 read_path green.
- `cargo test -p dragonfruit-system-status` — green.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; clippy on `dragonfruit-power`,
  `dragonfruit-system-status` — clean.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.

Decisions / gotchas for T-15.6b and later:

- **Absence is per daemon.** `PowerData.profiles == None` means
  power-profiles-daemon is absent; the adapter stays `Available` and the
  battery half works. `Unavailable` only when **both** daemons are unreachable.
  Do not collapse the two.
- **`battery_view` does not project profiles yet.** T-15.6b extends the
  `dragonfruit-system-status` `Battery` view (or adds a sibling interface) with
  the profile list/active profile/degradation and a `SetActiveProfile` write.
- **Profile glyphs are not in `Icon.qml`/`StatusGlyph.qml` yet.** The adapter
  returns `power-saver`/`power-balanced`/`power-performance`; T-15.6b adds the
  painted glyphs.
- **The write is snapshot-neutral.** `set_active_profile` does not mutate the
  snapshot; the host re-reads after the daemon's `PropertiesChanged`, exactly
  as Bluetooth/storage/audio.
- **`Capacity`/`ChargeCycles` are read via `Properties.Get`** and accepted as
  double or integer, because UPower's spelling of `Capacity` has varied; the
  typed device proxy carries the rest.
- **Holds are read, not written.** `holds` counts `ActiveProfileHolds`;
  `HoldProfile`/`ReleaseProfile` are deferred until a consumer needs them.
- Live visual check: `bash scripts/capture-t15-power-adapter.sh` produced
  `docs/captures/t15-6a-power-adapter.png` (3840x2160). No surface of its own,
  so the capture confirms only that the nested desktop renders; vision found no
  blank areas, clipping, or stray artifacts.

## T122 — T-15.6b Battery and power profiles pane and tile

**State: done.** The Settings Battery pane and the Control Center Battery tile
ship as one functional unit over the T-15.6a adapter (ADR 0129). The bridge
host's `Battery` interface gains the one `SetActiveProfile` write; the pane
picks the active profile live, reads battery health/charging from UPower, and
the tile summarizes charge + active profile. Absence is layered and documented
per daemon. The Usage History range switch and chart frames render an honest
absent state because UPower exposes no charge history.

Real paths:

- `services/system-status/src/lib.rs` — `battery_view` now carries
  `health`/`healthLabel`/`capacity`/`chargeCycles`, `chargeState`,
  `profilesAvailable`, `activeProfile`/`profileLabel`, and the `profiles`
  list; new `StatusHost::set_active_profile(&str) -> Value` and
  `profile_report(&ProfileOutcome)`.
- `services/system-status/src/dbus.rs` — `BatteryInterface` gains
  `SetActiveProfile(profile)`; the interface is no longer read-only.
- `services/system-status/tests/host.rs` + `src/lib.rs` tests — profile
  read/write and absence.
- `design-system/components/Icon.qml` — new painted glyphs `battery`,
  `power-saver` (leaf), `power-balanced` (balance scale), `power-performance`
  (bolt), `info`.
- `apps/settings/BatteryClient.{h,cpp}` (new) — `DbusBatteryClient` /
  `MockBatteryClient` (`DF_BATTERY_FIXTURE`), one read + one write.
- `apps/settings/SettingsBridge.{h,cpp}` — `battery`/`batteryAvailable`,
  `refreshBattery()`, `setPowerProfile(id)`.
- `apps/settings/BatteryPane.qml` (new) — Power Mode / Battery / Usage History
  groups; `apps/settings/SettingsPanes.qml` `battery` shipped; `SettingsShell`
  body; `CMakeLists.txt`.
- `shell/src/shellcontroller.cpp` — pushes `battery` into the Control Center;
  `onBatterySettingsRequested`; `kControlCenterHeight` 980 → 1040.
- `shell/src/systemstatusclient.cpp` — the `DF_STATUS_FIXTURE` battery view
  carries health + the profile list so the tile renders in the demo.
- `shell/control-center/ControlCenter.qml` — `battery` property, visibility/
  label/glyph, tile at index 9, `batterySettingsRequested`.
- Tests — `apps/settings/tests/tst_settings_battery.{cpp,qml}` (new);
  `tst_settings_shell.qml` and `tst_settings_absence.qml` (counts 11 → 12,
  battery in the id list, a battery absence case); `tst_controlcenter.qml`
  (10 tiles, battery tile tests, fit at 1040).
- Docs — ADR `0129-battery-pane-and-tile.md`;
  `docs/design/07-system-integration.md` battery pane/tile section + interface
  row; capture script `scripts/capture-t15-battery-pane.sh`;
  `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 33 lib + 4 host green.
- `ctest --test-dir build --output-on-failure -j4` — 60/60.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; clippy on `dragonfruit-system-status` — clean.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.
- `./scripts/check-gallery-snapshots.py` — 78 snapshots green.

Decisions / gotchas for later tasks:

- **The Control Center panel is now 360x1040** (was 980). A further tile needs
  the fit test and every capture crop revisited; the T-15.5b capture script's
  crop was updated to 1040.
- **No settingsd keys were added.** The active profile lives in
  power-profiles-daemon; the pane writes it through the bridge host. Do not add
  a `battery.*` key for it.
- **Absence stays layered:** host absent → note; no battery but profiles →
  picker only; no profiles daemon → battery rows only + note.
- **UPower charge history is still unread.** The `Usage History` group's range
  switch and two chart frames are honest empty states; a history provider is
  T-15.x. `Options…`/`?` are omitted (no function → no dead controls).
- **`battery_view` is consumed by both the menu bar and the tile.**
  `normalize()` still hides the battery item when `present:false`; the Control
  Center tile deliberately ignores that and computes its own visibility from
  `present || profilesAvailable`.
- Live visual check: `bash scripts/capture-t15-battery-pane.sh` produced
  `docs/captures/t15-6b-battery-pane.png` (2088x1410) and
  `docs/captures/t15-6b-battery-control-center.png` (360x1040). Vision confirmed
  the pane's three groups (Power Mode `Balanced`, Battery `Normal`/`On Battery`,
  Usage History `Last 24 Hours`/`Not available`), the battery-level chart's
  `No battery history for the last 24 hours.` absent state, and the Battery tile
  (`82% · Balanced`, balance-scale glyph, `Battery Settings…`) above an
  unclipped Clipboard tile.

## T123 — T-15.7a Notifications and Focus adapter

**State: done.** New workspace crate `dragonfruit-notify-adapter`
(`services/notify-adapter`): a projection adapter over the **existing**
notification service (`services/notifications`), not a second service. It
reuses the service's `FocusMode` vocabulary and its shell-facing JSON views,
and it is the model the T-15.7b pane/tile bind to (ADR 0130).

Real paths:

- `services/notify-adapter/` (new crate, workspace member) —
  - `src/source.rs` — `NotificationsSource` seam; `NotificationsData` /
    `NotificationRecord`; `FocusOutcome {Applied,Absent,Failed}`; `MockNotifications`
    (`absent`/`present`/`failing`/`fail_writes`/`push`/`kill`/`restart`/`reads`/
    `focus_writes`).
  - `src/model.rs` — `NotificationsSnapshot` (`FocusSnapshot` mode/allow_list/
    batched, active banners, history), `AppNotifications` (+`status_label`),
    pure `NotificationsSnapshot::changes -> Vec<NotificationsChange>`.
  - `src/adapter.rs` — `NotificationsAdapter<S>` over the shared `Subscription`
    lifecycle; `refresh`, `set_focus_mode`, `set_focus_allow_list`,
    `drain_changes`; implements `Adapter` with `AdapterId::NOTIFICATIONS`.
  - `src/dbus.rs` — `DbusNotifications` live session-bus source
    (`new()` ambient, `at(addr)` for tests); reads `FocusPolicy()`/`Banners()`/
    `History()`, writes `SetFocusMode`/`SetFocusAllowList`. Absence = no bus, no
    owner, or a daemon that does not serve `org.dragonfruit.Notifications1`.
  - `tests/session_bus.rs` — 5 tests over a private `dbus-daemon` (read, write,
    adapter-driven, unserved-bus absence, slot id).
- `services/system-adapters/src/state.rs` — `AdapterId::NOTIFICATIONS`
  (`"notifications"`) and `AdapterId::FOCUS` (`"focus"`); the id test updated.
- `Cargo.toml` — workspace member; `Makefile` e2e now runs
  `cargo test -p dragonfruit-notify-adapter`.
- Docs — `docs/design/07-system-integration.md` adapter table row + new
  "The Notifications and Focus path (T-15.7a)" section; ADR
  `0130-notifications-focus-adapter.md`; capture script
  `scripts/capture-t15-notify-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-notify-adapter` — 31 lib + 5 session_bus green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo fmt --all -- --check`; clippy on both crates `-D warnings` — clean.
- `make e2e` — EXIT 0 (includes the new crate's tests).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (Makefile GNOME Calculator, app-index tray, apppicker, zoo), unchanged.

Decisions / gotchas for T-15.7b and later:

- **Reuse is load-bearing.** Do not add a queue/history/policy to the adapter.
  The service owns them; if per-app policy grows, it grows in
  `services/notifications` and the adapter's read view.
- **The global notification prefs have no owner yet.** `Show previews`, `when
  display is sleeping`, `when screen is locked`, `when mirroring` are not in
  the notification service; T-15.7b adds them as settingsd keys. Do not put
  them on the adapter.
- **The adapter is standalone; T-15.7b adds the host seam.** Mirror
  bluetooth/storage/battery: add `services/system-status/src/notifications.rs`
  (a `NotificationsHost` + JSON `notifications_view`) and a
  `org.dragonfruit.SystemStatus1.Notifications` interface, then the C++
  settings client/tile. `AdapterId::NOTIFICATIONS`/`FOCUS` are already in the
  contract.
- **Writes are snapshot-neutral.** `set_focus_mode`/`set_focus_allow_list`
  leave the snapshot alone on success; the host re-reads after the service's
  `Changed`. Only an absent write resyncs the adapter to `Unavailable`.
- **The live source treats a foreign notification daemon as absence**, not
  error: a daemon that owns `org.freedesktop.Notifications` but lacks
  `org.dragonfruit.Notifications1` yields `Ok(None)` (hidden), so a dunst/mako
  session never shows a broken item.
- Live visual check: `bash scripts/capture-t15-notify-adapter.sh` produced
  `docs/captures/t15-7a-notify-adapter.png` (3840x2160). No surface of its own,
  so the capture confirms only that the nested desktop renders; vision found
  the menu bar, Dock, wallpaper, and windows composited without clipping or
  stray artifacts (the one dark rectangle it flagged is the host screen outside
  the nested output, matching T121's capture).

## T124 — T-15.7b Notifications and Focus pane and tile

**State: done.** The Settings Notifications and Focus panes and the Control
Center Focus tile ship as one functional unit over the T-15.7a adapter
(ADR 0131). The bridge host grows a `Notifications` interface; the four global
presentation preferences are settingsd keys (revision 14); the two panes bind
the same adapter view, so a Focus write in one reflects in the other and in the
existing Control Center Focus tile. Absence is layered and documented.

Real paths:

- `services/system-status/src/notifications.rs` (new) — `NotificationsHost`
  (new/refresh/view/state/set_focus_mode/set_focus_allow_list),
  `notifications_view` / `notifications_snapshot_view`, `focus_report`; six
  unit tests. `lib.rs` re-exports them and adds `NOTIFICATIONS_INTERFACE`.
- `services/system-status/src/dbus.rs` — `LiveNotifications`,
  `NotificationsInterface` (`State`/`Refresh`/`SetFocusMode`/`SetFocusAllowList`),
  `run` takes the host, `interface_names()` is 7.
- `services/system-status/Cargo.toml` — depends on `dragonfruit-notify-adapter`.
- `services/system-status/src/main.rs` — `--print-notifications`, live host.
- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 13 → 14; new
  `KeyGroup::Notifications` (ALL 12 → 13) and four keys
  (`notifications.showPreviews` text allowed always/when-unlocked/never;
  `notifications.showWhenSleeping` false; `notifications.showWhenLocked` true;
  `notifications.showWhenMirroring` false); new revision-14 test.
- `docs/settings-keys.md` — the four rows + a consumer-map row.
- `libs/settings-client/settingsclient.cpp` — `settingsSchemaDefaults()` mirrors
  revision 14.
- `apps/settings/NotificationsClient.{h,cpp}` (new) — `DbusNotificationsClient`
  / `MockNotificationsClient` (`DF_NOTIFICATIONS_FIXTURE`), one read + two
  writes; `setFocusApp` in the bridge is the whole-list replace.
- `apps/settings/SettingsBridge.{h,cpp}` — `notifications`/
  `notificationsAvailable`, `refreshNotifications()`, `setFocusMode(id)`,
  `setFocusApp(name, allowed)`; `SettingsBridge.cpp` gained
  `<QRegularExpression>`.
- `apps/settings/NotificationsPane.qml`, `apps/settings/FocusPane.qml` (new);
  `SettingsPanes.qml` ships both (`notifications` icon `bell` with the header
  description, `focus` icon `focus`); `SettingsShell.qml` registers both bodies;
  `CMakeLists.txt`.
- `design-system/components/Icon.qml` — new painted `bell` glyph.
- Tests — `apps/settings/tests/tst_settings_notifications.{cpp,qml}` (new);
  `tst_settings_shell.qml` (shipped 12 → 14, search list), `tst_settings_absence.qml`
  (shipped 12 → 14, id list, two absence cases), test target in
  `apps/settings/tests/CMakeLists.txt`.
- Docs — ADR `0131-notifications-pane-and-tile.md`;
  `docs/design/07-system-integration.md` T-15.7b section + interface row;
  capture script `scripts/capture-t15-notifications-pane.sh`;
  `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 39 lib (six new) + 4 test binaries green.
- `cargo test -p dragonfruit-settingsd` — green (schema_doc included).
- `ctest --test-dir build --output-on-failure -j4` — 61/61.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; clippy on `system-status`/`notify-adapter`/
  `settingsd` `--all-targets -D warnings` — clean.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.
- `./scripts/check-gallery-snapshots.py` — 78 snapshots green.

Decisions / gotchas for later tasks:

- **The Focus mode + allow list are adapter state, not settingsd.** Only the
  four presentation prefs are settingsd keys. Do not move the mode/allow list
  into settingsd.
- **The four prefs are stored policy, not yet enforced.** The service does not
  read them to gate a banner; enforcement is a follow-up (see `## Follow-ups`).
- **Repeater delegate width.** Both new panes' dynamic delegates use
  `width: parent.width`, not `<repeater>.width` (a Repeater is non-visual and
  has no width); the wrong form collapses the rows to width 0 and they vanish.
  The pane tests assert `itemAt(0).width > 0` to catch it.
- **The Control Center tile is the existing T-11.3b Focus tile.** It reads the
  notification service's `FocusPolicy` via the shell's `NotificationClient`; the
  panes write the same service through the system-status host, so the tile
  reflects the same state. No second tile was added and the shell was untouched.
- **Absence is layered:** no bridge host or a foreign notification daemon hides
  the inventory/Focus controls with a note; the settingsd prefs stay live.
- Live visual check: `bash scripts/capture-t15-notifications-pane.sh` produced
  `docs/captures/t15-7b-notifications-pane.png` (2088x1410),
  `t15-7b-focus-pane.png` (2088x1410), and `t15-7b-control-center.png`
  (360x1040). The first capture caught a real bug (the per-app rows rendered at
  width 0); after the fix vision confirmed the Application Notifications rows
  (chat/Mail/Pager with Default/Default/Allowed), the Focus segmented control
  selecting `Focus` with its summary, the Allowed Apps rows with the Pager
  toggle on, and the Control Center panel with the Focus tile (off — no live
  notification service on the host).

## T125 — T-15.8a Lock Screen policy adapter

**State: done.** New workspace crate `dragonfruit-lock-adapter`
(`services/lock-adapter`): a projection over the **session idle/lock engine**
and the **compositor lock state**, not a second timer or a second lock. It
reuses `dragonfruit-session`'s `IdlePolicy`/`IdleStage` vocabulary and the
T-07 adapter contract; it is the model T-15.8b's pane/tile bind to (ADR 0132).

Real paths:

- `services/lock-adapter/` (new crate, workspace member) —
  - `src/source.rs` — `LockPolicySource` seam; `LockPolicyData` (`locked`,
    `idle: IdlePolicy`, `display: LockDisplay`); `LockDisplay`
    (`show_user_name_and_photo`/`show_password_hints`/`show_message_when_locked`/
    `message`/`show_power_buttons`, with the shipped defaults);
    `MockLockPolicy` (`absent`/`present`/`failing`/`push`/`kill`/`restart`/
    `reads`).
  - `src/model.rs` — `LockState` (`Unlocked`/`Locked`), `LockDisplayOption`
    (stable ids are the settings key suffixes), `LockPolicySnapshot`
    (`from_data`, `is_locked`, `glyph`, `display_off_after`, `lock_after`,
    `display_option`, `message`), `LockPolicyChange` (state / per-stage delay /
    display option / message), `IDLE_STAGES`.
  - `src/adapter.rs` — `LockPolicyAdapter<S>` over the shared `Subscription`
    lifecycle; `refresh`, `drain_changes`; implements `Adapter` with
    `AdapterId::LOCK`.
  - `src/lib.rs` — re-exports `dragonfruit_session::{IdlePolicy, IdleStage}`.
  - `tests/read_path.rs` — 7 acceptance tests.
- `services/system-adapters/src/state.rs` — `AdapterId::LOCK` (`"lock"`); id
  test updated.
- `Cargo.toml` — workspace member; `Makefile` e2e runs
  `cargo test -p dragonfruit-lock-adapter`.
- Docs — `docs/design/07-system-integration.md` adapter-table row + new "The
  Lock Screen policy path (T-15.8a)" section; ADR
  `0132-lock-screen-policy-adapter.md`; capture script
  `scripts/capture-t15-lock-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-lock-adapter` — 22 lib + 7 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo fmt --all -- --check`; clippy on both crates `--all-targets -D warnings`
  — clean.
- `make e2e` — EXIT 0.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.
- `./scripts/check-gallery-snapshots.py` — 78 snapshots green.

Decisions / gotchas for T-15.8b and later:

- **Reuse is load-bearing.** The adapter depends on `dragonfruit-session` for
  `IdlePolicy`/`IdleStage`; do not re-model the idle chain or re-time a stage.
- **Read-only adapter.** The compositor owns lock, the session owns timing, and
  the display preferences are settingsd's. T-15.8b declares the `lock.*` keys
  (likely revision 15) and the engine applies them live; no write on the
  adapter.
- **No live D-Bus source.** Lock state and idle policy are not on the bus; the
  concrete source is the shell's `df_toplevel_manager` client. T-15.8b is
  expected to be shell-native + settingsd keys (T-15.5b Mission Control
  precedent), not a system-status host.
- **Linux adaptation:** one `blank` delay, not the macOS battery/AC pair.
- Live visual check: `bash scripts/capture-t15-lock-adapter.sh` produced
  `docs/captures/t15-8a-lock-adapter.png` (3840x2160); no surface of its own,
  so it only confirms the nested desktop renders. Vision found the menu bar,
  Dock, wallpaper, Settings window, and X11 demo window composited with no
  blank regions or stray artifacts.

## T126 — T-15.8b Lock Screen policy pane and tile

**State: done.** The Settings Lock Screen pane and the Control Center Lock
Screen tile ship as one functional unit, shell-native plus settingsd keys
(ADR 0133), exactly as T-15.5b Mission Control anticipated. Lock policy has no
external daemon, so the pane writes only settingsd keys and the shell projects
the tile locally; the compositor lock hot path is never touched.

Real paths:

- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 14 → 15; new
  `KeyGroup::Lock` (`lock`, ALL 13 → 14) and five keys:
  `lock.showUserNameAndPhoto` (bool, true), `lock.showPasswordHints` (bool,
  false), `lock.showMessageWhenLocked` (bool, false), `lock.message` (text, ""),
  `lock.showPowerButtons` (bool, true). New
  `the_lock_screen_keys_are_declared_in_revision_fifteen` test.
- `docs/settings-keys.md` — the five rows + a Lock Screen consumer-map row.
- `libs/settings-client/settingsclient.cpp` — `settingsSchemaDefaults()` mirrors
  revision 15 (comment updated).
- `apps/settings/LockScreenPane.qml` (new) — the pane: display-off and
  require-password `Select` rows bound to the **reused** `idle.blank`/`idle.lock`
  keys, an inline yellow-triangle energy warning, the four `lock.*` toggles, a
  `Set...` `Dialog` writing `lock.message`, and a "When Switching User" note.
- `apps/settings/SettingsPanes.qml` — `lock-screen` shipped `true`, icon
  `lock`; `SettingsShell.qml` registers the body; `CMakeLists.txt` adds the pane
  to `QML_FILES` and `df_qml_lint`.
- `design-system/components/Icon.qml` — new painted `lock` padlock glyph
  (added to `paintedGlyphs`).
- `shell/src/controlcenterpolicy.{h,cpp}` — pure `lockPolicyView(values)`:
  `{state:"available", glyph:"lock", label, requirePassword, lockSeconds}`;
  `idle.lock == 0` → `No password required`, else `Password after <duration>`
  (`5 s`/`10 min`/`1 h`).
- `shell/control-center/ControlCenter.qml` — `lockPolicy` property, computed
  `lockScreenVisible`/`lockScreenLabel`, the `tiles` entry (11th, after Battery),
  the tile QML, and the `lockScreenSettingsRequested` signal.
- `shell/src/shellcontroller.{h,cpp}` — connects and handles
  `onLockScreenSettingsRequested` (logs until T-16), and pushes `lockPolicy` in
  `applyControlCenterData`. `kControlCenterHeight` 1040 → 1120 (new tile).
- Tests — `apps/settings/tests/tst_settings_lock_screen.{cpp,qml}` (new, 8
  cases); `tst_settings_absence.qml` (shipped 14 → 15, id list, new lock-screen
  absence case); `tst_settings_shell.qml` (shipped 14 → 15, search list);
  `shell/tests/tst_controlcenterpolicy.cpp` (3 new `lockPolicyView` cases);
  `shell/tests/tst_controlcenter.qml` (11 tiles, lock tile + fit at 1120).
  Test target added to `apps/settings/tests/CMakeLists.txt`.
- Docs/scripts — ADR `0133-lock-screen-pane-and-tile.md`;
  `docs/design/07-system-integration.md` T-15.8b subsection;
  `scripts/capture-t15-lock-pane.sh`; `docs/captures/README.md`.
  Three older Control Center capture scripts updated to 360×1120.

Commands that work (repo root):

- `cargo test -p dragonfruit-settingsd` — green (incl. `schema_doc`).
- `ctest --test-dir build --output-on-failure -j4` — 62/62.
- `make e2e` — EXIT 0.
- `cargo fmt --all -- --check`; `make check-design-tokens check-tokens
  check-no-capture-grab` — clean; `./scripts/check-gallery-snapshots.py` — 78
  green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (unchanged from T124/T125).

Decisions / gotchas for later tasks:

- **Timing keys are reused, not duplicated.** The pane binds `idle.blank` /
  `idle.lock` (revision 5). Do not add `lock.displayOffAfter`; the session idle
  engine already reads the `idle.*` keys live.
- **The pane writes keys, never the adapter.** `dragonfruit-lock-adapter`
  stays read-only; there is no system-status host for lock.
- **`lock.*` display keys are stored policy.** The lock-screen renderer does
  not read them yet (see `## Follow-ups`); the timing keys already apply.
- **The `lock` glyph is new in `Icon.qml`.** It is a painted glyph; add future
  lock-related marks to the Canvas, not the bar list.
- **Panel height is now 360×1120.** Any later tile must re-check the
  `test_panel_content_fits_the_shell_surface` assertion and bump the constant,
  test, and capture-script crop together.
- Live visual check: `bash scripts/capture-t15-lock-pane.sh` produced
  `docs/captures/t15-8b-lock-screen-pane.png` (2088x1410) and
  `t15-8b-lock-screen-control-center.png` (360x1120). Vision confirmed the pane
  rows (For 5 minutes / After 10 minutes; Show user name and photo ON,
  password hints OFF, message OFF, power buttons ON; Set… enabled*), the
  "When Switching User" note, and the Control Center Lock Screen tile
  (`Password after 10 min`, link present) with no clipping, overlap, or stray
  artifacts. (*Vision read the disabled `Set…` as enabled; the unit test
  `test_set_message_button_gates_on_the_toggle` asserts it is disabled while
  the toggle is off — it renders dimmed.)

## T127 — T-15.9a Menu Bar configuration adapter

**State: done.** New workspace crate `dragonfruit-menubar-adapter`
(`services/menubar-adapter`) projects the **shell menu bar** and the
**menu-broker**, not a second bar or menu resolver. The menu bar is
shell-native (no external daemon), so the adapter faces the state the shell
already publishes and reuses the T-07 contract, exactly like
`dragonfruit-overview` and `dragonfruit-lock-adapter` (ADR 0134).

Real paths:

- `services/menubar-adapter/` (new crate, workspace member) —
  - `src/source.rs` — `MenuBarSource` seam; `MenuBarData` (`hidden`,
    `auto_hide`, `show_background`, `global_menu`, `clock`, `controls[7]`);
    `MenuBarControlState` (`ABSENT`/`PRESENT`/`INERT`, `is_visible`/
    `is_enabled`, default absent); `MockMenuBar`
    (`absent`/`present`/`failing`/`push`/`kill`/`restart`/`reads`).
  - `src/model.rs` — `MenuBarAutoHide` (`never`/`always`/`full-screen`,
    default `full-screen`); `MenuBarControl`
    (`clock`/`wifi`/`bluetooth`/`battery`/`volume`/`focus`/`accessibility`,
    ids = shell status-item ids); `ClockOption` (`showDate`/`showSeconds`) +
    `ClockOptions`; `MenuBarSnapshot` (`from_data`, `is_hidden`,
    `clock_option`, `control`, `visible_controls`, `label`, `glyph` =
    `"menu-bar"`); `MenuBarChange` (visibility / auto-hide / background /
    global-menu / clock option / control slot).
  - `src/adapter.rs` — `MenuBarAdapter<S>` over the shared `Subscription`
    lifecycle; `refresh`, `drain_changes`; implements `Adapter` with
    `AdapterId::MENU_BAR`.
  - `tests/read_path.rs` — 8 acceptance tests.
- `services/system-adapters/src/state.rs` — `AdapterId::MENU_BAR`
  (`"menu-bar"`); id test updated.
- `Cargo.toml` — workspace member; `Makefile` e2e runs
  `cargo test -p dragonfruit-menubar-adapter`.
- Docs — `docs/design/07-system-integration.md` adapter-table row + new "The
  Menu Bar configuration path (T-15.9a)" section; ADR
  `0134-menu-bar-configuration-adapter.md`; capture script
  `scripts/capture-t15-menubar-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-menubar-adapter` — 29 lib + 8 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo fmt --all -- --check`; clippy on both crates `--all-targets
  -D warnings` — clean.
- `make e2e` — EXIT 0.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean.
- `./scripts/check-gallery-snapshots.py` — 78 snapshots green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new crate); unchanged from T125/T126.

Decisions / gotchas for T-15.9b and later:

- **Reuse is load-bearing.** The adapter has no dependency beyond the
  dependency-free adapter contract; it re-models nothing the shell renders.
  Do not add a second bar model or a menu resolver.
- **Read-only adapter.** The durable preferences are settingsd's (the keys
  land in T-15.9b) and the shell applies them live. T-15.9b declares the
  `menu.*`/clock keys and wires the pane/tile shell-native (T-15.5b /
  T-15.8b precedent); no write on the adapter.
- **Control ids are the shell status-item ids.** `MenuBarControl::Sound`
  carries the audio adapter's id `volume`; `Clock` is `clock`. Use these for
  the settings key values and the wire.
- **Per-control absence is not adapter absence.** A missing control daemon is
  an absent `MenuBarControlState` slot inside an `Available` snapshot (only
  that slot hides); the whole adapter is `Unavailable` only when the menu-bar
  bridge is missing.
- **Not modelled:** richer clock options and Apple-only controls (see
  `## Follow-ups`); the default `MenuBarData` is a fully-populated bar
  (`controls = [PRESENT; 7]`).
- Live visual check: `bash scripts/capture-t15-menubar-adapter.sh` produced
  `docs/captures/t15-9a-menubar-adapter.png` (3840x2160). No surface of its
  own, so it confirms only that the nested desktop renders; vision found the
  menu bar, Dock, wallpaper, and open windows composited with no blank
  regions, clipping, z-order issues, or stray artifacts.

## T128 — T-15.9b Menu Bar configuration pane and tile

**State: done.** The Settings Menu Bar pane and the Control Center Menu Bar
tile ship as one functional unit, shell-native plus settingsd keys (ADR 0135),
exactly as T-15.9a anticipated. The bar is shell-native (no external daemon),
so the pane writes only settingsd keys, the shell applies what has a runtime,
and the shell projects the tile locally. The menu-broker and the bar hot path
are never touched.

Real paths:

- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 15 → 16; new
  `KeyGroup::Menu` keys: `menu.autoHide` (text, `full-screen`),
  `menu.showBackground` (bool, true), `menu.recentItems` (int, 10, 0–50),
  `menu.clock.showDate` (bool, true), `menu.clock.showSeconds` (bool, false),
  `menu.control.wifi/bluetooth/battery/sound/focus/accessibility` (bool, true).
  New `the_menu_bar_keys_are_declared_in_revision_sixteen` test.
- `docs/settings-keys.md` — the eleven rows + a Menu Bar pane consumer-map row.
- `libs/settings-client/settingsclient.cpp` — `settingsSchemaDefaults()` mirrors
  revision 16 (comment updated).
- `apps/settings/MenuBarPane.qml` (new) — behavior group (auto-hide `Select`,
  background `Toggle`, recent-items `Select`), the `Menu Bar Controls` group
  with its description, a `Clock` row whose `Clock Options…` `Dialog` holds the
  two clock toggles, and a `Repeater` of six per-control toggles bound to the
  `menu.control.*` keys.
- `apps/settings/SettingsPanes.qml` — `menu-bar` shipped `true`, icon
  `menu-bar`; `SettingsShell.qml` registers the body; `CMakeLists.txt` adds the
  pane to `QML_FILES` and `df_qml_lint`.
- `design-system/components/Icon.qml` — new painted `menu-bar` glyph (added to
  `paintedGlyphs`).
- `shell/menubar/MenuBar.qml` — new `showBackground` property; `color` becomes
  `showBackground ? Theme.color.chrome : "transparent"`.
- `shell/src/controlcenterpolicy.{h,cpp}` — new pure `menuBarView(values)`
  (`{state, glyph:"menu-bar", label, autoHide, showBackground, globalMenu}`);
  label mirrors `MenuBarSnapshot::label()`.
- `shell/src/shellcontroller.{h,cpp}` — `applyMenuBarPolicy` now also applies
  the clock options and background live (with change tracking so an unrelated
  settings change does not re-render); `applyStatusItems` gates each status item
  on its `menu.control.<id>` key (additive with adapter availability);
  `applyControlCenterData` pushes `menuBar`; `onMenuBarSettingsRequested` logs
  until T-16; `kControlCenterHeight` 1120 → 1160.
- `shell/control-center/ControlCenter.qml` — `menuBar` property,
  `menuBarVisible`/`menuBarLabel`, the 12th tile (after Lock Screen), the tile
  QML, the `menuBarSettingsRequested` signal, and inter-tile `spacing` `sm` →
  `xs` so the taller panel fits the nested output.
- Tests — `apps/settings/tests/tst_settings_menu_bar.{cpp,qml}` (new, 6 cases);
  `tst_settings_absence.qml` (shipped 15 → 16, id list, new menu-bar absence
  case); `tst_settings_shell.qml` (shipped 15 → 16, search list);
  `shell/tests/tst_controlcenterpolicy.cpp` (3 new `menuBarView` cases);
  `shell/tests/tst_controlcenter.qml` (12 tiles, menu bar tile + link, fit at
  1160). Test target added to `apps/settings/tests/CMakeLists.txt`.
- Docs/scripts — ADR `0135-menu-bar-pane-and-tile.md`;
  `docs/design/07-system-integration.md` T-15.9b subsection;
  `scripts/capture-t15-menubar-pane.sh`; `docs/captures/README.md`. The four
  earlier Control Center capture scripts updated 1120 → 1160.

Commands that work (repo root):

- `cargo test -p dragonfruit-settingsd` — green (incl. `schema_doc`).
- `cargo clippy -p dragonfruit-settingsd --all-targets -- -D warnings` — clean.
- `ctest --test-dir build --output-on-failure -j4` — 63/63.
- `make e2e` — EXIT 0. (One run hit a pre-existing flake in
  `dragonfruit-lock-auth --test helper` (`a_missing_user_is_a_usage_error`,
  `BrokenPipe` writing the password after the helper exits); the test passes
  5/5 in isolation and the next full run was green. Unrelated to T128.)
- `cargo fmt --all -- --check`; `make check-design-tokens check-tokens
  check-no-capture-grab` — clean; `./scripts/check-gallery-snapshots.py` — 78
  green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new work); unchanged from T125/T126/T127.

Decisions / gotchas for later tasks:

- **The adapter stays read-only.** The pane writes settingsd keys; the shell
  applies them. There is no system-status host for the menu bar.
- **Control ids are the shell status-item ids.** `menu.control.<id>` uses the
  adapter's `MenuBarControl::id()` spellings (`wifi`, `bluetooth`, `battery`,
  `volume`, `focus`, `accessibility`); note `Sound` → `volume`.
- **Per-control visibility is additive with adapter availability.** A control
  shows only when its daemon is present *and* the key is true; a missing daemon
  still hides only its slot.
- **`menu.autoHide`/`menu.recentItems` are stored policy** (shell does not
  consume them yet); clock options, background, and control visibility apply
  live.
- **Panel height is at the nested-output ceiling.** 360×1160 fits the
  1920×1200 nested output (1164 below the bar). Any further tile needs a
  scrolling panel or a taller output — do not just bump the constant.
- Live visual check: `bash scripts/capture-t15-menubar-pane.sh` produced
  `docs/captures/t15-9b-menu-bar-pane.png` (2088x1410) and
  `t15-9b-menu-bar-control-center.png` (360x1160). The Settings window opens at
  the fixed 800×640 default, so the pane scrolls; the committed pane capture
  shows the behavior group, the `Menu Bar Controls` header, `Clock Options…`,
  and Wi-Fi/Bluetooth/Battery/Focus/Sound, with Accessibility just below the
  fold. A scrolled verification capture (`/tmp` only) confirmed the
  `Accessibility` toggle renders and works, so nothing is clipped inside the
  pane. Vision confirmed the Control Center tile (`In Full Screen Only`, link
  present) and that the panel's last tile is fully visible with no clipping or
  overlap. The pane capture is otherwise clean: no missing text or stray
  artifacts inside the Settings window.

## T129 — T-15.10a General, About, and Updates adapter

**State: done.** New workspace crate `dragonfruit-update-adapter`
(`services/update-adapter`) projects the **host stack**: the host identity
(About/General) and the distribution **update provider** (Updates) behind a
seam. The adapter never reimplements package management — the concrete
`SystemProvider` is distro-specific and belongs with packaging
(T-16.9/T-16.10), exactly as `docs/design/12-packaging.md` puts it — so this
crate ships the `UpdateProvider` trait, `MockUpdateProvider`, and a live
`HostSystem` identity read (ADR 0136).

Real paths:

- `services/update-adapter/` (new crate, workspace member) —
  - `src/source.rs` — `SystemSource` seam; `SystemIdentity` (host name,
    `PRETTY_NAME`/`VERSION_ID`/`ID`, kernel, architecture, DMI
    model/serial, processor, memory; `os_label`/`memory_label`/`device_name`/
    `has_serial`); `UpdatePhase` (`idle`/`checking`/`up-to-date`/`available`/
    `installing`/`reboot-required`/`failed`), `UpdateSeverity`
    (`normal`/`important`/`security`), `UpdateItem`, `UpdateData`;
    `SystemData { identity, updates: Option<UpdateData> }`; `UpdateOutcome`
    (`Applied`/`Absent`/`Failed`); `MockSystem` (`absent`/`present`/`failing`/
    `fail_writes`/`push`/`kill`/`restart`/`reads`/`checks`/`installs`/`reboots`).
  - `src/model.rs` — `SystemSnapshot` (`from_data`, `updates_available`,
    `phase`, `is_busy`, `is_reboot_required`, `updates`, `update_count`,
    `security_count`, `last_checked_ms`, `message`, `label`, `glyph` =
    `"software-update"`); `UpdateChange` (identity / provider presence / phase /
    list size / last checked / message).
  - `src/adapter.rs` — `UpdateAdapter<S>` over the shared `Subscription`
    lifecycle; `refresh`, `check`, `install`, `reboot`, `drain_changes`;
    implements `Adapter` with `AdapterId::UPDATES`.
  - `src/host.rs` — `HostSystem` (live `SystemSource`, root-configurable
    identity read from `/etc/os-release`, `/etc/hostname`,
    `/proc/sys/kernel/osrelease`, `/proc/cpuinfo`, `/proc/meminfo`,
    `/sys/devices/virtual/dmi/id/*`, `std::env::consts::ARCH`) + the
    `UpdateProvider` trait and `MockUpdateProvider`; pure
    `parse_os_release`/`parse_cpu_model`/`parse_mem_total`.
  - `tests/read_path.rs` — 8 acceptance tests.
- `services/system-adapters/src/state.rs` — `AdapterId::UPDATES`
  (`"updates"`); id test updated.
- `Cargo.toml` — workspace member; `Makefile` e2e runs
  `cargo test -p dragonfruit-update-adapter`.
- Docs/scripts — ADR `0136-general-about-updates-adapter.md`;
  `docs/design/07-system-integration.md` adapter-table row + new "The General,
  About, and Updates path (T-15.10a)" section; capture script
  `scripts/capture-t15-update-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-update-adapter` — 40 lib + 8 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo clippy -p dragonfruit-update-adapter --all-targets -- -D warnings` —
  clean; `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new crate); unchanged from T125–T128.

Decisions / gotchas for T-15.10b and later:

- **Absence is layered, like the battery adapter.** `SystemData::updates` is
  `Option`; a reachable host with no update provider is `Available` with
  `updates: None` and only the update controls disable. The whole adapter is
  `Unavailable` only when neither identity nor provider is reachable. A present
  provider that cannot be read is `Error` (visible, inert with the message).
- **The provider is a seam, not an implementation.** Do not add a package
  manager to this crate; attach a `Box<dyn UpdateProvider>` to `HostSystem`
  (or have the bridge host read one). `MockSystem`'s writes answer `Absent`
  when `SystemData::updates` is `None`, matching the layered absence.
- **`AdapterId::UPDATES` is `"updates"`**, not `"general"`. The crate covers
  General/About/Updates but the daemon-backed half and the tile are Updates.
- **The identity read is real and root-configurable** (`HostSystem::with_root`),
  so host tests can use a fixture tree; `HostSystem::new()` reads `/` with no
  provider.
- **Writes invent no snapshot.** `check`/`install`/`reboot` forward one request
  and return an outcome; the host must `refresh()` after the provider pushes.
- **T-15.10b:** the Settings pane is `General` (About + Software Update
  disclosure rows) and the Control Center tile is `Software Update`. The
  durable presentation preferences (if any) are settingsd keys; per the T-15.9b
  note the panel is at the nested-output height ceiling, so a new tile needs a
  scrolling panel or a taller nested output.
- Live visual check: `bash scripts/capture-t15-update-adapter.sh` produced
  `docs/captures/t15-10a-update-adapter.png` (3840x2160). No surface of its
  own, so it confirms only that the nested desktop renders; vision found the
  menu bar (Dragonfruit, Wi-Fi/battery/clock), the translucent Dock, the
  wallpaper, and the open Settings/X11 windows composited with no blank
  regions, clipping, or stray artifacts.

## T130 — T-15.10b General, About, and Updates pane and tile

**State: done.** The Settings `General` pane and the Control Center `Software
Update` tile ship as one functional unit over the T-15.10a adapter. Because the
host identity is a real read and the provider is a distro seam, the pair rides
the `org.dragonfruit.SystemStatus1` bridge host (ADR 0137), not settingsd —
there are **no new settingsd keys** (About is a live read; the update writes are
explicit actions, exactly like Storage).

Real paths:

- `services/system-status/src/updates.rs` (new) — `UpdatesHost<S>` (refresh/
  view/state/check/install/reboot) + pure `updates_view`/`updates_snapshot_view`
  + `update_report`. The view carries the identity (`hostName`, `deviceName`,
  `osLabel`, `kernel`, `architecture`, `processor`, `memoryLabel`,
  `serial`/`hasSerial`) and, when present, the provider (`updatesAvailable`,
  `phase`, `label`, `busy`, `rebootRequired`, `updateCount`, `securityCount`,
  `lastCheckedMs`, `message`, `updates[]`).
- `services/system-status/src/lib.rs` — `pub mod updates`, re-exports, and
  `UPDATES_INTERFACE = "org.dragonfruit.SystemStatus1.Updates"`.
- `services/system-status/src/dbus.rs` — `LiveUpdates = UpdatesHost<HostSystem>`;
  `UpdatesInterface` with `State`/`Refresh`/`Check`/`Install`/`Reboot`;
  `run(...)` takes the updates host; `interface_names()` is now 8.
- `services/system-status/src/main.rs` — `UpdatesHost::new(HostSystem::new())`
  (identity live, provider absent), `--print-updates`.
- `services/system-status/tests/updates.rs` (new) — 5 bridge acceptance tests.
- `services/system-status/Cargo.toml` — depends on `dragonfruit-update-adapter`.
- `services/update-adapter/src/host.rs` — `UpdateProvider` now `: Send` (the
  bridge host serves it from D-Bus worker threads; all implementors already
  were). This is the only T-15.10a crate change.
- `apps/settings/UpdatesClient.{h,cpp}` (new) — `UpdatesClient` seam;
  `DbusUpdatesClient` over the `Updates` interface; `MockUpdatesClient`
  (`DF_UPDATES_FIXTURE`) with `resetForTest()`.
- `apps/settings/SettingsBridge.{h,cpp}` — `updates`/`updatesAvailable`
  properties, `updatesChanged`, `refreshUpdates`/`checkUpdates`/
  `installUpdates`/`rebootUpdates`/`resetUpdatesFixture` invokables.
- `apps/settings/GeneralPane.qml` (new) — header card is the shell's
  `PaneHeader` (catalog description); grouped disclosure rows `About` (→ About
  This System dialog: computer glyph, device name, processor, Memory, Serial
  number when present, OS) and `Software Update` (row description = live update
  summary; dialog with `Check for Updates`/`Install`/`Restart` and the offered
  list). Absence note when the host is missing.
- `apps/settings/SettingsPanes.qml` — `general` shipped `true` + description;
  `SettingsShell.qml` registers the body; `CMakeLists.txt` adds the client,
  pane, and `df_qml_lint`.
- `design-system/components/Icon.qml` — two new painted glyphs: `general` (a
  gear) and `software-update` (a download/update arrow into a tray). Both were
  missing before (the General catalog row used a blank `general`).
- Shell: `systemstatusclient.{h,cpp}` (Updates methods + mock fixture),
  `systemstatusmodel.{h,cpp}` (`updates()`, `updatesVisible()`,
  `applyUpdatesJson`, request signals), `shellcontroller.{h,cpp}`
  (`onUpdatesState`, `applyControlCenterData` pushes `updates`, tile action and
  settings-link slots, `onStatusReport` refreshes updates), and
  `shell/control-center/ControlCenter.qml` (13th tile `software-update` with the
  state-chosen action link + `General Settings…`).
- Tests — `apps/settings/tests/tst_settings_general.{cpp,qml}` (new, 6 cases);
  `tst_settings_absence.qml` (shipped 16 → 17, id list, general absence case);
  `tst_settings_shell.qml` (shipped 16 → 17, search list, default index 3 → 4);
  `shell/tests/tst_controlcenter.qml` (13 tiles, fit at 1160, updates cases);
  `shell/tests/tst_statusmodel.cpp` (3 updates cases).
- Docs/scripts — ADR `0137-general-about-updates-pane-and-tile.md`;
  `docs/design/07-system-integration.md` D-Bus bullet + T-15.10b subsection;
  capture script `scripts/capture-t15-general-pane.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 46 lib + 10 host + 4 input + 4
  storage + 5 updates (integration) green.
- `cargo test -p dragonfruit-update-adapter` — 40 lib + 8 read_path green.
- `cargo clippy -p dragonfruit-update-adapter -p dragonfruit-system-status
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `ctest --test-dir build --output-on-failure -j4` — 64/64.
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t130.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new work); unchanged from T125–T129.

Decisions / gotchas for T-15.11b and later:

- **The bridge host serves `Updates`, not the adapter being linked.** No C++
  target links a Rust adapter. The pane and tile both decode the same JSON view
  (`SystemStatusModel` for the shell; `UpdatesClient` for Settings).
- **Absence is layered (ADR 0136/0137).** The view is `unavailable` only when
  neither identity nor provider is reachable; a reachable host with no provider
  is `available` with `updatesAvailable: false` (About live, update controls
  disabled). The tile stays visible whenever the host answers; the Settings pane
  shows the absence note only when the host is absent.
- **`UpdateProvider: Send`** was added in `services/update-adapter/src/host.rs`
  so `HostSystem` can live in the bridge host's `Mutex` across zbus threads.
  Adding a provider type only needs `Send` (all concrete providers should be).
- **No settingsd keys.** Do not add presentation keys for General: the update
  writes are explicit actions over the host stack, and the About rows are reads.
- **Control Center panel is at the ceiling.** T-15.10b fit the 13th tile by
  setting the tile padding to `Theme.primitive.spacing.xs` for every Control
  Center tile (`Theme.controls.settingsGroup.padding` is gone from
  `ControlCenter.qml`). Content is ~1097 px within 1140 px. A 14th tile needs a
  scrolling panel or a taller nested output.
- **`DF_UPDATES_FIXTURE`** selects the Settings mock and is process-global; the
  test calls `Settings.resetUpdatesFixture()` in `init()`. `DF_STATUS_FIXTURE`
  drives the shell's mock (which also has the updates fixture state).
- Live visual check: `bash scripts/capture-t15-general-pane.sh` produced
  `docs/captures/t15-10b-general-pane.png` (2088x1410) and
  `docs/captures/t15-10b-general-control-center.png` (360x1160). Vision
  confirmed the General header card and the `About` / `Software Update` rows
  with `1 Update Available`, and the Control Center's 13 tiles plus Clipboard
  with the Software Update tile (`1 Update Available`, `Install`, `General
  Settings…`) fully visible and no clipping or overlap.

## T131 — T-15.11a Users and Groups adapter

**State: done.** New workspace crate `dragonfruit-account-adapter`
(`services/account-adapter`) projects the **host stack**: AccountsService
(`org.freedesktop.Accounts`) for the user list and a distribution **group
provider** seam for groups, because AccountsService has no group API. It
reuses both and never reimplements account or group management (ADR 0138).

Real paths:

- `services/account-adapter/` (new crate, workspace member) —
  - `src/source.rs` — `AccountSource` seam; `AccountType`
    (`standard`/`administrator`; AccountsService codes 0/1), `PasswordMode`
    (`regular`/`none`/`set-at-login`/`empty`; codes 0–3), `AccountData`
    (uid, login/real name, account type, password mode, home, shell, email,
    language, icon, locked, system account, automatic login, login time,
    x session), `GroupData` (name, gid, members, system), `AccountsData
    { users, groups: Option<Vec<GroupData>> }`, `AccountOutcome`
    (`Applied`/`Denied`/`Absent`/`Failed`); `MockAccounts`
    (`absent`/`present`/`failing`/`deny_writes`/`fail_writes`/`push`/`kill`/
    `restart` + per-method write counters) that mutates its simulated stack so
    a re-read sees a write. Group writes are `Absent` when `groups` is `None`.
  - `src/model.rs` — `Account` (display name, uppercase initial, admin/locked/
    system, avatar), `Group`, `AccountsSnapshot` (`from_data` orders human
    users before system accounts and user groups before system ones;
    `human_users`/`system_users`/`groups`/`groups_available`/`user_by_uid`/
    `user_by_name`/`group_by_name`/`present`/`human_count`/`admin_count`/
    `locked_count`/`automatic_login_user`/`label`/`glyph` = `"users"`);
    `AccountsChange` (user added/removed/changed, automatic-login changed,
    group provider appeared/vanished, group added/removed/changed).
  - `src/adapter.rs` — `AccountsAdapter<S>` over the shared `Subscription`
    lifecycle; `refresh`, the five account writes (`create_user`,
    `delete_user`, `set_account_type`, `set_locked`, `set_automatic_login`),
    the three group writes (`create_group`, `delete_group`,
    `set_group_members`), `drain_changes`; implements `Adapter` with
    `AdapterId::ACCOUNTS`.
  - `src/accounts.rs` — live `HostAccounts` (`AccountSource`): `ListCachedUsers`
    at `/org/freedesktop/Accounts` then
    `Properties.GetAll("org.freedesktop.Accounts.User")` per user; writes
    `CreateUser`/`DeleteUser` (manager) and
    `SetAccountType`/`SetLocked`/`SetAutomaticLogin` (user object); pure
    `account_from_props` + `user_path`. `GroupProvider` trait
    (`status`/`create_group`/`delete_group`/`set_members`) + `MockGroupProvider`;
    `HostAccounts::new()` runs with no provider (groups absent) until one is
    attached, exactly like `HostSystem`'s update provider.
  - `tests/fixtures/accounts-workstation.json` — one admin + one locked
    standard user + root, and wheel/users groups.
  - `tests/read_path.rs` — 12 acceptance tests.
- `services/system-adapters/src/state.rs` — `AdapterId::ACCOUNTS`
  (`"accounts"`); id test updated.
- `Cargo.toml` — workspace member; `Makefile` e2e runs
  `cargo test -p dragonfruit-account-adapter`.
- Docs/scripts — ADR `0138-users-and-groups-adapter.md`;
  `docs/design/07-system-integration.md` adapter-table row + new "The Users and
  Groups path (T-15.11a)" section; capture script
  `scripts/capture-t15-account-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-account-adapter` — 45 lib + 12 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo clippy -p dragonfruit-account-adapter -p dragonfruit-system-adapters
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t131.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (all in app-index/shell/scripts; none in the new crate); unchanged from
  T125–T130.

Decisions / gotchas for T-15.11b and later:

- **Absence is layered.** The adapter is `Unavailable` only when neither
  AccountsService nor the group provider is reachable. `AccountsService`
  present with no cached user is `Available` with `present()` false; a running
  host with **no group provider** is `Available` with `groups: None`, so only
  the group controls disable. Do not collapse these into one hide rule.
- **A write never changes the adapter state.** Unlike the single-daemon storage
  adapter, an `Absent` group write (no provider) must not hide the live user
  list; the host re-reads after the daemon/provider publishes. The pane's group
  controls are the only thing that disables when `groups_available()` is false.
- **The group provider is a seam, not an implementation.** Do not parse
  `/etc/group` and do not add a group manager to this crate; attach a
  `Box<dyn GroupProvider>` to `HostAccounts` (or have the bridge host read one).
  `GroupProvider: Send` so it can live in the bridge host's `Mutex`.
- **`AdapterId::ACCOUNTS` is `"accounts"`.** The account writes are
  AccountsService method calls authorized by polkit, so `AccountOutcome::Denied`
  is surfaced per action without degrading the read state.
- **No settingsd keys.** Account and group management are explicit actions over
  the host stack; the pane's read-only fields are reads. T-15.11b adds the pane
  and tile; per the T-15.10b note the Control Center panel is at the nested
  output ceiling, so a 14th tile needs a scrolling panel or a taller nested
  output.
- Live visual check: `bash scripts/capture-t15-account-adapter.sh` produced
  `docs/captures/t15-11a-account-adapter.png` (3840x2160). No surface of its
  own, so it confirms only that the nested desktop renders; vision found the
  menu bar, Dock, wallpaper, and the composited windows with no blank regions,
  clipping, missing text, or stray artifacts.

## T132 — T-15.11b Users and Groups pane and tile

**State: done.** The Settings `Users & Groups` pane and the Control Center
`Users` tile ship as one functional unit over the T-15.11a adapter, through the
bridge host (ADR 0139), not settingsd — there are **no new settingsd keys**
(account/group management are explicit actions; the fields are reads). The tile
is a read-only summary.

Real paths:

- `services/system-status/src/accounts.rs` (new) — `AccountsHost<S>` (refresh/
  view/state + `create_user`/`delete_user`/`set_account_type`/`set_locked`/
  `set_automatic_login`/`create_group`/`delete_group`/`set_group_members`) +
  pure `accounts_view`/`accounts_snapshot_view` + `account_report`. The view
  carries `glyph`/`label`/`present`/`userCount`/`humanCount`/`systemCount`/
  `adminCount`/`lockedCount`/`groupsAvailable`/`groupCount`/`automaticLogin`
  (`User`)/`automaticLoginUid`/`users[]`/`groups[]`. An unknown account-type id
  is a `failed` report, not a guessed `Standard`.
- `services/system-status/src/lib.rs` — `pub mod accounts`, re-exports, and
  `ACCOUNTS_INTERFACE = "org.dragonfruit.SystemStatus1.Accounts"`.
- `services/system-status/src/dbus.rs` — `LiveAccounts =
  AccountsHost<HostAccounts>`; `AccountsInterface` with `State`/`Refresh`/the
  eight writes; `run(...)` takes the accounts host; `interface_names()` is now
  9.
- `services/system-status/src/main.rs` — `AccountsHost::new(HostAccounts::new())`
  (AccountsService live, no group provider), `--print-accounts`.
- `services/system-status/tests/accounts.rs` (new) — 7 bridge acceptance tests.
- `services/system-status/Cargo.toml` — depends on
  `dragonfruit-account-adapter`.
- `apps/settings/AccountsClient.{h,cpp}` (new) — `AccountsClient` seam;
  `DbusAccountsClient` over the `Accounts` interface; `MockAccountsClient`
  (`DF_ACCOUNTS_FIXTURE`) with a simulated user/group stack that mutates on
  every write and `resetForTest()`.
- `apps/settings/SettingsBridge.{h,cpp}` — `accounts`/`accountsAvailable`
  properties, `accountsChanged`, `refreshAccounts`, the eight write invokables,
  `resetAccountsFixture`.
- `apps/settings/UsersGroupsPane.qml` (new) — user list rows (circular initial
  avatar, display name, `Admin`/`Standard` + `Locked`, info button), `Add
  User…`/`Add Group…` buttons, `Automatically log in as` popup, group list
  (name, member count, info button), plus the per-user dialog (account type,
  Disabled, Automatically Log In, Delete User with confirmation), the add-user
  dialog, the per-group dialog (membership toggles, Delete Group), the add-group
  dialog, and the absence / no-group-provider notes.
- `apps/settings/SettingsPanes.qml` — `users-groups` shipped `true`, icon
  `users`; `SettingsShell.qml` registers the body.
- `apps/settings/CMakeLists.txt` (module + `df_qml_lint`) and
  `apps/settings/tests/CMakeLists.txt` (`tst_settings_users`).
- `design-system/components/Icon.qml` — new painted `users` glyph (two
  silhouettes).
- Shell: `systemstatusclient.{h,cpp}` (read-only `refreshAccounts` +
  `accountsState`, mock fixture), `systemstatusmodel.{h,cpp}` (`accounts()`,
  `accountsVisible()`, `applyAccountsJson`, `requestRefreshAccounts`),
  `shellcontroller.{h,cpp}` (`onAccountsState`, startup/status-report refresh,
  `applyControlCenterData` pushes `accounts`, `onUsersSettingsRequested`),
  `shell/control-center/ControlCenter.qml` (14th tile `users` with the state
  label + `Users & Groups Settings…` link).
- Tests — `apps/settings/tests/tst_settings_users.{cpp,qml}` (new, 9 cases);
  `tst_settings_absence.qml` (shipped 17 → 18, id list, users-groups absence
  case); `tst_settings_shell.qml` (shipped 17 → 18, search list);
  `shell/tests/tst_controlcenter.qml` (14 tiles, fit test, users cases);
  `shell/tests/tst_statusmodel.cpp` (3 accounts cases).
- Docs/scripts — ADR `0139-users-groups-pane-and-tile.md`;
  `docs/design/07-system-integration.md` D-Bus bullet + T-15.11b subsection;
  capture script `scripts/capture-t15-users-pane.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status` — 55 lib (9 accounts) + 7 accounts
  integration + the other integration suites green.
- `cargo test -p dragonfruit-account-adapter` — green (unchanged).
- `cargo clippy -p dragonfruit-system-status -p dragonfruit-account-adapter
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `ctest --test-dir build --output-on-failure -j4` — 65/65.
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t132.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new work); unchanged from T125–T131.

Decisions / gotchas for T-15.12a and later:

- **The tile is read-only; the pane owns the writes.** `SystemStatusClient`
  gains only `refreshAccounts`; the eight writes live in the Settings
  `AccountsClient`. Do not add tile writes.
- **Absence is layered (ADR 0138/0139).** The view is `unavailable` only when
  neither AccountsService nor the group provider is reachable; a reachable host
  with no group provider is `available` with `groupsAvailable: false` (users
  live, only group controls disabled, pane shows a distinct note). The tile
  stays visible whenever the host answers.
- **No settingsd keys.** Do not add presentation keys for Users & Groups.
- **A write never invents a snapshot.** The host re-reads after each action;
  `MockAccountsClient` mutates its simulated stack so a round-trip is
  observable headlessly.
- **`DF_ACCOUNTS_FIXTURE`** selects the Settings mock and is process-global; the
  test calls `Settings.resetAccountsFixture()` in `init()`. The shell fixture is
  `DF_STATUS_FIXTURE` (its mock now emits an accounts view too).
- **The pane derives `selectedUser`/`selectedGroup` from the live view by
  uid/name** (not a stored copy), so an open dialog converges after a write.
  Do not revert to storing the entry object.
- **Control Center padding compacted to `xxs`.** 14 tiles fit at ~1108 px
  inside 1140 px. A 15th tile needs a taller nested output (no pointer-axis
  forwarding, ADR 0139).
- Live visual check: `bash scripts/capture-t15-users-pane.sh` produced
  `docs/captures/t15-11b-users-pane.png` (2088x1410) and
  `docs/captures/t15-11b-users-control-center.png` (360x1160). Vision confirmed
  the pane's selected `Users & Groups` sidebar row, the `Dan Doe, Admin` /
  `Sam Smith, Standard · Locked` rows, both `Add User…`/`Add Group…` buttons,
  the `Automatically log in as` value `Dan Doe`, the `wheel` (1 member) /
  `users` (2 members) groups, and no clipped/blank/overlapping text; and the
  panel's 14 tiles through `Users` (`2 Users`, `Users & Groups Settings…`) plus
  Clipboard with the bottom fully visible and no artifacts.

## T133 — T-15.12a Printers and Scanners adapter

**State: done.** New workspace crate `dragonfruit-printer-adapter`
(`services/printer-adapter`) projects the **host stack** over the T-07 adapter
contract, with two independent halves, both reused, never reimplemented
(ADR 0140).

Real paths:

- `services/printer-adapter/src/source.rs` — `PrinterState` (CUPS `3`/`4`/`5` +
  `Unknown`; unrecognized code is `Unknown`, never `Idle`), `PrintJobData`
  (`{id, user, size}` — `lpstat` prints no job title), `PrinterData` (name,
  display name, make/model, location, URI, state, state message,
  accepting/enabled flags, is_default, jobs), `ScannerKind`
  (`flatbed`/`sheetfed`/`handheld`/`unknown`),
  `ScanDeviceData` (device, verbatim description, kind), `PrintData
  { printers: Option<Vec<..>>, default_printer, scanners: Option<Vec<..>> }`,
  `PrintOutcome` (`Applied`/`Denied`/`Absent`/`Failed`), `PrintSource`, and
  `MockPrint` (layered `kill_cups`/`restart_cups` + `kill_sane`/`restart_sane`,
  `push`, `deny_writes`/`fail_writes`, write counters, and a simulated queue
  that mutates on apply so a re-read sees a write).
- `services/printer-adapter/src/model.rs` — `PrintJob`, `Printer`, `Scanner`,
  `PrintSnapshot::from_data` (default queue first, then queues by name;
  scanners by kind then device), `printers_available()`/`scanners_available()`,
  `present()` (any queue or scanner), `label()` (`"2 Printers, 1 Scanner"`),
  `glyph()` = `"printer"`, counts, `default_printer()`, and the pure
  `changes(previous)` diff (`PrintChange`: daemon presence, queue/scanner
  add/remove/change, default destination, job add/remove/change).
- `services/printer-adapter/src/adapter.rs` — `PrinterAdapter<S>` over the
  shared `Subscription`; `refresh`, `set_default_printer`,
  `set_printer_accepting_jobs`, `cancel_job`, `drain_changes`; implements
  `Adapter` with `AdapterId::PRINTER`.
- `services/printer-adapter/src/printers.rs` — live `HostPrint`; the printers
  half is one `lpstat -l -t` per read via `printers_from_lpstat`; the scanners
  half is `scanimage -L` via `scanners_from_scanimage`; writes are
  `lpadmin -d`, `cupsaccept`/`cupsreject`, `cancel`. A non-zero write exit is
  classified `Denied` (policy/polkit wording) or `Failed`.
- `services/printer-adapter/src/lib.rs` — crate docs + re-exports.
- `services/printer-adapter/tests/fixtures/printers-workstation.json` — two
  queues + one scanner; `tests/read_path.rs` — 12 acceptance tests.
- `services/system-adapters/src/state.rs` — `AdapterId::PRINTER`
  (`"printer"`); id test updated.
- `Cargo.toml` workspace member; `Makefile` e2e runs
  `cargo test -p dragonfruit-printer-adapter`.
- Docs/scripts — ADR `0140-printers-and-scanners-adapter.md`;
  `docs/design/07-system-integration.md` table row + new "The printers and
  scanners path (T-15.12a)" section; `scripts/capture-t15-printer-adapter.sh` +
  `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-printer-adapter` — 41 lib + 12 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo clippy -p dragonfruit-printer-adapter -p dragonfruit-system-adapters
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t133.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new crate); unchanged from T125–T132.

Decisions / gotchas for T-15.12b and later:

- **Two halves, layered absence.** `printers: None` = CUPS absent (print half
  disables only); `scanners: None` = SANE absent (scanner half disables only);
  the adapter is `Unavailable` only when **both** are absent. CUPS with no
  queue is `Some(vec![])` and `printers_available()` is true. Do not collapse
  these into one hide rule.
- **CUPS is read through its client tools, not a library.** `HostPrint` runs
  `lpstat -l -t`; the output churn is pinned in `printers_from_lpstat` and must
  stay behind the `PrintSource` seam. No libcups/IPP reimplementation.
- **`lpstat` prints no make/model and no job title.** `make_and_model` stays
  empty on the live read and `PrintJob` is `{id, user, size}`; the field/type
  exist for fixtures and a future IPP read. Do not guess.
- **Writes are explicit and per-request.** `set_default_printer`,
  `set_printer_accepting_jobs`, `cancel_job`; a `Denied` does not degrade the
  read state. A `Failed`/`Absent` does not invent a snapshot — the host
  re-reads. The T-15.12b bridge host should follow the account adapter's
  pattern (a `PrinterHost` in `services/system-status`, interface
  `org.dragonfruit.SystemStatus1.Printers`, and a `PrinterClient` seam in
  Settings).
- **No settingsd keys here.** `Default paper size` and any presentation
  preference are T-15.12b's to declare. Adding a printer (discovery +
  `lpadmin -p …`) is not in this adapter; decide that flow in T-15.12b and
  extend the adapter rather than reimplementing CUPS.
- **`DF_*` fixture convention.** The mock is `MockPrint`; T-15.12b should add a
  `DF_PRINTERS_FIXTURE` seam on the Settings client and a shell fixture like
  the other T-15 adapters.
- Live visual check: `bash scripts/capture-t15-printer-adapter.sh` (now
  captures the nested compositor's active window) produced
  `docs/captures/t15-12a-printer-adapter.png` (2115x1437). Vision found the
  menu bar with status items, the Dock, the wallpaper, and the Settings/demo
  windows, with no blank areas, clipping, or artifacts.

## T134 — T-15.12b Printers and Scanners pane and tile

**State: done.** The Settings `Printers & Scanners` pane and the Control Center
`Printers` tile ship as one functional unit over the T-15.12a adapter, through
the bridge host (ADR 0141), not by linking the Rust crate. One new settingsd
key (`printers.defaultPaperSize`); the queue/scanner state stays CUPS/SANE and
the three queue writes are explicit actions. The tile is a read-only summary.

Real paths:

- `services/system-status/src/printers.rs` (new) — `PrintersHost<S>` (refresh/
  view/state + `set_default_printer`/`set_printer_accepting_jobs`/`cancel_job`)
  + pure `printers_view`/`printers_snapshot_view` + `printers_report`. The view
  carries `glyph`/`label`/`present`/`printersAvailable`/`scannersAvailable`/
  `printerCount`/`scannerCount`/`queuedJobCount`/`defaultPrinter`/`printers[]`/
  `scanners[]`.
- `services/system-status/src/lib.rs` — `pub mod printers`, re-exports, and
  `PRINTERS_INTERFACE = "org.dragonfruit.SystemStatus1.Printers"`.
- `services/system-status/src/dbus.rs` — `LivePrinters =
  PrintersHost<HostPrint>`; `PrintersInterface` with `State`/`Refresh`/
  `SetDefaultPrinter`/`SetPrinterAcceptingJobs`/`CancelJob`; `run(...)` takes the
  printers host (`#[allow(clippy::too_many_arguments)]`); `interface_names()` is
  now 10.
- `services/system-status/src/main.rs` — `PrintersHost::new(HostPrint::new())`,
  `--print-printers`.
- `services/system-status/tests/printers.rs` (new) — 5 bridge acceptance tests.
- `services/system-status/Cargo.toml` — depends on `dragonfruit-printer-adapter`.
- `services/settingsd/src/schema.rs` — `SCHEMA_VERSION` 16 → 17;
  `KeyGroup::Printers`; `printers.defaultPaperSize` (Text, default `us-letter`,
  allowed `us-letter`/`us-legal`/`a3`/`a4`/`a5`, since 17) + declaration test.
- `docs/settings-keys.md` — the new row (the schema-doc test parses it).
- `apps/settings/PrintersClient.{h,cpp}` (new) — `PrintersClient` seam;
  `DbusPrintersClient` over the `Printers` interface; `MockPrintersClient`
  (`DF_PRINTERS_FIXTURE`) with a simulated two-queue/one-scanner stack that
  mutates on every write and `resetForTest()`.
- `apps/settings/SettingsBridge.{h,cpp}` — `printers`/`printersAvailable`
  properties, `printersChanged`, `refreshPrinters`, `setDefaultPrinter`,
  `setPrinterAcceptingJobs`, `cancelPrinterJob`, `resetPrintersFixture`.
- `apps/settings/PrintersScannersPane.qml` (new) — `Default printer` popup
  (CUPS default; `Last Printer Used` is the value when CUPS names none),
  `Default paper size` popup (settingsd), printer rows (state dot + message +
  per-printer disclosure), scanner rows, per-half absence notes, and the
  per-printer detail dialog (status/model/location, accept-jobs toggle, jobs
  with Cancel, `Set as Default Printer`). No Add button (see deviations).
- `apps/settings/SettingsPanes.qml` — `printers` shipped `true`, icon
  `printer`; `SettingsShell.qml` registers the body.
- `apps/settings/CMakeLists.txt` (module + `df_qml_lint`) and
  `apps/settings/tests/CMakeLists.txt` (`tst_settings_printers`).
- `design-system/components/Icon.qml` — new painted `printer` and `scanner`
  glyphs.
- Shell: `systemstatusclient.{h,cpp}` (read-only `refreshPrinters` +
  `printersState`, mock fixture), `systemstatusmodel.{h,cpp}` (`printers()`,
  `printersVisible()`, `applyPrintersJson`, `requestRefreshPrinters`,
  the `present` second-hide rule), `shellcontroller.{h,cpp}` (`onPrintersState`,
  startup/status-report refresh, `applyControlCenterData` pushes `printers`,
  `onPrintersSettingsRequested`), `shell/control-center/ControlCenter.qml`
  (15th tile `printers` with the count label + `Printers & Scanners Settings…`
  link).
- Tests — `apps/settings/tests/tst_settings_printers.{cpp,qml}` (new, 6 cases);
  `tst_settings_absence.qml` (shipped 18 → 19, id list, printers absence case);
  `tst_settings_shell.qml` (shipped 18 → 19, list); `shell/tests/
  tst_controlcenter.qml` (15 tiles, fit test, printers cases);
  `shell/tests/tst_statusmodel.cpp` (3 printers cases).
- Docs/scripts — ADR `0141-printers-scanners-pane-and-tile.md`;
  `docs/design/07-system-integration.md` D-Bus bullet + T-15.12b subsection;
  capture script `scripts/capture-t15-printers-pane.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status -p dragonfruit-printer-adapter
  -p dragonfruit-settingsd` — green (system-status 62 lib + 5 printers
  integration; settingsd 39 lib + schema_doc).
- `cargo clippy -p dragonfruit-system-status -p dragonfruit-printer-adapter
  -p dragonfruit-settingsd --all-targets -- -D warnings` — clean;
  `cargo fmt --all -- --check` — clean.
- `ctest --test-dir build --output-on-failure -j4` — 66/66.
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t134.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new work); unchanged from T125–T133.

Decisions / gotchas for T-15.13a and later:

- **One settingsd key only.** `printers.defaultPaperSize` (rev 17). Do not add
  keys for the default printer, queue state, or jobs: CUPS owns them and the
  pane's controls are explicit adapter actions.
- **Absence is layered (ADR 0140/0141).** `printers: None` = CUPS absent (print
  half marks absent only); `scanners: None` = SANE absent (scanner half marks
  absent only); the view is `unavailable` only when **both** are absent. The
  tile hides on `unavailable` or `present: false` (no queue and no device); the
  pane shows a per-half note.
- **A write never invents a snapshot.** The host re-reads after each action;
  `MockPrintersClient` mutates its simulated queue so a round-trip is observable
  headlessly.
- **`DF_PRINTERS_FIXTURE`** selects the Settings mock and is process-global;
  the test calls `Settings.resetPrintersFixture()` in `init()`. The shell
  fixture is `DF_STATUS_FIXTURE` (its mock now emits a printers view too).
- **The pane derives `selectedPrinter` from the live view by name**, so an open
  dialog converges after a write.
- **Control Center content gap is now `xxs` (was `xs`) and the printers tile's
  own internal gap is `xs`**; 15 tiles fit the fixed 360×1160 surface. A 16th
  tile needs a taller nested output (no pointer-axis forwarding, ADR 0139/0141).
- **Follow-up: `Add Printer, Scanner, or Fax…`.** The reference row is not
  shown; there is no CUPS discovery seam in the adapter (`lpinfo -v` + `lpadmin
  -p …`). Adding it belongs in `dragonfruit-printer-adapter`, not the pane.
- Live visual check: `bash scripts/capture-t15-printers-pane.sh` produced
  `docs/captures/t15-12b-printers-pane.png` (2088x1410) and
  `docs/captures/t15-12b-printers-control-center.png` (360x1160). Vision
  confirmed the pane's selected `Printers & Scanners` sidebar row, the
  `Default printer` value `Canon MF230`, `Default paper size` `US Letter`, the
  `Canon MF230` (`Idle, Last Used · Default`) and `HP LaserJet` (`Printing`)
  rows, and the `Epson GT-1500 flatbed scanner` (`Flatbed`) row with no
  clipped/blank/overlapping text; and the panel's 15 tiles through Printers
  (`2 Printers, 1 Scanner`, `Printers & Scanners Settings…`) plus Clipboard with
  the bottom fully visible and no artifacts.

## T135 — T-15.13a Privacy and Security adapter

**State: done.** New workspace crate `dragonfruit-privacy-adapter`
(`services/privacy-adapter`) projects the **xdg-desktop-portal
PermissionStore** over the T-07 adapter contract, with state, events, and
absence (ADR 0142). Reused, never reimplemented.

Real paths:

- `services/privacy-adapter/src/source.rs` — `AppPermissionData {app,
  permissions}`, `ResourceData {id, apps}`, `TableData {table, resources}`,
  `PrivacyData {tables}`, `PrivacyOutcome` (`Applied`/`Denied`/`Absent`/
  `Failed`), `PrivacySource` (`read` + `set_permission` + `delete_permission`),
  and `MockPrivacy` (`absent`/`present`/`failing`, `kill`/`restart`, `push`,
  `deny_writes`/`fail_writes`, read/write counters, and a simulated store that
  mutates on apply so a re-read sees a write).
- `services/privacy-adapter/src/model.rs` — `PermissionState`
  (`allowed`/`denied`/`ask`/`unset`; `from_permissions` maps `yes`/`no`/`ask`,
  anything else stays `Unset` and the raw strings are preserved),
  `AppPermission`, `PermissionResource`, `PermissionCategory` (one portal
  table; `summary()` = `None`/`1 app`/`N apps`), `PrivacySnapshot::from_data`
  (always emits **all 14 `KNOWN_TABLES` categories** in curated order, empty
  ones included; unknown tables kept last), `app_count`/`granted_count`/
  `denied_count`/`present`/`label` (`N Apps`)/`glyph` = `privacy`, and the pure
  `changes(previous)` diff (`PrivacyChange`: ResourceAdded/Removed,
  AppAdded/Removed/Changed).
- `services/privacy-adapter/src/adapter.rs` — `PrivacyAdapter<S>` over the
  shared `Subscription`; `refresh`, `set_permission`, `delete_permission`,
  `drain_changes`; implements `Adapter` with `AdapterId::PRIVACY`.
- `services/privacy-adapter/src/privacy.rs` — live `HostPrivacy` over the
  **session bus**: `org.freedesktop.impl.portal.PermissionStore` at
  `/org/freedesktop/impl/portal/PermissionStore`. One read = `List(table)` +
  `Lookup(table, id)` per id for each known table (only tables with entries are
  carried); writes = `SetPermission(table, true, id, app, as)` and
  `DeletePermission(table, id, app)`. `apps_from_lookup` is the pure decoder.
- `services/privacy-adapter/src/lib.rs` — crate docs + re-exports.
- `services/privacy-adapter/tests/fixtures/permission-store-workstation.json` —
  4 tables (devices/location/notifications/screencast), 6 app permissions;
  `tests/read_path.rs` — 11 acceptance tests.
- `services/system-adapters/src/state.rs` — `AdapterId::PRIVACY` (`"privacy"`);
  id test updated.
- `Cargo.toml` workspace member; `Makefile` e2e runs
  `cargo test -p dragonfruit-privacy-adapter`.
- Docs/scripts — ADR `0142-privacy-and-security-adapter.md`;
  `docs/design/07-system-integration.md` table row + new "The privacy and
  security path (T-15.13a)" section; `docs/design/08-settings.md` Privacy row;
  `scripts/capture-t15-privacy-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-privacy-adapter` — 38 lib + 11 read_path green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo clippy -p dragonfruit-privacy-adapter -p dragonfruit-system-adapters
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t135.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new crate); unchanged from T125–T134.

Decisions / gotchas for T-15.13b and later:

- **The host stack is the portal PermissionStore, not Secret Service/polkit.**
  The adapter is the portal-permission projection only: `org.freedesktop.impl
  .portal.PermissionStore` on the **session bus** at
  `/org/freedesktop/impl/portal/PermissionStore`. The reference's Secret
  Service (passkeys) and polkit (App Management) services are not in this
  adapter; a later task may add a read-only Secret Service projection.
- **The categories are the portal tables, not macOS rows.** `KNOWN_TABLES`
  pins the fourteen `xdg-desktop-portal` tables (`devices`=Camera,
  `location`, `notifications`, `screencast`, `remote-desktop`, `screenshot`,
  `background`, `usb`, `input-capture`, `gamemode`, `inhibit`, `realtime`,
  `wallpaper`, `desktop-used-apps`). Do not invent Calendars/Photos/etc. with
  no Linux host owner. The `devices` table's camera id is `camera`.
- **Every known table is always a category row** (empty ones show `None`); the
  snapshot model injects the missing empty categories, so the live read only
  carries tables with entries.
- **The value is a tristate with raw strings preserved.** `yes`/`no`/`ask` →
  `Allowed`/`Denied`/`Ask`; a location accuracy pair or empty list → `Unset`
  (never a guess). Keep `AppPermission.permissions` verbatim.
- **Absence is single-layered.** `read() = Ok(None)` = no session bus or no
  store name owner → hidden (`Unavailable`, normal). Empty store is
  `Available`; the tile/pane hide rule is `snapshot.present()` (any app
  permission). A store that owns its name but cannot be read is `Error`,
  visible and inert.
- **A write never invents a snapshot.** The store publishes the result and the
  host re-reads; `MockPrivacy` mutates its simulated tables so a round-trip is
  observable headlessly.
- **`DF_*` fixture convention.** T-15.13b should add a `DF_PRIVACY_FIXTURE`
  seam on the Settings client and a shell fixture like the other T-15 adapters,
  plus the bridge host pattern (`PrivacyHost` in `services/system-status`,
  interface `org.dragonfruit.SystemStatus1.Privacy`) — through the host, not by
  linking the Rust crate.
- **No `org.gnome.*`/`org.kde.*` strings in code.** `check-desktop-names.sh`
  greps `-w` for `gnome|kde|…`; test app ids must use neutral names (the crate
  uses `org.example.*`, `org.mozilla.firefox`, `com.obsproject.Studio`).
- Live visual check: `bash scripts/capture-t15-privacy-adapter.sh` produced
  `docs/captures/t15-13a-privacy-adapter.png` (2115x1437). Vision found the menu
  bar, the Dock, the wallpaper, the Settings window, and the demo client
  window composited with no blank areas, tearing, or ghosting (the only note is
  the usual nested-session X11 demo-window edge, not a regression).

## T136 — T-15.13b Privacy and Security pane and tile

**State: done.** The Settings `Privacy & Security` pane and the Control Center
`Privacy` tile ship as one functional unit over the T-15.13a adapter, through
the bridge host (ADR 0143), not by linking the Rust crate. No new settingsd
key: the portal PermissionStore is the state and the two writes are explicit
adapter actions. The tile is a read-only summary.

Real paths:

- `services/system-status/src/privacy.rs` (new) — `PrivacyHost<S>` (refresh/
  view/state + `set_permission`/`delete_permission`) + pure `privacy_view`/
  `privacy_snapshot_view` + `privacy_report`. The view carries `glyph`/`label`/
  `present`/`appCount`/`grantedCount`/`deniedCount`/`categoryCount` plus
  `categories[]` (each `{id,label,summary,appCount,grantedCount,resources[]}`;
  each resource `{id,appCount,apps[]}`; each app `{app,state,stateLabel,
  permissions}`). Every known table is a category, empty ones included.
- `services/system-status/src/lib.rs` — `pub mod privacy`, re-exports, and
  `PRIVACY_INTERFACE = "org.dragonfruit.SystemStatus1.Privacy"`.
- `services/system-status/src/dbus.rs` — `LivePrivacy = PrivacyHost<HostPrivacy>`;
  `PrivacyInterface` with `State`/`Refresh`/`SetPermission(table,id,app,
  permission)`/`DeletePermission(table,id,app)`; `run(...)` takes the privacy
  host; `interface_names()` is now 11.
- `services/system-status/src/main.rs` — `PrivacyHost::new(HostPrivacy::new())`,
  `--print-privacy`.
- `services/system-status/tests/privacy.rs` (new) — 6 bridge acceptance tests.
- `services/system-status/Cargo.toml` — depends on `dragonfruit-privacy-adapter`.
- `apps/settings/PrivacyClient.{h,cpp}` (new) — `PrivacyClient` seam;
  `DbusPrivacyClient` over the `Privacy` interface; `MockPrivacyClient`
  (`DF_PRIVACY_FIXTURE`) with a simulated store (devices/camera:
  firefox=yes, Snapshot=no; location: Maps=exact+timestamp; notifications:
  Calendar=ask; screencast: OBS=yes) that mutates on every write and
  `resetForTest()`.
- `apps/settings/SettingsBridge.{h,cpp}` — `privacy`/`privacyAvailable`
  properties, `privacyChanged`, `refreshPrivacy`, `setPrivacyPermission`,
  `deletePrivacyPermission`, `resetPrivacyFixture`.
- `apps/settings/PrivacyPane.qml` (new) — flat list of the 14 portal
  categories (icon + label + secondary summary + chevron, separators, no inset
  card per ADR 0122), an empty note, an absence note, and a per-category dialog
  whose app rows use a four-way `Select` (`Allowed`/`Denied`/`Ask`/`Not Set`).
  The dialog captures its row list once on open (`dialogApps`) and the Selects
  re-read live state through a `Binding`, so a write converges without
  rebuilding the Select mid-interaction.
- `apps/settings/SettingsPanes.qml` — `privacy` shipped `true`, icon `privacy`,
  header description; `SettingsShell.qml` registers the body.
- `apps/settings/CMakeLists.txt` (module + `df_qml_lint`) and
  `apps/settings/tests/CMakeLists.txt` (`tst_settings_privacy`).
- `design-system/components/Icon.qml` — new painted `privacy` shield-with-check
  glyph.
- Shell: `systemstatusclient.{h,cpp}` (read-only `refreshPrivacy` +
  `privacyState`, mock fixture), `systemstatusmodel.{h,cpp}` (`privacy()`,
  `privacyVisible()`, `applyPrivacyJson`, `requestRefreshPrivacy`, the `present`
  second-hide rule), `shellcontroller.{h,cpp}` (`onPrivacyState`, startup /
  status-report refresh, `applyControlCenterData` pushes `privacy`,
  `onPrivacySettingsRequested`), `shell/control-center/ControlCenter.qml`
  (16th tile `privacy`, `present` hide rule, and the `sm` → `xs` internal-gap
  compaction on the 14 existing tiles).
- Tests — `apps/settings/tests/tst_settings_privacy.{cpp,qml}` (new, 6 cases);
  `tst_settings_absence.qml` (shipped 19 → 20, id list, privacy absence case);
  `tst_settings_shell.qml` (shipped 19 → 20, list); `shell/tests/
  tst_controlcenter.qml` (16 tiles, fit test, privacy cases, `SignalSpy`);
  `shell/tests/tst_statusmodel.cpp` (3 privacy cases).
- Docs/scripts — ADR `0143-privacy-security-pane-and-tile.md`;
  `docs/design/07-system-integration.md` D-Bus bullet + T-15.13b subsection;
  `docs/design/08-settings.md` Privacy routing row; capture script
  `scripts/capture-t15-privacy-pane.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status -p dragonfruit-privacy-adapter` —
  green (system-status 70 lib + 6 privacy integration + 10 host).
- `cargo clippy -p dragonfruit-system-status -p dragonfruit-privacy-adapter
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `cmake -S . -B build -G Ninja && cmake --build build` — EXIT 0.
- `ctest --test-dir build -j4` — 67/67.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new work); unchanged from T125–T135.

Decisions / gotchas for T-15.14a and later:

- **No settingsd key.** The portal PermissionStore is the state; the pane's
  controls are explicit adapter writes. Do not add a durable preference for a
  permission. (Contrast `printers.defaultPaperSize`.)
- **Absence is single-layered (ADR 0142/0143).** `state: "unavailable"` = no
  session bus / no store owner; `available` + `present: false` = a store that
  records no app permission. The tile hides on either; the pane lists every
  category (empty ones say `None`) and shows the absence note only when
  `unavailable`.
- **A write never invents a snapshot.** The host re-reads after each action;
  `MockPrivacyClient` mutates its simulated store so a round-trip is observable
  headlessly.
- **`DF_PRIVACY_FIXTURE`** selects the Settings mock and is process-global; the
  test calls `Settings.resetPrivacyFixture()` in `init()`. The shell fixture is
  `DF_STATUS_FIXTURE` (its mock now emits a privacy view too).
- **Permission `Select` inside a dynamic dialog.** The per-category dialog
  captures `dialogApps` once on open and each Select binds `currentIndex` to
  the live view through `permissionStateFor`; do not bind the Repeater model to
  a live-derived array, or the Select is destroyed mid-`activateIndex` and Qt
  errors on the dangling ContextMenu.
- **Control Center now holds 16 tiles.** The tiles' internal gap is `xs` (was
  `sm`); the content gap stays `xxs`. Content height measured 1143 at 15 tiles
  (pre-change), so a **17th** tile needs either more compaction or a taller
  nested output (the capture scripts assume 1920×1200 with a 1160 panel). No
  pointer-axis forwarding, so the panel cannot scroll.
- **Reference deviations, once.** The categories are the 14 portal tables, not
  the macOS rows; no Calendars/Photos/Full Disk Access/Psskeys rows are
  invented (Passkeys maps to the host Secret Service in a later task,
  reuse-only); security rows (FileVault, Lockdown Mode, Find My) are omitted
  with the Apple-only set. The macOS Allow/Deny toggles become a four-way
  selector so `ask` and removal are reachable.
- Live visual check: `bash scripts/capture-t15-privacy-pane.sh` produced
  `docs/captures/t15-13b-privacy-pane.png` (2088x1410) and
  `docs/captures/t15-13b-privacy-control-center.png` (360x1160). Vision read
  the header (`Privacy & Security` + the reference description + shield-check
  icon) and the rows `Camera · 2 apps`, `Location Services · 1 app`,
  `Notifications · 1 app`, `Screen Recording · 1 app`, then `None` rows with no
  clipped/blank/overlapping text; the panel showed the `Privacy` tile
  (`3 Apps`, `Privacy & Security Settings…`) with every tile through Clipboard
  fully visible and no artifacts.

## T137 — T-15.14a Accessibility adapter

**State: done.** New workspace crate `dragonfruit-accessibility-adapter`
(`services/accessibility-adapter`) projects the **AT-SPI accessibility bus**
over the T-07 adapter contract, with state, events, and absence (ADR 0144).
Reused, never reimplemented.

Real paths:

- `services/accessibility-adapter/src/source.rs` — `AccessibilityData
  {enabled, screen_reader}`, `AccessibilitySource` (`read`), and
  `MockAccessibility` (`absent`/`present`/`failing`, `kill`/`restart`, `push`,
  `reads`, `is_present`).
- `services/accessibility-adapter/src/model.rs` — `AccessibilitySnapshot`
  (`is_enabled`/`is_screen_reader_enabled`/`present`/`enabled_label`/
  `screen_reader_label`/`label`/glyph `accessibility`) + pure
  `changes(previous)` (`AccessibilityChange::{Enabled,ScreenReader}`).
- `services/accessibility-adapter/src/adapter.rs` — `AccessibilityAdapter<S>`
  over the shared `Subscription` (`refresh`/`snapshot`/`drain_changes`/
  `pending_changes`), `Adapter` with `AdapterId::ACCESSIBILITY`.
- `services/accessibility-adapter/src/accessibility.rs` — live
  `HostAccessibility`; session-bus `org.a11y.Bus` at `/org/a11y/bus`,
  interface `org.a11y.Status`, one `GetAll` read; pure
  `status_from_properties`.
- `services/accessibility-adapter/src/lib.rs` — crate docs + re-exports.
- `services/accessibility-adapter/tests/read_path.rs` — 9 acceptance tests.
- `services/system-adapters/src/state.rs` — `AdapterId::ACCESSIBILITY`
  (`"accessibility"`); id test updated.
- `Cargo.toml` workspace member; `Makefile` e2e runs
  `cargo test -p dragonfruit-accessibility-adapter`.
- Docs/scripts — ADR `0144-accessibility-adapter.md`;
  `docs/design/07-system-integration.md` table row + "The accessibility path
  (T-15.14a)" section; `docs/design/08-settings.md` Accessibility routing row;
  `scripts/capture-t15-accessibility-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-accessibility-adapter` — 25 lib + 9 read_path
  green.
- `cargo test -p dragonfruit-system-adapters` — green.
- `cargo clippy -p dragonfruit-accessibility-adapter
  -p dragonfruit-system-adapters --all-targets -- -D warnings` — clean;
  `cargo fmt --all -- --check` — clean.
- `make build` — EXIT 0; `make e2e` — EXIT 0 (captured
  `/tmp/opencode/e2e-t137.log`).
- `ctest --test-dir build --output-on-failure -j4` — 67/67.
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new crate); unchanged from T125–T136.

Decisions / gotchas for T-15.14b and later:

- **The host stack is AT-SPI, not a desktop settings schema.** Only
  `org.a11y.Status` (`IsEnabled`, `ScreenReaderEnabled`). Do not read a named
  desktop's GSettings; `scripts/check-desktop-names.sh` forbids it and the bus
  is the portable contract.
- **Read-only.** `org.a11y.Status` has no setter. Durable accessibility
  preferences and compositor magnification are T-15.14b's; do not add a write
  here.
- **Absence is single-layered.** `read() = Ok(None)` = no session bus or no
  `org.a11y.Bus` owner → hidden (`Unavailable`, normal). An all-off bus is
  `Available` with `present() == false` (the second hide rule). A bus that owns
  its name but cannot be read is `Error`, visible and inert.
- **`GetAll` answers `a{sv}`.** Deserialize into `HashMap<String, OwnedValue>`
  and unwrap each bool at the boundary; deserializing straight into
  `HashMap<String, bool>` fails the signature check (this was caught by the
  live-read test). Missing/unexpected property → `false`.
- **`AdapterId::ACCESSIBILITY` is `"accessibility"`**, matching
  `menu.control.accessibility` and the shell status-item id. T-15.14b needs a
  `dragonfruit-system-status` `Accessibility` interface + a
  `DF_ACCESSIBILITY_FIXTURE` seam (through the host, not by linking the crate).
- **Reference mapping.** Vision/VoiceOver → the screen-reader flag here;
  Zoom/Display/Motion → compositor + settingsd (`accessibility.reduceMotion`
  already exists); Hearing Devices/Captions/Live Captions/Name Recognition are
  Apple-only or host-ownerless (omit per ADR 0122).
- Live visual check: `bash scripts/capture-t15-accessibility-adapter.sh`
  produced `docs/captures/t15-14a-accessibility-adapter.png` (2115x1437).
  Vision found menu bar, Dock, wallpaper, Settings window, and the demo client
  composited with no blank areas, tearing, or ghosting (only the usual nested
  X11 demo-window edge).

## T138 — T-15.14b Accessibility pane and tile

**State: done.** The Settings `Accessibility` pane and the Control Center
`Accessibility` tile ship as one functional unit over the T-15.14a adapter,
through the bridge host (ADR 0145), not by linking the Rust crate. The adapter
is read-only, so the pane has **no new settingsd key**: its one durable
preference is the existing `accessibility.reduceMotion` (shell theme / Dock /
compositor consumers), and the live AT-SPI rows are status-only.

Real paths:

- `services/system-status/src/accessibility.rs` (new) — `AccessibilityHost<S>`
  (refresh/view/state) + pure `accessibility_view`/`accessibility_snapshot_view`.
  The view carries `glyph`/`label`/`present`/`enabled`/`enabledLabel`/
  `screenReader`/`screenReaderLabel`. Read-only: no writes.
- `services/system-status/src/lib.rs` — `pub mod accessibility`, re-exports,
  `ACCESSIBILITY_INTERFACE = "org.dragonfruit.SystemStatus1.Accessibility"`.
- `services/system-status/src/dbus.rs` — `LiveAccessibility =
  AccessibilityHost<HostAccessibility>`; `AccessibilityInterface` with
  `State`/`Refresh`; `run(...)` takes the accessibility host; `interface_names()`
  is now 12.
- `services/system-status/src/main.rs` — `AccessibilityHost::new(HostAccessibility::new())`,
  startup refresh, `--print-accessibility`.
- `services/system-status/tests/accessibility.rs` (new) — 6 bridge tests.
- `services/system-status/Cargo.toml` — depends on `dragonfruit-accessibility-adapter`.
- `apps/settings/AccessibilityClient.{h,cpp}` (new) — read-only seam;
  `DbusAccessibilityClient` over the `Accessibility` interface;
  `MockAccessibilityClient` (`DF_ACCESSIBILITY_FIXTURE`) with bridge and screen
  reader on; `resetForTest()`.
- `apps/settings/SettingsBridge.{h,cpp}` — `accessibility`/`accessibilityAvailable`
  properties, `accessibilityChanged`, `refreshAccessibility`,
  `resetAccessibilityFixture`.
- `apps/settings/AccessibilityPane.qml` (new) — grouped `Vision` card with the
  read-only `Screen Reader` and `Accessibility` status rows, an absence note,
  and a `Motion` card with the `Reduce Motion` toggle bound to
  `accessibility.reduceMotion`.
- `apps/settings/SettingsPanes.qml` — `accessibility` shipped `true`, header
  description; `SettingsShell.qml` registers the body.
- `apps/settings/CMakeLists.txt` (module + `df_qml_lint`) and
  `apps/settings/tests/CMakeLists.txt` (`tst_settings_accessibility`).
- `design-system/components/Icon.qml` — new original painted `accessibility`
  person-in-circle glyph.
- Shell: `systemstatusclient.{h,cpp}` (read-only `refreshAccessibility` +
  `accessibilityState`, mock fixture), `systemstatusmodel.{h,cpp}`
  (`accessibility()`, `accessibilityVisible()`, `applyAccessibilityJson`,
  `requestRefreshAccessibility`, the `present` second-hide rule),
  `shellcontroller.{h,cpp}` (`onAccessibilityState`, startup / status-report
  refresh, `applyControlCenterData` pushes `accessibility`,
  `onAccessibilitySettingsRequested`), `shell/control-center/ControlCenter.qml`
  (17th `accessibility` tile, `present` hide rule, tile internal gap `xs` →
  `xxs`).
- Tests — `apps/settings/tests/tst_settings_accessibility.{cpp,qml}` (new, 4
  cases); `tst_settings_absence.qml` (shipped 20 → 21, id list, accessibility
  absence case); `tst_settings_shell.qml` (shipped 20 → 21, list, keyboard
  index 4 → 5); `shell/tests/tst_controlcenter.qml` (17 tiles, fit test, 3
  accessibility cases); `shell/tests/tst_statusmodel.cpp` (3 accessibility
  cases).
- Docs/scripts — ADR `0145-accessibility-pane-and-tile.md`;
  `docs/design/07-system-integration.md` D-Bus bullet + T-15.14b subsection;
  `docs/design/08-settings.md` Accessibility routing row; capture script
  `scripts/capture-t15-accessibility-pane.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-system-status -p dragonfruit-accessibility-adapter`
  — green (system-status 75 lib + 6 accessibility + the rest).
- `cargo clippy -p dragonfruit-system-status -p dragonfruit-accessibility-adapter
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `cmake -S . -B build -G Ninja && cmake --build build` — EXIT 0.
- `ctest --test-dir build -j4` — 68/68 (one `tst_dock` flake under `-j4` on the
  first run; `make lint`'s ctest run reported 68/68).
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t138.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new work); unchanged from T125–T137.

Decisions / gotchas for T-15.15a and later:

- **Read-only adapter, one existing key.** `org.a11y.Status` has no setter, so
  the `Accessibility` interface is `State`/`Refresh` only. The pane's one
  durable preference is `accessibility.reduceMotion`; **no new settingsd key
  and no schema bump**. A compositor magnifier and a display-contrast preference
  are a follow-up (documented in ADR 0145), not shipped as dead rows.
- **Absence is single-layered (ADR 0144/0145).** `state: "unavailable"` = no
  session bus / no `org.a11y.Bus` owner; `available` + `present: false` = an
  all-off bus. The tile hides on either; the pane shows the absence note beside
  the still-live `Reduce Motion` toggle.
- **The pane's groups are `Vision` and `Motion`.** `Hearing` and the Apple-only
  rows (`Zoom`/`Hover Text`/`Display`/`Read & Speak`/`Audio Descriptions`/
  `Live Captions`/`Name Recognition`) are omitted per ADR 0122; no row is
  invented without a Linux host owner.
- **`DF_ACCESSIBILITY_FIXTURE`** selects the Settings mock; the shell mock is
  `DF_STATUS_FIXTURE` (its mock now emits an accessibility view too).
- **Control Center now holds 17 tiles.** To fit, the tile internal gap is `xxs`
  (was `xs`), and the Accessibility tile is a compact single-row summary (no
  separate link line; tapping the tile opens the pane). Content height measured
  1130 ≤ 1160; an **18th** full tile would need another compaction or a taller
  nested output. No pointer-axis forwarding, so the panel cannot scroll.
- **The menu-bar `accessibility` status item is still inert** (`false &&
  controlEnabled("accessibility")` in `shellcontroller.cpp`); wiring it is a
  separate surface, not this task.
- Live visual check: `bash scripts/capture-t15-accessibility-pane.sh` produced
  `docs/captures/t15-14b-accessibility-pane.png` (2088x1410) and
  `docs/captures/t15-14b-accessibility-control-center.png` (360x1160). Vision
  read the header (`Accessibility` + the reference description), the `Vision`
  card (`Screen Reader` / `Accessibility` status rows) and the `Motion` card
  (`Reduce Motion`) with no clipping/overlap/artifacts; the panel showed the
  `Accessibility` tile (`Screen Reader On`) with all tiles fully visible and no
  bottom clipping.

## T139 — T-15.15a Network advanced (VPN) adapter

**State: done.** The Network advanced (VPN) adapter landed as a **second
adapter inside the existing `dragonfruit-networkmanager` crate**
(`services/networkmanager/src/vpn/`), because a VPN is not a second daemon:
NetworkManager already owns every `vpn`/`wireguard` connection. It projects the
configured settings objects plus the live active connections over the same
`zbus` client, with state, events, absence, and the two explicit writes
(ADR 0146).

Real paths:

- `services/networkmanager/src/vpn/mod.rs` (new) — module docs + re-exports.
- `services/networkmanager/src/vpn/source.rs` (new) — `VpnData`,
  `VpnConnectionData`, `ActiveVpnData`, `VpnSource` (`read`/`activate`/
  `deactivate`), `VpnRequest`, `VpnOutcome`, and `MockVpn`
  (`absent`/`present`/`failing`, `deny_writes`/`fail_writes`/`allow_writes`,
  `push`/`kill`/`restart`, `reads`/`activations`/`deactivations`). An accepted
  write mutates the simulated store (connect marks the connection active,
  disconnect removes it) so the adapter's write-then-re-read round-trip is
  observable headlessly.
- `services/networkmanager/src/vpn/model.rs` (new) — `VpnSnapshot::from_data`
  joins settings connections to active connections by settings path (UUID
  fallback); `VpnKind` (`Vpn`/`OpenVpn`/`OpenConnect`/`Ipsec`/`Pptp`/`L2tp`/
  `WireGuard`) from `connection.type` + `vpn.service-type`; `VpnState`
  (`Disconnected`/`Connecting`/`Connected`/`Disconnecting`/`Unknown`) from
  `NMActiveConnectionState`; `VpnConnection`; `present`/`connected_count`/
  `active_uuid`/`active_name`/`glyph` (`vpn`/`vpn-off`)/`label`.
- `services/networkmanager/src/vpn/adapter.rs` (new) — `VpnAdapter<S>`
  (`refresh`/`snapshot`/`connect`/`deactivate`) + `VpnAccess`
  (`ReadWrite`/`ReadOnly{note}`) + `VpnResult`; `Adapter` with
  `AdapterId::VPN`.
- `services/networkmanager/src/dbus.rs` — `DbusVpn` (`VpnSource`) + proxies
  `NmSettings` (`Settings.Connections`, `GetConnectionByUuid`),
  `ActiveConnection` (`Connection`/`Uuid`/`Id`/`State`/`Vpn`/`Type`),
  `NetworkManager.ActiveConnections`; `read_vpn`, `get_connection_settings`,
  pure+tested `vpn_connection_from_settings`, `classify_vpn_error`; const
  `VPN_SETTINGS_PATH`.
- `services/networkmanager/src/lib.rs` — `mod vpn;` + re-exports; crate docs
  got a "The VPN path (T-15.15a)" section.
- `services/networkmanager/tests/vpn_read_path.rs` (new) — 7 acceptance tests.
- `services/networkmanager/tests/fixtures/nm-vpn.json` (new) — 3 connections
  (OpenVPN connected, IPsec connecting, WireGuard disconnected) + 2 active.
- `services/system-adapters/src/state.rs` — `AdapterId::VPN` (`"vpn"`); id
  test updated.
- Docs/scripts — ADR `0146-network-advanced-vpn-adapter.md`;
  `docs/design/07-system-integration.md` table row + "The Network advanced
  (VPN) path (T-15.15a)" section; `docs/design/08-settings.md` routing row;
  capture script `scripts/capture-t15-vpn-adapter.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-networkmanager` — green (67 lib + 4 read_path +
  5 join_path + 7 vpn_read_path). `cargo test -p dragonfruit-system-adapters`
  — green (16 lib + …).
- `cargo clippy -p dragonfruit-networkmanager -p dragonfruit-system-adapters
  --all-targets -- -D warnings` — clean; `cargo fmt --all -- --check` — clean.
- `cmake -S . -B build -G Ninja && cmake --build build` — EXIT 0;
  `ctest --test-dir build -j4` — 68/68.
- `make e2e` — EXIT 0 (captured `/tmp/opencode/e2e-t139.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — still fails only on the pre-existing `check-desktop-names`
  lines (none in the new work); unchanged from T125–T138.

Decisions / gotchas for T-15.15b and later:

- **Adapter id is `vpn` (`AdapterId::VPN`), not `network`.** The Settings
  catalog pane is already `{ id: "network", title: "Network", shipped: false }`
  (`apps/settings/SettingsPanes.qml`); T-15.15b ships that pane and wires it
  to this adapter through the bridge host. Pane id and adapter id need not
  match (`printers` vs `printer`, `users-groups` vs `accounts`).
- **Grow the crate, do not make a new one.** `dragonfruit-networkmanager`
  already carries the D-Bus client, proxies, and absence discipline. A later
  VPN task must extend `src/vpn/`, not fork a `dragonfruit-vpn-adapter` crate
  (ADR 0146).
- **Two writes are `connect`/`deactivate`, by UUID**, one
  `ActivateConnection`/`DeactivateConnection` each. A polkit refusal degrades
  to `VpnAccess::ReadOnly{note}` and further writes are refused locally; reads
  stay live. A write invents no snapshot — the daemon pushes and the host
  re-reads.
- **Absence is layered.** `Unavailable` = no system bus / no
  `org.freedesktop.NetworkManager` owner; `available` + `present: false` = the
  daemon answered with no VPN configured. T-15.15b should hide the tile on
  either, and show the pane's empty note for the latter. A name owned but
  unreadable is `Error`, visible and inert.
- **`libnm` is never linked.** The live path is `Settings.Connection.
  GetSettings` (`a{sa{sv}}`; decode into `HashMap<String, HashMap<String,
  OwnedValue>>` and use `OwnedValue::downcast_ref::<String>()` /
  `::<bool>()`, which yield owned values) plus `Connection.Active` properties
  and `ActivateConnection`/`DeactivateConnection` on
  `/org/freedesktop/NetworkManager`.
- **`DbusVpn` and `MockVpn` are compile-checked/CI green; the live D-Bus path
  is not exercised in CI** (no bus/daemon), same as the Wi-Fi source.
- **Reference mapping.** The macOS Network pane shows Wi-Fi / Firewall / Other
  Services and no VPN; per ADR 0122 and the T-15.15b task, map the VPN rows
  and the connections behind the chevrons to this adapter. `Thunderbolt Bridge`
  is Apple-hardware-specific; bridges/ethernet are out of scope for this
  adapter (VPN only).
- Live visual check: `bash scripts/capture-t15-vpn-adapter.sh` produced
  `docs/captures/t15-15a-vpn-adapter.png` (2115x1437). The adapter has no
  surface of its own, so this confirms the desktop renders; vision found menu
  bar, Dock, wallpaper, Settings window, and the demo client composited with
  no blank areas, tearing, or stray artifacts.

## T140 — T-15.15b Network advanced (VPN) pane and tile

**State: done.** The Settings `Network` pane and the Control Center `VPN` tile
ship as one functional unit over the T-15.15a `dragonfruit-networkmanager` VPN
adapter, through the bridge host (ADR 0147), not by linking the Rust crate. The
adapter owns the connection state, so the pane has **no new settingsd key**: its
two writes are the adapter's `Connect`/`Deactivate`, by UUID. Wi-Fi is the
separate `wifi` pane; Firewall and Apple-only services are omitted (ADR 0122).

Real paths:

- `services/system-status/src/vpn.rs` (new) — `VpnHost<S>` (refresh/view/state/
  connect/deactivate) + pure `vpn_view`/`vpn_snapshot_view`/`vpn_report`. View
  carries `{state, glyph, label, present, connectionCount, connectedCount,
  activeUuid, activeName, readOnly, note, connections:[{id,uuid,kind,kindLabel,
  state,stateLabel,connected,autoconnect,label}]}`.
- `services/system-status/src/lib.rs` — `pub mod vpn`, re-exports,
  `VPN_INTERFACE = "org.dragonfruit.SystemStatus1.Vpn"`.
- `services/system-status/src/dbus.rs` — `LiveVpn = VpnHost<DbusVpn>`;
  `VpnInterface` `State`/`Refresh`/`Connect(uuid)`/`Deactivate(uuid)`;
  `run(...)` takes vpn; `interface_names()` is 13.
- `services/system-status/src/main.rs` — `VpnHost::new(DbusVpn::new())`,
  startup refresh, `--print-vpn`.
- `services/system-status/tests/vpn.rs` (new) — 7 bridge tests.
- `services/networkmanager/src/vpn/model.rs` — additive `VpnKind::id()` /
  `VpnState::id()` stable ids.
- `apps/settings/VpnClient.{h,cpp}` (new) — `DbusVpnClient` + `MockVpnClient`
  (`DF_VPN_FIXTURE`, mutable store so connect/disconnect round-trips headless).
- `apps/settings/SettingsBridge.{h,cpp}` — `vpn`/`vpnAvailable`/`vpnChanged`,
  `refreshVpn`, `connectVpn`, `disconnectVpn`, `resetVpnFixture`.
- `apps/settings/NetworkPane.qml` (new) — `VPN` group, per-connection live
  connect/disconnect `Toggle`, empty/read-only/absence notes.
- `apps/settings/SettingsPanes.qml` (`network` shipped true),
  `SettingsShell.qml` (`NetworkPane`), `CMakeLists.txt`; catalog counts 21 → 22.
- `design-system/components/Icon.qml` — original `network`, `vpn`, `vpn-off`.
- Shell: `systemstatusclient.{h,cpp}`, `systemstatusmodel.{h,cpp}`,
  `shellcontroller.{h,cpp}`, `shell/control-center/ControlCenter.qml` (18th
  `vpn` tile). Panel height `1160 → 1164`; Accessibility + VPN tiles are
  single-line compact (`IconTile tileSize: 24`) so eighteen fit.
- Tests — `apps/settings/tests/tst_settings_network.{cpp,qml}` (4 cases);
  `tst_settings_absence.qml` (22, id list, network absence);
  `tst_settings_shell.qml` (22, keyboard index 5 → 6);
  `shell/tests/tst_controlcenter.qml` (18 tiles, fit → 1164, 3 vpn cases);
  `shell/tests/tst_statusmodel.cpp` (3 vpn cases).
- Docs/scripts — ADR `0147`; `docs/design/07-system-integration.md` (Vpn
  interface bullet + T-15.15b subsection); `docs/design/08-settings.md`;
  `scripts/capture-t15-vpn-pane.sh` + `docs/captures/README.md`.

Commands that work (repo root):

- `cargo test -p dragonfruit-networkmanager -p dragonfruit-system-status` —
  green. `cargo clippy -p ... --all-targets -- -D warnings` — clean;
  `cargo fmt --all -- --check` — clean.
- `cmake --build build` — EXIT 0; `ctest --test-dir build -j4` — 69/69.
- `make e2e` — EXIT 0 (`/tmp/opencode/e2e-t140c.log`).
- `make check-design-tokens check-tokens check-no-capture-grab` — clean;
  `./scripts/check-gallery-snapshots.py` — 78 green.
- `make lint` — only the pre-existing `check-desktop-names` failures
  (`apppicker.h`, `tst_dockcore.cpp`, `zoo-run.sh`); unchanged from T125–T139.

Decisions / gotchas for T-15.16 and later:

- **No settingsd key.** NetworkManager owns the state; the pane's writes are
  the adapter's `Connect`/`Deactivate`, by UUID.
- **Absence is layered (ADR 0146/0147).** `unavailable` = no system bus / no
  `NetworkManager` owner; `available` + `present: false` = the daemon answered
  with no VPN (pane empty note, tile hides). `readOnly: true` is a polkit
  degradation, not absence: the list stays visible and the writes disable.
- **Integration gotcha caught by the live check.** The Control Center VPN tile
  first captured blank because `ShellController`'s startup refresh block lacked
  `refreshVpn()`. The headless tests inject the `vpn` property into the panel,
  so they cannot catch a missing controller refresh; always inspect the capture.
- **Panel capacity.** 18 tiles fit only at 1164 px with single-line compact
  Accessibility/VPN tiles. A 19th full tile needs another compaction.
- Live visual check: `bash scripts/capture-t15-vpn-pane.sh` produced
  `docs/captures/t15-15b-vpn-pane.png` (2088x1410) and
  `docs/captures/t15-15b-vpn-control-center.png` (360x1164); vision read the
  pane (`VPN` group: `Work VPN` connected, `Home` idle, no clipping) and the
  panel's `VPN`/`Work VPN` tile above Clipboard, fully visible.
- Not touched (other tasks): launching Settings on a pane from the Control
  Center (`onVpnSettingsRequested` is the log-only T-16 stub), the Firewall
  pane, and the menu-bar VPN status item.

## T141 — T-15.16 Absent-daemon matrix and breadth capture

**State: done.** The T-15 track is closed. The absent-daemon masking matrix is
documented for all 22 shipped panes and reproduced headlessly; the breadth
capture set is committed as a linked set with a whole-desktop still. No pane
ships of its own.

Real paths:

- `docs/design/08-settings.md` — new "The absent-daemon masking matrix
  (T-15.16)" section: two absence families, the 22-row per-pane state table,
  the negative space (`Error` / `present:false` / polkit read-only), and the
  headless-gate locations.
- `docs/design/adr/0148-t15-absent-daemon-masking-matrix.md` (new).
- `apps/settings/tests/tst_settings_absence.qml` — new
  `test_every_shipped_pane_routes_and_stays_live_with_providers_absent` (the
  routing sweep) and `test_mouse_pane_degrades_cleanly_without_the_bridge_host`.
  Now 28 cases.
- `scripts/t15-absence-matrix.sh` (new) + `make t15-absence-matrix` →
  `docs/captures/t15-absence-matrix.txt` (583 lines, all rows green).
- `docs/captures/t15-absence-matrix.md` (new) — reviewed 22-pane state matrix.
- `docs/captures/t15-breadth.md` (new) — the linked breadth capture index.
- `docs/captures/t15-breadth.png` (new, 2115x1437) + `scripts/capture-t15-breadth.sh`
  (new) + `make t15-breadth-capture`.
- `docs/captures/README.md` and `Makefile` (targets + help) updated.

Commands that work (repo root):

- `ctest --test-dir build -j4` — 69/69 (absence suite 28 cases).
- `make e2e` — EXIT 0 (`/tmp/opencode/t141-e2e.log`).
- `make t15-absence-matrix` — EXIT 0.
- `make check-tokens check-design-tokens check-no-capture-grab` — clean.
- `make check-desktop-names` — only the pre-existing failures (`apppicker.h`,
  `tst_dockcore.cpp`, `zoo-run.sh`, `services/app-index/*`); unchanged from
  T125–T140.

Decisions / gotchas for T-16 and later:

- **Two families, one pane.** A pane can mix settingsd-backed controls (live on
  schema defaults, write in memory) and an adapter-read half (one-line note, no
  write). The matrix says which half is which; never add a settingsd
  "unavailable" banner.
- **Absence ≠ error.** `Error` is visible+inert; `present:false` is the second
  hide rule; a polkit refusal is read-only, not absence.
- **"No bridge host" = the whole hidden state** on a live session (the host is a
  separate process), so the nested no-host still is valid live evidence without
  a VM.
- **Remaining human step:** mask each real daemon on a VM and compare the
  rendered pane/tile to `docs/captures/t15-absence-matrix.md`; batched at the
  T-15 track sign-off (SLICING-REVIEW).
- Live visual check: `bash scripts/capture-t15-breadth.sh` produced
  `docs/captures/t15-breadth.png`; vision found the menu bar, Dock, wallpaper,
  and client windows composited with no blank areas, clipping, or missing
  chrome.

## T142 — T-16.1a Per-output chrome sizing and reserved zones

**State: done.** Reserved zones are now **per output**: `aggregate_reserved_for`
filters chrome surfaces with `matches_output`, so a surface pinned to one
display reserves only there while an all-output surface (menu bar, Dock)
reserves on every display. Window usable/Zoom geometry subtracts the zones of
the window's own output. Chrome geometry (`chrome_surfaces`) was already
per-output and now shares the same zone map. ADR 0149.

Real paths:

- `compositor/src/shell/layer.rs` — `aggregate_reserved_for(output_name,
  layers, zones)`; shared `reserve` helper; `aggregate_reserved` kept as the
  global-union fallback. 4 new unit tests.
- `compositor/src/shell/mod.rs` — `ShellProtocolState.output_reserved:
  HashMap<String, ReservedZones>`; `reserved_zones_for(name)`;
  `refresh_reserved_zones` rebuilds the per-output map (union kept as fallback);
  `send_output_properties` and `broadcast_reserved_zones` send per-output zones
  (the session's `outputs` map is already keyed by name).
- `compositor/src/state.rs` — `usable_geometry_for` uses
  `output_name_for(window)` + `reserved_zones_for`.
- `protocols/dragonfruit-shell.xml` — `get_layer_surface` description corrected
  to the all-output `NULL` rule.
- `docs/design/adr/0149-per-output-chrome-reserved-zones.md` (new).
- `compositor/tests/shell_protocol_conformance.rs` — new
  `per_output_chrome_reserves_are_scoped_and_scale_aware`; the test harness now
  binds `wl_output` so a surface can be pinned to one display.

Commands that work (repo root):

- `cargo test -p dragonfruit-compositor` — green (all suites, new test
  included).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (`/tmp/opencode/e2e-t142.log`).
- Live visual check: `/tmp/opencode/t142.png`; vision found the menu bar, Dock,
  wallpaper, and client windows composited with no artifacts.

Decisions / gotchas for T-16.1b and later:

- **`matches_output` is the one filter.** A pinned surface reserves only on its
  own output; `NULL` reserves on every output. Each edge keeps the deepest
  reserve (no summing).
- **Reserved thicknesses are logical pixels**; output scale changes the
  geometry they subtract from, not the reserve. T-16.1b gets per-output usable
  geometry via `usable_geometry_for` for free.
- **One surface, one configure.** A no-output chrome surface is still a single
  `wl_surface`, so its client `configure` size resolves against the primary
  output; true per-output client buffers need one layer surface per output in
  the shell. This is the one open half of "chrome sizes per output" (ADR 0149).
- **`DfState.reserved_zones` is now only a fallback** (the global union), not
  the source of truth. New consumers should use `usable_geometry_for` /
  `ShellProtocolState::reserved_zones_for`.

## T143 — T-16.1b Per-output window placement

**State: done.** New windows now open on the **focused output** and inside
that output's reserved zones. `DfState::focused_output()` resolves the output
under the pointer first, then the active window's output, then the primary.
Ordinary windows cascade in `reserved_zones_for(name).usable(output)` so a menu
bar or Dock pushes them clear of chrome. Workspace assignment follows the
placement output (focused output; the transient parent's output for a dialog).
The Xwayland placement path uses the same resolver. ADR 0150.

Real paths:

- `compositor/src/state.rs` — new `focused_output()` + `output_geometry_named()`;
  `map_pending_windows` computes `placement_output` from the focused output (or
  the parent for a transient) and cascades over the usable rect;
  `assign_new_window_space(window, id, output_name)` now takes the target
  output; `primary_output` kept as the fallback.
- `compositor/src/xwayland.rs` — same focused-output + usable-area cascade and
  output-aware workspace assignment.
- `compositor/tests/shell_protocol_conformance.rs` — new
  `new_windows_land_on_the_focused_output`: two outputs, a 28 px bar pinned to
  the primary, three windows read back via `query decorations`; A (pointer on
  primary) clears the reserve, B (pointer on HDMI-A-1) centers in the full
  second-output height, C (pointer back) clears the reserve again. The
  `identity_app_ids` / `decoration_content_rects` / `window_rect` helpers parse
  the synthetic-input introspection.
- `docs/design/02-compositor.md` — Placement bullet updated.
- `docs/design/adr/0150-per-output-window-placement.md` (new).

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64` when libudev is absent):

- `cargo test -p dragonfruit-compositor` — all suites green
  (`shell_protocol_conformance` 36 → 37 tests).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (`/tmp/opencode/t143-e2e.log`).
- Live visual check: `/tmp/opencode/t143.png` (2115x1437) via
  `/tmp/opencode/capture-t143.sh`; vision found the menu bar, Dock, wallpaper,
  the centered Settings client, and the X11 demo window compositing with no
  blank areas, clipping, or stray artifacts.

Decisions / gotchas for T-16.2 and later:

- **Focused output = pointer-first**, then active window, then primary.
  Documented tradeoff in ADR 0150: a keyboard launch while the pointer rests on
  another display opens there. Pointer-first is also the only rule that can
  seed a display with no windows yet.
- **Placement subtracts only the focused output's zones**, never the global
  union. `cascaded_geometry` receives the usable rect; the per-output cascade
  index still keys on the output name.
- **Assignment follows placement.** A window placed on output X is assigned to
  X's active Space, not the primary's. Before T143 the geometry was per-output
  but the Space assignment still used the primary — a latent mismatch now gone.
- **Hotplug fallback is automatic.** `focused_output()` walks the live
  `space.outputs()`, so a removed output cannot be selected; T-16.2 inherits it.
- **No protocol change.** The shell sees the existing `df_toplevel.output_entered`
  and per-output `df_output` events; nothing new is exposed.

## T144 — T-16.2 Hotplug under load and lockstep

**State: done.** Output hotplug now preserves lockstep and loses no windows.
Two real gaps fixed: a hotplugged display joined at index 0 instead of the
session's current lockstep index (so later switches never realigned), and a
detached display's windows kept geometry on the dead display (tracked in the
model but off every remaining output). ADR 0151.

Real paths:

- `compositor/src/workspace/mod.rs` — `add_output` mirrors the existing
  displays' Space shape (length + `fullscreen_for`) and adopts their active
  index; new unit test
  `hotplugged_output_joins_lockstep_and_mirrors_a_fullscreen_space`. The
  `hotplug_attach_detach_matrix_loses_no_windows` assertion updated from
  index 0 to the primary's index (the point of the fix).
- `compositor/src/state.rs` — `on_output_removed` calls new
  `relocate_windows_off_output(removed, primary)` before the output leaves the
  `Space`: windows mapped to or geometrically on the dead display are re-homed
  into the primary's usable area (floating cascades, zoomed re-fits via
  `insets_for`, fullscreen fills the primary).
- `compositor/src/window/state.rs` — new `WindowStateMachine::relocate(target)`
  moves the current state's geometry (minimized moves its restore state) + unit
  test `relocate_rehomes_every_visible_state_and_the_restore_geometry`.
- `compositor/tests/shell_protocol_conformance.rs` — two new tests
  (`workspace_switch_stays_lockstep_after_hotplug`,
  `hotplug_under_load_keeps_every_window_on_a_live_display`) and the
  `wait_for_window_rect_in` helper. 37 → 39 tests.
- `docs/design/02-compositor.md` (new Output hotplug bullet),
  `docs/design/03-workspaces.md` (hotplug bullet rewritten),
  `docs/design/adr/0151-hotplug-lockstep-and-window-rehoming.md` (new).

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64` when libudev is absent):

- `cargo test -p dragonfruit-compositor` — all suites green (binary unit 285,
  `shell_protocol_conformance` 39).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (`/tmp/opencode/t144-e2e.log`).
- Live visual check: `/tmp/opencode/t144-active.png` (2115x1437 active nested
  window) + `/tmp/opencode/t144.png`; vision found the Dock, wallpaper, native
  Settings and X11 demo clients, sharp text, and no blank/clipped/artifact
  regions. Nested has **no** output hotplug (synthetic output is headless-only),
  so this confirms the desktop renders; the cable hotplug matrix is the batched
  human VM step.

Decisions / gotchas for T-16.3a and later:

- **Hotplug joins lockstep.** New displays align to the first output's active
  index and mirror fullscreen Spaces. There is no per-output activate path;
  `switch_all`/`activate_all` stay the only lockstep entry points.
- **Relocation is geometry-only** and runs before `remove_output` + unmap, so
  `outputs_for_element` still resolves the removed output. A window is
  considered on it when mapped there *or* its stored geometry overlaps it
  (windows on the output's inactive Spaces are not mapped).
- **Nested cannot hotplug.** `DRAGONFRUIT_SYNTHETIC_OUTPUT` is installed only by
  the headless backend; T-16.3's scaling matrix should reuse that harness.

## T145 — T-16.3a Integer-scaled Xwayland

**State: done.** The integer-scale half of `docs/xwayland-scaling.md` is built.
The compositor chooses the Xwayland integer scale once per session as
`ceil(primary_output_scale).max(1)` and records it in
`XwaylandState::integer_scale`; this is the value Smithay already advertises as
the output's integer `wl_output.scale`. `render::post_repaint` no longer sends
X11 windows a fractional `preferred_scale` (`accepts_fractional_scale` skips
windows with an `x11_surface`), which is the effective enforcement of the
"never advertise `wp_fractional_scale_v1` to Xwayland" policy — Smithay 0.7 has
no per-client global filter, so not sending the event is the safe form. The
per-surface viewport downscale remains T-16.3b. ADR 0152.

Real paths:

- `compositor/src/xwayland.rs` — `XwaylandState.integer_scale`; computed and
  logged in `start()` (`Xwayland ready: DISPLAY=... (start #N, integer scale S)`).
- `compositor/src/render.rs` — `accepts_fractional_scale(window)`; `post_repaint`
  skips the fractional preferred scale for X11 surfaces and still sends frame
  callbacks for them.
- `compositor/src/backend/mod.rs` — `ENV_HEADLESS_SCALE` +
  `parse_output_scale`; `compositor/src/backend/headless.rs` boots the headless
  output at that scale. The synthetic-output harness runs only *after* Xwayland
  starts, so the primary must boot at the target scale for an X11 scale fixture.
- `compositor/tests/xwayland_conformance.rs` — new
  `x11_fixture_at_integer_scale_two` (8 tests total): boots headless at scale 2,
  asserts the X server screen is the logical size (640x360, not 1280x720), a
  200x120 X11 window stays 200x120 (no double scale), zoom fills the logical
  usable area (640x320), and the X server receives that logical configure. Also
  `CompositorProcess::start_with_synthetic_at_scale` and `click_light_at`.
- `docs/xwayland-scaling.md` — "Current state" rewritten; `-scale`/`-hidpi`
  mechanism corrected (`-hidpi` is rootful-only); stale T-31 deferral → T-16.3b.
- `docs/design/02-compositor.md` — new "Integer-scaled Xwayland" bullet.
- `docs/design/adr/0152-integer-scaled-xwayland.md` (new).

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64` when libudev/libseat are absent):

- `cargo test -p dragonfruit-compositor` — all suites green (bin unit 286,
  `xwayland_conformance` 8, `shell_protocol_conformance` 39).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (`/tmp/opencode/t145-e2e-2.log`). A first run flaked in
  the unrelated `dragonfruit-lock-auth` `a_missing_user_is_a_usage_error`
  broken-pipe race; it passes alone and on rerun.
- Live visual check: `/tmp/opencode/t145-demo-active.png` (2115x1437) via
  `/tmp/opencode/capture-t145-demo.sh`; vision found the menu bar, Dock,
  wallpaper, Settings client, and the X11 xmessage client composited with no
  clipping/tearing/oversized chrome. Default nested scale is 1
  (`Xwayland ready ... integer scale 1`).

Decisions / gotchas for T-16.3b and later:

- **The mechanism is `wl_output.scale`, not an Xwayland flag.** Upstream
  Xwayland has no `-scale`; `-hidpi` is integer-only *and* rootful-only. A
  rootless Xwayland only ever learns scale from the output global, which
  Smithay already serializes as `ceil(fractional_scale)`.
- **Not sending `preferred_scale` is the enforcement.** `wp_fractional_scale_v1`
  is still bindable by Xwayland; `accepts_fractional_scale` just never fires for
  X11 surfaces. If a stricter hide is ever wanted, the manager global would have
  to be hand-rolled with a client filter (Smithay 0.7 does not expose one).
- **Nested can now boot at scale > 1 only via `DRAGONFRUIT_HEADLESS_SCALE`'s
  sibling — i.e. it cannot.** Nested is hardcoded 1.0. A scale-2 nested run
  exposed a real chrome bug (below), so the knob was intentionally not kept;
  T-16.3b should add the nested scale seam together with chrome sizing.
- **Discovered chrome bug (for T-16.3b).** At output scale > 1 the wallpaper
  image element is placed in physical pixels but sized in logical pixels
  (`render::wallpaper_render_elements`), so the image draws only in the
  top-left fraction of the output (solid fallback still covers the rest). This
  is chrome sizing, explicitly deferred by T-16.3a.
- **Fractional output scale is a documented fallback.** Until T-16.3b, a 1.5x
  output shows X11 windows at the integer `ceil` (2x) size.
- The headless `integer_scale` is only recomputed on Xwayland (re)start; the
  shell can `df_output.set_scale` after boot (the demo does, to 1.0), so the
  logged value is the boot-time policy, not a live mirror. Xwayland itself
  tracks later output scale changes through Wayland as usual.

## T146 — T-16.3b Viewport downscale and chrome sizing

**State: done.** Fractional-scale chrome is sized per output and the nested
session can run at a fractional scale. Two real bugs fixed: the nested render
path hardcoded the damage tracker's scale to 1.0 (so at any output scale != 1
elements were placed in physical pixels but sized in logical pixels — the
wallpaper top-left bug T-16.3a found), and `df_output.set_scale` /
`set_transform` / `set_mode` did not re-configure the chrome layers (so after a
scale change the menu bar/Dock stayed sized for the old logical output).
ADR 0153.

Real paths:

- `compositor/src/backend/mod.rs` — `ENV_NESTED_SCALE` = `DRAGONFRUIT_NESTED_SCALE`.
- `compositor/src/backend/nested.rs` — reads the env via `parse_output_scale`
  (default 1.0) for the nested `add_output`; `NestedData { tracker_scale,
  tracker_size }` + `ensure_tracker(size, scale)` rebuilds the
  `OutputDamageTracker` on a physical-size or **scale** change (called from
  `render_frame` and the `Resized` handler).
- `compositor/src/shell/mod.rs` — `df_output` request dispatch tracks
  `geometry_changed` for `SetMode`/`SetScale`/`SetTransform` and calls
  `state.reconfigure_layers()` before the property ack.
- `compositor/src/render.rs` — `update_chrome_preferred_scale(state, output)`
  (called from `post_repaint`) sends each visible chrome surface the output's
  fractional `wp_fractional_scale` preferred scale. `accepts_fractional_scale`
  still returns false for X11.
- `compositor/src/input/synthetic.rs` — new `query scale` report:
  `scale output <name> physical=WxH logical=WxH scale=S`, then
  `scale window <id> logical=x,y,w,h physical=x,y,w,h buffer_scale=N buffer=WxH`
  and `scale chrome <ns> logical=x,y,w,h physical=x,y,w,h`.
- `compositor/tests/shell_protocol_conformance.rs` — new
  `fractional_scale_reconfigures_chrome_to_the_logical_output` (top bar
  1280x28 → 854x28 at scale 1.5; verified to fail with the reconfigure call
  removed). 39 → 40 tests.
- `compositor/tests/xwayland_conformance.rs` — new
  `x11_fixture_at_fractional_scale_sizes_per_output` (scale 1.5: X screen
  853x480, output 1280x720 physical / 854x480 logical, a 200x120 X11 window →
  300x180 physical, `buffer_scale` 1). 8 → 9 tests.
- `docs/xwayland-scaling.md`, `docs/design/02-compositor.md`,
  `docs/design/adr/0152-integer-scaled-xwayland.md` (Update section),
  `docs/design/adr/0153-per-output-chrome-sizing-and-nested-scale.md` (new).

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cargo test -p dragonfruit-compositor` — all suites green (bin unit 286,
  `shell_protocol_conformance` 40, `xwayland_conformance` 9).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (`/tmp/opencode/t146-e2e.log`).
- Live visual check: `/tmp/opencode/t146-frac-active.png` (2115x1437) +
  `/tmp/opencode/t146-frac-full.png` via `/tmp/opencode/capture-t146-frac.sh`
  (`DRAGONFRUIT_NESTED_SCALE=1.5`, `display.scale=1.5` seeded into
  `$XDG_RUNTIME_DIR/<socket>.session/config/dragonfruit/settings.json`, then
  `make dev-full`). Vision found the wallpaper **filling the whole output**,
  the menu bar and Dock present/correctly sized, nothing clipped or oversized;
  text slightly soft from the shell's 1x QML raster (shell-side follow-up).

Decisions / gotchas for T-16.4 and later:

- **The damage tracker's static scale is the element-geometry scale.** Any
  backend created with `OutputDamageTracker::new` must pass the output's real
  scale and rebuild on scale change, or every element is sized wrong. Nested
  is the only backend that builds a static tracker by hand (DRM uses the
  compositor's auto mode source; headless renders nothing).
- **A scale change is a geometry change.** Any output mode/scale/transform
  change must `reconfigure_layers()`, because chrome layer sizes are logical
  functions of the output geometry. Reserved thicknesses are logical and do
  not change.
- **Chrome now gets a fractional preferred scale; the shell ignores it.** The
  shell does not bind `wp_fractional_scale`/`wp_viewporter`, so at a fractional
  scale its QML buffers upscale. See PROGRESS.md Follow-ups (T-16.3b shell-side
  sharpness) — this is the remaining "blurry chrome" gap, shell-only.
- **Xwayland renders at the output's logical size, not an integer supersample.**
  In this environment (`Xwayland 24.1.13`) an X11 surface's buffer is
  `buffer_scale=1` at the logical size even at output scale 2, so the
  compositor upsamples at >1. T-16.3a's "integer-scaled buffer downscaled"
  description was inaccurate and is corrected in `docs/xwayland-scaling.md`.
  Xwayland does **not** bind `wp_fractional_scale` for its X11 surfaces.
- **`query scale` is append-only** and reports the model logical rect plus the
  `to_physical_precise_round` mapping the render layer uses; use it for any
  future per-output sizing assertion. The window line's `physical` uses
  comma-separated `x,y,w,h` tokens (the output line keeps `WxH`).
- **Nested cannot add outputs**, so the multi-output fractional matrix is still
  the headless synthetic-output harness + the batched human VM step.

## T147 — T-16.6a AT-SPI and keyboard-only audit

**State: done.** The live AT-SPI dump/walkthrough and the keyboard-only
operation proof both pass. Contract frozen in ADR 0154.

Real paths:

- `scripts/t16-a11y-audit.sh` + `scripts/t16-a11y-audit.py` (new;
  `make t16-a11y-audit`): launch the nested demo twice (Settings, Files) with
  `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1` and `DRAGONFRUIT_SYNTHETIC_INPUT`, dump
  the live AT-SPI tree, fail on any unnamed interactive node, and drive a
  keyboard-only walkthrough reading focus/pane changes back from AT-SPI.
  Writes `docs/captures/t16-a11y-atspi.txt` (both passes `RESULT: PASS`) and
  `docs/captures/t16-a11y-desktop.png`.
- `compositor/tests/shell_protocol_conformance.rs` — new
  `keyboard_only_walkthrough_dispatches_every_system_binding` (40 → 41 tests):
  every default system chord through the synthetic path, asserting the outbox
  records the action with a `keyboard` trigger.
- `docs/design/adr/0154-atspi-and-keyboard-audit-boundary.md` (new);
  `docs/design/04-shell.md` (T-16.6a status); `docs/design/10-design-system.md`
  (quality-gate layering); `docs/captures/README.md`.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cargo test -p dragonfruit-compositor --test shell_protocol_conformance` —
  41 passed.
- `cargo fmt --all -- --check` and
  `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` — clean.
- `make e2e` — EXIT 0 (`/tmp/opencode/t147-e2e.log`).
- `bash scripts/t16-a11y-audit.sh` — PASS (host Wayland + pyatspi + Pillow +
  spectacle; not in `make e2e`).
- Live visual check: `docs/captures/t16-a11y-desktop.png` (2115x1437); vision
  found the desktop fully composited (menu bar, Dock, wallpaper, Settings, X11
  client), sharp text, no artifacts.

Decisions / gotchas for T-16.6b and later:

- **The shell offscreen chrome is not on the AT-SPI bus.** Qt's Linux AT-SPI
  bridge registers from shown `QWindow`s via the platform plugin; the offscreen
  QPA exposes no accessibility backend, so the shell (menu bar, Dock, Control
  Center, OSD, dialogs) never appears to `pyatspi` — only the first-party apps
  do. This is architectural (ADR 0154), not a missing test. Do not try to
  "fix" it per component; a compositor-side a11y bridge is a new track.
- **Live keyboard walkthrough needs `DRAGONFRUIT_SYNTHETIC_INPUT` and
  `QT_LINUX_ACCESSIBILITY_ALWAYS_ON=1`** exported before the demo; the
  compositor's synthetic harness works on the nested backend too. The client
  binds `<path>.reply`; `key <evdev> down|up` chords reach the focused client.
- **AT-SPI focus is the live proof.** `pyatspi` `STATE_FOCUSED` reflects Qt
  focus; Tab order in Settings is Close→Minimize→Zoom→Search→Sidebar→content.
  After 6 Tabs one Shift+Tab returns to the Sidebar (Down + Return activates a
  pane; read the `tool bar` name back). This is deterministic and used by the
  audit.
- **The global-flow proof is the headless test**, not the live dump: the
  compositor test exercises all 16 system chords. Any new system shortcut must
  be added to `default_system_bindings()` and to that test's `walkthrough`
  table.

## T148 — T-16.6b Magnifier and reduced-motion sweep

**State: done.** Two units landed: a compositor-owned screen magnifier and an
always-on reduced-motion sweep. Contract in ADR 0155.

Real paths:

- `compositor/src/magnifier.rs` (new) — the pure `Magnifier` policy: zoom
  (clamped 1.0..=8.0, 1.0 disables), the scene point kept at the output centre,
  `MagnifierMode::{FollowFocus,FollowCaret}`, `map_from_view`/`map_to_view`/
  `set_center`/`follow`/`seed_center`, and `view_transform` (the physical
  Rescale(+Relocate) numbers). 10 unit tests.
- `compositor/src/state.rs` — `DfState.magnifier` + `set_magnifier_enabled`/
  `_zoom`/`_mode`/`_center`, `magnifier_output_geometry`,
  `magnifier_geometry_at`, `refresh_magnifier_follow` (called from
  `focus_changed`).
- `compositor/src/backend/nested.rs` — `NestedFrameElements`
  (`Plain`/`Magnified`) + `magnify_elements`: when active, every element is
  `Rescale`d about the output centre and translated; the live frame and the
  capture render the same wrapped list. `age = 0` while active forces a full
  damage frame.
- `compositor/src/input.rs` — `magnified_location` maps a view point to the
  scene point under it; `magnified_delta` divides relative motion by the zoom;
  applied to absolute pointer, relative pointer, and touch.
- `compositor/src/input/synthetic.rs` — `set magnifier on|off`,
  `set magnifier-zoom <f>`, `set magnifier-mode follow-focus|follow-caret`,
  `set magnifier-center <x> <y>`, `query magnifier` (enabled/active/zoom/mode/
  centre/output/origin/translation) + parse tests.
- `compositor/tests/window_conformance.rs` — new
  `magnifier_reports_its_view_transform_and_zoom` (36 tests total).
- `compositor/tests/reduced_motion_sweep.rs` (new; in `make e2e` via the
  Makefile) — 3 tests: (a) the Rust and QML motion catalogs are parsed and
  cross-checked, every token has `reduced_ms == 0` and `full_ms > 0`;
  (b) every QML animation site in `shell/`, `design-system/`, `apps/` resolves
  through `Theme.motion` or gates on `Theme.reducedMotion` (56 sites);
  (c) every `WindowMotionKind` routes through `Tween::from_motion` and the
  overview keeps its reduced single-step branch.
- `scripts/capture-t16-magnifier.sh` (new) + `docs/captures/t16-magnifier.png`
  — the A/B live capture (off top / 2x centred bottom).
- `docs/design/adr/0155-compositor-magnifier.md` (new);
  `docs/design/02-compositor.md`, `docs/design/07-system-integration.md`,
  `docs/design/10-design-system.md`, `docs/captures/README.md`.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cargo test -p dragonfruit-compositor` — all suites green (lib 11, bin unit
  296, `reduced_motion_sweep` 3, `window_conformance` 36,
  `shell_protocol_conformance` 41, `xwayland_conformance` 9, the rest as
  before).
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` and
  `cargo fmt --all -- --check` — clean.
- `make e2e` — EXIT 0 (`/tmp/opencode/t148-e2e-final.log`).
- Live visual check: `docs/captures/t16-magnifier.png` (1057x1466 stack).
  Vision confirmed the bottom half is a 2x centred zoom of the top (UI larger,
  top bar and Dock cropped out) with no blank/black/torn regions. The
  on→off probe (`/tmp/opencode/probe-t148-off.sh`) shows 0.0% sampled pixel
  change after toggling on then off — no stale scaled damage.

Decisions / gotchas for later tasks:

- **The magnifier's durable trigger is not wired.** The Settings row,
  its settingsd key, and the private-protocol request that carries the value
  are out of this task's `compositor/`, `shell/` area, so the magnifier is
  only reachable through the synthetic seam today. ADR 0155 records the
  contract; wiring the row/key/request is the bounded follow-up.
- **DRM magnification is not wired.** The nested backend applies the element
  rescale; the DRM rail does not yet, and its cursor must **not** scale with
  the scene. Nested is the demo/dev backend and is what the capture proves.
- **`follow-caret` falls back to the focused window** because the compositor
  has no caret-position source. The mode is modelled and accepted; a later
  text-input caret source makes it real without changing the contract.
- **Input mapping is required, not optional.** `map_from_view` maps the
  physical pointer to the scene point under it and relative deltas are divided
  by the zoom; without it a rendering magnifier silently misroutes clicks.
- **The damage tracker handles the transform change.** Forcing `age = 0` while
  active plus the per-element geometry change is enough; toggling off returns
  to identical pixels.
- **The reduced-motion sweep fails on a literal-duration animation.** Scope of
  the site scan: `shell/`, `design-system/components`, `design-system/gallery`,
  `apps/`, excluding `tests/`. A new animation must use `Theme.motion.*` or an
  `enabled/running` guard on `Theme.reducedMotion`. The compositor half scans
  `design_tokens.rs`/`Theme.qml` (generated from
  `design-system/tokens/tokens.json`) and the lifecycle enum.

## T149 — T-16.7 Localization and i18n

**State: done.** The shell and first-party apps are translatable and a locale
switch translates them; dates/numbers follow the locale. Contract in ADR 0156.

Real paths:

- `libs/i18n/` (new static lib `dragonfruit-i18n`) — `i18n.cpp`/`i18n.h`:
  `CatalogTranslator` (a `QTranslator` that parses Qt `.ts` XML at runtime with
  `QXmlStreamReader`; context+source lookup then source-only fallback),
  `resolveLocale()` (`DRAGONFRUIT_LOCALE` then `QLocale::system()`),
  `catalogSearchPaths()` (`$DRAGONFRUIT_TRANSLATIONS_DIR` → compiled-in
  source tree → `../share/dragonfruit/translations` → `/usr[/local]/share/...`),
  `installTranslations[For]()` (installs the catalog and `QLocale::setDefault`).
  9 tests in `libs/i18n/tests/tst_i18n.cpp`.
- `shell/src/main.cpp`, `apps/settings/main.cpp`, `apps/files/main.cpp` —
  `Dragonfruit::installTranslations(app)` before QML loads; the executables link
  `dragonfruit-i18n`.
- `apps/files/FilesDirectoryModel.cpp` — size/modified now use the default
  `QLocale()` (not `QLocale::system()`), and the Kind strings
  (`Folder`/`File`/`Alias`/`Item`) are translated.
- `scripts/i18n-extract.py` + `make check-i18n` (in `make lint` + CI) — the
  string-extraction gate: `--check` fails on source/template drift and on a
  bare literal in a user-visible QML property; `--update` regenerates the
  template. Scopes: `shell/`, `apps/`, `design-system/components`, excluding
  `tests/`.
- `translations/dragonfruit.ts` (template, 909 strings / 65 contexts),
  `translations/dragonfruit_es.ts` (458 Spanish translations / 45 contexts).
- `scripts/capture-t16-i18n.sh` + `make t16-i18n-capture` —
  `docs/captures/t16-i18n-{en,es}.png` and stacked `t16-i18n.png`.
- CMake install rule puts the `.ts` files under
  `${CMAKE_INSTALL_DATADIR}/dragonfruit/translations`.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cmake --build build`, then `ctest --test-dir build` — 70/70 pass (new
  `tst_i18n`).
- `./scripts/i18n-extract.py --check` — OK (909 strings / 65 contexts).
- `make e2e` — EXIT 0 after the change.
- `bash scripts/capture-t16-i18n.sh` — PASS (host Wayland + Pillow + spectacle;
  not in `make e2e`).
- Live visual check: `docs/captures/t16-i18n.png` (1920x2400 stack). Vision
  found the bottom half fully Spanish (title `Ajustes`, sidebar labels, `Buscar`,
  General header/description/absence message), top half English, no leakage,
  no clipped rows, no artifacts.

Decisions / gotchas for later tasks:

- **No Qt Linguist tools in the toolchain.** `.ts` files are the shipped
  artifact and are parsed at runtime; there is no `lrelease`/`.qm` step.
  `lupdate` will still read the files if ever adopted.
- **Locale is chosen at process start**, not switched in place. Runtime
  switching would need `QQmlEngine::retranslate()` and a language-change event.
- **Spanish is a partial reference locale** (458/909); missing entries fall back
  to source text. Product names identical in Spanish (Bluetooth, Mission
  Control, General) are intentionally unchanged. RTL polish deferred.
- **The gate's bare-literal scan is QML-only.** C++ literal strings are only
  caught when wrapped in `tr()`/`QCoreApplication::translate`; a bare C++
  literal assigned to a user-visible label is not detected.
- Escape hatch for a genuinely non-translatable property: a
  `// df-allow-untranslated` comment on the line (or the line above).

## T150 — T-16.8a Crash/kill matrix

**State: done.** The crash/kill matrix covers every restartable shipped
component headlessly. Contract in ADR 0157.

Real paths:

- `services/session/tests/kill_matrix.rs` (5 → 9 tests) — the stand-in plan is
  derived from `SessionPlan::default_session()` (name/stage/policy/anchor/gate
  preserved), so the matrix cannot drift. New:
  `every_restartable_service_restarts_in_place_and_keeps_the_session`,
  `the_restartable_set_is_exactly_the_shipped_services`,
  `killing_the_portal_backend_restarts_it_and_keeps_the_session`,
  `killing_an_app_is_a_client_exit_not_a_session_event`; the compositor-death
  test stops the whole shipped set.
- `compositor/tests/window_conformance.rs` — new
  `a_crashed_app_leaves_the_compositor_and_the_other_app_running` (36 → 37):
  two real Wayland clients; one hard-closes its socket; only its window goes.
- `scripts/t16-kill-matrix.sh` + `make t16-kill-matrix` (new, depends on
  `cargo-build`) — headless transcript `docs/captures/t16-kill-matrix.txt`
  (session kill_matrix + compositor app-crash + lock fail-secure; all PASS).
- `docs/captures/t16-kill-matrix.md` (reviewed matrix + live check) and
  `docs/captures/t16-kill-matrix.png` (live nested capture).
- `docs/design/adr/0157-t16-crash-kill-matrix.md` (new);
  `docs/design/11-session-and-dev-workflow.md` (kill-matrix table grown);
  `docs/captures/README.md`.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cargo test -p dragonfruit-session --test kill_matrix` — 9 passed.
- `cargo test -p dragonfruit-compositor --test window_conformance` — 37 passed.
- `cargo clippy -p dragonfruit-session -p dragonfruit-compositor --all-targets
  -- -D warnings`, `cargo fmt --all -- --check` — clean.
- `make t16-kill-matrix` — all rows passed.
- Live visual check: `/tmp/opencode/t150-desktop.png` → committed
  `docs/captures/t16-kill-matrix.png` (1920x1080). Vision found the nested
  Dragonfruit desktop fully composited (menu bar, Dock, wallpaper, Settings,
  demo client), no artifacts. No surface of its own changed.

Decisions / gotchas for T-16.8b and later:

- **Compositor death is an outcome row, not a recovery row.** T-16.8b owns the
  compositor-death behavior/restart-policy docs.
- **Apps are not session services.** The shipped plan has no app entry; the
  session test models an app as a `Never` service, and the compositor test
  proves a real client disconnect is non-fatal. Never add apps to
  `SessionPlan::default_session`.
- **Adding a restartable session service fails
  `the_restartable_set_is_exactly_the_shipped_services`** until the matrix
  covers it; keep the test derived from `default_session()`.
- **The real-binary `kill -9` drill is manual/VM** (recorded in
  `docs/captures/t16-kill-matrix.md`); T-17.5a re-verifies a subset.

## T151 — T-16.8b Compositor-death behavior and restart-policy docs

**State: done.** Compositor death is documented once as session-ending and the
restart policy is enforced by a headless 3x3 matrix. Contract in ADR 0158.

Real paths:

- `services/session/tests/restart_policy_matrix.rs` (new; 3 tests) — the
  restart-policy matrix. Crosses `always`/`on-failure`/`never` with
  `exit 0`/non-zero/signal; crosses compositor death with the same three exits
  (session ends, survivors stop, anchor restarts == 0); and pins the shipped
  anchor (`SessionPlan::default_session()`) as `Never` + `ends_session` with
  `validate()` rejecting a restartable anchor.
- `docs/design/adr/0158-compositor-death-ends-the-session.md` (new) — one
  decision: compositor death ends the session, it is never restarted, recovery
  is a fresh login via the T-12.2 entry, no live handoff, nested ends only the
  nested session, lock stays fail-secure, Xwayland subprocess restarts are not
  session restarts. Includes the restart-policy table.
- `docs/design/11-session-and-dev-workflow.md` — new "Compositor death ends
  the session (T-16.8b)" subsection; the kill-matrix paragraph now links ADR
  0158 instead of "documented by T-16.8b".
- `docs/captures/t16-kill-matrix.md` — "Restart policy matrix (T-16.8b)"
  section; compositor row points at the new test + ADR 0158.
- `scripts/t16-kill-matrix.sh` — new section "1b. The restart-policy matrix";
  `docs/captures/t16-kill-matrix.txt` regenerated (all rows PASS).
- `docs/captures/README.md` — lists the new row and ADR 0158.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cargo test -p dragonfruit-session --test restart_policy_matrix` — 3 passed.
- `cargo test -p dragonfruit-session` — all suites green.
- `cargo clippy -p dragonfruit-session --all-targets -- -D warnings`,
  `cargo fmt --all -- --check` — clean.
- `make t16-kill-matrix` — all rows passed.
- Live visual check: `/tmp/opencode/t151-desktop.png`. Vision confirmed the
  nested desktop fully composited, no blank/black/torn regions. No surface
  changed; no committed PNG.

Decisions / gotchas for later tasks:

- **The restart-policy contract lives in `services/session`, not
  `compositor/`.** The task area says `compositor/ + packaging/`, but the
  policy is data (`SessionPlan`/`Supervisor`); the matrix test sits beside
  `kill_matrix.rs`/`restart.rs`. No compositor code changed.
- **The unit-level half is separate.** `Restart=` vs `plan.rs` is enforced by
  `services/session/tests/units.rs`; the new matrix is the pure policy/exit
  half. Changing a policy requires updating both.
- **A restartable anchor is a plan error** (`PlanError::RestartableSessionAnchor`);
  the matrix's third test guards it so no future change can satisfy
  compositor-death by restarting it.
- **The real-binary `kill -9` drill is still manual/VM** (recorded in
  `docs/captures/t16-kill-matrix.md`); T-17.5a re-verifies a subset.

## T151a — T-16.12 Synthetic chrome pointer injection timestamps

**State: done.** Every chrome surface that re-injects the compositor's pointer
events into its offscreen QML window now goes through one timestamped helper, so
the zero-timestamp `DragHandler` grab that killed Dock clicks (T-14.7x) cannot
recur on the menu bar, Control Center, chooser, screenshot, screencast, polkit,
overview, or notification banner. The rule in ADR 0110 is now chrome-wide.

Real paths:

- `shell/src/chromepointer.{h,cpp}` (renamed from `dockpointer.*`, still in
  `dragonfruit-shell-dockcore`) — `ChromePointer::timestamp()` (one shared
  `QElapsedTimer`, `msecsSinceReference() + 1`, bumped past the previous value)
  and `ChromePointer::send(window, type, pos, button, buttons)`. `namespace
  DockPointer = ChromePointer;` is the compatibility alias, so the Dock path and
  `tst_dock`'s `DockInject` are unchanged.
- `shell/src/shellcontroller.cpp` — all pointer handlers (menu bar / main,
  Control Center, chooser, screenshot, screencast, polkit, overview, banner,
  Dock) call `ChromePointer::send` for move / button / leave, keeping their
  existing `m_*Buttons` bookkeeping; `#include <QMouseEvent>` removed.
  `grep -n "QMouseEvent(" shell/src/shellcontroller.cpp` is empty.
- `shell/tests/tst_chromepointer.cpp` (new unit test) — first timestamp
  nonzero, strictly increasing across a sequence, every delivered event
  stamped, null window a no-op.
- `shell/tests/tst_chromepointerui.{cpp,qml}` (new) — `ChromeInject` context
  property over the production helper; a stationary injected tap beside a
  `DragHandler` taps and a slop drag lifts (non-Dock surface, the T110x probe
  repeated).
- `shell/tests/tst_dock.cpp` / `tst_dock.qml` — only the include/type name and a
  comment changed; Dock behavior identical.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cmake --build build` — exit 0.
- `ctest --test-dir build -R "tst_dock$|tst_dockcore|tst_menubar|tst_controlcenter$|tst_controlcenterpolicy|tst_overview|tst_chromepointer"` — 8/8 green.
- `make qml-test` — 72/72 pass (was 70; the two new tests).
- `make e2e` — exit 0.
- `make check` — fails only at `check-desktop-names` on the pre-existing
  StatusNotifier/zoo/apppicker lines (same as the T110x note); every other
  component green (`fmt-check`, `clippy`, `check-tokens`, `check-design-tokens`,
  `check-no-capture-grab`, `check-i18n`, `cargo-test` 143 suites, `visual-test`
  78 snapshots, `soak` 100 clean cycles).
- Pre-fix probe: zeroing the timestamp in `chromepointer.cpp` and rebuilding
  `tst_chromepointerui` yields 3 passed / 1 failed (the stationary tap does not
  tap); restored green.
- Live: `make demo` nested + synthetic pointer. Menu-bar status click opened a
  popover; a Control Center panel click closed the panel and opened Settings >
  Appearance. Captures `/tmp/opencode/t151cap/*.png`, `/tmp/opencode/t151-post.png`.

Decisions / gotchas for later tasks:

- **One constructor, one timestamp source.** A call site that builds its own
  `QMouseEvent` is the bug; grep for `QMouseEvent(` in the shell pointer
  handlers must stay empty. Later chrome input must call `ChromePointer::send`.
- **The compatibility alias is intentional.** `DockPointer` still resolves to
  `ChromePointer`; the Dock method names (`onDockPointerMoved/Button/Left`) are
  unchanged and are not a violation.
- **Test seam pattern.** A QML injection test wraps `ChromePointer` in a
  context-property object (`DockInject`, `ChromeInject`) that mirrors the
  controller's button-state bookkeeping; QtTest's own `mouseClick` stamps its
  events and hides the bug.
- **Live tile-click captures have stale geometry.** The existing Control Center
  capture drivers assume output scale 1.0 and wallpaper-based window detection,
  but the nested output reports `scale=1.1600` on this host, so fixed switch
  coordinates miss. Not caused by this change; a driver refresh would key
  coordinates off the reported scale.

## T152 — T-17.1a Nested window loop verification

**State: done.** The nested window loop is verified with a committed capture.
The T-17 premium gate's first verification unit leaves no product code; it adds
a reproducible scripted loop and its evidence. `make e2e` is green and the live
capture was reviewed.

Real paths:

- `scripts/capture-t17-window-loop.sh` + `scripts/t17-window-loop-driver.py`
  (new) — `make demo` nested with the synthetic-input harness, a real
  third-party Qt SSD client (`kcalc`) started against the private socket, then
  launch/appear → focus → move (exact (90,70) titlebar drag) → zoom → minimize
  → restore → close on the Qt SSD window, plus the CSD-unaffected and X11 SSD
  checks. `make t17-window-loop-capture`.
- `docs/captures/t17-window-loop.{png,mp4,txt}` and the step stills
  (`-focused`, `-move`, `-zoom`, `-minimized`, `-restored`, `-closed`, `-csd`,
  `-x11`). `t17-window-loop.txt` is the JSON transcript of the
  `query decorations`/`query identity` report around each primitive.
- `docs/captures/t17-window-loop.md` (reviewed matrix + live check),
  `docs/design/adr/0159-t17-nested-window-loop-capture.md` (new),
  `docs/captures/README.md`, `docs/design/11-session-and-dev-workflow.md`,
  `Makefile`.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cargo test -p dragonfruit-compositor --test window_conformance --test
  xwayland_conformance --test milestone_e2e` — 37 + 9 + 1 passed.
- `make e2e` — exit 0; 129 `test result: ok`.
- `make t17-window-loop-capture` (or `bash scripts/capture-t17-window-loop.sh`)
  — exit 0; loop complete.
- Live visual check: `docs/captures/t17-window-loop.png` + step stills via the
  vision model — menu bar and Dock present, Qt SSD traffic lights, no
  blank/torn/ghosted regions.

Decisions / gotchas for T-17.1b, T-17.1c, and T-17.2:

- **Capture by active window, not wallpaper colour.** T-18 ships a photo
  wallpaper, so the T-01 driver's `WALL=(45,35,51)` nested-window detection no
  longer finds the window. The T-17 driver raises the nested window via the
  KWin scripting D-Bus and crops the active-window capture (the T-16.7
  pattern); window tiers come from `query decorations`/`query identity`, and a
  window occluding the loop titlebar is dragged clear first.
- **The first-party Qt apps are CSD by design.** `apps/settings`/`apps/files`
  use `Qt.FramelessWindowHint` and draw the design-system `TitleBar`, so the
  loop's *SSD* traffic lights need a real third-party Qt client. `kcalc` is
  installed and negotiates `ssd=True`; without it the loop falls back to the
  X11 window and records the deviation.
- **CSD titlebar drags need a stepped motion.** A CSD client turns the press
  into an `xdg_toplevel.move` request, so an abrupt `down;motion;up` is lost;
  the driver pauses after the press and sends the delta in steps. This also
  fixed the `clear-the-stage` drag.
- **Restore-from-Dock is not synthesized in the agent capture.** `kcalc` is an
  unpinned running app: minimizing makes its tile appear and the pinned tiles
  re-center, so a Dock-band pixel diff can hit a pinned app (it launched an
  extra Files window in a trial). The capture uses the compositor `restore
  <id>` primitive instead; the Dock-tile click is human-verified and pinned
  headlessly by the T-01/T-02 suites. A stable Dock-tile locator is a
  follow-up.

## T153 — T-17.1b Workspace, Mission Control, and app-switch verification

**State: done.** The T-17 premium gate's navigation verification unit lands
with a committed capture. It adds no product code; it adds a reproducible
scripted navigation loop and its evidence over the already-green headless
conformance. `make e2e` is green (129 `test result: ok`) and the live capture
was reviewed.

Real paths:

- `scripts/capture-t17-navigation.sh` + `scripts/t17-navigation-driver.py`
  (new) — `make demo` nested with the synthetic-input harness, then workspace
  switching (keyboard `Ctrl+Left`/`Ctrl+Right`, a three-finger swipe caught
  mid-slide, and a pointer click on a Mission Control workspace-strip card),
  Mission Control (keyboard `Ctrl+Up` and a top-left hot-corner dwell), and app
  switching (keyboard `Cmd+Tab` with commit on modifier release, and a pointer
  click on a live preview). Every path is asserted through `query spaces`,
  `query grid`, `query wallpaper`, and `query switcher`; a path that does not
  reach the documented state fails the run.
- `docs/captures/t17-navigation.{png,mp4,txt}` and the step stills
  (`-workspace-keyboard`, `-workspace-gesture`, `-mission-control-keyboard`,
  `-mission-control-pointer`, `-workspace-pointer`, `-app-switch-keyboard`,
  `-app-switch-pointer`, `-app-switch-pointer-committed`). The `.txt` is the
  JSON transcript of the four queries at each step.
- `docs/captures/t17-navigation.md` (reviewed matrix + live check),
  `docs/design/adr/0160-t17-navigation-capture.md` (new),
  `docs/captures/README.md`, `docs/design/11-session-and-dev-workflow.md`,
  `Makefile` (`t17-navigation-capture`).

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cargo test -p dragonfruit-compositor --test window_conformance --test
  shell_protocol_conformance` — 37 + 41 passed.
- `make e2e` — exit 0; 129 `test result: ok`.
- `make lint` — fails only at `check-desktop-names` (pre-existing
  StatusNotifier/`org.kde` + `scripts/zoo/zoo-run.sh`); all other components
  green. No new file is flagged.
- `make t17-navigation-capture` (or `bash scripts/capture-t17-navigation.sh`)
  — exit 0; navigation loop complete.
- Live visual check: `docs/captures/t17-navigation*.png` via the vision model —
  menu bar and Dock present, the Mission Control grid with the workspace strip,
  the two live surfaces, and the app-switcher scrim + cards; no
  blank/torn/ghosted regions.

Decisions / gotchas for T-17.1c and T-17.2:

- **Observe through the compositor, not the pixels.** The four `query *`
  reports are the evidence; the stills are the reviewer's aid. The app-switcher
  chrome is a faint scrim plus cards, and the vision model reads it (and the
  committed T-06 stills) as a plain desktop — trust the transcript and the
  frame delta.
- **One trigger per column, same state machine.** Keyboard, gesture, hot
  corner, and pointer all resolve to the same `InputAction` and the same
  overview/switcher machine, so the Mission Control keyboard and pointer stills
  are byte-identical by design.
- **The Mission Control strip-card geometry lives in the compositor too.**
  `strip_card_rect` (`compositor/src/overview/grid.rs`) and the shell's centered
  `Row` agree: top = `menu_bar::HEIGHT (28) + overview::STRIP_MARGIN (16)`,
  cards 132x84, gap 12. The driver reproduces it for the pointer workspace
  switch.
- **A Space card click activates but does not dismiss.** `onOverviewWorkspaceActivated`
  sends `activateWorkspace` only; the keyboard/gesture path dismisses. Recorded,
  not changed.
- **Mission Control window-card pointer selection is not synthesized.** The
  shell's centered title-card row is not the live-surface rect; the synthetic
  click did not reliably hit it. The compositor round-trip is headless-pinned
  by `overview_click_selects_and_focuses_the_live_representation`; the pointer
  workspace switch (strip card) *is* live-captured.

## T154 — T-17.1c Flatpak/browser end-to-end verification

**State: done.** A real Flatpak browser (`org.mozilla.firefox`) file-chooses,
screenshots, and screen-shares on the nested session with the **live shell** as
the portal presenter, and the nested capture is committed. The verification
surfaced and fixed one real presenter bug. `make e2e` is green (129
`test result: ok`) and the live capture was reviewed.

Real paths:

- `scripts/capture-t17-flatpak-browser.sh` + `scripts/t17-flatpak-browser-driver.py`
  + `scripts/t17-flatpak-flow.py` (new) — a private session bus runs the real
  `xdg-desktop-portal` frontend with the Dragonfruit backend, `make demo` runs
  nested with the synthetic-input harness, and the flow client runs *inside*
  `flatpak run org.mozilla.firefox`. The host driver completes each shell
  presenter (picker / selection overlay / source picker) by synthetic input and
  fails unless the client prints `FLOW: RESULT: PASS`. `make
  t17-flatpak-browser-capture`.
- `docs/captures/t17-flatpak-browser.{png,mp4,txt}` and the step stills
  (`-file-choose`, `-file-choose-accepted`, `-screenshot`,
  `-screenshot-accepted`, `-screen-share`, `-screen-share-accepted`).
  `t17-flatpak-browser.txt` is the JSON transcript of the client's portal calls
  and responses.
- `docs/captures/t17-flatpak-browser.md`,
  `docs/design/adr/0161-t17-flatpak-browser-capture.md` (new),
  `docs/captures/README.md`, `docs/design/11-session-and-dev-workflow.md`,
  `Makefile`.
- **Product fix**: `shell/src/chooserbridge.cpp` `suggestedUri` now trims the
  trailing NUL from `current_file`/`current_folder`; `shell/tests/tst_chooser.cpp`
  pins the real NUL-terminated option.

Commands that work (repo root; `PKG_CONFIG_PATH=$HOME/.local/df-devroot/lib64/pkgconfig`,
`RUSTFLAGS=-L $HOME/.local/df-devroot/lib64`):

- `cmake --build build -j` — exit 0.
- `ctest --test-dir build -R "tst_chooser$|tst_screenshot$|tst_screencast$"` —
  3/3 passed.
- `make e2e` — exit 0; 129 `test result: ok`.
- `make lint` — fails only at `check-desktop-names` (pre-existing
  StatusNotifier/`org.kde`, `scripts/zoo/zoo-run.sh`, and
  `scripts/capture-t17-window-loop.sh`); no new file flagged.
- `make t17-flatpak-browser-capture` (or `bash
  scripts/capture-t17-flatpak-browser.sh`) — exit 0; all three flows `PASS`.
- Live visual check: `docs/captures/t17-flatpak-browser*.png` via the vision
  model — picker with `picked.txt`, region overlay with the badge, and the
  `Share your screen` card naming the browser + `NESTED-1`; no artifacts.

Decisions / gotchas for T-17.2:

- **Qt's `ay` `current_folder` carries a trailing NUL.** `QFile::decodeName` on
  a Qt-decoded D-Bus `ay` kept the NUL, so `QUrl::fromLocalFile` produced
  `…%00` and the listing failed with `Cannot open file:%00.`. Trim trailing
  NULs (the portal backend's `decode_path_bytes` does the same). The T-13
  fixture tests missed it because they pass a bare `QByteArray`.
- **The live shell is the presenter; the browser's response is the proof.**
  The host stand-in of T-13.7 is replaced by real synthetic input into the
  shell's picker/overlay/source-picker, and the run asserts the returned
  file/screenshot URI or stream list.
- **Diff-calibrate the overlay rect.** The picker and source picker are the
  only change between a pre-flow baseline still and the mapped still, so the
  driver recovers their rects (`640x440` at `(640,378)`, `560x460` at
  `(680,368)`) instead of hard-coding geometry. The screenshot overlay is
  full-output and driven by a region drag.
- **KWin/Spectacle use the host session bus.** The demo is on the private
  portal bus; pin `DBUS_SESSION_BUS_ADDRESS` to the host bus for the
  active-window raise and `spectacle -a`, or the capture fails silently.
- **The screen-share stream is the stills fallback** (`df_fallback =
  pipewire-producer-unavailable`, `id = monitor:NESTED-1`) on this host; the
  live PipeWire producer is a host capability. The screenshot flow writes one
  real PNG to `~/Pictures/Screenshots`.

## Follow-ups

- **Live PipeWire screencast producer.** The ScreenCast portal flow returns the
  documented stills fallback on this host; a capture with a real PipeWire
  producer would show a live stream instead (T-17.2/T-13.4b territory).
- **Stable Mission Control window-card locator.** So a future capture can
  synthesize the pointer selection round-trip; the shell's cards are centered
  title cards, not the compositor's live-surface rects, and the headless
  round-trip already passes.
- **Stable Dock-tile locator for a synthesized restore-from-Dock click.**
  The agent capture cannot reliably click the minimized tile of an unpinned
  running app (see above); T-17.5a/T-17.2 may want the real click automated.
- **The nested output scale seam.** Captures on this host still assume the
  nested logical 1920x1200 maps 1:1 after the active-window crop; if a future
  host reports a different nested scale, the per-window crop boxes need to be
  keyed off the reported scale (same class as the T151a Control Center note).
- **`check-desktop-names` fails on `HEAD` (pre-existing, blocks `make check`).**
  The gate flags `org.kde.*`/`org.gnome.*`/`org.kde.KWin` in
  `services/app-index`, `shell/src/trayclient.h`, `shell/tests/tst_dockcore.cpp`,
  `scripts/zoo/zoo-run.sh`, `scripts/capture-t17-window-loop.sh`, and the
  `Makefile` zoo comment — none touched by T176b. Reproduced on a pristine `HEAD`
  worktree. A future task should either add `df-allow-desktop-name` markers or
  widen the gate; until then `make check` aborts at lint even though every other
  lint/test/soak gate is green.
- **`skeleton_dark.png` gallery golden is shimmer-phase flaky.** Repeated
  `--update` runs produce different hashes; `--strict` still passes against the
  committed golden, so the phase should be pinned (or the shimmer captured at a
  fixed progress) before trusting an `--update` diff on that page.
- **T-19.2 (Applications drawer) needs the first-party mapping to be shared.**
  The desktop-id → bundled-tile map lives in `shell/dock/DockGlyph.qml`
  (`bundledIcon`/`firstPartyId`), which the drawer should not import. Lift it to
  a small shared helper/singleton (or a design-system API) when the drawer
  starts consuming it, and have `DockGlyph` call through it; keep the key the
  normalized desktop id (see the T176d gotcha about not matching `appId`).

## T-14.7aa — Dock hover reference polish: bar geometry, zoom profile, and label tail

**State: done.** The Dock's hover state is retuned to
`docs/reference/macos/Dock_Tile_Mouseover.png` (local-only, never shipped).
Pixel analysis of the capture (2272x300 at 2x, 2026-09-28) measured the plate,
the tooltip, and the zoom profile deterministically; the vision markdown's
"white text on translucent background" tooltip claim is wrong (the capsule is a
light pill with dark text). Four things changed:

- **The background geometry.** macOS keeps the dock background a constant
  *height* and gives it exactly two *horizontal* levels — base and one zoomed
  level — that never vary while the pointer moves along the Dock. `plateRect`
  now does the same: the cross axis is always the resting `barThickness`, and
  the along axis is the base length blended to a single `magnifiedPlateLength`
  by the hover engagement. `magnifiedPlateLength` is the swept union of the
  fully magnified row at every resting centre, centred on the surface with the
  end padding, so one fixed bar always contains the row. The T-14.7y
  plate-edge peak-hold (`plateTrackRaw`/`smoothPlateTrack`/its animation) is
  deleted: there is no edge signal left to damp, and the declared compositor
  backdrop panel now changes only on hover engage/release.
- **The zoom bubble.** The measured profile is a quadratic bubble
  (`magnifyFalloff` 3.0 → 4.1): ~90 % of the peak effect one tile away, ~55-60 %
  two, zero by ~4 icon widths. The peak mapping is retuned to the capture's
  1.245× (`magnifyPeak` 1.6 → 1.25, `magnifyPeakMax` 2.2 → 1.5), so the default
  hover matches the reference and the magnified artwork stays inside the fixed
  bar.
- **Continuous geometry.** The discrete anchor pin translated the whole row by
  a full magnified pitch at every tile boundary (~20 px with the old profile,
  ~35 px with the wider reference profile) — the hover twitch a stop-motion
  sweep exposes. Magnified positions are now a continuous warp: accumulate the
  row from the resting leading edge, then translate it so the pointer maps to
  itself through the resting-centre → magnified-centre map.
- **Motion and hover style.** Entering/leaving eases the profile amplitude with
  the new `motion.dockHover` token (a CSS-style ease; the magnet spring reaches
  most of its target in one frame and popped). The anchored tile drops the flat
  hover wash (the zoom *is* the hover state) and shows a small lift shadow. The
  plate gains an interior gloss band and an anchored-edge lip. The Tooltip
  gains an opt-in pointer tail and a pill capsule, used by the Dock.

Reference measurements (2x px): plate height constant at 159 in both the
resting and hovered captures; hovered tile scale 1.245; tooltip capsule
276x52 (fill `166,183,211`, top edge `208,232,255`, dark text `25,27,31`), tail
base ~36 wide x 13 tall pointing at the icon, capsule bottom 32 px above the
icon; no hover wash (plate pixels beside the hovered tile match beside a
resting tile, only a ~3 px tile shadow).

Verification: `ctest --test-dir build` 72/72; `./scripts/gen-tokens.py --check`,
`check-design-tokens.sh`, `i18n-extract.py --check`,
`check-gallery-snapshots.py --strict`, `check-no-capture-grab.sh` all green.
`tst_dock` adds the profile-shape, engage/release, single-zoomed-level,
boundary-continuity, zoomed-wash/shadow, plate-layer, and tooltip-tail cases,
and updates the magnification-geometry cases to the fixed-bar contract. Live
captures refreshed with the flat-wallpaper pin (`DF_DEFAULT_WALLPAPER`):
`t14-dock-tooltip.png`, `t14-dock-tahoe-{dark,light,reduced}.png`,
`t14-dock-magnify-{left,center,right}-{dark,light}.png`,
`t14-dock-plate-corners-*.png`, `t14-dock-plate-magnified-*.png`. A stop-motion
synthetic sweep (23 frames across the row) confirms the plate's top and bottom
edges are identical in every frame while the icons zoom and move.

Gotcha: the capture drivers detect the nested window by the flat wallpaper
colour `(33,13,41)`; on a host where the shipped `Default.jpg` resolves, the
nested session no longer paints it, so the drivers need
`DF_DEFAULT_WALLPAPER=<flat purple png>` (the dev/test override the shell's
shipped-default resolver honours) or a detector update.

## T176a — T-19.1a Vendor Phosphor, QML resource plumbing, glyph primitive

**State: done.** Track 19's foundation lands: the Phosphor icon set is vendored
and licensed, its raw SVGs are exposed to QML at a stable resource prefix, and
a `PhosphorIcon` primitive renders a named glyph tinted/sized from properties.
`make e2e` and the qml/visual/lint gates are green.

Real paths:

- `assets/icons/phosphor/` (new) — Phosphor **2.0.8** (`v2.0.8`,
  `d42782b2abe747d904b971ccab48b182a1455f86`), the pinned **`regular`** and
  **`fill`** weights, flattened with upstream names: `<name>.svg` (regular) and
  `<name>-fill.svg` (fill). 111 curated glyphs (222 files) + `LICENSE` +
  `README.md`. MIT. Add a glyph by copying both weights and running
  `scripts/gen-phosphor-glyphs.py`.
- `design-system/PhosphorGlyphs.qml` (generated, singleton) — `path(weight,
  name)`/`has(...)` for the vendored set. Regenerate with
  `scripts/gen-phosphor-glyphs.py`; `--check` fails if stale.
- `design-system/components/PhosphorIcon.qml` (new) — `name`, `color`
  (default `Theme.color.textPrimary`), `size`, optional `weight`
  (default `regular`), optional `accessibleName`; `source` is the qrc path
  (`qrc:/icons/phosphor/<name>[-<weight>].svg`). Renders a `ShapePath` whose
  `fillColor` binds to `color`.
- `design-system/CMakeLists.txt` — `qt_add_resources(... PREFIX
  "/icons/phosphor" BASE assets/icons/phosphor FILES <glob>)`, plus the new
  QML files in `qt_add_qml_module`/`df_qml_lint`.
- `scripts/gen-phosphor-glyphs.py`, `scripts/check-phosphor-icons.py` (new) and
  `make check-phosphor` (in `make lint`), `NOTICE`, `LICENSES/Phosphor.txt`,
  `LICENSES/README.md`, `docs/design/adr/0163-phosphor-icons.md`.
- Gallery page `Phosphor` + `phosphor_{light,dark,dark_reduced}.png` goldens;
  `tst_design_system.qml` gains four Phosphor cases (size/tint on real pixels,
  name/weight resolution, unknown name is blank not a crash, default color).

Decisions / gotchas for T-19.1b/c/d:

- **Tint mechanism is a `ShapePath`, not `MultiEffect`.** Qt 6.11's
  `MultiEffect` colorization and `Image`-over-SVG tint are GPU-only: under the
  headless **software** backend the gallery and QML tests use, MultiEffect
  rendered nothing. A `ShapePath` with a bound `fillColor` renders and tints in
  both backends. Glyph path data is generated once from the SVGs into the
  `PhosphorGlyphs` singleton; the SVG stays the shipped resource.
- **Phosphor v2 file naming:** the `fill` weight is `<name>-fill.svg`; other
  weights use a `-<weight>` suffix (upstream convention). `PhosphorIcon`
  builds `source` accordingly. The primitive's default weight is `regular`, so
  the task's `qrc:/icons/phosphor/<name>.svg` is the regular path.
- **Existence is a build gate, not a silent blank.** `make check-phosphor`
  runs the generator `--check` and a QML scan that fails on any literal glyph
  name not under `assets/icons/phosphor/`. A non-vendored weight also fails.
  The runtime component warns and renders blank for an unknown name.
- **Only the curated set is vendored** (not all 1,248 upstream icons) so the
  gate is meaningful. T-19.1b/c/d must add both weight files and re-run the
  generator when they need a name that is not in the set; the scan then
  enforces it.
- Gallery pages are enumerated in **two** places: `GalleryContent.qml`
  (`pages` + the `Loader` switch) and `scripts/check-gallery-snapshots.py`
  (`PAGES`). The Phosphor page was appended as index 26 so prior goldens do
  not shift.
- `skeleton_{light,dark}.png` goldens are shimmer-phase sensitive; the new
  page/singleton shifted `skeleton_dark.png`, which was regenerated and
  re-verified stable across two `--update` runs (`--strict` green).

## T176b — T-19.1b System Settings category style

**State: done.** System Settings now draws its category icons as
`SettingsCategoryIcon` gradient tiles — the rounded, gradient-backed container
with a top inner highlight, token drop shadow, and near-white Phosphor glyph —
in the sidebar rows and the detail-pane header hero. The only surface with this
container; bare `PhosphorIcon` stays for T-19.1c/d.

Real paths:

- `design-system/components/SettingsCategoryIcon.qml` (new) — props `source`,
  `gradientStart`, `gradientEnd`, `symbolColor` (default
  `primitive.color.neutral50`), `size`, `radius` (default
  `round(size * settingsCategory.radiusRatio)`), optional `glow`. Draws
  `Shadow level:"low"` + `Rectangle` with a vertical `Gradient` + top highlight
  + centered `PhosphorIcon` (`weight:"fill"`). Software-renderable (layered
  `Shadow` + `Rectangle.gradient` + `ShapePath`; no `MultiEffect`).
- `apps/settings/SettingsPanes.qml` — `categoryStyles` (the one mapping table,
  keyed by pane `icon`) + `categoryStyle(pane)`; 21 shipped panes mapped,
  **Trackpad intentionally falls back to `Icon.qml`**. Adding a category is one
  row; a new glyph is a file drop + `scripts/gen-phosphor-glyphs.py`.
- Adoption: `apps/settings/PaneHeader.qml` (hero, `spacing.xxxl`=48px) and
  `apps/settings/SettingsShell.qml` (passes `category` into the sidebar item);
  `design-system/components/Sidebar.qml` renders `SettingsCategoryIcon` when an
  item carries `category` (size token `sidebar.categoryIconSize`=20px), else
  `Icon`.
- Tokens: `component.settingsCategory.*` (size/radiusRatio/iconRatio/
  highlightRatio/highlightOpacity/glowOpacity) + `sidebar.categoryIconSize`;
  regenerated `Theme.qml`/`compositor/src/design_tokens.rs`.
- Gallery page 27 `SettingsCategory` (+ `PAGES` in
  `scripts/check-gallery-snapshots.py` + a gradient-pixel invariant); goldens
  `settingscategory_{light,dark,dark_reduced}.png`.
- Tests: `tst_design_system.qml` (size/tint/gradient from props; unknown glyph
  blank; default token gradient) and `tst_settings_shell.qml` (every mapped
  glyph resolves in the vendored `fill` weight + fallback; General header
  resolves the tile).
- `docs/design/adr/0164-settings-category-tile.md`, `10-design-system.md`,
  `08-settings.md`, track doc.
- Capture: `scripts/capture-t19-settings-category.sh` →
  `docs/captures/t19-settings-category-{light,dark}.png`.

Gotchas for later:

- **The design-token gate scans `design-system/components/*.qml` and the
  gallery for literal hex/`radius:`/`duration:`.** So the category hues cannot
  live in the component or the gallery; they are literal only in
  `SettingsPanes.categoryStyles` (app-side, unscanned), and the gallery demo
  uses `Theme.primitive.color.*`.
- **`check-phosphor-icons.py` regex-matches `source: "..."` inside a
  `SettingsCategoryIcon { }` block.** A ternary like
  `source: x ? x.source : ""` is misread as an empty glyph name and fails the
  gate. Resolve the glyph name on the enclosing Item (`categoryGlyph`) and bind
  `source: <that>`. The app-side table is validated by the Settings test.
- Adding a branch to `Sidebar` items requires copying the new field in the
  `entries` normalizer (it whitelists `label/icon/badge`).
- `check-desktop-names` is pre-existing red on `HEAD` (see Follow-ups); `make
  check` aborts at lint there, but every other lint/test/soak gate is green and
  `make e2e` passes.

## T176c — T-19.1c Menu-bar icon migration to Phosphor

**State: done.** Every menu-bar status mark now renders from Phosphor via
`PhosphorIcon`; no Canvas geometry remains in `StatusGlyph.qml`. `make e2e` and
the 72-test qml suite are green.

Real paths:

- `shell/menubar/StatusGlyph.qml` — rewritten. A `glyphName` switch maps each
  state to a glyph, and one literal `PhosphorIcon` per glyph is visible when
  active (`fill` weight for solid marks). Mapping: `wifi`/`wifi-secure` →
  `wifi-high`; `wifi-off`/`wifi-disabled` → `wifi-slash` (dimmed 0.4);
  `wifi-connecting` → `wifi-high` (dimmed 0.5, static); `wifi-error` →
  `wifi-x`; `bluetooth`; `volume` → `speaker-high`; `volume-muted` →
  `speaker-x`; `focus` → `moon`; `accessibility` → `person`;
  `control-center` → `sliders-horizontal`; `mission-control` → `squares-four`.
  Public API (`name`/`color`/`size`/`level`/`backgroundColor`) and the slot
  footprint are unchanged, so `StatusItem`/`WifiMenu`/`VolumeMenu`/`BatteryMenu`
  and the bar layout are untouched. The `DragonfruitLogo` is unchanged.
- **Battery is composed, not a single glyph.** `battery` = Phosphor
  `battery-empty` outline (regular) + a plain `Rectangle` level fill overlay
  inside the cell (viewBox x 40..192, y 88..168, from `battery-full`'s fill
  interior) scaled by `level`. `battery-charging` = Phosphor
  `battery-charging` (outline + bolt), no level fill (as before). See ADR
  `docs/design/adr/0165-menubar-phosphor-marks.md`.
- `shell/tests/tst_menubar.qml` — `imageHasMark`/`isBackground` helpers (the
  offscreen grab renders a transparent glyph background as an opaque colour,
  white on this backend, so alpha is always 1); state coverage over all 15
  states; `test_battery_level_fill_is_continuous` samples 60% across the cell.
- `scripts/capture-t19-menubar-phosphor.sh` →
  `docs/captures/t19-menubar-phosphor-{light,dark}.png` (status fixture, scratch
  settingsd for the scheme switch).

Gotchas for later:

- **`clip: true` on an ancestor Item does not clip a child `Shape` under the
  software scene graph** (`QT_QUICK_BACKEND=software`, what all QML tests use).
  A full `battery-full` glyph wrapped in a clipped `Item` rendered unclipped.
  The level fill is therefore a plain `Rectangle` with its own geometry.
- **`Shape.GeometryRenderer` does render holes** (`battery-empty`'s inner cell
  is transparent) — verify with RGB. Do **not** use pixel alpha to detect
  marks: `grabImage`/`grabToImage` returns an opaque image (background = white
  in the light software grab), so `pixel(x,y).a` is always 1. Compare against
  `pixel(0,0)` instead.
- **Two overlapping temporary glyphs share a window grab.** `grabImage` of two
  glyphs created by `createTemporaryObject` at the same position returns the
  same composite. Create, grab, then `destroy()` before creating the next.
- `PhosphorIcon` sets `preferredRendererType: Shape.GeometryRenderer` for
  software-renderer tinting (T-19.1a); its glyph names are validated by
  `make check-phosphor`, but a dynamic `name:` binding is skipped by
  `check-phosphor-icons.py`, so the mapping is covered by the render test.
- `check-desktop-names` remains pre-existing red on `HEAD`; `make check` aborts
  at lint there, all other gates green.

Verification run: `ctest --test-dir build` 72/72; `make e2e` green;
`./scripts/check-phosphor-icons.py`, `gen-phosphor-glyphs.py --check`,
`check-design-tokens.sh`, `check-gallery-snapshots.py --strict`,
`gen-tokens.py --check`, `i18n-extract.py --check`, `check-no-capture-grab.sh`
all green. Live stills inspected: crisp vector marks, no solid blobs, logo
unchanged.

## T176d — T-19.1d Dock Files tile and first-party app icons

**State: done.** Our two first-party apps now carry Phosphor-derived artwork:
one SVG per app that is both the shell's bundled QML resource and the file
installed into the icon theme, with the Dock mapping the desktop id to the
bundled tile. `make e2e`, the 72-test ctest suite, and every lint/visual gate
are green.

Real paths:

- `assets/icons/apps/org.dragonfruit.Files.svg` / `org.dragonfruit.Settings.svg`
  (new) — generated by `scripts/gen-app-icons.py` from the vendored Phosphor
  `folders`/`gear-fill` SVGs. Files = a flat blue rounded square + white
  `folders`; Settings = the gradient-container treatment (vertical
  `#8e8e93`→`#58585c`, top inner highlight, white `gear`). `rx=61.44`
  (= 0.24 * 256, matching `dock.icon.radiusRatio`/`settingsCategory.radiusRatio`).
  `make check-phosphor` now runs `gen-app-icons.py --check`.
- `design-system/CMakeLists.txt` — the app icons are exposed at the stable
  prefix `qrc:/icons/apps/<name>.svg` (glob over `assets/icons/apps/*.svg`).
  The design-system plugin is linked by shell/apps, so the prefix is reachable
  from the Dock, the drawer, and the QML tests.
- `shell/dock/DockGlyph.qml` — new `desktopId` property + `bundledIcon` map
  (normalize: lower-case, strip `.desktop`; `org.dragonfruit.files` →
  `qrc:/icons/apps/org.dragonfruit.Files.svg`, `org.dragonfruit.settings` →
  Settings). `iconUrl` = bundled first, else `file://` + app-index `iconPath`;
  `hasAppArtwork`/`isSvgIcon` now key off `iconUrl`, so the existing
  Image/Canvas-mask and VectorImage fallback paths are unchanged. `hasThemedIconHint`
  still means "has an app-index path" (test compatibility).
- Desktop-id plumbing: `DockEntry.qml` adds `desktopId` (from `entry`) and
  forwards it; `DockAppPicker.qml` passes `modelData.desktopId`;
  `DockOverflowPopover.qml` passes `modelData.desktopId` else `appId`.
  Pinned first-party entries always carry `desktopId` (dockmodel.cpp).
- `.desktop` names: `Icon=org.dragonfruit.Files` /
  `Icon=org.dragonfruit.Settings` (were `system-file-manager` /
  `preferences-system`). Each app's `CMakeLists.txt` installs its SVG into
  `${datadir}/icons/hicolor/scalable/apps/`.
- Tests: `shell/tests/tst_dock.qml` — `test_first_party_app_uses_bundled_tile`
  (Files center-top pixel is `#3b82f6`, i.e. the qrc SVG actually rasterized),
  `test_third_party_prefers_themed_icon_path`,
  `test_first_party_bundled_tile_wins_over_themed_path`;
  `services/app-index/src/icons.rs` —
  `shipped_first_party_app_icons_resolve_in_hicolor` (reads the shipped
  `.desktop` `Icon=` and the asset, asserts the resolver finds it).
- Docs: `docs/design/adr/0166-first-party-app-icons.md` (new),
  `docs/design/12-packaging.md` (icon payload row + rule),
  `docs/design/tracks/19-desktop-affordances.md` (T-19.1d paragraph).
- Capture: `scripts/capture-t19-app-icons.sh` →
  `docs/captures/t19-app-icons-{light,dark}.png` (1920x1200). Inspected: tile 1
  is the blue folder (Files), tile 2 the grey gradient gear (Settings), the
  Downloads stack and Trash are unchanged.

Gotchas for later (esp. T-19.2):

- **Do not match on `appId` for the bundled map — key on `desktopId`.** Two
  existing tst_dock tests (`test_glyph_prefers_a_themed_icon_path`,
  `test_placeholder_and_themed_artwork_share_the_tile`) build a glyph with
  `appId: "org.dragonfruit.Files"` and **no** `desktopId` and expect the
  placeholder; matching `appId` would flip them to bundled artwork. Overflow
  groups carry only `appId`, so the popover passes `appId` as `desktopId`.
- **`grabImage` composites over an opaque background and the masked corner and
  the white glyph can both read white.** Assert the tile *background* colour at
  a point clear of the glyph (Files: `(24, 4)` is `#3b82f6`), not the center.
- **The Settings app icon bakes the gradient into the SVG** (the theme file
  cannot reference QML): it reproduces the `SettingsCategoryIcon` geometry
  (container radius `0.24`, highlight ratio 0.45 / opacity 0.28, glyph ratio
  0.55) but is not the QML component. Keep the constants in
  `scripts/gen-app-icons.py` in sync with `SettingsPanes.categoryStyles`
  "general" if either changes.
- **`make check` remains red at `check-desktop-names`** (pre-existing, see
  Follow-ups); every other gate is green and `make e2e` passes.

## T177 — T-19.2 Applications drawer

**State: done.** The shell's launcher landed: a full-output `apps-drawer` overlay
fed by the app-index corpus, alphabetical with search and category pills, a
fixed-size tile grid, and a launch path shared with the Dock. `make e2e`,
`make soak` (100 clean cycles), the 74-test ctest suite, and every lint/visual
gate are green. Only `check-desktop-names` remains pre-existing red.

Real paths:

- `shell/src/appsdrawer.{h,cpp}` (new) — the pure list model:
  `AppsDrawerRow { desktopId, name, iconPath, categories }`,
  `appsDrawerCategoryKeys()`, `appsDrawerCategoryFor()`,
  `appsDrawerCategoriesFor()`, `appsDrawerPresentCategories()`, and
  `buildAppsDrawerList(entries, category, query)`. Drops `noDisplay`/
  non-launchable, dedupes by desktop id (first wins), maps the freedesktop
  `Categories` list, sorts by localized name with an id tiebreak, filters by
  category and query. Lives in the Wayland-free `dragonfruit-shell-dockcore`.
- **The one greppable category mapping is `kCategoryMap` in `appsdrawer.cpp`.**
  freedesktop → canonical key: Development→`developer-tools`;
  Office/Finance→`productivity`; Utility/System/Settings→`utilities`;
  AudioVideo/Audio/Video/Player→`entertainment`; Game→`games`;
  Network/Chat/InstantMessaging/Email/Telephony/VideoConference/ContactManagement→
  `social`; Graphics/AudioVideoEditing/Photography/Publishing→`creativity`;
  Education/Science/Documentation/Dictionary/News→`information`. Unknown and
  `GTK`/`Qt` categories map to nothing. `"all"` is the implicit first pill.
  Adding a pill = one `kCategoryMap` row + `appsDrawerCategoryKeys()` +
  `AppsDrawer.categoryLabels` (QML label).
- `shell/apps-drawer/AppsDrawer.qml` + `CMakeLists.txt` (new, URI
  `Dragonfruit.AppsDrawer`) — title (`squares-four` glyph + "Applications"),
  `SearchField`, a horizontally scrollable `SegmentedControl` pill row
  (default `All`), a `Flow` tile grid (fixed `tileWidth` = tileSize 64 +
  spacing.xl 24, column count follows width), themed `Image` icons with a
  Phosphor `app-window` fallback, hover/focus/selected states, and arrow/
  Return/Escape keys. Props injected by the controller: `apps`, `available`,
  `active`. Filtering is local (mirrors the helper's predicates). A capture-only
  `tileRectFor(desktopId)` returns a tile's scene rect.
- Protocol: `ShellProtocol::createAppsDrawerSurface` / `setAppsDrawerInputRegion`
  / `commitAppsDrawerImage` / `hideAppsDrawer`, namespace `"apps-drawer"`,
  `DF_SHELL_LAYER_OVERLAY`, full-output, `exclusive_zone -1`,
  `KEYBOARD_INTERACTION_ON_DEMAND`; new `appsDrawer*` signals and pointer/
  keyboard routing following the overview.
- `shell/src/shellcontroller.{h,cpp}` — the drawer scene, `show/hide/
  toggleAppsDrawer`, `refreshAppsDrawer` (reuses `m_index`/`m_appIndexClient`),
  `onAppIndexChanged` refreshes an open drawer, `DF_APPS_DRAWER_FIXTURE` capture
  seam (appends a synthetic corpus and auto-opens; logs the Files tile rect for
  the capture script), launch via `openApp` after recording the tile rect in
  `m_dockTiles` so `set_launch_origin` fires.
- Menu bar/compositor: `MenuBar.qml` gains an `applications` `StatusItem`
  (`grid-four`) raising `applicationsRequested()`; `StatusGlyph.qml` maps it;
  the compositor gains `InputAction::ShowApps` ("show-apps") bound to F4 in
  `default_system_bindings()`, routed to `toggleAppsDrawer()`.
- Tokens: `component.appsDrawer` in `tokens.json`, regenerated `Theme.qml` /
  `compositor/src/design_tokens.rs`.
- Tests: `tst_dockcore` gained 6 `appsDrawer*` headless tests (sort/tiebreak,
  drop/dedupe, mapping, category filter, present categories, query+category);
  `shell/tests/tst_appsdrawer.{cpp,qml}` (new; grid/labels, category filter,
  query filter, launch signal id, keyboard Return/Escape, absent-index row,
  unknown category ignored).
- Docs: `docs/design/adr/0167-applications-drawer.md` (new), `04-shell.md`
  (new "Applications drawer" section + picker-is-not-the-launcher note),
  track 19 doc. `translations/dragonfruit.ts` regenerated.
- Capture: `scripts/capture-t19-apps-drawer.sh` →
  `docs/captures/t19-apps-drawer-{light,dark,launch}.png` (1920x1200). The
  launch run clicked the logged Files tile and the demo log confirmed
  `Dock launched "Dragonfruit Files"`.

Gotchas for later:

- **The nested dev environment enumerates the host's `/usr/share/applications`
  through app-index**, so the live capture shows the host corpus (~60 apps) plus
  the T177 fixture apps; the grid is 8 columns at 1920x1200. A visually quiet
  capture needs a curated `XDG_DATA_DIRS`.
- **`qInfo() << desktopId` quotes a `QString` in the log**, so a capture-script
  regex over a logged desktop id must allow optional quotes
  (`\"?org\.dragonfruit\.Files\.desktop\"?`).
- **The pure helper owns category mapping; the QML owns labels.** The i18n gate
  extracts the QML labels; the C++ keys are not translatable. Keep
  `AppsDrawer.categoryLabels` and `kCategoryMap` in sync.
- The drawer's scene coordinates equal output coordinates (full-output 1:1
  surface), which is why the recorded tile rect feeds `set_launch_origin`
  unchanged.
- `make check` remains red at `check-desktop-names` (pre-existing, see
  Follow-ups); T177 added no new violations (its header comment uses
  `org.example.Calc.desktop`).
