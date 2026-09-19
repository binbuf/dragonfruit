# Progress Notes

Working notes for subsequent tasks — environment quirks, decisions made
beyond the design docs, and follow-ups discovered during implementation.
Newest entries last. Update this file whenever a task teaches something
the next task needs to know.

## Environment / toolchain (learned during T-01)

- **Qt/CMake toolchain lives at `~/.local/df-toolchain/usr`** (Qt 6.11,
  CMake 4.3, Ninja 1.13). It is NOT on `PATH` by default; the Makefile
  auto-discovers it, but pass `PATH=~/.local/df-toolchain/usr/bin:$PATH`
  (and `LD_LIBRARY_PATH=~/.local/df-toolchain/usr/lib64`) when driving
  cmake/ctest directly. `cmake` needs the LD_LIBRARY_PATH (librhash).
- **The host is Fedora 44 with a KDE Wayland session** (`wayland-0`), not
  GNOME — the nested backend works fine against it; design docs' "nested
  in GNOME" examples are host-agnostic in practice.
- **`libxkbcommon.so` (linker name) is missing** on this machine — only
  `libxkbcommon.so.0` exists (no `-devel` package, no sudo). Workaround
  in use: `RUSTFLAGS="-L ~/.local/lib"` with a `libxkbcommon.so` symlink
  pointing at the .so.0. Any `cargo build/test` that links the
  compositor needs this until the package is installed.
- rustfmt/clippy were not installed for the pinned toolchain; installed
  via `rustup component add rustfmt clippy`.

## T-01 — repo scaffolding, CI, licensing, nested workflow

**State: complete** (CI awaiting first green run — see below). All
gates verified locally: `cargo build/test/clippy -D warnings/fmt
--check`, CMake build + 8/8 qmllint ctests, `scripts/check-desktop-names.sh`,
100-cycle headless soak, 100 consecutive clean nested `dragonfruit dev
--nested` exits.

Follow-ups for later tasks:

- **T-02 must harden nested teardown.** Observed once in ~300 nested
  cycles: the compositor died silently mid-cycle and its Wayland socket
  survived (no stderr, no clean-exit print). Direct compositor cycling
  was 100/100 clean both before and after, so it is a rare race, likely
  in the winit teardown path. Actions: (1) RAII/panic-guard the socket
  cleanup instead of relying on the drop chain at the end of
  `compositor_loop` (compositor/src/main.rs), (2) reproduce under the
  real T-02 event loop, (3) consider a `Drop` guard in the dev tool too
  for the `--launch`ed children. — *Resolved in T-02: the socket is now
  owned by smithay's `ListeningSocketSource` (RAII) inside the calloop
  loop and the leak check is a tripwire; 100-cycle soak passes on the
  new loop. The dev-tool `Drop` guard for `--launch`ed children is
  still open (low priority).*
- **The skeleton compositor pump loop polls at 50 ms** — fine for T-01,
  but T-02 replaces it with an fd-driven calloop loop; don't build on
  `compositor_loop`'s structure, build on its contracts (socket print
  format, teardown verification, lockstep assert). — *Resolved in T-02:
  the pump loop is gone; the contracts (print format, teardown
  verification, lockstep assert) are preserved in `session.rs`.*
- **`cargo run` swallows signal-context detail**: when the process group
  gets SIGTERM (e.g. `timeout` or Ctrl-C under `make`), the dev tool's
  stale `SIGNALLED` flag used to make `shutdown_child` SIGKILL a
  compositor that was already exiting cleanly → silent exit 71. Fixed
  (flag cleared before teardown; final wait-status check), but keep the
  lesson: teardown code must distinguish the initiating signal from new
  signals. The Foundation phase-exit soak should include nested runs,
  not just headless, to keep exercising this.
- **CI (`.github/workflows/ci.yml`) has never run on GitHub.** It needs
  a first observed green run on a PR — including the Qt 6.11 install via
  `jurplel/install-qt-action@v4` (`version: "6.11.*"`, unverified on
  ubuntu-latest) and the rust-toolchain pin (1.98.1, matching local).
  If the Qt action can't provide 6.11, either mirror the df-toolchain
  approach in CI or lower `find_package(Qt6 6.11 ...)` — decide then,
  with a note here.
