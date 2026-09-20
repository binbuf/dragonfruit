# Roadmap and MVP Definition

## Summary

We distinguish a **demo MVP** from a **daily-driver MVP**. The temptation is
to spend months cloning every System Settings page before the desktop itself
feels good; we deliberately do almost the reverse.

## The first vertical slice

```text
Our compositor
├── single-monitor output
├── mouse + keyboard
├── floating windows
├── focus / move / resize / maximize / fullscreen
├── three workspaces
├── basic animations
├── Xwayland
└── private shell protocol

Our shell
├── top menu bar
├── Dock
├── clock
├── Wi-Fi menu
├── volume
├── battery
└── Mission Control button/gesture

First-party apps
├── Settings
└── Files
```

At that point we already have the essence of the product.

## The core interaction loop

Get this 30-second sequence absurdly polished — it matters more to product
identity than a complete printer configuration page:

```text
launch app
→ Dock animation
→ window appears
→ traffic lights
→ workspace switching
→ Mission Control
→ minimize
→ restore from Dock
→ app switch
→ close
```

## Daily-driver bar

| Area | Requirement |
|---|---|
| Displays | Multi-monitor, hotplug, scaling, rotation |
| Windowing | Xwayland, fullscreen, transient dialogs, popups |
| Input | Keyboard layouts, mouse, touchpad, gestures |
| Session | Clean startup/shutdown and crash behavior |
| Security | Trustworthy lock screen |
| Power | Idle, suspend/resume, battery |
| Desktop services | Notifications and OSD |
| Portals | Screenshot / screen sharing / file chooser integration |
| Networking | Wi-Fi / VPN basics |
| Bluetooth | Discovery / pair / connect |
| Audio | Input/output/volume/device switching |
| Storage | Removable media and mounting |
| Clipboard | Text, images, files |
| Accessibility | Keyboard navigation and AT-SPI-compatible first-party UI |
| Applications | Settings + usable file manager |
| Hardware | Intel/AMD baseline, then NVIDIA validation |

A contemporary desktop ships independent idle, OSD, notifications, screenshot,
portal, settings-daemon, greeter/session, and workspace components in addition
to the visually obvious compositor and panel — that is the reality check for
the breadth of "daily-driver."

## Implementation sequence

### Ticket status

Updated when a task is partially or completely finished; see
[tasks/](tasks/) for scope and acceptance criteria.

| Phase | Tickets |
|---|---|
| 1 · Foundation | T-01 ✅ done · T-02 🔄 partial (compositor core) · T-03 🔄 partial (input engine) · T-04 🔄 partial (window model) · T-05 🔄 partial (Spaces model) · T-06 🔄 partial (Xwayland) · T-07 🔄 partial (private shell protocols) |
| 2 · Experience | T-08 ✅ done (design system; app-level chrome lint + live AT-SPI dump deferred) · T-09 🔄 partial (menu bar + shell bootstrap + overlay-layer dropdown + scripted output hotplug; T-20/T-22 content open) · T-10 🔄 partial (Dock presentation core + shell surface + running entries/activation + launch/pinned persistence + launch/attention bounce + magnified-band input region + app context menus/window chooser + drag rearrangement (reorder/promote/remove) + live `dock.*` settings/divider menu/reduced motion + left/right reserved-zone foundation + per-position (left/right) surfaces, vertical layout, beside-the-bar popovers, corrected auto-hide translation and edge-band reveal/re-hide state machine; Trash, external drops, scene-graph render path open) · T-11 … T-14 pending |
| 3 · Flagship apps | T-15 … T-19 pending |
| 4 · System integration | T-20 … T-23 pending |
| 5 · Desktop infrastructure | T-24 … T-29 pending |
| 6 · Compatibility | T-30 pending |
| 7 · Polish | T-31 pending |
| Cross-cutting | T-32 pending |

T-01 is complete except one open item: the CI workflow is written and
all gates pass locally, but the first green run on a PR is still
pending. The dev tool also now owns every child in a `ChildGuard`, so a
panic or early return can never leak a compositor or launched app into
the host session (FR-5 hardening). T-02 is partial: the calloop event
loop, all three backends (nested verified live on the dev host; DRM
compiles, runtime-untested; headless used by CI), the full standard
protocol surface (CI-checked advertise list), outputs, input routing,
damage-driven rendering, and teardown hardening are done. Render-path
counters (FR-2/FR-5) are now dumped on SIGUSR1 and on clean exit so the
budgets can be measured without a debugger; the headless backend counts
redraw requests honestly and `compositor/tests/idle_trace.rs` (in
`make e2e`) asserts the FR-2 idle steady state — `frames_rendered` stays
flat across a second of inactivity. A headless protocol conformance
suite now maps a real client window end-to-end and drives the xdg state
machine (see T-04). Open: performance budgets FR-2/3/4 still need
on-hardware measurement (nested idle trace + input-to-photon latency),
VRR/night-light plumbing, multi-GPU runtime validation, direct-scanout
counter verification on real hardware. Details and hand-off notes:
[PROGRESS.md](PROGRESS.md).

