# T-05 — Loop v3: Mission Control on Live Surfaces

> **Track, not a single slice.** This file is the design reference. It is executed as 6 one-session units: [T-05.1](units/019-t-05.1-live-surface-transform-into-the-overview-grid.md) · [T-05.2](units/020-t-05.2-hit-testing-and-selection-on-live-representations.md) · [T-05.3](units/021-t-05.3-drag-a-live-representation-between-spaces.md) · [T-05.4](units/022-t-05.4-image-wallpaper-and-per-space-slide.md) · [T-05.5](units/023-t-05.5-desktop-reveal.md) · [T-05.6](units/024-t-05.6-overview-frame-budget-and-capture.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 5 of 17 — the flagship "we own the compositor" moment |
| **Area** | `compositor/` (overview scene transform, wallpaper) · `shell/` (overview chrome) |
| **Depends on** | T-04 |
| **Blocks** | T-06 |
| **Legacy detail** | [legacy/11-mission-control-workspace-ux.md](legacy/11-mission-control-workspace-ux.md) · [legacy/05-spaces-model.md](legacy/05-spaces-model.md) · [legacy/14-hot-corners-desktop-background.md](legacy/14-hot-corners-desktop-background.md) · [03-workspaces.md](../design/03-workspaces.md) |

## Demo

```
Ctrl+Up (or a four-finger swipe / hot corner) → the live Space shrinks and
neighbors slide in, with the real window surfaces scaled — a playing video
keeps playing at reduced scale
→ click a window → its Space activates, the window focuses, the scene
reverses
→ drag a window's live representation onto another Space → it moves there
→ Ctrl+Left/Right → a live slide with the per-Space wallpaper moving with it
→ Ctrl+Down → Desktop Reveal
```

Capture: `docs/captures/t05-mission-control-live.*`, plus a reduced-motion
capture.

## Why now

The overview state machine, chrome, and hit-test transfer already exist; the
one thing missing is the thing that makes it Mission Control rather than a
title-card list: **transforming the real surfaces**. That transform is exactly
the reusable pass T-04 just built. This slice is therefore mostly composition
work, and it closes the last structural gap in the 30-second loop before the
app switcher.

## Inherited and reused

- One overview state machine: trigger parity, commit rules, rubber-band,
  interruptibility, reduced motion, hit-test transfer, selection round-trip
  (`legacy/11-mission-control-workspace-ux.md`, slices 1–5).
- Shell overview chrome: workspace strip (incl. fullscreen Spaces), minimized
  strip, draggable window grid, reduced-motion variant.
- `apply_overview_scene` translation pipeline and the animation clock.
- Per-Space wallpaper data and the compositor-rendered clear color.
- T-04's scene-transform pass (scale/clip/blur).

## Scope

### In

1. **Live-surface transform (legacy B-1/B-3/B-4/B-9)**: scale/clip/blur the
   real window surfaces into the documented grid layout
   ([03-workspaces.md](../design/03-workspaces.md#window-layout-and-occlusion-t-11-u-5));
   no thumbnails, ever.
2. **Hit-testing and selection on live representations** (B-3): click a live
   window in the overview → the existing selection round-trip.
3. **Drag a live representation between Spaces** (B-4): reuses
   `move_to_workspace`; the shell grid remains the keyboard/accessible path.
4. **Image wallpaper + per-Space slide** (legacy T-05 open item, B-2): sample
   `source`/`fit`, render per Space, and move it with the Space during a
   switch. No image configured → current color behavior.
5. **Desktop Reveal** (legacy T-14) through the same pipeline.
6. **Multi-monitor lockstep** in the overview (model exists; verify on
   hardware in T-03/T-16).
7. **Frame budget**: 60 Hz, zero dropped frames for the full gesture, measured
   with the T-03 instrumentation; degrade tiers from T-04 apply.

### Out / explicitly deferred

- Per-output chrome sizing (T-16).
- Multi-monitor transition polish (T-16).
- Overview on DRM beyond the T-03 smoke (T-16).

## Acceptance

- [ ] The demo runs and both captures are committed.
- [ ] A "video" client (a surface committing fresh buffers) stays mapped,
      keeps receiving frame callbacks, and is visibly scaled inside the
      overview.
- [ ] Live-window click selection and drag-between-Spaces work by pointer.
- [ ] Wallpaper image moves with its Space; no image configured still works.
- [ ] The frame trace for the full gesture stays within budget on the
      available hardware, or the shortfall is recorded honestly.
- [ ] `make e2e` and `make soak` stay green; T-01…T-04 demos still pass.

## Test plan

- Headless: extend the existing live-surface frame-callback conformance test
  to assert the transform state; layout unit tests for the grid; wallpaper
  sampling unit tests.
- Nested: capture review (open, select, drag, wallpaper slide, reveal,
  reduced motion).
- Performance: gesture frame trace on nested and (when available) DRM.

## Risks

- **Blur + scale + many windows** on iGPU is the frame-budget risk; the T-04
  degrade tier is the mitigation.
- **Grid vs. cascade** is decided (grid); do not relitigate mid-slice.
- **Wallpaper memory**: large images per Space must be cached and scaled
  without per-frame decode.

## Hand-off

- T-06 builds the app switcher on the same transform pass and recency data.
- T-16 owns multi-monitor sizing and scaling edge cases.
