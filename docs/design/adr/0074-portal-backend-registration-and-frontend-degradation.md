# 0074 — Portal backend registration and frontend degradation

## Status

accepted

## Context

T-13.1a starts the portal track: a real session-bus backend and the
registration that makes `xdg-desktop-portal` route to it. The track's later
tasks (T-13.1b Settings/GlobalShortcuts, T-13.2a FileChooser, T-13.3a
Screenshot, T-13.4a ScreenCast, T-13.5 clipboard, T-13.6 auth) all add
interfaces to the same process and the same registration, so the name, the
object path, the discoverability files, and the behavior when the
`xdg-desktop-portal` frontend is missing must be chosen once now.

`xdg-desktop-portal` finds a backend through installed files, not a runtime
handshake (`writing-a-new-backend`): a `.portal` descriptor naming the D-Bus
service, a `portals.conf` `[preferred]` selection keyed off
`XDG_CURRENT_DESKTOP`, and a D-Bus activation file. The frontend may not be
running at all (a bare nested session, CI), and that must never block the
backend or the session.

## Decision

- **One process, the standard identity.** The backend is
  `xdg-desktop-portal-dragonfruit`, owns the well-known name
  `org.freedesktop.impl.portal.desktop.dragonfruit`, and serves
  `/org/freedesktop/portal/desktop` — the names the spec's `.portal`
  descriptor uses. The backend name in `portals.conf` is `dragonfruit`.
- **Registration is three installed files, generated from embedded sources.**
  `portal/data/dragonfruit.portal`,
  `portal/data/dragonfruit-portals.conf` (installed as
  `{DATADIR}/xdg-desktop-portal/dragonfruit-portals.conf`,
  `default=dragonfruit;gtk` so the generic backend covers what we have not
  implemented yet), and
  `portal/data/org.freedesktop.impl.portal.desktop.dragonfruit.service`.
  `portal::data::install_into(prefix)` writes them for packaging (T-16); the
  `dragonfruit.portal` `Interfaces=` list must equal
  `model::BACKEND_INTERFACES`, kept in lockstep by a test.
- **The frontend's absence is a normal state, not an error.** The backend
  registers and serves regardless. It only observes the frontend: one
  `NameHasOwner` probe at startup, then a live watch of
  `org.freedesktop.DBus.NameOwnerChanged`. A bus that cannot be asked is
  `unknown`, an unowned name is `absent`; neither aborts startup.
- **The observation is exposed on a Dragonfruit diagnostic interface, not the
  portal contract.** `org.dragonfruit.Portal1` is served at the standard
  object path alongside the future `org.freedesktop.impl.portal.*`
  interfaces and reports the identity, the advertised interfaces, and the
  frontend presence (`Frontend()`/`FrontendPresent()`/`FrontendOwner()`, the
  `FrontendChanged` signal). The portal frontend never calls it; tests and
  `--check-frontend` do.

## Consequences

- Every later T-13 task adds its interface to `BACKEND_INTERFACES` and to the
  set served at `/org/freedesktop/portal/desktop`; it must not change the
  backend name, path, or the three data files' identity.
- A session with no `xdg-desktop-portal` is supported and observable; a
  frontend that starts later is detected live and the state flips without a
  restart.
- The interface names are standard `org.freedesktop.impl.portal.*`, not
  `org.dragonfruit.*`, so `df_ipc::is_valid_dbus_name` does **not** apply to
  them (only the diagnostic `org.dragonfruit.Portal1` follows that policy).
- T-13.7's Flatpak validation relies on this registration; real
  `xdg-desktop-portal` routing is a hardware/session check, while the
  in-repo headless tests own the private-bus registration and the
  absent/present-frontend paths.