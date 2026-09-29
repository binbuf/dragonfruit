# Session Management and Development Workflow

## Summary

The desktop runs as its own Wayland session under the display manager —
alongside GNOME, never instead of it during development. There is **no live
compositor handoff**: Wayland clients are connected to a specific compositor,
and when that compositor exits, clients have no standardized way to migrate
live windows into a different compositor. "Gracefully swap out the current DE
and put it back" is therefore not our architecture.

## Nested mode is the daily workflow

The compositor is built with a Wayland (nested) backend from day one:

```text
Fedora / GNOME
└── our compositor running in a normal window
    ├── our menu bar
    ├── our Dock
    ├── our workspaces
    ├── Settings
    ├── Files
    └── test Wayland applications
```

Development command:

```bash
dragonfruit dev --nested
```

It creates its own Wayland socket and launches our shell, services, and apps
against it. When you quit the nested compositor, **GNOME was never
disturbed.**

This gets roughly 80–90% of everyday development: layouts, menus, Dock, app
launching, workspace logic, Files, Settings, animations, and most protocol
work.

### The demo harness: `make demo`

Every vertical slice is verified through one command, on the same path CI
uses:

```bash
make demo          # nested: shell + a Qt/Wayland app + an X11 app + checklist
make demo DEMO_ARGS=--headless   # the scripted half, forced headless
```

`make demo` builds the tree (Cargo + CMake) and then launches the nested
compositor, the shell, a first-party Qt/Wayland app, and an X11 app against
the private socket, and prints the checklist of steps the human performs at
track sign-off (drag, double-click zoom, minimize/restore, close, window
menu). Closing the Dragonfruit window ends the session; teardown is
deterministic and leaves the host session untouched. `--launch CMD...` (may
be repeated) adds extra programs to the session.

With no host Wayland session — CI — the same target falls back to the
**headless scripted half**: launch the compositor, shell, and clients,
settle, assert every child is still alive, tear down, and assert no socket
leaked. That is the launch+teardown smoke wired into `make e2e` (which runs
`make demo DEMO_ARGS=--headless`) and the CI workflow. The X11 half is
skipped, with a note, when Xwayland or the X11 demo app (`xmessage`) is
missing; a Wayland-only session is a normal state.

`DF_DEMO_QT_APP`, `DF_DEMO_X11_APP`, and `DF_QML_IMPORT_PATH` override the
app/QML paths the harness discovers under `build/`.

### Teardown soak and the leak/lock gate

The repeated-loop leak tripwire is `dragonfruit dev --soak N`: N compositor
sessions, each asserting its exit removed the socket, the socket lock, both
launch-token hand-off files, and the Xwayland **`DISPLAY` file**, and left no
orphaned client (`soak::teardown_artifacts` + `soak::dragonfruit_processes`).
`make soak` and `make check` run 100 cycles; `dragonfruit dev --soak 1 --drm`
runs one real backend cycle on the hardware rail.

`make t17-leak-lock-soak` is the premium-gate verification
(`scripts/t17-leak-lock-soak.sh`): it runs that soak **and** the
`session_lock_conformance` suite each under an isolated `XDG_RUNTIME_DIR` and
asserts the directory is empty afterwards, runs the lock suite `--release`,
and records the DRM/VT-master probe. Lock enforcement is proved there as:
trusted lock covers every output, an untrusted client is refused, a killed
lock UI stays locked (fail-secure), and 25 lock/unlock cycles each return to
`lock locked=0 surfaces=0` with no lock-surface leak. Compositor integration
tests remove their own runtime artifacts in `Drop` via
`compositor/tests/common`, so repeated test loops no longer accumulate stale
files. See [ADR 0178](adr/0178-t17-leak-lock-enforcement.md).

### The thin/full session split: `make dev-full`

`make dev` and `make demo` are deliberately **thin**: they start the core
services only (`dragonfruit-app-index` and `dragonfruit-menu-broker`, the Dock
identity and global menu the shell chrome needs) and never touch the
developer's real settings. The real session services —
`dragonfruit-settingsd`, `dragonfruit-system-status`,
`dragonfruit-notifications`, and `dragonfruit-wallpaperd` — are opt-in:

```bash
make dev-full              # nested session with the full service set
make dev-full FIXTURES=1   # ... and the deterministic DF_*_FIXTURE pane data
```

`make dev-full` runs the same nested compositor and shell on a **private
session bus** with scratch `XDG_CONFIG_HOME`/`XDG_CACHE_HOME`/`XDG_STATE_HOME`
under the session runtime dir. It is therefore safe alongside another desktop
(GNOME, KDE, Sway) on the workstation, and it can never read or overwrite the
developer's real settings or wallpaper cache. It waits for the shell-critical
bus names (`Settings1`, `AppIndex1`, `MenuBroker1`, `SystemStatus1`) before
starting the shell. Host daemons (BlueZ, UDisks2, WirePlumber, UPower) remain
optional — absence is a normal state — so `FIXTURES=1` is the deterministic
alternative when the hardware is not there.

