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
| 1 · Foundation | T-01 ✅ done · T-02 🔄 partial (compositor core; visual effects split to T-33) · T-03 🔄 partial (input engine) · T-04 🔄 partial (window model) · T-05 🔄 partial (Spaces model) · T-06 🔄 partial (Xwayland) · T-07 🔄 partial (private shell protocols) · **T-33 ⬜ new (effects/materials)** |
| 2 · Experience | T-08 ✅ done (design system; app-level chrome lint + live AT-SPI dump deferred) · T-09 🔄 partial (menu bar + shell bootstrap + overlay-layer dropdown + scripted output hotplug; T-20/T-22 content open) · T-10 ✅ done (blocked items reassigned to owning tickets: drag source T-17/T-18, GVfs backends, per-output sizing T-11/T-16, live AT-SPI T-31; landed: Dock presentation core + shell surface + running entries/activation + launch/pinned persistence + launch/attention bounce + magnified-band input region + app context menus/window chooser + drag rearrangement (reorder/promote/remove) + live `dock.*` settings/divider menu/reduced motion + left/right reserved-zone foundation + per-position (left/right) surfaces, vertical layout, beside-the-bar popovers, corrected auto-hide translation and edge-band reveal/re-hide state machine + Trash state/count via a home-trash watcher, click-to-Files at `trash://`, and the Open/Empty Trash menu with an Empty Trash confirmation + Control-F3/Super+Option+D `focus-dock`/`toggle-dock` shortcuts and in-Dock keyboard navigation (arrows/Return/menu/type-jump, FocusRing, Escape `release_keyboard_focus` + design-system nested submenus and the divider Position on Screen submenu + external drops (a Wayland data-device drag destination, drop-to-pin / open-with-files / trash / Downloads, live insertion gap and drop highlight, spring-loading hook) + the Downloads stack (folder watch, listing popover, new-items badge, drop-to-move, the spring-load consumer) and recent/suggested apps + `MenuBarMenu` submenus + the scene-graph render path for both chrome surfaces (`afterRendering` commits, no sampling timer, FR-14) + the app `Options ▸` submenu (Assign To / Open at Login / Show in Files) and the corrected minimized-entry window list + the divider drag-to-resize handle (`dock.size` live preview/commit) and the section 5.1 overflow clamp (effective icon size + temporary/recent hiding, one warning per session) + per-output overlay popovers (a popover with no explicit output renders/hit-tests only on the output chrome was last focused on, so it does not float across displays) + the animated auto-hide reveal/hide (`motion.dockReveal`, FR-14) with keyboard-focus re-hide suppression and reveal-cancels-rehide) + the section 22 lifecycle edge-case suite (the Dock running-app projection extracted to a pure, unit-tested core: identity re-homing/merge, cross-Space window lists, minimize state, rapid open/close, missing identity, exit-mid-bounce) + the compositor client drag-and-drop walkthrough + the Trash-unavailable lifecycle state (dimmed entry, warning badge, disabled menu, inert click) and the multi-output Dock hotplug conformance test) + the click-tree activation conformance and the Phase-2 core-loop window-state round-trip (activate/restore/close, `Closed` emitted for a cross-Space window too); drag *source* (T-17/T-18), per-output sizing, live AT-SPI dump open) · T-11 🔄 partial (one overview state machine + trigger parity + commit/rubber-band/interruptibility + explicit hit-testing transfer + selection round-trip + live-surface translation for workspace slides + a compositor animation clock so keyboard/hot-corner switches slide on the same pipeline as gestures + shell Mission Control chrome on a full-output `overlay` surface: workspace strip incl. fullscreen Spaces and click-to-activate, minimized-window restore strip, selection round-trip + multi-monitor lockstep conformance test + the overview window grid with drag-window-between-Spaces (FR-7: `df_toplevel.move_to_workspace`, shell drop targets) + shell-request trigger parity and directional commits (U-7), hit-test-transfer conformance (U-8), reduced-motion wiring end to end via the additive v3 `set_reduced_motion` request plus a shell-chrome variant (U-1/U-6), per-frame timing instrumentation with a `DRAGONFRUIT_FRAME_TRACE` dump and a CI-budget conformance test (U-2), the grid layout algorithm spec (U-5), full-output overlay coverage so Mission Control spans every display (U-4), and the FR-2 live-surface frame-callback conformance (U-3); live-surface scale/clip/blur grid (T-33), per-output chrome sizing (T-16), and the FR-8 60 Hz hardware budget open) · T-12 … T-14 pending |
| 3 · Flagship apps | T-15 … T-19 pending |
| 4 · System integration | T-20 … T-23 pending |
| 5 · Desktop infrastructure | T-24 … T-29 pending |
| 6 · Compatibility | T-30 pending |
| 7 · Polish | T-31 pending |
| Cross-cutting | T-32 pending |
| **MVP milestone** | **T-34 ⬜ new (vertical-slice gate at the end of the MVP critical path)** |

