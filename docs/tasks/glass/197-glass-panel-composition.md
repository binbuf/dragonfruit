---
verify: make gates
---
# T197 — Extend compositor material geometry and composition

## Goal

Let existing trusted chrome declare the visual regions needed for glass while preserving input geometry, scene ordering and damage bounds.

## Context (read first)

- Depends on T195–T196. Read `compositor/src/render.rs::{chrome_blur_panels,chrome_backdrop_render_elements}`, `compositor/src/window/{backdrop,blur,material_region,material_cache}.rs`, and `compositor/src/backend/{nested,drm}.rs`.
- `protocols/dragonfruit-shell.xml` defines commit-latched `set_panel_rect`; `compositor/src/shell/{layer,trust}.rs` owns surface placement and authorization.
- Read `docs/design/adr/{0207-material-region-padded-union,0209-stable-panel-keys-and-scene-commits,0212-rounded-span-damage-translation}.md`.

## Scope

- [x] Support trusted material geometry for a menu with attached corners, detached rounded cards, and a bounded set of independently keyed regions in one overlay when needed. Use known namespaces and validated preset/region metadata. Keep single-rect clients compatible and avoid an IPC extension unless existing metadata cannot express a case.
- [x] Keep visual bounds (including overscan for rim, shadow or veil), input region, and reservation independent. Clip to owning output. Apply new region metadata with the same commit as its content, including removal and empty-panel semantics.
- [x] Keep the sharp and filtered backdrop paths in compositor GPU memory. Resolve sample footprint for the actual blur and refraction kernel, and keep appropriate lower scene layers in the input for each region. Support later veil-only composition without scheduling blur.
- [x] Retain stable per-region element identity. Track geometry, radius, scheme/effective paint, tier, output generation and intersecting padded source damage, and reuse region textures across unrelated damage and tint-only changes where valid.

## Out of scope

- Numeric tuning (T198/T200/T202/T203/T205) and backdrop requests from arbitrary app windows.

## Design notes

Preserve DRM and nested behavior through shared helpers. Bound region count, allocation growth, and padding. Clear regions on output/surface removal and invalidate old as well as new bounds on motion. Preserve correct z-order with overlapping chrome and avoid self-sampling, sampling protected/hidden content, or exposing textures to QML. Follow `docs/ipc-versioning.md` if touching XML. Preserve safe existing behavior on SHM transport, unavailable sampling and legacy private protocol.

## Done when

- [x] Compositor unit/integration coverage exercises per-corner clipping, output edges and scale, multiple region identities, padded source damage, stale region removal, rejected invalid/untrusted metadata, and legacy fallback. Confirm stationary caches settle and hidden surfaces add no material or scanout veto.
- [x] `make gates` passes. Document region ownership and compatibility in `docs/private-protocols.md` if applicable and fill in Hand-off.

## Hand-off

### Changes

- **Private protocol v3 (additive).** `df_shell`/`df_layer_surface` are v3.
  `set_panel_region(key, preset, x, y, w, h, tl, tr, br, bl, overscan)` declares
  one independently keyed visual region in surface-local logical coordinates;
  `clear_panel_regions` commits an explicit empty set. `set_panel_rect` keeps
  its legacy single-rect behavior. Both new requests are appended after the
  existing request list so existing opcodes do not move. Region requests are
  commit-latched with the buffer they describe; the first request after a
  commit starts from the committed list (add/replace one region), and
  `clear_panel_regions` is the removal path.
- **Validation/bounds.** `compositor/src/shell/layer.rs` adds `PanelRegion`,
  `MAX_PANEL_REGIONS = 8`, key length 64 and `[A-Za-z0-9._-]`, preset names from
  the generated T195 vocabulary, overscan ≤ 512, radius ≤ 4096. Invalid keys,
  unknown presets, and overflow entries are ignored without disturbing the
  committed list. `ChromeSurface` now carries `regions` (output-local, clipped
  to the surface and owning output) plus the legacy `panel` union.
