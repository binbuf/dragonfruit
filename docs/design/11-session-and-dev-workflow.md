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
  socket. The desktop name is what toolkits, `portals.conf`, and
  `XDG_CURRENT_DESKTOP`-sensitive libraries use to find DE-specific behavior
  and our portal backend — it is a public contract, chosen once (see
  [12-packaging.md](12-packaging.md)).

## logind integration

- `LockSession` / `UnlockSession` requests drive our lock screen; idle
  locking uses `ext-idle-notify` thresholds owned by the compositor.
- `PrepareForSleep` / `PrepareForShutdown` hooks freeze animations, flush
  persisted state, and quiesce rendering before suspend; resume re-inits
  outputs and resumes render loops.

## Second VT, with isolation

Another good workflow is leaving GNOME on one virtual terminal and entering
our desktop through another. Two graphical sessions for the same Unix user
can collide through shared user-session services (portals, environment
variables), so we use a **dedicated development user** for this.

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
