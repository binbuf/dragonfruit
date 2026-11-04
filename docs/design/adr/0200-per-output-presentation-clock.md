# 0200 — Per-output presentation clock: monotonic timestamps, re-anchoring deadlines, backend sources

## Status

accepted (T-21.2b, T188a)

## Context

ADR [0198](0198-per-output-refresh-model.md) gives each output a period and
deadline; ADR [0199](0199-refresh-clock-wiring.md) wires the fastest period
into the one global animation clock. T188 needs to know, per output, when that
output last presented and when it is next due. That state must be testable
without DRM and must not be tied to the renderer or the event loop yet.

The two backends have different timestamp sources. DRM's `frame_finish()` has
the kernel vblank `DrmEventTime` (falling back to the compositor clock) and
knows whether `frame_submitted()` returned a queued frame. Smithay's nested
winit `submit()` only swaps the buffer and returns
`Result<(), SwapBuffersError>`; it exposes no platform presentation timestamp.

## Decision

- **One pure module, no renderer dependency.** `compositor/src/presentation.rs`
  (exported as `dragonfruit_compositor::presentation`) owns
  `PresentationClock` and the name-keyed `PresentationClocks` map. Timestamps
  are monotonic `Duration`s supplied by the backend, never `Instant`s.
- **The clock consumes `OutputRefresh`.** `period`, `refresh_hz`, and `vrr`
  come from the same resolved refresh as the animation clock and degrade
  budget; the clock never re-derives a period. A bare period is accepted only
  through `with_period()`, which applies the same 24–500 Hz policy and falls
  back to 60 Hz for zero.
- **Deadlines re-anchor on the actual presentation.** `note_presented(t)` sets
  `last_presented = t` and `next_deadline = t + period`. A missed deadline
  stays due (time-until-due saturates at zero) and the next presentation
  re-anchors from `t`; it never queues catch-up frames. A stale timestamp
  older than `last_presented` is ignored.
- **An idle clock has no samples.** Before the first presentation,
  `last_presented` and `next_deadline` are `None`, and `is_due()` reports the
  first frame as due. The clock only acquires a deadline through
  `note_presented()`.
- **Backend sources are explicit.** DRM records the vblank timestamp only when
  `frame_submitted()` confirms a queued frame. Nested records
  `state.clock.now()` immediately after a successful damage `submit()` as its
  completion timestamp; the no-damage swap is not a presentation. The fixed
  mode period remains the deadline source, and no single interval is used to
  infer VRR.
- **T188a only records.** `DfState` owns `PresentationClocks` and updates it on
  output add/remove/mode change and resume; no frame request, timer, or render
  gate changes here.

## Consequences

- T188b can arm one timer from the earliest due clock and update only the
  presenting output; a 60 Hz output cannot be forced onto a 144 Hz tick. It
  must gate `is_due()` on its per-output request set: a never-presented idle
  output reports "first frame due" but is not itself a render request.
- Mode changes preserve the last presentation and rebase the deadline; an
  output that disappears while suspended is dropped from the map.
- T188c owns VRR adaptation and mixed-refresh evidence; the clock carries
  `VrrState` but never changes its period from a measured interval.
- The nested rail is a single-output correctness rail only; its submit
  completion is not a hardware vblank timestamp.
