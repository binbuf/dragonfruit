# 0313 — Idle timer and DPMS ownership for the daily driver

## Status

Accepted (2026-11-22). Narrows the production placement in
[ADR 0070](0070-idle-timer-engine-and-policy.md); the stage rules and policy
keys are unchanged.

## Context

ADR 0070 put the idle chain in `dragonfruit-session` as a pure, clock-injected
engine and left the production trigger to a future service that would bind
`ext-idle-notify` and tell the compositor/shell over the private protocol. That
engine is complete and unit-tested (`services/session/src/idle.rs`), including
inhibitors, normalization, and wake restore. No shipped unit starts it. The
compositor advertises `ext-idle-notify` and `idle-inhibit`, but nothing consumes
the idle state; there is no DPMS path. The result:

- `idle.dim`, `idle.blank`, `idle.lock`, and `idle.suspend` are marked live but
  never fire;
- the display never blanks, so a laptop never sleeps its panel;
- the session never auto-locks, and a lid-close suspend wakes unlocked;
- `power.preventSleep` has no consumer.

The contract needs the behavior, not a particular process layout. A new
`dragonfruit-idle` daemon would add a Wayland client, a D-Bus client, a policy
cache, and a new failure domain for a 300-line timer. The compositor already
sees every input event and owns DRM; the shell already owns the lock UI,
logind, and the OSD.

## Decision

1. **The compositor owns the live inactivity timer and DPMS.**
   - It tracks `last_activity` in the existing `notify_activity` path.
   - It arms a calloop timer at the next enabled stage and emits
     `idle-dim`, `idle-blank`, `idle-lock`, `idle-suspend`, and `idle-awake`
     through the existing `input_action` event.
   - It implements per-output DPMS through the DRM connector property and
     reports the state to the shell.
   - It honors `idle-inhibit` surfaces and the suspended state.
2. **The shell owns the user-facing effects.**
   - Dim/restore brightness through logind (ADR 0316 / T319).
   - Blank/restore through the compositor DPMS request (T316).
   - Lock through the existing `ext_session_lock_v1` path (T306/T305).
   - Suspend through logind and lock before `PrepareForSleep(true)` (T318).
   - Present the existing OSD; no second HUD.
3. **The policy keys stay `idle.dim`, `idle.blank`, `idle.lock`,
   `idle.suspend`, and `power.preventSleep`.** The shell forwards them to the
   compositor with one `set_idle_policy` request on startup and on change. The
   stage order is normalized exactly as ADR 0070 specifies.
4. **`services/session/src/idle.rs` remains the unit-test oracle.** The
   compositor implements the same normalized rules; a shared crate can be
   extracted later if the duplication grows. The session engine is not the
   production trigger for this gate.
5. **No new daemon.** A `dragonfruit-idle` process is not part of this set. If
   a future requirement needs per-user idle policy outside the compositor, a
   new ADR can revisit the placement.

Rejected alternatives:

- **A new idle daemon.** It adds a process, a Wayland connection, a D-Bus
  connection, and a failure mode without changing the behavior.
- **A shell-only timer.** The shell does not see input outside its chrome, so
  it would never fire while a window has focus.
- **Lock-only, no DPMS.** The Settings UI promises display-off; a black
  overlay is not a panel off and does not save power.
- **Keep the session engine as the production trigger.** It cannot see input
  or DRM without new plumbing that duplicates the compositor.

## Consequences

- The compositor grows a small `idle` module and a `set_idle_policy` request;
  the shell grows the stage handlers. The private protocol version increments.
- The `ext-idle-notify` protocol remains available to apps; the compositor’s
  own timer is independent of a client notification.
- The session engine’s tests stay; a future extraction can share the stage
  rules without changing the policy keys.
- T347 must prove the panel actually blanks and the session locks on the
  reference laptop; headless tests only prove the state machine.