- **Geometry/composition.** `BlurPanel` carries per-corner `CornerRadii`,
  overscan, `RegionFilter` (`Filtered`/`Sharp`), and `RegionPaint`
  (scheme/preset/tier/mode/alpha/tint/edge/revision). `default_preset` maps the
  known namespaces (`dock`, `menubar`, `menubar-popup`, `dock-popup`,
  `control-center`, `notification`, `notification-center`, `osd`,
  `apps-drawer`) and resolves through the live T195/T196 policy. The fallback
  feather path is per-region and per-corner.
- **Sample footprint.** `BlurSpec::padding` + `GlassSpec::sample_padding_px`
  (refraction/lens reach) + declared overscan form the region padding in
  `MaterialRegion::from_panels`; the union is still clamped to the output. A
  sharp/veil region reserves only its overscan + one sample pixel.
- **Identity/cache.** `PanelKey::for_region(namespace, layer, surface_id,
  region_key)` keeps independent element ids. `PanelBlurKey` now includes the
  padded source footprint and filter; `PanelPaintKey` tracks scheme, preset,
  tier, mode, paint values, corner radii, output generation, and policy
  revision. Paint-only changes advance the element commit but reuse the
  filtered texture. `PanelTextureSlots::trim_to` drops removed regions.
- **Backends.** Nested and DRM share the same `BlurPanel`/`MaterialRegion`
  helpers. Filtered regions run the existing Kawase/liquid-glass chain; sharp
  regions sample the compositor's region scene texture directly with no chain
  allocation and no `blur_passes`/`blur_draws` increment. DRM records a
  material frame for a sharp-only veil; both paths remain GPU-resident and are
  never exposed to QML.
- **Tests/docs.** Per-corner span tests, padded-damage/paint-reuse/region-key
  cache tests, sample-footprint tests, a conformance test covering commit
  latching, multiple keys, invalid metadata, the 8-region bound, empty removal,
  and legacy fallback, plus the existing stationary/hidden/scanout coverage.
  ADR 0216, `docs/private-protocols.md`, `docs/design/02-compositor.md`, and
  `docs/rendering.md` document ownership/compatibility.

### Data flow and bounds

Surface-local region rect → `chrome_surfaces` intersects surface geometry and
the owning output → output-local `ChromeRegion` → `chrome_blur_panels` resolves
preset/paint/filter → `BlurPanel` → `MaterialRegion` expands by the full sample
footprint and clamps to the output → `panel_source` maps each region into the
scene texture → Kawase chain (filtered) or direct scene sample (sharp) →
`BackdropBlurElement` clips per-corner spans and damages only the element's
padded source.

The region list is bounded to 8 per surface; the scene texture remains
grow-only per output. Multi-rect scene tiling is not part of this slice: the
menu bar + Dock still produce one padded union.

### Verified renderer paths

- Nested `GlowRenderer`: filtered and sharp paths both draw; sharp path
  allocates no Kawase texture.
- DRM `MultiRenderer`/`UdevRenderer`: the same shared helpers and stats; sharp
  path records a material frame without a blur draw.
- Software/headless, SHM transport, unavailable sampling, and legacy protocol:
  `chrome_blur_panels` returns empty and the deterministic feather fallback
  runs. A surface that declares `Some(empty)` draws no material rather than
  falling back to an old rect.

### Limitations

- The shell still sends single-rect metadata for the Dock/menu bar; T202/T203/
  T205 will declare the attached/detached/multi-card regions. T197 provides and
  verifies the compositor contract.
- The sharp path currently does plain scene sampling; T200 owns the veil tint/
  rim and its no-blur composition. T198/T202/T203/T205 own optical tuning.
- Multi-rect/tiled scene textures remain a later optimization; correctness and
  identity are per-region today.
- Arbitrary app windows cannot request backdrop material; only trusted chrome.

### Checks (final tree)

- `make gates` → PASS: glass self-test 120 passed, presets 151 passed, e2e
  (including `shell_protocol_conformance` 49/49 with the new multi-region
  case), QML 83/83, total 385 s.
- `cargo test -p dragonfruit-compositor --lib` → 76 passed.
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` → 405
  passed.
- `cargo clippy -p dragonfruit-compositor --all-targets -- -D warnings` →
  clean.
- `cmake --build build --target dragonfruit-shell -j 2` → pass (the scanner
  still emits its pre-existing `since version not increasing` ordering
  warnings for `ack_configure`/the event block; build succeeds).
