# 0198 — Per-output refresh model: millihertz input, period deadline, 60 Hz fallback

## Status

accepted (T-21.2a, T187a)

## Context

The animation clock, timer, and degrade budget are fixed at 16 ms (60 Hz).
The live panel reports 143.999 Hz, whose deadline is 6.94 ms. Before wiring a
clock, the compositor needs one pure, testable representation of each output's
refresh so the 60/120/144/240 Hz table can be pinned without hardware.

Smithay's `Mode::refresh` is **millihertz** (`143_999` = 143.999 Hz), and the
backends already pass that value to presentation stats. The model must accept
that unit directly and must not round a real panel to a nominal rate.

## Decision

- **One pure module, no renderer dependency.** `compositor/src/refresh.rs`
  lives in the compositor library (`dragonfruit_compositor::refresh`) and
  exposes `OutputRefresh`, `RefreshSchedule`, `ModeRefresh`, `RefreshSource`,
  and `VrrState`. It takes a name plus a mode refresh; it never takes a
  Smithay `Output`.
- **Millihertz in, nanoseconds out.** `OutputRefresh.refresh_mhz` stays in
  millihertz and `period` is `1e12 / refresh_mhz` nanoseconds. A valid
  143.999 Hz mode therefore keeps 6,944,492 ns; it is not rounded to 144 Hz.
- **The deadline is the period.** `deadline == period` in this model. T188 may
  subtract a backend submit margin; the pure model does not invent one.
- **Fallback and clamp are separate.** No mode, zero, or a negative refresh
  installs the 60 Hz fallback (`source=fallback`). A positive mode outside the
  sane range is clamped to 24–500 Hz but still reports `source=mode`.
- **VRR is carried, not interpreted.** `VrrState` is `Unknown`/`Off`/`On`.
  It never changes `period` in T187a; T188 decides what a variable interval
  means.
- **Fastest is the shortest period, with an empty-schedule fallback.**
  `RefreshSchedule::fastest_period()` returns the minimum active period and
  the 60 Hz fallback when there are no outputs, so the clock always has a
  valid interval. A `BTreeMap` keeps iteration and tie-breaking deterministic.
- **Override is a source, not a hidden mode.** `RefreshSource::Override` and
  `as_str()` exist now so T187b can add `DRAGONFRUIT_REFRESH_HZ` without
  changing the model's shape.

## Consequences

- T187b can map `output.current_mode()` into `ModeRefresh` and drive the
  animation interval and degrade budget from `fastest_period()` / `deadline`.
- A headless or pre-mode output is scheduled at 60 Hz, never treated as fast.
- Stats can append `source=mode|fallback|override` and `refresh_hz` without
  re-deriving the period.
- T188 extends this model with per-output presentation clocks and VRR
  adaptation; it must not reinterpret the stored period as a half-period
  budget.
