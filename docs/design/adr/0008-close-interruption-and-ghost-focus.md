# 0008 — A close ghost is interruptible and releases focus

## Status

accepted

## Context

T-02.4a models close as a ghost: the window leaves the layout and input path
immediately and is removed from the model only when its `Close` motion settles
(ADR 0007). Two gaps remain for the T-02 acceptance:

- **Interruptibility.** The design-system rule is that every transition is
  progress-based and reversible; a discrete instant path is a bug. A close must
  therefore be cancellable before it settles (the shell/Dock activating the
  window, a restore, or an explicit cancel) and reverse from where it is,
  without waiting for the first motion to finish.
- **Phantom focus.** The ghost is unmaped from `Space`, but the seat's keyboard
  focus and the model's `active_window` still name it until the client
  destroys the surface. During the fade/scale-out the closing window is a
  phantom focus target: it can still receive keys and the shell reports a
  focused window that is input-inert.

Smithay only invokes `SeatHandler::focus_changed` when the focus is **set**; on
an unset it calls `leave` and not the handler. The compositor's own focus
bookkeeping (`active_window`, the `Unfocused` broadcast, the shortcut scope)
must therefore be updated by hand when a ghost releases focus.

## Decision

- Add `DfState::interrupt_close(window)` (and `interrupt_close_by_id`). While a
  live `Close` motion exists it re-maps the window into the layout/input path
  at once, replaces the `Close` with a `Restore` whose origin is the **current
  interpolated ghost rect** (`begin_window_motion_from` reads the live frame),
  and returns focus to the window. It is a no-op when nothing is closing.
  `restore_window` delegates a closing target to it, so the natural
  restore/activate path reverses a close.
- Replacing the `Close` motion drops the pending removal with it:
  `step_window_motions` commits removal only for a motion whose kind is still
  `Close`, so an interrupted close broadcasts no `Closed` and never removes the
  window. There is still exactly one teardown path (`remove_window`).
- `close_window` releases keyboard focus at ghost time: it clears the seat
  focus and, because smithay does not report an unset to `focus_changed`, it
  mirrors the change itself (`active_window = None`, `Unfocused` broadcast,
  shortcut scope cleared).
- The synthetic harness gains `interrupt-close <id>` (alias `reopen <id>`) so
  the reversal is drivable headless. No second clock/driver is registered.

## Consequences

- The close ghost now satisfies both the reversal rule and the "no phantom
  focus target" risk from the T-02 track.
- The idle trace after a close/reverse/close loop is asserted flat (no damage,
  no clock steps) in `window_conformance.rs`; `SIGUSR1` render counters are the
  measurement.
- Shell/Dock "activate a closing window" wiring is not done here: the T-07
  protocol seam should call `interrupt_close` when it lands (follow-up).
- `after_workspace_change` drops focus with the same smithay unset behavior and
  still leaves `active_window`/`Unfocused` unmirrored; left as a follow-up
  rather than folded into this task.
