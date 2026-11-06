# 0216 — Material regions are commit-latched, independently keyed surface metadata

## Status

accepted (T197)

## Context

`df_layer_surface.set_panel_rect` describes one rectangular material panel per
chrome surface. That is enough for the Dock plate and the Applications drawer,
but not for an attached menu with square top corners and rounded bottom
corners, nor for one overlay that paints several detached cards (Control
Center modules, a notification stack, or a bounded set of status popovers).
The T195 preset vocabulary already names those surfaces, and the T190 region
cache already has a stable panel identity, but the compositor had no way to
receive more than one visual rect per surface or to distinguish a filtered
backdrop from a sharp/veil one.

The input region and reserved zone are interaction/reservation contracts. They
must not be inferred from, or changed by, a visual material hint.

## Decision

- Add the additive v3 `df_layer_surface` requests `set_panel_region` and
  `clear_panel_regions`. `set_panel_rect` remains the legacy single-rect path
  and is unchanged. Region requests are commit-latched: they take effect with
  the next `wl_surface.commit`, exactly like the pixels they describe.
- One region carries a surface-local visual rect, a stable short key, an
  optional known glass preset, four independent corner radii, and a bounded
  overscan for rim/shadow/veil. The compositor clips each rect to the surface
  geometry and the owning output, so regions never leak across displays.
- Region keys are validated (`[A-Za-z0-9._-]`, bounded length), presets must
  exist in the generated T195 vocabulary, and the list is bounded to
  `MAX_PANEL_REGIONS` (8). Invalid metadata is ignored without changing the
  committed list; the first request after a commit starts from the committed
  list, and `clear_panel_regions` commits an explicit empty set.
- Each region resolves its preset through the live T195/T196 policy and keeps
  `PanelKey::for_region(namespace, layer, surface_id, region_key)` as its
  stable identity. A legacy surface keeps `PanelKey::new(...)` and one `panel`
  region.
- `BlurPanel` carries the per-corner shape, overscan, filter mode, and
  `RegionPaint` (scheme, preset, tier, render mode, alpha/tint/edge, policy
  revision). `PanelBlurKey` includes the sample padding and filter, so damage
  inside the padded source invalidates the cached texture while tint-only
  changes do not. `PanelPaintKey` advances the element commit for paint-only
  changes and reuses the filtered texture.
- The filtered path keeps the Kawase chain. The sharp/veil path samples the
  compositor's region scene texture directly, with no blur allocation and no
  `blur_draws`/`blur_passes` increment. Both paths stay in compositor GPU
  memory; no readback and no texture exposure to QML.
- The scene texture remains one padded union per output (multi-rect tiling is
  still a later optimization). The union now uses each region's full sample
  footprint: max(blur radius, Kawase footprint, refraction/lens reach, declared
  overscan), clamped to the output.

## Consequences

- An attached menu keeps square top corners and rounded bottom corners without
  a second protocol; a detached card rounds all four. One overlay can compose a
  bounded set of independently identified regions, and a returning region
  keeps its id and cached texture if its key is stable.
- A tint/scheme/tier change repaints the element but reuses the filtered
  texture. A geometry move, filter change, source move, or damage intersecting
  the padded source rebuilds it. Stale regions are trimmed with the panel set,
  including on output/surface removal.
- SHM transport, unavailable sampling, software/headless, and legacy private
  protocol clients keep the existing feather fallback; the new requests are
  additive and only trusted shell clients can bind them.
- Multi-rect scene tiling remains out of scope: a menu bar plus a Dock still
  produces one tall union. The helper and cache are shaped so that change can
  land without altering region identity.

## References

- `compositor/src/window/{material_region,material_cache,blur,corner}.rs`
- `compositor/src/render.rs` (`BlurPanel`, `chrome_blur_panels`)
- `compositor/src/shell/{layer,mod}.rs`
- `protocols/dragonfruit-shell.xml`
- ADR [0207](0207-material-region-padded-union.md), [0208](0208-region-scene-texture-and-grow-only-cache.md), [0209](0209-stable-panel-keys-and-scene-commits.md), [0212](0212-rounded-span-damage-translation.md), [0214](0214-glass-role-presets-and-resolution.md)
