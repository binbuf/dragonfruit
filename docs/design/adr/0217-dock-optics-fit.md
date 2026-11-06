# 0217 — The Dock uses a measured sharp-plus-multi-scale optical fit

- Status: Accepted
- Date: 2026-11-06
- Task: T198

## Context

T195 defined the Dock preset's flat-band fit (light alpha 0.253 / tint 0.604,
dark 0.149 / 0.207) and a 3.86 pt blur seed. T197 made the material regions,
per-corner clipping and filtered/sharp paths compositor-owned. A single
Gaussian loses the measured high-frequency floor: the reference checker
response is 0.161 / 0.150 / 0.325 / 0.670 / 0.701 at 2/4/8/16/32 pt, with a
sharp floor and a low-frequency shoulder that no one blur radius reproduces.
The QML plate also painted its own fill, gloss, rim, edge, border and shadow
over the compositor's glass, so the sampled Dock had duplicate edges.

## Decision

1. **The fit is a convex composite, not one Gaussian.** The Dock preset records
   `sharpFraction` 0.12, `edgeTail` 6 pt, and corner-lens
   `cornerLensPeak`/`Distance`/`Tail` 9.6/15/50 pt. The renderer composites the
   sharp scene (weight 0.12) with Kawase levels 1, 2 and 5 (0.26/0.40/0.22)
   into one panel-resolution texture. `window::dock_optics` is the pure model
   of the same composite; the fit matches all five measured periods within
   0.05 before the flat-band transmission is applied.
2. **The flat band, rim and corner lens are painted once, by the compositor.**
   The shader applies the neutral flat-band mix, an exponential rim with a
   1 pt half-amplitude and a 6 pt tail, and a corner-only lens that peaks
   9.6 pt at 15 pt from the corner and decays by 50 pt. Flat edges are outside
   the corner field. Increase Contrast raises the resolved rim peak and width
   after the measured fit.
3. **QML owns only the unsupported path.** On `sampled-glass` the Dock's QML
   plate layers are hidden, eliminating the duplicate tint/rim/shadow. Solid,
   software/no-sampled-blur and Reduce Transparency keep one coherent
   shell-painted plate using the resolved neutral alpha/tint; the legacy
   rollout keeps its milky fallback.
4. **The chain is panel-space for the Dock.** The shared filtered path keeps
   its full-output chain base; the Dock composite uses the visible panel's
   physical size so the sharp scene and every Kawase level are aligned and the
   blur radius is not stretched by the panel/output ratio.
5. **The scorer separates flat band from filtered response.** The capture
   divides the raw checker ratio by `1 - alpha` before comparing with the
   measured targets, so transmission and blur are not scored twice.

## Consequences

- The 1x reference layout passes the flat band, every resolved checker period,
  the rim peak/half/tail and flat-edge geometry in light, dark, Reduce
  Transparency and Increase Contrast. `make dock-optics-capture` records the
  evidence and a sweep cache report.
- The 1.5x/2x captures are recorded but their MTF rows remain OPEN: the nested
  scene-texture scale path attenuates the backdrop more than the 1x
  calibration. Closing that is a T207/T208 follow-up, not a change to the
  measured 1x fit.
- Other roles keep the legacy adaptive tint/specular path; the measured model
  is selected only for the replacement Dock preset.

## References

- `docs/tasks/glass/plan/REFERENCE.md` — Dock plate measurements.
- `compositor/src/window/dock_optics.rs`, `compositor/src/window/blur.rs`,
  `compositor/src/backend/nested.rs`, `shell/dock/Dock.qml`.
- `docs/captures/t198-dock-optics.md`, `scripts/capture-t198-dock-optics.sh`,
  `scripts/t198-dock-optics.py`.
- ADR [0214](0214-glass-role-presets-and-resolution.md), ADR
  [0216](0216-material-regions-and-composition.md).
