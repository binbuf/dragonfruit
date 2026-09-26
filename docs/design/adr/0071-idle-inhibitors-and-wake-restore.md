# 0071 — Idle inhibitors and wake restore

## Status

accepted

## Context

T-12.4a ships the pure `IdleTimers` engine and deliberately keeps inhibitors
out of it: a caller "simply declines to call `poll`" (ADR 0070). T-12.4b has to
make that caller-side decision concrete — honor inhibitors, restore state on
wake — without putting client surfaces or compositor types into the engine.
T-12.5a (suspend/resume) and T-12.5b (settingsd keys) consume the result, so
the wrapper's shape is a contract.

## Decision

- **`IdleController` wraps `IdleTimers` in `services/session/src/idle.rs`.** It
  owns an `IdleInhibitors` registry and returns `IdleEvent`
  (`Enter(stage)` / `Restore(stage)`), which names both the stage advanced to
  and the stage a wake restored from. The engine stays pure and
  surface-agnostic: the future idle service maps each compositor
  `idle-inhibit` surface to one opaque `InhibitorId`.
- **An inhibitor freezes the chain.** While any handle is held,
  `IdleController::poll` does nothing and `next_deadline` is `None`. Acquiring
  an inhibitor forces the chain back to `Active` (documented on
  `IdleStage::Active`) and reports `Restore(prior)`; releasing one neither
  moves the chain nor resets the inactivity clock, so a chain held past a
  deadline catches up on the next `poll`. A policy change while inhibited is
  stored but not evaluated until the next poll.
- **Wake reports, it does not act.** `activity` and a new inhibitor return
  `Restore(stage)` with the stage left. Waking from `Lock` reports
  `Restore(Lock)` but the controller never unlocks; the lock UI owns
  unlocking, preserving the T-12.3 fail-secure boundary.

## Consequences

- T-12.5a drives suspend from `IdleEvent::Enter(Suspend)` and resume/restore
  from `Restore(Suspend)`; T-12.5b feeds settings into
  `IdleController::set_policy`.
- Handles are unique only within a registry, so the service must recreate the
  controller (or call `clear_inhibitors`) across restarts; stale handles from
  another registry are not a supported input.
- The compositor's own `idle-inhibit` state stays protocol-level; the session
  wrapper does not read it directly. The idle service is the one adapter that
  translates one into the other.