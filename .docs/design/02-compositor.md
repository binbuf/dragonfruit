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
- Surface and window management: focus, move, resize, maximize, fullscreen,
  transient dialogs, popups
- Workspace model (see [03-workspaces.md](03-workspaces.md))
- Per-Space wallpaper rendering, part of the workspace scene so a Space's
  background slides with it during switches (see
  [03-workspaces.md](03-workspaces.md))
- Effects and animation policy (blur, shadows, scale/clip transforms)
- Security boundaries (seat/session, lock screen integration)
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
- Performance budgets are targets, not hopes — see
  [13-roadmap.md](13-roadmap.md).

## Event loop

One calloop event loop (Wayland clients, libinput, D-Bus, timers), rendering
driven per-output on vblank. Scene updates and GPU work are single-threaded
per output until profiling proves otherwise; correctness beats parallelism in
a display server.

## Protocol surface

Standard protocols the compositor implements or consumes:

- `xdg-shell`, `xdg-output`, `presentation-time`, `linux-dmabuf`
- `viewporter` + `fractional-scale` for fractional scaling
- `xdg-decoration` for SSD negotiation (see
  [05-window-decorations.md](05-window-decorations.md))
- Pointer constraints, relative pointer, `cursor-shape`, `idle-inhibit`
- `wlr-data-control`, so the shell's clipboard manager sees text, images, and
  files (see [13-roadmap.md](13-roadmap.md))
- `security-context`, so sandboxed (Flatpak) clients identify themselves
- The Wayland color-management protocol (`xx-color-management-v1` while
  staging) for per-output color and HDR — the basis of the Displays pane's
  color controls (see [08-settings.md](08-settings.md))
- `xdg-activation` (launch feedback / Dock bounce)
- `ext-session-lock-v1` for fail-secure locking; `ext-idle-notify` for idle
- `text-input` / input-method protocols for input methods
- Capture paths for screenshots and screen sharing are exposed through our
  portal backend as PipeWire streams (see
  [07-system-integration.md](07-system-integration.md))

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
roadmap (see [13-roadmap.md](13-roadmap.md)).

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
