# Progress Notes

<!-- symphony:digest:start -->
## Key facts (maintained by symphony — do not edit)

- **How this plan runs**: 168 one-session tasks; execution order is [ROADMAP.md](ROADMAP.md). Task ids; 158 nested/human tasks run first; the 10 `[hw]` tasks are Phase 18 (hardware
- **Environment / toolchain**: Qt/CMake toolchain at `~/.local/df-toolchain/usr` (Qt 6.11, CMake 4.3,; Host is Fedora 44 with a KDE Wayland session (`wayland-0`); the nested backend
- **Landed foundation**: Compositor core (nested/DRM/headless, calloop, damage-driven rendering), input
- **T01 — T-01.1 Titlebar render element**: **State: done.** The SSD titlebar element exists, is sized from the generated; **`compositor/src/window/decoration.rs`** — `TitlebarElement`,
- **T02 — T-01.2 Traffic-light actions**: **State: done.** Close/minimize/zoom are clickable through the titlebar; **`compositor/src/window/decoration.rs`** — `TitlebarElement::cluster_rect()`
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
