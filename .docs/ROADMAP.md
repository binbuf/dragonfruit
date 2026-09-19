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
| 2 · Experience | T-08 … T-14 pending |
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
session with no shell attached cannot grow without limit. Open:
DRM/hardware validation of multitouch + pen pressure; wiring the outbox
to the shell over the private protocol (T-07); menu-broker/portal
accelerator registration (T-22/T-27); per-device libinput
acceleration/scroll live application (needs backend device handles,
T-16); on-device gesture tuning; unclaimed-gesture pass-through policy.
See [keymap.md](../docs/keymap.md) for the in-repo keymap decision.
Details: [PROGRESS.md](PROGRESS.md).

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
fresh window. Open: the scripted shell-restart test (FR-9) — the state
is compositor-owned so the architecture supports it, but there is no
shell process to restart until T-08; the protocol-level `xdg_toplevel`
move/resize conformance suite (still unit-level; needs a seat button
grab to start the pointer grab); aspect hints (X11) and the
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
`scripts/gen-shell-protocol-bindings.sh` but not yet consumed until T-08;
VRR/night-light output requests are acked but not plumbed (T-16);
reserved zones are a single global union rather than per-output
(T-11/T-16); toplevel thumbnails for the Dock are deliberately absent
(token-gated if added, T-10); the `input_action`/`progress`/`hot_corner`/
`app_accelerator` compliance events still need the T-03 synthetic-seat
harness; the additive-only proof test (FR-7) is policy-enforced by the
lockstep handshake but not yet exercised because no interface has a
`since="2"` member. Details: [PROGRESS.md](PROGRESS.md).

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
event-pair coverage). Remaining for the phase exit: a live nested and a
DRM run of the same slice (the E2E is headless, so it is CI-able), and
the shell's own chrome rendering (T-09/T-10).

1. **Foundation**
   - [x] Smithay compositor (event loop, protocol surface, render stack — T-02)
   - [x] Nested backend (live-verified on the dev host)
   - [ ] Native DRM backend (implemented; runtime-untested — needs a real seat)
   - [ ] Input (T-03 engine done: keymaps, shortcuts, gestures, hot corners; hardware validation + shell/portal wiring open)
   - [x] Outputs (wl_output + xdg_output globals, modes/scale/transform, hotplug)
   - [x] Windows (T-04 model: states, focus, placement, regions, move/resize, popups; headless toplevel-state + popup conformance covered, move/resize still unit-level)
   - [x] Workspace model (T-05: per-output lockstep Spaces, fullscreen Spaces, wallpaper color, app memory, hotplug migration; image wallpaper + switch animation + protocol wiring open)
   - [x] Xwayland (T-06: eager start + `DISPLAY` export + crash respawn, X11 window model integration, WM_CLASS identity, Tier-2 SSD marking, clipboard bridge; XDnD + app-matrix + GIO identity open)
   - [x] Private shell protocols (T-07: `df_core` handshake/trust, chrome surfaces + reserved zones, window/workspace/output control, compliance client; chrome rendering + Qt bindings + per-output zones open)
2. **Experience**
   - [ ] Design system
   - [ ] Top bar
   - [ ] Dock
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
