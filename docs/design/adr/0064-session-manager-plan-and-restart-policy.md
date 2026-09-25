# 0064 — Session manager plan and restart policy

## Status

accepted

## Context

T-12.1a is the first slice that makes the desktop a *session* rather than a
nested window. The composition and restart rules already exist as prose in
[11-session-and-dev-workflow.md](../11-session-and-dev-workflow.md#session-composition-and-supervision):
the compositor starts first and owns the seat; the shell and user services
start in parallel once the compositor's private socket exists; the portal
registers last; everything but the compositor restarts in place, and a
compositor exit ends the session. T-12.1b (environment, systemd units,
second-VT), T-12.2 (display-manager entry, logout teardown), and T-12.5b (kill
matrix) all depend on one machine-readable form of that prose, so the ordering
and policy are data, not repeated shell.

## Decision

- **`services/session` owns the session composition as data.** A
  `SessionPlan` is an ordered list of `ServiceSpec` (`name`, `program`, `args`,
  `env`, `policy`, `stage`, `ends_session`, `gate`). `SessionPlan::default_session()`
  is the design-doc composition: stage 0 `compositor` (Never, anchor, gate),
  stage 1 `shell` (Always) + `settingsd`/`menu-broker`/`app-index`/`notifications`
  (OnFailure), stage 2 `portal` (OnFailure).
- **`RestartPolicy` has exactly three values:** `Always`, `OnFailure` (restart
  on a non-zero status or a signal), `Never`. The session anchor must be
  `Never`; a restarted anchor could not end the session. A plan with a
  restartable anchor, a duplicate name, or more than one anchor is rejected by
  `SessionPlan::validate`.
- **A stage starts when the previous stage is up.** A `gate` service holds
  the next stage until `Supervisor::set_ready(name)` is called; T-12.1b drives
  that from the compositor's private socket. A gate service that has already
  exited does not deadlock the stage.
- **The anchor is authoritative.** When an `ends_session` service exits (or
  fails to spawn), the supervisor stops every surviving child and moves to
  `SessionState::Ended`; the anchor is never restarted regardless of policy.
- **The supervisor is synchronous and runtime-free.** `start()` launches stage
  0; `tick()` reaps exited children, applies policy, and advances stages. The
  binary's `--exec` loop ticks every 50 ms and tears down on SIGINT/SIGTERM.
  This is what makes the headless kill test a one-tick assertion.

## Consequences

- T-12.1b attaches environment variables to `ServiceSpec::env` and calls
  `set_ready("compositor")` when the socket exists; it does not re-encode the
  order or policies.
- T-12.2 uses `Supervisor::shutdown` for logout teardown and extends it to
  process groups; today teardown `SIGKILL`s direct children.
- T-12.5b's kill matrix and the settingsd/notification restart tests assert
  against the same `ServiceState`/`SupervisorEvent` vocabulary.
- The systemd units T-12.1b ships remain the production supervisor; this
  in-process manager is the headless/testable contract and the seam the dev
  harness and the units must agree with.