The split is recorded in
[ADR 0123](adr/0123-thin-demo-and-full-dev-session.md). `wallpaperd` is never
on the thin path, and `make demo`'s headless scripted half starts **no**
services, so CI never warms the provider's network cache. The per-pane capture
scripts under `scripts/` still provision their own scratch daemons and
fixtures; `make dev-full` is the general-purpose version of what they do.

### Capturing the walkthrough

`scripts/capture-demo.sh` performs the checklist against the live nested
session instead of a human. It launches `make demo` with the compositor's
synthetic-input harness bound (`DRAGONFRUIT_SYNTHETIC_INPUT`, installed on
the nested backend as well as headless — test plumbing only, never set in a
real session), drives focus, the window menu, zoom, minimize,
restore-from-Dock, and close through the same `process_input_event` router a
real device uses, and screenshots each step with Spectacle. The stills and a
short walkthrough clip land in `docs/captures/`. It needs a host Wayland
session, Spectacle, ffmpeg, and Pillow, so it is deliberately not part of
`make e2e`.

`scripts/capture-t17-window-loop.sh` (`make t17-window-loop-capture`) is the
T-17.1a verification capture: it runs the same synthetic-input loop but on a
real third-party Qt SSD client (`kcalc`) started against the private socket,
records the exact titlebar drag, zoom, minimize, restore, and close, and also
captures the first-party CSD client and the X11 client. The stills, clip, and
machine transcript land under `docs/captures/t17-window-loop.*`; the harness
and its evidence boundary are
[ADR 0159](adr/0159-t17-nested-window-loop-capture.md).

`scripts/capture-t17-navigation.sh` (`make t17-navigation-capture`) is the
T-17.1b verification capture: it drives workspace switching, Mission Control,
and app switching on the same synthetic-input nested session, by pointer and
keyboard, asserting each path through `query spaces`/`query grid`/`query
wallpaper`/`query switcher`. The stills, clip, and machine transcript land
under `docs/captures/t17-navigation.*`; the harness and its evidence boundary
are [ADR 0160](adr/0160-t17-navigation-capture.md).

`scripts/capture-t17-flatpak-browser.sh` (`make
t17-flatpak-browser-capture`) is the T-17.1c verification capture: a private
session bus runs the real `xdg-desktop-portal` frontend with the Dragonfruit
backend and a real Flatpak browser (`org.mozilla.firefox`) is executed *inside
its sandbox* against the nested session, with the **live shell** as the portal
presenter. The driver completes file-choose, screenshot, and screen-share by
synthetic input and the run fails unless the client's portal response returns a
file URI, a screenshot URI, or a stream list. The stills, clip, and machine
transcript land under `docs/captures/t17-flatpak-browser.*`; the harness and its
evidence boundary are [ADR 0161](adr/0161-t17-flatpak-browser-capture.md).

### Tracing a pane switch

To find where a Settings pane switch spends its time, run the nested demo with
two diagnostic env vars:

```sh
DRAGONFRUIT_FRAME_TRACE=1 DF_SETTINGS_TRACE=1 make demo
```

Both sides then emit wall-clock `DFTRACE <event> ... t=<epoch-ms>` lines that
correlate across the process boundary:

- `selectPane <id>` / `paneLoaded <id>` — the app's `SettingsShell` enters and
  finishes the switch (QML work).
- `appframe` — each frame the app presents (`QQuickWindow::afterRendering`).
- `commit` — the compositor receives a window buffer commit.
- `present headless|nested` — the compositor presented a frame and sent client
  frame callbacks.

If `selectPane`→`paneLoaded` is fast but the next `present` is late, the delay
is in the compositor's frame path; if the app frames are late, it is in the
client. This is temporary diagnostic plumbing (compositor `src/trace.rs`,
`SettingsBridge::trace`), gated so an unset env var costs nothing.

## Real-hardware testing is a separate login session

For DRM/KMS, multi-monitor, suspend/resume, GPU-vendor behavior, VRR, and
real input, install the desktop alongside GNOME and register it with the
display manager:

```text
GDM
├── GNOME
├── GNOME Classic
└── Dragonfruit
```

If the compositor crashes, the session ends and GDM returns; the developer
picks GNOME or Dragonfruit. That is exactly the failure behavior you want
while developing a display server. **Do not uninstall GNOME during
development.**

The session package installs a Wayland-session descriptor plus user
services; `systemd`'s `graphical-session.target` gives us a clean model for
tying desktop services to session lifetime.

## Session composition and supervision

