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

- [ ] Support trusted material geometry for a menu with attached corners, detached rounded cards, and a bounded set of independently keyed regions in one overlay when needed. Use known namespaces and validated preset/region metadata. Keep single-rect clients compatible and avoid an IPC extension unless existing metadata cannot express a case.
- [ ] Keep visual bounds (including overscan for rim, shadow or veil), input region, and reservation independent. Clip to owning output. Apply new region metadata with the same commit as its content, including removal and empty-panel semantics.
- [ ] Keep the sharp and filtered backdrop paths in compositor GPU memory. Resolve sample footprint for the actual blur and refraction kernel, and keep appropriate lower scene layers in the input for each region. Support later veil-only composition without scheduling blur.
- [ ] Retain stable per-region element identity. Track geometry, radius, scheme/effective paint, tier, output generation and intersecting padded source damage, and reuse region textures across unrelated damage and tint-only changes where valid.

## Out of scope

- Numeric tuning (T198/T200/T202/T203/T205) and backdrop requests from arbitrary app windows.

## Design notes

Preserve DRM and nested behavior through shared helpers. Bound region count, allocation growth, and padding. Clear regions on output/surface removal and invalidate old as well as new bounds on motion. Preserve correct z-order with overlapping chrome and avoid self-sampling, sampling protected/hidden content, or exposing textures to QML. Follow `docs/ipc-versioning.md` if touching XML. Preserve safe existing behavior on SHM transport, unavailable sampling and legacy private protocol.

## Done when

- [ ] Compositor unit/integration coverage exercises per-corner clipping, output edges and scale, multiple region identities, padded source damage, stale region removal, rejected invalid/untrusted metadata, and legacy fallback. Confirm stationary caches settle and hidden surfaces add no material or scanout veto.
- [ ] `make gates` passes. Document region ownership and compatibility in `docs/private-protocols.md` if applicable and fill in Hand-off.

## Hand-off

_(Implementation session records data flow, bounds, verified renderer paths, and limitations.)_
