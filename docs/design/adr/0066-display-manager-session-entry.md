# 0066 — Display-manager session entry and logout teardown

## Status

accepted

## Context

T-12.1b shipped the session environment and the systemd user units that are the
production supervisor, but nothing yet put a *session* in front of the display
manager: no `wayland-sessions` descriptor, no startup/logout owner, and
`Supervisor::shutdown` still `SIGKILL`ed only direct children
([ADR 0064](0064-session-manager-plan-and-restart-policy.md)). A session a DM
can start must also be able to *end* without leaking user units, failed state,
or the `$XDG_RUNTIME_DIR` hand-off files, and it must never disturb the host
desktop ([ADR 0053](0053-real-session-dev-harness.md)).

## Decision

- **The session entry is a Wayland `.desktop` descriptor.**
  `services/session/dragonfruit.desktop` (`Name=Dragonfruit`,
  `DesktopNames=dragonfruit`, `Type=Application`) is installed under
  `share/wayland-sessions/`, next to the host desktop. Its `Exec` is the
  absolute `ENTRY_COMMAND` `/usr/bin/dragonfruit-session-entry`.
- **One entry script owns startup and logout.**
  `services/session/dragonfruit-session-entry` imports the T-12.1b session
  environment (`--print-env` + `systemctl --user import-environment` over
  `XDG_CURRENT_DESKTOP XDG_SESSION_TYPE WAYLAND_DISPLAY DISPLAY
  DRAGONFRUIT_LAUNCH_TOKEN`), starts `dragonfruit-session.target`, blocks
  until the compositor anchor unit leaves the active state, then stops the
  target, `reset-failed`s it, and removes
  `$XDG_RUNTIME_DIR/<socket>{,.lock,.x11-display,.launch-token}`. It only
  speaks to our own user units; the host DE and its display manager are never
  touched.
- **Logout tears down process groups.** Every supervised service is spawned as
  a process-group leader (`Command::process_group(0)`);
  `Supervisor::shutdown`, the anchor-exit path, and `kill` signal the whole
  group (`SIGTERM`, then `SIGKILL` after a 500 ms grace), so a wrapper's
  grandchildren cannot outlive the session.
- **The install layout is data.** `services/session/src/entry.rs` embeds the
  entry file, the script, and the units, and `entry::install_into(prefix)`
  writes them under `bin/`, `share/wayland-sessions/`, and
  `lib/systemd/user/`; `dragonfruit-session --install-session DIR` exposes the
  same seam. T-32 packaging calls it with its build root.

## Consequences

- T-12.6a selects `dragonfruit.desktop` by name; T-12.6b/c build on the entry
  and the same runtime-hand-off cleanup.
- The host's display-manager configuration is untouched by construction:
  installation is additive under the `dragonfruit` namespace
  ([12-packaging.md](../12-packaging.md)).
- The units test and `entry.rs` embed the same source files, so a unit edit
  that skips the installer fails the acceptance tests.
- The `--wait-socket` stale-path caveat remains; the entry's logout cleanup is
  the safety net, and a native `sd_notify` gate is still a later refinement.