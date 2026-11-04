# 0199 — Refresh clock wiring: fastest period, pinned budget overrides, and visible refresh stats

## Status

accepted (T-21.2a, T187b)

## Context

ADR [0198](0198-per-output-refresh-model.md) added the pure per-output refresh
model (`OutputRefresh`, `RefreshSchedule`, millihertz input, 60 Hz fallback).
The animation clock, timer, and material degrade budget were still fixed at
16 ms. T187b wires the model into those live paths without introducing
per-output scheduling (T188) or material floors (T189).

Two constraints matter for later tasks:

- the timer must not be re-armed every frame when the period did not change,
  because the idle zero-wakeup guarantee depends on one armed timer only;
- a diagnostic budget override must remain usable for acceptance runs and be
  visible in the trace instead of being silently overwritten by an output
  mode.

## Decision

- **The clock owns one derived interval.** `AnimationClock` stores its current
  tick interval (default `FRAME_INTERVAL` until an output exists).
  `DfState::sync_refresh_policy()` sets it from the effective refresh's
  `period`; `schedule_animation_timer()` and `poll_animations()` both read that
  interval. Animation progress stays on monotonic elapsed time.
- **Effective refresh precedence is override → fastest active output →
  60 Hz fallback.** `DRAGONFRUIT_REFRESH_HZ` installs
  `RefreshSource::Override`; it is absent from a normal session. The schedule
  is keyed by output name and `fastest_period()` remains the empty-schedule
  fallback.
- **Recompute at every output-policy edge.** Output add/remove, mode change,
  scale/transform change (same recompute path), hotplug, and resume all
  refresh the schedule and apply the policy. If the interval changed while an
  animation is live, the armed timer is cancelled and re-armed with the new
  interval; otherwise it is left alone.
- **The degrade budget follows the deadline, but overrides pin it.**
  `DegradeController::set_budget_from_deadline()` updates the budget only while
  the source is `Refresh`. `DRAGONFRUIT_FRAME_BUDGET_US` records
  `BudgetSource::Env`; the synthetic `set degrade-budget` records
  `BudgetSource::Manual`. Both are reported as `budget_source=env|manual` on
  the degrade stats line.
- **Refresh is visible in the trace.** `presentation stats` appends
  `refresh_hz=<Hz>` and `source=mode|fallback|override`; `animation stats`
  appends `interval_us`; a `refresh stats` line reports the effective
  refresh, interval, deadline, output count, and override flag. Existing field
  names are unchanged.

## Consequences

- A 144 Hz panel drives a 6.944 ms tick and a 6944 µs budget; a 60 Hz output
  drives 16.666 ms / 16666 µs. A 143.999 Hz mode keeps its exact period.
- T188 can replace the single global tick with per-output deadlines; it must
  consume the schedule/`OutputRefresh` values rather than re-deriving them.
- T189 can select a material floor per role/output from the same deadline.
- Idle behavior is unchanged: the timer is armed only while an animation is
  live, and a settled output has no timer, damage, or client wakeups.
