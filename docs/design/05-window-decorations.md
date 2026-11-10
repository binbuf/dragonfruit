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

## Window chrome is a style system

The 2026-11-08 macOS structural review measured the AppKit toolbar-style
matrix. Chrome height is a property of the window style, not one universal
height:

| Style | Chrome | Dragonfruit id | Notes |
|---|---:|---|---|
| Borderless / panel | 0 pt | `borderless` | HUDs, Quick Look-style panels, overlays |
| Titled, no toolbar | 32 pt | `titlebar` | Minimum real chrome; content underlaps with `.fullSizeContentView` |
| `.unifiedCompact` | 40 pt | `unifiedCompact` | Shorter toolbar row |
| `.expanded` | 48 pt | `expanded` | Title above the toolbar row |
| `.unified` | 52 pt | `unified` | Default modern toolbar |
| `.unified` + icon-and-label | 66 pt | `unified` + labelled row | Disk Utility; not a separate style |
| `.preference` | 88 pt | `preference` | Title row plus centred preference toolbar |

Real apps confirm the variants: TextEdit, Finder Get Info and About use 32 pt;
Finder, Safari, System Settings, Photos, Activity Monitor and Calculator use
52 pt; Disk Utility uses 66 pt; Pages' template chooser is 38–43 pt; a Pages
document is 70 pt (28 pt title row + 42 pt toolbar); Safari Settings is 88 pt
(32 pt title row + 56 pt toolbar).

AppKit reports a 16×16 pt accessibility/hit frame around each traffic light.
The reference build is 2× and the close button's full extent measures 28 px
(14 pt). We draw a 13 pt circle centred inside the 16 pt frame, the lower
bound of the ±1 pt acceptance, because the 14 pt fill read slightly oversized
against the reference MacBook Pro. The measured insets are 8 pt in a 32 pt bar
and 18 pt in a 52 pt bar, applied to the frame on both axes, and the spacing
is 23 pt centre-to-centre. Drawing the 16 pt frame as the circle is what made
the lights look oversized. Tab, path, status and format bars are
content, not chrome: Finder adds a 28 pt tab bar, a 30 pt path bar and a 28 pt
status bar inside the content area. Hiding the toolbar is a mode change that
also hides the Finder sidebar and moves the status bar.

Do not hard-code 52 pt for utility windows, and do not derive a window's style
from its size: Calculator is compact but uses a 52 pt unified toolbar. T230b
landed the design-system/compositor migration; fullscreen hidden/revealed
chrome is implemented, while long titles and RTL mirroring remain `OPEN` (see
below). See
[23-macos-structural-surface-review.md](23-macos-structural-surface-review.md)
and [ADR 0241](adr/0241-macos-structural-surface-review.md), with the T230b
implementation decision in [ADR 0242](adr/0242-window-chrome-style-system.md).

### Style model and measured geometry (T230b)

The one source is `component.windowStyle` in
`design-system/tokens/tokens.json`. Each named style carries `height`,
`titleRow`, and `toolbarRow`; the row sum is the baseline, and
`WindowStyle.chromeHeight(style, { labelledToolbar, toolbarRowHeight })`
derives the final height. The QML `WindowStyle` singleton and the compositor's
`WindowStyle`/`WindowChrome` types are generated from or checked against that
table. A `TitleBar` inside an `AppWindow` inherits the window's
`windowStyle`, so Settings and Files declare `unified` once and never hard-code
52 pt.

Traffic-light geometry is also one table: `component.trafficLights.buttonSize`
(16 pt AppKit AX/hit frame), `diameter` (13 pt drawn circle), and `spacing`
(23 pt centre-to-centre). The inset is derived from the title row, not the
final chrome height: `(titleRow - buttonSize) / 2`, applied on both axes. That
yields 8 pt in the 32 pt `titlebar` row and 18 pt in the 52 pt `unified` row,
and keeps `expanded`/`preference` lights in their 28/32 pt title row instead of
pushing them into the toolbar. The visible circle sits 1.5 pt inside the frame.
The hover glyphs use the 7 pt Tabler box: the minus and cross are ~4.7 pt
marks with a ~1.2 pt stroke, and the `maximize` corner brackets reach ~5.8 pt.
The compositor rasterizes the same lines with per-pixel coverage, so the SSD
and QML marks match. The
existing hover reveal, disabled/focused fills, keyboard activation, and
window-action routing are unchanged.