T-03 is partial: the Cmd→Super / Option→Alt mapping is fixed once in
`df-ipc` and consumed by the xkb keymap and shortcut engine; the global
shortcut engine (system bindings, focused-app accelerator admission,
conflict resolution, no client grabs + refused-grab audit), gesture
recognition feeding one shared progress pipeline (clamp/rubber-band/
velocity), dwell-based hot corners, the unified input dispatch outbox,
the live input settings model (keyboard repeat + gesture/hot-corner
config), and the tablet/touch forwarding pipeline are done and covered
by 25 unit tests including a trigger-type matrix. The gesture recognizer
is now reset on every gesture end (an unclaimed gesture used to leak its
recognizer state into the next one) and the shell outbox is bounded so a
session with no shell attached cannot grow without limit. A synthetic-input
harness (`compositor/src/input/synthetic.rs`, `DRAGONFRUIT_SYNTHETIC_INPUT`)
now gives the headless backend a real seat: a line protocol over a
`UnixDatagram` feeds libinput-equivalent events through the same router,
and the headless integration test drives a keyboard shortcut, a hot-corner
dwell, a four-finger gesture, and a pointer click end to end (asserting the
private-protocol `input_action`/`hot_corner`/`progress` events). Pointer
constraints (confine/lock) are now enforced by a real grab and gated by the
`GrabArbiter` — only a launch-token-sanctioned session client may install
one, and the headless suite proves both the sanctioned lock and the
refusal of an unsanctioned one. Open: DRM/hardware validation of
multitouch + pen pressure; wiring the outbox to the shell over the private
protocol (T-07); menu-broker/portal accelerator registration (T-22/T-27);
per-device libinput acceleration/scroll live application (needs backend
device handles, T-16); on-device gesture tuning; unclaimed-gesture
pass-through policy. See [keymap.md](../docs/keymap.md) for the in-repo
keymap decision. Details: [PROGRESS.md](PROGRESS.md).

T-04 is partial: the four-state window machine (floating/minimized/zoomed/
fullscreen with exact restore geometry, maximize→Zoom, no maximize state),
click-to-focus with lifecycle/focus broadcasts for the shell (window
mapped/unmapped, focused/unfocused, title/app_id changes, state changes),
per-output centered cascade with wrapping, transient centering and
minimize/restore/close-with-parent, reserved-zone-aware Zoom, interactive
move/resize grabs with client min/max-size clamping and output clamping,
popup positioner constraint handling (flip/slide/resize) with popup
keyboard/pointer grabs and parent-unfocus dismissal, and window-menu
primitives are done and covered by 30+ unit/property tests. A new
headless conformance suite (`compositor/tests/window_conformance.rs`)
now drives a real Wayland client through mapping, maximize/unmaximize,
and fullscreen/unfullscreen over the protocol, and asserts popup
configures stay inside the output — it caught and drove the fix for two
real bugs (pending windows never mapped because the buffer check read a
field `on_commit_buffer_handler` had already consumed; and missing
`Window::on_commit` left every window's bbox/restore geometry at 0×0).
A malformed-client suite now drives contradictory state requests,
contradictory min/max size hints, re-entrant fullscreen/maximize, and a
fullscreen-toplevel destroy, then proves the session still configures a
fresh window. The synthetic-input harness (T-03) now supplies the seat
button that starts a pointer grab, so a new conformance test drives
interactive `xdg_toplevel.move`/`resize` over the protocol and asserts the
resize configure (move has no xdg-shell geometry event, so it is a
grab-ran/window-survived assertion). Open: the scripted shell-restart test
(FR-9) — the state is compositor-owned so the architecture supports it, but
there is no shell process to restart until the shell chrome lands
(T-09/T-10); aspect hints (X11) and the
private-protocol wiring of the broadcast outbox (T-07). Details:
[PROGRESS.md](PROGRESS.md).

T-05 is partial: the per-output ordered Space model (three Spaces per
display) with lockstep switching, dedicated fullscreen Spaces with an
exact origin round-trip, window→Space assignment, app Space memory keyed
by `app_id`/`WM_CLASS`, minimized-window exclusion, display-hotplug
attach/detach migration, per-Space wallpaper data, and a bounded
workspace event outbox are done and covered by 16 unit tests — including
the two-output lockstep switch, the fullscreen round-trip, and the
attach/detach matrix with zero window loss. Keyboard and gesture triggers
now switch Spaces through the existing T-03 dispatch paths, and the
active Space's wallpaper color is compositor-rendered (the shell never
draws the desktop background) in the nested and DRM clear passes. Open:
image wallpaper rendering (`source`/`fit` are stored but not sampled) and
the sliding/scale scene mechanics during a switch (T-11 owns the
animation polish); protocol-level verification of lockstep and the
fullscreen Space lifecycle waits on the private protocol (T-07), so the
current tests are model-level; window placement still keys off the
primary output until multi-monitor placement lands (T-11/T-16); app
memory is session-scope only (settingsd persistence deferred). Details:
[PROGRESS.md](PROGRESS.md).

