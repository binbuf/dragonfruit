# Slicing review — are the 17 slices session-sized?

Reviewed 2026-09-23 against the repo as it stands (compositor ≈25.7k LOC /
169 Rust tests, shell ≈14.7k LOC, design system, Xwayland, private protocols,
Dock, menu bar, overview all landed but partial).

Scope of the review: **can this model (a fast, small-context coding agent)
complete each slice end to end in one working session?** The plan's own
[definition of done](../ROADMAP.md#the-rule-that-makes-it-work-definition-of-done)
is the bar: demo captured, scripted test green, no regression, reduced motion,
teardown, nothing silently deferred.

## Verdict up front

**No.** Most slices are 3–15 sessions of work, not one. Three problems, in
order of impact:

1. **The definition of done is not agent-completable as written.** "Demo
   captured and reviewed against the design docs" and the T-17 "unfamiliar
   user test" require a human to watch a running session. If every *slice*
   needs human sign-off, then no slice of any size is "done in a single
   session." The DoD has to be split into an *agent-completable* half
   (implementation + headless conformance + a capture artifact + notes) and a
   *human sign-off* half that is batched at track boundaries.
2. **Every slice spans more than one subsystem and more than three
   independently verifiable behaviours.** T-01 alone is: a new compositor
   render element, new input hit-testing, button actions, drag, double-click,
   a window menu, tier policy, a `make demo` harness, Dock/menu-bar
   integration, and two client-type conformance suites. That is not one
   session.
3. **The plan hides a hardware dependency on the critical path.** T-03 says
   it is "non-blocking," but T-12 (session) *depends on T-03*, and T-13 →
   T-14 → T-15 → T-16 → T-17 all depend on T-12. On a host with no free
   logind seat (this one — see `tasks/legacy/PROGRESS.md`), the entire back half of the plan
   is blocked, not parallel. The nested-testable parts of the session work
   must be separated from the DRM-only validation.

The order is *mostly* right for a critical path. It is not "sequential" — it
is a DAG with deliberate parallel rails (T-03, T-07/T-08). That is fine for a
team, but for one agent it needs to be linearised and re-cut.

## Rating legend

- **Fits** — one session, as written. (None qualify.)
- **Split N** — the slice is N coherent one-session units.
- **Track** — the slice is really a program of work (≥8 units) and should be
  relabelled as a track/milestone, not a slice.

| Slice | Verdict | Why it does not fit |
|---|---|---|
| T-01 Loop v0 | **Split 6** | New render element + input hit-test + 6 behaviours + demo harness + 2 client conformance suites |
| T-02 Loop v1 motion | **Split 4** | Clock + 4 distinct transitions + reduced motion + interruptibility + ghost lifetime + idle trace |
| T-03 Real session/perf | **Split 4** (2 host, 2 hardware) | DRM first bring-up is itself exploratory; plus input, budgets, soak, runbook |
| T-04 Materials | **Split 4** | Blur pass + shadows/rounding + reusable transform + degrade tiers + light/dark/reduced-motion |
| T-05 Mission Control live | **Split 6** | Transform + hit-test + cross-Space drag + image wallpaper + Desktop Reveal + budget |
| T-06 App switcher | **Split 2** | Compositor state machine; shell overlay |
| T-07 Menu bar live | **Split 6** | Adapter framework + 3 real daemons + menus + degradation; each adapter is a session |
| T-08 settingsd | **Split 3** | Daemon + API + persistence/migrations; consumer migration; restart/resync |
| T-09 Settings Wave 1 | **Split 6** | App shell + 4 panes + menu model + absence matrix |
| T-10 Files MVP | **Split 7** | files-core + ops + trash + UI shell + perf + Dock integration; two flagship-grade subsystems |
| T-11 Control Center | **Split 4** | Notification service + policy + panel + OSD |
| T-12 Session/lock/idle | **Split 5** (some hardware) | Session manager + DM entry + lock (security) + idle + suspend |
| T-13 Portals | **Split 7** | Backend + 4 portals + capture UI + clipboard + auth agent + Flatpak validation |
| T-14 Global menu/app index/compat | **Split 7** | app-index service + menu-broker + tray + DBusMenu + XDnD + zoo + retire interim |
| T-15 System breadth | **Track ~16** | 8 adapters × panes × tiles; the single largest item in the plan |
| T-16 Polish/packaging | **Track ~11** | Multi-monitor + scaling + soak + drivers + a11y + i18n + crash + 2 distro packages + perf |
| T-17 Premium gate | **Split 6 verification units** | Not implementable; a checklist that requires hardware and human review |

## The DoD split (do this first)

Replace the single "done" bar with two:

**Agent-completable (per unit).**
- code merged and building (`cargo build`, CMake build);
- headless conformance test added to `make e2e` and green;
- `make check` / `make soak` unchanged;
- a capture artifact committed under `docs/captures/` and a short
  `PROGRESS.md` note (what was observed, deviations);
- reduced-motion/degrade variant exercised where the unit adds an animation.

**Human sign-off (batched, per track).**
- watch the demo in a live nested session and compare against the design docs
  and gallery goldens;
- tick the slice's acceptance boxes;
- the T-17 unfamiliar-user test.

This does not weaken the rule ("a Dock nobody can use because windows have no
titlebar"). It moves the un-automatable part out of the per-unit critical
path so units can actually be finished by an agent.

## Proposed one-session units

IDs are `<slice>.<n>`; existing slice files stay the design reference. Each
unit is meant to be a single agent session with its own headless test and
capture note.

### T-01 — SSD window controls
- **T-01.1** Titlebar render element: token geometry/insets + flat fill +
  traffic-light *drawing* (hover/disabled states), no input yet. Test:
  element exists with expected insets for a mapped toplevel.
- **T-01.2** Traffic-light *actions*: click → close/minimize/zoom through the
  existing state machine; hit-test regions; hover reveal.
- **T-01.3** Titlebar drag-to-move + double-click honoring
  `dock.titlebarDoubleClick` + fullscreen hover reveal.
- **T-01.4** Window menu (Move to Space/Minimize/Zoom/Close) on
  right/Control-click.
- **T-01.5** Tier policy correctness: SSD/CSD/X11 no-double-decoration, with
  X11 protocol conformance.
- **T-01.6** `make demo` harness (build + nested launch + shell + Qt app + X11
  app + printed checklist; same path CI uses) and the Dock/menu-bar
  integration walkthrough capture.

### T-02 — lifecycle motion
- **T-02.1** Animation clock + per-window transform + **appear**, reduced
  motion + zero-damage-when-idle assertion.
- **T-02.2** Minimize/restore + shell→compositor Dock tile geometry hand-off.
- **T-02.3** Zoom/fullscreen transitions.
- **T-02.4** Close ghost + interruptibility + orphan-free teardown + capture.

### T-03 — real session & perf (two rails)
- **T-03.1 (host)** Nested budget work: 60 s idle trace, one-frame-per-
  animation, direct-scanout counter template, latency instrument; record raw
  numbers.
- **T-03.2 (hardware)** DRM single-GPU bring-up: LibSeat/udev/outputs/vblank/
  scanout/cursor. Expect this to need follow-ups.
- **T-03.3 (hardware)** Hardware input validation (mouse/keyboard/touchpad/
  gestures/hot corners/tablet/one non-US layout).
- **T-03.4 (hardware)** DRM soak + teardown + runbook + on-hardware latency.

### T-04 — materials
- **T-04.1** Real shadows + rounded-corner clipping, token-driven, for chrome,
  SSD titlebars, floating windows.
- **T-04.2** Backdrop blur pass for chrome surfaces, honoring client
  translucency.
- **T-04.3** Reusable scene-transform pass (scale/translate/clip/blur) +
  one-effect-pass-per-frame assertion.
- **T-04.4** Degrade tiers + light/dark + reduced-motion render correctness +
  visual-floor sign-off package.

### T-05 — Mission Control live
- **T-05.1** Live-surface transform into the documented grid (scale/clip/blur,
  never thumbnails).
- **T-05.2** Hit-testing + selection on live representations.
- **T-05.3** Drag a live representation between Spaces.
- **T-05.4** Image wallpaper sampling + per-Space slide (color fallback).
- **T-05.5** Desktop Reveal through the same pipeline.
- **T-05.6** Full-gesture frame budget + degrade + capture.

### T-06 — app switcher
- **T-06.1** Compositor switcher state machine + `app_switcher` protocol
  conformance (open/cycle/reverse/commit/cancel, recency).
- **T-06.2** Shell overlay with live previews + commit path + Cmd+` + a11y +
  reduced motion + capture.

### T-07 — menu bar live
- **T-07.1** Adapter framework: contract, mock, event subscription,
  re-subscribe on restart, absent/error states.
- **T-07.2** Networking adapter (NetworkManager D-Bus) + menu.
- **T-07.3** Audio adapter (PipeWire/WirePlumber) + menu.
- **T-07.4** Power adapter (UPower) + menu.
- **T-07.5** Remove `--placeholders`; wire status items to adapters; keyboard
  a11y.
- **T-07.6** Absent-daemon masking matrix + idle trace + capture.

### T-08 — settingsd
- **T-08.1** Daemon core: config model + `org.dragonfruit.Settings1` +
  persistence/atomic writes/migrations + unit/integration tests.
- **T-08.2** Consumer migration: shell drops `DockSettings`/`DockPins`,
  design-system `Theme` binding, compositor motion/input policy, delete the
  file watcher in the same change.
- **T-08.3** Restart/resync semantics + kill test + key-schema doc + capture.

### T-09 — Settings Wave 1
- **T-09.1** Settings app shell (sidebar, local search, chrome, traffic
  lights, a11y, live-apply plumbing).
- **T-09.2** Appearance pane.
- **T-09.3** Wallpaper pane.
- **T-09.4** Desktop & Dock pane.
- **T-09.5** Displays-basic pane.
- **T-09.6** Menu-model publication + absent-provider matrix + capture.

### T-10 — Files MVP
- **T-10.1** files-core: streaming listing + sorting + model + tests.
- **T-10.2** files-core operations (rename/new folder/move/copy/delete) with
  optimistic semantics.
- **T-10.3** files-core trash (GVfs/freedesktop fallback) + folder watcher.
- **T-10.4** Files UI shell (window/toolbar/sidebar/list+icon views/
  multi-select/context menus).
- **T-10.5** Performance: warm-1k <50 ms + 100k scroll 60 Hz windowed +
  benchmarks.
- **T-10.6** Dock integration: one Trash source, drop-to-trash, Empty Trash,
  Show in Files, `trash://`, Downloads opens, `.desktop` identity.
- **T-10.7** Capture + acceptance walkthrough.

### T-11 — Control Center + notifications + OSD
- **T-11.1** Notification service (`org.freedesktop.Notifications`) + history
  + actions + D-Bus tests.
- **T-11.2** DND/Focus policy + menu-bar reflection + replace Dock failure
  path.
- **T-11.3** Control Center panel + live-apply tiles (only adapters that
  exist — see ordering note).
- **T-11.4** OSD (volume/brightness) + reduced motion/fullscreen + keyboard/
  a11y + capture.

### T-12 — session + lock + idle (nested first, then DRM)
- **T-12.1** Session manager/services + env + systemd user units + restart
  policy (nested-testable).
- **T-12.2** Display-manager session entry + startup/logout + clean teardown
  (nested-testable; DRM entry validated in T-03 rail).
- **T-12.3** Lock screen (`ext-session-lock-v1` + UI + PAM helper + input
  capture + kill-resistance) — security gate.
- **T-12.4** Idle timers + inhibitors (nested-testable).
- **T-12.5** Suspend/resume one cycle + policy keys + kill matrix + capture.

### T-13 — portals + capture + clipboard
- **T-13.1** Portal backend skeleton + Settings + GlobalShortcuts + session
  service + absent-frontend behavior.
- **T-13.2** FileChooser portal (files-core) + test client.
- **T-13.3** Screenshot portal + shortcut path + region/window/fullscreen +
  save/copy.
- **T-13.4** ScreenCast portal (PipeWire) + source picker (+ stills-only
  fallback if timeboxed).
- **T-13.5** Clipboard text/image/`text/uri-list` round-trips + history if
  specified.
- **T-13.6** polkit auth agent.
- **T-13.7** Flatpak/browser end-to-end + no-capture gate + capture.

### T-14 — global menu + app index + compat
- **T-14.1** app-index service (identity resolution, themed icons,
  install/update events, launch registry, recency, subscription); replace the
  interim resolver.
- **T-14.2** menu-broker (export model, fixed menu with live state,
  accelerator registration, Settings toggle).
- **T-14.3** StatusNotifier/AppIndicator tray bridge.
- **T-14.4** DBusMenu bridge.
- **T-14.5** XDnD bridge.
- **T-14.6** Strange-app zoo run + fixes + matrix capture.
- **T-14.7** Retire interim paths (`.desktop` resolver, demo menu) +
  regression.

### T-15 — system breadth (a track, not a slice)
One unit per subsystem, each = adapter + pane + Control Center tile + absence
case (the no-half-panes rule makes splitting adapter from pane fragile):
Bluetooth · Storage/media · Sound/routing · Keyboard/Mouse/Trackpad ·
Mission Control/hot corners · Battery/power profiles · Notifications/Focus ·
Lock Screen policy · Menu Bar config · General/About/Updates · Users & Groups ·
Printers & Scanners · Privacy & Security · Accessibility · Network advanced
(VPN) · final absence matrix + capture. **≈16 units.**

### T-16 — platform polish + packaging (a track)
- **T-16.1** Per-output chrome sizing/reserved zones + per-output window
  placement.
- **T-16.2** Hotplug under load + lockstep transitions.
- **T-16.3** Fractional scaling (integer Xwayland + viewport downscale +
  chrome sizing).
- **T-16.4** 100-cycle suspend/resume soak.
- **T-16.5** Driver matrix (Intel/AMD baseline, NVIDIA recorded outcome,
  GPU-loss).
- **T-16.6** Accessibility audit (AT-SPI dump, keyboard-only, magnifier if
  specified, reduced-motion sweep).
- **T-16.7** Localization/i18n + locale formatting.
- **T-16.8** Crash-recovery kill matrix + docs.
- **T-16.9** Fedora packaging + CI.
- **T-16.10** Debian packaging + CI.
- **T-16.11** Packaged-build perf re-measure + reports/capture. **≈11 units.**

### T-17 — premium gate (verification units)
- **T-17.1** Nested full-loop verification + capture.
- **T-17.2** DRM full-loop verification + capture.
- **T-17.3** Visual floor + reduced-motion sign-off.
- **T-17.4** Performance-budget verification.
- **T-17.5** Robustness/crash/teardown verification.
- **T-17.6** Unfamiliar-user test + product judgment + sign-off report +
  post-gate backlog.

## Sequential order (single-track)

Linearised so no unit depends on a later one. Hardware is a rail that cannot
block the nested path.

1. T-01.1 → T-01.2 → T-01.3 → T-01.4 → T-01.5 → T-01.6
2. T-02.1 → T-02.2 → T-02.3 → T-02.4
3. T-04.1 → T-04.2 → T-04.3 → T-04.4
4. T-05.1 → T-05.2 → T-05.3 → T-05.4 → T-05.5 → T-05.6
5. T-06.1 → T-06.2
6. T-07.1 → T-07.2 → T-07.3 → T-07.4 → T-07.5 → T-07.6
7. T-08.1 → T-08.2 → T-08.3
8. T-09.1 → T-09.2 → T-09.3 → T-09.4 → T-09.5 → T-09.6
9. T-10.1 → T-10.2 → T-10.3 → T-10.4 → T-10.5 → T-10.6 → T-10.7
10. T-11.1 → T-11.2 → T-11.3 → T-11.4
11. T-12.1 → T-12.2 → T-12.3 → T-12.4 → T-12.5
12. T-13.1 → T-13.2 → T-13.3 → T-13.4 → T-13.5 → T-13.6 → T-13.7
13. T-14.1 → T-14.2 → T-14.3 → T-14.4 → T-14.5 → T-14.6 → T-14.7
14. T-15 (16 units, subsystem order)
15. T-16.1 → … → T-16.11
16. T-17.1 → … → T-17.6

**Hardware rail:** T-03.2/3/4 sit alongside step 2 onward; T-03.1 (host) can
run at step 2. The rail must complete before T-17.2 and T-16.5. If no seat is
available, mark the rail open exactly as the current plan says — but do not
let it gate T-12/13/14/15/16 (see ordering notes).

## Ordering / dependency corrections

1. **Do not gate T-12+ on T-03.** T-12 lists `Depends on T-03, T-08`. T-03 is
   hardware and explicitly non-blocking, yet T-12–T-16 all sit behind it.
   Change T-12's develop-dependency to T-08 (T-03 is validation-only for the
   DRM capture), and move the DRM session entry validation onto the T-03
   rail. Otherwise a hostless environment stalls the whole back half.
2. **T-11's Bluetooth tile is a forward dependency.** T-11 ships before T-15
   but its panel includes "Bluetooth (state from T-07/T-15)". Either move the
   Bluetooth adapter (T-15's first unit) before T-11.3, or ship T-11.3 without
   Bluetooth and add the tile in T-15.
3. **Make the parallel rails explicit.** T-03 (hardware), T-07, and T-08 have
   no dependency on T-02/T-04/T-05/T-06. If a strict single sequence is
   desired, they slot after T-01; if not, label them rails so an agent does
   not read the array as a strict order it cannot start.
4. **T-07 depends on T-01 only in the header, but its real prerequisite is
   the shell's status-slot plumbing**, which exists. Keep T-07 early so the
   bar stops being a prototype while the loop slices continue.
5. **`make demo` is T-01.6 and every later slice's acceptance references it.**
   It must land as its own unit before any later slice claims a demo, or the
   later slices inherit an impossible dependency.
6. **T-17 is a checklist, not a slice.** Decompose as above; do not leave a
   single 100-line checklist as the final "slice."
7. **Naming.** T-15 and T-16 are tracks (≈16 and ≈11 units). Relabel them
   `Track` in the index so nobody attempts them as one ticket.

## Adopted

The re-cut above was applied on 2026-09-23: the 17 slice files are now track
design references (each annotated with its unit map), and the executable
sequence lives in [../ROADMAP.md](../ROADMAP.md) → [tasks/units/](../tasks/units/). The definition
of done was split into an agent-completable half and batched human sign-off.

Two residual risks remain and should be re-checked at each track boundary:

- **T-15 units are the largest.** Each subsumes an adapter + pane + tile for one
  subsystem; some (Bluetooth, Storage, Printers, Users) may still need a further
  adapter/pane split when first attempted.
- **The hardware rail is real.** This host has no free logind seat, so T-03.2–4,
  T-16.4/5/9–11, and T-17.2/4 cannot be completed here. They must be tracked as
  open and swept together, not dribbled out and forgotten.