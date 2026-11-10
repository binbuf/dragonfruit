# 0255 — The Dock perimeter is one uniform rim; the lens is corner-gated

- Status: Accepted
- Date: 2026-11-10
- Task: T235a
- Supersedes: the directional-perimeter half of ADR [0217](0217-dock-optics-fit.md)

## Context

T198 fitted the Dock's flat band and multi-scale blur correctly, but the
perimeter was implemented as a directional treatment: a bright horizontal rim
mixed against a darker vertical side (`edgeSide`), straight top/bottom lifts in
panel-local `y`, and a corner lens parameterized by distance from the corner
centre with a panel-centre radial direction. The reference (`REPORT.md` §3.2)
shows one 1 px specular rim plus a ~6 pt inner glow around the whole rounded
rectangle, no outer shadow, and a corner-only magnifying bevel that is
strongest at rounded corners and ~0.5 pt on flat edges. The directional terms
made the sides dark, over-brightened the bottom, warped the flat end caps, and
left the fallback segmented and double-painted.

## Decision

1. **One rim profile, no direction.** The sampled shader evaluates a single
   rim against the distance inside the rounded-rect SDF. `edgeSide`,
   `topGlow`, `bottomGlow` and their heights are removed from the tokens, the
   resolver, the shader and the QML fallback. The first covered physical pixel
   carries the measured +0.397: a narrow specular line centred on that pixel
   (0.5 px inside the SDF edge) is combined with a super-exponential tail
   (half-amplitude at the token 1 pt, 10 % at ~2.5 pt, zero by the 6 pt tail).
   The line is in physical pixels so the peak is scale-consistent; the tail is
   in token logical points. There is no outer shadow.
2. **The corner lens is inside-rim and normal-gated.** The field is
   parameterized by `inside` (distance inside the rim), directed by the outward
   SDF normal, and multiplied by `4·nx²·ny²`. That gate is exactly zero on a
   flat top/bottom edge and on a flat side, and one at the 45° diagonal, so the
   lens cannot warp an end cap. The token peak is the displacement along the
   normal; at the 45° diagonal the measured horizontal component is
   `peak/√2`, calibrated to 9.6 pt at 15 pt inside (`cornerLensPeak` 13.6).
   `cornerLensDistance`/`cornerLensTail` are reinterpreted as distances inside
   the rim.
3. **The QML fallback uses the same perimeter.** One full-perimeter rim stroke
   plus three concentric glow strokes (6 px at 0.03, 3 px at 0.06, 1 px at
   0.10). `dockTopGlow`, `dockBottomGlow`, `dockBottomRim`, `dockEdge` and
   `bottomOutlinePath` are deleted; the Dock shadow is deleted. The border is
   an Increase Contrast-only uniform ink, never a dark side. The fallback has
   no `position` branch: the same path serves bottom, left and right Docks.
4. **The fallback's photometry is bounded by its opaque body.** The
   Minimal/floor-missed plate is an accessibility plate with a near-white
   near-opaque fill; a white rim can add only ~10/255 there. The sampled path
   is authoritative for the measured +0.397. The fallback is pinned to a
   uniform rim delta on every edge and the calibrated stroke opacity; its
   absolute peak is recorded as a deliberate deviation, not invented evidence.

## Consequences

- `tests/glass/test_glass_shader.py` probes the first pixel on all four edges
  at 1×/1.5×/2×, the half/10 %/tail points, corner-arc continuity, and flat
  edge/side lens identity. `window::dock_optics` unit tests pin the same model.
- `tst_dock` pins one full-perimeter rim, the three glow strokes, orientation
  independence, no directional layers, no double-painted bottom centre, no
  shadow, and uniform fallback rim delta.
- The capture matrix over the review backgrounds is recorded by
  `make dock-optics-capture`; unmeasured side/dark/scale rows stay explicit
  `OPEN` rows with the rerun command.

## References

- `docs/design/proposals/dock-liquid-glass-perimeter.md`
- `compositor/src/window/{blur,dock_optics}.rs`, `shell/dock/Dock.qml`
- `tests/glass/test_glass_shader.py`, `shell/tests/tst_dock.qml`
- ADR [0217](0217-dock-optics-fit.md), ADR [0112](0112-dock-plate-single-rounded-edge.md)