T-06 is partial: Xwayland is spawned eagerly at session start on every
backend (a missing binary degrades to a Wayland-only session), `DISPLAY`
is exported to the session environment and handed to the dev tool through
a per-socket runtime file, and an Xwayland crash respawns the server
without disturbing the session (FR-6). X11 windows are mapped through the
same `WindowModel`/`Space`/workspace machinery as `xdg_toplevel`s —
cascade and transient placement, focus, minimize/restore, zoom,
fullscreen, close, and stacking — with `WM_CLASS` resolved to a
`.desktop` application by an interim pure-std resolver (`StartupWMClass`,
then desktop id/stem; misses recorded for T-23) and marked Tier-2 SSD
unless `_MOTIF_WM_HINTS` opts out. Clipboard selection is bridged both
directions through the data-device selection the shell's
`wlr-data-control` manager observes. A headless conformance suite
(`compositor/tests/xwayland_conformance.rs`) drives a real X11 client over
the X protocol: it creates and maps a window, asserts it appears in
`_NET_CLIENT_LIST`, destroys it, and kills the Xwayland process to prove a
clean respawn; `xmessage` was also launched through
`dragonfruit dev --nested --launch` with `DISPLAY` exported and rendered
nested. Open: **XDnD drag-and-drop is not implemented** (Smithay 0.7's XWM
has no XDnD bridge), so FR-5 is unmet until a bridge lands (T-30/T-31);
the scripted Firefox/Steam/SDL/xterm matrix was not run on this host (no
spare GPU session or Steam) and is the first T-30 job; the resolver is
deliberately not GIO `AppInfo` (the compositor must not link the desktop
stack) and is replaced by `app-index` in T-23; Smithay 0.7's `X11Wm`
source closure forms a calloop `Rc` cycle, so the compositor explicitly
unlinks its socket on teardown (T-31/upstream follow-up). The
fractional-scaling policy is decided and documented in
[docs/xwayland-scaling.md](../docs/xwayland-scaling.md) (integer-scaled
Xwayland + per-surface viewport downscale; plumbing lands with
T-13/T-31). Details: [PROGRESS.md](PROGRESS.md).

T-07 is partial: the private, versioned Wayland protocol set is defined
and served. `df_core` is the lockstep handshake and launch-token trust
model (one-time, per-boot, role-scoped; refused binds/handshakes logged and
disconnected). `df_shell`/`df_layer_surface` carry anchored, layered chrome
surfaces with reserved zones folded into the Zoom area and reported to the
outputs. `df_toplevel_manager`/`df_toplevel`/`df_workspace`/`df_output`
carry window/workspace/output enumeration, request round-trips with `done`
acks, focus/attention/hot-corner/overview/app-switcher/input broadcasts,
and scene-consistent ordering driven by the T-03/T-04/T-05 outboxes. A
headless compliance client
(`compositor/tests/shell_protocol_conformance.rs`) passes the refusal
matrix (untrusted bind, invalid token, wrong version, replayed token),
chrome configure/reserved zones, and output/workspace/toplevel request
round-trips; the token model and layer geometry are unit-tested; deviations
from the `wlr-*` precedents are documented in
[docs/private-protocols.md](../docs/private-protocols.md). The compliance
client now also asserts output geometry/mode/transform, workspace
index/activated/fullscreen/removed, focus, attention (`xdg-activation`),
app-switcher state, and toplevel output/workspace/closed transitions, and
a malformed-traffic suite re-authenticates, sends extreme output values
and stale-handle requests, and proves the session survives — it caught
and fixed a real panic where a client-supplied `set_mode` size
constructed a negative Smithay `Size`. Open: chrome *rendering*, layer
stacking, and keyboard-mode seat grabs are T-09/T-10 (T-07 owns
placement/zones/configure only); Qt/C++ shell bindings are generated by
`scripts/gen-shell-protocol-bindings.sh` but not yet consumed until the shell
chrome lands (T-09/T-10);
VRR/night-light output requests are acked but not plumbed (T-16);
reserved zones are a single global union rather than per-output
(T-11/T-16); toplevel thumbnails for the Dock are deliberately absent
(token-gated if added, T-10); the `input_action`/`progress`/`hot_corner`
compliance events are now asserted through the T-03 synthetic-input
harness, leaving only `app_accelerator` (needs a focused app with a
registered accelerator, T-22); the additive-only proof test (FR-7) is
policy-enforced by the lockstep handshake but not yet exercised because no
interface has a `since="2"` member. Details: [PROGRESS.md](PROGRESS.md).