- **Naming gate exemptions**: lines carrying `df-allow-desktop-name` are
  exempt from `scripts/check-desktop-names.sh` (used by the negative
  D-Bus name test in df-ipc). Keep exemptions rare.
- **Smithay =0.7.0, calloop =0.14.4, wayland-server =0.31.10** are exact
  pins in the root Cargo.toml; upgrades are deliberate events
  (14-risks.md). df-ipc is std-only by policy (docs/licensing.md) —
  keep it that way.
- **Licensing is recorded but not human-reviewed** — the release-blocker
  signoff in 14-risks.md is still open. MIT/Apache for compositor +
  services + tools, MIT for protocol XMLs (enforced by the df-ipc test),
  LGPL-3.0-or-later for design-system, GPL-3.0-or-later for shell/apps.
  Full texts in LICENSES/; pointer LICENSE in every package dir.

## T-02 — compositor core: event loop, backends, renderer, effects

**State: partial.** Done and verified: fd-driven calloop session loop;
all three backends selected by `--backend nested|drm|headless` in one
binary (FR-1); the full standard protocol surface with an in-tree
integration test that asserts the exact advertise list (FR-7) and a CI
grep gate for capture-grab protocols (FR-8); wl_output/xdg_output
outputs with modes/scale/transform and hotplug; input routing
(pointer/keyboard/touch/gestures, click-to-focus); damage-driven
nested rendering with presentation-time feedback; teardown hardening.
`make check` passes (tests, clippy, fmt, qmllint, gates, 100-cycle
soak). Nested live-verified on the dev host's KDE session: window
opens, `wayland-info` shows the full surface, `eglgears_wayland` runs
against it, SIGTERM exits clean.

Open items (in ticket order):

- **FR-2/3/4 performance budgets unmeasured.** The damage-driven path
  exists (commit → `needs_redraw` → one render pass), and render-path
  counters (`DfState.stats`: frames_rendered / frames_skipped_no_damage /
  direct_scanouts) are in place. *Updated in the T-01…T-04 review:*
  `DfState::dump_stats` now prints them on SIGUSR1 and on clean exit
  (`session.rs`), so the idle trace and direct-scanout numbers can be
  read off a running nested/DRM session. Still open: the 60 s idle-trace
  assertion and on-hardware latency measurement (T-02 test plan).
- **DRM backend compiles but has never run** — the dev host has no
  logind seat free for a compositor. It is a condensed port of anvil's
  udev backend (smithay 0.7): LibSeatSession, udev hotplug, per-crtc
  DrmOutput on GbmGlesBackend, vblank frame scheduling with the
  throttle guard, direct scanout via FrameFlags::DEFAULT, procedural
  arrow cursor (Kind::Cursor → hardware cursor plane), multi-GPU via
  GpuManager. First real-session bring-up should happen in a VM with a
  spare GPU or via VT switch (FR-6 hotplug/GPU-loss tests as well).
- **`IdleInhibitHandler::uninhibit` / inhibitors list** keeps raw
  `WlSurface`s; aliveness-filtered on read. Fine until T-26 owns idle.
- **Session-lock filter is `|_| true`** (TODO T-26); input-method
  manager filter ditto (T-07 shell tokens).
- **VRR / night light**: not plumbed (T-16 displays pane drives them).
- **Per-surface dmabuf feedback tranches** (scanout preference) not
  built; only the global default feedback. Needed for zero-copy
  capture paths in T-27/T-28.

Decisions and gotchas (relevant to T-03+):