> **Revised delivery order (working-desktop MVP).** The phases below remain
> the taxonomy, but delivery now follows the 30-second loop rather than the
> phase numbers. After the completed T-10 Dock work, the order is:
> T-02 remaining → **T-33 effects/materials** → **T-35 window/app lifecycle
> animations** → T-13 traffic lights → T-11 workspace switch then Mission
> Control → T-12 app switch → **T-20 MVP slice** (Wi-Fi/audio/battery) →
> T-09 menu-bar content → T-15/T-16 Settings → T-17/T-18 Files → T-24 session
> → **T-34 MVP gate**. T-33 fixes the "functional but looks bad" state; T-35
> owns the loop's window appear/minimize/restore motion; T-34 makes the loop
> an explicit, measured deliverable instead of a T-31 discovery. See
> [tasks/00-index.md](tasks/00-index.md) for the graph and the T-26 safety
> gate for real sessions.

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
pass-through policy. See [keymap.md](.//keymap.md) for the in-repo
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
[docs/xwayland-scaling.md](.//xwayland-scaling.md) (integer-scaled
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
[docs/private-protocols.md](.//private-protocols.md). The compliance
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
coordinates). The shell snapshot-renderer backlog item is now landed by the
T-10 FR-14 slice: the menu bar and the Dock both commit from
`QQuickWindow::afterRendering` (a `FrameCommitGate` suppresses the readback's
own re-render), so the popup open/close fade and hover timing no longer rely
on a sampling timer. Details and hand-off: [PROGRESS.md](PROGRESS.md).

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

T-10 follow-up (opaque magnify band): `Dock.qml`'s root `Rectangle` was left
with Qt's default white color, so the transparent magnified band rendered as
a white strip above the Dock bar in a live session. The root is now
`color: "transparent"` and `tst_dock` gained a pixel check that paints an
opaque backdrop and asserts the band shows through it. Details:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (Trash slice): the Dock's Trash entry is backed by real
state (section 16). A new pure `TrashMonitor` (in the Wayland-free dockcore
library) watches `$XDG_DATA_HOME/Trash/{info,files}` and reports a full flag
and item count; any filesystem event, including a deletion by a third-party
application, updates the entry within one event (FR-6), and idle contributes
zero polling (FR-8). GIO dev headers are absent on this host, so this is the
sanctioned filesystem fallback over the home trash (it never follows symlinks
and never leaves the root); the class is marked to be replaced by the GVfs
`GFileMonitor` / `g_file_trash()` backend when the headers are available.
Clicking Trash launches/activates Files at `trash://` through the interim
resolver (`org.dragonfruit.Files.desktop`; T-18 owns the app and the
`org.dragonfruit.Files1` activation target). The Trash context menu (Open,
Empty Trash) now exists; Empty Trash is disabled when the Trash is empty and
swaps the menu to a confirmation step before emitting the destructive
`empty_trash` action, which the shell performs on the home trash. The
design-system `ContextMenu` gained a `keepOpen` item flag (with design-system
tests) so a menu step can swap models without dismissing. New `tst_dockcore`
cases cover empty/full/count, the third-party watcher, the empty operation
(files, directories, paired removal), and unsafe-root refusal; five `tst_dock`
cases cover the Trash menu, disabled Empty Trash, confirmation, cancel, and
Open. Still open: drop-to-trash and drop-on-icon external drops (T-17/T-18),
Empty Trash progress for large trash, and the remaining slices (external
drops, Options submenu, per-output sizing, scene-graph render path, a11y).
Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (keyboard-navigation slice): the Dock is now
keyboard-navigable (section 20, the keyboard half of FR-11). Two system
shortcuts were added in the compositor — **Control-F3** dispatches
`focus-dock` and hands the keyboard to the `dock` chrome surface, and
**Super+Option+D** dispatches `toggle-dock` — both through the existing
`df_toplevel_manager.input_action` broadcast (no protocol change). The shell
tracks which chrome surface holds the keyboard (`ShellProtocol::
dockKeyboardFocused`), routes synthesized key events to the Dock scene while
it has focus, and applies `toggle-dock` to `dock.autohide`. The Dock QML owns
entry-to-entry navigation: Left/Right (Up/Down on a vertical Dock) move a
design-system `FocusRing` between entries skipping the divider,
Return/Space runs the shared click tree, Up/Menu opens the context menu, and
typing jumps by app name. The slice exposed and fixed a latent T-03 bug: the
input filter resolves a key's level-0 symbol (lowercase for letters) while
the binding table uses `KEY_Q`-style uppercase constants, so every letter
shortcut (`Cmd+Q`, `Cmd+Shift+N`, `Cmd+Ctrl+Q`) silently missed in a live
session; `ShortcutEngine::resolve` now folds ASCII letters. Scripted by nine
new `tst_dock` cases and the compositor conformance test
`focus_dock_shortcut_hands_the_keyboard_to_the_dock`. Open for the remaining
slices: external drops, GVfs Trash completion, the Options/Position
submenus, per-output sizing, the scene-graph render path, the live AT-SPI
dump, and a protocol path for Escape to release compositor keyboard focus.
Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (keyboard-focus-release slice): Escape can now leave Dock
keyboard navigation and hand the keyboard back to the active window
(section 20). The private `df_toplevel_manager` grew an additive
`release_keyboard_focus` request, so the interface is now **version 2** (the
other private interfaces stay at version 1); the compositor's existing
`restore_window_keyboard_focus` path is reused, and the request is inert when
no chrome surface held the keyboard or the active window is gone. The Dock QML
emits `keyboardFocusReleaseRequested` on Escape and clears its ring; the shell
routes it to `ShellProtocol::releaseKeyboardFocus()`, and the compositor's
`wl_keyboard.leave` then drives `onDockKeyboardFocused(false)` so the state
stays compositor-led rather than shell-faked. Scripted by a new `tst_dock`
case (`test_keyboard_escape_releases_focus`) and an extension to
`focus_dock_shortcut_hands_the_keyboard_to_the_dock` asserting the Dock's
keyboard leave. Open for the remaining slices: external drops, GVfs Trash
completion, the Options/Position submenus, per-output sizing, the scene-graph
render path, and the live AT-SPI dump. Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (submenu + Position slice): the design-system `ContextMenu`
now opens real nested submenus. A row with `type: "submenu"` (children in its
`submenu`/`items` array) opens a nested panel on a delayed hover or
Right-arrow, with Up/Down/Left/Escape/Return keyboard operation and the T-09
drag-through rule (moving into the panel does not dismiss it). It publishes
`contentRect` — the union of both panels, flipping to the menu's left when it
would leave its parent — so the Dock commits the nested panel into its
`overlay` surface without clipping. The Dock's divider Control-click menu
gained **Position on Screen ▸** (Bottom/Left/Right, checkmarked), which writes
`dock.position` through the live settings model and re-anchors/re-lays-out the
Dock immediately; the new `contextMenu.submenuDelay` token drives the hover
delay. The app **Options ▸** submenu (Assign To / Open at Login / Show in
Files) is still deferred: it needs compositor app/Space requests and T-18
Files, and `MenuBarMenu` submenus are a follow-up. Scripted by four
design-system cases and one `tst_dock` case. Open for the remaining slices:
external drops, GVfs Trash completion, the app Options submenu, per-output
sizing, the scene-graph render path, and the live AT-SPI dump. Details and
hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (external-drop target slice): the Dock is now a real
external drop target (section 12, FR-9). The shell binds the seat's
`wl_data_device_manager`/`wl_data_device` and reads `text/uri-list` (files)
or an application alias on drop; the compositor already routes the drag to
the chrome surface under the pointer, so no compositor change was needed. A
new pure `dockdrops` core (`shell/src/dockdrops.{h,cpp}`, in the Wayland-free
dockcore lib) parses the URI list, classifies an application alias, resolves
the drop action (pin / open-with-files / trash / Downloads / no-op), and
exposes the shared spring-load delay. `TrashMonitor::trash()` performs the
freedesktop home-trash move (unique name, `.trashinfo` record, copy fallback
across filesystems) and refuses the trash root. The Dock QML gained
`beginExternalDrag`/`externalDragTo`/`externalDragLeft`/`externalDrop`: an
application alias opens a live insertion gap (a placeholder entry reflows the
layout, safe because no QML DragHandler holds the pointer), a file drop
highlights the target entry, the whole surface stays interactive for the drag
(FR-13), and a stack hover starts the spring-loading clock. The controller
performs the resolved action: an app alias is added to `dock.pinned`,
files open through the app's Exec with `%f/%F/%u/%U` substitution, Trash gets
`TrashMonitor::trash()`, and the Downloads stack moves files into
`~/Downloads`. `tst_dockcore` gains the URI/action/trash tests and `tst_dock`
the external-drag cases. The remaining half is the drag *source* (Files /
launcher, T-17/T-18) and its end-to-end walkthrough; the app Options submenu,
per-output sizing, the scene-graph render path, and the live AT-SPI dump are
unchanged. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (Downloads stack + recents slice): the Downloads stack and
the recent/suggested-app entries are landed (section 17), and the stack is the
first **consumer of `springLoadRequested`**. A new pure `DownloadsMonitor`
(in the Wayland-free dockcore lib) is a zero-polling `QFileSystemWatcher` on
`$XDG_DOWNLOAD_DIR` that exposes a newest-first listing, an item count, and a
new-items badge (baseline on first scan, cleared when the stack is opened),
and performs the drop move-in with a unique-name/copy fallback. The Dock's
right region now has a stack entry immediately before the Trash, present even
when the folder is empty; `DockStackPopover.qml` lists the folder (header
opens the folder, a row opens the file, an "Empty" row, an "N more…" row), the
entry carries the badge and an "N items, M new" accessible name, and the stack
is excluded from drag rearrangement. `springLoadTimer` now opens the stack
popover when a drag dwells over it, so the drop can target it. A pure
`buildRecentEntries` appends up to three recently-focused apps (skipping
pinned/running/unresolvable identities) to the app region; the shell tracks
recency from focus changes and only passes it to `buildDockEntries` when
`dock.showRecentApps` is on. The shell opens files/folders through
`QDesktopServices` until T-18 Files owns "open", and handles the stack's
"Open Downloads Folder" menu action. Open: the **drag source** (Files/launcher,
T-17/T-18) and its end-to-end walkthrough, the GVfs Trash/downloads backends
when the GIO headers exist, the app Options submenu, per-output sizing, the
scene-graph render path, the live AT-SPI dump, and `MenuBarMenu` submenus.
Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (MenuBarMenu submenus slice): the design-system `MenuBarMenu`
now opens real nested submenus, the same model as the `ContextMenu` port. A
top-level row of `type: "submenu"` (children in its `submenu`/`items` array)
opens a nested panel beside the dropdown on a delayed hover (the
`contextMenu.submenuDelay` token) or Right-arrow, with Up/Down/Left/Escape/
Return keyboard operation; Escape closes the submenu before the dropdown, and
the panel flips to the menu's left near the window edge. `MenuBarMenu`
publishes `contentRect` — the union of the dropdown and any open submenu — and
the shell's `MenuBar` overlay geometry now uses it, so the committed popup
surface grows to contain the nested panel instead of clipping it. The
`Popup` component gained an `escapeCloses`/`escapePressed` hook so a menu can
handle Escape itself; the demo app menu (`--placeholders`) carries a "Sort By"
submenu and the gallery MenuPage shows one. Scripted by three design-system
cases and a `tst_menubar` overlay-geometry case. Open for the remaining
slices: the **drag source** (Files/launcher, T-17/T-18) and its end-to-end
walkthrough, the GVfs Trash/downloads backends when the GIO headers exist, the
app Options submenu, per-output sizing, and the live AT-SPI dump. Details and
hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (scene-graph render path slice): the Dock and the menu bar
now commit frames from the scene graph instead of a sampling timer (FR-14).
Both offscreen `QQuickWindow`s connect `afterRendering`; a frame the scene
graph renders schedules a commit on the next event-loop turn, and a small
pure `FrameCommitGate` (`shell/src/framecommitgate.h`, unit-tested by
`tst_dockcore`) suppresses the re-entrant frame that the `grabWindow()`
readback itself produces, so the commit path cannot loop. The 16 ms
`m_animationTimer` (menu popup fade) and `m_dockPopupTimer` (Dock popover
fade) bursts are gone: QML-driven animation — magnification, popover
fade/scale, the drag-gap `Behavior`, and the popover close tail — now reaches
the compositor every frame it renders, while the idle shell commits nothing
(zero wakeups, FR-8). The launch/attention bounce clock still ticks the model
at 16 ms (compositor-clock semantics) but no longer renders directly; the
resulting QML change drives the commit. Live headless smoke logs the one-shot
`scene-graph commit path active` line for both surfaces. Open for the
remaining slices: the **drag source** (Files/launcher, T-17/T-18) and its
end-to-end walkthrough, the GVfs Trash/downloads backends when the GIO headers
exist, the app Options submenu, per-output sizing, and the live AT-SPI dump.
Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (app Options submenu slice): the app context menu now
matches section 13. Every app entry carries an `Options ▸` submenu (the
design-system `ContextMenu` one-level submenu): Assign to This Desktop / All
Desktops / None, Open at Login (T-24), and Show in Files (T-18). The Dock's
menu model was also corrected so a minimized-window entry shows its owning
app's full window list plus Show All Windows and Quit (previously a dead
"Open"). "Assign to This Desktop" is real: the shell remembers the Space the
compositor last activated (`workspace_activated`) and moves the app's live
windows there with `df_toplevel.move_to_workspace`; the compositor has no
sticky/all-Spaces or "no assignment" state, so "All Desktops" and "None" are
logged pending (T-04/T-05). "Show in Files" resolves the app's executable and
opens Files at it through the interim `.desktop` launcher (T-18 replaces the
target); "Open at Login" is a T-24 entry point. `tst_dock` covers the Options
submenu contents/dispatch, the pinned-not-running menu, and the
minimized-entry menu. Open for the remaining slices: the drag source
(Files/launcher, T-17/T-18) and its end-to-end walkthrough, the GVfs
Trash/downloads backends when the GIO headers exist, per-output sizing, and
the live AT-SPI dump. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (divider-resize slice): the separator between the Dock's
app and minimized/Trash regions is now the resize handle (section 5). A
`DragHandler` on a wider invisible hit target centred on the 1 px divider
grows the icons as it is dragged away from the Dock centre and shrinks them
as it moves back, clamped to the `iconSizeMin`/`iconSizeMax` token range and
mapped onto `dock.size`. The Dock emits a live un-persisted preview on every
move and a single committed value on release; the shell re-lays-out and
reconfigures the surface through a new `applyDockSizeOnly` path that
deliberately skips `rebuildDockEntries`, so the `DragHandler` holding the
pointer is never reset (the internal-reorder lesson). `magnifying` and
auto-hide re-hide are suppressed during the resize and the whole surface
stays in the input region so the drag never leaks to a window (FR-13). Four
new `tst_dock` cases cover the preview/commit round-trip, clamping, the
suppression/input behavior, and a real mouse drag. Open for the remaining
slices: the drag source (Files/launcher, T-17/T-18), the overflow clamp
(section 5.1), the GVfs Trash/downloads backends when the GIO headers exist,
per-output sizing, and the live AT-SPI dump. Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (overflow clamp slice): a Dock whose content exceeds the
output is now an error state instead of silent clipping (section 5.1). A new
pure `applyDockOverflow` core (`shell/src/dockmodel.{h,cpp}`) computes the
largest icon size that fits the output length along the Dock axis and, only if
even the minimum icon size overflows, hides temporary/recent entries — recents
from the end first, then temporaries — while never dropping pinned or
minimized entries. The shell keeps the full built entry list, re-clamps on a
size, position, or configure change, and logs one warning per session; the
divider drag is bounded by the same clamp without resetting the Repeater model
(the delegate holds the pointer), and the effective clamped size is never
persisted so a later shrink restores the requested `dock.size`. The
`minimizeIntoTileIcon` setting is honored (hidden minimized entries do not
count). Content that still overflows at the minimum is clipped to the output
by the compositor's surface bounds. Six new `tst_dockcore` cases cover the
fit/no-op, the icon clamp, the recents-then-temporaries hiding order, the
never-drop-pinned error, the hidden-minimized flag, and the unknown-length
startup case. Open for the remaining slices: the drag source
(Files/launcher, T-17/T-18) and its end-to-end walkthrough, the GVfs
Trash/downloads backends when the GIO headers exist, per-output sizing, and
the live AT-SPI dump. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (per-output popover slice): an `overlay` chrome surface
created without an explicit output (a menu bar dropdown, the Dock
context-menu/window-chooser popover, a future OSD) is now **per-output**: it
renders and hit-tests only on the output a chrome surface was last focused on,
so a popover opened on one display no longer floats across every display
(section 18, "per-output popovers do not float across outputs"). The
compositor captures the interaction output from the pointer whenever a chrome
surface takes keyboard focus (`ShellProtocolState::chrome_focus_output`) and
clears it on focus loss; `LayerSurfaceState::visible_on_output` applies the
rule, and `chrome_surfaces` filters with it for both rendering and input, so
the drawn and interactive surfaces stay consistent. Persistent `top` chrome
(the menu bar, the Dock) still spans every output, and an output that detaches
while focused matches no remaining output, so its popover disappears. A new
`shell_protocol_conformance` case (`overlay_popover_is_per_output`) hotplugs a
second output, focuses chrome on the first, and asserts the popover is hit at
popover-local coordinates on the focused output but not on the second. Open
for the remaining slices: the drag source (Files/launcher, T-17/T-18) and its
end-to-end walkthrough, the GVfs Trash/downloads backends when the GIO headers
exist, per-output *sizing* (chrome still sizes its buffer from the first
output; T-11/T-16), and the live AT-SPI dump. Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (auto-hide motion slice): the auto-hide reveal/hide is now a
real animated slide instead of a snap. A new `motion.dockReveal` token (160 ms,
ease-out, reduced-motion duration 0) drives a `Behavior` on the Dock's
`hideOffset`; because the FR-14 scene-graph commit path delivers every frame,
the bar slides off (and back onto) its edge instead of jumping, and reduced
motion makes the transition instant. The section 15 re-hide suppression list is
now complete: `hideIfIdle`/`scheduleHide` refuse to hide while the Dock holds
keyboard focus, `reveal()` cancels a pending re-hide timer (a reveal could
otherwise be undone by a stale timer), and leaving keyboard navigation
(`endKeyboardNavigation`) restores the normal re-hide delay. External-drag
reveal was already handled by the T-13 external-drop slice. Five new `tst_dock`
cases cover the animated slide (mid-flight and settled), the reduced-motion
snap, keyboard-focus suppression of re-hide, and reveal cancelling a pending
re-hide. Open for the remaining slices: the drag source (Files/launcher,
T-17/T-18) and its end-to-end walkthrough, the GVfs Trash/downloads backends
when the GIO headers exist, per-output *sizing* (chrome still sizes its buffer
from the first output; T-11/T-16), the app Options Assign-To follow-ups
(T-04/T-05), and the live AT-SPI dump. Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (lifecycle-suite slice): the section 22 lifecycle edge-case
matrix now has a scripted, headless home. The Dock's running-app projection
was extracted from `ShellProtocol::emitDockState` into a pure core
(`shell/src/dockprojection.{h,cpp}`, dockcore lib) and `tst_dockcore` gained
seven cases covering identity change (re-home + merge), windows on another
Space (the window list carries the Space), all-windows-minimized, rapid
open/close (no stale rows), last-window-close (entry gone), and missing
identity (one generic group); `tst_dock` gained the exit-mid-bounce
presentation case (entry removed, bounce resolves, input region shrinks). The
protocol method is now a thin adapter over the pure builder, so the grouping
rules are unit-tested instead of unreachable inside a `wl_*` listener. Launch
failure was already scripted and is unchanged. Open for the remaining slices:
the drag *source* (Files/launcher, T-17/T-18) and its end-to-end walkthrough,
the Trash-with-Files integration test (T-18), the GVfs Trash/downloads
backends when the GIO headers exist, per-output *sizing* (T-11/T-16), the app
Options Assign-To follow-ups (T-04/T-05), the live AT-SPI dump, and the
app-index/settingsd restart rows of the lifecycle matrix (T-23/T-15). Details
and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (external-drag walkthrough slice): the compositor's
client-initiated drag-and-drop path now has a scripted end-to-end conformance
test (`shell_protocol_conformance::client_drag_and_drop_reaches_a_chrome_surface`).
Two distinct clients run against the headless compositor with the T-03
synthetic-input harness: a source maps a window, creates a `wl_data_source`
offering `text/uri-list` + the app-alias mime, and starts a drag with a
synthetic button serial; a raw shell stand-in (trusted `df_core` handshake,
a buffered `df_layer_surface`, its own `wl_data_device`) receives the offer as
the drag enters the chrome bar, accepts and negotiates an action, the drop
delivers, the source writes the payload through the pipe, and the test asserts
the bytes arrive intact. This closes the "scripted end-to-end walkthrough"
half of the external-drops hand-off and guards the compositor contract the
shell's completed DnD target depends on (the shell's own Qt plumbing cannot
run in a cargo test; `tst_dock` covers the QML logic). Open for the remaining
slices: the drag *source* in Files/launcher (T-17/T-18), the Trash-with-Files
integration test (T-18), the GVfs Trash/downloads backends when the GIO
headers exist, per-output *sizing* (T-11/T-16), the app Options Assign-To
follow-ups (T-04/T-05), the live AT-SPI dump, and the core-loop/60 Hz
measurements. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (trash-unavailable + multi-output hotplug slice): the
section 22 lifecycle row "Trash mount unavailable" is now implemented and
scripted, and the section 18 acceptance criterion "Dock appears on hotplug"
has a scripted conformance test. `TrashMonitor::isAvailable()` treats a
missing trash root as a healthy empty trash but reports an unreadable root
(or an unwritable nearest ancestor — a read-only/unmounted mount) as
unavailable; the shell forwards `trashAvailable` to the Dock, which dims the
entry, adds a warning badge, says "Trash, unavailable" in its accessible
name, degrades its menu to a single disabled row, and makes the click inert —
the session is never blocked. `tst_dockcore` covers the missing/unreadable
roots and `tst_dock` the dimmed/disabled presentation.
`shell_protocol_conformance::dock_follows_output_hotplug` attaches a second
synthetic output and asserts the new output specifically receives both the
menu bar's top and the Dock's bottom reserved zones and that the Dock surface
is reconfigured (the harness gained per-output/per-surface event tracking).
This closes the last multi-output acceptance checkbox; the remaining open
slices are the drag *source* (T-17/T-18), the Trash-with-Files integration
(T-18), the GVfs backends, per-output *sizing* (T-11/T-16), the live AT-SPI
dump, and the core-loop/60 Hz measurements. Details and hand-off:
[PROGRESS.md](PROGRESS.md).

