# Progress Notes

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

- **How this plan runs**: 168 one-session tasks; execution order is [ROADMAP.md](ROADMAP.md). Task ids; 158 nested/human tasks run first; the 10 `[hw]` tasks are Phase 18 (hardware
- **Environment / toolchain**: Qt/CMake toolchain at `~/.local/df-toolchain/usr` (Qt 6.11, CMake 4.3,; Host is Fedora 44 with a KDE Wayland session (`wayland-0`); the nested backend
- **Landed foundation**: Compositor core (nested/DRM/headless, calloop, damage-driven rendering), input
- **T01 — T-01.1 Titlebar render element**: **State: done.** The SSD titlebar element exists, is sized from the generated; **`compositor/src/window/decoration.rs`** — `TitlebarElement`,
- **T02 — T-01.2 Traffic-light actions**: **State: done.** Close/minimize/zoom are clickable through the titlebar; **`compositor/src/window/decoration.rs`** — `TitlebarElement::cluster_rect()`
- **T03 — T-01.3 Titlebar drag, double-click, fullscreen reveal**: **State: done.** A floating window's titlebar drags to move (existing; **`compositor/src/window/decoration.rs`** — `TitlebarDoubleClick`
- **T04 — T-01.4 Window menu**: **State: done.** A right/Control-click on the SSD titlebar opens a; **`compositor/src/window/menu.rs`** — `WindowMenu`, `WindowMenuRow`,
- **T05 — T-01.5 Decoration tier policy and X11 correctness**: **State: done.** The three decoration classes are honored and the X11; **`compositor/src/state.rs`** — new `DfState::set_decoration_tier(window,
- **T06 — T-01.6a `make demo` harness**: **State: done.** One `make demo` target builds the tree and runs the T-01 loop; **`Makefile`** — `demo: build` → `cargo run -p dragonfruit-dev -- dev --demo
- **T07 — T-01.6b Loop integration walkthrough and capture**: **State: done.** The live nested walkthrough of the T-01 loop is scripted and; **`scripts/capture-demo.sh`** (new) + **`scripts/capture-demo-driver.py`**
- **T08 — T-02.1a Animation clock and frame discipline**: **State: done.** One shared compositor animation clock exists; the overview's; **`compositor/src/animation.rs`** (new) — `FRAME_INTERVAL` (16 ms),
- **T09 — T-02.1b Window appear transition**: **State: done.** A newly mapped window scales/fades in from its Dock tile on; **`compositor/src/window/appear.rs`** (new) — `AppearTransition`
- **T10 — T-02.2 Minimize and restore motion**: **State: done.** Minimize shrinks a window into its Dock entry's tile and; **`compositor/src/window/motion.rs`** (renamed from `appear.rs`) —
- **T11 — T-02.3 Zoom and fullscreen transitions**: **State: done.** Zoom/unzoom and fullscreen/unfullscreen animate between the; **`compositor/src/window/motion.rs`** — `WindowMotionKind::{Zoom,
- **T12 — T-02.4a Close ghost**: **State: done.** A closing window shrinks/fades out into its app's Dock tile; **`compositor/src/window/motion.rs`** — `WindowMotionKind::Close` (reverse
- **T13 — T-02.4b Close interruptibility and idle trace**: **State: done.** A live close ghost reverses mid-flight without waiting: a; **`compositor/src/state.rs`** — `close_window` clears keyboard focus when it
- **T14 — T-03.1a Nested idle trace and animation frame budget**: **State: done.** The idle/animation frame trace is now a real instrument (a; **60.0 s idle window**: `frames_rendered=1` (flat, +0),
- **T15 — T-03.1b Latency instrument and direct-scanout template**: **State: done.** The input-to-photon latency instrument is real and the; **nested latency**: n=50, min=3501 us, median=13878 us, p95=15223 us,
- **Follow-ups**: Add the QML import-dir define (`DF_QML_IMPORT_DIR`) to the first-party apps'; T-16: publish `_NET_FRAME_EXTENTS` for Tier-2 X11 windows so clients can
<!-- symphony:digest:end -->

Working notes for the plan in [ROADMAP.md](ROADMAP.md). The harness maintains the
"Key facts" digest near the top; each session appends a `## TNN` section with
environment quirks, decisions that go beyond the design docs, and follow-ups the
next task needs. Keep newest details last within a section.

The legacy (phase-based) plan's notes are archived at
[tasks/legacy/PROGRESS.md](tasks/legacy/PROGRESS.md); T-xx numbers there are
legacy numbers and do not match this plan.

## How this plan runs

- 168 one-session tasks; execution order is [ROADMAP.md](ROADMAP.md). Task ids
  `T01`–`T168` are positional; the stable design ids `T-01.1a` etc. are in each
  task file title and in [design/tracks/](design/tracks/).
- 158 nested/human tasks run first; the 10 `[hw]` tasks are Phase 18 (hardware
  rail) and need a seat, spare GPU or clean VM.
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

## Follow-ups

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