Accessory bars are content: `component.accessoryBar` records the Finder
tab/path/status heights (28/30/28 pt) and the TextEdit format bar (20 pt).
They are not added to `WindowStyle.height()`. The Files path bar consumes the
30 pt token; tab/status/format bars land with their owning app tasks.

The compositor's default SSD style is `titlebar` (32 pt) because it draws no
toolbar. A style can be applied per window through the window model and the
synthetic `set window-style <window-id> <style>` seam; the conformance test
covers every baseline and the fullscreen reveal strip uses the same height.
The compositor still never draws a second titlebar over a CSD client (the
tier-3 dignity rule).

**OPEN:** long-title truncation beyond the existing `Text.ElideMiddle`, and
RTL titlebar mirroring/traffic-light order, are not implemented. They are
recorded as `N-WIN-29`/`N-WIN-30` in the experience manifest and must not be
claimed with a clipped screenshot.

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
- Dragging any edge or corner of a Tier-2 (SSD) window starts an interactive
  resize on the shared `ResizeGrab` math. The compositor owns a 6 logical-pixel
  edge strip and 12-pixel corner handles — the same geometry as the first-party
  `WindowResizeGrips` — and shows the matching resize cursor. Client popups,
  the menu bar, the Dock, and other chrome keep priority over the strip; a CSD
  window keeps its own edges and asks for `xdg_toplevel.resize` when it wants
  the compositor to resize it.
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
T230b replaces the original single 40 pt token with the measured
`component.windowStyle` table: the compositor's default SSD style is
`titlebar` (32 pt), and a window can carry any named style through
`WindowModel`/the synthetic test seam. The titlebar reserves its style height
as a top inset above the client area, so a zoomed client is configured one
chrome height shorter through the single `configure_window_size` path;
fullscreen hides the titlebar and reserves nothing. A client that negotiates
client-side decoration via `xdg-decoration` (or never creates a decoration
object, or an X11 client marked undecorated) is never given a compositor
titlebar — the tier-3 dignity rule.

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
  top strip sized by the window's `WindowStyle` (32 pt for the default
  `titlebar`, 52 pt for `unified`, 88 pt for `preference`), the titlebar is
  laid out as an overlay on the content (no inset) and its glyphs reveal.
  Moving away hides it again. The overlay wins the titlebar hit-test even
  though a client surface is under it, because it is compositor chrome that
  only exists while revealed.

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
token-opaque surface described above. T230b landed the measured style
taxonomy: `WindowStyle` (QML singleton + Rust enum) resolves the
`borderless`/`titlebar`/`unifiedCompact`/`expanded`/`unified`/`preference`
baselines (0/32/40/48/52/88 pt), `unified` plus the 14 pt labelled row is
66 pt, and an app-specific toolbar row can take an expanded document to 70 pt.
The first-party `TitleBar` and the compositor SSD renderer both read the same
table, so their height and traffic-light geometry cannot drift. Traffic lights
use a 16 pt AX/hit frame with a 13 pt drawn circle, 23 pt centre spacing, and
a title-row-derived inset (8 pt in a 32 pt bar, 18 pt in a 52 pt bar).
Settings and Files are 52 pt `unified` windows; their main toolbar is the
titlebar row, while the Files path bar (30 pt) stays content. `_NET_FRAME_EXTENTS`
is still not published to X11 clients ([STATUS.md](STATUS.md) G-16). The
label-less window menu is the main decoration gap (G-06). The GTK steering bridge now runs from
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

SSD edge resizing uses the same `resize_geometry`/`ResizeGrab` path as a
client's `xdg_toplevel.resize`: the compositor hit-tests the 6 px edge strips
and 12 px corner handles before client input and clamps through the client's
min/max hints. The conformance test
`window_conformance::ssd_frame_edges_and_corners_resize_the_window` drives all
eight directions over the protocol, and the same path covers X11 Tier-2
windows.
