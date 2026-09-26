# 0070 — Idle timer engine and policy shape

## Status

accepted

## Context

T-12.4a ships the dim → blank → lock → suspend chain. Three later tasks build
on it: T-12.4b wraps the engine with idle inhibitors and wake restore, T-12.5a
drives one suspend/resume cycle from the `Suspend` stage, and T-12.5b adds the
settingsd policy keys and wires them in. The design reference calls the
thresholds "compositor-owned" (`legacy/26`), while the task plan puts T-12.4a
and T-12.4b in `services/session`; the engine's home and its policy contract
have to be settled once so those tasks do not each invent a different shape.

## Decision

- **The engine lives in `dragonfruit-session`** as `src/idle.rs`, not in the
  compositor. [`IdlePolicy`] and [`IdleTimers`] are dependency-free and
  clock-injected (`Duration` in, transitions out), so the named acceptance —
  a fake clock drives every stage — is a headless unit/integration test. T-12.4b
  adds inhibition as a caller-side decision and T-12.5a/T-12.5b consume the
  chains; the compositor keeps the `ext-idle-notify`/`idle-inhibit` *protocols*
  it already serves, but the policy chain is session state.
- **Stages are ordered and monotonic**: `Active < Dim < Blank < Lock < Suspend`.
  A policy is normalized so an enabled stage's effective delay is never earlier
  than an earlier enabled stage; a disabled stage (`None`) is skipped and does
  not constrain later ones. One `poll` jumps to the furthest due stage.
- **Policy keys are `idle.dim`, `idle.blank`, `idle.lock`, `idle.suspend`,
  whole seconds, `0`/negative/`never` disables.** `IdlePolicy::from_keys` reads
  those pairs from a settings snapshot and ignores unrelated keys; T-12.5b
  registers the keys in settingsd and calls it. Defaults (until then): dim
  150 s, blank 300 s, lock 600 s, no suspend.

## Consequences

- The compositor and the session cannot share the type directly (no dependency
  edge between the crates); when a real idle service binds `ext-idle-notify`,
  it feeds the engine and the compositor is told to dim/blank/lock over the
  existing private protocol. This is the seam T-12.4b/T-12.5a use.
- `IdleTimers::activity` both resets the inactivity clock and wakes the chain;
  T-12.4b's wake restore calls it and reapplies the prior screen state.
- Inhibitors are deliberately absent from the engine: a caller holding one
  simply does not call `poll`, which keeps the state machine pure.