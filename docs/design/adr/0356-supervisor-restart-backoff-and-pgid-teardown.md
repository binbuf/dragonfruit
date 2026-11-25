# 0356 — Supervisor restart backoff and PGID-safe teardown

## Status

accepted

## Context

The in-process session supervisor (`services/session/src/supervisor.rs`) is the
`--exec`/dev path and the headless contract the kill and restart matrices drive
([ADR 0064](0064-session-manager-plan-and-restart-policy.md)). Its `--exec`
loop ticks every 50 ms and respawned a crashed service on the next tick, so a
service that failed instantly was respawned twenty times a second — a fork bomb
with a friendly name. It also reaped the leader first and signalled the process
group afterwards. A process-group id is the leader's pid; once the leader is
reaped the kernel may hand that id to an unrelated process group, so the final
`kill(-pgid, SIGKILL)` could reach a stranger.

The shipped units are supervised by systemd, but the same supervisor runs the
dev session and is the behavior every headless matrix asserts. Both defects are
correctness bugs in that path.

## Decision

- **Bounded restart backoff.** `RestartBackoff` defaults to a 200 ms base,
  doubling per consecutive attempt and capped at 30 s, at most five consecutive
  attempts, and a 10 s stable-run window. The delay before attempt *n* is
  `min(base * 2^(n-1), max_delay)`.
- **A waiting restart stays `Exited`.** On a restartable exit the slot records
  `next_restart_at` and `tick()` spawns it when the deadline passes; there is no
  new `ServiceState` for "waiting". `Supervisor::next_restart_at` and
  `restart_attempts` expose the schedule for tests and logs.
- **Giving up is explicit.** After `max_attempts` consecutive restarts the
  supervisor emits `RestartAbandoned` and leaves the service `Exited`; it does
  not become `Failed` (the spawn itself worked) and it is not respawned again.
- **A stable run resets the counter.** A run at least `stable_run` long is a
  success, not a crash: the consecutive-attempt counter returns to zero. The
  lifetime `restarts` count is never reset.
- **Spawn failures are still terminal.** A program that cannot be spawned is
  `Failed`, as before; the backoff only covers processes that started and
  exited.
- **Kill the group before reaping the leader.** Teardown sends `SIGTERM` to the
  group, waits for the leader to exit *without reaping it*, sends `SIGKILL` to
  the group, and only then calls `wait()`. Linux uses a pidfd (`pidfd_open` +
  `poll`) to observe the exit; a kernel without pidfd support waits out the
  full grace. While the leader is a zombie or still running, its pid — and so
  the pgid — cannot be recycled. If the leader is already gone (`pidfd_open`
  returns `ESRCH`), the pgid is never signalled again.
- **The driver loop waits on scheduled restarts.** `Supervisor::any_active()`
  is running *or* restart-scheduled; the `--exec` loop uses it so the gap
  between a crash and its backoff is not mistaken for a finished session.
  `end_session` cancels scheduled restarts along with the children.

## Consequences

- A crash loop can start at most `max_attempts` processes over its backoff
  window, then stays down with a `RestartAbandoned` event. A service that
  crashes once after a long run always restarts.
- Teardown of a well-behaved service is as fast as before: the leader exits on
  `SIGTERM`, the group `SIGKILL` follows immediately, and only an
  unresponsive service pays the 500 ms grace.
- The anchor keeps its authority: its exit still ends the session and cancels
  any service's pending restart.
- systemd remains the production supervisor; its `StartLimit*`/`RestartSec`
  settings are independent of these in-process limits and neither replaces the
  other.
- [ADR 0064](0064-session-manager-plan-and-restart-policy.md) remains the plan
  and policy contract; this ADR refines restart timing and teardown ordering.
- Tests: `restart_policy_matrix` covers backoff growth, the attempt limit, and
  the stable-run reset; a `supervisor` unit test models pgid reuse and proves
  the final group `SIGKILL` precedes the reap; `kill_matrix` and `logout` keep
  the policy and group-teardown coverage.