T-08 is done: the design system is the single source of visual truth for the
whole desktop. `design-system/tokens/tokens.json` is generated into the QML
`Theme` singleton and `compositor/src/design_tokens.rs`, and `make
check-tokens` (CI) fails if either is stale. All twenty components from
[10-design-system.md](design/10-design-system.md) now ship — `AppWindow`,
`TitleBar`, `TrafficLights`, `Sidebar`, `Toolbar`, `SplitView`, `SettingsRow`,
`SettingsGroup`, `Toggle`, `SegmentedControl`, `Popup`, `ContextMenu`,
`MenuBarMenu`, `SearchField`, `SourceList`, `Icon`, `Dialog`, `Sheet`,
`Popover`, `ScrollView` — plus the supporting `Button`, `FocusRing`, and
`Shadow`. Every component has dark/light, reduced-motion, keyboard operation,
AT-SPI role mapping, and gallery coverage. The gallery renders every component
× state × scheme × motion; `scripts/check-gallery-snapshots.py` runs it
headless and `--strict` diffs the 66 committed goldens. `tst_design_system`
covers tokens, reduced motion, keyboard activation, AT-SPI roles, pixel
sampling, and the FR-3 screenshot diff between the app `TitleBar` and an
independent compositor SSD reference rendered from the same tokens.
`scripts/check-design-tokens.sh` forbids literal colors/durations/radii in
component QML. Open: the live `atspi` role dump (needs a session bus; the
keyboard walkthrough and per-type roles are asserted in the tests, and the
live dump belongs with T-31), joint blur/translucency tuning with T-02/T-13,
and the app-level "no hand-rolled chrome" lint (T-16/T-18). Details:
[PROGRESS.md](PROGRESS.md).

T-09 is partial: the menu bar render/interaction core and the shell process
bootstrap are landed. `shell/menubar/` now holds the real bar built from the
design system: the active application's menu (`MenuBarMenu` per top-level
menu, with the application name as the FR-2 priority-3 fallback until T-22's
menu-broker lands), unified status-item slots (Wi-Fi, Bluetooth, volume,
battery, Focus/DND, accessibility) that hide when their backing daemon is
absent and dim when disabled, a locale-formatted clock with a single
minute-aligned timer (no polling), the Control Center entry point, and the
Mission Control button. The FR-3 interaction rules — click-to-open,
delayed-hover drag-through, Escape/click-away/focus-loss dismissal, and live
focus-switch tracking — are implemented and scripted by the new
`shell/tests/tst_menubar.qml` (ctest). The shell process
(`shell/src/dragonfruit-shell`) is a real private-protocol client: it
connects with libwayland-client, authenticates with the one-time launch token,
creates the `df_layer_surface` menu bar anchored top/left/right with an
exclusive zone, renders the QML offscreen into a `wl_shm` buffer, and tracks
`df_toplevel_manager` focus broadcasts for the app name. The compositor now
**composites** chrome surfaces above the window space
(`DfState::chrome_surfaces` + `render::chrome_render_elements` in the nested
and DRM backends), and the nested backend's pre-existing vertical-flip bug
(the winit/EGL back buffer needs `Transform::Flipped180`) is fixed. A live
nested capture confirms the menu bar renders at the top, correctly oriented,
with `eglgears` upright. `dragonfruit dev --shell` (and `make dev`) launches
it with the token the compositor provisioned. The restart and idle
acceptance criteria are now scripted: `shell_restart_reanchors_chrome_and_
preserves_windows` (in `shell_protocol_conformance.rs`) maps a window from an
independent client, crashes the shell connection, asserts the reserved zone
clears and the window survives, then reconnects a second shell with a fresh
up-front token and asserts the bar returns at 1280×28; and
`compositor/tests/shell_idle_trace.rs` (in `make e2e`) attaches a mapped menu
bar, lets it idle, and asserts `frames_rendered` stays flat. The bar is now
**interactive in a live session**: the compositor hit-tests chrome surfaces
above the window space (`input::chrome_under`), routes pointer/keyboard to
them honoring `df_layer_surface.set_keyboard_interaction`, and the shell binds
`wl_seat`, synthesizes Qt input into its offscreen scene, and opens the
dropdown on a separate `overlay` `df_layer_surface` (the bar stays a constant
28 px `top` surface; `ShellController` splits one offscreen render into the
bar strip and the popup rectangle and places the overlay by top/left margins,
`exclusive_zone = -1`, `keyboard = none`). A `--placeholders` demo app menu
stands in for T-22. The same work fixed a latent input bug: `surface_under`
returned a window-relative surface origin, so pointer/touch coordinates were
offset for any window not at the output origin. Open: background/bottom layer
stacking is T-10/T-11; status items are placeholders until T-20 and the app
menu is a stand-in until T-22. The last host-closable FR-1 item — output
hotplug re-anchoring — is now scripted: a chrome surface created without an
explicit output targets every output (wlr-layer-shell semantics), and the
headless backend's new opt-in synthetic-output harness
(`DRAGONFRUIT_SYNTHETIC_OUTPUT`) lets
`shell_output_hotplug_reanchors_chrome` attach/detach a second output and
assert the new `df_output`, its three Spaces, the bar's reserved zone, and
the chrome reconfigure. The overlay dropdown is scripted by
`overlay_popup_sits_above_the_bar_and_reserves_nothing` (popup configures to
its rectangle, reserves nothing, and is hit-tested in popup-local
coordinates). Deferred interaction polish (shell snapshot renderer): choppy
popup open/close, hover/drag-through timing, and a launch highlight on the
first title — the durable fix (commit from `QQuickWindow::afterRendering`
while the scene is dirty) is shared with T-10's Dock animation. Details and
hand-off: [PROGRESS.md](PROGRESS.md).

