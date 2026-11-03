# 0189 — Native DRM sampled material is default; its fallback boundaries are explicit

## Status

accepted

## Context

T-20 landed the two-pass GPU material (offscreen scene texture, Kawase blur,
liquid-glass shader) on the nested backend. The native DRM rail still drew the
pre-T-20 feather backdrop. The 2026-11-02 audit
([`../architecture-audit-native-rendering.md`](../architecture-audit-native-rendering.md))
identified the missing `chrome_blur_panels` call as the root cause, corrected
the renderer API assumptions for Smithay 0.7, and proposed P0 instrumentation,
P1 effective-material state + milky fallback, and P2 the DRM sampled path.

The implementation then made several decisions that were not recorded in an
ADR: DRM uses the sampled path by default, only on a single GPU and a normal
output transform; a failed material frame disables the path for the session;
and the shell is told the effective material over a new event so QML can switch
to its no-blur complement.

## Decision

- **DRM uses the sampled material by default** when the effective material says
  blur is active. The pass is the same two-stage pipeline as nested: render the
  live scene into an output-sized offscreen texture, build one Kawase-blurred
  texture per panel, optionally run the glass program, then composite the scene
  below the blur and the chrome above it. The feather stack remains the
  fallback.
- **The first supported boundary is one GPU and `Transform::Normal`.** A
  multi-GPU seat disables the sampled path at startup and reports the reason;
  a rotated output uses the feather stack for that output. Cross-GPU scene
  sampling is never attempted. The `multi_gpu::plan_scene_routes` model remains
  the routing plan, not a completed copy implementation.
- **Failure is session-scoped, not per-frame panic.** A failed scene/blur frame
  sets `material_blur_supported = false`, reports the failure, and leaves the
  rail on the feather path. The next session may try again.
- **The shell learns the effective material from the compositor.** Additive
  `df_output.material_state` (v3) carries tier, `blur_active`, and a fallback
  reason. QML uses that to choose the translucent-glass complement or the
  milky fallback; it must not re-derive the degrade policy. The event is
  emitted on change, not per frame.
- **Diagnostics are opt-outs, not the default.** `DRAGONFRUIT_DRM_MATERIAL=0`
  and `DRAGONFRUIT_DRM_GLASS=0` disable the sampled path/glass for rollback and
  diagnostics. They do not change the shipped default.
- **Counters distinguish "resolved" from "drawn".** `blur_passes` means the
  panels were resolved and the material pass was claimed; `blur_draws` means
  the blur elements were actually built and handed to the compositor. Native
  acceptance requires `blur_draws > 0`, not `blur_passes > 0` alone.

## Consequences

- The native Dock/menu bar/overlays get the same material as nested on the
  supported path; hybrid laptops and rotated displays degrade to the feather
  stack with an honest reason instead of a broken glass sample.
- The renderer remains single-threaded and output-local; the scene texture is
  per output/GPU and is reused across frames at a fixed mode/scale.
- Multi-GPU composition, transform-aware sampling, panel-region-only rendering,
  and the persistent-chrome quality floor are explicit follow-ups (see
  [`../STATUS.md`](../STATUS.md) G-08/G-12).
- Native hardware acceptance is still required before the material is called
  verified on DRM; nested captures are not native proof (G-03).
- The effective-material event is an additive protocol change; older shells
  keep their conservative default and the compositor does not require an ack.

## References

- [`../architecture-audit-native-rendering.md`](../architecture-audit-native-rendering.md)
  — the implementation audit and the P0/P1/P2 record.
- [`0182-tahoe-liquid-glass-material-pass.md`](0182-tahoe-liquid-glass-material-pass.md)
  — the material decision.
- [`0183-chrome-material-rollout.md`](0183-chrome-material-rollout.md)
  — the chrome role rollout.
- [`0184-material-capture-determinism-and-software-path.md`](0184-material-capture-determinism-and-software-path.md)
  — the software/headless fallback and tolerance captures.
- `compositor/src/backend/drm.rs`, `compositor/src/window/blur.rs`,
  `compositor/src/render.rs`, `compositor/src/shell/mod.rs`,
  `protocols/dragonfruit-toplevel.xml`, `shell/src/shellprotocol.*`.
