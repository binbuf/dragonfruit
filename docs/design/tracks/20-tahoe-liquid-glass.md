# Track 20 — Tahoe liquid-glass material (GPU backdrop pass)

This track replaces the flat feather-layer stand-in for the chrome material
with a **real, GPU-sampled backdrop blur plus the Tahoe liquid-glass pass**.
The reference is macOS **Tahoe** ([ADR 0122](../adr/0122-tahoe-interface-language-across-chrome.md)),
the material contract is [ADR 0013](../adr/0013-backdrop-blur-pass.md) and its
degrade ladder ([ADR 0015](../adr/0015-material-degrade-tiers.md)), and the
decision to move it onto the GPU is [ADR 0182](../adr/0182-tahoe-liquid-glass-material-pass.md).

## Why now

ADRs 0013/0091 shipped the token-driven approximation and explicitly deferred
the texture-sampling blur and refraction. The approximation cannot sample the
live scene (video/text under the menu bar do not blur) and cannot express
Liquid Glass (refraction, specular rim, adaptive tint). The project now assumes
a GPU for the material, and the renderer primitives exist: the nested backend
already renders the scene to an offscreen `GlesTexture`, and Smithay 0.7 exposes
custom texture shaders and a texture render element.

## Shape

```
scene → offscreen scene texture ─┬─▶ downsample + Kawase blur (per panel) ─▶ liquid-glass pass ─┐
                                 │                                                              ▼
                                 └────────────────────────────────────────────▶ compose ◀── chrome surfaces
```

One effect pass per frame per output; the existing `BackdropPass` counters
(`backdrop_passes` / `backdrop_skipped`) guard it. Every chrome surface inherits
the material through its `MaterialRole`.

## Tasks

| Task | Scope |
|---|---|
| T179 — T-20.1 | Offscreen scene texture + Kawase backdrop blur pass (nested proof: menu bar + Apps drawer). |
| T180 — T-20.2 | Liquid-glass pass: edge refraction, specular inner rim, adaptive tint. |
| T181 — T-20.3 | Chrome material rollout: menu bar, Dock, context menus/popovers, OSD, notifications, Control Center, Applications drawer. |
| T182 — T-20.4 | Multi-GPU composition, software/headless `Minimal` fallback, golden determinism, frame-budget traces. |

## Acceptance

- [ ] The chrome backdrop visibly blurs **live** content (a playing video under
      the menu bar keeps updating), light and dark, reduced-motion, and under
      `Full`/`Reduced`/`Minimal`.
- [ ] The liquid-glass panels show refraction at the rim, a specular highlight,
      and an adaptive tint sampled from the backdrop.
- [ ] `Full` fits the frame budget on the recommended GPU; `Minimal` renders the
      deterministic approximation with zero idle wakeups.
- [ ] Each task carries a nested capture and a `PROGRESS.md` note; `make e2e`
      stays green and the goldens are tolerance-based.

## References

- [02-compositor.md](../02-compositor.md) — materials, the one-pass rule, the
  `set_panel_rect` mechanism.
- [ADR 0182](../adr/0182-tahoe-liquid-glass-material-pass.md) · [ADR 0013](../adr/0013-backdrop-blur-pass.md)
  · [ADR 0015](../adr/0015-material-degrade-tiers.md) · [ADR 0089](../adr/0089-dock-plate-geometry-and-live-panel-rect.md)
  · [ADR 0091](../adr/0091-dock-tahoe-floating-glass-language.md) · [ADR 0122](../adr/0122-tahoe-interface-language-across-chrome.md).