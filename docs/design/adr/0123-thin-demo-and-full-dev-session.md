# 0123 — A thin loop demo and a separate full nested dev session

## Status

accepted

## Context

`make demo` (T-01.6a) and `dragonfruit dev --nested` are the daily nested
session. They launch the compositor, the shell, and a small set of
best-effort session services, and they must stay usable on a workstation that
is already running another desktop (GNOME/KDE/Sway) without disturbing it.

Two pressures conflict:

- The **loop demo and CI** want to be thin, deterministic, and free of a
  session-bus dependency. `make e2e` runs `make demo --headless`; it must not
  need a bus, host daemons, or the network.
- **Driving the real UI** (Settings panes, Control Center, notifications, the
  wallpaper provider) wants those services present. Today the harness starts
  `dragonfruit-app-index`, `dragonfruit-menu-broker`, and (nested only)
  `dragonfruit-wallpaperd`, but not `dragonfruit-settingsd` or
  `dragonfruit-system-status`, so most Settings panes report "the settings
  daemon is not running" / "the system status service is not running".

The per-pane capture scripts already solve the second case by hand: each
starts a scratch `settingsd` (and often `wallpaperd`) with a scratch
`XDG_CONFIG_HOME`, and guards against the D-Bus name already being owned.
That logic is duplicated across roughly thirty scripts.

## Decision

- **Two commands, one split.** `make demo` and `make dev` stay thin (core
  services: `app-index` + `menu-broker`). A new `make dev-full` brings up the
  full nested session. The dev tool selects the set with `--services
  none|core|full`; the headless scripted path always uses `none`.
- **`wallpaperd` moves off the thin path.** It is a network-capable content
  provider, not shell chrome; the thin loop and CI never start it. (The
  wallpaper capture and absence-matrix scripts provision their own scratch
  provider and are unaffected.)
- **Isolation is not opt-out on the full path.** `make dev-full` runs on a
  private `dbus-daemon --session` and points `XDG_CONFIG_HOME`,
  `XDG_CACHE_HOME`, and `XDG_STATE_HOME` at a scratch directory under the
  session runtime dir. This is what makes it safe beside another desktop: it
  cannot own the host's `org.dragonfruit.Settings1`/`SystemStatus1`, and it
  cannot read or overwrite the developer's real settings or wallpaper cache.
  `--private-bus` is the dev-tool flag; it degrades to the inherited bus with
  a warning when `dbus-daemon` is absent (still using the scratch XDG dirs).
- **Readiness replaces the blind settle.** After starting services, the tool
  polls the shell-critical bus names (`Settings1`, `AppIndex1`,
  `MenuBroker1`, `SystemStatus1`) via `gdbus`, falling back to a short settle
  when `gdbus` is unavailable.
- **Fixtures are a first-class opt-in.** `--fixtures` (or
  `make dev-full FIXTURES=1`) exports the deterministic `DF_*_FIXTURE` pane
  data to the session, so Settings and Control Center render populated on a
  host without BlueZ/UDisks2/WirePlumber/UPower. Host daemon absence stays a
  normal state.
- **`make demo` is unchanged in contract.** Its nested session is now core
  services only and its scripted half is service-free; the checklist, the
  capture scripts, and `make e2e` are unaffected.

## Consequences

- `services/` and the real session supervisor remain the single owner of the
  production service set ([0064](0064-session-manager-plan-and-restart-policy.md));
  `make dev-full` is a dev-only approximation, not a second session manager.
- The capture scripts can migrate to `make dev-full` incrementally, and the
  duplicated scratch-bus/scratch-home setup can be retired once they do.
- The full session depends on `dbus-daemon` and (for populated panes) either
  host daemons or fixtures; both are best-effort, so a missing one degrades
  rather than fails.
- `make dev-full` is for humans on a workstation; it is not part of `make
  e2e` and never runs in CI.