```text
graphical-session.target (systemd --user)
  └── dragonfruit-session.target
        ├── dragonfruit-compositor.service     # starts first, owns the seat
        ├── dragonfruit-shell.service          # After=compositor, Restart=always
        ├── dragonfruit-settingsd.service      # Restart=on-failure
        ├── dragonfruit-menu-broker.service   # Restart=on-failure
        ├── dragonfruit-app-index.service      # Restart=on-failure
        ├── dragonfruit-notifications.service  # Restart=on-failure
        └── dragonfruit-portal.service         # Restart=on-failure
```

- **Startup order:** compositor first; shell and services start in parallel
  once the compositor's private protocol socket exists; the portal backend
  registers with `xdg-desktop-portal` last.
- **Restart policy:** everything but the compositor restarts in place. A
  compositor crash ends the session and GDM returns — that is the display
  server reality, and the failure behavior we want.
- **Locking is fail-secure:** enforced in the compositor (`ext-session-lock-v1`
  semantics); no crash path unlocks a locked session.
- **Session environment** exported at startup: `XDG_CURRENT_DESKTOP=dragonfruit`,
  `XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY` pointing at the session
  socket, `DISPLAY` when Xwayland is up, and — for the compositor and shell
  only — `DRAGONFRUIT_LAUNCH_TOKEN`, the one-time private-protocol token. The
  desktop name is what toolkits, `portals.conf`, and
  `XDG_CURRENT_DESKTOP`-sensitive libraries use to find DE-specific behavior
  and our portal backend — it is a public contract, chosen once (see
  [12-packaging.md](12-packaging.md)). The variable names and the token
  ownership are frozen in
  [ADR 0065](adr/0065-session-environment-and-units.md).

The composition above is **data, not prose**: `services/session`'s
`SessionPlan`/`ServiceSpec` encode the stages, the per-service restart policy,
the compositor anchor, and the readiness gate, and `Supervisor` spawns, reaps,
and restarts the children. T-12.1b attaches the environment and the socket
readiness signal; T-12.2 uses the supervisor's shutdown path for logout. The
policy semantics are frozen in
[ADR 0064](adr/0064-session-manager-plan-and-restart-policy.md).

### The shipped systemd user units (T-12.1b)

The production supervisor is the systemd user units in
`services/session/units/`, one per `ServiceSpec` in the default plan, tied
together by `dragonfruit-session.target`:

```text
dragonfruit-session.target                 # WantedBy=graphical-session.target
  ├── dragonfruit-compositor.service       # --backend drm, Restart=no (anchor)
  ├── dragonfruit-shell.service            # Restart=always
  ├── dragonfruit-settingsd.service        # Restart=on-failure
  ├── dragonfruit-menu-broker.service
  ├── dragonfruit-app-index.service
  ├── dragonfruit-notifications.service
  └── dragonfruit-portal.service
```

Each service waits for the compositor's private socket with
`ExecStartPre=/usr/bin/dragonfruit-session --wait-socket dragonfruit-wayland`
— the unit-level form of the supervisor's `set_ready("compositor")` gate — and
passes the dynamic session variables
(`PassEnvironment=DISPLAY DRAGONFRUIT_LAUNCH_TOKEN`) to the child. The session
entry (T-12.2) imports the environment, including a fresh token, before
starting the target:

```bash
eval "$(dragonfruit-session --print-env --socket-name dragonfruit-wayland)"
systemctl --user import-environment \
    XDG_CURRENT_DESKTOP XDG_SESSION_TYPE WAYLAND_DISPLAY \
    DISPLAY DRAGONFRUIT_LAUNCH_TOKEN
systemctl --user start dragonfruit-session.target
```

The compositor pre-mints the `DRAGONFRUIT_LAUNCH_TOKEN` it finds in its own
environment (rather than a random one) and writes the shell's hand-off file,
so the token the shell presents is the one the session chose.

### The display-manager session entry (T-12.2)

The display manager lists the session from a Wayland descriptor,
`services/session/dragonfruit.desktop`
(`Name=Dragonfruit`, `DesktopNames=dragonfruit`), installed under
`share/wayland-sessions/` — so Dragonfruit appears next to the host DE in GDM,
SDDM, LightDM, or any spec-compliant greeter. Its `Exec` is the entry script
`dragonfruit-session-entry`:

```text
DM greeter → dragonfruit-session-entry
  1. eval "$(dragonfruit-session --print-env …)"   # base env + fresh token
  2. systemctl --user import-environment XDG_CURRENT_DESKTOP XDG_SESSION_TYPE \
         WAYLAND_DISPLAY DISPLAY DRAGONFRUIT_LAUNCH_TOKEN
  3. systemctl --user start dragonfruit-session.target
  4. wait for dragonfruit-compositor.service to leave the active state
  5. systemctl --user stop dragonfruit-session.target; reset-failed
  6. rm "$XDG_RUNTIME_DIR/<socket>"{,.lock,.x11-display,.launch-token}
```

