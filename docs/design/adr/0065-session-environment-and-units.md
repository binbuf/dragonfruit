# 0065 — Session environment and systemd user units

## Status

accepted

## Context

T-12.1a made the session composition data (`SessionPlan`/`ServiceSpec`) and
froze the launch order and restart policy ([ADR 0064](0064-session-manager-plan-and-restart-policy.md)),
but left `ServiceSpec::env` empty and shipped no production supervisor.
T-12.1b attaches the session environment, ships the systemd user units that
are the production supervisor, and documents the dedicated-user second-VT
workflow. The environment variable names and the token ownership are a
contract the shell, the services, the session entry (T-12.2), and the dev
harness (T-12.6) all consume, so they are fixed here.

## Decision

- **The base environment every session child gets** is exactly
  `XDG_CURRENT_DESKTOP=dragonfruit` (the public desktop-name contract),
  `XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY=<socket>`, and — only when
  Xwayland is up — `DISPLAY=<:N>`. It lives in
  `services/session/src/env.rs` (`SessionEnvironment::base`) and is attached to
  a plan with `SessionPlan::with_environment` / `default_session_for`.
- **`DRAGONFRUIT_LAUNCH_TOKEN` is session-scoped and shared by the compositor
  and the shell.** The session mints one 32-byte token
  (`generate_launch_token`, lowercase hex) and gives it to the compositor (the
  issuer, which pre-mints it instead of a random one and writes the existing
  `<socket>.launch-token` hand-off file) and to the trusted client
  (`ServiceSpec::trusted`). Non-trusted services never see it. The token value
  is never logged.
- **The shipped production supervisor is the systemd user units** in
  `services/session/units/`: `dragonfruit-session.target` plus one unit per
  `ServiceSpec` in `default_session()`. The compositor unit is the anchor
  (`Restart=no`), the shell unit is `Restart=always`, every other service is
  `Restart=on-failure`, mirroring `RestartPolicy`.
- **The readiness gate is a unit `ExecStartPre`.** Every non-compositor unit
  runs `dragonfruit-session --wait-socket dragonfruit-wayland`, the unit-level
  form of `Supervisor::set_ready("compositor")`. The fixed socket name
  `dragonfruit-wayland` keeps `WAYLAND_DISPLAY` and the units in lockstep.
- **The session entry imports the environment.** `dragonfruit-session
  --print-env` prints the base variables plus a fresh token, and the entry
  imports them into the systemd user manager (or its own environment) before
  starting the target; units `PassEnvironment=DISPLAY DRAGONFRUIT_LAUNCH_TOKEN`
  and set the static variables with `Environment=`.

## Consequences

- T-12.2's session entry uses `--print-env` + `import-environment` +
  `systemctl --user start dragonfruit-session.target`, and adds the
  `.desktop` session file and logout teardown. It does not re-encode the
  environment or the units.
- T-12.6c's second-VT harness reuses `--wait-socket`/`--print-env` and the
  dedicated-user rule documented in `docs/testing-ladder.md` rung 2.
- The compositor stays unchanged: it already pre-mints
  `DRAGONFRUIT_LAUNCH_TOKEN` and writes the hand-off file (`shell::provision`).
- The `--wait-socket` helper uses file existence, not a connect test; a stale
  socket path is the compositor's teardown bug, not the gate's.