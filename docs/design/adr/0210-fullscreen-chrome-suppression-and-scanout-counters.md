# 0210 — Fullscreen chrome suppression and per-output material counters

**Status:** accepted

**Context.** T-21.3 moved the material pass to a padded panel region, but the
persistent menu bar and Dock still composite above every window. A fullscreen
client is therefore never the last visible element on the output, so
`DrmCompositor` can never put its buffer on the primary plane, and the sampled
pass runs even when there is nothing to frost. The render trace also mixed
material frames with direct scanouts and had no per-output breakdown.

**Decision.**

- A settled fullscreen client that covers an output owns that output. The
  compositor hides persistent `top` chrome (menu bar, Dock) on that output;
  transient `overlay` surfaces (OSD, menus, notifications, popovers) still
  composite above it. The visibility rule is one pure helper,
  `shell::layer::visible_in_fullscreen`, and the filtered list is
  `DfState::visible_chrome_surfaces`. Frame-callback bookkeeping keeps using
  the unfiltered `chrome_surfaces`, so a hidden chrome surface is not left
  waiting on a callback. T201 adds one deliberate menu-bar exception: the
  `menu.autoHide` policy can keep the bar visible over fullscreen (`never`) or
  reveal it on demand (`always`/`full-screen`), and a concealed bar drops its
  reservation and material panel. See
  [ADR 0220](0220-menu-bar-auto-hide-visibility.md).
- The direct-scanout policy is the pure
  `window::scanout::direct_scanout_candidate`: a fullscreen rect that covers
  the output and no visible material panel intersecting it. Both backends
  record a candidate; only DRM advances the real `direct_scanouts` counter,
  from `PrimaryPlaneElement::Element`. Nested records candidates but can never
  claim a scanout.
- `RenderStats` gains `material_frames` (separate from `direct_scanouts`),
  `direct_scanout_candidates`, and an `output_material` map with monotonic
  per-output `material_frames`, `direct_scanouts`,
  `direct_scanout_candidates`, and `material_allocations`, plus the last
  `material_region_px`/`material_region`. The global counters remain for the
  existing trace contract.
- `DRAGONFRUIT_MATERIAL_FULL_OUTPUT=1` remains the diagnostic/rollback switch
  for the full-output scene texture (ADR 0208).

**Consequences.** Fullscreen now behaves like an immersive Space: the bars
disappear when the motion lands, and reappear when it exits. The shell still
renders its hidden chrome surfaces; skipping those commits is a later
transport optimization, not a correctness requirement. A visible overlay keeps
the sampled material and loses scanout for that frame, which is the required
frost behavior. The real `direct_scanouts>0` hardware trace remains the T191a
native sweep; this task commits the nested policy trace and an explicit OPEN
native artifact rather than faking the counter.