T-10 is partial (first slice): the Dock presentation and interaction core is
landed. `shell/dock/` now holds the real component built from the design
system — `Dock.qml`, `DockEntry.qml`, and `DockGlyph.qml` — with the entry
regions (apps | divider | minimized windows | Trash), one running indicator
per app entry, progress-based pointer-anchored cosine magnification, and
bottom/left/right placement plus the auto-hide translation. A new
`component.dock` token group (icon size/padding/gap, indicator, magnify
peak/falloff, Trash size, edge margin, reveal/hide delays) drives it; the
original app-tile/Trash artwork is our own geometry (14-risks.md). The shell
process creates a second offscreen QML scene and a `top`-layer `dock` chrome
surface (namespace `dock`, anchored bottom, reserved zone = the baseline bar
thickness, keyboard `none`), feeds it the running-app projection drained from
`df_toplevel_manager`/`df_toplevel` (grouped per app, minimized windows
excluded when `minimizeIntoTileIcon` is on), routes pointer input to it, sets
its input region to the visible bar slab only (the transparent magnified band
passes clicks through), and calls `df_toplevel_manager.activate_app` on a
click. `tst_dock.qml` (ctest `tst_dock`) covers the regions, indicators,
magnification anchor/peak, placement, auto-hide, activation, accessibility,
and artwork; a new headless conformance test
(`dock_surface_reserves_the_bottom_zone_and_coexists_with_the_bar`) asserts
the Dock and menu-bar reserved zones coexist and the Dock configures to its
requested size. Open for the remaining slices: context menus and the window
chooser, drag rearrangement and external drops, the GVfs-backed Trash state
and drop-to-trash, live settings persistence (`dock.*` keys, T-15) and the
Settings pane hooks (T-16), attention/launch bounce, the magnified-band input
region, per-output/per-position surfaces (left/right reserved zones are not
yet compositor-supported), and the scene-graph-driven render path shared with
the T-09 snapshot-renderer backlog. Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (launch slice): pinned apps can now be launched. The shell
holds an interim, deletable `.desktop` resolver/launcher (T-23 replaces it),
seeds and persists `dock.pinned` under
`$XDG_CONFIG_HOME/dragonfruit/settings.json` in the eventual
`org.dragonfruit.Settings1` key shape (T-15 adopts it without migration), and
merges the pinned set with the running-window projection into one ordered
entry list. Clicking a pinned or temporary entry activates a running app
(`df_toplevel_manager.activate_app`) or launches a not-running one through
`QProcess::startDetached`; the entry shows a launching state, resolves to
running on its first window, and degrades to a transient failure badge after a
bounded timeout. Unresolvable pinned identities render as a generic "not
found" entry with a disabled launch (never a crash). The new pure core
(`desktopentry`, `dockpins`, `dockmodel`) is unit-tested headless by
`tst_dockcore`; `tst_dock` covers the launching/failed/missing presentation.
Open for the remaining slices: context menus and the window chooser, drag
rearrangement and external drops, the GVfs-backed Trash state, live `dock.*`
settings and the Settings pane hooks, attention/launch bounce, the
magnified-band input region, per-output/per-position surfaces, and the
scene-graph-driven render path. Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (animation + input slice): the Dock now bounces. The
compositor's `attention` event (`xdg-activation`) is routed through
`ShellProtocol` to the owning app's entry, and a 16 ms shell animation clock
drives a launch bounce (three hops over 0.6 s, amplitude `B/4`) and a taller,
repeating attention bounce (amplitude `B/2`, bounded to 2 s, stopped on click
or focus — FR-4). The launch bounce keeps settling after the first window
resolves, so the entry never snaps mid-flight. Reduced motion removes the
translation and keeps the state legible as a subtle scale pulse. The
magnified-band input region is now real (FR-13): the Dock publishes the union
of the visible bar and the currently magnified/bouncing icon rectangles, the
shell commits them as a multi-rectangle `wl_region` every frame, and the
hidden auto-hide Dock publishes an empty region. The pure bounce clocks
(`dockLaunchBouncePhase`/`dockAttentionBouncePhase`) are unit-tested in
`tst_dockcore`, and `tst_dock` covers the launch/attention offset, reduced
motion, accessibility state, and the input-region membership. Open for the
remaining slices: context menus and the window chooser, drag rearrangement
and external drops, the GVfs-backed Trash state, live `dock.*` settings, and
the scene-graph-driven render path (the 16 ms clock still samples with
`grabWindow`). Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (menus + chooser slice): the Dock's app context menus and
window chooser are landed (FR-5, FR-10 app menus). The shell's running
projection now carries a per-app `windowList` (stable window handle, title,
focused/minimized, and resolved Space index/name) across every Space; the
window chooser lists those rows most-recent-first with a checkmark on the
frontmost and a minimized/Space marker, and selecting a row performs the
compositor round-trip (`select_overview_toplevel` restores, switches Space,
and focuses). A plain click on a running app with more than one window opens
the chooser; one window activates directly (section 8). The right-click menu
is a design-system `ContextMenu` carrying the live window list, Show All
Windows, Keep/Remove from Dock, Quit (closes every window; the private
protocol has no app-level quit), and Open for a not-running entry. Dock
popovers render through a second `overlay` chrome surface anchored bottom|left
(namespace `dock-popup`, no reserved zone); the shell grows the offscreen Dock
scene upward with headroom and commits the popover rectangle, and pointer
input is translated from popover-local back into the Dock scene. Open for the
remaining slices: the divider/Trash menus and Options submenu, Show in Files,
drag rearrangement and external drops, the GVfs-backed Trash state, live
`dock.*` settings (the divider toggles), and the scene-graph-driven render
path. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (drag rearrangement slice): pinned entries can now be
reordered, promoted, and removed by dragging (FR-9, section 12). A
`DragHandler` on each app entry lifts it (scale + shadow, magnification
suppressed, the whole surface keeps pointer input); the Dock computes a stable
insertion index from the pre-drag pinned slot centers and shifts the app
slots in `layout` without reordering the Repeater model — so the delegate that
holds the pointer survives. Dropping a temporary running app in the pinned
region promotes it ("Keep in Dock"); dragging a pinned entry off the Dock (or
off the surface) shows a "Remove" label and removes it. On drop the Dock emits
the complete ordered pinned set (`pinnedOrderChanged`), which the shell writes
to `dock.pinned` and rebuilds from. Reduced motion removes the gap spring.
External file/app drops and spring-loading remain deferred (they need the
Files/launcher drag sources, T-17/T-18). Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (live-settings slice): the Dock's `dock.*` keys are now a
live model, not just `dock.pinned`. A new pure `DockSettings` core
(`shell/src/docksettings.*`, unit-tested by `tst_dockcore`) reads and writes
every non-pinned key in the eventual `org.dragonfruit.Settings1` shape,
clamping and validating each value and re-reading on save so it and
`DockPins` never clobber each other's keys. The shell applies the model at
startup and watches the settings file (the interim stand-in for settingsd's
change signals, T-15) to re-layout live without a restart: `dock.size` maps
onto the new `iconSizeMin`/`iconSizeMax` token range, `dock.magnification`
now scales the peak (0.5 lands on the `magnifyPeak` token), and
`dock.animateOpening` gates the launch hop while attention still bounces.
Changing the size or auto-hide reconfigures the chrome surface in place
(`df_layer_surface.set_size`/`set_exclusive_zone`), so the reserved zone and
Zoom target follow live. The divider Control-click now opens a design-system
`ContextMenu` with the Turn Magnification On/Off toggle and Dock Settings…,
and `accessibility.reduceMotion` is bound onto the design-system
`Theme.reducedMotion` singleton. On the compositor side,
`LayerSurfaceState::reserved()` now understands left/right edges (a vertical
Dock reserves its side), covered by a new unit test and the
`vertical_dock_reserves_the_left_and_right_zones` conformance test. Position
switching (left/right), the vertical surface/popover geometry, and the
auto-hide reveal/hide state machine remain deferred to the per-position
surface slice; Trash, external drops, and the scene-graph render path are
unchanged. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (per-position surfaces slice): `dock.position` now really
moves the Dock. `ShellProtocol` anchors the Dock surface (and its popover
overlay) to the configured edge and sizes it to the baseline bar plus the
magnify band perpendicular to that edge, re-applying the anchor live so a
`dock.position` change needs no surface recreation. The shell tracks the
configured surface dimensions per edge, validates the first configure
(the compositor's pre-layout full-output configure is still ignored), and
pauses rendering across a position switch until the new geometry arrives so
no stale frame is committed. The QML layout mirrors for a vertical Dock: the
bar hugs the anchored edge, entries pack from that edge with the running
indicator against the screen edge, bounce moves into the magnify band, and
the auto-hide translation now goes off the correct edge (down for bottom,
left/right for a vertical Dock). Dock popovers open beside a vertical bar;
the shell grows the offscreen scene by a horizontal gutter and computes the
overlay margins from the Dock's edge (bottom margin for a bottom Dock,
left/right margin plus a top margin for a vertical one). A live geometry
change dismisses any open popover rather than letting it float detached. New
`tst_dock` cases cover the vertical bar/entry containment (magnified), the
side-edge hide translation, and the beside-the-bar popover; live headless
runs confirm `Dock configured 124x720` with the left/right reserved zone at
60 px and a live bottom→left change. The auto-hide reveal/hide state machine
is still open (the hidden translation is correct but nothing yet summons the
Dock from the edge); Trash, external drops, and the scene-graph render path
are unchanged. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (auto-hide reveal slice): the Dock's auto-hide state
machine is landed (section 15, FR-3). A hidden Dock keeps only a thin
`edgeTrigger` band of its surface in the input region, so the transparent
body still passes clicks through (FR-13) while a pointer at the output edge
can summon the Dock. The QML owns the reveal/re-hide clocks: a pointer
dwelling in the edge band for `dock.revealDelay` (120 ms) reveals it,
leaving the bar for `dock.hideDelay` (350 ms) hides it, and the hide is
suppressed while a context menu, window chooser, or drag is active. Enabling
`dock.autohide` starts hidden and disabling it always reveals; an
`attention` request reveals immediately. The translation snaps rather than
animates until the scene-graph render path (FR-14) can commit every frame
(the earlier lesson that an on-demand renderer must not freeze a half-played
animation). The divider menu's "Turn Hiding On/Off" toggle is wired now that
reveal works. `tst_dock` gains 8 cases (edge-band input region for all three
edges, hidden-Dock magnification suppression, dwell reveal, leave/re-hide,
leave-before-delay cancel, popover-suppressed re-hide, the hiding toggle,
and autohide-off no-op). Open: external-drag reveal, the global shortcut,
keyboard-focus suppression of re-hide, the animated translation, and the
remaining slices (external drops, Trash, Options submenu, per-output
sizing, scene-graph render path, a11y). Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 follow-up (popover dismissal): the Dock surface is now `OnDemand`
keyboard (section 20), and the compositor focuses a chrome surface on any
button press, not just left. Together with `onKeyboardFocused(false)` closing
the Dock popovers and `onKeyEvent` routing Escape to the Dock while a popover
is open, an open context menu/chooser now dismisses on Escape, a click on an
app, the desktop, or the menu bar, and on focus loss (section 13) — previously
only choosing a menu item closed it. New
`chrome_surface_takes_keyboard_focus_on_right_click` conformance test and a
`tst_dock` Escape case. Details: [PROGRESS.md](PROGRESS.md).

