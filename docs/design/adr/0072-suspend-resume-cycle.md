# 0072 — Suspend/resume cycle ownership and recovery

## Status

accepted

## Context

T-12.4b ends the idle chain at `IdleEvent::Enter(Suspend)` and reports
`Restore(Suspend)` on wake, but nothing suspends the machine and nothing
quiesces the compositor for sleep. T-12.5a has to make one suspend/resume
round trip recover **outputs, input, and clients without a restart**. The
platform signal is logind's `PrepareForSleep`; the request side is
`org.freedesktop.login1.Manager.Suspend`.

## Decision

- **The recovery state lives in the compositor.** A pure
  `SuspendModel` (`compositor/src/suspend.rs`) holds the one `suspended` flag
  and a completed-cycle count. `DfState::suspend_session` disarms the
  animation timer, closes the post-input cadence, and drops pending redraws;
  `DfState::resume_session` requests a repaint of every output, notifies idle,
  and re-arms anything still animating. Every client, output, and scene object
  is kept, so recovery is a repaint and never a reconnect or restart. The
  session loop and the DRM `render_surface` skip rendering while suspended,
  and `process_input_event` drops user input (device hotplug still routes).
- **The request state lives in `dragonfruit-session`.** `SuspendCycle` is a
  pure state machine (`Awake` / `Requested` / `Asleep`) and
  `SuspendController<B: SuspendBackend>` binds it to the platform seam:
  `Enter(Suspend)` calls `request_suspend`, activity before sleep lands calls
  `cancel_suspend`, and logind's `PrepareForSleep` confirms sleep/resume and
  counts the cycle. The backend trait is the only platform coupling, so the
  real logind client and the CI `MockSuspend` are interchangeable.
- **The headless trigger is synthetic.** The existing
  `DRAGONFRUIT_SYNTHETIC_INPUT` harness gains `suspend`, `resume`, and
  `query session`, so both the headless conformance test and the nested
  capture command drive the exact same production path without hardware.

## Consequences

- The later 100-cycle soak (T-16) asserts `suspended=0 cycles=N` and a live
  scene; the counter is the contract.
- T-12.5b/real-session work supplies the concrete logind `SuspendBackend` and
  forwards `PrepareForSleep` to the compositor; on a VT switch the DRM session
  pause/activate path already drives the same model.
- Waking from sleep is not an unlock: the lock UI/PAM path still owns
  unlocking, and a suspended-but-locked session stays locked across the cycle.