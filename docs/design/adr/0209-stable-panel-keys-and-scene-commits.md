# 0209 — Stable panel keys, scene ids, and damage-only commits

## Status

accepted (T190c)

## Context

T190b made the scene and Kawase textures reusable, but both backends still
built `SceneTextureElement` and `BackdropBlurElement` with a fresh `Id::new()`
every frame. Smithay's damage tracker treats an unknown id as a new element and
damages its whole geometry; the previous frame's id is then "gone" and damages
the old geometry too. A stationary Dock therefore repainted its panel every
frame, and the full-output diagnostic scene base repainted the whole output.

Panel slots were indexed by the front-to-back list position, so a popover
opening, a panel disappearing, or a stacking-order change moved a panel onto
another panel's cached textures and identity.

## Decision

- `PanelKey` is an allocation-free hash of the chrome surface's namespace,
  layer, and Wayland object id. Geometry is deliberately excluded: a Dock that
  magnifies keeps one identity while its geometry changes.
- `PanelTextureSlots` is keyed by `PanelKey`. Each slot owns one `Id` created
  when the panel first appears; the slot, id, and Kawase textures are reused
  across frames and stacking-order changes. `trim_to` drops slots (and their
  `GlesTexture` handles) for panels that disappeared.
- `SceneTextureCache` owns one `Id` per output and a `CommitCounter`. The
  commit advances only when the offscreen scene's damage tracker reports a
  change. A reallocation keeps the id and the commit.
- The offscreen scene tracker is persisted in the scene cache and called with
  age 1, so the first frame (or a reallocation/region-size/scale change)
  renders the whole region and an unchanged scene renders nothing. The tracker
  is reset when the scene texture is reallocated or its region key changes.
- `BackdropBlurElement::with_id` and `SceneTextureElement::with_identity` carry
  the cached identity into the draw list. Panel commits stay at the default;
  the full-output scene base uses the scene commit.

## Consequences

- A stationary Dock over an unchanged wallpaper produces no output damage and
  no scene render. Moving the Dock damages only its old and new panel rects;
  a real scene change damages the scene element's geometry (the whole output in
  the diagnostic shape, the live scene elements in the default region shape).
- Panel add/remove is the lifetime boundary: a removed panel's textures are
  released immediately, and a returning panel is a new element that damages
  once on appearance. Reordering panels does not change identity.
- The panel key is a 64-bit hash. A collision would alias two panels; the
  namespace/layer/surface-id inputs make this vanishingly unlikely for the
  trusted shell's layer surfaces.
- T190d can rely on a settled scene producing no damage when it decides the
  direct-scanout/material transition; T192 can measure the settled frame
  without the old per-frame panel repaint.

## References

- `compositor/src/window/material_cache.rs`
- `compositor/src/window/blur.rs`
- `compositor/src/render.rs` (`BlurPanel`, `chrome_blur_panels`)
- `compositor/src/backend/nested.rs`, `compositor/src/backend/drm.rs`
- ADR 0208, `docs/rendering.md` R1.2/R1.3
