# 0201 — Per-output frame requests and earliest-deadline animation arming

## Status

accepted (T-21.2b, T188b)

## Context

ADR [0198](0198-per-output-refresh-model.md) and [0199](0199-refresh-clock-wiring.md)
give each output a period, but the animation clock still armed one timer at the
fastest active period and every tick set the global `needs_redraw` flag. Both
backends therefore rendered every output on the fastest tick: a 60 Hz output
was forced to present at 144 Hz whenever another output animated, and a settled
output could be woken by another output's clock. ADR [0200](0200-per-output-presentation-clock.md)
added the pure per-output presentation clock but deliberately did not schedule.

## Decision

- **Two request kinds, one pure scheduler.** `compositor/src/frame_scheduler.rs`
  (exported as `dragonfruit_compositor::frame_scheduler`) owns per-output
  `urgent` and `animation` bits plus an in-flight marker. It has no timer or
  renderer dependency.
- **Urgent work never waits.** Input, client/chrome commits, resize, and a
  global scene change remain urgent. `DfState::needs_redraw` is the global form
  (all outputs urgent); the scheduler's per-output urgent bit is the same
  immediate path. Urgent work is not gated by a deadline or by an in-flight
  animation frame.
- **Animation work is per output and deadline-gated.** An animation request is
  presented only when that output's `PresentationClock::is_due(now)` is true
  and no frame is already queued. A 144 Hz tick therefore does not present a
  60 Hz output every 6.94 ms. The animation tick no longer sets the global
  redraw flag; it reconciles the request set with the motions that are actually
  live (window motions on their output, overview/workspace transitions on every
  output).
- **The timer arms from the earliest due request.** `FrameScheduler::next_interval`
  is the minimum `time_until_due` over requested, non-in-flight outputs. A
  never-presented clock legitimately arms zero for its first frame. A clock
  that has presented and is still due returns its period instead of zero, so a
  no-damage frame cannot busy-loop; the next presentation re-anchors it.
  `frame_finish()` / nested `submit()` / the headless render pass update the
  clock and re-arm immediately.
- **A settled output drains once and then stops.** The request stays through the
  final animation step, is presented on that output's own deadline, and is
  cleared only after the settled frame presents. No request means no timer and
  no render. A timer that was armed before the final frame is cancelled when no
  work remains.
- **In-flight means one queued output-frame.** The DRM rail marks an output
  in-flight when `queue_frame()` succeeds and clears it at `frame_finish()`.
  The scheduler ignores in-flight outputs for both the render gate and timer
  arming, so a queued frame is not followed by an empty retry that fights the
  next deadline.
- **Observability.** `frame_finish()` keeps the existing 1.5× period missed
  rule; `presentation stats` already reports `refresh_hz`. A new
  `frame scheduler stats` line plus one `frame request` line per output expose
  the request bits, in-flight state, timer armed state, and clock
  `refresh_hz`/deadline on every backend.

## Consequences

- Mixed-refresh scheduling works from one timer without a global 16 ms fallback
  or a busy loop; the 60 Hz output presents at its own deadline while the fast
  output keeps its cadence.
- T188c owns VRR adaptation, chrome frame callbacks, and the native 60+144
  evidence. The scheduler carries no VRR policy; it consumes the clock period.
- The nested rail remains a single-output correctness trace, not mixed-refresh
  proof. Headless now treats its render pass as the presentation timestamp so
  the same scheduler is exercised without a display.
