# 0343 — The Dock end lens is a split long-axis / cross-axis field

- Status: Accepted
- Date: 2026-11-24
- Task: T198 / T235a follow-up
- Supersedes: the lens-direction and along-axis-reach half of ADR
  [0255](0255-dock-uniform-perimeter-and-corner-gate.md); amends the
  short-end interpretation in ADR [0217](0217-dock-optics-fit.md)

## Context

The measured `4 / 8.5 / 9.6 pt` Dock lens profile was recorded by tracking a
**vertical grid line at the top-left corner** of the reference capture
(`REPORT.md` §3.2 Lens; `measurements.json` →
`dock.lens.measured_vertical_line`). The line is 35 px (17.5 pt) from the
plate's left edge, inside the 26.9 pt corner radius, and the reported values
are its **horizontal** displacement at 5 / 10 / 15 pt inside the top rim.

The implementation had read those values as a displacement along the SDF
normal at the **short-end middle** and applied the whole field along that
normal. Two problems followed:

1. **The corner lost the horizontal component.** At the research's actual
   corner point the power-4 squircle normal is nearly vertical, so the current
   shader produced only about `0.5 / 2.4 / 4.3 pt` of horizontal displacement
   at 5 / 10 / 15 pt instead of the measured `4 / 8.5 / 9.6 pt`. At the
   45-degree diagonal it produced `6.6 pt` at 15 pt, not the measured `9.6 pt`.
2. **The flat edges gained a vertical lens.** Because `bevel_end_gain` used
   `cornerLensTail` (50 pt) as the **along-axis** e-fold, the same field kept
   `3.7 pt` of vertical displacement on the top edge 50 pt from the corner
   centre and `1.6 pt` at 100 pt. The reference's long flats are the measured
   `~0.5 pt` (`REPORT.md` §3.2; `docs/design/04-shell.md` resting proportions).

The 50 pt token is the **depth/bezel influence radius** inside the rim, not an
along-axis reach. The research did not measure the along-axis reach.

## Decision

1. **Split the measured bevel into two components.**
   - The **long-axis component** is the measured horizontal end/corner
     displacement. It is applied across the whole short end cap (the corner
     and the short-end middle share it) and fades along the long edges over a
     separate `cornerLensAlong` reach.
   - The **cross-axis component** is the Snell bevel normal component with the
     measured `DOCK_BEVEL_FLAT_GAIN` floor. It keeps the flat top/bottom at
     `~0.5 pt` and preserves the two-axis bend at the corner.
2. **Keep the established `4 / 8.5 / 9.6 pt` profile** as the horizontal
   magnitude. The newer window-edge study (`6.34 / 9.55 pt` at `6 / 9 pt`,
   `docs/DOCK-REFRACTION.md` §3.6) remains **OPEN**: it is steeper than the
   established grid-line fit, and a future capture must decide whether the
   measurement methods differ or the profile needs a re-fit. The shader and
   model continue to use the established profile until that decision.
3. **Add `cornerLensAlong` as a separate Dock token** (`12 pt` in both
   schemes). `cornerLensTail` remains the `50 pt` depth/bezel range. The
   along-axis value is not a measured reference value; `12 pt` is chosen so
   the measured flat-edge probes stay near `~0.5 pt` (about `0.6 pt` at 50 pt
   from the corner centre and `0.5 pt` at 100 pt) and the caustic is confined
   to the end cap plus a short transition.
4. **Gate the sharp caustic with the same along-axis reach.** Its depth band
   remains `6.6–18 pt` (`~11.4 pt`, matching the measured `~12 pt` band), but
   it no longer runs `~126 pt` along the top/bottom edges. The caustic is a
   short-end band, not a long-edge stripe.
5. **Pin the correction in tests.** The Rust model test probes the research's
   actual corner point and the `50 / 100 pt` flat-edge probes. The GPU shader
   test measures the corner horizontal displacement and asserts the caustic
   does not spill along the long edge. The T198 scorer records
   `corner_lens_along_pt` alongside the existing corner metadata.

## Consequences

- Corner horizontal displacement at the research point becomes
  `~4.0 / 7.9 / 9.2 pt` at `5 / 10 / 15 pt` (measured `4 / 8.5 / 9.6 pt`,
  within the existing `±0.7 pt` fit tolerance at the model level).
- The top/bottom flat-edge cross-axis displacement at `50 / 100 pt` from the
  corner centre becomes `~0.6 / 0.5 pt` instead of `~3.7 / 1.6 pt`.
- The caustic along-axis reach shrinks from `~126 pt` to `~30 pt`; its depth
  band is unchanged.
- The short-end middle keeps the full measured lens. The `cornerLensPeak`
  token remains the `13.6 pt` normal/horizontal relation; the measured
  horizontal peak is `cornerLensPeak / sqrt(2) = 9.6 pt`. The Snell fit yields
  `9.24 pt` at 15 pt, within the existing `±0.7 pt` fit tolerance.
- The old single normal-directed `bevel_end_gain` is retained only as the
  cross-axis gain. The long-axis gain is `end_lens_along_gain`.
- The `materialize` axis still scales the whole displacement. At the default
  `materialize = 1.0` the measured fit is unchanged.
- The dark scheme continues to reuse the light lens fit; the research did not
  separately measure a dark lens.

## Scope and limitations

- The current product plate uses 48 pt icons, a 78 pt bar and 15 / 16 pt
  padding (ADR [0308](0308-dock-resting-proportions-restored.md)). The
  reference material measurement is a 1163 × 80 pt plate with ~60 pt tiles and
  ~8.5 pt horizontal padding. The lens fit is per-plate and independent of
  those proportions, but a like-for-like visual comparison must use a matched
  plate width or the reference proportions.
- The T198 fixture is 483 pt wide, not the 1163 pt reference. The old absolute
  50 pt along-axis reach occupied a much larger fraction of the narrow fixture;
  the 12 pt reach reduces that width sensitivity, but it does not make the two
  widths optically identical.
- The newer window-edge corner profile (`6.34 / 9.55 pt` at `6 / 9 pt`) remains
  OPEN. The implementation preserves the established grid-line fit until a
  capture decides whether the methods differ or the profile needs a re-fit.
- The rim inner-glow shape and the dark-scheme lens remain secondary
  calibration items; this ADR changes the end-lens direction and reach only.

## References

- `docs/REPORT.md` §3.2 Lens and §4.1 Dock model (reference repository)
- `docs/DOCK-REFRACTION.md` §3.6–3.7 (reference repository)
- `docs/measurements.json` → `dock.lens.measured_vertical_line`
- `compositor/src/window/dock_optics.rs`, `compositor/src/window/blur.rs`
- `tests/glass/test_glass_shader.py`, `scripts/t198-dock-optics.py`
- ADR [0217](0217-dock-optics-fit.md), ADR
  [0255](0255-dock-uniform-perimeter-and-corner-gate.md)
