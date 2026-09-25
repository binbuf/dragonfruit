# Window Decorations (Traffic Lights)

## Summary

Left-side close/minimize/maximize "traffic lights" are achievable with three
levels of quality, bounded by a Wayland limitation we accept from the outset.

## The Wayland constraint

The Wayland `xdg-shell` specification explicitly says an `xdg_toplevel` is, by
default, responsible for its own intended visual representation, including its
title bar and window controls. Modern Wayland applications can draw their
**own** titlebar. The compositor cannot reliably replace part of an
application's pixels without creating ugly overlapping controls or breaking
application interaction.

## Three quality tiers

| Tier | Applications | Quality |
|---|---|---|
| 1 | Our own applications (Settings, Files, …) | **Perfect** — exactly our chosen left-side controls, via our design system (see [10-design-system.md](10-design-system.md)) |
| 2 | Applications accepting server-side decoration | **Very good** — the compositor draws its own titlebar and traffic lights |
| 3 | Applications drawing custom client-side decorations (CSD) | **Best effort** — we do not inject controls |

Tier 3 can be improved through GTK/Qt configuration and themes so cooperative
applications place controls on the left.

## Negotiation

Decoration mode follows `xdg-decoration` the way the protocol defines it:

- The compositor advertises server-side decoration as its preference when a
  client creates a `zxdg_toplevel_decoration_v1` object.
- A client that negotiates server-side decoration gets our compositor-drawn
  titlebar (Tier 2).
- A client that never creates a decoration object, or explicitly requests
  client-side decoration, keeps its own decoration. This covers first-party
  Tier-1 apps (frameless Qt windows that draw the design-system `TitleBar`)
  as well as third-party CSD clients, so the compositor never draws a second
  titlebar over them.
- Expected third-party outcomes: Qt apps typically negotiate SSD (very good);
  GTK apps draw their own headerbars (Tier 3, steerable left via GTK
  settings we document and configure); Electron/Chromium and most SDL games
  are CSD (Tier 3).
- Xwayland windows: the X server is an ordinary `xdg-shell` client and does
  not draw Wayland CSD, so **X11 applications reliably land in Tier 2** —
  compositor-drawn decoration without per-app cooperation.

## Behavior spec

Our decorations, wherever they appear, behave identically:

- Traffic-light glyphs reveal on hover of the button cluster; colorless
  otherwise.
- Dragging the titlebar starts an interactive move.
- Double-click on the titlebar dispatches the `dock.titlebarDoubleClick`
  action (`zoom` by default, or `minimize`/`none`); right-click opens the
  window menu (Move to Space, Minimize, Zoom, Close — see
  [03-workspaces.md](03-workspaces.md)).
- Fullscreen windows hide the titlebar; a hover reveal of the top strip
  overlays it on the content and keeps controls reachable.

## Product promise

> **"Traffic lights wherever the application permits native desktop
> decoration"** — not "all windows."

We never inject a fake compositor titlebar above every CSD application; that
would make the desktop *less* premium, not more. This is one of the two
accepted compromises (see [00-overview.md](00-overview.md)).

## Decoration policy

- Server-side decoration is offered and preferred for cooperative clients.
- Our decoration visual design (traffic lights, typography, shadows,
  translucency) is owned by the design system so compositor-drawn titlebars
  and first-party app titlebars look identical.
- Third-party decoration themes ship later, as a compatibility work item in
  the roadmap (see [ROADMAP.md](../ROADMAP.md)).

## Implementation status (T-01)

The T-01 loop lands the SSD titlebar element from the generated tokens
(`compositor/src/window/decoration.rs`): a flat token fill with square
corners, left-side close/minimize/zoom lights, and hover/disabled *drawing*.
The titlebar reserves its height as a top inset (`component.titlebar.height`,
40 logical px) above the client area, so a zoomed client is configured one
titlebar shorter through the single `configure_window_size` path; fullscreen
hides the titlebar and reserves nothing. A client that negotiates client-side
decoration via `xdg-decoration` (or never creates a decoration object, or an
X11 client marked undecorated) is never given a compositor titlebar — the
tier-3 dignity rule.

Explicitly deferred: real materials (blur/shadow/rounded corners, T-04),
drag-to-move and double-click zoom on the titlebar (T-01.3), the window
menu's real labels/icons (T-01.4 lands the geometry and command routing as
flat token fills, the same placeholder stage as the T-01.1 glyphs), and the
live color scheme (settings, T-08).

T-01.2 adds the traffic-light interaction on top of the element: the
compositor tracks which button cluster the pointer is over (glyph reveal),
and a left press on a light is consumed before client routing and mapped to
the existing window state machine — close (`xdg_toplevel.close` /
`WM_DELETE_WINDOW`), minimize, and zoom/unzoom. There is no new window state
and no second owner of truth. The titlebar is only hit-tested when no client
surface (toplevel, popup, or input region) and no chrome surface is under the
pointer, so popups, IME, and per-surface input regions keep priority. Reduced
motion is N/A here: no animation is introduced (lifecycle motion is T-02).