The compositor is the anchor: when it exits (a clean logout or a crash), the
entry stops the target, clears its failed state, and removes the runtime
hand-off files, so the next login starts clean and the greeter returns. Every
step touches only our own user units and paths; the host desktop and its
display manager are never stopped, restarted, or reconfigured.

**Logout tears down process groups.** Each supervised service is spawned as
the leader of its own process group, and shutdown signals the whole group
(`SIGTERM`, then `SIGKILL` after a short grace), so a shell or service wrapper
cannot leak grandchildren past the session. The session binary installs the
descriptor, script, and the systemd user units under one prefix with
`dragonfruit-session --install-session DIR` (`entry::install_into`); packaging
(T-32) passes its build root. The contract is frozen in
[ADR 0066](adr/0066-display-manager-session-entry.md).

### The lock screen (T-12.3a)

Locking is fail-secure and enforced in the compositor over
`ext-session-lock-v1`: `SessionLockHandler::lock` enters the locked state and
clears client keyboard focus *before* it confirms, `process_input_event` keeps
every user input away from clients while locked (keyboard goes to the lock
surface, everything else is dropped; T-12.3c), and `xdg-activation`/window
activation refuse focus. A compositor crash ends the session; a shell crash
leaves the session locked because only `ext_session_lock_v1.unlock_and_destroy`
clears it.

The lock UI is the **shell**, not the compositor: Cmd+Ctrl+Q resolves to
`InputAction::LockScreen`, the compositor broadcasts it over the private
protocol, and the shell requests the lock and paints a `Dragonfruit.Lock` QML
scene into one lock surface per output. The upstream
`ext-session-lock-v1` XML is vendored (MIT) under `protocols/wayland-protocols/`
and compiled into the shell with the same `wayland-scanner` step as the private
protocols. The compositor composites the lock surfaces above the cursor,
windows, and chrome, and configures each to its output's exact size, so the UI
covers every output. Authentication (PAM) is T-12.3b and input capture /
kill-resistance is T-12.3c; the decision is frozen in
[ADR 0067](adr/0067-session-lock-protocol-and-ui.md).

`make e2e`'s `session_lock_conformance` locks a headless session, asserts the
`locked` event and a full-output configure on every output, and unlocks cleanly.

Authentication (T-12.3b) is a **small helper**, never a credential store. The
shell spawns `dragonfruit-pam-helper` (crate `services/lock-auth`) once per
attempt, writes the password to its standard input, and reads back only the
exit status (`0` authenticated, `1` rejected, `2` unavailable). The helper
loads the host's libpam at runtime (`dlopen`), runs only `pam_authenticate`
and `pam_acct_mgmt`, and keeps nothing. The shell calls
`ext_session_lock_v1.unlock_and_destroy` only on success; a rejected password
sets the lock card's message, and any unavailable/error state leaves the
session locked. The PAM service is `dragonfruit`, falling back to `login`; the
`--confdir` seam lets the headless suite run the real libpam against a
throwaway `permit`/`deny` service. Input capture (typing on the lock surface)
is T-12.3c; the decision is frozen in
[ADR 0068](adr/0068-lock-pam-helper.md).

Input capture (T-12.3c) focuses the seat keyboard on the lock surface, so the
lock UI — and nothing else — reads key events while locked; pointer, touch, and
gestures stay dropped. If the lock surface dies, `LockModel::input_surface`
returns nothing and input is dropped rather than falling back to a client:
losing surfaces never clears the `locked` flag, so killing the lock UI keeps the
session locked. The `ext_session_lock_manager_v1` global is also gated on the
T-07 launch-token handshake, so only the trusted shell can request a lock or
call `unlock_and_destroy`. The shell maps the captured keys through a small
US/ASCII evdev table (`shell/src/lockinput.cpp`) and exposes only the password
length to the lock scene, which draws a bullet mask; the password itself goes
straight to the PAM helper's stdin. The decision is frozen in
[ADR 0069](adr/0069-locked-input-capture-and-kill-resistance.md).

### Idle timers (T-12.4a)

The idle chain is **dim → blank → lock → suspend**, each stage a delay from
the last user activity. The chain is state in `dragonfruit-session`, not the
compositor: `services/session/src/idle.rs` holds `IdlePolicy` (one optional
delay per stage) and `IdleTimers`, a pure clock-injected state machine, so a
headless test drives every stage with a fake clock and no real time passes
(ADR [0070](adr/0070-idle-timer-engine-and-policy.md)). The compositor keeps
serving the `ext-idle-notify` and `idle-inhibit` protocols; an idle service
binds them, feeds activity into the engine, and asks the compositor to
dim/blank/lock over the private protocol. Policy keys are `idle.dim`,
`idle.blank`, `idle.lock`, `idle.suspend` in whole seconds (`0` disables a
stage); `IdlePolicy::from_keys` is the seam settingsd uses.

