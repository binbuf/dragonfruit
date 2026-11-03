# Track 21 — Presentation foundation (v0.1.1)

| | |
|---|---|
| **Status** | proposed |
| **Date** | 2026-11-03 |
| **Release** | v0.1.1 |
| **Scope** | shell chrome transport, HiDPI, refresh-aware scheduling, material cost/floor, native verification, presentation benchmarks |
| **Related** | [`docs/rendering.md`](../../rendering.md), [`architecture-audit-native-rendering.md`](../architecture-audit-native-rendering.md), [`STATUS.md`](../STATUS.md), [`02-compositor.md`](../02-compositor.md), [`04-shell.md`](../04-shell.md), ADR [0189](../adr/0189-drm-sampled-material-default-and-fallbacks.md) |

## Why this track

The architecture review reached the same conclusion as the native-rendering
audit: **keep Rust + Smithay + Qt Quick, but stop adding UI surface area until
the presentation path is fixed.** The two P0 gaps are not renderer choices:

1. The shell is a Qt Quick process on the offscreen QPA. Every Dock/menu/
   popover frame is rendered, read back with `grabWindow()`, deep-copied into a
   fresh `wl_shm` buffer, uploaded by the compositor, and committed without a
   buffer scale or a frame callback. That is CPU and memory-bandwidth work on
   the critical path, and it is soft on fractional-scale outputs.
2. Animation and degrade scheduling still assume one 60 Hz budget. A 144 Hz
   panel gets a 16 ms budget instead of a 6.94 ms deadline, and there is no
   per-output or mixed-refresh scheduling model.

The compositor-owned effects architecture is the right one: live surfaces,
GPU scene textures, Kawase blur, liquid-glass compose, and one material pass per
output-frame. This track optimizes that path; it does **not** replace Smithay,
Qt Quick, or the renderer with Skia/Impeller/Flutter.

### Renderer decision

Qt Quick's RHI scene graph is already GPU-accelerated, and Smithay owns the
compositor effects. The measurable gap is the shell transport and the fixed
refresh model, not the renderer. Skia/Impeller/Flutter are therefore out of
scope: switching renderers without the T183/T192 evidence would add risk
without addressing the two P0 gaps.

## Shape

```text
T-21.1  measure → HiDPI → pooled SHM/frame pacing → GPU-backed chrome
T-21.2  per-output refresh model → presentation-paced mixed refresh
        → persistent-chrome material floor
T-21.3  panel-region material + stable IDs → direct-scanout preservation
T-21.4  native material acceptance → presentation benchmark + soak
T-21.5  regression matrix, docs, and release close-out
```

Every measurement task records p50/p95/p99, missed presentations, CPU copies,
GPU render time where available, and the exact hardware/driver. Averages alone
are not acceptance evidence. Nested screenshots are reference evidence only;
they are never labeled native proof.

## Tasks

The executable task files live in
[`docs/tasks/0.1.1/`](../../tasks/0.1.1/README.md).

| Task | Unit | Depends on | Hardware |
|---|---|---|---|
| T183 | T-21.1a Chrome transport baseline and decision | — | optional |
| T184 | T-21.1b Chrome HiDPI and fractional scale | T183 | no |
| T185 | T-21.1c Chrome SHM transport hygiene and frame pacing | T183, T184 | no |
| T186 | T-21.1d GPU-backed chrome transport | T185 | optional |
| T187 | T-21.2a Per-output refresh model and animation clock | T183 | no |
| T188 | T-21.2b Presentation-paced mixed-refresh scheduling | T185, T187 | yes for mixed-refresh |
| T189 | T-21.2c Persistent-chrome material floor and degrade | T187 | no |
| T190 | T-21.3 Panel-region material and direct-scanout preservation | T189 | optional |
| T191 | T-21.4a Native DRM material acceptance | T184–T190 | yes |
| T192 | T-21.4b Presentation benchmark and soak | T183–T191 | yes |
| T193 | T-21.5 Regression matrix and v0.1.1 close-out | all | optional |

## Acceptance

- [ ] Dock/menu-bar text, icons, and rims are native-resolution at 1x, 1.5x,
      and 2x; logical layout and hit-testing are unchanged.
- [ ] The fallback SHM path performs at most one full-surface CPU copy per
      committed frame and zero steady-state buffer allocations; the GPU path
      performs zero CPU readbacks on a supported driver.
- [ ] Animations are scheduled against the real per-output period (60/120/144/
      240 Hz), with p95/p99 and missed-deadline counts; a 60 Hz output does not
      throttle a 144 Hz output.
- [ ] The Dock/menu bar keep a deliberate material under load; `Minimal` is a
      token-driven milky fallback, not a silent flat slab.
- [ ] Native DRM evidence exists for the material and HiDPI at 1x/1.5x/2x,
      Full/Reduced/Minimal, light/dark, and 1080p/4K.
- [ ] A benchmark report records Dock-sweep p95/p99, missed presentations,
      CPU-to-GPU copies, GPU render time, idle wakeups, and a 30–60 minute soak.
- [ ] `make e2e`, `make check`, and `make soak` stay green; no hidden deferral.

## Non-goals

- Replacing Smithay, Qt Quick, or the compositor-owned effects model.
- Skia/Impeller/Flutter experiments or a second UI renderer.
- Multi-GPU cross-device sampling, rotated-output material, VRR/night-light/
  color-management **feature breadth**, and Files/compatibility work. T188
  consumes a VRR state only if the backend already exposes one; it does not
  add a VRR UI or policy. Existing fallbacks stay in place and are documented;
  they are not silently claimed fixed.
- New user-facing chrome features. This track is a rendering and scheduling
  foundation release.

## Rollback

- Shell transport: keep the pre-T-21 `grabWindow`/SHM path selectable until the
  new path is accepted; revert the transport without touching the private
  layer protocol.
- Material: `DRAGONFRUIT_DRM_MATERIAL=0` and `DRAGONFRUIT_DRM_GLASS=0` remain
  the documented rollback flags.
- Scheduling: keep the 60 Hz path until the per-output path is proven; never
  replace 16 ms with a global 8 ms.
- Native session: `make native-recover` is the first recovery step from a TTY.

## References

- [Rendering — remaining work](../../rendering.md) — R0–R8 work queue.
- [Architecture audit — native DRM presentation, material, and shell transport](../architecture-audit-native-rendering.md) — §5 shell transport, §9.4 60 Hz constants, §11 acceptance sketches.
- [STATUS](../STATUS.md) — G-01 (transport), G-02 (scheduling), G-03 (native evidence), G-08 (multi-GPU/rotation), G-12 (quality floor).
- [ADR 0189](../adr/0189-drm-sampled-material-default-and-fallbacks.md) — native material default and fallback boundaries.