**Foundation milestone E2E (T-01…T-07).** The whole vertical slice now has a
headless end-to-end test, `compositor/tests/milestone_e2e.rs` (`make e2e`):
one live session with a shell client, a Wayland app, and an X11 app attached
at once. It drives the `df_core` handshake, menu-bar chrome with a reserved
zone, scene replay, live window announcements with identity, Space
activation and move-to-Space, the window state machine through the shell,
and a clean teardown (no surviving socket/token/`DISPLAY` file/orphaned
Xwayland). The test caught and drove the fix for a real compositor panic:
an X11 `WM_NAME`/`WM_CLASS` carrying its conventional trailing NUL reached
the private protocol's generated `CString` serializer and aborted the
process (see PROGRESS.md). The milestone gate now also runs
`compositor/tests/idle_trace.rs` (FR-2 steady state) and the extended
per-ticket conformance suites (malformed client/private traffic, full
event-pair coverage, and input-driven shortcuts/gestures/hot corners plus
interactive move/resize via the synthetic-input harness). Remaining for
the phase exit: a live nested and a
DRM run of the same slice (the E2E is headless, so it is CI-able), and
the shell's own chrome rendering (T-09/T-10).

1. **Foundation**
   - [x] Smithay compositor (event loop, protocol surface, render stack — T-02)
   - [x] Nested backend (live-verified on the dev host)
   - [ ] Native DRM backend (implemented; runtime-untested — needs a real seat)
   - [ ] Input (T-03 engine done: keymaps, shortcuts, gestures, hot corners, pointer constraints; headless synthetic-input + grab-refusal integration done; hardware validation + shell/portal wiring open)
   - [x] Outputs (wl_output + xdg_output globals, modes/scale/transform, hotplug)
   - [x] Windows (T-04 model: states, focus, placement, regions, move/resize, popups; headless toplevel-state + popup + protocol-level move/resize conformance covered)
   - [x] Workspace model (T-05: per-output lockstep Spaces, fullscreen Spaces, wallpaper color, app memory, hotplug migration; image wallpaper + switch animation + protocol wiring open)
   - [x] Xwayland (T-06: eager start + `DISPLAY` export + crash respawn, X11 window model integration, WM_CLASS identity, Tier-2 SSD marking, clipboard bridge; XDnD + app-matrix + GIO identity open)
   - [x] Private shell protocols (T-07: `df_core` handshake/trust, chrome surfaces + reserved zones, window/workspace/output control, compliance client; chrome rendering + Qt bindings + per-output zones open)
