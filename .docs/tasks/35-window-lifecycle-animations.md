# T-35 — Window and App Lifecycle Animations

| | |
|---|---|
| **Phase** | 2 · Experience (compositor-driven) |
| **Area** | `compositor/` (animation clock + per-window transitions) + `shell/` (Dock origin geometry) |
| **Depends on** | [T-02](02-compositor-core.md) (frame pacing) · [T-04](04-window-model.md) (window states) · [T-33](33-compositor-effects-materials.md) (transform/blur pass) · [T-08](08-design-system.md) (motion tokens) · [T-10](10-dock.md) (tile geometry) |
| **Blocks** | [T-34](34-mvp-vertical-slice-gate.md) · Phase-2 exit (loop animations) |
| **Estimate** | L |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [04-shell.md](../design/04-shell.md) · [10-design-system.md](../design/10-design-system.md) · [ROADMAP.md](../ROADMAP.md) |

## Summary

The compositor-owned motion for the **per-window lifecycle**: window open
(appear), close, minimize-into-Dock and restore, and Zoom/fullscreen
transitions. The design system owns the curves; T-33 owns the materials;
T-11 owns the workspace/overview transforms; this ticket owns everything
else in the 30-second loop that currently has no owner ("window appears",
"minimize", "restore from Dock", plus the launch-appear origin).

## Background

The vertical slice and the loop name these animations explicitly
([ROADMAP.md](../ROADMAP.md)): *window appears*, *minimize*, *restore from
Dock*. T-10 owns the Dock bounce, T-11 owns workspace switch and the
overview, T-33 owns the transform/blur machinery they compose, but no ticket
currently owns the window's own open/close/minimize/zoom motion. Without it
the loop is functional but abrupt — the same class of gap as the missing
materials.

Motion policy is fixed by the design system
([10-design-system.md](../design/10-system-design.md)): every animation has
a reduced-motion variant, and gesture/transition paths are progress-based
and interruptible with no discrete "instant" path.

## Scope

### In scope

1. **Animation clock / timeline**, shared with T-11 and T-33: one frame of
   compositor work per frame of animation (T-02 FR-4); no animation → no
   damage (T-02 FR-2).
2. **Window appear**: scale/fade from the owning Dock entry's tile geometry
   (provided by the shell over T-07) or a centered origin when there is no
   entry; coordinated with the launch attention/bounce (T-10) and
   `xdg-activation` (T-02/T-10).
3. **Window close**: fade/scale out with the surface held as a ghost until
   the animation completes; the window is input-inert and removed from the
   model only at completion.
4. **Minimize / restore**: scale into (and out of) the owning Dock entry
   with a reduced-motion crossfade fallback; windows with no entry (hidden
   Dock, no identity) use a documented origin.
5. **Zoom / fullscreen**: interruptible geometry tween driven by T-04 state
   transitions; re-entrant requests resolve to the latest target.
6. **Interruptibility and correctness**: mid-flight input reverses or
   completes without waiting; a transition that cannot finish (window
   destroyed, output removed) degrades to the end state, never a stuck
   ghost.
7. **Reduced-motion variants** for every transition (translation removed,
   state change kept legible).

### Out of scope

- Materials/blur/shadows ([T-33](33-compositor-effects-materials.md)).
- Workspace switch and Mission Control motion ([T-11](11-mission-control-workspace-ux.md)).
- Dock internal motion: bounce, magnification, popovers ([T-10](10-dock.md)).
- SSD titlebar hover reveal ([T-13](13-window-decorations-ssd.md)).
- Notifications/OSD/launch OSD ([T-25](25-notifications-and-osd.md)).

## Requirements

- **FR-1**: The loop steps *window appears*, *minimize*, and *restore from
  Dock* are animated at 60 Hz with zero dropped frames on baseline
  Intel/AMD (measured, not eyeballed).
- **FR-2**: Every transition is progress-based and interruptible; a
  discrete "instant" path is a bug (design-system rule).
- **FR-3**: Reduced-motion variants pass for every transition (Phase-2 exit
  criterion).
- **FR-4**: Appear origin follows the owning Dock entry; a window with no
  entry still animates from a defined fallback (never a jump).
- **FR-5**: Close/minimize never leave a ghost or a held input grab when
  the client disconnects mid-transition.
- **FR-6**: Idle desktop stays zero-damage: an animation that isn't running
  costs nothing.

## Acceptance criteria

- [ ] Nested capture review of appear / close / minimize / restore /
      zoom against the design-system references; light/dark +
      reduced motion.
- [ ] Frame-time trace across the full loop within budget.
- [ ] Interrupt-during-transition suite passes (input mid-flight, destroy
      mid-flight, output detach mid-flight).
- [ ] Idle-trace assertion still flat with animations armed but idle.

## Test plan

- Headless: timeline/curve unit tests, state-machine transitions with a
  fake clock, interrupt and teardown cases.
- Nested: visual capture + frame timing; the loop walkthrough shared with
  T-34.
- Real hardware: frame-time budget confirmation.

## Risks / open questions

- Minimize-into-Dock needs a reliable shell→compositor tile-geometry
  round-trip (T-07/T-10); define the stale-geometry behavior (Dock moved/
  hidden/multi-monitor) before implementation.
- Multi-monitor appear origin and focus: which output's Dock owns the
  animation — coordinate with T-11's per-output rules.
- Keep the animation set small and token-driven; a per-feature bespoke
  transition is how motion systems drift.