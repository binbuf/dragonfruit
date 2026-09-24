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

#### Window lifecycle motion (T-02.1b appear; T-02.2 minimize/restore; T-02.3 zoom/fullscreen)

A window scales and fades between an *origin* rectangle and its final
geometry on that clock. For the launch motions the origin is the owning Dock
entry's tile, supplied by the shell over the private protocol
(`df_toplevel_manager.set_launch_origin`, additive in v4, keyed by `app_id`
and remembered for the app's lifecycle motions); when the shell never sends
one — headless, or a non-Dock launch — the compositor degrades to a centered
origin (the final rect shrunk about its center). Five motions share one
`WindowMotion` type (`compositor/src/window/motion.rs`): **appear** (a newly
mapped window grows/fades in), **restore** (a minimized window grows back
out), and **minimize** (a visible window shrinks/fades into the tile), plus
**zoom** and **fullscreen** (T-02.3). Appear and restore are the same
interpolation; minimize is its reverse; zoom/fullscreen interpolate
`origin → target` with alpha fixed at `1.0` (the window is visible at both
ends — geometry only). One `MotionFrame` carries the render transform for all
five.

Zoom/fullscreen are **render-only**: the state machine and the model geometry
change immediately (the window stays mapped, focused, and input-correct at
the destination geometry) and the `WindowMotion` only interpolates the
drawing, so an interrupt retargets from the current interpolated rect without
waiting for the first transition to finish. Because the client surface
resizes mid-flight, the render layer scales each committed buffer onto the
interpolated rect (`MotionFrame::scale_for`) rather than assuming a
target-sized buffer; appear/minimize keep their buffer at the target size, so
this reduces to the target-relative scale.

The render layer (`render::window_render_elements`) wraps the window's
surface elements (and its SSD titlebar) in a scale/relocate pair with a
surface alpha fade, so the model geometry stays the final one and **input
never moves**. A minimizing window leaves the layout and the focus/input path
immediately (state broadcast, unmapped from `Space`) but its surface is held
as a **ghost rendered from the window model** until the motion completes, so
there is no orphaned surface and no stale state. `dock.minimizedAnimation`
is `scale` in this slice (`none` is the reduced-motion fallback: the states
still change and the motion collapses to one step). See
[ADR 0004](adr/0004-window-appear-origin-and-transform.md),
[ADR 0005](adr/0005-minimize-restore-motion-and-ghost.md), and
[ADR 0006](adr/0006-zoom-fullscreen-geometry-motion.md). T-04 reuses this
per-window transform for scale/clip/blur.

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