2. **Experience**
   - [x] Design system (T-08: token architecture + all 20 components + gallery/visual regression; app-level chrome lint + live AT-SPI dump deferred)
   - [ ] Top bar (T-09 partial: menu-bar render/interaction + shell bootstrap live, restart/idle scripts passing, chrome input routing + overlay-layer dropdown landed and scripted, output hotplug scripted; T-20 status adapters, T-22 app menu open)
    - [ ] Dock (T-10 partial: presentation/interaction core + shell `top` surface with bottom reserved zone + launch/pinned persistence + launch/attention bounce + magnified-band input region + app context menus and window chooser + drag rearrangement (reorder/promote/remove) + live `dock.*` settings + per-position (left/right) surfaces and vertical layout + edge-band auto-hide reveal/re-hide landed and scripted; external drops, Trash state, scene-graph render path open)
   - [ ] Window switching
   - [ ] Mission Control
   - [ ] Workspace gestures
   - [ ] Animations
3. **Flagship apps**
   - [ ] Settings, built entirely in our design system
   - [ ] Files, built entirely in our design system
4. **System integration** — all expose the APIs needed for custom frontends
   (see [design/07-system-integration.md](design/07-system-integration.md))
   - [ ] NetworkManager
   - [ ] BlueZ
   - [ ] PipeWire/WirePlumber
   - [ ] UPower
   - [ ] UDisks
