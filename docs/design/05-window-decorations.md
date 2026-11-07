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

### GTK steering (compatibility bridge)

GTK steering combines three mechanisms, all in place before any application
can launch:

- On Wayland, GTK 3 decides between CSD and SSD from the legacy KDE
  `org_kde_kwin_server_decoration_manager` default mode, not from
  `xdg-decoration` or `GTK_CSD`. The compositor advertises that global with
  default mode `Server`; a cooperative GTK 3 client then requests SSD and
  gets the same left-side close/minimize/maximize titlebar as every other
  Tier-2 window. A GTK 3 app that explicitly asks for CSD (a headerbar) is
  honored and keeps its own decoration.
- On X11/Xwayland, `GTK_CSD=0` in the session environment asks GTK 3 for
  server-side decorations; the X11 path is already Tier 2 under our window
  manager.
- The standard GTK window-control layout (`close,minimize,maximize:` — close,
  minimize, maximize on the left) covers clients that keep client-side
  decoration (GTK 4/libadwaita, or a GTK 3 app that still draws its own
  headerbar). `dragonfruit-session --sync-toolkit` writes it to the user's
  `gtk-3.0/settings.ini`, `gtk-4.0/settings.ini`, and the GLib GSettings
  keyfile, then updates the live `org.gnome.desktop.wm.preferences
  button-layout` value through `gsettings` when that tool is available. The
  session entry calls it at login; running the command by hand inside a live
  session updates a dconf-backed desktop immediately.

GTK 4/libadwaita may ignore both `GTK_CSD` and the KDE protocol; the layout
setting is the fallback.

Sandboxed apps cannot read the host GTK files. Two bridges cover them:
`--sync-toolkit` writes the same `gtk-decoration-layout`/`button-layout`
keys into every existing `$HOME/.var/app/*/config` sandbox, and the Settings
portal projects the layout as `org.gnome.desktop.wm.preferences`
`button-layout`. The portal backend also starts the distro
`xdg-desktop-portal` frontend (whose unit requires
`graphical-session.target`). Electron/Chromium read GtkSettings (and GTK 4
reads the portal), so Flatpak VS Code and other Electron apps get the same
left-side buttons without a per-app build.

This remains a best-effort bridge: an application that draws its own controls
(Electron/Chromium, SDL, some toolkits, and apps with a hard-coded headerbar)
keeps them where it draws them. We do not inject a second titlebar or fake
controls into client pixels (the tier-3 dignity rule above).

### Other toolkits

| Toolkit | What makes the controls appear on the left |
|---|---|
| Qt 5/6 | Qt Wayland negotiates server-side decoration by default when the compositor offers `xdg-decoration`; Dragonfruit advertises it. Do not set `QT_WAYLAND_DISABLE_WINDOWDECORATION=1`. |
| GTK 3 | Wayland: the compositor's KDE server-decoration default mode `Server` → compositor SSD. X11: `GTK_CSD=0` (session default). If the app forces CSD, `gtk-decoration-layout` moves its controls left. |
| GTK 4/libadwaita | `gtk-decoration-layout` moves `GtkWindowControls`/`AdwHeaderBar` controls left; many apps still draw their own headerbar. |
| Electron/Chromium | Electron 43+ Window Controls Overlay reads the desktop's GTK/GNOME `button-layout` (GtkSettings, with GTK 4 using the Settings portal); the session writes that layout into Flatpak sandboxes too (VS Code: `"window.titleBarStyle": "custom"`, `"window.controlsStyle": "native"`). Chromium can also use the platform titlebar via `--gtk-window-decorations` / `--enable-features=UseGtkThemeFrame`; VS Code `"window.titleBarStyle": "native"` is the fallback. Apps without such an option keep their own frame. |
| X11 (Xwayland) | The compositor draws the frame; X11 apps are Tier 2 without per-app cooperation. |

## Negotiation

Decoration mode follows `xdg-decoration` (and, for GTK 3, KDE's legacy
server-decoration protocol) the way the protocols define them:

- The compositor advertises server-side decoration as its preference when a
  client creates a `zxdg_toplevel_decoration_v1` object. It also advertises
  `org_kde_kwin_server_decoration_manager` with default mode `Server`; a
  surface that creates a KDE decoration object is recorded as SSD unless the
  client explicitly requests `Client`.
- A client that negotiates server-side decoration gets our compositor-drawn
  titlebar (Tier 2).
- A client that never creates a decoration object, or explicitly requests
  client-side decoration, keeps its own decoration. This covers first-party
  Tier-1 apps (frameless Qt windows that draw the design-system `TitleBar`)
  as well as third-party CSD clients, so the compositor never draws a second
  titlebar over them.
- Expected third-party outcomes: Qt apps typically negotiate SSD (very good);
  GTK 3 apps negotiate SSD through the KDE default mode (very good); GTK
  4/libadwaita apps draw their own headerbars (Tier 3, steerable left via GTK
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

At T-01, explicitly deferred: real materials (blur/shadow/rounded corners,
T-04), drag-to-move and double-click zoom on the titlebar (T-01.3), the window
menu's real labels/icons (T-01.4 landed the geometry and command routing as
flat token fills, the same placeholder stage as the T-01.1 glyphs), and the
live color scheme (settings, T-08). The shadow/rounding half of the material
deferral landed in T-04; the menu labels did not (see the current status at the
end of this document).

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
still flat and label-less: the compositor has no text/icon renderer, so the
menu draws token-colored rows without labels. T-04 did not add the promised
labels, and T-14 reused the menu model rather than solving text rendering.
This is a first-day usability gap, tracked in [STATUS.md](STATUS.md) G-06; do
not describe the window menu as shipped until it is readable.

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
  `_NET_FRAME_EXTENTS` is not published yet; it was deferred to the T-16
  compatibility polish, which did not land it. It remains open (G-16).

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

**Current status (2026-11).** T-04.1a/1b landed the shadow and rounded-corner
geometry; T-20's sampled material is a chrome-panel pass and deliberately does
not turn the SSD titlebar into glass, so the titlebar fill remains the
token-opaque surface described above. `_NET_FRAME_EXTENTS` is still not
published to X11 clients ([STATUS.md](STATUS.md) G-16). The label-less window
menu is the main decoration gap (G-06). The GTK steering bridge now runs from
the session entry (`dragonfruit-session --sync-toolkit`), the session exports
`GTK_CSD=0` for X11 GTK 3, and the compositor advertises KDE's legacy
server-decoration manager with default mode `Server` for Wayland GTK 3. The
Settings portal projects the layout into the standard GNOME namespace for
sandboxed apps, and the portal backend starts the distro frontend. The
compositor also answers a decoration object created after the initial
configure with the SSD mode, so a lazily negotiating client still gets the
left-side titlebar. Electron/Chromium and other self-drawn frames remain the
documented tier-3 boundary; Electron 43+ can follow the portal layout in its
Window Controls Overlay.