`IdleController` (T-12.4b) wraps that engine with inhibitors and wake restore
(ADR [0071](adr/0071-idle-inhibitors-and-wake-restore.md)). An inhibitor is an
opaque handle in an `IdleInhibitors` registry: while any handle is held the
controller's `poll` is a no-op and `next_deadline` is `None`, so the chain
cannot advance. A wake — a recorded `activity` or a fresh inhibitor, which
forces the chain back to the top — returns `IdleEvent::Restore(stage)` naming
the stage the caller must undo; waking from `Lock` reports `Restore(Lock)` but
never unlocks, because the lock UI owns unlocking. Releasing an inhibitor does
not move the chain or reset the inactivity clock, so a chain held past its
deadline catches up on the next poll. The suspend/resume cycle is T-12.5a; the
settingsd keys are T-12.5b.

### Suspend/resume (T-12.5a)

One suspend/resume round trip recovers outputs, input, and clients without a
restart. The request side is a pure `SuspendCycle` in
`services/session/src/suspend.rs` (`Awake` → `Requested` → `Asleep`), driven
from `IdleEvent::Enter(Suspend)` / `Restore(Suspend)` and logind's
`PrepareForSleep`; `SuspendController` binds it to a `SuspendBackend`, the one
seam that talks to logind (a recording mock in CI). The recovery side is a
`SuspendModel` in the compositor: `suspend_session` disarms animations and
drops pending frames, `resume_session` repaints every output and re-arms the
loop, and user input is dropped while suspended. Clients, outputs, and the
scene are never torn down, so the wake is a repaint. `query session` reports
`suspended` and the completed-cycle count; the headless synthetic harness
drives `suspend`/`resume`. The decision is frozen in
[ADR 0072](adr/0072-suspend-resume-cycle.md).

## logind integration