T-10 continuation (click-tree activation slice): the Dock's two private-protocol
activation paths are now scripted and a real "most recent window" bug is fixed.
`dock_click_tree_activation_conformance` maps two windows of one app plus one of
another and drives `df_toplevel_manager.activate_app` (a plain click on a
running app entry, section 8) and `select_overview_toplevel` (a window-chooser
row, section 9/FR-5) over the protocol: `activate_app` must select the app's
most recent window even before any of its windows has held focus, restore a
minimized one, and `select_overview_toplevel` must switch to the window's Space
and focus it. The test caught that `WindowModel::recency` treated a newly mapped
window as the *least* recent until focused, so `activate_app` (and the app
switcher's `apps_by_recency`) picked the oldest never-focused window of an app —
the opposite of "most recent". `WindowModel::insert` now enters a window at the
front of `recency` (a window is most recent when mapped, and `touch_recency`
moves it to the front on focus), so a Dock click on a background app resolves to
its newest window. Open for the remaining slices: the drag *source*
(Files/launcher, T-17/T-18), the Trash-with-Files integration (T-18), the GVfs
backends, per-output *sizing* (T-11/T-16), the live AT-SPI dump, the core-loop/
60 Hz measurements, and the xdg-activation focus gap noted in PROGRESS (a
launched app's first window is not keyboard-focused until clicked; fixed by a
later slice). Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (core-loop close slice): the Phase-2 exit box — the core
interaction loop's compositor-observable window-state round-trip — is now
scripted and the box is checked. `dock_click_tree_activation_conformance`
was extended with the **close** step: `df_toplevel.close` (the Dock "Quit" action) must reach the client as
`xdg_toplevel.close`, the client tears its surface down, and the manager must
announce `Closed`; the test asserts this for the frontmost (active-Space)
window **and** for a window left on another Space. That second case exposed a
real compositor bug: `window_for_surface` only searched the active Space's
element list, so destroying a window on an inactive Space never resolved and
never emitted `Closed`, stranding a stale Dock chooser row / running indicator
(FR-5) — the lookup now falls back to the whole `WindowModel` (and then
pending/popup surfaces). The other unchecked boxes (60 Hz magnification on
hardware, Trash-with-Files, drag rearrangement walkthroughs, keyboard/AT-SPI)
are unchanged. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (acceptance-audit slice): two more acceptance boxes are
closed by audit rather than new machinery. The section 22 lifecycle edge-case
suite and the drag-rearrangement/context-menu walkthroughs had already been
scripted across `tst_dockcore` (pure running-app projection: identity
re-homing/merge, cross-Space window lists, rapid open/close, unknown app id)
and `tst_dock` (launch-failure badge, exit-mid-bounce animation resolution,
the real-mouse reorder/promote/remove/divider-resize drags, and the
entry/divider/Trash/Options menus with live window lists and confirmation
steps); they were simply never ticked. The one weak spot — FR-1's "bounce
stops, notice, no stuck running indicator" on launch failure — was
strengthened in `test_failed_launch_shows_badge` (asserts `running == false`,
no indicator, and `entryBounce == 0`). Still open and genuinely blocked:
Trash-with-Files integration (T-18), 60 Hz magnification on baseline hardware,
and the live keyboard + AT-SPI walkthrough (T-31; the keyboard half is
scripted). Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-10 continuation (xdg-activation focus slice): the deferred "a launched app's
first window is not keyboard-focused until clicked" gap is fixed. The
compositor's `XdgActivationHandler::request_activation` now resolves the
window through the model (it could be on another Space — the old
active-Space-only lookup dropped background requests entirely), always emits
the Dock attention signal (the design's authoritative launch feedback,
section 8 step 3/FR-4), and then **grants keyboard focus** when the window is
on the active Space (`activate_window_id`: restore if minimized, raise, set
the seat keyboard focus) so the launched app is usable without a click. A
window on a background Space keeps the attention bounce only and does not yank
the user to another Space. The shell's `onDockAttention` ignores an attention
request for the already-focused app, so FR-4's "attention stops on focus"
holds regardless of the focused/attention broadcast order. Scripted by the new
`xdg_activation_focuses_the_active_space_window` conformance test (active
Space: focus + attention; background Space: attention, no focus change, no
Space switch); `event_coverage_conformance` keeps the attention-event
coverage. Details and hand-off: [PROGRESS.md](PROGRESS.md).

T-11 (first slice): the compositor now has **one** overview state machine.
`compositor/src/overview/` wraps the T-03 progress pipeline (clamp →
rubber-band → velocity → commit) and adds the overview decisions: what a
transition is (`OverviewKind`: adjacent-Space switch, Mission Control,
Desktop Reveal), what a committed release applies (`TransitionCommit`),
explicit pointer-hit-test ownership (`InputOwner`), and the selection
round-trip. Every trigger — gesture, keyboard shortcut, hot corner, and the
shell's private-protocol enter/exit requests — drives the *same* machine;
`DfState::apply_overview_commit` is the single place a transition changes the
scene. A trigger arriving mid-transition cancels the in-flight one
immediately (FR-4); the commit rule (progress *or* release velocity) and the
rubber-band constants are the existing tunables. Hit-testing transfers
explicitly: while an overview-kind transition runs or the overview is open,
`surface_under` does not hit-test the window space until ownership returns
(FR-6). Workspace switching is a real progress pipeline over **live** window
surfaces: `DfState::apply_overview_scene` translates the active and adjacent
Spaces as the gesture progresses (this slice is translation-only; per-surface
scale/clip/blur are T-33's material passes), and the shell protocol already
carries the `progress` and `overview_changed` events. Scripted by 10 new
`overview` unit tests (trigger parity across keyboard/hot-corner/shell/gesture,
direction commit, toggle, mid-transition reversal, threshold, rubber-band,
hit-test transfer, selection round-trip, reduced motion) and a new headless
`shell_protocol_conformance::overview_state_machine_has_trigger_parity`
(a four-finger gesture opens the overview, the hot corner toggles it closed,
and a three-finger horizontal gesture slides the Space). The Mission Control
overview *chrome* (workspace strip incl. fullscreen Spaces, minimized-window
bottom strip, and the window grid with drag-window-between-Spaces, FR-7)
landed in the second and third slices, along with the compositor animation
clock and the multi-monitor lockstep conformance test. The remaining
unblocked work is now largely closed: reduced motion is wired end to end (the
additive `df_toplevel_manager` v3 `set_reduced_motion` request mirrors the
shell's `accessibility.reduceMotion` into the one machine, with unit and
conformance coverage for the single-step path — FR-9); `RenderStats` records
per-frame work durations and frame intervals, `dump_stats`/SIGUSR1 emits a
summary and (with `DRAGONFRUIT_FRAME_TRACE`) the full trace, and a headless
test asserts the gesture stays within a CI interval budget (FR-8
instrumentation, hardware run still open); the shell requests now drive the
shared pipeline with directional commits (U-7); a hit-test-transfer
conformance test proves window surfaces are unreachable while the overview
owns input (U-8); the shell chrome has an explicit reduced-motion variant
(U-6); the overview layout algorithm is specified as a grid in
[design/03-workspaces.md](design/03-workspaces.md) (U-5); and a full-output
overlay (anchored on all four edges) now spans every output so Mission Control
appears on each display (U-4 coverage; per-output sizing remains the T-16
seam), and the FR-2 live-surface conformance test proves a client surface stays
mapped and keeps receiving `wl_surface.frame` callbacks — and committing fresh
buffers — through the overview and a workspace round-trip (U-3; the headless
backend now delivers frame callbacks from its render hook). Still open: the
per-surface scale/clip/blur *live-surface* grid and the
wallpaper slide (T-33's reusable scene-transform pass), overview hit-testing
of window representations, and the FR-8 60 Hz hardware budget.
Details and hand-off: [PROGRESS.md](PROGRESS.md).

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
   - [ ] Top bar (T-09 partial: menu-bar render/interaction + shell bootstrap live, restart/idle scripts passing, chrome input routing + overlay-layer dropdown landed and scripted, output hotplug scripted, always-present system menu (dragonfruit mark) + application menu (Files on the desktop) landed; T-20 status adapters, T-22 app-exported menus, and the fixed-menu action wiring (T-16/T-24/T-26) open)
     - [ ] Dock (T-10 partial: presentation/interaction core + shell `top` surface with bottom reserved zone + launch/pinned persistence + launch/attention bounce + magnified-band input region + app context menus and window chooser + drag rearrangement (reorder/promote/remove) + live `dock.*` settings + per-position (left/right) surfaces and vertical layout + edge-band auto-hide reveal/re-hide + Trash state/count via a home-trash watcher with click-to-Files and the Open/Empty Trash menu with confirmation + Control-F3/Super+Option+D `focus-dock`/`toggle-dock` shortcuts with in-Dock keyboard navigation and an Escape `release_keyboard_focus` path and design-system nested submenus with the divider Position on Screen submenu + external drops (Wayland data-device drag destination, drop-to-pin/open-with-files/trash/Downloads, live insertion gap and drop highlight, spring-loading hook) + the Downloads stack (folder watch, listing popover, new-items badge, drop-to-move, the spring-load consumer) and recent/suggested apps landed and scripted + design-system `MenuBarMenu` submenus + the scene-graph render path for both chrome surfaces (`afterRendering` commits, no sampling timer, FR-14) + the app `Options ▸` submenu (Assign To / Open at Login / Show in Files) and the corrected minimized-entry window list + the divider drag-to-resize handle (`dock.size` live preview/commit) and the section 5.1 overflow clamp (effective icon size + temporary/recent hiding, one warning per session) + per-output overlay popovers (a popover with no explicit output renders and hit-tests only on the output chrome was last focused on, so it does not float across displays) + the section 22 lifecycle edge-case suite (pure running-app projection core: identity re-homing/merge, cross-Space window lists, minimize state, rapid open/close, missing identity, exit-mid-bounce) + the compositor client drag-and-drop walkthrough (a second source client drops `text/uri-list`/app-alias payloads onto a trusted chrome target, proving the offer/enter/drop routing the shell's DnD target relies on) + the Trash-unavailable lifecycle state (dimmed entry, warning badge, disabled menu, inert click, scripted in `tst_dockcore`/`tst_dock`) and the multi-output Dock hotplug conformance test (`dock_follows_output_hotplug`) + the click-tree activation conformance and the Phase-2 core-loop window-state round-trip (activate/restore/close, `Closed` emitted for a cross-Space window too); drag *source* (T-17/T-18), per-output sizing, live AT-SPI dump open)
    - [ ] Window switching (T-11 partial: the workspace-switch progress pipeline and its live-surface translation landed; the T-12 app switcher is a separate overlay)
     - [ ] Mission Control (T-11 partial: the one overview state machine, trigger parity, commit rules, hit-test transfer, selection round-trip, and the shell overview chrome — workspace strip incl. fullscreen Spaces, minimized-window restore strip, and the draggable window grid with move-between-Spaces (FR-7) — landed; shell-request trigger parity (U-7), hit-test-transfer conformance (U-8), reduced-motion wiring (U-1/U-6), frame-time instrumentation with a CI-budget trace (U-2), the grid layout spec (U-5), full-output overlay coverage across displays (U-4), and the FR-2 live-surface frame-callback conformance (U-3) landed; the live-surface scale/clip/blur grid (T-33), overview hit-testing of window representations, per-output chrome sizing (T-16), and the FR-8 60 Hz hardware budget open)
    - [ ] Workspace gestures (T-11 partial: trackpad gestures drive the same pipeline as keyboard/hot corner; wallpaper slide open (T-33), multi-monitor lockstep landed)
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
