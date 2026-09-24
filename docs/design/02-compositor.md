# Compositor

## Summary

The compositor is the heart of the desktop: a Rust process built on Smithay
that owns windows, workspaces, effects, and shell protocols. It remains
conceptually small — **display, input, surfaces, windows, workspaces, effects,
security boundaries, and shell protocols** — and deliberately excludes
hardware-management and product logic.

## Responsibilities

- Outputs and display configuration (via DRM/KMS/GBM on real hardware)
- Input handling (libinput, xkbcommon)
- Surface and window management: focus, move, resize, zoom, fullscreen,
  transient dialogs, popups (see [Window model](#window-model))
- Workspace model (see [03-workspaces.md](03-workspaces.md))
- Per-Space wallpaper rendering, part of the workspace scene so a Space's
  background slides with it during switches (see
  [03-workspaces.md](03-workspaces.md))
- Effects and animation policy (blur, shadows, scale/clip transforms)
- Security boundaries (seat/session, lock screen integration)
- Screen magnification (accessibility zoom) with follow-focus and
  follow-caret modes — compositor-owned because it transforms the whole
  scene, not one window
- Private shell protocols (see below)
- Xwayland compatibility

## Backends

The compositor is built with multiple backends from day one:

- **DRM/KMS backend** — native, owns the physical display; used for real
  sessions and hardware testing.
- **Wayland (nested) backend** — runs as a normal window inside an existing
  session; the daily development workflow (see
  [11-session-and-dev-workflow.md](11-session-and-dev-workflow.md)).
- **Headless backend** — for CI and automated testing.

Compositor frameworks are designed around reusable backends; we never require
every run to take ownership of the physical display.

## Input

- libinput delivers pointer, keyboard, touch, and tablet events. Touch and
  tablet are first-class at the input layer from the start, not a later port.
- Gesture recognition (swipe, pinch) lives in the compositor and feeds the
  **same** progress pipelines as keyboard and hot-corner triggers — there is
  no gesture-only code path (see [03-workspaces.md](03-workspaces.md)).
- Keymaps are xkbcommon with the Cmd/Super and Option/Alt mapping fixed once
  (see [Keymap conventions](#keymap-conventions)); keyboard repeat,
  per-device pointer acceleration, and scroll configuration are compositor
  settings surfaced through the Settings input panes (see
  [08-settings.md](08-settings.md)).
- The global shortcut engine owns system shortcuts (workspace switching,
  Mission Control, app switcher, screenshots). Application accelerators are
  admitted only through the menu-broker while the owning window is focused
  (see [06-global-menu.md](06-global-menu.md)), and sandboxed applications
  register shortcuts through the portal's GlobalShortcuts interface — no
  client grabs keys directly.

## Renderer and effects

- Rendering goes through Smithay's GBM/EGL renderer stack on the DRM backend.
- Effects (blur, shadows, workspace scale/clip transforms) are compositor
  render passes over live surface buffers — never client re-renders, never
  screenshots, never recompositing by a third-party program.
- Fullscreen surfaces take the direct-scanout path where possible;
  `tearing-control` is honored for fullscreen games (see
  [14-risks.md](14-risks.md)).
- Damage tracking keeps the idle path cheap: no damage, no render, no client
  wakeups. Animations drive damage every frame while running.
- Hardware cursor planes are used where the hardware offers them; software
  cursors are the fallback, and cursor motion never waits on effects or
  damage.
- VRR/adaptive-sync is enabled per output where connector and mode support
  it, and is a Displays-pane toggle (see [08-settings.md](08-settings.md)).
- Night light (per-output gamma ramps) is compositor-owned, like everything
  else about outputs (see [08-settings.md](08-settings.md)).
- Multi-GPU: render on the primary node, import GBM buffers across devices,
  and fall back automatically when a GPU disappears — device *loss* is part
  of the hotplug story, not just display hotplug (see
  [14-risks.md](14-risks.md)).
- Performance budgets are targets, not hopes — see
  [ROADMAP.md](../ROADMAP.md).

## Event loop

One calloop event loop (Wayland clients, libinput, D-Bus, timers), rendering
driven per-output on vblank. Scene updates and GPU work are single-threaded
per output until profiling proves otherwise; correctness beats parallelism in
a display server.

### Animation clock

Every compositor-side lifecycle transition (window appear/minimize/zoom/close,
the overview slide, the scene transforms) runs on **one** shared animation
clock (`compositor/src/animation.rs`). A single frame-scheduled calloop timer
is armed only while an animation is live; each tick advances every running
transition exactly once and requests exactly one compositor frame, and a
settled clock is not armed at all — zero damage, zero client wakeups
(FR-2). `accessibility.reduceMotion` is a single flag on the clock: each
transition reads it when it builds its tween and takes one step through the
same commit path, never a second "instant" path. See
[ADR 0003](adr/0003-shared-animation-clock.md).

#### Idle and animation frame trace (T-03.1a)

The render-path counters emitted on `SIGUSR1`/clean exit are the trace
contract (`compositor/src/state.rs::dump_stats`): `frames_rendered`,
`frames_skipped_no_damage`, `direct_scanouts`, `animation_frames_stepped`
and `client_wakeups`. `client_wakeups` counts the frame-callback batches
handed to mapped windows, so the idle budget is measured directly instead of
inferred from the render count. `idle_trace.rs` asserts every budget counter
flat across a configurable window (`DF_IDLE_TRACE_SECS`) and asserts
`frames_rendered` advances by exactly `animation_frames_stepped` while the
clock's calibration animation runs; `scripts/idle-trace.sh` / `make idle-trace`
records the 60 s acceptance line. See
[ADR 0009](adr/0009-idle-trace-instrumentation.md).

#### Input-to-photon latency and direct scanout (T-03.1b)

The same trace carries two more instruments. `instrument::LatencyInstrument`
measures **input-to-photon latency**: `input::process_input_event` stamps the
earliest input since the last presented frame, and the presenting backend
samples the delta at nested `render::post_repaint`, headless
`render::post_repaint_headless`, or DRM `backend::drm::render_surface`. An
input older than 250 ms when a frame presents is discarded rather than
credited, so a no-damage pointer move cannot inflate a later redraw. The
render-stats line appends `latency_us_last`, `latency_us_max`,
`latency_samples` and `latency_dropped` after `client_wakeups` (append-only,
ADR 0009); `scripts/latency-trace.sh` records the nested raw samples with
`query latency`. `instrument::ScanoutCounter` is the **direct-scanout counter
template**: a read-only snapshot of `direct_scanouts`/`frames_rendered`/
`frames_skipped_no_damage` that the DRM rail snapshots before/after a
condition to prove `direct_scanouts` engages for an unobstructed fullscreen
client and not otherwise (`scanout stats`, `query scanout`). See
[ADR 0010](adr/0010-latency-and-scanout-instruments.md).

#### Window lifecycle motion (T-02.1b appear; T-02.2 minimize/restore; T-02.3 zoom/fullscreen; T-02.4a close; T-02.4b interrupt)

A window scales and fades between an *origin* rectangle and its final
geometry on that clock. For the launch motions the origin is the owning Dock
entry's tile, supplied by the shell over the private protocol
(`df_toplevel_manager.set_launch_origin`, additive in v4, keyed by `app_id`
and remembered for the app's lifecycle motions); when the shell never sends
one — headless, or a non-Dock launch — the compositor degrades to a centered
origin (the final rect shrunk about its center). Six motions share one
`WindowMotion` type (`compositor/src/window/motion.rs`): **appear** (a newly
mapped window grows/fades in), **restore** (a minimized window grows back
out), **minimize** (a visible window shrinks/fades into the tile), **close**
(a closing window shrinks/fades out), plus **zoom** and **fullscreen**
(T-02.3). Appear and restore are the same interpolation; minimize and close
are its reverse; zoom/fullscreen interpolate `origin → target` with alpha
fixed at `1.0` (the window is visible at both ends — geometry only). One
`MotionFrame` carries the render transform for all six.

Zoom/fullscreen are **render-only**: the state machine and the model geometry
change immediately (the window stays mapped, focused, and input-correct at
the destination geometry) and the `WindowMotion` only interpolates the
drawing, so an interrupt retargets from the current interpolated rect without
waiting for the first transition to finish. Because the client surface
resizes mid-flight, the render layer scales each committed buffer onto the
interpolated rect (`MotionFrame::scale_for`) rather than assuming a
target-sized buffer; appear/minimize/close keep their buffer at the target
size, so this reduces to the target-relative scale.

The render layer (`render::window_render_elements`) wraps the window's
surface elements (and its SSD titlebar) in a scale/relocate pair with a
surface alpha fade, so the model geometry stays the final one and **input
never moves**. A minimizing window leaves the layout and the focus/input path
immediately (state broadcast, unmapped from `Space`) but its surface is held
as a **ghost rendered from the window model** until the motion completes, so
there is no orphaned surface and no stale state. A **closing** window uses
the same ghost machinery: `close_window` unmaps it at once (input-inert) and
starts a `Close` motion; the model entry is removed exactly once when it
settles, broadcasting `Closed` (a client destroy during the motion is
deferred to the same single removal). Every transition is **interruptible**:
a new request mid-flight starts from the current interpolated rect rather
than waiting. For a close this is `DfState::interrupt_close` (or a restore of a
closing window): it replaces the `Close` with a `Restore` from the partial
ghost rect, re-enters the window at once, and restores focus, so the pending
removal is dropped and no `Closed` is broadcast. A close ghost also releases
keyboard focus the moment it becomes a ghost, so an input-inert window is
never a phantom focus target (the compositor mirrors the unset itself because
Smithay does not report it). `dock.minimizedAnimation`
is `scale` in this slice (`none` is the reduced-motion fallback: the states
still change and the motion collapses to one step). See
[ADR 0004](adr/0004-window-appear-origin-and-transform.md),
[ADR 0005](adr/0005-minimize-restore-motion-and-ghost.md),
[ADR 0006](adr/0006-zoom-fullscreen-geometry-motion.md),
[ADR 0007](adr/0007-close-ghost-deferred-removal.md), and
[ADR 0008](adr/0008-close-interruption-and-ghost-focus.md). T-04 reuses this
per-window transform for scale/clip/blur.

### Window shadows (T-04.1a)

Every decorated window is drawn over a soft drop shadow
(`compositor/src/window/shadow.rs`, `render::window_shadow_render_elements`).
The shadow is a stack of translucent rectangles — the same layered-rectangle
formula the design system's `Shadow.qml` uses — built entirely from the
**generated elevation tokens** so a compositor-drawn SSD window and a
first-party QML `AppWindow` cannot drift (FR-2): `blur`/`offsetY`/`layers`
come from `component.elevation.{low,med,high,overlay}` (which reference
`primitive.elevation.*`), and the color/opacity come from the active scheme's
`color.shadowColor`/`material.shadowOpacity`. A floating/SSD window is at
`high`; the QML `Shadow` selects the same level by name. The whole decorated
window (the SSD titlebar strip included, via `WindowInsets::outset`) is
shadowed, a window mid-appear/minimize/zoom carries the same lifecycle
`MotionFrame` as its content, and a minimizing/closing ghost keeps its shadow.
Fullscreen windows cast none. Shadows are custom elements appended **after**
`window_render_elements` in the front-to-back list, so they composite *below*
their window. Rounded corners (T-04.1b) and the chrome backdrop (T-04.2) land
on this same geometry; the shadow becomes one layer of the material pass.
See [ADR 0011](adr/0011-elevation-shadow-tokens.md).

### Rounded-corner clipping (T-04.1b)

The compositor's flat renderer has no vector rasterizer, so a rounded corner is
a **token-derived span decomposition**, not a shader:
`compositor/src/window/corner.rs` turns a `CornerMask` (a radius plus which
corners round) into non-overlapping horizontal spans. The radii come only from
the generated tokens — `WINDOW_RADIUS` = `component.window.radius`,
`TITLEBAR_RADIUS` = `component.titlebar.cornerRadius` — so the compositor and
the first-party QML `AppWindow`/`TitleBar` round from one source (FR-2/FR-3).

The compositor rounds the surfaces **it draws**: the SSD titlebar fill clips
its top corners (the bottom edge stays square, joining the content below —
exactly the QML `TitleBar`'s full-radius rectangle plus square bottom patch),
and every shadow layer rounds to `window.radius + blur * spread`, the same
formula as `Shadow.qml`'s per-layer `radius`. `ShadowLayer` carries the radius;
`ShadowSpec.radius` is the window token.

A third-party client surface's own pixels are **not** split per window here:
Smithay owns those elements and keys them for presentation feedback, so
per-window cropping would duplicate ids and risk the frame-callback and
direct-scanout paths. The reusable scene-transform pass (T-04.3) owns the
`clip` of live surfaces and consumes the same mask. First-party QML windows
already round themselves. See
[ADR 0012](adr/0012-rounded-corner-mask.md).

### Backdrop blur pass (T-04.2)

Every chrome surface — the menu bar, the Dock, and overlay popovers/menus — is
drawn over a **frosted backdrop**
(`compositor/src/window/backdrop.rs`, `render::chrome_backdrop_render_elements`)
so it reads as a material rather than a flat rectangle. The backdrop is built
entirely from the **generated material tokens**: `material.chromeBlur` /
`chromeOpacity` for persistent chrome and `material.popupBlur` / `popupOpacity`
for overlays, per `ColorScheme`, plus the component radius. `TitleBar.qml` and
`Dock.qml` consume the same `Theme.material` group, so the two sides cannot
drift (FR-2). A `MaterialRole` maps `df_shell.layer` to the token group; the
panel is clipped with the same `CornerMask` geometry as the titlebar and
shadow.

The flat renderer has no texture sampler yet, so the backdrop is a **stack of
translucent rounded feather layers** from the panel edge inward — the same
solid-rectangle approximation T-04.1a used for the shadow, with the token blur
setting the feather depth. It is a separate element drawn **below** the
chrome's Wayland surface, so a client's translucent regions blend over it
rather than being punched out (FR-3). The backdrop elements are appended
**after** the chrome surfaces and **before** the windows, so the panel sits
behind its chrome and in front of the scene it stands in for; no chrome mapped
means no backdrop and no extra damage.

`BackdropPass` gates the pass: the backend opens a rendered frame once
(`DfState::begin_render_frame`) and each output applies the backdrop at most
once. A repeated `(frame, output)` request is counted as `skipped` and draws
nothing — the T-04.2 **one effect pass per frame (no double-blur)** invariant,
reported on the render-stats line as `backdrop_passes` / `backdrop_skipped`
(`backdrop_skipped` must stay zero). A real texture-sampling blur is deferred,
swappable behind the token values; T-04.3 folds the pass into the reusable
scene transform and T-04.4a degrades it. See
[ADR 0013](adr/0013-backdrop-blur-pass.md).

### Reusable scene-transform pass (T-04.3)

T-05's Mission Control live-surface grid and T-06's app switcher transform the
**real** window surfaces — never thumbnails — and they must not each build
their own pipeline. The one transform is
`compositor/src/window/scene_transform.rs`: a `SceneTransform` maps a `source`
rectangle onto a `target` rectangle (scale/translate) and carries an optional
token-derived `CornerMask` clip (ADR 0012) and an optional token-driven
`BackdropSpec` blur (ADR 0013). It is pure logical geometry plus rectangle
materials, so it is asserted headless without a GPU.

The T-02 lifecycle motion renders through the same transform:
`SceneTransform::from_motion` derives the scale/offset from a `MotionFrame`
using `MotionFrame::scale_for` (the committed surface size, not the
target-relative chrome scale), so appear/minimize/zoom and the T-05 grid share
one mapping and one code path.

`SceneTransformPass` owns the per-frame guard on the shared
`window/pass.rs::FramePass` (which `BackdropPass` also uses): the backend opens
the frame once (`DfState::begin_render_frame`) and each output composes at most
one transform; a duplicate is counted as `skipped` and draws nothing — the
T-04.3 **one effect pass per frame (no double-transform)** invariant, reported
on the render-stats line as `scene_transforms` / `scene_transform_skipped`
(`scene_transform_skipped` must stay zero). The pass never forces a redraw, so
the idle trace stays at zero damage. Applying the clip to a third-party
surface's own elements is deferred to the renderer; the transform supplies the
mask geometry (ADR 0012). See
[ADR 0014](adr/0014-reusable-scene-transform-pass.md).

### Material degrade tiers (T-04.4a)

The materials are the first compositor effect with a real iGPU frame-budget
risk, so they degrade along one ordered ladder —
`compositor/src/window/degrade.rs`:

- `Full` is the token material as designed.
- `Reduced` shrinks the radius/blur geometry and roughly halves the layers.
- `Minimal` turns the backdrop blur **off** entirely and collapses the shadow
  to a small tight ring; the window stays rounded and legible.

A tier maps a token-resolved `BackdropSpec`/`ShadowSpec` through pure geometry
scaling, so `render::chrome_backdrop_render_elements` and
`render::window_shadow_render_elements` apply it where the material is
resolved. At `Minimal` the backdrop pass emits no elements and owns no damage;
the design tokens remain the single source and no opacity/tone is invented
(ADR 0011/0013). The lifecycle motion and the T-04.3 scene transform are
untouched — the tier changes material geometry, never the scene mapping, so
T-02 transitions render correctly at every tier.

`DegradeController` selects the tier from rendered-frame durations against a
frame budget (default `animation::FRAME_INTERVAL`, 16 ms; overridable with
`DRAGONFRUIT_FRAME_BUDGET_US`) using an EMA plus hysteresis, so a stray spike
cannot degrade the UI and a tier cannot oscillate. A tier can be pinned
(`set degrade-tier` over the synthetic harness, or `set_degrade_tier`) for a
deterministic test or feature. The selection never forces a redraw, so the
idle trace stays flat. The state is reported on a `degrade stats` render-stats
line and by `query degrade`. See
[ADR 0015](adr/0015-material-degrade-tiers.md).

### Light/dark and reduced motion (T-04.4b)

Every compositor-drawn material resolves its tones from the **live color
scheme** on `DfState` (`ColorScheme::{Light, Dark}`): the SSD titlebar and
traffic lights (`window/decoration.rs`), the window menu (`window/menu.rs`),
the chrome backdrop (`render::chrome_backdrop_render_elements`), and the window
shadow (`render::window_shadow_render_elements`). The scheme is compositor
state, not a render-argument: `DfState::set_color_scheme` flips it and requests
a redraw, `titlebar_element`/`titlebar_element_for_motion` and
`open_window_menu` stamp it onto the element they build, and the two material
passes read `state.color_scheme`. The default is dark (it reads over arbitrary
client pixels); the settings owner (T-08) mirrors `appearance.colorScheme`
here. The scheme is reported on a `scheme stats` render-stats line and by
`query material` (scheme plus the resolved chrome/elevated/border/accent
tones), so a headless conformance test proves both schemes render without a
GPU. See [ADR 0016](adr/0016-live-color-scheme-owner.md).

Reduced motion (`accessibility.reduceMotion`) is already the shared
`AnimationClock`/`OverviewMachine` policy (T-02/T-11): a transition still runs
the one machine and the same commit rule, collapsed to a single clock step. It
composes with either scheme — the material pass resolves the same tokens and
the lifecycle mapping is unchanged. The headless conformance test drives a full
appear at the light scheme and a reduced-motion minimize at dark, asserting the
same committed geometry, and `scripts/check-gallery-snapshots.py` covers the
design-system reference for light, dark, and dark+reduced.

### Mission Control live-surface grid (T-05.1a)

Mission Control lays the **real** window surfaces out as a grid, never a
cascade and never a thumbnail. The layout is pure geometry in
`compositor/src/overview/grid.rs`: `grid_layout` orders the candidates by
active-Space-then-strip, most-recently-used first, picks the near-square column
count (`ceil(sqrt(n))`, adjusted between `floor`/`ceil(sqrt(n))` toward the
output's aspect ratio), assigns one **uniform scale** so the largest window
fits its cell minus the token margin, and centers every window in its cell.
Membership excludes minimized windows (bottom strip) and fullscreen windows
(dedicated Space card); cells never overlap by construction.

The compositor draws each placement through the T-04 reusable transform — the
grader is `DfState::overview_grid_frame`, which builds a `MotionFrame` whose
rect lerps from the window's committed geometry to its grid target by the
overview progress (`0` normal, `1` grid). `window_render_frame` returns that
frame while the grid is shown and the lifecycle motion otherwise, so the
client surface, the SSD titlebar, and the shadow all carry the same mapping
and the transition is continuous and reversible. The committed geometry, focus,
and Space assignment are untouched; input ownership transfer is T-05.2.
`query grid` reports the progress and one line per placed live surface, and the
headless conformance test asserts the geometry. See
[ADR 0017](adr/0017-overview-grid-render-transform.md).

Neighbour-Space reveal (transforming the adjacent Spaces' surfaces, which are
unmapped from the `Space`) and per-output chrome insets are later T-05 slices.

## Window model

- **States.** A window is floating, minimized, zoomed, or fullscreen, with
  its restore geometry remembered across each transition. **Zoom and
  fullscreen are distinct states**, macOS-style: Zoom grows the window to
  fill the Space minus the menu bar and Dock; fullscreen creates its own
  Space (see [03-workspaces.md](03-workspaces.md)). There is no separate
  "maximize" state.
- **Focus.** Click-to-focus only; focus never follows pointer motion alone.
  Focus changes are broadcast over the private protocol so the shell, Dock,
  and menu-broker track the active application without polling (see
  [04-shell.md](04-shell.md), [06-global-menu.md](06-global-menu.md)).
- **Placement.** New windows open near the center of the active Space with a
  per-output cascade offset; transient dialogs center on their parent, stay
  above it, and minimize with it.
- **Regions.** Client-provided opaque, translucent, and input regions are
  honored: input outside the input region falls through, and translucent
  regions participate in the blur pass rather than fighting it.
- Windows survive shell restarts untouched — workspace assignment,
  stacking, and focus are compositor state the shell never co-owns.

## Protocol surface

Standard protocols the compositor implements or consumes:

- `xdg-shell`, `xdg-output`, `presentation-time`, `linux-dmabuf`
- `viewporter` + `fractional-scale` for fractional scaling
- `xdg-decoration` for SSD negotiation (see
  [05-window-decorations.md](05-window-decorations.md))
- Pointer constraints, relative pointer, `cursor-shape`, `idle-inhibit`
- `wp-content-type-manager-v1` content-type hints, so fullscreen video and
  conferencing clients can take efficient/scanout paths
- `wlr-data-control`, so the shell's clipboard manager sees text, images, and
  files (see [ROADMAP.md](../ROADMAP.md))
- `security-context`, so sandboxed (Flatpak) clients identify themselves
- The Wayland color-management protocol (`xx-color-management-v1` while
  staging) for per-output color and HDR — the basis of the Displays pane's
  color controls (see [08-settings.md](08-settings.md))
- `xdg-activation` (launch feedback / Dock bounce)
- `ext-session-lock-v1` for fail-secure locking; `ext-idle-notify` for idle
- `text-input` / input-method protocols for input methods
- **Capture is portal-only.** Screenshots and screen sharing are exposed
  exclusively through our portal backend as PipeWire streams, per-window or
  per-monitor, each with explicit user consent (see
  [07-system-integration.md](07-system-integration.md)). We deliberately do
  **not** implement `wlr-screencopy`-style grab access for arbitrary clients:
  if a capture is not a portal request, the answer is no.

### Private shell protocols

The shell's chrome surfaces (menu bar, Dock, Control Center, notifications,
overview strip) are Wayland surfaces created by the shell process through a
private, versioned protocol in the style of `wlr-layer-shell`: reserved
zones, stacking layers, and explicit keyboard-interaction modes, restricted
to our shell's client. Window/workspace control and output configuration
follow the precedents of `wlr-foreign-toplevel-management` and
`wlr-output-management`, extended with workspace enumeration, Mission
Control control, and app-switcher state. The precedents exist; we version
and own ours (see [01-architecture.md](01-architecture.md)).

Access is restricted to a fixed set of trusted session processes, each
provisioned with a one-time launch token out-of-band at startup: the shell,
and — when desktop icons ship — the Files desktop surface (see
[09-files.md](09-files.md)). `bind` attempts from any other client are
refused. The chrome protocols are a privilege of these processes, not a
public extension surface.

## Why the compositor owns everything visual

The compositor already knows every mapped window, output, workspace, focus
state, stacking relationship, and surface texture. This is what makes Mission
Control, the Dock, and workspace switching feel coherent instead of
screen-scraped:

- Mission Control renders the **real surfaces** while applying scale,
  translation, clipping, blur/shadow, and workspace transformations — no
  scraping, no re-rendering by a third-party program.
- The Dock obtains window/app state directly from the compositor instead of
  inferring it through public protocols.
- Workspace organization is an internal compositor primitive; the shell
  consumes a private, versioned interface for listing/reordering/creating/
  removing workspaces.

## Application identity

Wayland's `xdg_toplevel` gives applications an `app_id`, useful for mapping
windows back to `.desktop` applications. In practice the app resolver must
maintain fallbacks:

- `xdg_toplevel` `app_id` (primary, Wayland clients)
- Xwayland `WM_CLASS` (X11 clients)
- Heuristics for applications with inconsistent identifiers

The `app-index` service resolves application identity (icon, name, `.desktop`
entry, launch semantics) and is shared by the Dock, app switcher, and shell.

## Keymap conventions

The macOS-Cmd role maps to **Super/Mod4**; Option maps to **Alt**. The
mapping is chosen once here and shared by system shortcuts and first-party
application accelerators alike (see [06-global-menu.md](06-global-menu.md),
[09-files.md](09-files.md)), so applications cannot drift from the shell.

## Xwayland

X11 applications are supported through Xwayland. This is "good," not perfect:
strange Xwayland applications are an explicit compatibility work item in the
roadmap (see [ROADMAP.md](../ROADMAP.md)).

## Out of scope

- Wi-Fi/Bluetooth/audio/power logic — these belong to NetworkManager, BlueZ,
  PipeWire/WirePlumber, and UPower (see
  [07-system-integration.md](07-system-integration.md)).
- File-manager behavior — belongs to the Files app (see [09-files.md](09-files.md)).
- Menu models — belong to applications and the menu-broker (see
  [06-global-menu.md](06-global-menu.md)).

The difficult part of the compositor is not drawing; it is making the
underlying environment boringly reliable: GPU hotplug, suspend/resume, multiple
displays, unusual DPI combinations, input methods, drag/drop, full-screen
games, GPU/driver failures, and applications that bend protocol expectations.
See [14-risks.md](14-risks.md).