T-01.3 completes the titlebar gesture set on the same element and the same
state machine:

- A left press on a floating window's titlebar (not on a light) activates the
  window and starts the existing interactive `MoveGrab`; movement clamps to
  the output and geometry keeps flowing through the one
  `configure_window_size`/`move_window` path.
- Two titlebar presses on the same window within 400 ms and 4 logical pixels
  count as a double-click; it dispatches `TitlebarDoubleClick` — `zoom`
  (default), `minimize`, or `none` — mirroring the shell's
  `dock.titlebarDoubleClick`. Until T-08 settingsd owns the live value, the
  compositor holds the field (default `zoom`) and a synthetic-input command
  sets it for tests (see ADR 0001).
- A fullscreen window has no persistent titlebar; when the pointer enters the
  top `component.titlebar.height` strip, the titlebar is laid out as an
  overlay on the content (no inset) and its glyphs reveal. Moving away hides
  it again. The overlay wins the titlebar hit-test even though a client
  surface is under it, because it is compositor chrome that only exists while
  revealed.

Zoomed and fullscreen windows are not interactively moved (matching the grab
module's contract); multi-monitor drag polish is T-16.

T-01.4 adds the window menu on the same element and the same state machine.
A right-click or Control-click on a titlebar opens a compositor-owned menu
(`compositor/src/window/menu.rs`) whose rows are **Move to Space** (a submenu
of the window output's Spaces), **Minimize**, **Zoom**, and **Close**. The
menu holds no window state: each row resolves to a `WindowMenuCommand` and
is applied by `DfState::window_menu_command`, the same primitives the traffic
lights and the shell protocol use. Panel geometry and dismissal mirror the
design-system `ContextMenu`: rows are `component.contextMenu.rowHeight` tall
inside `padding`, the panel flips/clamps to stay inside the output, Escape
closes an open submenu and then the menu, Up/Down/Home/End move the
highlight, Right/Left open/close the submenu, Enter/Space activates, and
click-away or focus loss dismisses. While open the menu is modal to keyboard
and pointer input. The four commands are proven for a Wayland and an X11
window in `window_conformance` / `xwayland_conformance`. The visual pass is
flat and label-less for now (no text renderer yet); T-04 draws the real
labels and materials, and T-14 reuses the menu for decoration themes.

T-01.5 pins down the tier policy and the X11 path:

- The tier comes from `xdg-decoration`: the compositor default and an
  explicit `ServerSide` request are Tier 2; only an explicit `ClientSide`
  request is Tier 3. An X11 window that sets `_MOTIF_WM_HINTS` with the
  decoration bit clear (it draws its own) is Tier 3; every other X11 window is
  Tier 2, because Xwayland never draws Wayland CSD. The three cases are
  asserted side by side in
  `window_conformance::decoration_tier_matrix_default_explicit_ssd_and_csd_side_by_side`,
  and an X11 `_MOTIF_WM_HINTS` flip is covered in `xwayland_conformance`.
- Only Tier 2 is given a compositor titlebar; Tier 3 keeps its full client
  area and reserves no inset (no double decoration in either direction).
- The tier is not permanent. A runtime `xdg-decoration` mode change or an X11
  `_MOTIF_WM_HINTS` update re-applies the reserved inset through the single
  `configure_window_size` path (`DfState::set_decoration_tier`): a zoomed
  window that flips to Tier 3 reclaims the titlebar strip and one that flips
  back gives it up again. Without this the titlebar would overlap the client
  or leave the strip empty.
- The X11 configure is the same geometry path as a Wayland toplevel: a zoomed
  Tier-2 X11 window is configured one titlebar shorter than the output (its
  frame offset by the strip), verified against the real X server geometry in
  `xwayland_conformance::x11_decoration_tier_follows_motif_hints_and_configures_insets`.
  `_NET_FRAME_EXTENTS` is not published yet; the report is documented as
  deferred to the T-16 compatibility polish.

T-04.1a makes the decoration read as a real material: an SSD window now casts
a soft drop shadow drawn by the compositor behind the whole decorated window
(titlebar included), and the same elevation tokens drive the first-party QML
`AppWindow`/`Shadow` (FR-2; see
[02-compositor.md](02-compositor.md) and
[ADR 0011](adr/0011-elevation-shadow-tokens.md)). T-04.1b then rounds the
chrome: the compositor SSD titlebar clips its top corners from
`component.titlebar.cornerRadius` using the same span decomposition the QML
`TitleBar`'s full-radius rectangle plus square bottom patch produces, and the
shadow layers round to `component.window.radius + blur * spread`, matching
`Shadow.qml` (see [ADR 0012](adr/0012-rounded-corner-mask.md)). The titlebar
chrome fill is still opaque; translucency and the real backdrop blur are the
following T-04 material tasks, on the same element and the same geometry.
