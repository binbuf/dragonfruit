# 0208 — Region scene texture with a live scene base and a grow-only cache

## Status

accepted (T190b)

## Context

ADR 0207 defined the pure `MaterialRegion` padded-union helper but left both
backends rendering a full-output scene texture. T-21.3 needs the scene rendered
only where the material samples it, while keeping the panel result
pixel-equivalent. Two shapes were possible: keep a full-output scene texture
and update damaged regions, or draw the live scene elements as the output base
and use a region texture only as the blur source. The second shape also leaves
room for the T190d direct-scanout policy.

A Dock sweep changes the padded union's size as the live plate magnifies.
Reallocating on every size change would be a per-frame GPU allocation even
though the region is stable for most frames.

## Decision

- Both backends call `MaterialRegion::for_material` and render the scene
  elements into a texture allocated at `region.texture_size`. Elements are
  relocated by `-region.physical.loc`; each panel's source is translated by
  `MaterialRegion::panel_source`. `BackdropBlurElement` keeps the full panel
  for its rounded spans and samples only the visible `texture_panel`, so an
  output-edge clip does not invent a rounded corner.
- The default shape draws the live scene elements as the output base and uses
  the region texture only as the blur source. The old shape — the full-output
  scene texture composited as the base — is available with
  `DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1` for diagnosis and rollback.
- The Kawase chain's first destination remains the full output's physical
  size. Sizing it from the region would change the panel's effective blur
  radius; T190b must not change the visual result at the panel.
- `SceneTextureCache` is grow-only. The key is the allocated texture size,
  output mode, and scale; the region origin is deliberately absent. A larger
  region allocates once at the new maximum, a smaller region reuses the larger
  texture, and an output mode/scale change always reallocates. `PanelTextureSlots`
  trims removed panel slots so their `GlesTexture` handles are dropped.
- The material path is single-GPU (`drm.rs` disables multi-GPU sampled
  material), so the scene texture and Kawase chain share one GL context. Each
  offscreen frame is finished before the next bind; the nested `GlesFrame`
  waits on its fence when dropped. No cross-GPU copy or fence hand-off is
  needed.

## Consequences

- A stable panel set reaches its maximum region once and then allocates
  nothing per frame; a mode/scale change or a new panel is the only allocation
  source. `material stats` exposes the total, the scene/panel split, the region
  pixels, and the region dimensions.
- The helper returns one padded-union rectangle. With both the top menu bar and
  the bottom Dock blurred, that rectangle can be the full output even though
  the two panels are thin. A Dock-only frame is smaller. Multi-rect/tiled
  regions are a later optimization, not a correctness issue.
- T190c still owns stable scene/panel IDs and damage; T190d owns the
  direct-scanout policy and the per-output counter contract; T192 owns the
  region-vs-full-output benchmark.
- The full-output diagnostic renders the old base shape; the region path's
  correctness is compared at the panel, not by assuming the two compositions
  are byte-identical.

## References

- `compositor/src/window/material_region.rs`
- `compositor/src/window/material_cache.rs`
- `compositor/src/window/blur.rs`
- `compositor/src/backend/nested.rs`, `compositor/src/backend/drm.rs`
- ADR 0207, ADR 0182, ADR 0189
- `docs/rendering.md` R1.1/R1.3, `docs/design/02-compositor.md`
