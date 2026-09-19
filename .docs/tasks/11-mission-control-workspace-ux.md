# T-11 — Mission Control & Workspace-Switching UX

| | |
|---|---|
| **Phase** | 2 · Experience |
| **Area** | `compositor/` (overview state machine) + `shell/` (strip/chrome) |
| **Depends on** | [T-03](03-input-keymaps-shortcuts.md) · [T-05](05-spaces-model.md) · [T-07](07-private-shell-protocols.md) · [T-08](08-design-system.md) · [T-09](09-menu-bar.md) |
| **Blocks** | Phase-2 exit (zero-dropped-frame loop) · [T-12](12-app-switcher.md) (cross-Space activation) · [T-14](14-hot-corners-desktop-background.md) (Desktop Reveal shares the pipeline) · [T-31](31-polish-hardening.md) |
| **Estimate** | XL |
| **Design docs** | [03-workspaces.md](../design/03-workspaces.md) · [10-design-system.md](../design/10-design-system.md) · [13-roadmap.md](../design/13-roadmap.md) |

## Summary

The single overview state machine and its animations: Mission Control
transforming **live window surfaces** (never thumbnails), gesture-progress
commit rules (one rule for every trigger), explicit hit-testing transfer, the
workspace strip, minimized-window bottom strip, and the workspace-switch
animation as a continuous reversible progress pipeline.

## Background

Mission Control does not screen-scrape: the compositor keeps rendering the
real surfaces while applying scale, translation, clipping, blur/shadow, and
workspace transformations ([03-workspaces.md](../design/03-workspaces.md)).
This is the flagship "because we own the compositor" feature and the core of
the 30-second interaction loop
([13-roadmap.md](../design/13-roadmap.md)).

## Scope

### In scope

1. **The transition** (from
   [03-workspaces.md](../design/03-workspaces.md)):
   ```text
   normal scene
       │ gesture progress 0 → 1
       ▼
   shrink visible workspace
       ├── reposition live window surfaces
       ├── reveal neighboring workspaces
       ├── display workspace strip
       └── transfer hit-testing to overview controller
   ```
   And selection: `overview → activate workspace → raise/focus window →
   reverse animation`.
2. **One state machine**: gesture, keyboard, hot corner, and the Mission
   Control button all drive the **same** progress pipeline; there is no
   second, discrete "instant" code path.
3. **Commit rules**:
   - Progress clamped 0→1; **rubber-banding** when swiping past the first
     or last Space.
   - Release commits if **progress or release velocity** crosses a
     threshold; otherwise the transition animates back.
   - One rule for swipes, pinches, and hot corners.
4. **Workspace switching** (adjacent Space slide) as the same progress
   pipeline: live surfaces + per-Space wallpaper repositioned with
   scale/translation/blur (T-05 scene mechanics), reversible at any progress.
5. **Overview chrome** (shell, via private protocol):
   - Workspace strip (renders Spaces — including the dedicated Spaces of
     fullscreen windows while they exist, [T-05](05-spaces-model.md) FR-3;
     never keeps a second copy of workspace state).
   - Minimized-windows bottom strip, restorable by click.
   - Window selection → activate; window dragging between Spaces in the
     overview ("windows move between Spaces by dragging in the overview").
6. **Hit-testing transfer**: while the overview is active its controller
   owns input; on reversal, ownership returns to the normal focus path —
   explicitly, never implicitly.
7. **Reduced-motion variants** for every transition (design-system rule).
8. **Performance work**: this feature owns the "60 Hz, no dropped frames
   for the full gesture" budget — scale/blur passes must fit the frame
   budget on baseline Intel/AMD.

### Out of scope

- Spaces *model* (T-05 — events and lists already exist).
- App switcher overlay (T-12 — different feature, compositor-driven too).
- Desktop Reveal (T-14, a window-aside transition sharing the pipeline).

## Requirements

- FR-1: One overview state machine; a test drives it via gesture, keyboard
  shortcut, hot corner, and menu-bar button and asserts identical
  progress/commit behavior.
- FR-2: Real textures are transformed throughout — a playing video keeps
  playing (at reduced scale) inside the overview; no thumbnails are ever
  substituted.
- FR-3: Commit thresholds: implement the progress-or-velocity rule; the
  rubber-band constants are tunable, not hardcoded magic.
- FR-4: Interruptibility: mid-transition input reverses direction without
  waiting for the animation to finish.
- FR-5: Window selection round-trip: overview → workspace activates →
  window raises/focuses → reverse animation → normal scene with focus.
- FR-6: Minimized windows render in the bottom strip, excluded from Space
  layout, clickable to restore.
- FR-7: Dragging a window's overview representation onto another Space
  moves it there (with T-05 window-assigned events).
- FR-8: 60 Hz with zero dropped frames for the full gesture on baseline
  hardware (measured, in the dev loop, not the polish phase).
- FR-9: Reduced-motion variant passes (Phase-2 exit criterion).
- FR-10: Fullscreen windows appear as their own Space in the workspace
  strip while fullscreen and leave it on unfullscreen (T-05 FR-3).

## Acceptance criteria

- [ ] The transition diagram above is implemented and visual-frame
      reviewed in nested mode.
- [ ] Trigger-parity test passes (FR-1).
- [ ] Frame-time trace during full gesture stays within budget on
      baseline Intel/AMD.
- [ ] Video-in-overview test (FR-2) passes.
- [ ] Phase-2 exit: the 30-second loop including Mission Control at zero
      dropped frames.

## Test plan

- Gesture math unit tests shared with T-03 (clamp, velocity, rubber-band).
- Headless: state-machine transitions, hit-test transfer.
- Nested + hardware: frame-time instrumentation during gestures; multi-
  monitor lockstep during overview.

## Risks / open questions

- Blur + scale + many windows on iGPU is the frame-budget risk; fall back
  to cheaper transforms (e.g. scale-only) under budget pressure — decide
  threshold and keep it measurable.
- Overview window occlusion/layout algorithm (which window peeks at what
  position) needs a spec: grid vs. macOS-style cascade; pick, document,
  iterate.
