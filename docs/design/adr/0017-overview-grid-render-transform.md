# 0017 — Mission Control's live-surface grid is a render-time scene transform

## Status

accepted

## Context

T-05.1a must transform the **real** window surfaces into the documented
Mission Control grid ([03-workspaces.md](../03-workspaces.md#window-layout-and-occlusion-t-11-u-5)),
never a cascade and never a thumbnail. T-04.3 already shipped the one reusable
`SceneTransform`/`MotionFrame` mapping (ADR 0014) and the T-02 lifecycle motion
renders through it. The open questions were: where the grid layout lives, who
owns the transform, and whether the grid moves the committed window geometry
(as the workspace slide does with `Space::map_element`) or only the drawing.

Moving the committed geometry would change input hit-testing, focus retargeting,
and the restore geometry mid-gesture, and would still not scale a Smithay
`Space` element. The task is explicit that the transform is the T-04 pass, and
hit-testing transfer is the next slice (T-05.2).

## Decision

- Add `compositor/src/overview/grid.rs` as the **one** Mission Control layout:
  pure geometry, no Smithay scene and no GPU. `grid_layout` orders candidates
  (active Space first, then strip order, most-recently-used first), picks the
  near-square column count, applies one uniform scale so the largest window
  fits its cell minus the token margin, and centers each window in its cell.
  Membership excludes minimized and fullscreen windows.
- The grid is a **render-time transform**, not a geometry move. `DfState`
  exposes `overview_grid_progress`, `overview_grid_layout`, and
  `overview_grid_frame`; `window_render_frame` returns the grid frame while the
  grid is shown and the lifecycle `MotionFrame` otherwise. The client surface,
  the SSD titlebar, and the shadow all read that one accessor, so they share
  one mapping (the T-04.3 invariant) and the transform interpolates with the
  shared overview progress.
- The committed geometry, focus, and Space assignment are **unchanged**;
  hit-testing transfer to the overview controller is T-05.2.
- Intrinsic window sizes read correctly because every window shares one scale;
  the transition is reversible because the frame lerps from the committed rect
  to the cell.
- The placement is instrumented through `query grid` (the progress and one
  line per placed live surface). The headless conformance test asserts one
  uniform scale, centered non-overlapping cells from each committed geometry,
  and that the client surfaces stay mapped.

## Consequences

- There is still exactly one scale/translate model; T-05.2 hit-testing and
  T-05.3 drag read the same placements instead of growing a second one, and
  T-05.6's frame budget degrades the same pass.
- The compositor renderer is a flat solid-color renderer, so the grid is
  asserted as geometry (the placement, not pixels); the T-04.2 backdrop blur
  remains the flat-tone feather approximation behind the tokens.
- Neighbour-Space reveal (transforming surfaces unmapped from the `Space`) and
  per-output chrome insets are deferred to later T-05 slices; `grid_layout`
  already models multi-Space membership via `GridCandidate::space_index`.