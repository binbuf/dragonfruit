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
  signoff in 14-risks.md is still open. MIT repo-wide, including the
  protocol XMLs (enforced by the df-ipc test). Full text in LICENSES/;
  pointer LICENSE in every package dir; third-party attribution in
  NOTICE.

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
  libinput events drive shortcuts" was covered at unit level (engine +
  pipeline matrix), not by synthesizing libinput into the headless
  backend. *Resolved:* the headless backend now has a real synthetic
  `InputBackend` + injection socket; see the "T-03 synthetic-input
  harness" section at the end of this file. `shell_protocol_conformance::
  synthetic_input_drives_shortcuts_hot_corners_and_gestures` drives
  keyboard, hot-corner, gesture, and pointer events end to end.

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
  exist until the shell chrome lands (T-09/T-10). Re-run the scripted test
  when the shell lands.
- **Headless conformance suites are unit-level, not protocol-level.** The
  ticket asks for a headless client driving `xdg_toplevel` move/resize and
  `xdg_popup` requests (malformed positioners included). *Updated in the
  T-01…T-04 review:* `compositor/tests/window_conformance.rs` now drives
  a real `wayland-client` on the headless backend through mapping,
  maximize/unmaximize, fullscreen/unfullscreen, and a constrained popup.
  *Updated again (T-03 harness):* the same suite now drives interactive
  `xdg_toplevel.move`/`resize` over the protocol with a synthetic seat
  button, asserting the resize configure (`move_and_resize_requests_are_
  served_over_protocol`). Move has no client-visible geometry event, so it
  is asserted as "grab ran, window survived".
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

## T-06 — Xwayland integration

**State: partial.** Done and verified: Xwayland is spawned eagerly on
every backend (nested/DRM/headless) with graceful degradation when the
binary is missing; `DISPLAY` is exported to the process environment and
written to `$XDG_RUNTIME_DIR/<socket>.x11-display` for the dev tool;
Xwayland crash/disconnect respawns the server without disturbing the
session (FR-6); X11 windows are mapped through the exact same
`WindowModel`/`Space`/`WorkspaceModel` machinery as `xdg_toplevel`s
(cascade/transient placement, workspace assignment, focus, minimize/
restore, zoom, fullscreen, close, stacking); `WM_CLASS` resolves to a
`.desktop` application via an interim pure-std resolver with misses
recorded; X11 windows are marked Tier-2 SSD unless `_MOTIF_WM_HINTS` opts
out; `WM_NORMAL_HINTS` min/max **and aspect** now feed
`DfState::window_size_constraints` (the T-04 follow-up); clipboard
selection is bridged both directions through the data-device selection.
`make`-equivalent gates pass (workspace tests incl. 5 new resolver tests,
2 new conformance tests, clippy `-D warnings`, fmt, desktop-name and
no-capture gates, 10-cycle soak). Live-verified: `xmessage` under
`dragonfruit dev --nested --launch` gets `DISPLAY` and renders; a raw
`x11rb` client maps/unmaps through `_NET_CLIENT_LIST`; SIGKILLing
Xwayland respawns it (`start #2`) and the session exits clean.

Decisions made (beyond the design docs):

- **Eager start, not lazy.** True lazy start needs the launcher to know
  an app's toolkit before it runs (T-23's job). Eager makes FR-1 true for
  every launch path today and is the standard session model (Xwayland is
  up before the first X11 client). Documented in `xwayland.rs`.
- **Interim identity resolver is pure std, not GIO `AppInfo`.** The
  compositor is a thin policy layer and must not link the desktop stack
  (df-ipc is std-only by policy; the same spirit applies here). It scans
  the XDG `applications` dirs, matches `StartupWMClass` then the desktop
  id/stem, and records misses. T-23 replaces it with an `app-index` query.
  `DfState::dump_stats` prints the resolution rate + miss list (acceptance
  criterion).
- **Clipboard only.** Primary selection is not advertised (it is not in
  the pinned T-02 protocol surface), so `send_selection`/`new_selection`
  handle `SelectionTarget::Clipboard` and drop `Primary`. `wlr-data-control`
  sees the data-device selection the shell's clipboard manager consumes.
- **XDnD is a known gap.** Smithay 0.7's XWM has no XDnD translation at
  all (grep confirms). FR-5 (file drag X11↔Wayland) is therefore unmet;
  it needs a custom bridge (T-30 strange-app zoo or T-31 hardening).
- **SSD marking lives in the window model.** Added
  `window::DecorationTier` and `WindowModel::set_decorations`; T-13 reads
  it to draw the titlebar. X11 default is `ServerSide` (Tier 2); motif
  hints that request no decorations become `ClientSide`.
- **`DISPLAY` hand-off contract:** the compositor writes
  `$XDG_RUNTIME_DIR/<wayland-socket-name>.x11-display` when Xwayland is
  ready and removes it on teardown; the dev tool waits for it (≤5 s) and
  exports `DISPLAY` to `--launch`ed children. T-24's session manager
  should consume the same file.

Gotchas hit the hard way (important for T-07/T-13/T-23/T-24):

- **`client_compositor_state` must special-case the Xwayland client.**
  The Xwayland connection carries `XWaylandClientData`, not
  `DfClientState`; the old `unwrap()` panicked the moment Xwayland
  committed a surface. It now returns the Xwayland client's own
  `CompositorClientState` first.
- **Smithay 0.7's `X11Wm` leaks the calloop loop.** `X11Wm::start_wm`
  inserts an internal channel source whose closure captures a strong
  `LoopHandle`; calloop's `LoopInner` is an `Rc`, so that is a reference
  cycle. The `ListeningSocketSource` RAII unlink therefore never runs once
  Xwayland has started, and neither does `XWayland`'s own `X11Lock` drop,
  so `/tmp/.X<n>-lock` + `/tmp/.X11-unix/X<n>` would accumulate across
  sessions. `run_session` now explicitly unlinks the Wayland socket + lock
  on teardown and `xwayland::cleanup_x11_files(display_number)` unlinks the
  X11 socket/lock (and `maybe_restart` does the same before respawning).
  Our own `XWayland` source closure avoids the same trap by using
  `state.loop_handle` instead of capturing a handle. Upstream/T-31
  follow-up: patch or upstream a `WeakLoopHandle` in the X11Wm source.
- **Restart needs the flag cleared before `start`.** `start()` guards on
  `pending_restart`; `maybe_restart` must set it false before calling
  `start` or the respawn is a silent no-op (found by the restart test).
- **`std::env::set_var("DISPLAY", …)` is edition-2021-safe but is
  technically unsound with other threads running** (Rust 2024 makes it
  `unsafe`). The compositor has X11/Xwayland worker threads. It is low
  risk (they never read `DISPLAY`) and the file hand-off is the real
  contract; revisit if the workspace moves to edition 2024.
- **`WindowModel` now carries a decoration tier and X11 size hints.**
  T-13 should read `WindowModel::decorations`; `SizeConstraints` gained an
  `aspect` field and `clamp` applies it (then re-clamps to max).

Notes for subsequent tasks:

- **T-07** should drain `WindowDispatch` for X11 windows too; the X11
  path already broadcasts `Mapped`/`Unmapped`/`Focused`/`TitleChanged`/
  `AppIdChanged`/`StateChanged` through the same outbox. The private
  protocol's window list must include X11 windows (they are ordinary
  `WindowId`s).
- **T-13** draws SSD; use `window::DecorationTier` (X11 defaults to
  `ServerSide`). X11 `configure` currently gets the bare client geometry;
  once decorations have insets, `DfState::configure_window_size` is the
  single place that pushes geometry to the X server.
- **T-23** owns the real resolver. Replace `DfState::app_resolver`
  (`identity::AppResolver`) with an app-index query; the miss set
  (`AppResolver::misses`) is exactly the heuristic input, and
  `IdentitySource` records which rule matched.
- **T-24** session manager: consume
  `$XDG_RUNTIME_DIR/<socket>.x11-display` (same contract as the dev tool)
  and export `DISPLAY` to launched apps. The compositor also sets its own
  `DISPLAY`, so apps it spawns inherit it directly.
- **T-30** app matrix: run Firefox (X11), Steam, an SDL game, and xterm;
  the conformance harness (`xwayland_conformance.rs`, `x11rb` dev-dep)
  and `xmessage` are the cheap local stand-ins used here.
- **T-31** hardening: the calloop cycle above, plus a synthetic-input
  harness would let the `move_request`/`resize_request` X11 paths be
  covered over the protocol (currently unit-level like the Wayland
  move/resize path).



## T-07 — private shell protocols: trust, chrome, window/workspace/output control

**State: partial.** Done and verified: three MIT protocol XMLs
(`dragonfruit-core.xml`, `dragonfruit-shell.xml`,
`dragonfruit-toplevel.xml`) compiled into Rust server bindings with
`wayland-scanner`; the `df_core` lockstep handshake and launch-token trust
model (one-time, per-boot, role-scoped; `refused` + disconnect, logged);
`df_shell`/`df_layer_surface` chrome surfaces with anchor/margin/size/
exclusive-zone/keyboard-mode requests, `configure`/`ack_configure`, and
reserved zones folded into the Zoom area and broadcast as `df_output`
`reserved_zone` events; `df_toplevel_manager`/`df_toplevel`/`df_workspace`/
`df_output` with scene replay on bind, request round-trips acked with
`done`, and focus/attention/hot-corner/overview/app-switcher/input
broadcasts; the T-03/T-04/T-05 outboxes are drained into the protocol each
loop iteration. `cargo test -p dragonfruit-compositor` (102 unit + 4
conformance + protocol-surface + window/xwayland suites) is green,
`cargo clippy --workspace --all-targets -D warnings` and `cargo fmt --check`
pass, and the desktop-name/no-capture gates pass. The compliance client
passes the refusal matrix, chrome configure/reserved zones, and
output/workspace/toplevel request round-trips. Reference:
`docs/private-protocols.md`.

Notes for subsequent tasks:

- **The trust seam is `DfState.shell` (`ShellProtocolState`).** It owns the
  `TrustModel`, the per-client sessions, the chrome `layers`, the aggregated
  `reserved` zones, and the Mission Control/app-switcher state. The T-03
  `input_dispatch`, T-04 `window_dispatch`, and T-05
  `workspaces.dispatch()` outboxes are drained by
  `DfState::broadcast_shell_events()` in `session.rs` (one call per loop
  iteration, before `flush_clients`). New state-changing code must keep
  writing those outboxes (they are the seam), not call the protocol directly.
- **Tokens are provisioned in `shell::provision` at session start.**
  `DRAGONFRUIT_LAUNCH_TOKENS` (comma-separated hex) provisions multiple
  tokens; `DRAGONFRUIT_LAUNCH_TOKEN` is the single-token hand-off; otherwise
  one random shell token is minted. The shell's token is written 0600 to
  `$XDG_RUNTIME_DIR/<socket>.launch-token` and removed at teardown. **T-24
  must export `DRAGONFRUIT_LAUNCH_TOKEN` to the shell process and mint a
  fresh token per shell start** (a restarted shell needs a new one-time
  token). Never log a token; `LaunchToken: Debug` redacts it.
- **Wire token encoding is lowercase hex.** `df_core.authenticate` carries
  the hex string; the compositor decodes with
  `LaunchToken::parse_hex` before `TrustModel::authenticate`. Version is
  checked before the token, so a wrong-version retry does not burn it.
- **Chrome surfaces are placed, not rendered.** T-07 sends `configure` and
  computes reserved zones; T-09/T-10 own rendering, layer stacking, and the
  keyboard-mode seat grab. The `wl_surface` is stored in
  `LayerEntry.surface` for that work. `LayerSurfaceState::geometry` (pure,
  in `shell/layer.rs`) is the single placement function.
- **Reserved zones are a single global union** (`DfState.reserved_zones`),
  not per-output; `send_output_properties` reports the union to every
  output. Per-output zones are T-11/T-16.
- **The manager re-syncs from the model, not from event ids.** T-05's
  structural workspace events carry the first output's `SpaceId` for
  `Created`/`Reordered`, which is ambiguous across outputs, so
  `sync_workspaces`/`sync_outputs` reconcile the `df_workspace`/`df_output`
  object list against `WorkspaceModel`/`Space` whenever a structural event
  arrives. `WindowAssigned`/`Activated` are handled directly.
- **`WindowId`/`SpaceId` are the protocol ids.** `df_toplevel` user data is
  `ToplevelUserData { id: WindowId }`, `df_workspace` is
  `WorkspaceUserData { id: SpaceId }`, `df_output` is
  `OutputUserData { name }`. The shell never invents ids. `WindowModel`
  now tracks recency (`touch_recency`/`recency`) for the app switcher.
- **Scene-consistent ordering** comes from draining the outboxes in order
  and from the T-05 rule that workspace events are pushed in the same call
  that mutates the model. If a new broadcast is added, push it to the
  relevant outbox before the scene change, then drain in
  `broadcast_shell_events`.
- **Output requests are best-effort.** `set_mode`/`set_scale`/`set_transform`
  go through `Output::change_current_state`; `set_vrr`/`set_night_light` are
  accepted and acked but not plumbed (T-16). Every `set_*` re-sends the full
  output properties and `done`.
- **`wayland-scanner` generated server events take owned `String`s** (and
  the generated enums, not wire ints); client events take `WEnum<..>`.
  `df_toplevel_manager`'s new_id events require the client-side
  `event_created_child!` specialization (see the conformance test).
- **Qt/C++ bindings** for the shell are generated by
  `scripts/gen-shell-protocol-bindings.sh` (uses `qtwaylandscanner` when
  present, else `wayland-scanner`). The shell (T-09/T-10) consumes them; the
  shell must authenticate with its token before binding any private global.
- **Known gaps:** chrome rendering/keyboard grabs (T-09/T-10), Qt bindings
  consumption (T-09/T-10), per-output reserved zones (T-11/T-16), VRR/night
  light plumbing (T-16), toplevel thumbnails for the Dock (T-10, token-gated
  if added). The conformance suite is headless; a live shell
  restart/re-anchor integration test lands with the shell (T-09/T-10).
