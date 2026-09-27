# 0100 — Dock bounce phases and the pre-sized popover buffer

## Status

accepted

## Context

T-14.7b made the Dock plate and the compositor's backdrop follow a live
`plateRect` committed every frame (FR-14). The Dock's launch/attention bounce,
however, was still implemented by rebuilding the full `QVariantList` of entries
on a 16 ms timer and reassigning the Repeater model: every animation tick
recreated all delegates, dropped hover/press state, and allocated a fresh model
(~37 rebuilds per 600 ms launch bounce). Separately, opening a context
menu/chooser/stack computed a headroom/gutter from the popover rectangle and
`resize()`d the offscreen `QQuickWindow` on the open frame, reallocating the
render target mid-gesture.

## Decision

- **Bounce phases are a separate map, never the entry model.** The shell keeps
  the built entries free of `bounce`/`attention`; it publishes an
  `entry id -> { phase, attention }` map as the QML `bouncePhases` property on
  every animation frame. `Dock.qml` reads the phase through
  `entryPhase`/`entryAttention` for geometry and injects it into each delegate
  as `bouncePhase`/`bounceAttention`. The Repeater model — and therefore every
  live delegate's hover/press state — survives a bounce. An entry-embedded
  `bounce`/`attention` still wins so direct model callers keep working. The map
  is only pushed when it actually changes, so an idle Dock makes no binding
  churn.
- **The animation clock stops when idle.** The 16 ms tick only advances the
  phase map and schedules a commit; it stops as soon as both the launch and
  attention clocks are empty. The entry model is rebuilt only on a real model
  change (running-window projection, launch state, pin order), not on a quiet
  launch-timeout tick.
- **The offscreen buffer is pre-sized to a fixed popover budget.** The shell
  sizes the Dock window to `dock + 2 * popoverGutter` by
  `dock + popoverHeadroom` at configure time and never resizes it on a popover
  open. `Dock.qml` clamps every popover rectangle into that budget
  (`clampPopoverX`/`clampPopoverY`); a zero budget (the QML default) keeps the
  legacy unbounded placement. The Dock item sits at the constant
  `(gutter, headroom)` offset inside the larger buffer, so committed
  surface-local coordinates are unchanged.
- **Discrete layout changes animate; magnification stays progress-based.** A
  root `Behavior on iconSize` (gated off during a divider resize and before the
  first configure) animates `dock.size`/overflow changes with
  `motion.dock-magnify`; the per-entry x/y Behaviors cover the drag gap and its
  close. Magnification never passes through a Behavior.

## Consequences

- The T-14.7c trace (`docs/captures/t14-dock-motion-trace.txt`) shows the
  sweep, size-change, and idle windows at zero over-budget frames; the nested
  popover open/close shows a handful (5/63) with no degrade-tier downgrade. The
  pre-sized buffer's extra readback therefore did **not** trip the degrade
  controller, so the split-into-a-separate-window fallback is deferred until a
  real-hardware trace shows a tier downgrade.
- Later Dock units must use `popoverHeadroom`/`popoverGutter` for popover
  placement and `bouncePhases` for any new per-frame entry state; mutating the
  entry list to carry per-frame state is now a regression.
- `Dock.qml`'s `laidOutOnce`/`dragSettling`/`Behavior on iconSize` are the
  motion contract: the first configure snaps, everything after springs.