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
