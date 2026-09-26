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
clears client keyboard focus *before* it confirms, `process_input_event` drops
every user input while locked, and `xdg-activation`/window activation refuse
focus. A compositor crash ends the session; a shell crash leaves the session
locked because only `ext_session_lock_v1.unlock_and_destroy` clears it.

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

## logind integration

- `LockSession` / `UnlockSession` requests drive our lock screen; idle
  locking uses `ext-idle-notify` thresholds owned by the compositor.
- `PrepareForSleep` / `PrepareForShutdown` hooks freeze animations, flush
  persisted state, and quiesce rendering before suspend; resume re-inits
  outputs and resumes render loops.

## Second VT, with isolation

Another good workflow is leaving the host desktop on one virtual terminal and
entering our desktop through another. Two graphical sessions for the same Unix
user can collide through shared user-session services (portals, environment
variables), so we use a **dedicated development user** for this.

T-12.1b documents the manual, units-based workflow; T-12.6c automates it as
`dragonfruit dev --real`:

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
```

`Ctrl+Alt+F1` returns to the host desktop exactly as it was; the host session
is never logged out. This is the low-friction "fully test on the workstation"
command.

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
