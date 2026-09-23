# T-33 — Compositor Effects and Materials (Visual Fidelity)

| | |
|---|---|
| **Phase** | 1 · Foundation (split from [T-02](02-compositor-core.md)) |
| **Area** | `compositor/` (render passes) + `design-system/` (token consumption) |
| **Depends on** | [T-02](02-compositor-core.md) · [T-08](08-design-system.md) |
| **Blocks** | [T-13](13-window-decorations-ssd.md) · [T-11](11-mission-control-workspace-ux.md) · [T-35](35-window-lifecycle-animations.md) · [T-09](09-menu-bar.md)/[T-10](10-dock.md) visual quality · [T-34](34-mvp-vertical-slice-gate.md) · Phase-2 exit |
| **Estimate** | L |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [10-design-system.md](../design/10-design-system.md) · [05-window-decorations.md](../design/05-window-decorations.md) |

## Summary

The compositor-owned real-time effects that make chrome and windows read as
**native materials**: backdrop blur/translucency sampled from live surface
buffers, rounded-corner clipping, real soft shadows, and the scale/clip/blur
scene transforms that Mission Control and workspace switching reuse. This is
the split-out visual-fidelity half of T-02 and the direct fix for the current
"functional but looks bad" state.

## Background

The visual language is specified but not rendered. `material.chromeBlur` /
`popupBlur` / `shadowOpacity` and the elevation tokens exist in
`design-system/tokens/tokens.json`, and the design docs are explicit that
effects are a **compositor render pass over live surface buffers**:

> Effects (blur, shadows, workspace scale/clip transforms) are compositor
> render passes over **live surface buffers** — never client re-renders,
> never screenshots, never third-party recompositing.
> ([02-compositor.md](../design/02-compositor.md))

> Client-provided opaque, translucent, and input regions are honored: input
> outside the input region falls through, and translucent regions
> participate in the blur pass rather than fighting it.
> ([02-compositor.md](../design/02-compositor.md))

Today `compositor/src/render.rs` only builds plain surface elements — no
blur, shadow, or corner pass — and the design system ships a layered
`Rectangle` approximation (`Shadow.qml`) that explicitly defers real blurred
shadows to the compositor. That approximation is why the Dock, menu bar, and
popovers look flat.

## Scope

### In scope

1. **Backdrop blur pass** for chrome surfaces (menu bar, Dock, popovers,
   and later notifications/OSD): sample the scene beneath the surface's
   translucent regions and blur it, honoring client translucent regions
   rather than fighting them.
2. **Real shadows + rounded-corner clip** for chrome, SSD titlebars (T-13),
   and floating windows, driven by the shared elevation/radius tokens so
   compositor-drawn SSD and QML `TitleBar` cannot drift.
3. **One reusable scene-transform pass** (scale/translate/clip + optional
   blur) that the workspace-switch and Mission Control pipelines (T-11)
   compose, so effects are built once instead of per feature.
4. **Token-driven and budget-disciplined**: radii, opacities, and blur radii
   come from the generated `design_tokens.rs`; one effect pass per rendered
   frame (T-02 FR-4); degrade (smaller radius → blur off) under frame-budget
   pressure; the headless/software path keeps a cheap approximation for CI.
5. **Reduced-power / reduced-motion fallback** that keeps legibility and
   never causes idle wakeups (T-02 FR-2).

### Out of scope

- Motion timing and transitions themselves ([T-11](11-mission-control-workspace-ux.md)).
- `xdg-decoration` negotiation and traffic-light behavior ([T-13](13-window-decorations-ssd.md)).
- Night light / color management (T-02/T-16) and a11y magnification (T-31).
- Notifications/OSD surfaces ([T-25](25-notifications-and-osd.md)) — only the
  shared material they will consume.

## Requirements

- **FR-1**: Backdrop blur renders **live** content — a playing video under a
  translucent menu bar keeps updating at 60 Hz, never a stale sampled frame.
- **FR-2**: Shadows and rounded corners are token-driven and visually
  identical between compositor SSD and the QML `TitleBar` (feeds T-13 FR-2).
- **FR-3**: Client translucent regions participate in the blur pass
  ([02-compositor.md](../design/02-compositor.md)); input-region semantics are
  unchanged.
- **FR-4**: One effect pass per frame; the overview transform + blur fits the
  frame budget with several live windows on baseline Intel/AMD.
- **FR-5**: A reduced/low-power mode disables blur while keeping state
  legible; no effect wakes the shell or the compositor while idle.
- **FR-6**: No screenshots, no client re-renders, no third-party
  recompositing ([02-compositor.md](../design/02-compositor.md)); the
  no-screencopy gate (T-02 FR-8) still passes.

## Acceptance criteria

- [ ] Nested side-by-side capture of menu bar / Dock / a window matched
      against the design-system reference; blur visibly samples live content
      (video-in-the-background test).
- [ ] Frame-time trace: workspace-overview transform + blur within budget on
      baseline Intel/AMD.
- [ ] The `Shadow.qml` layered approximation is no longer the rendering path
      for chrome; compositor shadows/rounding are the source of truth.
- [ ] Headless visual-regression suite still passes (cheap approximation
      path), and dark/light + reduced-motion variants verified.

## Test plan

- Headless: render-pass ordering and damage-accounting unit tests; an
  effect-on/effect-off image assertion in the nested capture harness.
- Nested: live-content blur capture, light/dark, reduced-motion.
- Real hardware: frame-time trace during the overview transform (shared
  instrumentation with T-11) and during idle (zero-damage assertion).

## Risks / open questions

- Blur + scale on an iGPU is the frame-budget risk
  ([14-risks.md](../design/14-risks.md)); define quality tiers (radius,
  downsample) and a measurable degrade threshold.
- GL vs Qt scene-graph rounding/antialiasing may break literal pixel-diff
  (T-13 risk); diff with tolerance and keep identity via shared tokens + the
  spec if so — document the decision.
- Decide the blur implementation with T-02's renderer (Kawase vs dual-pass
  Gaussian) before committing to token ranges; keep it swappable behind the
  token values.