- `LockSession` / `UnlockSession` requests drive our lock screen; idle
  locking uses the `dragonfruit-session` idle chain
  ([above](#idle-timers-t-12-4a)) on top of the compositor's `ext-idle-notify`
  activity source.

### Session policy keys and the kill matrix (T-12.5b)

The four idle keys are registered in `dragonfruit-settingsd` (schema v5, the
new `session` group) as `x` whole seconds with `0` disabling a stage:
`idle.dim` 150, `idle.blank` 300, `idle.lock` 600, `idle.suspend` 0. They are
exactly the names and parser ADR [0070](adr/0070-idle-timer-engine-and-policy.md)
freezes; a consumer converts each `Value::Integer` to a string and calls
`IdlePolicy::from_keys`. The daemon is the one owner of the values; the
Settings Lock Screen pane (T-15.8b) will write them. There is no production
idle service yet, so the headless proof that the keys drive lock/idle/suspend
is `services/session/tests/session_policy.rs` (a fake clock walks the whole
chain from the settings snapshot, including a live key change and a held
inhibitor).

The **kill matrix** is deliberately split by owner. The compositor owns the
fail-secure lock invariant — no crash unlocks — and
`compositor/tests/session_lock_conformance.rs` closes the lock UI's socket and
asserts `locked=1`. The session manager owns supervision and
`services/session/tests/kill_matrix.rs` asserts the documented outcomes
(T-16.8a grew the T-12.5b subset to the whole shipped `SessionPlan::default_session`):

| Killed | Outcome |
|---|---|
| shell / lock UI | restarts (`always`); the session keeps running |
| settingsd, menu-broker, app-index, notifications, wallpaperd | restart (`on-failure`); the session keeps running |
| portal backend | restarts (`on-failure`) and fails soft; the session keeps running |
| app (a user-launched client) | a plain client exit: never restarted; the session keeps running |
| lock UI (a crash while locked) | fail-secure: the session stays locked, never unlocks |
| compositor | the session ends; every other service is stopped; never restarted |

Apps are not supervised session services: the shell/compositor launches them,
so an app crash is neither a session event nor a compositor crash. The
compositor half (`window_conformance::
a_crashed_app_leaves_the_compositor_and_the_other_app_running`) proves a
crashed app leaves the compositor and the surviving app running. The headless
reproduction for the whole matrix is `make t16-kill-matrix`
(`docs/captures/t16-kill-matrix.txt`); the live/VM real-binary half is recorded
by hand in `docs/captures/t16-kill-matrix.md`. The contract is
[ADR 0157](adr/0157-t16-crash-kill-matrix.md).

### Compositor death ends the session (T-16.8b)

The compositor is the session anchor: **its death — a crash, a `kill -9`, or a
clean quit — ends the session, and it is never restarted.** In
`SessionPlan::default_session()` it is the one `ends_session` service with
`RestartPolicy::Never`, the shipped `dragonfruit-compositor.service` is
`Restart=no`, and `SessionPlan::validate()` rejects a restartable anchor. The
reason is Wayland reality: clients are bound to one compositor and there is no
live handoff, so a compositor restart would strand every client. Whether the
death is a clean exit, a non-zero exit, or a fatal signal, the supervisor stops
every surviving service and moves to `SessionState::Ended`.

Recovery is a fresh login, not an in-place restart: the T-12.2 session entry
waits for `dragonfruit-compositor.service` to leave the active state, stops
`dragonfruit-session.target`, clears failed state, removes the runtime
hand-off files, and the display manager returns the greeter. Everything that is
*not* the compositor restarts in place ([the matrix
above](#session-policy-keys-and-the-kill-matrix-t-125b)); apps are plain
clients, and a crash while locked can never unlock because the lock is
fail-secure inside the compositor. The compositor may restart subprocesses it
owns (Xwayland), but that is not a session restart.

The restart policy is enforced headlessly by
`services/session/tests/restart_policy_matrix.rs`, which crosses all three
policies with every exit kind (`exit 0`, non-zero, signal) and crosses
compositor death with the same three exits. The contract, the policy table,
and the nested/dev outcome are frozen in
[ADR 0158](adr/0158-compositor-death-ends-the-session.md).

The T-12 track capture is `docs/captures/t12-session.*` (nested desktop, the
real lock UI after Cmd+Ctrl+Q, a short clip, the `query lock`/`query session`
transcript, and the kill-matrix output), produced by `make session-capture`.
The real greeter/login and DRM logout ends of the demo are T-12.6.
- `PrepareForSleep` / `PrepareForShutdown` hooks freeze animations, flush
  persisted state, and quiesce rendering before suspend; resume re-inits
  outputs and resumes render loops. T-12.5a implements the round trip as the
  compositor `SuspendModel` plus the session `SuspendController`
  ([above](#suspendresume-t-125a)); the concrete logind backend and the
  `PrepareForSleep` forwarding land with the real-session work.

## Absent services never block session start (T-17.5a)

The premium gate's robustness contract has two halves: an absent daemon degrades
its surface, and the session still starts. The pane/adapter half is the T-15.16
masking matrix ([08-settings.md](08-settings.md#the-absent-daemon-masking-matrix-t-1516));
the supervision half is this one.

A service that **cannot be spawned at all** — a missing binary, a daemon that
fails to `exec` — is not the same as one that dies later. The supervisor
([above](#session-composition-and-supervision)) records a spawn failure as
`ServiceState::Failed` **once** and never retries it, and `stage_ready`
withholds a stage only while a `gate` service is *running but not signalled*.
An absent gate is already non-pending, so the next stage starts; absence is
never a deadlock. The compositor is the only `gate` and the only `ends_session`
anchor in `SessionPlan::default_session`, so every optional absence is a
degraded-but-running desktop and only the compositor's absence ends the session.

`services/session/tests/absent_services.rs` derives its stand-in plan from
`SessionPlan::default_session`, replaces every non-anchor program with a
guaranteed-missing binary, and asserts the session reaches `Running` with each
absent service `Failed`, `restarts == 0`, and no pid; an absent gate never blocks
the next stage; only the anchor's absence ends the session; and the shipped plan
has exactly one gate (the compositor). The premium-gate reproduction is
`make t17-robustness-matrix` (`scripts/t17-robustness-matrix.sh`), which re-runs
the absence rows, the T-16.8a/T-16.8b kill and restart-policy matrices, and the
app-crash/lock kill-resistance rows into
`docs/captures/t17-robustness-matrix.txt`. The contract is
[ADR 0177](adr/0177-absent-services-never-block-session-start.md).

## The premium-gate sign-off (T-17.6)

The T-17 track is a checklist, not a feature: the loop, the visual floor, the
performance budget, robustness, and product judgment
([17-premium-gate.md](tracks/17-premium-gate.md)). T-17.6 closes it as a
**report over the per-unit evidence**, with two hardware rows left open:

```bash
make t17-premium-gate-capture   # assembled desktop still + transcript
make check                      # lint + tests + 100-cycle teardown soak
make e2e                        # foundation + conformance suites
```

`scripts/capture-t17-premium-gate.sh` launches the assembled desktop in a
nested session, captures it to `docs/captures/t17-premium-gate.png`, re-runs the
fast headless rows (gallery `--strict`, reduced-motion sweep, absent services)
into `docs/captures/t17-premium-gate.txt`, and is indexed by the reviewed
sign-off [docs/captures/t17-premium-gate.md](../captures/t17-premium-gate.md).
The report reproduces every checklist item with a verdict and evidence link,
records the post-gate backlog, and carries the unfamiliar-user protocol; the
human verdict is the batched track-boundary item. The gate is **incomplete, not
passed**, until the DRM loop (T-17.2) and the baseline Intel/AMD frame budget
(T-17.4) land on hardware. The evidence boundary is
[ADR 0179](adr/0179-t17-premium-gate-sign-off.md).

The report's acceptance is `make check` and `make e2e` green on the tree. The
long-red `check-desktop-names` gate was corrected so `make check` can pass: it
now ignores third-party `org.<name>.<member>` identifier namespaces
([ADR 0180](adr/0180-third-party-identifier-desktop-name-gate.md)).

## Second VT, with isolation

Another good workflow is leaving the host desktop on one virtual terminal and
entering our desktop through another. Two graphical sessions for the same Unix
user can collide through shared user-session services (portals, environment
variables), so we use a **dedicated development user** for this.

T-12.1b documents the manual, units-based workflow; T-12.6c automates it as
`dragonfruit dev --real` (see [Mode A](#mode-a--second-vt-no-logout-the-daily-mode)),
which fails the preflight if the user already holds a seat graphical session:

```bash
sudo useradd -m dfdev                       # once
sudo passwd dfdev                           # set a password
# From the host desktop, find a free VT (for example, VT1 is the host):
sudo chvt 3                                 # or Ctrl+Alt+F3
# Log in as dfdev on that VT (a text login is fine), then from its shell:
export XDG_RUNTIME_DIR=/run/user/$(id -u)
eval "$(dragonfruit-session --print-env --socket-name dragonfruit-wayland)"
systemctl --user import-environment \
    XDG_CURRENT_DESKTOP XDG_SESSION_TYPE WAYLAND_DISPLAY \
    DRAGONFRUIT_LAUNCH_TOKEN
systemctl --user start dragonfruit-session.target
```

`Ctrl+Alt+F1` returns to the host desktop exactly as it was; the host session
is never logged out and the two sessions never share `XDG_RUNTIME_DIR`.
`dragonfruit-session --wait-socket dragonfruit-wayland` is the readiness probe
when a script needs to know the session is up. A dedicated user is required,
not optional: `dragonfruit dev --real` refuses to run for a user that already
holds a seat session (T-12.6c).

## The real-session dev harness

`make dev`/`make demo` are nested and never touch the seat. To test the desktop
as a **primary session** on a dev workstation — real login, DRM/KMS, udev
input, logind, session services — T-12.6 adds a harness with two modes. Both
are display-manager-agnostic: they **never stop, restart, or introspect the
host compositor**, so they work the same on GNOME, KDE, Sway, or anything else.

There is no live compositor handoff (see [above](#summary)): every mode crosses
a fixed logout/login or VT boundary, and no client survives it. The rationale
and rejected alternatives are recorded in
[ADR 0053](adr/0053-real-session-dev-harness.md).

### Mode A — second VT (no logout; the daily mode)

The host desktop stays on VT1; the harness starts a real DRM session on a free
VT for a **dedicated user**, so the two graphical sessions do not collide over
`XDG_RUNTIME_DIR`/portals:

```bash
dragonfruit dev --real --user dfdev      # start on a second VT
dragonfruit dev --real --plan --user dfdev   # preflight + plan, start nothing
dragonfruit dev --real --teardown --user dfdev
```

The harness runs the dedicated-user preflight against `loginctl` and **refuses**
the command when `dfdev` already holds an active seat graphical session,
explaining that a second graphical session for the same user collides over
shared user-session state (portals, `XDG_RUNTIME_DIR`) and printing the
`sudo useradd -m dfdev` bootstrap. Otherwise it picks the **smallest free VT**
from the `loginctl` snapshot (never assuming VT1) and starts the shipped
session entry as a `PAMName=login` session on that VT through `systemd-run`, so
the dedicated user gets its own user manager, `XDG_RUNTIME_DIR`, and units.
`Ctrl+Alt+F<host-vt>` (or the printed `chvt`) returns to the host desktop
exactly as it was; the host session is never logged out. `--teardown`
terminates the dedicated user's session with `loginctl terminate-session` and
verifies no orphaned process survives.

`--plan` performs the whole preflight and VT selection without starting
anything, so it is safe to run next to the host desktop; the logic is pinned by
`second_vt::tests` against a mocked `loginctl` and `make second-vt-validation`
records the refusal and the selection. The start → switch → return → teardown
cycle on real hardware is part of the T-159…T-161 rail. This is the
low-friction "fully test on the workstation" command; the decision is frozen in
[ADR 0175](adr/0175-second-vt-dev-harness.md).

### Mode B — display-manager round-trip (same user; the realism mode)

Close the host session, log into Dragonfruit at the display manager, test, log
out, and return. The greeter is the restore point:

```bash
dragonfruit dev --real --round-trip      # arm the next login, then log out
```

1. **Arm.** Write `$XDG_STATE_HOME/dragonfruit/dev-session.json` (previous
   default session, whether autologin was armed, timestamp); select the
   Dragonfruit session for the next login; optionally arm DM autologin; then
   ask the host session manager to log out — apps get their normal
   unsaved-work prompts, nothing is `SIGKILL`ed.
2. **In session.** The session exports `DRAGONFRUIT_DEV_RETURN=<previous
   session>`. The system/Control Center menu shows **"Quit to \<previous
   desktop\>"** only when that variable is set.
3. **Return.** The item disarms autologin, restores the previous default
   session, and logs out.
4. **Crash recovery.** Compositor death ends the session by design. The state
   file is the single source of truth, so a one-shot check on the next login —
   clean or after a crash — restores the previous default and clears the armed
   state. A "session-ready" beat before committing autologin prevents a crash
   loop back into Dragonfruit.

### Display-manager adapters

Session preselection is per-DM and mostly user-writable. An unrecognized DM
degrades to "the human picks it in the greeter":

| DM | Preselect session | Autologin (needs root) |
|---|---|---|
| GDM | AccountsService `XSession` (`SetXSession`) | `/etc/gdm/custom.conf` |
| SDDM | SDDM state file | `/etc/sddm.conf.d/` |
| LightDM | `~/.dmrc` | `/etc/lightdm/lightdm.conf` |

Autologin is optional and off by default. It is armed only for a round trip and
force-disabled on return or recovery. Session files and units come from
T-12.1b/T-12.2.

The table is data, not prose: T-12.6a implements it in
`tools/dragonfruit-dev/src/session_selector.rs`. A `DmSession` adapter per DM
reads, selects, and restores the session id (a minimal keyfile edit that keeps
every unrelated line), and a separate `DmAutologin` adapter snapshots, arms,
disarms, and restores autologin. Detection is by a running DM process
(`/proc/*/comm`) or, when none is running, exactly one installed DM; a host
that is unknown *or has several DMs installed and none running* is
`Selected::Unsupported` and **no write is attempted**, so the caller falls back
to "pick it in the greeter" rather than guessing. Every path derives from an
explicit root and home, so the acceptance suite drives real reads and
exact-byte writes against fixture directories with no DM installed. The
decision is frozen in [ADR 0173](adr/0173-display-manager-session-selection-seam.md).

### The round-trip state file (T-12.6b)

The round trip is one state file, not a set of ad-hoc writes.
`$XDG_STATE_HOME/dragonfruit/dev-session.json`
(`dragonfruit_session::dev_session`) is written once by
`dragonfruit dev --real --round-trip` and cleared by the return; it holds the
previous default session, the DM, the autologin request and its prior
snapshot, the dev-tool path, and **exact byte snapshots** of the DM files the
arm may edit. The snapshots are what make a clean round trip byte-identical on
GDM, SDDM, and LightDM, even for a file that did not exist before.

Autologin is not armed at arm time. `dragonfruit dev --real --ready` is the
**session-ready beat**: once the session is up it commits the requested
autologin and marks the state ready, so a crash during startup can never loop
the display manager back into Dragonfruit. Recovery is the same restore path
as a clean return — idempotent, and a no-op when nothing is armed.

The session entry (`dragonfruit-session --print-env`) reads the state file and
exports `DRAGONFRUIT_DEV_RETURN` (the previous session) and
`DRAGONFRUIT_DEV_BIN` (the dev tool) into the login environment. The shell's
system menu shows **"Quit to \<previous desktop\>"** only when
`DRAGONFRUIT_DEV_RETURN` is set; activating it runs
`dragonfruit dev --real --return`, which restores the snapshot, clears the
state, and asks the session manager to log out. Keeping the restore in the
harness is what lets the shell stay a normal user process that never edits
root-owned DM files. The decision is frozen in
[ADR 0174](adr/0174-real-session-round-trip-state.md).

### What "seamless" cannot mean

- **No app continuity across the swap.** Wayland has no live handoff, and the
  host DE's own session-restore is per-DE and not authoritative. The harness may
  relaunch an explicit `.desktop` allowlist captured before logout, but never
  resurrects unsaved document state.
- **Never script the close.** The harness triggers the host session manager's
  logout; it does not kill the host compositor or its clients.
- **No per-DE stop/start.** We never `systemctl stop gdm` or kill the host DE;
  VT switching and DM session selection already handle restoration for every DE.

## The testing ladder

```text
fast iteration
    ↓
nested compositor in GNOME
    ↓
dedicated-user real Wayland session
    ↓
VM
    ↓
real primary machine
    ↓
multi-GPU / laptop / dock / NVIDIA test machines
```

The VM stage is particularly valuable once we start intentionally crashing
the compositor, lock screen, or portal service.

## Portal timing

Portal support arrives surprisingly early: once the desktop is a real
Wayland session, browsers, Electron programs, Flatpak applications,
conferencing programs, and screen capture all need
`xdg-desktop-portal-dragonfruit` (see
[07-system-integration.md](07-system-integration.md)).
