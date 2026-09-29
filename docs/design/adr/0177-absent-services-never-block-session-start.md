# 0177 — An absent service degrades the session; it never blocks its start

## Status

accepted

## Context

The premium gate's robustness checklist has two clauses: *every absent-daemon
case degrades, and nothing blocks session start*, and the crash/kill matrix
where compositor death ends the session by design. The second clause and the
pane/adapter half of the first were already frozen — the T-15.16 absent-daemon
masking matrix ([0148](0148-t15-absent-daemon-masking-matrix.md)), the T-16.8a
kill matrix ([0157](0157-t16-crash-kill-matrix.md)), and compositor death
([0158](0158-compositor-death-ends-the-session.md)). What no test asserted was
the session-manager half of "nothing blocks start": a shipped service whose
program **cannot be spawned at all** (a missing binary, a daemon that fails to
exec) is not the same as one that dies later, and only the latter was covered.

The supervisor already has the right shape: a spawn failure records
`ServiceState::Failed` once and is never retried; `stage_ready` treats anything
that is not `Pending` as ready, and only withholds readiness for a `gate`
service that is *running but not signalled*. The compositor is the only gate
and the only anchor, so no optional absence can hold the launch order open.

## Decision

- Session start is verified against absence by
  `services/session/tests/absent_services.rs` (T-17.5a). It derives the
  stand-in plan from `SessionPlan::default_session`, replaces every non-anchor
  program with a guaranteed-missing binary, and asserts the session reaches
  `Running`; every absent service is `Failed` with `restarts == 0` and no pid;
  an absent gate never blocks the next stage; and only the anchor's absence
  ends the session.
- The test also pins the structural invariant: `the_only_gate_in_the_shipped_plan_is_the_compositor`.
  A new `gate` on a non-anchor service would fail this test before it could
  reintroduce a start deadlock.
- The premium-gate reproduction is
  `scripts/t17-robustness-matrix.sh` (`make t17-robustness-matrix`), which
  re-runs the session absence test, the T-15.16 absence rows, and the
  T-16.8a/T-16.8b kill and restart-policy matrices on the release tree into
  `docs/captures/t17-robustness-matrix.txt` and fails if any row fails.
- The real-binary drill (killing shipped binaries under a live session, stopping
  real daemons in a VM) is the same hardware/VM rail as T-15.16/T-16.8a and is a
  manual step; the live nested check is recorded in
  `docs/captures/t17-robustness-matrix.md`.

## Consequences

- `make e2e` already runs `cargo test -p dragonfruit-session`, so the new
  absence suite is part of the gate, not an extra command.
- The contract is now explicit: absence of any non-anchor service is a degraded
  desktop, and only the Wayland anchor's absence ends the session.
- T-17.5b (leaks and lock enforcement) is unaffected; the lock kill-resistance
  row is only re-run here, not re-implemented.
- No user-visible surface changed; the live check confirms the desktop still
  renders.