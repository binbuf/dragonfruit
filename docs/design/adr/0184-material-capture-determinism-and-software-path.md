# 0184 — Material captures are tolerance artifacts; the software path is Minimal

## Status

accepted

## Context

[ADR 0182](0182-tahoe-liquid-glass-material-pass.md) moved the chrome material
onto a GPU-sampled backdrop pass and left three hardening questions to T-20.4:
how the pass behaves under `renderer_multi`, what the software/headless path
renders, and how its captures can be verified in CI. The pass runs custom GLES
shaders whose blur/refraction arithmetic is **driver-dependent**: the same scene
composes to slightly different pixels on different GPUs. Byte-for-byte golden
comparison of the GPU material would fail for reasons that are not regressions.

At the same time, the project still has a software/headless backend for CI
(`compositor/src/backend/headless.rs`), which has no renderer and no scene
texture. It must stay deterministic and cheap.

## Decision

- **The scene texture is routed per GPU, never sampled across GPUs.** Under
  `renderer_multi` an output whose GPU owns the scene texture composes the blur
  locally; an output on another GPU copies the scene texture onto its own GPU
  first. The decision is the pure `multi_gpu::scene_route` /
  `plan_scene_routes` model, so it is asserted without two cards. A panel that
  straddles outputs takes the copy fallback rather than sampling across devices.
- **The software/headless path is `Minimal`.** `MaterialPath::Software` resolves
  every selected `DegradeTier` to `Minimal` (blur off, the deterministic feather
  stack) and never takes the GPU blur pass. It is a *quality* fallback, not a
  correctness one: it uses the same `MaterialRole`/`BackdropSpec` inputs.
- **Material captures are tolerance artifacts, not byte goldens.** A capture is
  reproducible when a re-run over the same **static fixture backdrop** agrees
  within a tolerance (`scripts/t20-material-compare.py`: mean ≤ 8/255, and
  < 0.5 % of pixels may exceed 64/255). The fixtures pin the bar/card content so
  the structure is identical; the tolerance absorbs the driver's blur
  arithmetic, and the outlier fraction absorbs unrelated pixels (the shell
  clock, a cursor, text anti-aliasing), not the material. Headless stays off the
  GPU path entirely, so anything headless still compares exactly.
- **The pass is instrumented.** The `material stats` line carries
  `path=`, `blur_passes`, `blur_panels`, and `blur_downsample_max`;
  `backdrop_skipped` must stay `0` and the counters flat while idle. The
  degrade ladder keeps its per-tier frame counts (`tiers=full:..`), so a trace
  shows it reacting under load.

## Consequences

- The recommended GPU baseline (GLES 3.x / Vulkan-capable) is documented in
  `02-compositor.md` and the packaging doc; the host-iGPU trace is recorded
  honestly and the baseline-machine run is **OPEN** on the T-16 hardware rail.
- A future GPU-material regression is caught by the tolerance comparator plus
  the structural counters, not by a byte diff; the design-system gallery goldens
  (deterministic QML, no GL blur) keep their exact comparison.
- The DRM rail keeps the feather stack until a follow-up folds the two-pass
  composition into `UdevRenderer`/`DrmCompositor`; the `Multi-GPU: SCENE …`
  marker already reports the routing plan it will consume.
- No new visual feature is introduced; T-20.4 only hardens and measures what
  T-20.1–T-20.3 built.