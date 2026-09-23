# T-02 — Loop v1: Window Lifecycle Motion

> **Track, not a single slice.** This file is the design reference. It is executed as 4 one-session units: [T-02.1](units/007-t-02.1-animation-clock-and-window-appear.md) · [T-02.2](units/008-t-02.2-minimize-and-restore-motion.md) · [T-02.3](units/009-t-02.3-zoom-and-fullscreen-transitions.md) · [T-02.4](units/010-t-02.4-close-ghost-interruptibility-capture.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 2 of 17 — motion that makes the loop legible |
| **Area** | `compositor/` (animation clock, per-window transitions) · `shell/` (Dock tile geometry hand-off) |
| **Depends on** | T-01 |
| **Blocks** | T-04 (transform reuse), T-05 |
| **Legacy detail** | [legacy/35-window-lifecycle-animations.md](legacy/35-window-lifecycle-animations.md) · [legacy/11-mission-control-workspace-ux.md](legacy/11-mission-control-workspace-ux.md) (animation clock) · [legacy/10-dock.md](legacy/10-dock.md) (tile geometry) |

## Demo

The T-01 loop with motion:

```
launch app → the window scales/fades in from its Dock entry (the entry keeps
bouncing) → minimize shrinks it into the Dock → restore grows it back out →
zoom animates between geometries → close fades/scales out while the surface
is held as a ghost
```

Capture: `docs/captures/t02-lifecycle-motion.*`, plus a reduced-motion capture
showing instant transitions.

## Why now

T-01 makes the loop *possible*; motion makes it *readable*. This is also the
slice that establishes the animation clock and per-window transform machinery
that T-04's scene pass and T-05's overview compose. Doing it before materials
means the visual pass lands on a system that already animates correctly.

## Inherited and reused

- The discrete-trigger animation clock and progress pipeline from the legacy
  T-11 work (`OverviewMachine`, `DfState` frame timing).
- Dock launch/attention bounce and tile geometry (`legacy/10-dock.md`).
- Window state machine + restore geometry (`legacy/04-window-model.md`).
- Design-system motion tokens (durations, curves, reduced-motion collapse)
  (`legacy/08-design-system.md`).
- `FrameCommitGate` and the shell's scene-graph commit path.

## Scope

### In

1. **Compositor animation clock** shared with T-04/T-05: one frame of
   compositor work per frame of animation; **no animation → no damage**.
2. **Window appear**: scale/fade from the owning Dock entry's tile geometry
   (provided by the shell over the private protocol — an additive request is
   fine) or a centered origin when there is no entry; coordinated with the
   existing launch bounce and xdg-activation.
3. **Window close**: fade/scale out with the surface held as a ghost until the
   animation completes; the window is input-inert and removed from the model
   only at completion.
4. **Minimize / restore**: scale into and out of the owning Dock entry's tile;
   respects `dock.minimizedAnimation` (`scale` in this slice; `genie` remains a
   post-gate effect, `none` is the reduced-motion fallback).
5. **Zoom / fullscreen**: animate between geometries using the same clock.
6. **Reduced motion**: every transition takes a single step through the same
   commit path; the state change stays legible.
7. **Interruptibility**: a new transition mid-flight reverses or retargets
   without waiting, using the existing progress semantics.

### Out / explicitly deferred

- Blur/shadow/rounded corners (T-04).
- Genie minimize (post-gate).
- Multi-monitor animation polish (T-16).

## Acceptance

- [ ] The demo runs and the capture is committed.
- [ ] The reduced-motion capture shows instant, still-legible transitions.
- [ ] Headless test asserts the clock produces one compositor frame per
      animation frame and zero frames when idle (extend the idle trace).
- [ ] The minimize → restore → close sequence resolves with no orphaned ghost
      and no stale Dock entry.
- [ ] `make soak` passes.
- [ ] `make e2e` stays green.

## Test plan

- Headless: drive minimize/restore/zoom/close with the synthetic-input harness;
  assert the animation clock's frame count, the final geometry, and the
  `Closed` event after the close animation.
- Headless: idle trace — after the loop, `frames_rendered` stays flat.
- Nested: capture review, including reduced motion.
- Regression: T-01 demo still passes.

## Risks

- **Ghost lifetime.** A close animation must not keep the window in the input
  or focus path; verify no phantom focus target.
- **Clock vs. damage.** The legacy idle budget is strict; animation must stop
  the clock and leave zero damage.
- **Tile geometry coupling.** Keep the shell → compositor tile hand-off
  additive and optional; the compositor must degrade to the centered origin
  when the shell is absent (headless).

## Hand-off

- T-04 reuses the per-window transform to add scale/clip/blur.
- T-05 reuses the clock for the overview transition.
- T-16 extends the animation paths to multi-monitor.
