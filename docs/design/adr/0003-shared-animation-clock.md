# 0003 — One shared compositor animation clock

## Status

accepted

## Context

Loop v1 (T-02) adds lifecycle motion — window appear, minimize/restore, zoom,
close — on top of the T-01 loop. Every one of those transitions has the same
frame discipline: one compositor frame per animation frame while it runs, and
zero damage (no timer, no client wakeups) when nothing animates. The overview
already had a one-off calloop timer (`overview_timer`, T-11); a second timer
per transition would drift the frame pacing, and each new transition would
re-implement the idle guarantee and the reduced-motion collapse
(`accessibility.reduceMotion`: one step through the same commit path).

The design-system owns the curves and durations ([10-design-system.md]);
T-02 through T-05 and T-16 all need them.

## Decision

There is exactly one animation clock, `compositor/src/animation.rs`:

- `AnimationClock<S>` holds the running `Animation<S>` set, the reduced-motion
  flag, and the frame counters. A `Tween` (start, duration, design-system
  curve) is the shared timing/easing primitive.
- A single calloop timer in `compositor/src/input.rs`
  (`schedule_animation_timer`) is armed **only** while
  `DfState::animations_active()`; each tick calls `poll_animations`, which
  advances every animation exactly once, requests exactly one redraw, and
  re-arms until nothing is live. `overview_timer` is gone: the overview's
  discrete slide is one of the clock's drivers.
- `DfState::set_reduced_motion` writes the one flag on both the overview
  machine and the clock; a transition consults it when it builds its `Tween`
  (`Tween::from_motion`), so reduced motion is the same path with a shorter
  duration, never a second path.
- `frames_stepped` is the instrument: the render-path `frames_rendered`
  must advance by exactly the same count while an animation is live, and
  both must stay flat when idle.

## Consequences

- T-02.1b…T-02.4 register their transitions with
  `DfState::start_animation`; T-04/T-05 reuse the same clock and `Tween`.
- No transition may arm its own timer or drive a discrete "instant" path;
  that is the bug the clock exists to prevent.
- T-03.1a's 60 s trace compares `frames_rendered` to
  `animation_frames_stepped` (both on the `SIGUSR1`/exit stats line).
- The clock's frame interval is fixed at 16 ms for now; per-output refresh
  rate is T-16's extension point (`AnimationClock::frame_interval`).
