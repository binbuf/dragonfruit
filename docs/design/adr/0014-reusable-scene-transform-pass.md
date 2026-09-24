# 0014 — One reusable scene-transform pass for live surfaces

## Status

accepted

## Context

T-04.3 must give T-05's Mission Control live-surface grid and T-06's app
switcher a single way to draw the **real** window surfaces scaled, translated,
clipped to their rounded corners, and (optionally) over the token-driven
backdrop material — never thumbnails, never client re-renders. The T-04 track
explicitly forbids per-feature transform variants, and T-04.2 already shipped
the backdrop blur as a token-driven `BackdropPass` with a one-pass-per-frame
guard. The window lifecycle motion (T-02) already draws a scale/translate
through `MotionFrame`; T-04.1b shipped the token-derived `CornerMask`; T-04.1a
shipped the elevation/material tokens.

The compositor renderer is still a flat solid-color renderer with no shader or
texture sampler, so a "pass" here is pure geometry plus rectangle materials,
asserted headless. The open questions were: what the transform *is*, who owns
the once-per-frame guard, and how later features compose it without forking it.

## Decision

- Add `compositor/src/window/scene_transform.rs` as the **one** transform.
  `SceneTransform` maps a `source` rectangle onto a `target` rectangle
  (scale/translate), with an optional token-derived `CornerMask` clip (ADR
  0012) and an optional token-driven `BackdropSpec` blur (ADR 0013). It exposes
  `scale`/`offset`/`map_rect`/`map_point`/`clip_spans`/`blur_layers`/
  `blur_elements`; all geometry is logical and renderer-free.
- `SceneTransform::from_motion` derives the transform from a lifecycle
  `MotionFrame` and the committed surface size, using `MotionFrame::scale_for`
  (not the target-relative chrome scale). The T-02 appear/minimize/zoom render
  path now draws through this transform, so the lifecycle motion and the T-05
  grid share one mapping and one code path.
- `SceneTransformPass` is the per-frame guard, built on the shared
  `compositor/src/window/pass.rs::FramePass`. `DfState::begin_render_frame`
  opens it once per rendered frame; an output composes at most one transform
  and a duplicate is counted as `scene_transform_skipped`. `BackdropPass` is
  re-expressed on the same `FramePass`, so there is one guard implementation,
  not one per effect.
- The pass is **instrumented** on the render-stats line
  (`scene_transforms` / `scene_transform_skipped`, appended after the T-03.1b
  latency fields per ADR 0009 append-only), and the idle trace asserts it stays
  flat while idle. The pass never forces a redraw.
- Clipping a third-party client surface's own pixels stays deferred to the
  renderer: the transform carries the `CornerMask` and T-05 composes the
  actual element wrap. This pass supplies the geometry; it does not duplicate
  Smithay's surface elements or their presentation ids.

## Consequences

- After this slice there is exactly one scale/translate/clip/blur model;
  T-05/T-06 compose `SceneTransform` instead of growing their own, and
  T-04.4a degrades the same pass (smaller radius, blur off) rather than a new
  one.
- The T-02 lifecycle motion now renders through the reusable pass, so the
  append-only trace line reports `scene_transforms` for ordinary window
  motion, and the existing idle/animation trace stays the budget guardrail.
- The transform is asserted headless (mapping, `from_motion` equivalence with
  `MotionFrame`, clip spans, token blur, the pass guard); no GPU or pixel diff
  is needed. `make e2e` stays green.
- Blur remains the T-04.2 flat-tone feather approximation behind the tokens;
  folding the chrome backdrop's element generation into this pass is a later
  refinement (`BackdropPass`/`MaterialRole` are the seam), as is applying the
  clip to live surface elements.