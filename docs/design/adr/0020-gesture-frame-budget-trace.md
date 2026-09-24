# 0020 — The overview gesture frame budget is a `DfState`-owned, gesture-scoped trace

## Status

accepted

## Context

T-05.6 must judge the full Mission Control gesture against one 60 Hz frame and
record the shortfall honestly. The T-03 idle/latency instruments
([0009](0009-idle-trace-instrumentation.md), [0010](0010-latency-and-scanout-instruments.md))
measure the whole session, so idle frames hide the gesture cost. The overview
state that scopes a gesture already lives on `DfState`
(`overview.is_active()`, `overview_active`, `desktop_revealed`), and T-06's app
switcher builds on the same transform and recency data, so whatever reads the
budget must read one shared instrument rather than re-deriving it per caller.

## Decision

- `instrument::GestureBudgetTrace` is a pure timing-plus-counters instrument
  with no clock of its own; it is unit-testable and inert while idle.
- It lives on `DfState`, **not** inside `RenderStats`, because `DfState` owns
  the overview state that scopes it.
- `DfState::observe_rendered_frame` is the single per-rendered-frame entry
  point (the session loop already had one timing seam): it records the global
  trace, feeds the T-04.4a degrade controller, and records/finishes the gesture
  trace from the overview-active predicate. Callers never feed the gesture
  trace directly.
- The trace keeps **render duration** (over-budget frames) and **presented
  interval** (dropped presentations, past `1.5 ×` the budget) distinct, so a
  slow-but-caught-up frame is not a dropped presentation and a jittery cadence
  is not a slow frame. `held` is the honest verdict (≥ 1 frame, no miss).
- `query gesture` and the `gesture budget` stats line report the trace plus the
  live T-04.4a `tier`, the seam that shows the budget held *because* the
  material degraded.

## Consequences

- T-06 (and T-16) read `query gesture`/`gesture budget`; they do not add a
  second gesture timer or a second overview-active predicate.
- The nested capture (`scripts/capture-overview.sh`,
  `docs/captures/t05-gesture-budget-nested.txt`) records the real shortfall
  (`held=0`) on the development iGPU; the acceptance is "within budget **or**
  shortfall recorded", so a red `held` is data, not a failure.
- A new overview transition must be expressible through the existing
  `overview` state predicates; adding one that is not covered by
  `overview_frame_budget_active` would silently drop it from the trace.