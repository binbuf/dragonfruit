# Progress Notes

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

_(44 earlier sections omitted)_

- **T42 — T-08.1a settingsd config model and D-Bus API**: **State: done.** `services/settingsd` is no longer a stub: the desktop-settings; **`services/settingsd/src/schema.rs`** (new) — `KEYS`: 20 named keys
- **T43 — T-08.1b settingsd persistence and migrations**: **State: done.** `settingsd` now owns; **`services/settingsd/src/persist.rs`** (new) — the persisted format,
- **T44 — T-08.2a Shell migration to settingsd**: **State: done.** The shell no longer owns Dock settings: `DockSettings`/; **`shell/src/settingsclient.{h,cpp}`** (new) — `SettingsClient` (typed key
- **T45 — T-08.2b Design-system Theme binding**: **State: done.** The design-system `Theme` singleton's `dark`/`reducedMotion`; **`shell/src/themebinding.{h,cpp}`** (new) — `ThemeBinding` is the one
- **T46 — T-08.2c Compositor motion/input policy migration**: **State: done.** The compositor now consumes the settingsd motion/input policy;; **`protocols/dragonfruit-toplevel.xml`** — `df_toplevel_manager` is v5 with
- **T47 — T-08.3 Restart, resync, and key-schema documentation**: **State: done.** `settingsd` is restartable with no lost write and the shell; **`docs/settings-keys.md`** (new) — the human-facing key table (type,
- **T48 — T-09.1a Settings app shell**: **State: done.** The `apps/settings` stub is now a real shell: frameless; **`apps/settings/`** is a reusable QML module `Dragonfruit.Settings` (static
- **Follow-ups**: **T-12.1b follow-ups.** (a) The shipped units are not yet installed by a; **T-10.4a follow-ups.** (a) The files-core bridge is deliberately not
- **T49 — T-09.1b Settings live-apply plumbing**: **State: done.** The Settings app is a real settingsd consumer: a QML `Settings`; **`libs/settings-client/`** (new; `libs/CMakeLists.txt`) — the former
- **T50 — T-09.2 Appearance pane**: **State: done.** The Appearance pane is real and live: a Light/Dark/Auto; **`apps/settings/AppearancePane.qml`** (new) — `SettingsGroup`/`SettingsRow`
- **T51 — T-09.3 Wallpaper pane**: **State: done.** The Wallpaper pane is real and live: our own gradient; **Schema (since 2)** — `wallpaper.source` (s, empty = solid),
- **T52 — T-09.4 Desktop & Dock pane**: **State: done.** The Desktop & Dock pane is real and live: the `Dock` group; **`apps/settings/DesktopDockPane.qml`** (new) — `SettingsGroup` "Dock" with
- **T53 — T-09.5 Displays-basic pane**: **State: done.** The Displays-basic pane is real and live: the `Built-in; **Schema (since 3)** — `display.scale` (d, 0.5–2.0, default 1.0),
- **T54 — T-09.6a Settings menu-model publication**: **State: done.** The Settings app publishes its native menu model and the; **`apps/settings/SettingsMenu.qml`** (new; QML singleton) — single source of
- **T55 — T-09.6b Settings absence matrix and wave captures**: **State: done.** The T-09 Settings wave is signed off: the absent-provider; **`docs/design/08-settings.md`** — new "The absent-provider matrix (T-09.6b)"
- **T56 — T-10.1a files-core streaming listing and model**: **State: done.** `files-core` now exists as a headless Rust library and a; **`services/files-core/`** (new crate `dragonfruit-files-core`, workspace
- **T57 — T-10.1b files-core sorting and platform fallback**: **State: done.** `files-core` now sorts its streamed model and the; **`services/files-core/src/sort.rs`** (new module) — `SortKey`
- **T58 — T-10.2a files-core operations**: **State: done.** `files-core` gained the one operations seam: rename, new; **`services/files-core/src/ops.rs`** (new module):
- **T59 — T-10.2b Optimistic semantics and state preservation**: **State: done.** `files-core` now applies rename / new-folder / delete to the; **`services/files-core/src/optimistic.rs`** (new) — `OptimisticModel`
- **T60 — T-10.3a files-core trash**: **State: done.** `files-core` now speaks the freedesktop Trash spec and the; **`services/files-core/src/trash.rs`** (new) — the trash engine:
- **T61 — T-10.3b files-core folder watcher**: **State: done.** `files-core` now has the one change monitor: one watch per; **`services/files-core/src/watch.rs`** (new) — the watch seam and fallback:
- **T62 — T-10.4a Files window, toolbar, and sidebar**: **State: done.** The Files window, toolbar, and sidebar are real. `apps/files`; **`apps/files/FilesBridge.{h,cpp}`** — `QML_SINGLETON`, `QML_NAMED_ELEMENT(Files)`:
- **T63 — T-10.4b Files list and icon views**: **State: done.** The Files icon and list views render the `files-core` listing.; **`services/files-core/src/ffi.rs`** (new) — the C ABI: `df_files_begin`,
- **T64 — T-10.4c Files context menus, multi-select, optimistic UI**: **State: done.** Context menus, multi-select, and optimistic; **`services/files-core/src/ffi.rs`** — `FfiSession` now wraps
- **T65 — T-10.5 Files performance budgets**: **State: done.** Files meets both budgets with incremental/windowed delivery.; **`services/files-core/src/ffi.rs`** — `df_files_row` / `df_files_delta`,
- **T66 — T-10.6a Dock trash source**: **State: done.** The Dock's Trash state comes from `files-core` over the one; **`services/files-core/src/trash_source.rs`** (new) — `TrashSource`
- **T67 — T-10.6b Drop-to-trash, Empty Trash, trash://**: **State: done.** The Dock's drop and Empty Trash already routed through; **`services/files-core/src/optimistic.rs`** — `PendingKind::Empty` +
- **T68 — T-10.6c Show in Files, Downloads, and .desktop identity**: **State: done.** The Files identity is installed and the Dock's two navigation; **`apps/files/org.dragonfruit.Files.desktop`** (new) — `Exec=dragonfruit-files
- **T69 — T-10.7 Files capture and acceptance walkthrough**: **State: done.** T-10 (Files MVP) is captured at the track boundary and the; **`scripts/capture-files.sh`** (new) — `make files-capture` (new target in
- **T70 — T-11.1a Notification service core**: **State: done.** The `org.freedesktop.Notifications` service and the shell; **`services/notifications/`** (new crate `dragonfruit-notifications`,
- **T71 — T-11.1b Notification actions and Dock badge replacement**: **State: done.** Notification actions round-trip to the originating app, the; **`services/notifications/src/dbus.rs`** — `GetCapabilities` adds `actions`;
- **T72 — T-11.2a DND/Focus policy**: **State: done.** The notification service now owns a three-mode Focus/DND; **`services/notifications/src/policy.rs`** (new) — `FocusMode` (`off` /
- **T73 — T-11.2b DND/Focus menu-bar reflection and Dock failure path**: **State: done.** The menu bar reflects the notification service's Focus/DND; **`shell/src/notificationclient.{h,cpp}`** — the seam gains
- **T74 — T-11.3a Control Center panel and core tiles**: **State: done.** The Control Center panel opens (menu-bar item or; **`shell/control-center/ControlCenter.qml`** (rewritten) — the panel scene
- **T75 — T-11.3b Focus/DND, dark mode, and Control Center a11y**: **State: done.** The Control Center panel has five tiles now: Wi-Fi, Focus,; **`shell/control-center/ControlCenter.qml`** — Focus and Dark Mode tiles, a
- **T76 — T-11.4a OSD overlay**: **State: done.** A volume/brightness change presents a brief centered OSD card; **`shell/src/osdmodel.{h,cpp}`** (new, dockcore) — the pure `OsdModel`:
- **T77 — T-11.4b OSD keyboard/a11y and captures**: **State: done.** The OSD is keyboard/AT-SPI accessible and the T-11 capture; **`shell/osd/Osd.qml`** — `Accessible.role: Alert` + value-derived
- **T78 — T-12.1a Session manager and restart policy**: **State: done.** `services/session` is a real session manager: the composition; **`services/session/src/plan.rs`** (new) — `RestartPolicy` (`always` /
- **T79 — T-12.1b Session environment, systemd units, second-VT**: **State: done.** The session environment is data and reaches every child; the; **`services/session/src/env.rs`** (new) — `SessionEnvironment` (`new`,
- **T80 — T-12.2 Display-manager entry and logout teardown**: **State: done.** The session now has a display-manager `.desktop` entry, an; **`services/session/dragonfruit.desktop`** (new) — Wayland session
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
