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
| 1 · Foundation | T-01 ✅ done · T-02 🔄 partial (compositor core) · T-03 🔄 partial (input engine) · T-04 … T-07 pending |
| 2 · Experience | T-08 … T-14 pending |
| 3 · Flagship apps | T-15 … T-19 pending |
| 4 · System integration | T-20 … T-23 pending |
| 5 · Desktop infrastructure | T-24 … T-29 pending |
| 6 · Compatibility | T-30 pending |
| 7 · Polish | T-31 pending |
| Cross-cutting | T-32 pending |

T-01 is complete except one open item: the CI workflow is written and
all gates pass locally, but the first green run on a PR is still
pending. T-02 is partial: the calloop event loop, all three backends
(nested verified live on the dev host; DRM compiles, runtime-untested;
headless used by CI), the full standard protocol surface (CI-checked
advertise list), outputs, input routing, damage-driven rendering, and
teardown hardening are done. Open: performance budgets FR-2/3/4,
VRR/night light plumbing, multi-GPU runtime validation, direct-scanout
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
by 23 unit tests including a trigger-type matrix. Open: DRM/hardware
validation of multitouch + pen pressure; wiring the outbox to the shell
over the private protocol (T-07); menu-broker/portal accelerator
registration (T-22/T-27); per-device libinput acceleration/scroll live
application (needs backend device handles, T-16); on-device gesture
tuning; unclaimed-gesture pass-through policy. See
[keymap.md](../docs/keymap.md) for the in-repo keymap decision. Details:
[PROGRESS.md](PROGRESS.md).

1. **Foundation**
   - [x] Smithay compositor (event loop, protocol surface, render stack — T-02)
   - [x] Nested backend (live-verified on the dev host)
   - [ ] Native DRM backend (implemented; runtime-untested — needs a real seat)
   - [ ] Input (T-03 engine done: keymaps, shortcuts, gestures, hot corners; hardware validation + shell/portal wiring open)
   - [x] Outputs (wl_output + xdg_output globals, modes/scale/transform, hotplug)
   - [ ] Windows (scene machinery in T-02; window model is T-04)
   - [ ] Workspace model
   - [ ] Xwayland
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
