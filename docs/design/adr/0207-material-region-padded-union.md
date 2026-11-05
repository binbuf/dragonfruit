# 0207 — The material region is the padded panel union

## Status

accepted

## Context

The GPU material pass currently renders the whole output into a scene texture
and samples each chrome panel from it. T-21.3 wants a region-sized scene
texture, but a region that stops at the panel edge would clamp the blur kernel
and show a hard seam. The region calculation must therefore be a pure,
renderer-free helper shared by both backends, and its padding must come from
the same `BlurSpec` that drives the Kawase chain rather than a literal margin.

## Decision

- `compositor/src/window/material_region.rs` owns `MaterialRegion`. It takes
  the resolved `BlurPanel`s, an output-local logical output rectangle, and the
  output scale, and returns:
  - `rect`: the padded union in output-local logical coordinates;
  - `physical`: the same region after `to_physical_precise_round` and a second
    clamp to the physical output rectangle;
  - `texture_size`: the buffer size for the region scene texture.
  `None` means no panel has a visible area (empty, degenerate, fully outside,
  or an invalid output/scale), so the caller keeps the existing path.
- Each panel is expanded by **its own** padding before the union, then the
  union is clamped to the output. A menu bar at one tier and a popup at another
  therefore keep their own margins; expanding the bare union by the largest
  padding would under-pad the smaller panel.
- `BlurSpec::padding()` is the larger of:
  - the token blur `radius` (the requested soft reach), and
  - the Kawase chain footprint `downsample = 2^iterations` (the discrete taps
    reach `2^iterations - 1` source pixels, plus the first bilinear texel).
  `padding_px()` rounds up to whole logical pixels. There is no padding
  literal; a tiny radius still keeps the minimum one-iteration footprint.
- The helper is pure geometry. T190a does not change either backend; T190b
  consumes `physical` as the region texture geometry/origin and
  `texture_size` as its allocation size.

## Consequences

- The region always contains the full blur kernel at a panel edge while still
  being smaller than the output for a Dock-only frame. At a fractional scale
  the physical rectangle is clamped again after independent location/size
  rounding, so a texture cannot poke past the output edge.
- The logical `rect` stays the canonical damage/ownership rectangle; the
  physical `physical.loc` is the origin a panel `src` rectangle is translated
  by. The full-output diagnostic path remains unchanged.
- Padding is conservative by the token radius even though the current shader
  only uses the iteration count. That keeps the region correct if the shader
  later consumes the radius directly, at the cost of a modestly larger region.
- A future rotated-output material path must pass a transform-aware output
  rectangle; today the helper and the material pass are normal-transform only.

## References

- `compositor/src/window/material_region.rs`
- `compositor/src/window/blur.rs` (`BlurSpec::padding`)
- `compositor/src/render.rs` (`BlurPanel`)
- `docs/rendering.md` R1.1, `docs/design/02-compositor.md`
- ADR 0182 (Tahoe liquid-glass material pass)
