# T-04 — Loop v2: Materials (Blur, Shadows, Rounded Corners)

> **Track, not a single slice.** This file is the design reference. It is executed as 4 one-session units: [T-04.1](units/015-t-04.1-real-shadows-and-rounded-corner-clipping.md) · [T-04.2](units/016-t-04.2-backdrop-blur-pass.md) · [T-04.3](units/017-t-04.3-reusable-scene-transform-pass.md) · [T-04.4](units/018-t-04.4-material-degrade-tiers-schemes-sign-off-package.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 4 of 17 — the visual floor |
| **Area** | `compositor/` (render passes) · `design-system/` (token consumption) |
| **Depends on** | T-01, T-02 |
| **Blocks** | T-05 (scene transform), T-17 (visual floor) |
| **Legacy detail** | [legacy/33-compositor-effects-materials.md](legacy/33-compositor-effects-materials.md) · [02-compositor.md](../design/02-compositor.md) · [10-design-system.md](../design/10-design-system.md) |

## Demo

The loop with native materials instead of flat rectangles:

```
menu bar, Dock, popovers, and SSD titlebars show backdrop blur and
translucency sampled from the live scene underneath; windows and chrome have
real soft shadows and rounded corners; light and dark schemes both read
correctly
```

Capture: `docs/captures/t04-materials.*` (light + dark, before/after, and a
side-by-side against the design-system reference/gallery goldens).

## Why now

Materials are the affordance layer for everything after this slice: T-05's
Mission Control scale/clip/blur composes the reusable scene transform built
here, and the "premium" judgment on every later slice depends on the visual
floor existing. They are also the direct fix for the current "functional but
looks bad" state.

## Inherited and reused

- `material.chromeBlur` / `popupBlur` / `shadowOpacity` and elevation tokens in
  `design-system/tokens/tokens.json`; generated `design_tokens.rs`.
- `Shadow.qml`'s layered-rectangle approximation (to be superseded for chrome;
  it remains the QML fallback).
- Existing render element enum pattern (`render_elements!`, per-backend
  instantiation) from the legacy T-02 work.
- `FrameCommitGate` / damage-driven render loop.

## Scope

### In

1. **Backdrop blur pass** for chrome surfaces (menu bar, Dock, popovers, OSD
   later): sample the scene beneath the surface's translucent regions and
   blur it, honoring client translucent regions rather than fighting them.
2. **Real shadows + rounded-corner clipping** for chrome, SSD titlebars, and
   floating windows, driven by the shared elevation/radius tokens so
   compositor-drawn SSD and QML `TitleBar` cannot drift.
3. **One reusable scene-transform pass** (scale/translate/clip + optional
   blur) that T-05's workspace-switch/Mission Control pipeline composes — built
   once here, not per feature.
4. **Token-driven and budget-disciplined**: radii/opacities/blur radii from
   `design_tokens.rs`; one effect pass per rendered frame; degrade tiers
   (smaller radius → blur off) under budget pressure, instrumented.
5. **Light/dark + reduced-motion** variants; no new animation is introduced,
   but T-02's transitions must render correctly through the material pass.

### Out / explicitly deferred

- Per-surface dmabuf feedback tranches for zero-copy capture (T-13).
- HDR/color-management staging (post-gate backlog).
- Genie minimize (post-gate).

## Acceptance

- [ ] The demo runs and both captures are committed.
- [ ] A side-by-side against the design-system reference is reviewed; the
      visual floor is explicitly signed off.
- [ ] One effect pass per frame is asserted (no double-blur) and the idle
      trace still shows zero damage.
- [ ] The degrade tier is exercised (force blur off, confirm the UI still
      reads correctly).
- [ ] `make e2e` and `make soak` stay green; T-01/T-02 demos still pass.

## Test plan

- Headless: token/geometry unit tests; damage-rect assertion for the chrome
  band; a forced-degrade case.
- Nested: capture review against gallery goldens; light/dark/reduced-motion.
- Performance: frame trace with blur on, compared against the T-03 budget
  numbers.

## Risks

- **Frame budget** on iGPU is the named risk; keep the degrade tier measurable
  and do not ship a blur that cannot be turned down.
- **Token drift** between compositor-drawn SSD and QML `TitleBar`; the shared
  token source is the guardrail, and the existing
  `test_fr3_titlebar_matches_ssd_reference` must stay green.
- **Software renderer**: Qt shader effects no-op under
  `QT_QUICK_BACKEND=software`; the compositor pass is the real path, the QML
  approximation stays only as a fallback.

## Hand-off

- T-05 consumes the scene-transform pass for the live-surface grid and
  wallpaper slide.
- T-11 consumes the blur pass for Control Center and OSD.
- T-17 gates on the visual floor.