- **Smithay 0.7's winit backend implements `calloop::EventSource`**
  (it wraps the winit EventLoop in `Generic<_, Interest::READ>` and
  sets `ControlFlow::Poll`). We insert it as a source, so the nested
  loop is fully fd-driven — no anvil-style 1 ms polling, no client
  starvation while pumping winit. Client dispatch uses
  `Generic::new(display, Interest::READ, Mode::Level)` with an unsafe
  `get_mut()` (anvil's pattern); the display lives inside the source.
- **Teardown gotcha found the hard way**: a surviving `LoopHandle`
  clone keeps calloop's sources (and the listening socket) alive even
  after the `EventLoop` is dropped. `run_session` therefore drops
  `event_loop`, `state`, **and `loop_handle`** before the socket-leak
  check. Any future code holding a handle (timers, channels) must be
  dropped before that check too.
- **Socket lifecycle is fully RAII now** via smithay's
  `ListeningSocketSource` (bind → drop removes socket + lock). The
  T-01 "silent death, socket survived" race cannot recur through this
  path; the leak check remains as the tripwire.
- **The protocol advertise list is pinned by a test**
  (`compositor/tests/protocol_surface.rs`): exact global set from the
  ticket, plus a forbidden list (screencopy, layer-shell,
  foreign-toplevel, dmabuf-on-headless). Any protocol change must
  update that test — it *is* the CI check for the acceptance criterion.
  Note the real interface name is `ext_idle_notifier_v1` (not
  `..._notify_v1`).
- **`render_elements!` where-clauses apply to the impls, not the enum
  definition.** An enum variant whose type has trait bounds (e.g.
  `SpaceRenderElements<R, WaylandSurfaceRenderElement<R>>`) fails
  well-formedness at the enum's own definition. Keep the element enum
  generic over `E` (anvil's `OutputRenderElements<R, E>` pattern) and
  instantiate per-backend.
- **Rust method resolution needs the trait *imported*, not just named
  in a where clause** (`use smithay::backend::input::Device as _`).
  Hit with `device.has_capability`, `event.time_msec`, `Window::
  wl_surface` (via `WaylandFocus`), `WlSurface::id/is_alive` (via
  `Resource`).
- **Serials**: use `smithay::utils::SERIAL_COUNTER.next_serial()` for
  all synthetic input events. T-03 should adopt the same counter for
  shortcut-driven events so seat-serial pairing stays coherent.
- **SSD default**: `XdgDecorationHandler` negotiates ServerSide by
  default (T-13 ships SSD). Nothing draws decorations yet — T-13 owns
  the frame rendering; clients already honor the mode.
- **Window mapping** (pre-T-04): xdg toplevels configure with output
  bounds at creation, map into the `Space` on first buffer commit,
  placed center + 24 px cascade (`DfState::map_pending_windows`).
  T-04 replaces placement/stacking policy wholesale; the machinery
  (`pending_windows`, `Space<Window>`, popups) stays.
- **Dmabuf imports are lazy**: the protocol layer accepts buffers whose
  format was advertised and defers GPU import to render time (checked
  against the format list in `DmabufHandler::dmabuf_imported`); a bad
  buffer fails the render pass, not the compositor. anvil imports
  eagerly on the primary GPU — revisit if clients need early failure.
- **`ImportEgl`/`bind_wl_display` is feature-gated on `use_system_lib`**
  in smithay 0.7 — we build the rs backend, so EGL wl_drm acceleration
  is unavailable; linux-dmabuf v4 covers hardware clients.
- **Nested mode has no vblank**; rendering is demand-driven and
  presentation feedback uses `Refresh::fixed(60 Hz)`. The nested
  window is also our main FPS/timing testbed — keep an eye on the
  budget work before trusting it for FR-2/3/4 numbers.

## T-03 — input stack: keymaps, shortcuts, gestures, hot corners

**State: partial.** Done and verified: the Cmd→Super / Option→Alt
mapping fixed once in `df_ipc::keymap` and consumed by both the xkb
keymap (`compositor/src/input/keymap.rs`) and the shortcut engine;
the global shortcut engine with system bindings, focused-app
accelerator admission, conflict resolution, active-key release
swallowing, and the `GrabArbiter` (logs + refuses unsanctioned grabs);
gesture recognition feeding one shared `ProgressPipeline`
(clamp/rubber-band/velocity) for swipes and pinches; dwell-based hot
corners with a single calloop timer; the unified `InputDispatch`
outbox/audit log; the live `InputSettings` model (keyboard repeat +
gesture/hot-corner/commit tunables apply live); tablet
proximity/axis/pressure/tip/button forwarding; touch unchanged.
23 unit tests pass, including a trigger-type matrix asserting keyboard,
gesture, and hot corner produce the same `InputAction` and the same
progress curve; `cargo clippy --workspace --all-targets -D warnings`
and `cargo fmt --check` pass. In-repo keymap record:
`docs/keymap.md`.

Notes for subsequent tasks:

- **Keymap is plain-data.** Shortcut bindings store raw `u32` keysyms
  (`input::keymap::KeysymValue`), not the `xkeysym::Keysym` newtype;
  `Keysym::raw()` bridges to smithay's `KeysymHandle`. Use
  `keysym.raw_latin_sym_or_raw_current_sym()` (not `modified_sym`) for
  layout-agnostic bindings, so `Cmd+Shift+3` matches the `3` binding
  rather than `#`.
- **App accelerators are focus-scoped but unfed.** `ShortcutEngine`
  has `register_app_accelerator` / `register_portal_shortcut` and
  matches only `focused_app`, but nothing calls `set_focused_app` yet:
  T-22 (menu-broker) owns populating it from xdg `app_id`/WM_CLASS
  (T-23 identity), T-27 owns the portal path. `AppAcceleratorEvent`
  already lands in the outbox.
- **The outbox is the T-07 seam.** Every trigger writes to
  `DfState::input_dispatch` (`ShellInputEvent::{Action, Progress,
  AppAccelerator}`); `drain()` returns the events. T-07 should drain
  it into the private shell protocol rather than re-deriving events.
- **Hot-corner timers:** `DfState::hot_corner_timer` holds one pending
  `RegistrationToken`; `input::schedule_hot_corner_timer` /
  `poll_hot_corner` keep it armed. Drop/clear it before the session
  teardown leak check (same rule as T-02's LoopHandle lesson). Each
  output has its own corners — `output_bounds_at` uses the output
  under the pointer (T-14 documents multi-monitor assignment).
- **Gestures are compositor-claimed, not forwarded.** Swipe/pinch
  libinput gestures are consumed by the recognizer and are *not* sent
  to clients. Decide the unclaimed-gesture pass-through policy in
  T-05/T-11 (e.g. three-finger vertical currently unclaimed); the
  `zwp_pointer_gestures_v1` global is still advertised.
- **Tablet pipeline is code-complete but hardware-untested.** It
  registers devices/tools and forwards axes (incl. pressure) and
  tip/button. The tool only gets `proximity_in` when the tool is over a
  mapped surface; a real DRM session with a pen is the first
  validation (FR-1). `touch_location` maps tablet/touch `Raw`
  coordinates onto the first output — revisit with per-device mapping
  in T-16.
- **Progress numbers need on-device tuning.** `GestureConfig` defaults
  (300 px swipe, 0.6 pinch, 10 px deadzone) and `ProgressConfig`
  commit thresholds (0.4 progress / 0.8 velocity) are starting points;
  keep them in `InputSettings`, never hardcode (T-03 risk note).
- **Per-device pointer acceleration/scroll is stored, not applied.**
  `PointerSettings` is read/write in the model, but live libinput
  application needs the backend's device handles; leave a hook when
  the DRM backend learns its devices (T-16 input panes).
- **Screenshot / notification-center / lock / app-switcher actions
  are recorded and progress-emitted but have no consumers yet** —
  T-11/T-12/T-25/T-26/T-28 attach behavior to the same
  `InputAction`s.
- **Pointer-constraint grabs are still unimplemented** (state.rs
  handler comment promised T-03). Left as a follow-up: confine/lock
  needs a `PointerGrab` implementation; not on T-03's acceptance path.
  Note here so T-04/T-05 do not assume it works.
- **Test-plan gap:** the ticket's "integration (headless): synthetic
  libinput events drive shortcuts" is covered at unit level (engine +
  pipeline matrix), not by synthesizing libinput into the headless
  backend. The headless backend has no seat devices, so a synthetic
  input harness would be needed; consider it with T-31 hardening.

## T-04 — window model: states, focus, placement, regions

**State: partial.** Done and verified: the pure window state machine
(`compositor/src/window/state.rs`) with floating/minimized/zoomed/fullscreen,
exact restore geometry after every round-trip, and maximize→Zoom (FR-1/2);
click-to-focus with lifecycle/focus broadcasts — mapped, unmapped, focused,
unfocused, title/app_id changes, state changes — written to a bounded
`WindowDispatch` outbox (FR-4); per-output centered cascade that wraps
instead of leaving the output, and transient dialogs centered on the parent
(FR-5/6); reserved-zone-aware Zoom geometry; interactive move/resize pointer
grabs with client min/max-size clamping and output clamping (FR-10); popup
positioner constraint handling (flip/slide/resize) plus popup
keyboard/pointer grabs and parent-unfocus dismissal (FR-11); window-menu
primitives (Move to Space, Minimize, Zoom, Close). 35 unit/property tests
pass; `make check` (clippy -D warnings, fmt, qmllint, gates, 100-cycle soak)
is green.

Open items:

- **Shell-restart test (FR-9) has no subject yet.** All window state lives
  in `DfState.windows` / `Space` / `Seat`, so the compositor already
  survives a shell restart by construction, but the shell process does not
  exist until T-08. Re-run the scripted test when the shell lands.
- **Headless conformance suites are unit-level, not protocol-level.** The
  ticket asks for a headless client driving `xdg_toplevel` move/resize and
  `xdg_popup` requests (malformed positioners included). *Updated in the
  T-01…T-04 review:* `compositor/tests/window_conformance.rs` now drives
  a real `wayland-client` on the headless backend through mapping,
  maximize/unmaximize, fullscreen/unfullscreen, and a constrained popup.
  Move/resize is still unit-level: starting a pointer grab needs a seat
  button press, which the headless backend has no device to produce (see
  the T-03 synthetic-input harness gap).
- **Aspect hints are not wired.** `window::resize::apply_aspect` exists and
  is tested, but xdg-shell has no aspect hint and X11 `WM_NORMAL_HINTS`
  arrive with T-06; call it from `DfState::window_size_constraints` then.
- **Reserved zones are still zero.** `DfState.reserved_zones` is the hook;
  T-07's private protocol fills it from the shell's menu-bar/Dock anchor
  zones. Zoom currently equals the full output.
- **Fullscreen does not create a Space yet.** The state machine and geometry
  are correct, but the dedicated fullscreen Space is T-05. Until then a
  fullscreen window simply fills the output and stays in the normal stack.
- **Move-to-Space is accepted but inert** (`WindowMenuCommand::MoveToSpace`)
  until T-05 owns Spaces.
- **`window_dispatch` is not drained.** It is the T-07 seam exactly like
  `InputDispatch`; the private protocol should drain both rather than
  re-deriving events.
- **Popup grab arbitration.** Popup grabs are installed unconditionally
  because they are standard xdg-shell behavior; if T-07 wants to restrict
  them to sanctioned clients it should consult `GrabArbiter` in the `grab`
  handler.

Notes for subsequent tasks:

- **`WindowModel` is keyed by Smithay `Window`** (hash by identity) and
  hands out a compositor-stable `WindowId(u64)`; use that id in the private
  protocol so the shell never invents its own. `WindowId` ordering is
  registration order, not stacking.
- **Geometry lives in the state machine, not `Space`.** Use
  `DfState.windows.geometry(&window)` for the authoritative restore
  geometry and `space.element_location` for the live position. Interactive
  move/resize keeps them in sync via `move_window`/`resize_window`.
- **Transient relationships are in `WindowModel`** (`set_parent`, `parent`,
  `children`, `transient_tree`); `toplevel_destroyed` already closes
  children and `minimize_window`/`restore_window` walk the tree. T-05 should
  move the whole tree between Spaces together.
- **Focus is single-source.** `DfState.active_window` tracks the focused
  window; `focus_changed` also sets `ShortcutEngine::set_focused_app` from
  the window's `app_id` (the T-03 follow-up), so app accelerators now scope
  to the focused window. T-22 should populate the registrations.
- **`popups.commit(surface)` must stay in `CompositorHandler::commit`** or
  unmapped popups never join their parent's tree and stacking breaks.
- **`toplevel_app_id`/`toplevel_title` read `XdgToplevelSurfaceData`**;
  `new_toplevel` sends the initial configure with `bounds` only, and
  `map_pending_windows` captures the metadata at first commit.

## T-05 — Spaces: the compositor workspace model

**State: partial.** Done and verified: the per-output ordered Space model
(`compositor/src/workspace/`) with three Spaces per display, lockstep
switching, dedicated fullscreen Spaces with an exact origin round-trip,
window→Space assignment, app Space memory, minimized-window exclusion,
display-hotplug attach/detach migration, per-Space wallpaper data, and a
bounded workspace event outbox; keyboard/gesture triggers switch Spaces
through the existing T-03 paths; the active Space's wallpaper color drives
the nested and DRM clear passes (the shell never draws the background).
16 new unit tests cover the model (two-output lockstep, fullscreen
round-trip, hotplug matrix with zero window loss, create/remove/reorder,
app memory). `cargo clippy --workspace --all-targets -D warnings`,
`cargo fmt --check`, and the desktop-name / no-capture gates are green.

Notes for subsequent tasks:

- **Workspace truth is `DfState.workspaces`.** The model is pure — keyed
  by [`WindowId`]/[`SpaceId`] and free of Smithay types — so it is
  unit-tested without a live compositor. The shell must never keep a
  second copy: T-07 should drain `WorkspaceModel::dispatch()` (a bounded
  outbox exactly like `WindowDispatch`) into the private protocol rather
  than re-deriving events. Nothing drains it yet, so it is bounded by
  capacity (1024).
- **Space ids are per-output instances; lockstep is by index.** Each
  output owns its own `Space` objects with distinct ids. Cross-output code
  must compare *indices or names* (`space_names`), never ids. A fullscreen
  window is assigned to the owning output's Space, but every output gets an
  empty fullscreen Space at the same index so the lists stay aligned and a
  lockstep switch still works. `enter_fullscreen`/`exit_fullscreen` return
  the owner's ids.
- **Scene mapping is centralized in `DfState::apply_workspace_layout`.**
  Only visible windows whose assigned Space is active on their output are
  mapped. Any new code that maps/unmaps windows must respect
  `window_on_active_space` or a window will leak onto the wrong Space.
  `apply_window_transition`, `restore_window`, and `map_pending_windows`
  already consult it; `minimize_window` still unmaps directly (always
  correct).
- **Trigger routing.** `dispatch_input_action` applies workspace actions
  immediately after recording/progress; gesture commits bypass it and call
  `handle_workspace_action` from `input.rs::end_gesture`. A new trigger
  path (shell, portal) must go through one of those two, not call
  `WorkspaceModel` directly.
- **Fullscreen lifecycle now owns a Space.** `fullscreen_window` creates
  it (guarding against a repeated request), `unfullscreen_window` destroys
  it, and `toplevel_destroyed` destroys it when a fullscreen window dies.
  The `window_conformance` fullscreen round-trip still passes over the
  protocol.
- **Hotplug ordering.** `backend::add_output` calls `on_output_added`
  (fresh Space list). DRM `connector_disconnected`/`device_removed` call
  `on_output_removed` *before* `space.unmap_output`, so the remaining
  primary can be computed from `space.outputs()` (first output whose name
  differs). Migration re-homes the removed output's windows to that
  output's current Space; if it was the last output, assignments are kept
  so a re-attach can re-home them.
- **Wallpaper is color-only today.** `Wallpaper { color, source, fit }` is
  stored per Space and the active color is rendered via
  `wallpaper_color_for` (nested/DRM clear pass). Image sampling and the
  slide/scale during a switch need a scene-element refactor: `render_output`
  takes a single `Space<E>` type, so a real per-Space background element
  means a `SceneElement` enum wrapping `Window` + wallpaper. Consider that
  with T-11 rather than bolting on a second typed Space.
- **App memory is keyed by index, not `SpaceId`**, so it survives lockstep
  changes and fullscreen insertions. It is updated on new-window
  assignment and on "Move to Space"; persistence across compositor
  restarts is still deferred (settingsd, T-16).
- **Window placement still keys off the primary output.** New windows are
  placed on the first output and assigned there; multi-monitor placement
  (which output a window opens on) is T-11/T-16.

## Devroot: user-space native-dep sysroot (learned during T-02)

The DRM backend needs libdrm/gbm/libinput/libseat/libudev *headers and
linker names*, which this host lacks (no sudo). Recipe that works:

```bash
dnf download --destdir /tmp/rpms libdrm-devel mesa-libgbm-devel \
    libinput-devel libseat-devel systemd-devel libseat
mkdir -p ~/.local/df-devroot/lib64 ~/.local/df-devroot/lib64/pkgconfig \
         ~/.local/df-devroot/include
for r in /tmp/rpms/*x86_64.rpm; do rpm2cpio "$r" | cpio -idm -D ~/.local/df-devroot; done
# -devel RPMs ship dangling .so symlinks; point them at the host's
# runtime libs (lld refuses dangling linker-name symlinks):
cd ~/.local/df-devroot/lib64
ln -sf /usr/lib64/libgbm.so.1 libgbm.so   # ... and friends
```

The Makefile auto-exports `PKG_CONFIG_PATH`/`RUSTFLAGS` when
`~/.local/df-devroot/lib64` exists, and the compositor's `build.rs`
adds the search path + rpath (runtime libs live in the devroot).
Normal systems with the -devel packages installed are unaffected; CI
installs the apt `-dev` packages.

## Conventions established in T-01 (follow in all later tasks)

- Every source file carries an SPDX header matching its directory's
  LICENSE file.
- Constants that are contracts (desktop name, lockstep version, D-Bus
  names) live in `df-ipc` and nowhere else.
- QML lint runs as ctest (`df_qml_lint` in the root CMakeLists) — every
  new QML module registers its files there.
- `make check` is the local CI-equivalent gate; run it before every
  commit.

## T-01…T-04 review — bugs found and fixed

A conformance pass over T-01…T-04 found two real compositor bugs that the
existing unit tests could not see, plus several smaller gaps. All fixes
are covered by tests; `make check`-equivalent is green.

- **Pending windows never mapped (T-02/T-04).** `DfState::map_pending_windows`
  tested `SurfaceAttributes.current().buffer`, but
  `on_commit_buffer_handler` (called first in `commit`) *consumes* that
  buffer into the renderer surface state, so the check was always false.
  Fix: `smithay::backend::renderer::utils::with_renderer_surface_state(
  surface, |s| s.buffer().is_some())`. This means the earlier T-02
  "nested live-verified with a client" note did not actually hold on the
  committed code; the new `window_conformance` test is the guard.
- **Missing `Window::on_commit` (T-04).** Nothing refreshed the window's
  cached bounding box, so `Window::bbox()/geometry()` were `0×0`: every
  window was placed with zero size and restore geometry was lost. Fix:
  call `window.on_commit()` in `CompositorHandler::commit` for the
  pending/mapped window whose surface committed. Keep this in `commit`;
  placement reads `bbox()` right after.
- **`xdg_toplevel` pending-state desync (T-04).** `zoom_window` /
  `fullscreen_window` set the client's `Maximized`/`Fullscreen` pending
  state *before* the state machine accepted the transition, so a request
  from a minimized window left a stale state that the next configure
  would deliver. Fix: apply the transition first and only touch the
  pending state when `transition.changed`.
- **Gesture recognizer leaked state (T-03).** `GestureRecognizer::end` /
  `cancel` were never called; an unclaimed gesture (e.g. three-finger
  vertical) stayed active and could be evaluated against the next
  gesture. Fix: `input::end_gesture` resets the recognizer on every end.
- **Input outbox grew without bound (T-03).** `InputDispatch.outbox` was
  never drained until T-07; a long session with no shell attached would
  grow it forever. Fix: bounded `VecDeque` (keeps the newest tail).
- **Render-path counters were unobservable (T-02 FR-2/FR-5).**
  `DfState::dump_stats` prints `frames_rendered` /
  `frames_skipped_no_damage` / `direct_scanouts` on SIGUSR1 and on clean
  exit.
- **Dev-tool child teardown (T-01).** A `ChildGuard` now owns the
  compositor and every `--launch`ed child, so a panic or early return
  cannot leak them into the host session; normal teardown disarms it.

Notes for T-05+:

- `compositor/tests/window_conformance.rs` is the headless protocol
  harness (spawns the compositor, drives `xdg_toplevel`/`xdg_popup` with
  `wayland-client` + `wayland-protocols`; the latter is a dev-dependency).
  Move/resize is still unit-only — a pointer grab needs a seat button,
  which headless cannot produce. Reuse this harness for T-05 Space and
  T-07 protocol conformance.
- Window placement/restore geometry depends on `Window::on_commit()` being
  called on every toplevel commit; if T-05/T-07 change the commit path,
  keep that call.
- The popup configure constraint test anchors at the parent and asserts
  the configure fits the output; smithay's `PositionerState::get_unconstrained_geometry`
  does the flip/slide/resize work, our `window::popup` only floors the
  size at 1×1.