5. **Desktop infrastructure**
   - [ ] Notifications
   - [ ] Idle/lock
   - [ ] Portal backend
   - [ ] Screenshot/screencast
   - [ ] Polkit/auth UI integration
   - [ ] Clipboard
   - [ ] Session lifecycle
6. **Compatibility**
   - [ ] Third-party decoration themes
   - [ ] DBusMenu global-menu bridge
   - [ ] StatusNotifier/AppIndicator support
   - [ ] Strange Xwayland applications
7. **Polish**
   - [ ] Multi-monitor transitions
   - [ ] Fractional-scale edge cases
   - [ ] Suspend/resume
   - [ ] Graphics-driver testing
   - [ ] Accessibility
   - [ ] Localization
   - [ ] Performance
   - [ ] Crash recovery

### Phase exit criteria

Each phase has a definition of done; phases do not "fade" into each other:

- **Foundation:** nested and DRM sessions both run the vertical slice;
  Xwayland windows map; session teardown is clean (no leaked VT master,
  no orphaned clients).
- **Experience:** the 30-second interaction loop runs with zero dropped
  frames on baseline Intel/AMD hardware; reduced-motion variants pass.
- **Flagship apps:** Settings fully configures every pane it ships; Files
  covers the MVP scope; both publish menu models and render identical
  traffic lights.
- **System integration:** every adapter degrades gracefully when its daemon
  is missing — verified by masking the systemd unit in a VM.
- **Desktop infrastructure:** the lock screen survives kill tests; portals
  work for a Flatpak browser (file chooser, screenshot, screen share).
- **Compatibility:** a DBusMenu-exporting Qt app shows a global menu; a
  StatusNotifier tray item renders in the menu bar.
- **Polish:** multi-monitor hotplug, 100 suspend/resume cycles, and
  fractional scaling pass scripted soak tests in a VM matrix.

## Performance budgets

Targets, checked on baseline Intel/AMD hardware, enforced inside the dev
loop rather than discovered in the polish phase:

| Path | Budget |
|---|---|
| Workspace switch / Mission Control | 60 Hz, no dropped frames for the full gesture |
| Input-to-photon latency | Under one frame, nested and DRM |
| Idle desktop | Zero damage, zero client wakeups from our shell |
| Animation system | One frame of compositor work per frame of animation |

## Engineering estimates

Assuming one very strong full-time Linux/Wayland engineer:

| Milestone | Estimate |
|---|---|
| Convincing prototype | 1–3 months |
| Plausibly daily-drivable (controlled Fedora hardware) | 6–12 months |
| Confidently given to arbitrary users | Beyond a year |

A genuinely "Apple-like premium" distribution-quality environment is
realistically a multi-year effort or a project for a small team. These are
engineering estimates, not upstream project timelines.
