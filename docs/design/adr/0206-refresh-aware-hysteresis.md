# 0206 — The degrade hysteresis is a wall-clock window with a flap latch

## Status

accepted

## Context

`DegradeController` selected its tier with fixed frame counts: 12 over-budget
frames downgraded and 180 clearly-under-budget frames recovered. At 60 Hz that
is 200 ms and 3 s, but at 144 Hz it is 83 ms and 1.25 s: the same panel load
degrades and recovers ~2.4x faster. A steady pointer sweep whose cost depends
on the tier (Full misses the deadline, Reduced fits) is then a classic
Full → Reduced → Full limit cycle, not a stable selection.

## Decision

- The hysteresis windows are wall-clock `Duration`s, accumulated from the
  rendered-frame durations the session loop already feeds the controller:
  - downgrade after `200 ms` of over-budget frames (12 × 16.666 ms at 60 Hz);
  - recover after `3 s` of clearly-under-budget frames (180 × 16.666 ms at
    60 Hz).
  The EMA (`0.3`) and recovery ratio (`0.75`) are unchanged. A 120/144 Hz
  panel now takes the same wall-clock time as 60 Hz; only the frame count
  scales.
- A downgrade within `10 s` of an upgrade across the same ladder boundary is a
  flap. That boundary is **latched**: recovery across it needs the EMA below
  `0.4 × budget` for `10 s`, not the normal `0.75 × budget` for `3 s`. A
  successful upgrade clears the latch; the next downgrade re-latches it if the
  load has not actually settled. A forced tier resets the selection memory.
- `degrade stats` / `query degrade` keep their existing fields and add
  `down_window_us`, `up_window_us`, and `latched` (0/1) so a trace can prove
  the policy without parsing frame counts.
- The synthetic harness gains `set degrade-frames <count> <microseconds>`,
  which feeds the real `observe_rendered_frame` path. The headless load test
  and the committed trace use it; there is no parallel test-only controller.

## Consequences

- A 60 s steady sweep at 60/120/144 Hz now shows at most one probationary
  recovery before the latch holds `Reduced`; the committed trace records
  `downgrades=2 upgrades=1 latched=1` at every rate. A genuinely lighter load
  (below the latch threshold) still recovers, so the latch is not sticky.
- The fallback ladder, the per-role floor, and the shell `MaterialPolicy` are
  unchanged. The shell applies the same change-only role states; the load
  tests only make the compositor's selection deterministic.
- The windows are time-based, not refresh-locked: the policy still works when
  an output has no refresh model (the 16.666 ms fallback budget) and when the
  budget is pinned by `DRAGONFRUIT_FRAME_BUDGET_US` or `set degrade-budget`.
- A pathological tier cost ratio can still flap once per latch window; that is
  bounded and visible in `degrade stats`, and is preferable to an unbounded
  per-3-second cycle.

## References

- `compositor/src/window/degrade.rs`
- `compositor/src/input/synthetic.rs`
- `compositor/tests/shell_protocol_conformance.rs`
- `scripts/t21-degrade-load.sh`, `docs/captures/t21-degrade-load.txt`
- ADR 0015 (material degrade tiers), ADR 0203 (role-keyed floor)
