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
- **T16 — T-04.1a Real shadows**: **State: done.** Elevation-token shadows exist on both sides of the process; **Tokens** — new `component.elevation.{low,med,high,overlay}` in
- **T17 — T-04.1b Rounded-corner clipping**: **State: done.** Rounded corners are a token-derived geometry mask on the; **`compositor/src/window/corner.rs`** (new) — `CornerMask`
- **T18 — T-04.2 Backdrop blur pass**: **State: done.** The chrome material pass exists and is token-driven, applied; **`compositor/src/window/backdrop.rs`** (new) — `MaterialRole`
- **T19 — T-04.3 Reusable scene-transform pass**: **State: done.** One reusable scene transform exists (scale/translate + optional; **`compositor/src/window/pass.rs`** (new) — `FramePass`: the single shared
- **T20 — T-04.4a Material degrade tiers and instrumentation**: **State: done.** One ordered material-quality ladder (`Full` → `Reduced` →; **`compositor/src/window/degrade.rs`** (new) — `DegradeTier` (`Full`
- **T21 — T-04.4b Light/dark, reduced motion, and sign-off package**: **State: done.** One live `ColorScheme` on `DfState` now drives every; **`compositor/src/window/decoration.rs`** — `ColorScheme::name`/`parse`
- **T22 — T-05.1a Live-surface transform into the grid**: **State: done.** Mission Control now transforms the **live** window surfaces; **`compositor/src/overview/grid.rs`** (new) — `grid_layout` (candidates
- **T23 — T-05.1b Live video at scale and degrade**: **State: done.** A committing "video" client keeps advancing at the reduced; **`compositor/src/overview/grid.rs`** — `GridMaterial { tier, shadow, blur }`
- **T24 — T-05.2 Hit-testing and selection on live representations**: **State: done.** A left click on a live Mission Control representation; **`compositor/src/overview/grid.rs`** — `GridLayout::window_at(point,
- **T25 — T-05.3 Drag a live representation between Spaces**: **State: done.** Pressing on a live Mission Control representation and dragging; **`compositor/src/overview/grid.rs`** — `GridDrag { window, start, current }`
- **T26 — T-05.4 Image wallpaper and per-Space slide**: **State: done.** A Space's wallpaper can be an image (`source` + `fit`) decoded; **`compositor/src/wallpaper.rs`** (new) — `sample_wallpaper` (pure
- **Follow-ups**: T-05.1a (done in T22): the live-surface grid landed as a render-time; T-04.4b (done in T21): the live scheme is on `DfState` (ADR 0016) and
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

## Follow-ups

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