- **FR-7 (additive-only proof) is policy-enforced, not yet exercised.** The
  lockstep handshake refuses cross-version mixing and
  `protocol_xmls_match_lockstep_version` binds every XML to
  `LOCKSTEP_VERSION`, but no interface has a `since="2"` member yet, so there
  is nothing for an older client to tolerate. When the first v2 event/request
  is added, add a conformance case that binds version 1 and asserts the v2
  member is neither sent nor required — and remember Wayland's rule: a
  `since="N"` event must only be sent to resources bound with `version >= N`
  (the compositor's `send_*` calls must gate on `resource.version()`).
- **The compliance client does not yet assert every event pair.** It covers
  the handshake/refusals, chrome configure + reserved zones,
  output name/scale, workspace name/wallpaper/activated, manager
  output/workspace/toplevel/done, focus/state, overview, and per-window
  requests. `attention`, `hot_corner`, `input_action`, `progress`,
  `  app_accelerator`, `app_switcher`, output geometry/mode/transform, and
  workspace `removed`/`fullscreen` are emitted but not yet asserted; extend
  the client as those consumers land (T-10/T-11/T-12/T-14/T-22).

## Foundation milestone E2E (T-01…T-07) — integration pass

**State: the headless vertical slice is now covered end to end.** Added
`compositor/tests/milestone_e2e.rs` and `make e2e`. Unlike the per-ticket
conformance suites, this runs one live headless session with three clients
attached at once — a shell (authenticated, menu-bar chrome, manager), a
Wayland app, and an X11 app through Xwayland — and asserts the pieces
interoperate:

- `df_core` handshake and a menu-bar `df_layer_surface` with an exclusive
  zone that shows up as an output `reserved_zone` (T-07 FR-1/FR-4),
- manager scene replay (1 output, 3 Spaces) then *live* `df_toplevel`
  announcements for both the Wayland and the X11 window, with title/app_id
  identity (T-04/T-06/T-07 FR-2),
- Space activation and move-to-Space through the shell handles (T-05),
- zoom/unzoom/minimize/unminimize state broadcasts through the shell (T-04),
- clean teardown: exit 0, no surviving Wayland socket, launch-token file,
  `DISPLAY` file, or orphaned Xwayland process (T-01/T-06/T-07).

The X11 half is skipped (with a note) when `Xwayland` is absent, so the test
is CI-safe; a unit test (`protocol_strings_never_contain_nul`) keeps the
NUL-sanitization regression covered regardless.

### Bug found and fixed: NUL in an X11 title panicked the compositor

The E2E test aborted the compositor the moment an X11 window mapped:

```text
thread 'main' panicked at compositor/src/shell_protocol/mod.rs:48:
called `Result::unwrap()` on an `Err` value: NulError(7, [69, 50, 69, 32, 88, 49, 49, 0])
```

`[69, 50, 69, 32, 88, 49, 49, 0]` is `"E2E X11\0"`. Root cause: X11
`WM_NAME`/`WM_CLASS`/`_NET_WM_NAME` are conventionally NUL-terminated and
Smithay 0.7's `read_window_property_string` does **not** trim the trailing
NUL. The compositor stored that string in `WindowModel` and broadcast it
through a private-protocol event; `wayland-scanner`'s generated server code
builds a `CString` from the `String` and `unwrap()`s, so any interior NUL
aborts the process. A client-controlled string must never be able to kill
the compositor.

Fix, two layers:

- **Protocol boundary (the safety guarantee):** `shell::protocol_string` /
  `protocol_string_opt` strip NULs before any client-controlled string is
  serialized (`df_toplevel.title`/`app_id`, `df_toplevel_manager`
  `app_switcher`/`app_accelerator`, `df_workspace.wallpaper` source).
- **Source (keeps the model clean):** `xwayland::x11_string` strips NULs
  from X11 `title`/`instance`/`class` before they enter `WindowModel`
  (title on map and on `property_notify`; `resolve_x11_identity` for
  `WM_CLASS`).

Regression tests: the E2E test (Xwayland path) and the
`protocol_strings_never_contain_nul` unit test (always). Wayland client
strings cannot carry NUL (the wire parser truncates), so this was
X11-specific, but the boundary fix is defense-in-depth for any future
client-controlled string.

### Notes for subsequent tasks

- **`make e2e` is the fast milestone gate; `make test` includes it.** It is
  headless and needs no display, so it belongs in CI. A live nested run of
  the same slice (`dragonfruit dev --nested` + a shell/app) and a DRM run
  are still the human steps for the phase exit; the automated test cannot
  cover them because the harness has no seat.
- **Watch every generated-event `String`.** Any new private-protocol event
  that carries a client-supplied string must go through
  `protocol_string*` (or strip NULs at the source). The generated bindings
  panic rather than error, so this is a crash class, not a cosmetic bug.
- **`WindowModel` still stores the raw (possibly NUL-bearing) X11 title**
  only if a future path bypasses `x11_string`; the protocol layer is the
  backstop. If a consumer starts comparing titles (e.g. a Dock tooltip),
  prefer the sanitized value.
- **The E2E harness captures compositor stderr to a temp file and prints
  it on panic** (`std::thread::panicking()` in `Drop`), which is how the
  panic above was diagnosed after the first run. Reuse that pattern in new
  process-spawning conformance tests.

## Remaining work: T-02…T-07 (consolidated, next sessions)

The per-ticket notes above are the detailed record; this is the actionable
index of what is still open between T-02 and T-07. It is ordered so the
headless-closable work happens first, then the hardware work, then the items
blocked on later tickets. **After closing a batch, re-run `make e2e`
(`compositor/tests/milestone_e2e.rs`) and extend it where the new work adds
integration coverage — that harness is the return point before starting
T-08.**

Tags: **[host]** closable on this headless/nested dev host · **[hw]** needs
a DRM seat, VM, or real hardware · **[blocked: T-xx]** cannot close until a
later ticket lands · **[upstream]** depends on Smithay/upstream.

### T-02 — compositor core

- [ ] **[hw]** First DRM bring-up (never run on this host). Closes "all
      three backends run a windowed client end-to-end" and the Phase-1
      "nested and DRM both run the vertical slice" exit. Verify LibSeat,
      udev hotplug, per-crtc outputs, vblank scheduling, direct scanout,
      hardware cursor plane. First stop: VM with a spare GPU or VT switch.
- [x] **[host]** FR-2 idle trace: `compositor/tests/idle_trace.rs` (in
      `make e2e`) asserts `frames_rendered` stays flat across a second of
      inactivity on the headless backend. The headless render hook now
      counts redraw requests (`frames_rendered` / `frames_skipped_no_damage`)
      instead of silently clearing `needs_redraw`.
- [ ] **[hw]** FR-2 nested idle trace (60 s, zero damage/client wakeups) and
      FR-3 input-to-photon latency (nested + DRM), FR-4
      one-frame-per-animation. `DfState::dump_stats` already prints the
      counters on SIGUSR1 and clean exit; the headless idle assertion is the
      scripted template. Latency needs hardware.
- [ ] **[hw]** FR-5 direct-scanout counter verification on real hardware.
- [ ] **[hw]** FR-6 GPU removal / forced driver loss degrades gracefully
      (VM virtio-GPU).
- [ ] **[host]** FR-7 upstream smoke clients (`weston-simple-shm`,
      `eglgears_wayland`, etc.) against nested/headless where installed;
      extend the headless conformance for protocols with no upstream suite.
- [ ] **[blocked: T-16]** VRR / night-light plumbing (compositor-owned; the
      Displays pane is the consumer).
- [ ] **[blocked: T-26]** session-lock filter is still `|_| true`;
      idle-inhibit inhibitors keep raw `WlSurface`s.
- [ ] **[blocked: T-27/T-28]** per-surface dmabuf feedback tranches for
      zero-copy capture.
- [ ] **[hw]** multi-GPU import/fallback validation.

### T-03 — input stack

- [x] **[host]** Build the synthetic-input harness for the headless backend
      (seat device + injected libinput-equivalent events). Unblocks the
      ticket's own integration test ("synthetic libinput drives shortcuts")
      and **T-04's protocol-level move/resize** conformance. Highest-value
      host-closable item. *Done:* `compositor/src/input/synthetic.rs` +
      `DRAGONFRUIT_SYNTHETIC_INPUT`; the T-03 integration test and the T-04
      move/resize conformance both use it (see the section at the end of
      this file). Next T-03 host item: pointer-constraint grabs.
- [x] **[host]** Implement pointer-constraint grabs (confine/lock).
      *Done:* `compositor/src/input/constraint.rs` supplies a
      `PointerConstraintGrab` that freezes a locked pointer and clamps a
      confined one; `DfState::new_constraint` activates it. Constraints are
      gated by `GrabArbiter` (only a launch-token-sanctioned session client
      may install one), so an unsanctioned request is logged + refused.
- [x] **[host]** Malicious-client integration test for FR-5: *Done:* the
      shell authenticates and locks the pointer (frozen), while an
      unauthenticated client's lock is refused and the pointer keeps moving
      (`shell_protocol_conformance::{sanctioned_client_can_lock_the_pointer,
      unsanctioned_pointer_constraint_is_refused}`).
- [ ] **[host]** Decide and document the unclaimed-gesture pass-through
      policy (three-finger vertical is currently swallowed).
- [ ] **[hw]** FR-1 multitouch + pen pressure on DRM; one non-US layout.
- [ ] **[hw]** On-device gesture-threshold tuning; device quirk table.
- [ ] **[blocked: T-11/T-12/T-28]** attach behavior to the Mission
      Control / app-switcher / screenshot `InputAction`s (the events are
      already emitted).
- [ ] **[blocked: T-16]** live per-device pointer acceleration/scroll
      application (stored, not applied; needs backend device handles).

### T-04 — window model

- [x] **[host]** Protocol-level move/resize conformance (needs T-03's
      synthetic seat button). *Done:* `window_conformance.rs`
      `move_and_resize_requests_are_served_over_protocol` drives
      `xdg_toplevel.move`/`resize` with an injected button press and
      asserts the resize configure. Move has no xdg-shell geometry event,
      so it is a grab-ran/survived assertion (see the harness notes).
- [x] **[host]** Malformed-client suite: `window_conformance.rs`
      `malformed_client_requests_never_crash` drives no-op transitions on a
      floating window, contradictory min>max size hints, re-entrant
      fullscreen/maximize, and a fullscreen-toplevel destroy, then proves a
      fresh window still configures.
- [x] **[host]** FR-7 input-region passthrough: confirmed, no compositor
      change needed. Smithay's `Space::element_under`
      (`desktop/space/mod.rs:194`) filters top-to-bottom by
      `is_in_input_region`, and `Window::is_in_input_region` delegates to
      `Window::surface_under` (input regions honored). The compositor's
      click-to-focus and `surface_under` both go through that path, so an
      empty input region passes the click through. *Test added:* with the
      synthetic seat, `shell_protocol_conformance::
      empty_input_region_passes_clicks_through` maps two overlapping
      windows and proves the click reaches the one underneath once the top
      window's input region is cleared.
- [ ] **[blocked: T-09/T-10]** FR-9 shell-restart scripted test (no shell
      process yet; state is compositor-owned by construction).
- [ ] **[blocked: T-13]** FR-8 translucent-region blur pass; nested UI
      test with the design-system gallery (the gallery now exists).
- [ ] **[blocked: T-13]** decoration insets through
      `DfState::configure_window_size` (X11 aspect hints already wired).

### T-05 — Spaces

- [ ] **[hw]** Two-output lockstep switch and full hotplug matrix at
      runtime on DRM (model is unit-tested; protocol/DRM path is not).
- [ ] **[host]** FR-4 image wallpaper rendering (`source`/`fit` are stored
      but never sampled) and the slide/scale scene mechanics. Needs the
      `SceneElement` enum refactor noted above; the *polish* is T-11.
- [ ] **[blocked: T-09/T-10]** integration assertion that the shell holds no
      shadow workspace state (single owner of truth).
- [ ] **[blocked: T-11/T-16]** multi-monitor window placement (still keys
      off the primary output).
- [ ] **[blocked: T-16]** persist app Space memory (session-scope only
      today).

### T-06 — Xwayland

- [ ] **[host]** FR-4 clipboard image + file-list round-trip test. The
      selection bridge is mime-agnostic; add a conformance case that offers
      `image/png` and `text/uri-list` and reads them back both directions.
- [x] **[host]** Decide and document the Xwayland fractional-scaling policy:
      **integer-scaled Xwayland + per-surface viewport downscale**, written
      up in [docs/xwayland-scaling.md](../docs/xwayland-scaling.md). Xwayland
      runs at `ceil(output_scale)`; the compositor downscales each X11
      toplevel with `wp_viewport` to the fractional logical size. Plumbing
      is deferred to T-13/T-31; X11 currently renders at 1x.
- [ ] **[host]** Malformed X-message robustness test (never crash).
- [ ] **[hw]** Reference app matrix (Firefox X11, Steam, one SDL game,
      xterm) under nested/DRM. First T-30 job; partially runnable nested if
      the apps are installed.
- [ ] **[upstream]** FR-5 XDnD: Smithay 0.7's XWM has no XDnD translation,
      so file drag across the boundary is unmet. Needs a bridge
      (T-30/T-31).
- [ ] **[upstream]** X11Wm calloop `Rc` cycle: we unlink sockets/locks
      manually; upstream a `WeakLoopHandle` fix.
- [ ] **[blocked: T-13]** X11 Tier-2 SSD titlebar rendering (tier marking
      done).

### T-07 — private shell protocols

- [x] **[host]** Extend the compliance client to assert emitted event pairs:
      output geometry/mode/transform, workspace index/activated/fullscreen/
      removed, focus, attention (`xdg-activation`), app-switcher state, and
      toplevel output/workspace/closed transitions are now asserted by
      `event_coverage_conformance`. *Updated with the T-03 harness:*
      `input_action`, `progress`, and `hot_corner` are now asserted by
      `synthetic_input_drives_shortcuts_hot_corners_and_gestures`. Only
      `app_accelerator` remains — it needs a focused app with a registered
      accelerator (T-22 owns registration).
- [ ] **[host]** FR-7 additive-only proof test. Introduce the first
      `since="2"` member (or a test-only interface), bind version 1, and
      assert the v2 member is neither sent nor required; gate every
      `send_*` on `resource.version()`.
- [x] **[host]** Fuzzed/malformed sequence suite: `shell_protocol_conformance.rs`
      `malformed_private_traffic_never_crashes` re-authenticates (refusal
      code 4), sends extreme output values and out-of-range reorders,
      drives stale workspace/toplevel handles, then proves a fresh request
      round-trips. It caught and fixed a real compositor panic: a
      client-supplied `set_mode` size built a negative Smithay `Size`; the
      output dispatch now rejects non-positive/overflowing modes and
      non-finite/non-positive scales.
- [ ] **[blocked: T-09/T-10]** FR-1 keyboard-interaction seat grabs and
      chrome rendering/layer stacking (T-07 owns placement/configure only).
- [ ] **[blocked: T-09/T-10]** consume the generated Qt/C++ bindings and add
      the live shell restart/re-anchor integration test (T-08 built the QML
      components; the shell that binds the protocol is T-09/T-10).
- [ ] **[blocked: T-11/T-16]** per-output reserved zones (single global
      union today).
- [ ] **[blocked: T-16]** plumb VRR/night-light output requests (accepted
      and acked but inert).
- [ ] **[blocked: T-10]** toplevel-thumbnail decision (token-gated if
      added).

### The E2E return point and suggested order

1. ~~T-03 synthetic-input harness → T-04 move/resize conformance.~~ **Done**
   (see the section at the end of this file); it also closed three of the
   four T-07 input-driven events. The input-region click-through test can
   now be written on top of it.
2. ~~T-03 pointer constraints + malicious-grab test.~~ **Done** (see the
   pointer-constraint section at the end of this file); the T-04
   input-region click-through test landed with it. Next host-closable item:
   T-06 clipboard image/file round-trip + malformed X.
3. ~~T-04 input-region + malformed-client suite.~~ **Done** (input-region
   confirmed; malformed-client suite added).
4. ~~T-07 compliance-client completeness + additive-only proof + malformed
   suite.~~ **Mostly done**: event coverage + malformed suite + the three
   input-driven events landed; the additive-only proof and
   `app_accelerator` (needs T-22) remain.
5. T-06 clipboard image/file test + ~~fractional-scale decision~~ (done) +
   malformed X.
6. ~~T-02 idle-trace assertion (nested/headless).~~ **Headless done**; the
   nested trace still needs a display.
7. **Re-run and extend `make e2e`.** New coverage that belongs in
   `milestone_e2e.rs` once the above lands: move/resize through the shell,
   per-output zones, a v1/v2 additive handshake, an image/file clipboard
   round-trip. Keep it the integration gate before the shell (T-09/T-10).
8. Then the **[hw]** items (DRM bring-up, budgets, hotplug, GPU loss) when a
   seat is available, and the **[blocked]** items as their tickets land.

## Foundation hardening pass (T-02…T-07) — host-closable test batch

**State: landed.** A batch of the host-closable items from the index above,
all green under `make e2e` / `make test` / `make lint`:

- **T-02 FR-2 idle trace.** `compositor/src/backend/headless.rs` now counts
  redraw requests (`frames_rendered` when `needs_redraw`, else
  `frames_skipped_no_damage`) instead of clearing the flag silently, and
  `compositor/tests/idle_trace.rs` samples the counters via SIGUSR1 twice
  around a one-second idle window and asserts `frames_rendered` is flat.
  Added to `make e2e`.
- **T-07 compliance-client completeness.** `event_coverage_conformance`
  asserts output geometry/mode/transform, workspace
  index/activated/fullscreen/removed, focus, attention (via
  `xdg-activation`), app-switcher state, and toplevel
  output/workspace/closed transitions.
- **T-07 malformed-traffic suite.** `malformed_private_traffic_never_crashes`
  re-authenticates, sends extreme output values and out-of-range reorders,
  drives stale workspace/toplevel handles, and proves a fresh request
  round-trips.
- **T-04 malformed-client suite.**
  `malformed_client_requests_never_crash` drives contradictory state
  requests, contradictory min>max size hints, re-entrant
  fullscreen/maximize, and a fullscreen-toplevel destroy.
- **T-06 fractional-scaling policy** documented in
  [docs/xwayland-scaling.md](../docs/xwayland-scaling.md).
- **T-04 input-region passthrough** confirmed against Smithay (no code
  change).

### Bug found and fixed: a client-supplied mode size panicked the compositor

The malformed-traffic suite killed the compositor with:

```text
thread 'main' panicked at smithay-0.7.0/src/utils/geometry.rs:714:9:
Attempting to create a `Size` of negative size: (-1, -1)
```

Root cause: `df_output.set_mode(width, height, refresh)` is
client-controlled `u32`; the handler did `(width as i32, height as i32)`,
so `u32::MAX` became `-1` and `Size::from((-1, -1))` panics inside
Smithay. Fix (`compositor/src/shell/mod.rs`, `SetMode`/`SetScale`): reject
modes with zero/overflowing dimensions and scales that are non-finite or
non-positive, log the refusal, and still ack with the live properties. A
malformed private-protocol message can no longer abort the session.

### Notes for subsequent tasks

- **`wait_for` in `shell_protocol_conformance.rs` is `#[track_caller]`**, so
  a timeout panic points at the waiting call site rather than the helper.
  Keep that when adding waits.
- **`CompositorProcess` in `shell_protocol_conformance.rs` captures
  stderr to a temp file and prints it on panic** (the milestone pattern);
  that is how the mode-size panic above was diagnosed. Reuse it.
- **`window_conformance.rs::shm_buffer` uses an atomic sequence suffix.**
  A test may map several windows; the backing file is `create_new`, so a
  name keyed only on the `wl_shm` pointer collided on the second map.
- **Any client-controlled size/value that reaches a Smithay constructor
  must be validated at the protocol boundary.** `Size::new` panics on
  negative dimensions; `set_mode`, `set_scale`, and future output/geometry
  requests need the same guard. This is the same crash class as the X11
  NUL-title panic (client data must never abort the compositor).
- **The T-07 input compliance events (`input_action`, `progress`,
  `hot_corner`, `app_accelerator`) are emitted from the T-03 outbox and
  need injected input.** *Updated:* the T-03 synthetic-seat harness now
  exists and asserts the first three; only `app_accelerator` remains
  (needs a focused app with a registered accelerator, T-22).
- **The FR-7 additive-only proof still needs a real `since="2"` member.**
  Do not add a fake one to production; add the conformance case in the
  same change that introduces the first v2 member, and gate every
  `send_*` on `resource.version()`.
- **`docs/xwayland-scaling.md` records the chosen policy but not the
  plumbing.** The viewport downscale and the multi-monitor "largest scale"
  decision are T-13/T-31 work.

## T-03 — synthetic-input harness (headless seat) + T-04 move/resize

**State: complete.** The headless backend now has a real
`InputBackend` (`compositor/src/input/synthetic.rs`) whose events are
parsed from a line protocol delivered over a `UnixDatagram` socket. It is
opt-in and headless-only: the socket is bound only when
`DRAGONFRUIT_SYNTHETIC_INPUT` names a path, and it is removed at teardown.
Events are libinput-equivalent — evdev keycodes get the xkb +8,
absolute coordinates are device-normalized `0..=1`, relative motion is
logical pixels — and flow through the one `process_input_event` router,
so a synthetic shortcut/gesture/hot corner exercises the same code path
as a real device. A datagram may batch newline-separated commands; a
malformed line is logged and skipped, never fatal.

New integration coverage (both in `make e2e`):

- `shell_protocol_conformance::synthetic_input_drives_shortcuts_hot_corners_and_gestures`
  drives Ctrl+Right (`workspace-next`), a top-left hot-corner dwell
  (`mission-control`), and a four-finger vertical swipe
  (`mission-control` + progress events), asserting the private-protocol
  `input_action`/`hot_corner`/`progress` events and the real Space switch;
  it also proves a synthetic pointer click focuses a mapped window.
- `window_conformance::move_and_resize_requests_are_served_over_protocol`
  uses the harness for the seat button that starts the pointer grab and
  asserts an interactive `xdg_toplevel.resize` produces a configure with
  the dragged size.

Notes for subsequent tasks:

- **Wire format is documented in the module header.** Verbs: `key`,
  `motion`, `motion-abs`, `button`, `axis`, `swipe-*`, `pinch-*`,
  `touch-*`. `motion-abs` and touch coordinates are normalized like
  libinput, not logical pixels. Send a command per datagram, or batch
  lines.
- **The harness is the headless seat now.** Use it for any protocol test
  that needs a key/button/gesture. `motion-abs 0.5 0.5` hits the center
  of the first output; the first mapped window is centered there too.
- **Only `app_accelerator` remains uncovered.** It needs a focused app
  with a registered accelerator (T-22 owns registration): when T-22 lands,
  register an accelerator for a mapped window's `app_id`, focus it, inject
  the chord, and assert the event.
- **Move has no client-visible geometry event** in xdg-shell, so the T-04
  move half only asserts the grab ran and the window survived (resize is
  the strong assertion). Tighten it if a future ticket adds a geometry
  query.
- **Pointer-constraint grabs are the next T-03 host item.**
  `PointerConstraintsHandler::new_constraint` is still a no-op and the
  protocol is advertised; the synthetic harness can now drive a
  lock/confine client. The malicious-grab test (real client + audit log)
  is the other.
- **`DRAGONFRUIT_SYNTHETIC_INPUT` must never be set in a real session.**
  Only the headless backend reads it; nested/DRM ignore it. It is a local
  test channel, not an input API.
- **Host env for direct cargo test:** `PKG_CONFIG_PATH=~/.local/df-devroot/
  lib64/pkgconfig` and `RUSTFLAGS="-L ~/.local/df-devroot/lib64
  -L ~/.local/lib"`; the Makefile sets both. `make check`-equivalent
  (fmt, clippy, qmllint, desktop-name/no-capture gates, `cargo test`,
  `make e2e`, 100-cycle soak) is green.

## T-03 — pointer-constraint grabs + FR-5 malicious-grab test

**State: complete.** `zwp_pointer_constraints_v1` was advertised but
unenforced. `compositor/src/input/constraint.rs` now supplies a
`PointerConstraintGrab`: a locked pointer is frozen at its lock location,
a confined one is clamped to the region's global bounding box, and the
grab self-heals (if the client destroys the constraint, the next event
unsets the grab instead of leaving the pointer stuck). `DfState::
new_constraint` activates the constraint and installs the grab when the
pointer is over the constrained surface.

Grabs are gated by `GrabArbiter` (now generic over the client key so the
live compositor can key on Smithay's `ClientId`): the compositor is the
sole arbiter, and only a session client that authenticated with a launch
token (`shell_protocol_conformance` shell) is sanctioned. `df_core`
authentication success now calls `grab_arbiter.sanction(client.id())`.

Tests (all in `make e2e`):

- `shell_protocol_conformance::sanctioned_client_can_lock_the_pointer` —
  the shell locks the pointer; injected motion leaves the client's
  `wl_pointer.motion` at the lock point.
- `shell_protocol_conformance::unsanctioned_pointer_constraint_is_refused`
  — an unauthenticated client requests the same lock; no `locked` event
  arrives and the pointer keeps moving (the refusal is logged).
- `shell_protocol_conformance::empty_input_region_passes_clicks_through`
  — two cascaded windows; after the top window's input region is cleared,
  the click focuses the window underneath (T-04 FR-7).

Notes for subsequent tasks:

- **Pointer constraints are sanctioned-only by design.** Third-party
  pointer lock would need a sanctioned mechanism (e.g. the portal, T-27)
  before it can be granted; the arbiter is the single gate. Do not bypass
  it in `new_constraint`.
- **Activation is focus-time only.** A constraint created while the
  pointer is not over its surface stays registered but inactive; we do not
  yet activate on a later pointer-enter. Hook the pointer focus path if a
  client needs lock-before-enter.
- **`cursor_position_hint` is ignored** (headless has no cursor to place).
  A cursor-rendering compositor should move the lock point there.
- **First `wl_pointer.motion` after an enter is not sent** (smithay sends
  `enter` only on focus change); tests that need a motion baseline must
  inject a follow-up relative motion. This bit the first version of the
  constraint tests.
- **`shell_protocol_conformance::shm_buffer` now uses an atomic sequence
  suffix** (a test maps two windows; `create_new` collided on the second).
  Same fix as `window_conformance`.
- **Remaining host-closable items:** T-06 clipboard image/`text-uri-list`
  round-trip and malformed X-message robustness; T-03 unclaimed-gesture
  pass-through policy; extend `milestone_e2e.rs` with the new input/move
  coverage.

## T-08 — design system (complete)

**State: done.** The Phase-2 art-direction checkpoint cleared, so the
remaining 13 components were built from the token groups that were already
defined, wired into the gallery, and covered by keyboard/AT-SPI/pixel tests.
All twenty library components plus the supporting `Button`/`FocusRing`/
`Shadow` now exist, and the visual regression covers 22 pages × 3 variants
(66 goldens). Two acceptance items are deliberately deferred and do not
block the ticket: the live `atspi` role dump (session-bus dependent; the
keyboard walkthrough and per-type roles are asserted in `tst_design_system`)
and the app-level "no hand-rolled chrome" lint (T-16/T-18, once the real apps
exist).

### What exists now

- `design-system/tokens/tokens.json` is the single source of visual truth
  (primitive -> semantic -> component + motion + material), with component
  token groups for **all 20** design-doc components. A supporting `button`
  group was added for the shared `Button`.
- `scripts/gen-tokens.py` generates `design-system/Theme.qml` (QML
  singleton) and `compositor/src/design_tokens.rs` (Rust). `make
  check-tokens` / CI fails if either is stale. The generator output is
  rustfmt-stable on purpose (`cargo fmt --check` stays green).
- Components under `design-system/components/` — all twenty from the design
  doc plus supporting ones: `AppWindow`, `TitleBar`, `TrafficLights`,
  `Sidebar`, `Toolbar`, `SplitView`, `SettingsRow`, `SettingsGroup`,
  `Toggle`, `SegmentedControl`, `Popup`, `ContextMenu`, `MenuBarMenu`,
  `SearchField`, `SourceList`, `Icon`, `Dialog`, `Sheet`, `Popover`,
  `ScrollView`, plus `Button`, `FocusRing`, `Shadow`. Each has dark/light via
  `Theme.color`, reduced-motion via `Theme.motion.*.duration` (collapses to
  0), keyboard operation, and `Accessible` roles.
- `design-system/gallery/` (`Dragonfruit.Gallery` module) renders every
  component x state x scheme x motion (22 pages x 3 variants) and has a
  headless snapshot driver (`DRAGONFRUIT_GALLERY_SNAPSHOT`). The gallery
  window header is a `Flow`, so adding pages does not overflow.
- `design-system/tests/tst_design_system.qml` (Qt Quick Test) covers tokens,
  reduced motion, keyboard activation, AT-SPI roles, and pixel sampling for
  every interactive component; `test_fr3_titlebar_matches_ssd_reference`
  diffs the app `TitleBar` against an independent SSD renderer
  (`SsdTitlebarReference.qml`) with `QImage.equals` and proves both are
  non-blank.
- `scripts/check-gallery-snapshots.py` runs the gallery offscreen and checks
  deterministic token/pixel invariants; `--update` writes the art-direction
  goldens to `design-system/gallery/snapshots/` (66 files), `--strict` diffs
  them.
- `scripts/check-design-tokens.sh` forbids literal colors, durations, and
  radii in component QML (the generated `Theme.qml` is exempt).

### Gotchas learned (important for the next session and for T-09+)

- **QML property names may not match `on[A-Z].*`.** They are parsed as
  signal handlers. The semantic "on" colors were renamed: `onSurface` ->
  `textPrimary`, `onSurfaceSecondary` -> `textSecondary`,
  `onSurfaceTertiary` -> `textTertiary`, `onAccent` -> `accentContent`.
  Keep this in mind when adding token roles.
- **`pragma Singleton` is not auto-detected by this Qt build system.**
  `qt_add_qml_module` only marks a QML file as a singleton if
  `set_source_files_properties(<file> PROPERTIES QT_QML_SINGLETON_TYPE
  TRUE)` is set before the call. Without it the qmldir lists `Theme` as a
  normal type and every `Theme.*` access is `undefined` at runtime.
- **Runtime-compiled QML needs a filesystem import root.** Static modules
  are registered as resources, which a plain `TestCase` loaded from disk
  cannot import. The top-level CMake sets
  `QT_QML_OUTPUT_DIRECTORY=${CMAKE_BINARY_DIR}/qml`; ctest sets
  `QML2_IMPORT_PATH` to it and the gallery app calls `engine.addImportPath`
  with a compile definition. `df_qml_lint` also passes `-I build/qml`.
- **Qt shader effects no-op on the software scene graph.** `MultiEffect`
  and `RectangularShadow` silently render nothing with
  `QT_QUICK_BACKEND=software` (the CI/headless path). `Shadow.qml` is a
  layered rounded-rectangle approximation instead. T-13 owns real blurred
  shadows in the compositor.
- **`grabImage` needs a real window.** The QML test root must be a visual
  `Item` (the QuickTest window content) that *contains* the `TestCase`;
  with `TestCase` as the root and `Item` children, `grabImage` returns blank
  pixels and image-equality tests pass vacuously. `stage` is the root.
- **`Accessible` has no `enabled` property.** Disabled state comes from
  `Item.enabled`; setting `Accessible.enabled` is a compile error.
- **Traffic-light glyphs are drawn geometry, not assets.** `Icon.qml` builds
  the X / minus / plus / check / chevron marks from rotated rectangles, so
  there are no bitmap assets to license (14-risks.md).
- **The AppWindow titlebar is top-rounded only** via two rectangles
  composited inside one `Item` whose `opacity` applies to the group (avoids
  stacked translucency).
- **Repeater delegate required properties must be declared on the delegate
  root, not re-declared in the instantiation body.** Writing
  `delegate: Segment { required property var modelData; required property int
  index }` when `Segment` already declares them makes the Repeater fail with
  "Required property modelData was not initialized" and the delegates are
  never created (the positioner then reports zero implicit size). Use
  `delegate: Segment { }` and declare the required properties once in the
  component.
- **The failure above is invisible in the visual gate.** The gallery
  snapshot driver ignores stderr on a zero exit, so a page can render a
  zero-width sliver and still "pass". When a component looks empty, run the
  gallery binary by hand and read stderr:
  `QT_QPA_PLATFORM=offscreen QT_QUICK_BACKEND=software
  DRAGONFRUIT_GALLERY_SNAPSHOT=/tmp/snaps
  build/design-system/gallery/dragonfruit-gallery-app`.
- **Repeater delegates do contribute to a positioner's implicit size** (the
  `Row`/`Column` reports the summed child widths) once the delegates are
  actually created — the zero-width case was failed instantiation, not a
  positioner limitation.
- **Flattened models must carry every field the row reads.** The Sidebar
  header entries originally omitted `icon`/`badge`, so the row's
  `.length` access threw `Cannot read property 'length' of undefined`. Give
  every entry all keys (`icon: ""`, `badge: ""`).
- **Expansion state has to override the model in both directions.** A
  `collapsed` set cannot expand an item whose model seed is
  `expanded: false`; `SourceList` stores an explicit per-index `expansion`
  override (a copied plain object so the `visibleEntries` binding re-fires).
- **`ScrollView` parents content through `flick.contentItem.data`.** The
  default property aliases the Flickable's content item, so consumers'
  children are laid out in content coordinates and `contentHeight` tracks
  `contentItem.childrenRect.height`.
- **The design-doc list has no generic `Button`.** One was added as a
  supporting component (like `Icon`/`FocusRing`/`Shadow`) so Toolbar,
  Dialog, Sheet, Popover, and SettingsRow do not each roll their own. Its
  tokens live in the new `component.button` group.
- **`Dialog`/`Sheet` size to their parent** (`width: parent ? parent.width :
  ...`), so place them in the window content (or a fixed-size host in the
  gallery). `Sheet` slides from the top; `Dialog` is centred.
- **`Toolbar`'s optional title is centred and can sit under leading
  content.** Prefer a title-only toolbar or keep the leading slot short.

### Hand-off / open items

- **The components are the hand-off.** T-09/T-10/T-11/T-12/T-13/T-16/T-18/
  T-21/T-22/T-25/T-26/T-28/T-29 build *from* these components, not around
  them. No new component should introduce a literal value:
  `make check-design-tokens` enforces it. Run
  `make gallery-snapshot` (or `dragonfruit-gallery-app`) to review the
  visual direction; the committed goldens under
  `design-system/gallery/snapshots/` are the reference.
- **Live AT-SPI dump is deferred** (needs a session bus). The component
  `Accessible.role`/`name`/`checked`/`checkable`/`selected`/`searchEdit`
  values are asserted in `tst_design_system`; a real `atspi` walkthrough
  belongs with the a11y polish (T-31).
- **Blur/translucency joint tuning** with T-02/T-13 is still open: the
  `material.*` tokens (opacity/blur) are defined and consumed by the QML
  chrome, but the compositor blur pass that must match them is T-13.
- **`Theme.dark`/`Theme.reducedMotion` are writable** and currently default
  to `Application.styleHints.colorScheme` / `false`. The shell should bind
  `reducedMotion` to settingsd (T-15/T-16) and may bind `dark` to the host
  appearance. Assigning them (as the gallery/tests do) breaks the binding by
  design.
- **App-level "no hand-rolled chrome" lint** is T-16/T-18; the
  design-system-side literal gate (`check-design-tokens.sh`) is the part
  that exists now.

## Session & host-DE expectations (hand-off for T-09/T-24/T-32)

Recorded after an end-to-end gallery review on a KDE Plasma host. These are
the behaviors the desktop must have when the shell (T-09/T-10) and session
(T-24)/packaging (T-32) land, so we do not accidentally build a
"take over the running DE" mechanism that Wayland cannot support.

### Theme is ours; only the scheme may come from the host

- The accent/highlight pink is **our token** (`Theme.color.accent` =
  `primitive.color.magenta600`/`magenta400`), not the KDE/Fedora accent
  color. No host accent is read anywhere.
- The only host-derived appearance value is the light/dark scheme:
  `Theme.dark` defaults to `Application.styleHints.colorScheme === Qt.Dark`.
  On Plasma that follows the Plasma color scheme. The gallery/tests
  override it.
- **Do not add host-accent adoption silently.** If we ever want it, make it
  an explicit settingsd option (T-15/T-16), because the visual identity is
  deliberately ours (`.docs/design/14-risks.md`).

### There is no live compositor handoff — never attempt one

Wayland clients are bound to one compositor; there is no standardized way
to migrate live windows. `.docs/design/14-risks.md` ("No live compositor
handoff") and `.docs/design/11-session-and-dev-workflow.md` are the
authority. The two supported paths, and the expected behavior of each:

1. **Nested on the existing desktop (daily dev, e.g. KDE Plasma).**
   `make dev` / `dragonfruit dev --nested` opens Dragonfruit as a normal
   window *inside* Plasma on its own private Wayland socket. Plasma is
   never disturbed. On exit the dev tool's `ChildGuard` tears down the
   compositor and every `--launch`ed client and fails loudly on a stray
   process/socket (`make soak` is the 100-cycle scripted form).
   - Expected UX: the host DE stays up the whole time; there is nothing to
     "restore".
2. **Separate login session alongside the host DE (real DRM/hardware).**
   Register Dragonfruit as a session with the display manager (SDDM/GDM)
   *in addition to* Plasma. The user picks it at the greeter or switches
   VTs. If the compositor exits or crashes, the session ends and the
   display manager returns; the user picks Plasma again. That is the
   desired display-server failure behavior — do not paper over it.
   - **Do not uninstall or disable the host DE during development.**
   - Two graphical sessions for the same Unix user collide through shared
     user-session services (portals, `XDG_RUNTIME_DIR`, DBus), so the
     second-VT rung uses a dedicated development user
     (`docs/testing-ladder.md` rung 2).

### Dev-tool current state and the shell TODO

- `dragonfruit dev --nested` today launches **only the compositor**; the
  shell is a placeholder until T-09/T-10, so `make dev` shows the
  compositor window + wallpaper + any `--launch`ed apps, not a menu bar or
  Dock yet.
- When the shell binary exists, `make dev` (and a `dragonfruit dev
  --nested --shell` flag) should launch it against the nested socket and
  include it in `ChildGuard` teardown. Keep the "host session
  undisturbed" guarantee: `make soak` must still pass.
- Session packaging (T-24/T-32) is what makes the desktop selectable in
  SDDM; there is no code path that shuts down or replaces the running
  Plasma session.

### Acceptance checks for the above (when implemented)

- `make dev` on a Plasma Wayland host: a Dragonfruit window opens inside
  Plasma; quitting returns to Plasma with no stray Dragonfruit processes,
  sockets, or exported `WAYLAND_DISPLAY`/`DISPLAY` left in the host env.
- A session entry appears in the display manager next to Plasma; selecting
  it starts the compositor; killing the compositor returns to the greeter;
  Plasma is still selectable and unaffected.
- No code anywhere kills, suspends, or "replaces" the host compositor, and
  no attempt is made to migrate live windows between compositors.

## T-09 — menu bar + shell process bootstrap

**State: partial.** The menu bar render/interaction core, the shell process
bootstrap, the scripted shell-restart test, and the scripted idle trace are
landed and verified; the menu dropdown overlay (plus live input routing to
chrome surfaces), the output-hotplug script, T-20 status adapters, and the
T-22 app menu remain (see the task file's hand-off list).

What exists:

- `shell/menubar/MenuBar.qml` (plus `StatusItem`, `StatusGlyph`,
  `MenuBarClock`): the real bar built from the design system. App-menu region
  (`MenuBarMenu` per top-level menu, application name as the FR-2
  priority-3 fallback), status-item slots (Wi-Fi/Bluetooth/volume/battery/
  Focus-DND/accessibility) that hide when absent and dim when disabled, a
  locale-formatted clock, Control Center entry, Mission Control button. FR-3
  interaction: click-to-open, 140 ms delayed-hover drag-through,
  Escape/click-away/focus-loss dismissal, and `setFocusedApp()` live
  focus-switch tracking (reopens the same index on the new model, closes when
  the new app exports none). Reduced motion is inherited from the design
  system's `Popup` motion tokens.
- A new `component.menuBar` token group (height, status-item padding/gap,
  icon/font size, clock gap, radii); `make check-tokens` and the gallery
  goldens stay green.
- `shell/tests/tst_menubar.qml` (ctest `tst_menubar`, offscreen + software):
  layout zones, fallback name vs menu model, adapter degradation, status
  activation, Control Center/Mission Control signals, open/close, Escape,
  click-away, drag-through, focus-switch tracking, focus-loss dismissal,
  clock locale formatting, and a pixel check that every status glyph paints.
- `shell/src/dragonfruit-shell`: the shell process. Connects with
  libwayland-client, `df_core` launch-token handshake, creates the
  `df_layer_surface` menu bar (anchor top|left|right, `set_size(0,28)`,
  exclusive zone 28, `on_demand` keyboard), renders the QML offscreen into a
  `wl_shm` ARGB8888 buffer per configure, and tracks `df_toplevel_manager`
  `focused`/`app_id`/`title` to set the app name. Mission Control button
  calls `df_toplevel_manager.enter_mission_control`.
- `dragonfruit dev --nested --shell` (and `make dev`) reads
  `$XDG_RUNTIME_DIR/<socket>.launch-token`, exports
  `DRAGONFRUIT_LAUNCH_TOKEN`, launches the shell, and owns it in
  `ChildGuard`.

Verified live headless (`dragonfruit dev --headless --shell`): the shell
authenticates as `shell`, gets a 1280×28 configure, the compositor reports
`reserved_zone edge=0 thickness=28`, and the dev tool exits with a clean
teardown.

### Gotchas learned (important for T-10+ and the T-09 continuation)

- **A plain Qt executable does not auto-register static QML modules.**
  `qt_import_qml_plugins(dragonfruit-shell)` links the plugin init objects,
  but `QQmlComponent::loadFromModule("Dragonfruit.MenuBar", "MenuBar")` still
  failed with "contains no type named MenuBar". Fix: add the build QML import
  root at runtime (`engine.addImportPath("${CMAKE_BINARY_DIR}/qml")`), exactly
  like `dragonfruit-gallery-app` does. The same will be needed by any other
  non-QML-module executable that loads shell/app QML.
- **wayland-scanner emits `namespace` as a C argument name** for
  `df_shell.get_layer_surface`; it is a C++ keyword. The shell wraps that one
  generated include with `#define namespace df_layer_namespace` /
  `#undef`. (Renaming the XML arg is the cleaner long-term fix if we ever
  want the C header to be C++-clean; the wire ABI is unaffected by arg names.)
- **The root project must enable the C language** for wayland-scanner's
  generated `*-protocol.c` to compile. `project(... LANGUAGES C CXX)`.
- **The Qt toolchain's `libwayland-client.so` is a broken symlink** (no
  `.so.0` in the toolchain lib64), and `wayland-client.pc` needs a missing
  `libffi.pc`. The shell's CMake finds `libwayland-client.so.0` in the system
  lib dirs as a fallback; CI's `libwayland-dev` uses the normal pkg-config
  path.
- **Launch tokens are 32 bytes = 64 hex chars** (`TOKEN_BYTES`), not 16. A
  short token makes `LaunchToken::parse_hex` return `None`, the compositor
  silently mints a random one, and the handshake fails as `invalid-token`.
- **The token is one-time.** The dev tool passes the provisioned token to the
  shell; a shell *restart* needs a fresh token, which is T-24's job. The
  scripted restart test is blocked on that.
- **The shell forces `QT_QPA_PLATFORM=offscreen`** and speaks Wayland itself.
  This is deliberate: the Qt Wayland platform plugin would create an unrelated
  xdg toplevel instead of our `df_layer_surface`. Any future shell component
  (Dock, Control Center, OSD) should follow the same "offscreen Qt + own
  libwayland connection" pattern.
- **The compositor sends an initial configure at the full output size**
  before applying the layer surface's size/anchor requests, then the real
  1280×28 configure. The shell renders on every configure; harmless, but a
  future optimization is to skip the pre-layout configure (or gate rendering
  on the final size).
- **The open menu is clipped to the bar surface.** `MenuBarMenu`'s popup
  overflows the 28 px `wl_surface`, so a live dropdown needs a second
  `df_layer_surface` on the `overlay` layer. *(Resolved in the fifth session:
  the popup is now its own `overlay` surface — see "T-09 continuation — true
  overlay dropdown" below.)*

### Notes for subsequent tasks

- **T-10 (Dock)** should reuse `shell/src/shellprotocol.{h,cpp}` (it already
  binds `df_shell`/`df_toplevel_manager` and tracks toplevels) and
  `ShellController`'s offscreen-render pattern; the layer-surface helper is
  currently hard-coded to the menu bar and should be generalized to a
  `createLayerSurface(namespace, anchor, size, exclusiveZone)` call.
- **T-11/T-12** can drive Mission Control/app-switcher through the same
  `df_toplevel_manager` requests; `ShellProtocol::enterMissionControl()` is
  the template.
- **T-20** replaces `ShellController::applyStatusItems()`'s placeholders with
  adapter state; the `StatusItem` slot already renders `available`/`enabled`/
  `level`/`label`/`tint`.
- **T-22** replaces `ShellController::applyFocusedApp()`'s name-only fallback
  with a broker-resolved `appMenuModel`; `MenuBar.setFocusedApp()` already
  keeps an open menu tracking focus.
- **T-24** must export a fresh `DRAGONFRUIT_LAUNCH_TOKEN` per shell start and
  own the shell in the session; the dev tool's hand-off is the reference.

### T-09 follow-up — compositor chrome rendering + the nested Y-flip

The first visual check of the nested session (a full-screen capture via
`spectacle` while `dragonfruit dev --nested --shell` ran) exposed two gaps
that no headless test could see:

1. **Chrome surfaces were placed but never composited.** T-07 stored the
   `wl_surface` in `LayerEntry` and computed reserved zones, but no backend
   ever rendered it, so the menu bar was invisible. Fix:
   - `DfState::chrome_surfaces(output_name, geometry)` (`shell/mod.rs`)
     returns the mapped chrome surfaces as `ChromeSurface { surface,
     location, layer }`, output-local and sorted by layer.
   - `render::chrome_render_elements::<R, E>(renderer, state, output,
     scale)` builds `WaylandSurfaceRenderElement`s for the `top`/`overlay`
     layers. Both `nested.rs` and `drm.rs` pass them as the renderer's
     custom-element list, so chrome composites **above** the window space
     (Smithay's damage tracker renders the list in reverse, so the custom
     elements end up on top). `DrmOutputElements` gained a `Chrome` variant.
   - `CompositorHandler::commit` already set `needs_redraw` for every
     commit, so a chrome commit schedules a frame with no extra wiring.
   - Background/bottom layer stacking (wallpaper, desktop reveal) is still
     T-10/T-11; only `top`/`overlay` are composited for now.

2. **The whole nested output rendered vertically flipped** (windows and
   chrome). This was a **pre-existing T-02 bug** — `eglgears_wayland`
   inside the nested session was upside down, confirmed by stashing the
   T-09 changes and reproducing at HEAD, and by comparing against
   `eglgears` on the host (correct). Root cause: the winit/EGL back buffer
   is bottom-up, and Smithay's `minimal.rs` winit example renders with
   `Transform::Flipped180`; our nested backend used
   `OutputDamageTracker::from_output(&output)` (output transform
   `Normal`), so `render_output` passed `Normal` to the renderer. Fix:
   build the nested damage tracker with
   `OutputDamageTracker::new(size, Scale::from(1.0), Transform::Flipped180)`
   and recreate it on `WinitEvent::Resized` (its mode is static). The
   output's own transform stays `Normal`, so client-visible output
   properties and input mapping are unaffected. The DRM backend uses
   `DrmCompositor::render_frame`, which handles DRM framebuffer
   orientation itself, so it was not affected.

After the fix, a nested capture shows the menu bar at the top ("Desktop" +
locale clock, correctly oriented) and `eglgears` upright. This is the
first live-nested evidence for T-09 FR-1/FR-2.

Notes for subsequent tasks:

- **Visual checks catch render-path bugs that headless tests cannot.** The
  headless backend has no renderer, and `milestone_e2e`/`idle_trace` only
  count frames. A nested (or DRM) screenshot is required to validate
  compositing and orientation. `spectacle -b -n -a -o out.png` works on
  this KDE host; the shell can save its own grabbed frame for comparison
  (temporarily) to isolate shell-vs-compositor issues.
- **Any new backend that renders into a bottom-up GL surface must use
  `Transform::Flipped180`** in its damage tracker (or render with it
  directly, as `minimal.rs` does). The nested backend is the only such
  case today.
- **`chrome_render_elements` filters to `layer >= 2`.** When T-10 adds a
  Dock on `bottom` or a wallpaper on `background`, the render list must be
  split so those layers composite *below* the window space instead of
  being skipped or drawn on top.

### T-09 — menu-bar icon style pass + the Canvas grab timing fix

After the first nested capture, the status glyphs were given a visual style
pass. Decision (maintainer): **original geometry only, polished to read as a
macOS-style family** — not replicas of Apple's SF Symbols. `.docs/reference/
macos/` holds real macOS screenshots (the menu-bar status icons are AirDrop,
Bluetooth, Wi-Fi, battery, Spotlight, date, Control Center). Per
[14-risks.md](../.docs/design/14-risks.md) ("reproduce the interaction
quality and mental model, not Apple's bitmap output"; do not ship "Apple
icons" / "pixel-copied proprietary artwork"), the glyphs stay our own
geometry. The reference is a style guide only.

What changed in `shell/menubar/StatusGlyph.qml`:

- One uniform optical stroke (`max(1.3, size * 0.10)`), rounded caps/joins.
- Wi-Fi: three thin arcs + a small filled origin dot.
- Bluetooth: the rune (spine + two-triangle bowtie) with corrected
  proportions.
- Volume: filled speaker cone + thin waves (`volume-muted` adds the X).
- Battery: **outline cell + nub + rounded level fill** (was a solid block),
  via a small `roundedRect` path helper.
- Control Center: two toggle pills with knobs (was two sliders), matching
  the macOS Control Center silhouette.
- Mission Control keeps our own window-grid mark (not a macOS menu-bar item).
- The clock now shows the locale short **date** as well as the time
  (`ShellController` sets `showDate`).

### Gotcha: `Canvas` items were blank in the shell's one-shot grab

The status glyphs rendered in `tst_menubar` (via `grabImage`) but were blank
in the shell. Root cause: the shell renders once on `configure`, and
`QQuickWindow::grabWindow()` at that moment runs before the `Canvas` items
have painted; because the shell never rendered again, the compositor kept the
blank frame. (`tst_menubar` does not hit this because `grabImage` triggers a
fresh scene-graph render.) Fix: `ShellController::onConfigured` schedules a
one-shot re-render 200 ms after the first (`QTimer::singleShot`), once the
event loop has let the Canvases paint. Also added `onWindowChanged`/
`onVisibleChanged` `requestPaint()` to `StatusGlyph` as a belt-and-braces.

Notes for subsequent tasks:

- **Any shell chrome built with `Canvas` + a one-shot `grabWindow` must
  re-render after the scene graph has painted.** The Dock (T-10) and OSD
  (T-25) will hit this if they use `Canvas`. A cleaner long-term fix is to
  drive the commit from `QQuickWindow::afterRendering` (or a render-control
  path) instead of a timer; revisit with T-10.
- **`.docs/reference/macos/` is a style reference, not an asset source.** See
  the IP rule in 14-risks.md before drawing anything that looks like an Apple
  icon. The status glyphs are original geometry in `StatusGlyph.qml`.
- **Date format is the host locale's** (`Qt.locale().dateFormat`); this host
  renders ISO (`2026-09-19`). If a locale like `Fri Sep 19` is wanted, that is
  a settingsd/locale concern (T-15), not the shell.

### T-09 continuation — scripted shell restart + idle trace

This session closed the two remaining T-09 acceptance criteria with headless
integration tests; the task is still partial because the overlay dropdown,
the hotplug script, and the T-20/T-22 content remain.

**Scripted shell restart (FR-5/FR-9).**
`shell_restart_reanchors_chrome_and_preserves_windows` in
`compositor/tests/shell_protocol_conformance.rs`. The shape:

1. Start the compositor with **three** tokens via
   `DRAGONFRUIT_LAUNCH_TOKENS` (comma-separated). This is the test-only
   stand-in for T-24's per-start mint: the store is provisioned up front and
   each shell start redeems a different one-time token.
2. An *observer* trusted client stays connected for the whole test and binds
   `df_toplevel_manager`, so it can watch the reserved zone and the window
   list across both shell lifetimes.
3. An ordinary application client maps an `xdg_toplevel` ("Restart
   Survivor") and stays alive. **It must be a separate connection from the
   shell** — if the shell owned the `wl_surface`, dropping the shell would
   destroy the window and the test would prove nothing.
4. Shell #1 authenticates with token 0, creates the top `df_layer_surface`
   (anchor top|left|right, size 0×28, exclusive 28), and the observer sees
   `reserved_zone(edge=0, 28)`.
5. The shell connection is dropped without a clean `destroy` (a crash). The
   observer sees `reserved_zone(edge=0, 0)` and the toplevel is still
   announced, never `closed`.
6. Shell #2 authenticates with token 1 on a fresh connection, recreates the
   bar, and the observer sees the zone return; the last configure is
   1280×28 (the pre-layout full-output configure is still sent first — see
   the earlier gotcha).

Gotchas confirmed while writing it:

- **Clear the observer's event vector *before* triggering the transition.**
  `wait_for` dispatches the observer queue; an event that arrived while the
  queue was not being dispatched sits in the socket, not in the `Vec`, so
  clearing before the trigger is race-free. Clearing after the transition
  can miss an already-dispatched event and time out.
- **A chrome surface's reserved zone clears on client disconnect** because
  `df_layer_surface::destroyed` removes the layer and calls
  `refresh_reserved_zones`, which broadcasts the zeroed zones. This is what
  the test asserts; no explicit "shell died" notification exists or is
  needed (the manager is a pure consumer, FR-9).
- The compositor sends `reserved_zone` for **all four edges** on every
  change (including zeros), which is what makes the clear observable.
- `map_toplevel`/`shm_buffer` already existed in the conformance harness;
  reuse them rather than re-deriving the xdg state machine.

**Scripted idle trace (FR-6).** `compositor/tests/shell_idle_trace.rs` (added
to `make e2e`). It starts a headless compositor with piped stdout, attaches a
raw `wayland-client` stand-in for the shell (authenticate → create the menu
bar → commit one `wl_shm` frame), then samples the SIGUSR1 render counters a
second apart and asserts `frames_rendered` is flat and `direct_scanouts` never
advances. The stand-in is deliberate: `cargo test` runs before the Qt build in
CI, so the real `dragonfruit-shell` binary is not available there. The
shell-side half of the budget is the one-shot minute-aligned
`MenuBarClock` timer (no polling loop).

Notes for subsequent tasks:

- **The overlay dropdown is blocked on input routing, not just a surface.**
  The compositor's `input::surface_under` hit-tests only `state.space`
  windows; chrome `df_layer_surface`s are never candidates, and the shell
  (offscreen Qt + its own libwayland connection) has no path to inject
  `wl_pointer`/`wl_keyboard` events into the QML scene. Implementing the
  overlay therefore means (a) hit-testing chrome surfaces by layer
  (`overlay` > `top` > window space), (b) setting pointer/keyboard focus to
  the chrome `wl_surface` honoring `df_layer_surface.set_keyboard_interaction`,
  and (c) a shell-side event bridge (synthesize Qt events, or call the
  `MenuBar` functions directly from `ShellProtocol` pointer/keyboard
  handlers). Consider splitting that into its own sub-task.
- **Hotplug scripting needs a runtime output hook.** `on_output_added`
  already calls `reconfigure_layers`, so the compositor behavior is in
  place; the headless backend just has no way to add a second output after
  startup. A `SIGUSR2`-driven or env-driven test output (like the
  `DRAGONFRUIT_SYNTHETIC_INPUT` opt-in) would make the hotplug case
  scriptable without touching the production path.
- **Keep the restart test's "independent app connection" invariant.** Any
  future restart/reconnect test must map the window from a client that
  outlives the shell, or the window is destroyed by the disconnect and the
  test silently stops testing FR-9.
- `make e2e` now includes `shell_idle_trace`; `cargo test --workspace` picks
  up both new tests automatically (integration test files are auto-discovered).

## T-09 continuation — interactive chrome (input routing + dropdown)

**State: the menu bar is now interactive in a live session.** The compositor
routes pointer/keyboard input to chrome surfaces; the shell binds the seat,
synthesizes Qt input into its offscreen scene, and grows the menu-bar surface
to reveal the open dropdown. Verified live nested (File menu renders below the
bar with rows + shortcut labels) and live headless (configure round-trip
1280×28 → 1280×124 → 1280×28 driven by synthetic pointer + Escape).

What landed:

- **Compositor (`compositor/src/input.rs`).**
  - `chrome_under(state, point)` hit-tests chrome surfaces above the window
    space, topmost layer first, via `smithay::desktop::utils::
    under_from_surface_tree`. It needs the output under the point and
    `DfState::chrome_surfaces(output, geometry)` (which now carries the
    surface's `KeyboardInteraction`).
  - `surface_under` checks chrome first, then windows, and returns the
    surface origin **in global space**.
  - Click-to-focus: a chrome hit calls `DfState::focus_chrome_surface`,
    which refuses `KeyboardInteraction::None` and otherwise sets keyboard
    focus; a window hit focuses the window as before.
- **Compositor (`compositor/src/shell/mod.rs`).**
  - `DfState::is_chrome_surface`, `chrome_keyboard_interaction`,
    `focus_chrome_surface`, `restore_window_keyboard_focus`.
  - `Exclusive` chrome takes focus on `set_keyboard_interaction`; a destroyed
    chrome surface that held focus restores the active window (shell crash
    safe).
  - `focus_changed` (`state.rs`) preserves `active_window` while a chrome
    surface holds the keyboard, so focus can be handed back.
- **Shell (`shell/src/shellprotocol.{h,cpp}`).** Binds `wl_seat` and owns a
  `wl_pointer`/`wl_keyboard`; emits `pointerMoved`, `pointerButton`,
  `pointerLeft`, `keyboardFocused`, `keyEvent`, plus
  `setMenuBarSize(width,height)` (a `df_layer_surface.set_size` + commit).
- **Shell (`shell/src/shellcontroller.{h,cpp}`).** Synthesizes `QMouseEvent`/
  `QKeyEvent` into the offscreen `QQuickWindow` (`QCoreApplication::sendEvent`)
  so the QML `HoverHandler`/`TapHandler`/`Keys` logic is reused unchanged;
  maps evdev → Qt keys for Escape/arrows/Home/End/Return/Space; tracks
  `shellFocused`; on `appMenuOpened`/`appMenuClosed` reads
  `MenuBar.dropdownBottom` and resizes the surface; a `--placeholders` demo
  app menu (`demoAppMenu`) stands in for T-22 so the dropdown is usable.
- **QML (`shell/menubar/MenuBar.qml`).** `readonly property real
  dropdownBottom`; the bar's click-away `TapHandler` now ignores taps inside
  the app-menu row (the title's own handler toggles; without the guard the
  click-away closed the menu on the same tap — caught by the new
  `tst_menubar` case).

Bugs found and fixed along the way:

- **`surface_under` returned a window-relative surface origin.** Smithay's
  `Window::surface_under` returns a location relative to the window;
  `input::surface_under` passed it straight to `PointerHandle::motion`, which
  subtracts it from the global pointer position. Every window not at the
  output origin therefore received pointer/touch coordinates offset by its
  position. Fix: add `window_loc` to the inner location (anvil's pattern).
  This affects *all* pointer input to windows, not just chrome. The chrome
  hit test has the same trap (`under_from_surface_tree` takes the point
  relative to the surface and offsets the returned origin): it passes the
  surface-local point with location `(0,0)` and adds the global origin back.
  `chrome_surface_receives_pointer_and_keyboard` includes a bottom-right
  panel with a non-zero origin as the regression guard (it fails on the
  window-relative variant).
- **QML functions are not invokable as `openMenu(int)`.** `QMetaObject::
  invokeMethod(item, "openMenu", Q_ARG(int, 0))` fails with "No such method";
  QML-declared functions take `QVariant` parameters. Prefer exposing a QML
  `readonly property` (as `dropdownBottom` does) over `invokeMethod`.
- **The bar's click-away `TapHandler` fired on the same tap as a menu
  title's `TapHandler`**, opening and immediately closing the menu. The
  click-away now ignores the app-menu row's bounds. This was invisible to the
  existing tests because they call `bar.openMenu(0)` directly; the new
  `test_click_menu_title_opens_and_stays_open` uses a real `mouseClick`.

Design decision (recorded for T-10/T-21):

- **The dropdown rides the `top` menu-bar surface, grown while open**, rather
  than a second `overlay` `df_layer_surface`. The design doc puts menus on
  `overlay`; the reason for the deviation is that the design-system `Popup`
  is a child item of the bar's `MenuBarMenu`, so a separate surface would
  need a second `QQuickWindow`/content item (a real refactor). Growing the
  bar surface keeps one surface, one render path, and one input region, and
  the exclusive zone stays 28 so Zoom is unaffected. The trade-off: while a
  menu is open the whole expanded rectangle captures clicks (this *is* the
  click-away behaviour). Move to a true overlay surface when a second `top`
  surface or an OSD-above-menu stacking need appears (T-10/T-21/T-25).

Notes for subsequent tasks:

- **Input routing to chrome is in `input::chrome_under` /
  `surface_under`.** Any new chrome surface is automatically hit-tested; set
  `df_layer_surface.set_keyboard_interaction` correctly (`None` for
  non-interactive chrome). The compositor will not focus a `None` surface.
- **The shell is a normal seat client now.** It binds `wl_seat` at registry
  time (before authentication is fine) and gets pointer/keyboard only when a
  chrome surface has focus. `ShellProtocol::m_pointerX/Y` cache the last
  position so `button` events (which carry no coords) can be placed.
- **Qt event synthesis works offscreen** (`QCoreApplication::sendEvent` to the
  `QQuickWindow`). The Dock (T-10), Control Center (T-21), and OSD (T-25)
  should follow the same pattern rather than calling QML functions directly.
- **`MenuBar.dropdownBottom` is the geometry contract** between QML and the
  shell's surface sizing; update it if the popup is ever reparented.
- **The `surface_under` global-origin fix changes behavior for every window
  hit-test.** The existing `window_conformance` / `shell_protocol_conformance`
  suites still pass, but a pointer-position assertion for an off-origin
  window would be a good addition (the synthetic harness can place one).
- **`chrome_surfaces` still returns the single global `reserved` union**;
  per-output zones remain T-11/T-16.

### T-09 HITL follow-up — nested pointer coordinates + launch focus ring

The first human-in-the-loop nested session surfaced two issues that no
headless test could see:

- **Nested pointer input was completely dead.** `InputEvent::
  PointerMotionAbsolute` treated `event.position()` as device-normalized
  (`0..=1`) and multiplied by the output size. That is correct for
  libinput/synthetic, but winit's `CursorMoved` (via
  `WinitMouseMovedEvent::x()`) is **window pixels**, so every nested motion
  computed a location millions of pixels off-output, `surface_under` returned
  `None`, and the shell never got pointer enter/motion/button. Fix: use
  `event.position_transformed(geometry.size)`, the backend-agnostic
  conversion (winit divides by the window size then scales; libinput scales
  the normalized value). The synthetic-input and window conformance suites
  still pass because their `x_transformed` is the same as before.
- **The first menu title drew its `FocusRing` on launch.** The offscreen
  `QQuickWindow` hands active focus to the first `activeFocusOnTab` item
  (`MenuBarMenu`) when shown, so "File" looked pre-selected. The shell now
  clears `activeFocusItem()` on the first event-loop turn; the popup takes
  focus itself when it opens.

Notes:

- **Touch/tablet absolute handling still uses `event.position()`** in
  `touch_location` (normalized). Winit does not emit touch, so the nested bug
  did not affect it, but if a backend ever reports pixel-space touch the same
  `position_transformed` treatment is needed.
- **Nested input has no automated coverage.** The synthetic harness only
  exists on the headless backend, and the winit event type is private to
  Smithay. A host-side pointer move is the only current way to exercise it;
  keep the `position_transformed` rule when adding backends.

### T-09 FR-3 interaction polish (second HITL pass)

The nested dropdown worked but felt laggy/janky. Root cause: the shell
snapshots the QML scene into a shm buffer **on demand**, so QML visual changes
that the shell did not know about (hover highlights, the popup's 160 ms
fade/scale-in) were never committed. Fixes in `ShellController`:

- `scheduleRender()` (coalesced `singleShot(0)`) is called after every
  synthesized pointer/key event, so hover/title/row highlights reach the
  compositor.
- `startAnimationRenders(ms)` renders every ~16 ms for the popup open
  (220 ms) and close (160 ms) animations.
- `setMenuBarSize` no longer commits the old buffer; `updateSurfaceHeight`
  sets the size and renders in one step, so the compositor never shows the
  28 px buffer stretched/blank at the new height.
- The close shrink is delayed by the close animation, and `MenuBar.closeMenus`
  only emits `appMenuClosed` when a menu was actually open (a menu item's own
  handler plus the bar click-away used to emit twice).
- The launch `FocusRing` fix is now `m_item->forceActiveFocus()` on the bar
  root (clearing `activeFocusItem()` was not reliable; the window reassigned
  focus to the first `activeFocusOnTab` title).
- `appMenuTriggered` is wired to a log slot so placeholder item clicks have
  feedback; T-22 replaces it with the resolved action.

Remaining known limitation: animations are still sampled by a timer, not the
scene graph, so motion is choppy at 60 Hz budget. The durable fix is to commit
from `QQuickWindow::afterRendering` (or a render-control path) while the scene
is dirty — flagged for T-10 (Dock animations need the same).

### T-09 FR-3 polish follow-up — desktop click-away

Clicking empty desktop space did not dismiss an open menu: the shell only
sees clicks that land on its chrome surface, and the compositor's
click-to-focus did nothing when no window was under the pointer, so the chrome
kept keyboard focus and the menu stayed open. `process_input_event`
(`compositor/src/input.rs`) now drops chrome keyboard focus on a left click
over empty space (`DfState::chrome_has_keyboard_focus` +
`keyboard.set_focus(None)`), which delivers `wl_keyboard.leave` to the shell;
`MenuBar.onShellFocusedChanged` then closes the menu. Clicking a window
already worked because focus moves to the window. Covered by
`chrome_surface_receives_pointer_and_keyboard` (a click at an empty
desktop point asserts `wl_keyboard.leave`).

Also hardened the launch focus: the `MenuBar` delegate sets
`activeFocusOnTab: false` so the offscreen window cannot auto-focus the first
title at all (the `forceActiveFocus` on the bar root remains). The remaining
"File looks highlighted at launch" report was reproduced as *hover*: the
shell receives no pointer event at startup, so a highlight can only come from
the host cursor already being over the title (winit's first `CursorMoved`).
It clears as soon as the pointer leaves the bar.

### T-09 deferred polish backlog (known, not blocking T-10+)

T-09 is closed for development purposes; these are the remaining rough edges,
all in the shell's snapshot renderer / interaction feel rather than the
compositor or protocol:

1. **Launch highlight on the first title.** Reproduced by the user with the
   host cursor *outside* the window, so it is not host-cursor hover. The
   shell receives no pointer event at startup, so the only remaining source
   is the offscreen window's initial focus/hover state at (0,0), committed in
   the first frame and not refreshed until interaction. Fixed two ways:
   `MenuBarMenu.showFocusRing` (new, default true) is set false by the shell
   delegate so a title can never draw a FocusRing; and
   `ShellController::settleInitialState` forces focus to the bar root, clears
   hover with an off-bar synthetic move, and re-commits at 0 ms and 400 ms.
   If a fill still appears, the next step is a diagnostic that dumps
   `openMenuIndex`/hover/activeFocus and saves the committed frame.
2. **Choppy popup open/close.** The shell samples QML with a 16 ms timer, so
   motion is not frame-accurate. The durable fix — commit from
   `QQuickWindow::afterRendering` while the scene is dirty — is shared with
   T-10 (Dock magnification) and should be done there, not bolted on here.
3. **Hover/drag-through timing.** Mechanism is per-spec (140 ms
   `motion.menuOpen`), but the feel is untuned; the constant lives in the
   design-system tokens, so tune it in T-10/T-31 with the real Dock.
4. **Hover highlights on the bar** may lag a frame behind the pointer for the
   same snapshot reason as (2).

None of these affect the compositor contracts T-10/T-11/T-20/T-22 build on
(chrome input routing, keyboard focus, reserved zones, dropdown geometry).

## T-09 continuation — output hotplug (FR-1) + the synthetic-output harness

**State: the last host-closable T-09 FR-1 item is done.** The menu bar now
anchors on outputs attached after the shell starts, and the hotplug path is
scripted end to end on the headless backend. T-09 stays partial only for the
overlay-dropdown polish and the T-20/T-22 content (the T-20/T-22 blockers are
external tickets).

What landed:

- **`None` output now means *all* outputs.** `df_shell.get_layer_surface`
  used to resolve an unspecified output to the primary; that made a
  single-surface bar unable to appear on a hotplugged display. The handler
  now stores `output: None`, and `LayerSurfaceState::matches_output` is the
  one predicate `chrome_surfaces` filters on (`map_or(true, ...)`). This is
  the `wlr-layer-shell` contract and is documented on the method. A
  `LayerSurfaceState::configure` still uses the *first* output's geometry for
  sizing, so on mixed-resolution multi-monitor the bar buffer is sized to the
  first output and stretched on the others — acceptable until T-11/T-16 own
  per-output chrome and per-output reserved zones.
- **Synthetic-output harness** (`compositor/src/backend/synthetic_output.rs`,
  `DRAGONFRUIT_SYNTHETIC_OUTPUT`). Opt-in, headless-only, mirrors the T-03
  synthetic-input pattern: a `UnixDatagram` line protocol
  (`add <name> <width> <height> <x> <y> [scale]`, `remove <name>`),
  `parse_command` unit-tested, malformed lines logged and skipped. The
  callback owns a `HashMap<String, Output>` of the outputs it created
  (`Output` needs a live instance to unmap); `remove` runs the same ordering
  as the DRM hotplug path — `on_output_removed` (which migrates windows to
  the remaining primary) then `space.unmap_output`. Socket removed at
  teardown by `headless.rs`.
- **Scripted test** `shell_output_hotplug_reanchors_chrome`
  (`compositor/tests/shell_protocol_conformance.rs`). One trusted client is
  both shell and observer; `open_trusted_bar` creates the bar with
  `output = None`; the harness attaches `HDMI-A-1`; the test asserts the new
  `df_output` name, its geometry `(1280, 0, 1920, 1080)`, its three fresh
  Spaces (3 → 6), the bar's `reserved_zone(top, 28)` on the new output, and a
  new `df_layer_surface` configure; detaching asserts the Spaces are removed.
  `CompositorProcess::start_with_harnesses` now takes both harness paths
  (`start`/`start_with_synthetic` are thin wrappers).

Notes for subsequent tasks:

- **`DRAGONFRUIT_SYNTHETIC_OUTPUT` is test-only.** Only the headless backend
  reads it; nested/DRM ignore it. Never set it in a real session.
- **The `wl_output` global is not removed on synthetic detach** (the DRM path
  has the same gap — `add_output` discards the `GlobalId`). The shell-visible
  removal (`df_output.removed`, Spaces, migration) is correct because the
  manager reconciles against `space.outputs()`. If a future ticket needs the
  `wl_output` global to actually disappear, thread the `GlobalId` out of
  `backend::add_output` and remove it on the detach path.
- **Per-output reserved zones remain a single global union** and per-output
  chrome sizing is first-output-only; both are T-11/T-16. `matches_output`
  makes adding per-output *placement* a filter change, not a redesign.
- **T-10 should reuse `matches_output`** for the Dock (also created without
  an output) so it appears on every display and on hotplug the same way.
- **`start_with_harnesses` is the compositor-test entry point** for any new
  synthetic channel; keep new opt-in hooks headless-only with a
  teardown removal in `headless.rs` (the socket is session state, not a
  leak — the phase-exit leak check only covers the Wayland socket).
- **The full gate is green** (`make lint`, `make cargo-test`, `make e2e`,
  `make soak` 100 cycles). `shell_output_hotplug_reanchors_chrome` is
  auto-discovered by `cargo test --workspace` and included in `make e2e`
  through the `shell_protocol_conformance` binary.

## T-09 continuation — true overlay dropdown (fifth session)

**State: the dropdown is now a real `overlay`-layer chrome surface.** T-09's
last in-scope host-closable deviation from the design is closed; the ticket
stays partial only for the T-20 status adapters and the T-22 menu-broker
(external tickets). The bar is again a constant 28 px `top` surface and no
longer grows while a menu is open.

What landed:

- **Two chrome surfaces, one offscreen window.** `createPopupSurface()` in
  `ShellProtocol` creates a second `wl_surface` + `df_layer_surface` on
  `DF_SHELL_LAYER_OVERLAY` with namespace `menubar-popup`, anchored
  `top|left`, `set_exclusive_zone(-1)` (it never enlarges the reserved
  zone) and `keyboard = none` (the bar surface keeps focus for Escape and
  menu navigation). It is created unmapped (null-buffer commit) and mapped on
  the first menu open. The popup's position is the open menu's rectangle:
  `setPopupGeometry(x, y, w, h)` sends `set_margin(top=y, left=x)` +
  `set_size(w, h)`.
- **`ShellController` splits one render.** The QML window still renders
  bar+popup in one scene (so all interaction logic and the QML test suite are
  unchanged), but the committed frame is split: the top `m_barHeight` strip
  goes to the `top` bar surface and the dropdown rectangle goes to the
  `overlay` surface. `render()` sizes the window to
  `max(barHeight, popupY + popupHeight)`. `MenuBar` exposes
  `dropdownX/Y/Width/Height` (computed from the open `MenuBarMenu.popup`'s
  `mapToItem`); `tst_menubar`'s `test_dropdown_geometry_tracks_open_menu`
  pins the contract. On close the overlay stays mapped for the ~140 ms close
  animation, then `updatePopupGeometry()` unmaps it.
- **Pointer translation.** `wl_pointer` coordinates arriving over the popup
  surface are surface-local; `ShellProtocol` records the popup origin
  (`m_popupX/Y` from `setPopupGeometry`, `onPointerEnter` compares the
  entered `wl_surface` to `m_popupSurface`) and emits window coordinates, so
  `ShellController`'s existing Qt synthesis is untouched.
- **Scripted compositor test**
  `overlay_popup_sits_above_the_bar_and_reserves_nothing`
  (`shell_protocol_conformance.rs`): maps a top bar and an overlay popup on
  the headless output, asserts the popup configures to 200×300, that the
  reserved zone stays 28, and that a synthetic pointer at global (140,100)
  enters the popup at popup-local (100,72) — i.e. the overlay is hit-tested
  with its own origin below the bar. The test binds `df_toplevel_manager` to
  observe `output_reserved`, and the test client now records
  `pointer_enter_positions` (the `wl_pointer.enter` position).

Notes for subsequent tasks:

- **A focus change emits `wl_pointer.enter` but not always a `motion`.** The
  first motion onto a new surface sets focus and carries the position in the
  enter event; the compositor does not also send a `motion` for that
  transition. Tests must assert the enter position, not wait for a motion —
  that was the trap in the new test.
- **`overlay` popups target every output too** (they are created with
  `output = None`, like the bar). On a multi-monitor host the dropdown would
  currently appear at the same output-local rectangle on each monitor. This
  is the same per-output-chrome limitation the bar has and stays with
  T-11/T-16.
- **The shell popup is not gated on its configure.** `setPopupGeometry`
  flushes and the buffer is committed immediately; Smithay ignores
  `ack_configure` for this interface, so the buffer-size/configure ordering
  is not enforced. The compositor places the surface from the requested
  geometry, so this is fine — but if a future protocol revision enforces
  acks, the shell should wait for `popupConfigured` before the first commit.
- **Live nested verification was not re-run this session** (the sandbox has
  no pointer automation). Verify by hand: `make dev`, click a `--placeholders`
  app-menu title; the dropdown should render below the bar with the correct
  width and dismiss on Escape/click-away as before.
- Gate green: `make lint` (qmllint + ctest incl. `tst_menubar`), `make e2e`
  (incl. the new conformance test), `cargo clippy --workspace --all-targets`,
  and a clean `dragonfruit dev --headless --shell` startup.

## T-10 — Dock (first slice: presentation core + shell surface)

**State: partial.** The Dock's presentation and interaction core, its chrome
surface, and the running-app projection are landed and verified; the ticket
splits into further slices (launch, menus, drag, Trash state, settings,
bounce, a11y). See the hand-off list at the end of this section.

### What exists

- `shell/dock/Dock.qml`, `DockEntry.qml`, `DockGlyph.qml`: the real Dock built
  from the design system. Entry regions (apps | divider | minimized | Trash,
  Trash always last), one running indicator per app entry (hidden when
  `showIndicators` is off), progress-based pointer-anchored cosine
  magnification with scaled gaps and a baseline-sized bar slab, bottom/left/
  right placement, the auto-hide translation, hover/press states, left-click
  activation and right-click menu signals, and `Accessible` listitem roles
  with state-bearing names. App artwork is an original deterministically
  coloured tile with the app initial; the Trash is original geometry with an
  empty/full state (no Apple assets — 14-risks.md).
- A new `component.dock` token group (iconSize, padding, gap, radius,
  indicatorSize/Gap, magnifyPeak, magnifyFalloff, labelSize, trashSize,
  edgeMargin, revealDelay, hideDelay). `Theme.controls.dock.*` and the Rust
  `design_tokens::dock` module are generated.
- `shell/src/shellprotocol.{h,cpp}`: `createDockSurface` (a `top` layer
  `df_layer_surface`, namespace `"dock"`, anchored bottom|left|right,
  `exclusive_zone` = baseline bar thickness, keyboard `none`),
  `commitDockImage`, `setDockInputRegion`, `activateApp`, dock pointer
  routing (`dockPointerMoved/Button/Left`), and `dockStateChanged` — the
  running-app projection built from `m_toplevels` (one entry per non-empty
  `app_id`, plus a per-window `minimized` entry; `displayNameForAppId` falls
  back to the last reverse-DNS segment until T-23).
- `shell/src/shellcontroller.{h,cpp}`: a second offscreen `QQuickWindow` for
  the Dock, loaded from `Dragonfruit.Dock`; reads `barThickness`/`magnifyBand`
  from QML to size the surface (`1280x95` on the headless output); feeds
  `entries` from `dockStateChanged`; renders on state/pointer changes; sets
  the input region to the visible bar slab (`barRect`) each frame; routes a
  left click to `activate_app`, logs Trash/menu clicks.
- Tests: `shell/tests/tst_dock.qml` + `tst_dock.cpp` (ctest `tst_dock`, 20
  cases) and
  `dock_surface_reserves_the_bottom_zone_and_coexists_with_the_bar`
  (`shell_protocol_conformance.rs`). Live headless verified:
  `dragonfruit dev --headless --shell` reports `Dock configured 1280x95` and
  `output reserved zone edge=1 thickness=60`.

### Gotchas learned (important for the next slice)

- **A QML module with no backing C++ target produces a `MODULE_LIBRARY`
  plugin that cannot be linked into an executable.** `qt_add_qml_module`
  alone made `dragonfruit-shell-dockplugin` un-linkable (`target_link_libraries`
  error). Fix: declare `qt_add_library(dragonfruit-shell-dock STATIC)` first,
  exactly like the menubar module, then `qt_add_qml_module` on it.
- **The shell must link every QML plugin it loads.** The shell only linked the
  menubar plugin, so `loadFromModule("Dragonfruit.Dock", ...)` fell through to
  the filesystem import root (`build/qml`) and loaded a stale
  `libdragonfruit-shell-dockplugin.so` from an earlier build — the placeholder
  `Dock` with no signals/properties (`barThickness` read as 0, and
  `QObject::connect: No such signal ...`). Link
  `dragonfruit-shell-dockplugin` in `shell/src/CMakeLists.txt` and delete any
  stale `.so` under `build/qml/`. A clean build is unaffected.
- **QML `var` signal parameters are exposed to C++ as `QVariant` and `real` as
  `double`.** Old-style `SIGNAL(entryActivated(QVariant))` /
  `SIGNAL(...(QVariant,qreal,qreal))` connects work (`qreal` normalizes to
  `double`). The connect must be after the QML object is created and the
  plugin linked.
- **`QQuickTest` `-input <dir>` runs *every* `tst_*.qml` in the directory.** A
  new test file was picked up by the old binary too, which does not link the
  new plugin. Point each ctest at its own file: `-input .../tst_dock.qml`.
- **The Dock surface height includes the magnified band, but the reserved
  zone is only the baseline bar.** `barThickness` = `iconSize + 2*padding`
  (60), `magnifyBand` = `ceil((peak-1)*iconSize) + padding` (35), so the
  surface is 95 px but reserves 60. The controller sets the input region to
  `barRect` so the transparent band passes clicks through (FR-13).
- **The compositor's `LayerSurfaceState::reserved()` only understands
  top/bottom anchors.** A left/right Dock (anchored top|bottom|left) would
  report no reserved zone because both vertical anchors are set. The QML
  supports all three positions, but the shell only creates a bottom surface
  today; left/right need the compositor's reserved-zone math extended.
- **The compositor's `chrome_render_elements` filters `layer >= 2`.** The Dock
  is a `top` surface, so it composites above the window space with no render
  change. A `bottom`-layer Dock/wallpaper still needs the render list split
  (T-11).

### Hand-off / open items (next T-10 sessions)

1. **Launch.** Pinned apps cannot be launched yet (there is no resolver or
   launcher). Add the interim `.desktop` resolver/launcher (pure Qt: scan the
   XDG `applications` dirs, parse `Name`/`Icon`/`Exec`/`StartupWMClass`, launch
   with `QProcess::startDetached` stripping field codes), explicitly marked
   for deletion when T-23's app-index lands. The pinned set is `dock.pinned`
   (section 19); until settingsd lands, persist under
   `$XDG_CONFIG_HOME/dragonfruit/` in the eventual key shape.
2. **Context menus and the window chooser** (section 13/9). The Dock already
   emits `entryContextMenuRequested`/`dividerContextMenuRequested`; build the
   design-system `ContextMenu` on the existing `overlay` popup surface pattern.
   The chooser needs a per-app window list from the shell (the projection
   already has the windows) and `df_toplevel.activate` / `unminimize`.
3. **Drag rearrangement and external drops** (section 12): reorder pinned,
   promote-to-pinned, remove-from-Dock, file/app drops.
4. **Trash** (section 16): GVfs `trash://` `GFileMonitor` for the icon/badge
   state (currently a `trashFull` property), click-to-Files (`org.dragonfruit.Files1`),
   drop-to-trash via GIO, Empty Trash with confirmation.
5. **Settings** (section 19): observe `dock.*` keys, live re-layout, the
   `dock.position`/`autoHide` surface changes (exclusive zone 0 when
   auto-hiding), and the T-16 pane hooks.
6. **Attention/launch bounce** (section 8.1) from the `attention` event
   (currently ignored by the shell) and the `launching` entry state.
7. **Magnified-band input region**: set the region to the bar plus the
   currently magnified icon rectangles each frame (currently bar-only, so the
   upper half of a magnified icon passes clicks through).
8. **Per-output / per-position surfaces**: the Dock is created with
   `output = None` (every output) like the bar, sized to the first output
   (the known T-09/T-11/T-16 limitation); left/right reserved zones need the
   compositor change above.
9. **Scene-graph render path** (FR-14): the Dock still commits on demand via
   `grabWindow`; the durable `QQuickWindow::afterRendering` fix is shared with
   the T-09 deferred-polish backlog and is required for smooth magnification.
10. **Keyboard + AT-SPI walkthrough** (section 20): Fn-Control-F3 focus,
    arrow navigation, Super+Option+D; the QML roles exist, the seat path and
    the live AT-SPI dump are not built.

### Gate status

`make lint`-equivalent green: `cargo fmt --check`, `cargo clippy --workspace
--all-targets -D warnings`, `ctest` 12/12 (incl. `tst_dock` and
`qmllint_shell-dock`), `gen-tokens --check`, the design-token/desktop-name/
no-capture gates, and the 66-snapshot gallery regression. The full
`shell_protocol_conformance` suite is 15/15.

## T-10 continuation — launch + pinned apps (second slice)

**State: partial.** Pinned apps now launch, the pinned set persists, and the
running projection merges with the pinned set. The remaining slices (context
menus/chooser, drag, Trash, live settings, bounce, input region, per-output
surfaces, scene-graph render path, a11y) are unchanged and listed at the end.

### What landed

- **Interim `.desktop` resolver/launcher** (`shell/src/desktopentry.{h,cpp}`).
  `DesktopEntryIndex::scan()` reads the XDG application dirs (user first, then
  `$XDG_DATA_DIRS`, then Flatpak exports) and parses `[Desktop Entry]` fields
  (`Name`, `Icon`, `Exec`, `StartupWMClass`, `Categories`, `Terminal`,
  `NoDisplay`/`Hidden`; localized keys ignored). `resolve()` matches desktop
  id (with/without `.desktop`), `StartupWMClass` (case-insensitive), then the
  reverse-DNS basename stem. `buildLaunchCommand()` tokenizes the Exec line
  (quotes/escapes) and expands field codes (`%f/%F/%u/%U`, `%i`, `%c`, `%k`,
  `%%`; deprecated codes dropped), wrapping `Terminal=true` apps in
  `$TERMINAL`/`x-terminal-emulator`. **Delete this file when T-23's
  app-index lands** — the Dock switches to `org.dragonfruit.AppIndex1` and
  behavior is unchanged.
- **Interim `dock.pinned` persistence** (`shell/src/dockpins.{h,cpp}`). Writes
  `$XDG_CONFIG_HOME/dragonfruit/settings.json` as
  `{"schema":1,"keys":{"dock.pinned":[...]}}`, preserving unknown keys and
  future top-level fields. Defaults are seeded **only when the file is
  absent**, so an intentionally emptied pin set survives. Default set:
  `org.dragonfruit.Files`/`Settings` by id, then the first non-`NoDisplay`
  entry with the freedesktop `TerminalEmulator` / `WebBrowser` category.
- **Pure entry merge** (`shell/src/dockmodel.{h,cpp}`). `buildDockEntries()`
  takes the pinned ids, the index, the shell's running projection, and the
  transient launch states, and returns the ordered entry list (pinned in order,
  then unmatched temporary running apps, then minimized windows). A running
  app that resolves to a pinned id collapses into the pinned entry; the pinned
  entry carries the compositor `appId` for activation. A pinned id that does
  not resolve becomes a `missing:true` generic entry.
- **Shell wiring** (`shell/src/shellcontroller.{h,cpp}`). The controller scans
  the index and loads/seeds pins at startup, stores the running projection,
  and rebuilds the merged entries on every change. The click tree:
  running → `activateApp(appId)`; minimized → `activateApp(appId)` (restores
  via the compositor); not running + launchable → `QProcess::startDetached`;
  missing → logged, no launch. A launch sets a `launching` state, coalesces a
  second click, resolves to running on the first window, and a bounded 8 s
  timer turns a windowless launch into a transient `failed` badge (cleared
  after 4 s). The real notification surface is T-25.
- **Dock presentation** (`shell/dock/DockEntry.qml`). New `launching` (dim),
  `failed`/`missing` (a `statusBadge` dot in `danger`/`warning`), and
  state-bearing accessible names ("launching", "failed to launch",
  "not found").
- **Build/tests.** The pure core is a static library
  `dragonfruit-shell-dockcore` (no Wayland) linked by the shell and by the new
  `tst_dockcore` (QtTest, 12 cases: parsing, resolution/precedence, Exec
  expansion, pins round-trip/unknown-key preservation/defaults, entry merge).
  `tst_dock` gained launching/failed/missing cases. `find_package(Qt6 ... Test)`
  was added for `Qt6::Test`.

### Gotchas learned (important for the next slice)

- **The desktop-name gate forbids distro names in code.** `check-desktop-names.sh`
  rejects whole-word `gnome|kde|plasma|xfce|...` anywhere in `.cpp/.h/.qml`.
  Do **not** hardcode candidate `.desktop` ids like `org.kde.konsole.desktop`
  or executables like `gnome-terminal`; pick defaults by the freedesktop
  `Categories` (`TerminalEmulator`, `WebBrowser`) and skip `NoDisplay`. Test
  fixtures must use neutral ids (`org.example.*`).
- **The settings file shape is invented here.** T-15 must either adopt
  `{"schema":1,"keys":{"<dotted key>":...}}` or add a migration; the Dock
  preserves unknown keys so a second writer is safe. The file is
  `settings.json` (not per-topic), so T-15 owns the same file the Dock seeds.
- **The index is scanned once at startup.** Install/uninstall is not observed
  (T-23 pushes `installed`/`uninstalled` events). A pinned app installed after
  the shell starts stays `missing` until restart.
- **Default browser is the first `WebBrowser` entry, not the `mimeapps.list`
  default.** `extlinks.desktop` (anaconda's `NoDisplay=true` helper) was the
  first hit before the `NoDisplay` filter. A future improvement is to read the
  `x-scheme-handler/http` default; not needed for the vertical slice.
- **`QProcess::startDetached` cannot see an app that starts and exits.** Only
  "start failed" is immediate; "started but produced no window" is caught by
  the 8 s timeout. When T-23's activation/`attention` event lands, that
  becomes the authoritative signal (FR-4) and the self-timer should go.
- **Activation needs the compositor `appId`, not the desktop id.** A running
  pinned entry stores `appId` = the compositor identity (e.g.
  `org.dragonfruit.Files`); the launch path uses `desktopId`. Keep the two
  distinct when extending the click tree.
- **`find_package(Qt6 ... Test)` is now required** at the root for the new
  QtTest target; QuickTest alone did not expose `Qt6::Test` as a link target.

### Hand-off / open items (remaining T-10 slices)

1. **Context menus and the window chooser** (sections 13/9). The Dock emits
   `entryContextMenuRequested`/`dividerContextMenuRequested`; build the
   design-system `ContextMenu` on the existing `overlay` popup pattern. The
   chooser needs a per-app window list (the projection has it) and
   `df_toplevel.activate`/`unminimize`. The new merge keeps `windows` count
   and `appId` per entry, so the chooser can reuse them.
2. **Drag rearrangement and external drops** (section 12): reorder pinned
   (`DockPins::move` already exists), promote-to-pinned, remove-from-Dock,
   file/app drops.
3. **Trash** (section 16): GVfs `trash://` `GFileMonitor`, click-to-Files,
   drop-to-trash via GIO, Empty Trash with confirmation.
4. **Live `dock.*` settings** (section 19): observe settingsd keys (T-15),
   live re-layout, position/auto-hide surface changes, T-16 pane hooks. The
   Dock currently reads defaults from QML tokens and the seeded pins only.
5. **Attention/launch bounce** (section 8.1): `onManagerAttention` is still a
   no-op in `shellprotocol.cpp`; wire it to an `attention` entry state and a
   compositor-clock bounce. Replace the 8 s self-timer with the attention
   signal (FR-4).
6. **Magnified-band input region**: set the region to the bar plus the
   currently magnified icon rectangles each frame (currently bar-only).
7. **Per-output / per-position surfaces**: left/right reserved zones need the
   compositor `LayerSurfaceState::reserved()` extension; per-output sizing is
   the known T-09/T-11/T-16 limitation.
8. **Scene-graph render path** (FR-14): still commits on demand via
   `grabWindow`; the durable `QQuickWindow::afterRendering` fix is shared with
   the T-09 deferred-polish backlog.
9. **Keyboard + AT-SPI walkthrough** (section 20): Fn-Control-F3 focus, arrow
   navigation, Super+Option+D; the live AT-SPI dump is unbuilt.

### Gate status

`make lint` green (`cargo fmt --check`, `clippy -D warnings`, ctest 13/13 incl.
`tst_dockcore`/`tst_dock`/`qmllint_shell-dock`, `gen-tokens --check`, the
design-token/desktop-name/no-capture gates, the 66-snapshot gallery
regression); `make e2e` green (15/15 `shell_protocol_conformance` plus the
window/Xwayland/idle suites); live headless smoke
(`dragonfruit dev --headless --shell`) reports `Dock configured 1280x95` and
`output reserved zone edge=1 thickness=60`, and seeds
`~/.config/dragonfruit/settings.json` with the resolved defaults.

## T-10 continuation — launch/attention bounce + magnified-band input region (third slice)

**State: partial.** The Dock now animates its launch/attention bounce and
publishes a real input region; the remaining slices (context menus/chooser,
drag, Trash, live settings, per-output surfaces, scene-graph render path, a11y)
are unchanged and listed at the end.

### What landed

- **Attention is routed to the Dock** (FR-4). `ShellProtocol::onManagerAttention`
  resolves the toplevel's `app_id` and emits `attentionRequested(appId)`; an
  id whose `app_id` has not arrived yet is ignored, never a crash.
  `ShellController` starts a 2 s attention bounce for the app, stops it on
  entry click or on `focusedAppChanged` (the window gaining focus), and
  expires it on the animation tick.
- **Bounce clocks are pure and unit-tested** (`dockmodel.{h,cpp}`):
  `dockLaunchBouncePhase(elapsed)` is three hops over `kLaunchBounceMs` (600 ms)
  and returns `-1` once finished; `dockAttentionBouncePhase(elapsed)` repeats
  one hop every `kAttentionBounceHopMs` (400 ms). The shell maps the hop phase
  to `sin(pi * phase)`, so each hop leaves the bar and returns. `tst_dockcore`
  covers both clocks (start/boundary/peak/done).
- **The shell drives a 16 ms Dock animation clock** (`m_dockAnimTimer`) while a
  launch or attention bounce is in flight and stops it when both are done, so
  the idle Dock still contributes zero wakeups (FR-8). `withBounce()` injects a
  per-entry `bounce` phase and `attention` flag after the pure entry merge; the
  launch bounce keeps settling after the first window resolves because
  `m_launchStart` is separate from the transient `launch` state, so the entry
  never snaps mid-flight.
- **Launch and attention bounce are distinguishable**: amplitude `B/4` vs
  `B/2`, single 0.6 s vs repeating 2 s (FR-4).
- **Magnified-band input region is real** (FR-13). `Dock.qml` publishes
  `inputRects`: the visible bar plus every entry whose icon is magnified beyond
  baseline or is currently bouncing; an auto-hidden Dock publishes `[]`.
  `ShellProtocol::setDockInputRegion` now takes a `QList<QRect>` and adds every
  rectangle to one `wl_region`; `renderDock` converts `inputRects` each frame.
  This closes the old bar-only limitation where the upper half of a magnified
  icon passed clicks through.
- **Reduced motion** (FR-11): the translation is removed (`entryBounce` returns
  0) and the state stays legible as a subtle scale pulse on the artwork;
  `tst_dock` resets `Theme.reducedMotion` in `init()` so a mid-test failure
  cannot leak the global into the next case.
- **Accessibility**: an attending entry's name gains ", needs attention".

### Gotchas learned (important for the next slice)

- **`Theme.reducedMotion` is a process-wide singleton.** A QML test that sets
  it must restore it even on failure; `TestCase.init()` is the reliable hook
  (`tst_dock.qml` now resets it before every test). The shell does not yet bind
  it to a settings key — that is T-15/T-16.
- **The bounce translation belongs in the Dock's `layout`, not the entry.** The
  shell's input region is derived from `layout`, so applying the bounce inside
  `DockEntry` would desync the hit area from the artwork. `DockEntry` only owns
  the reduced-motion pulse.
- **`inputRects` must be computed after `barRect`.** QML `readonly property`
  binding order does not matter, but the rects are unioned with `barRect`, so
  keep the two properties adjacent for clarity.
- **The 16 ms clock still uses `grabWindow()`** (software render of the whole
  Dock scene per frame). It is fine for a bounded 0.6–2 s bounce, but the
  durable `QQuickWindow::afterRendering` path (FR-14) is still required before
  magnification at 60 Hz can be claimed; do not measure FR-2 against this clock.
- **Attention needs `app_id` to have arrived.** The manager announces the
  toplevel and its `app_id` before an `xdg-activation` in practice; if a future
  client activates before announcing, the bounce is dropped. A pending-attention
  map keyed by `df_toplevel*` would close that, if it ever matters.
- **A multi-rect `wl_region` is standard Wayland**, so no compositor change was
  needed; `empty_input_region_passes_clicks_through` already proves the
  compositor honors input regions on chrome.

### Hand-off / open items (remaining T-10 slices)

1. **Context menus and the window chooser** (sections 13/9). The Dock emits
   `entryContextMenuRequested`/`dividerContextMenuRequested`; build the
   design-system `ContextMenu` on the existing `overlay` popup pattern. The
   chooser needs a per-app window list (the projection has counts, not titles)
   and `df_toplevel.activate`/`unminimize`.
2. **Drag rearrangement and external drops** (section 12): reorder pinned
   (`DockPins::move` already exists), promote-to-pinned, remove-from-Dock,
   file/app drops.
3. **Trash** (section 16): GVfs `trash://` `GFileMonitor`, click-to-Files,
   drop-to-trash via GIO, Empty Trash with confirmation.
4. **Live `dock.*` settings** (section 19): observe settingsd keys (T-15), live
   re-layout, position/auto-hide surface changes, T-16 pane hooks, and bind
   `Theme.reducedMotion` to the accessibility setting.
5. **Per-output / per-position surfaces**: left/right reserved zones need the
   compositor `LayerSurfaceState::reserved()` extension; per-output sizing is
   the known T-09/T-11/T-16 limitation.
6. **Scene-graph render path** (FR-14): still commits on demand via
   `grabWindow`; the durable `QQuickWindow::afterRendering` fix is shared with
   the T-09 deferred-polish backlog. This is the prerequisite for the FR-2
   60 Hz magnification budget and should replace the 16 ms bounce clock too.
7. **Keyboard + AT-SPI walkthrough** (section 20): Fn-Control-F3 focus, arrow
   navigation, Super+Option+D; the live AT-SPI dump is unbuilt.

### Gate status

`make lint` green (`cargo fmt --check`, `clippy -D warnings`, ctest 13/13 incl.
`tst_dockcore`/`tst_dock`/`qmllint_shell-dock`, `gen-tokens --check`, the
design-token/desktop-name/no-capture gates, the 66-snapshot gallery
regression); `make e2e` green (15/15 `shell_protocol_conformance` plus the
window/Xwayland/idle suites); live headless smoke
(`dragonfruit dev --headless --shell`) reports `Dock configured 1280x95` and
`output reserved zone edge=1 thickness=60`.

## T-10 continuation — app context menus + window chooser (fourth slice)

**State: partial.** The Dock's app-entry context menus and window chooser are
landed (FR-5, FR-10 app menus). The remaining slices (divider/Trash menus and
Options, drag rearrangement, Trash state, live settings, per-output surfaces,
scene-graph render path, a11y) are unchanged and listed at the end.

### What landed

- **Per-window projection** (`shellprotocol.{h,cpp}`). Each running app entry
  now carries a `windowList` (`windowId`, `title`, `minimized`, `focused`,
  `workspaceIndex`/`workspaceName`) across all Spaces, ordered most-recent-
  first (focused first, then reverse announcement order). The window handle is
  the `df_toplevel` pointer serialized as a **decimal string** (a 64-bit
  pointer does not survive a QML `number`). Minimized-window entries carry
  their app's full `windowList` so the owning app's menu/chooser works from
  the minimized entry too. Workspace index/name are tracked from the
  `df_workspace` `index`/`name` events; `onManagerFocused` re-emits the
  projection so the chooser's checkmark tracks focus.
- **Window actions** (`ShellProtocol`). `selectToplevel(windowId)` calls
  `df_toplevel_manager.select_overview_toplevel` — the compositor restores a
  minimized window, activates its Space, and focuses it in one round-trip
  (FR-5). `closeToplevel` closes one window; `closeApp(appId)` closes every
  window of an app (the interim `Quit`, since the private protocol has no
  app-level quit).
- **Dock popover overlay surface** (`ShellProtocol::createDockPopupSurface`).
  A second `overlay` layer surface, namespace `dock-popup`, anchored
  `bottom|left`, `exclusive_zone = -1`, keyboard `none`, placed with a bottom
  margin via `setDockPopupGeometry(x, bottomMargin, w, h)`. Pointer events on
  it are routed through new `dockPopupPointer*` signals.
- **Popover rendering** (`ShellController::renderDock`). The shell reads the
  Dock scene's `popoverRect`, grows the offscreen window upward by the
  popover's headroom, pushes the Dock item down by that offset so scene y=0
  stays the surface top, commits the bar from `(0, headroom, w, m_dockHeight)`
  and the popover from `(px, headroom+py, pw, ph)`. Popover-local pointer
  coordinates are translated back into scene coordinates (plus the headroom)
  before they reach the offscreen window. A 16 ms `m_dockPopupTimer` burst
  captures the open/close fade (the durable FR-14 path is still deferred).
- **Dock presentation** (`shell/dock/Dock.qml`, new `DockWindowChooser.qml`).
  A right-click opens a design-system `ContextMenu` above the entry (beside it
  on a vertical Dock), with the live window list (checkmark on frontmost,
  "(minimized)" and the Space name), Show All Windows, Keep/Remove from Dock,
  Quit, and Open. A plain click on a running app with more than one window
  opens the window chooser (a custom popover with a downward arrow) instead of
  activating; one window activates directly. Both popovers suppress
  magnification (section 14), dismiss on Escape / empty-Dock click, and expose
  the popover rectangle the shell renders. `menuActionRequested(action,
  payload)` and `windowActivated(windowId)` drive the shell.
- **Build/tests.** `tst_dock` gains 8 cases (menu open + magnification
  suppression, menu model content, Keep vs Remove, window-activation action,
  multi-window click opens the chooser, single-window click does not, chooser
  accessible names, click-away dismiss). `tst_dockcore` asserts the pin merge
  preserves `windowList`. `shell_protocol_conformance` gains
  `dock_popup_is_bottom_anchored_and_reserves_nothing` (bottom|left anchor,
  bottom margin, no reserved-zone growth, popover-local pointer coordinates on
  the 1280x720 headless output).

### Gotchas learned (important for the next slice)

- **A 64-bit window handle cannot round-trip through QML as a number.** JS
  numbers are doubles, so pointer values above 2^53 lose precision. The
  projection carries `windowId` as a decimal string and the shell parses it
  back with `toULongLong`.
- **A bottom-anchored popover is placed with a bottom margin, not a top one.**
  The Dock popup surface anchors `bottom|left` and the shell computes
  `bottomMargin = m_dockHeight - (sceneY + height)`, so it never needs the
  output height. The existing menu-bar popup anchors `top|left` and uses the
  top margin.
- **Growing the offscreen Dock scene upward needs an item offset.** The window
  is `m_dockHeight + headroom` tall and the Dock item is placed at
  `y = headroom`; the Dock surface commit takes the bottom `m_dockHeight`
  rows. Every pointer event delivered on the Dock surface must add `headroom`
  to reach window coordinates, or hover/hit-testing is off by the popover
  height whenever a popover is open.
- **`popoverRect` must key on `open || visible`, not `visible` alone.** The
  `ContextMenu`/chooser fade in from opacity 0 (`visible: opacity > 0`), so a
  `visible`-only rect is empty on the first frame and the shell would never
  map the overlay surface; `open` keeps the rect valid while the fade plays
  and `visible` covers the close tail.
- **`df_toplevel_manager.select_overview_toplevel` already does the whole
  selection.** `activate_window_id` restores a minimized window, activates its
  Space, and sets keyboard focus, so the chooser needs no separate
  `unminimize`/`activate_workspace` calls.
- **`onManagerFocused` now calls `emitDockState()`.** Previously it only
  emitted `focusedAppChanged`; the chooser's frontmost checkmark needs the
  focus change in the projection.
- **The divider menu and the Trash menu are not implemented.** The divider
  Control-click still logs; its toggles write settings, so they belong with
  the live-settings slice. `Options ▸`, Show in Files, Empty Trash, and the
  Downloads stack remain deferred.
- **`Quit` closes every window of the app.** There is no app-level quit in the
  private protocol; a process that outlives its last window would survive
  (the same window-vs-process caveat as section 4.1).

### Hand-off / open items (remaining T-10 slices)

1. **Drag rearrangement and external drops** (section 12): reorder pinned
   (`DockPins::move` already exists), promote-to-pinned, remove-from-Dock,
   file/app drops, live gap animation. `windowList`/`desktopId` are on every
   entry now.
2. **Trash** (section 16): GVfs `trash://` `GFileMonitor`, click-to-Files
   (`org.dragonfruit.Files1`), drop-to-trash via GIO, Empty Trash with
   confirmation, and the Trash context menu (the Dock popup surface now
   exists).
3. **Live `dock.*` settings** (section 19): observe settingsd keys (T-15),
   live re-layout, the divider menu toggles (magnification, hiding, position),
   position/auto-hide surface changes, T-16 pane hooks, and binding
   `Theme.reducedMotion` to the accessibility setting.
4. **Divider and Trash context menus + Options submenu** (section 13): the
   divider menu (Turn Magnification/Hiding On/Off, Position, Dock Settings…)
   and the app Options submenu (Assign To, Open at Login, Show in Files).
5. **Per-output / per-position surfaces**: left/right reserved zones need the
   compositor `LayerSurfaceState::reserved()` extension; per-output sizing is
   the known T-09/T-11/T-16 limitation. The Dock popup surface would need its
   own anchor for a left/right Dock.
6. **Scene-graph render path** (FR-14): the Dock still commits on demand via
   `grabWindow`; the durable `QQuickWindow::afterRendering` fix is shared with
   the T-09 deferred-polish backlog and should replace the bounce and popover
   timers.
7. **Keyboard + AT-SPI walkthrough** (section 20): Fn-Control-F3 focus, arrow
   navigation, Super+Option+D; the QML roles exist, the seat path and the live
   AT-SPI dump are not built.

### Gate status

`make lint` green (`cargo fmt --check`, `clippy -D warnings`, ctest 13/13 incl.
`tst_dockcore`/`tst_dock`/`qmllint_shell-dock`, `gen-tokens --check`, the
design-token/desktop-name/no-capture gates, the 66-snapshot gallery
regression); `make e2e` green (16/16 `shell_protocol_conformance` plus the
window/Xwayland/idle suites); live headless smoke
(`dragonfruit dev --headless --shell`) reports `Dock configured 1280x95` and
`output reserved zone edge=1 thickness=60` with the `dock-popup` overlay
surface created cleanly.

## T-10 continuation — drag rearrangement (fifth slice)

**State: partial.** Pinned entries can be reordered, promoted, and removed by
dragging (FR-9, section 12). The remaining slices (external drops, GVfs Trash,
live settings, divider/Trash/Options menus, per-output surfaces, scene-graph
render path, a11y) are unchanged and listed at the end.

### What landed

- **Drag model** (`shell/dock/Dock.qml`). `beginDrag`/`dragTo`/`dropAt`/
  `endDrag`/`dragPointerLeft` own the tentative reorder. The target insertion
  index is computed against `dragBaseCenters` (the pre-drag pinned slot
  centers, excluding the dragged entry), so the live layout cannot feed back
  into the target. The app slots are permuted in `layout` via `appSlot()`
  rather than reordering the Repeater model, so the delegate holding the
  pointer survives; neighbours animate into the gap with
  `Theme.motion.dockMagnify` (reduced motion → 0).
- **Promote / remove** (section 12). A temporary running app dropped in the
  pinned region is promoted; a pinned entry dragged outside the bar (or off
  the surface) shows a "Remove" label and is removed. On drop the Dock emits
  `pinnedOrderChanged(desktopIds)` with the complete ordered pinned set; the
  shell (`ShellController::onDockPinnedOrderChanged`) writes it to
  `dock.pinned` and rebuilds. Cross-output drag is not supported (leaving the
  surface finalizes the drag).
- **Entry gesture** (`shell/dock/DockEntry.qml`). A `DragHandler` (threshold
  8) reports scene coordinates via `dragBegan`/`dragMoved`/`dragEnded`; the
  entry scales up (`liftScale`) and casts a `Shadow` while lifted. The divider,
  minimized windows, and Trash are not draggable.
- **Promotion needs a resolved desktop id.** `buildDockEntries` now resolves a
  temporary running app's `appId` through the interim index and carries
  `desktopId`/`icon`/`name` so the Dock can build the new pin order.
- **Tests.** `tst_dock` gains 12 cases (reorder, first-to-end, no-op,
  live-gap visual order, promote at end/before first, out-of-dock remove,
  out-of-dock keeps a temporary, pointer-leave remove, magnification
  suppression, lifted entry + shadow, and a real `mousePress`/`mouseMove`/
  `mouseRelease` drag through the `DragHandler`). `tst_dockcore` asserts the
  temporary entry carries `desktopId`.

### Gotchas learned (important for the next slice)

- **Do not reorder the Repeater model during an active drag.** A `Repeater`
  over a JS array resets its delegates on any change, destroying the
  `DragHandler` that holds the pointer; the drag then ends silently. The fix
  is to keep the model stable and permute the layout slots instead
  (`appSlot()`), leaving the gap to the `Behavior on x/y`.
- **QtTest drag simulation needs the button argument.** `mouseMove(item, x, y,
  delay, Qt.LeftButton)` (and the release) is required for the handler to stay
  active; without it the `DragHandler` never deactivates and `dragEnded` never
  fires. A press/move/move/release sequence with a move past the threshold is
  the reliable shape.
- **The pointer-leave path finalizes a drag.** When the pointer leaves the
  Dock surface the shell sends a synthetic move to `(-1,-1)`; the Dock's
  `HoverHandler.onHoveredChanged` then calls `dragPointerLeft()`, which removes
  a pinned entry (drag-out) or snaps a temporary back. The eventual
  `mouseRelease` finds `dragging` already false and is ignored.
- **`dragOutside` and `dragOutOfDock` are distinct.** `dragOutside` (pointer
  past the bar bounds) applies to any entry; `dragOutOfDock` (removal intent)
  only to pinned. Conflating them promoted a temporary dragged far to the left.
- **The full pinned set is the wire format.** Rather than separate
  reorder/promote/remove signals, the Dock emits the complete ordered pinned
  id list and the shell sets it; a no-op drag emits nothing because the list is
  compared before signalling.
- **External drops are not implemented.** A file/app dropped from Files or a
  launcher, and spring-loading, need the drag sources (T-17/T-18) and the
  compositor XDnD/file-drop plumbing; the Dock currently owns only its own
  internal drag.

### Hand-off / open items (remaining T-10 slices)

1. **External drops and spring-loading** (section 12): an app dropped from
   Files/a launcher pins it; a file/folder dropped on an app icon opens it with
   that app; a file dropped on Trash trashes it; a file dropped on the
   Downloads stack moves it. Needs the drag source plus `app-index`/GIO launch
   with a file argument.
2. **Trash** (section 16): GVfs `trash://` `GFileMonitor`, click-to-Files
   (`org.dragonfruit.Files1`), drop-to-trash via GIO, Empty Trash with
   confirmation, and the Trash context menu.
3. **Live `dock.*` settings** (section 19): observe settingsd keys (T-15),
   live re-layout, the divider menu toggles (magnification, hiding, position),
   position/auto-hide surface changes, T-16 pane hooks, and binding
   `Theme.reducedMotion` to the accessibility setting.
4. **Divider and Trash context menus + Options submenu** (section 13): the
   divider menu (Turn Magnification/Hiding On/Off, Position, Dock Settings…)
   and the app Options submenu (Assign To, Open at Login, Show in Files).
5. **Per-output / per-position surfaces**: left/right reserved zones need the
   compositor `LayerSurfaceState::reserved()` extension; per-output sizing is
   the known T-09/T-11/T-16 limitation. The Dock popup surface would need its
   own anchor for a left/right Dock.
6. **Scene-graph render path** (FR-14): the Dock still commits on demand via
   `grabWindow`; the durable `QQuickWindow::afterRendering` fix is shared with
   the T-09 deferred-polish backlog and should replace the bounce and popover
   timers.
7. **Keyboard + AT-SPI walkthrough** (section 20): Fn-Control-F3 focus, arrow
   navigation, Super+Option+D; keyboard reordering is an explicit later
   accessibility enhancement. The QML roles exist; the seat path and the live
   AT-SPI dump are not built.

### Gate status

`make lint` green (`cargo fmt --check`, `clippy -D warnings`, ctest 13/13 incl.
`tst_dockcore`/`tst_dock`/`qmllint_shell-dock`, `gen-tokens --check`, the
design-token/desktop-name/no-capture gates, the 66-snapshot gallery
regression); `make e2e` green (16/16 `shell_protocol_conformance` plus the
window/Xwayland/idle suites); live headless smoke
(`dragonfruit dev --headless --shell`) reports `Dock configured 1280x95` and
`output reserved zone edge=1 thickness=60`.

## T-10 continuation — live settings, divider menu, reduced motion (sixth slice)

**State: partial.** The Dock's `dock.*` keys are now a live settings model
with a divider options menu and a global reduced-motion binding (FR-12,
section 19); the compositor's reserved-zone resolver now understands
left/right edges (the foundation for a vertical Dock). The remaining slices
(external drops, Trash, per-position surfaces/auto-hide reveal, Options
submenu, scene-graph render path, a11y) are unchanged and listed at the end.

### What landed

- **`DockSettings`** (`shell/src/docksettings.{h,cpp}`, in the Wayland-free
  `dragonfruit-shell-dockcore` static lib). Owns every non-pinned `dock.*`
  key plus `accessibility.reduceMotion`; reads/writes the same
  `$XDG_CONFIG_HOME/dragonfruit/settings.json` in the eventual
  `org.dragonfruit.Settings1` named-key shape. Values are clamped
  (`dock.size`/`dock.magnification` to 0..1) and validated (`position`,
  `minimizedAnimation`, `titlebarDoubleClick` fall back to their defaults on
  an unknown string). `save()` re-reads the file and merges, so it and
  `DockPins` cannot clobber each other's keys; `equals()` lets the shell
  ignore the file-watch notification for its own write. `tst_dockcore` gains
  4 cases (defaults, round-trip/clamping/validation, shared-file interleave
  both orders, equals).
- **`DockPins::save()` re-reads** the file before writing (previously it
  wrote its stale in-memory root, which would have dropped a `dock.size`
  written after startup). The existing `pinsRoundTripAndPreserveUnknownKeys`
  still passes; `settingsAndPinsShareTheFileWithoutClobbering` covers the new
  guarantee.
- **Live apply** (`ShellController::applyDockSettings`). At startup the
  saved settings are applied *before* the chrome surface exists (no
  reconfigure), so `m_dockBarThickness`/`m_dockHeight` reflect `dock.size`.
  A `QFileSystemWatcher` on the settings file is the interim stand-in for
  settingsd's change signals (T-15); on a change the shell reloads, applies,
  and (for `dock.size`/`dock.autohide`) reconfigures the surface in place via
  the new `ShellProtocol::configureDockSurface` (send `set_size` +
  `set_exclusive_zone`, which the compositor already honors live). Verified
  in a live headless session: `dock.size` 0.5→1.0 reconfigured the Dock to
  `1280x159` and the bottom reserved zone to 76 (was `1280x124`/60).
- **Magnification value.** `dock.magnification` (0..1) now maps onto the peak
  icon factor: `1 + magnification * (magnifyPeakMax - 1)`, so the default 0.5
  lands on the existing `magnifyPeak` token (1.6) and 1.0 on the new
  `magnifyPeakMax` (2.2). `magnifyBand` is sized for `magnifyPeakMax` so a
  live change never needs to grow the scene. New tokens
  `component.dock.iconSizeMin` (32)/`iconSizeMax` (64)/`magnifyPeakMax`
  (2.2); `dock.size` maps across the icon range (default 0.5 → 48).
- **Divider menu** (`Dock.qml`). Control-clicking the divider now opens the
  design-system `ContextMenu` (it previously only emitted a signal) with the
  live "Turn Magnification On/Off" toggle and "Dock Settings…". The shell
  writes `dock.magnification` (0.5 when re-enabling) and saves.
- **Reduced motion.** `accessibility.reduceMotion` is set on the
  design-system `Theme.reducedMotion` singleton through
  `QQmlEngine::singletonInstance("Dragonfruit", "Theme")`, so every shell
  surface reacts, not just the Dock. `dock.animateOpening=false` suppresses
  the launch hop while an attention bounce still plays.
- **Compositor left/right reserved zones.** `LayerSurfaceState::reserved()`
  now returns `Edge::Left`/`Edge::Right` for a single horizontal anchor
  (checking top/bottom first, so a bottom Dock's `bottom|left|right` anchor
  never leaks into a side reserve). New unit test plus
  `vertical_dock_reserves_the_left_and_right_zones` in
  `shell_protocol_conformance` (edge 2/edge 3 reported for left/right
  docks). `aggregate_reserved` and `ReservedZones::usable` already handled
  left/right.

### Gotchas learned (important for the next slice)

- **The chrome protocol already supports live reconfigure.** `set_anchor`,
  `set_size`, `set_margin`, and `set_exclusive_zone` are honored after
  creation and trigger a fresh configure (`compositor/src/shell/mod.rs`), so
  live settings need no surface recreation — just `configureDockSurface`.
- **`QSaveFile` breaks a `QFileSystemWatcher` watch.** It writes a temp file
  and renames over the target, so the watched path is removed on the first
  write. `onSettingsFileChanged` re-adds the path on the next event-loop
  turn (and the file may briefly not exist).
- **Guard the watcher against the shell's own write.** The shell updates
  `m_settings` before saving, so reloading into a fresh `DockSettings` and
  comparing with `equals()` makes the self-notification a no-op. This
  disappears when settingsd (T-15) becomes the single writer.
- **`reserved()` must prefer the vertical edge.** A bottom Dock anchors
  `bottom|left|right`; checking top/bottom first is what stops it reserving a
  side strip as well. A left/right Dock anchors `left|top|bottom`
  (or `right|…`) and falls through to the horizontal match.
- **The vertical layout is not visually complete.** The QML `barRect` for a
  vertical Dock is placed at `x = magnifyBand` with the transparent band to
  the left, and `hideOffset` is *subtracted*, which moves a bottom Dock up
  rather than down off the edge. Position switching is therefore deliberately
  not wired (the setting is persisted and warned about, not applied); the
  vertical geometry and auto-hide translation belong to the per-position
  slice.
- **Auto-hide has no reveal/hide state machine.** `dock.autohide` flips the
  reserved zone to 0 correctly, but nothing calls `Dock.hide()`/`reveal()`
  from pointer dwell, and the hidden input region is empty so the edge band
  cannot be re-entered. The divider menu deliberately omits the hiding
  toggle until the edge-band reveal lands.
- **The divider menu is flat.** The design-system `ContextMenu` renders a
  `submenu` row (chevron) but has no submenu open logic, so "Position on
  Screen ▸" and the app "Options ▸" submenu are still deferred; when they
  land they should use the T-09 drag-through rule.

### Hand-off / open items (remaining T-10 slices)

1. **External drops and spring-loading** (section 12): an app dropped from
   Files/a launcher pins it; a file/folder dropped on an app icon opens it
   with that app; a file dropped on Trash trashes it; a file dropped on the
   Downloads stack moves it. Needs the drag source plus `app-index`/GIO
   launch with a file argument (T-17/T-18).
2. **Trash** (section 16): GVfs `trash://` monitor, click-to-Files
   (`org.dragonfruit.Files1`), drop-to-trash via GIO, Empty Trash with
   confirmation, and the Trash context menu. Note: this build host has only
   the libgio runtime (no GIO dev headers/pc), so the backend needs either
   the headers or a filesystem-watcher fallback over
   `$XDG_DATA_HOME/Trash/{files,info}`.
3. **Per-position surfaces** (section 5): apply `dock.position` (left/right)
   with vertical surface geometry, horizontal popover gutters/anchor, and the
   corrected auto-hide translation direction; then add the auto-hide
   edge-band reveal/re-hide state machine and re-add the divider hiding
   toggle. The compositor `reserved()` left/right support is already in.
4. **Options submenu + Show in Files + Trash/divider completeness** (section
   13): the app Options ▸ submenu (Assign To, Open at Login, Show in Files)
   and the Trash menu. Needs the design-system submenu, T-18 Files, and
   compositor app/space requests.
5. **Per-output / per-output sizing** (section 18): chrome surfaces still
   size from the first output; `matches_output` is the filter hook.
6. **Scene-graph render path** (FR-14): the Dock still commits on demand via
   `grabWindow`; the durable `QQuickWindow::afterRendering` fix is shared with
   the T-09 deferred-polish backlog and should replace the bounce and popover
   timers.
7. **Keyboard + AT-SPI walkthrough** (section 20): Fn-Control-F3 focus, arrow
   navigation, Super+Option+D; keyboard reordering is an explicit later
   accessibility enhancement. The QML roles exist; the seat path and the live
   AT-SPI dump are not built.

### Gate status

`make lint` green (`cargo fmt --check`, `clippy -D warnings`, ctest 13/13 incl.
`tst_dockcore`/`tst_dock`/`qmllint_shell-dock`, `gen-tokens --check`, the
design-token/desktop-name/no-capture gates, the 66-snapshot gallery
regression); `make e2e` green (17/17 `shell_protocol_conformance` incl. the
new vertical-dock reserve test, plus the window/Xwayland/idle suites); live
headless smoke (`dragonfruit dev --headless --shell`) reports
`Dock configured 1280x124` and `output reserved zone edge=1 thickness=60`,
and a live `dock.size` 0.5→1.0 edit reconfigured the Dock to `1280x159` with
`edge=1 thickness=76`.

### Follow-up fix — Dock icons froze in the left corner

The drag slice (fifth) added an always-on `Behavior on x`/`y` to the Dock
entry delegates so the reorder gap springs open. The shell creates the Dock
offscreen scene at width 0, sets `entries`, and only then receives the
surface configure that sets the real width. The delegates therefore animated
from the width-0 positions toward the centered layout — but the idle shell
commits on demand and stops rendering after the startup burst, so the
committed frame froze the icons mid-flight near the left edge (the bar, whose
position is not animated, stayed centered, which made it look like the icons
had detached).

Fix: the position `Behavior` is now enabled **only while `dock.dragging` and
not the dragged delegate** (`Dock.qml`). Every other layout change — initial
configure, live resize/magnification, popover — snaps, so the on-demand
renderer can never freeze a half-played animation. A new
`test_entries_snap_to_layout_after_configure` in `tst_dock` reproduces the
shell's width-0 → entries → configure order and asserts every delegate's
`x`/`width` equals `layout[i]`. Verified live: the committed Dock image now
shows the icons centered (`560..659`) inside the centered bar (`562..718`).

Lesson for the scene-graph render path (FR-14): any QML animation on Dock
geometry is invisible unless the shell keeps committing frames, so prefer
snap-to-target outside explicitly animated interactions.

## T-10 continuation — per-position (vertical) surfaces (seventh slice)

**State: partial.** `dock.position` (`bottom` | `left` | `right`) now actually
moves the Dock: the shell anchors and sizes the `dock` layer surface (and its
`dock-popup` overlay) to the configured edge, and the QML layout mirrors for a
vertical Dock. Live position changes work without a surface recreate, and the
auto-hide translation now goes off the correct edge. The auto-hide
reveal/hide state machine is still open (nothing summons the hidden Dock from
the edge). All other remaining slices (external drops, Trash, Options submenu,
per-output sizing, scene-graph render path, a11y) are unchanged and listed at
the end.

### What landed

- **`ShellProtocol` edge-aware Dock surfaces** (`shellprotocol.{h,cpp}`). New
  `DockPosition { Bottom, Left, Right }`; `createDockSurface(position,
  thickness, exclusiveZone)` and `configureDockSurface(...)` anchor
  `bottom|left|right` (size `0 x thickness`) for a bottom Dock and
  `left|top|bottom` / `right|top|bottom` (size `thickness x 0`) for a vertical
  one; `configureDockSurface` re-applies the anchor so a live position change
  moves the surface. `createDockPopupSurface(position)` anchors the overlay to
  the Dock's edge (`bottom|left`, `left|top`, or `right|top`), and
  `setDockPopupGeometry(top, right, bottom, left, w, h)` now takes full
  margins instead of a bottom margin, so a vertical popover can hug the side.
  `configureDockSurface` also re-anchors an existing popover overlay.
- **Shell per-edge geometry** (`ShellController`). `m_dockThickness` (bar +
  magnify band) and `m_dockPosition` are tracked; `onDockConfigured` validates
  the extent on the correct axis (height for bottom, width for vertical) and
  ignores the compositor's pre-layout full-output configure. A position change
  zeroes the unknown stretch dimension and pauses `renderDock` until the new
  configure arrives, so no stale-aspect frame is committed. `dockPosition()`
  maps the settings string to the enum. `renderDock` grows the offscreen scene
  by a left/right gutter (in addition to the bottom `headroom`) and computes
  the popover overlay margins from the Dock's edge; pointer input is offset by
  both `m_dockItemOffsetX` and `m_dockItemOffsetY`. A live geometry change
  dismisses any open popover (`closePopovers`) so it never floats detached.
- **QML vertical layout** (`shell/dock/Dock.qml`). The bar hugs the anchored
  edge (left `x=0`, right `x=width-barThickness`, both perpendicular offsets
  included); entries pack from that edge with the running indicator against
  the screen edge (`x=padding` for left, right-aligned for right); bounce
  moves into the magnify band. `hideX`/`hideY` split the auto-hide translation
  per axis (bottom hides down, a vertical Dock hides off its side edge);
  `draggedX` right-aligns a lifted right-Dock entry.
- **Tests.** `tst_dock` gained vertical bar/entry containment under full
  magnification (no clipping), the side-edge hide direction, and the
  beside-the-bar popover (left menu to the right of the bar, right menu to its
  left); the left/right placement assertions were tightened. Live headless:
  `dock.position=left` → `Dock configured 124x720`, reserved `edge=2
  thickness=60`; `right` → `edge=3 thickness=60`; and a live bottom→left edit
  reloaded, ignored the pre-layout `1280x124`, then reconfigured to `124x720`
  with the left reserve.

### Gotchas learned (important for the next slice)

- **The compositor re-anchors live.** `df_layer_surface.set_anchor`/`set_size`
  are honored after creation; `configureDockSurface` sends them on every
  change and the compositor answers with a fresh configure. The first
  configure after re-anchoring is the old geometry (`1280x124` when moving
  bottom→left); the axis check in `onDockConfigured` correctly ignores it.
- **Popover placement needs the Dock's *edge*, not output coordinates.** The
  popover rect lives in Dock-scene coordinates (scene x=0 = surface left). For
  a right Dock the popover can have a negative scene `x`; the shell grows a
  left gutter and translates the scene inside the offscreen window, but the
  overlay margins are computed from the un-offset scene rect. For a right
  Dock the right margin is `m_dockWidth - (px + pw)`; a left-anchored overlay
  would wrongly place it off the output's left edge.
- **Anchor the popover `top|left`/`top|right`, not bottom.** A vertical Dock
  spans the full output height, so a top margin (`py`) places the popover
  without needing the output height; the bottom-anchored bottom-Dock popover
  uses `m_dockHeight - (py + ph)` from the surface bottom (= output bottom).
- **A vertical Dock's surface is only `thickness + magnifyBand` wide.** A
  context menu is wider than that, so the offscreen scene must grow
  horizontally; without the gutter `grabWindow().copy()` returns a partially
  empty buffer and the menu is clipped.
- **The QML vertical layout was written but mirrored wrong for `left`.** It
  placed the bar at `x=magnifyBand` for both left and right (band on the
  screen-edge side), so a left Dock's icons grew *into* the edge. The fix is
  to hug the anchored edge and grow inward; the `magnifyBand` is now only a
  sizing input for the surface, not a positional offset.
- **Auto-hide still has no reveal/hide state machine.** The translation
  direction is now correct, but `revealed` defaults true and nothing calls
  `Dock.hide()`/`reveal()` from pointer dwell; the hidden input region is
  empty so the edge band cannot be re-entered either. The divider hiding
  toggle stays omitted until that lands.

### Hand-off / open items (remaining T-10 slices)

1. **Auto-hide edge-band reveal/re-hide state machine** (section 15) plus
   re-adding the divider hiding toggle. The hidden Dock's input region is
   empty, so the compositor needs to report pointer proximity to the edge band
   (or the shell keeps a thin edge input strip) before the Dock can be summoned
   back. `dock.autohide` already flips the reserved zone to 0 and the
   translation is correct.
2. **External drops and spring-loading** (section 12): app/file drops on an
   icon, Trash, or the Downloads stack. Needs the drag source plus
   `app-index`/GIO launch with a file argument (T-17/T-18).
3. **Trash** (section 16): GVfs `trash://` monitor, click-to-Files
   (`org.dragonfruit.Files1`), drop-to-trash via GIO, Empty Trash with
   confirmation, and the Trash context menu. This host has only the libgio
   runtime (no GIO dev headers/pc), so the backend needs either the headers or
   a filesystem-watcher fallback over `$XDG_DATA_HOME/Trash/{files,info}`.
4. **Options submenu + Show in Files + Trash/divider completeness** (section
   13): the app Options ▸ submenu (Assign To, Open at Login, Show in Files)
   and a "Position on Screen" menu entry now that position switching works.
   Needs the design-system submenu, T-18 Files, and compositor app/space
   requests.
5. **Per-output / per-output sizing** (section 18): chrome surfaces still
   size from the first output; `matches_output` is the filter hook. A vertical
   Dock compounds this (the stretch dimension is the output height).
6. **Scene-graph render path** (FR-14): the Dock still commits on demand via
   `grabWindow`; the durable `QQuickWindow::afterRendering` fix is shared with
   the T-09 deferred-polish backlog.
7. **Keyboard + AT-SPI walkthrough** (section 20): Fn-Control-F3 focus, arrow
   navigation, Super+Option+D; the QML roles exist, the seat path and live
   AT-SPI dump are not built.

### Gate status

`make lint` green (`cargo fmt --check`, `clippy -D warnings`, ctest 13/13 incl.
`tst_dockcore`/`tst_dock`/`qmllint_shell-dock`, `gen-tokens --check`, the
design-token/desktop-name/no-capture gates, the 66-snapshot gallery
regression); `make e2e` green (17/17 `shell_protocol_conformance` incl.
`vertical_dock_reserves_the_left_and_right_zones`, plus the
window/Xwayland/idle suites); live headless smoke reports `Dock configured
124x720` / `edge=2 thickness=60` for `dock.position=left`, `edge=3` for
`right`, and a live bottom→left edit reconfigured the surface in place.
