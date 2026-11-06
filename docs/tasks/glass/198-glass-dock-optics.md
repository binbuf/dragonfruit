---
verify: make gates
---
# T198 — Calibrate Dock optics

## Goal

Reproduce the measured Dock transmission, sharpness, rim and corner refraction with the existing Dock and compositor pipeline.

## Context (read first)

- Depends on T194–T197. Read [Dock measurements and source caveats](plan/REFERENCE.md), `docs/dock-learnings.md` §§3–5 and the recent Dock changes at `638f3dc`, `0339949` and `94129f7`.
- `shell/dock/Dock.qml` paints the plate and reports the live rect. `compositor/src/window/{backdrop,blur,material_region,material_cache}.rs` owns sampled optics, geometry and cache behavior.

## Scope

- [ ] Fit the Light/Dark flat band and a sharp-plus-multi-scale filtered response to the measured checker MTF. Use existing blur passes as a starting point. Treat 3.86 pt and 0.192 sharp fraction as fit seeds.
- [ ] Fit the rounded Dock rim and outward corner lens in logical units. Eliminate duplicate tint/rim/outer shadow on the sampled path. Keep a coherent readable shell-painted plate on unsupported paths and accessibility overrides.
- [ ] Reconcile the visual continuous corner with the actual compositor clip and the live Dock rect during hover, drag, overflow and reveal. Keep the plate framing consistent for left/right/bottom placement and at fractional scale.

## Out of scope

- Changes to magnification, tooltip timing, tile interactions, or slider extremes (T199/T202 and reference gaps).

## Design notes

Use neutral sRGB fit and saturated fixtures to calibrate the combined shader/output. Apply a perceptual tone curve with deliberate colour-space conversions when needed. A pure Gaussian loses the high-frequency floor. Calibrate the mid-grey rim peak +0.397, 1 pt half-amplitude and 6 pt tail. Measure sample displacement direction from grid lines, including corners rotated for side Docks. Use the reference layout and radius as calibration fixtures. Preserve user-configurable size and the existing work-area geometry.

## Done when

- [ ] The actual sampled Dock matches the measured MTF at each checker size within ±0.05, rim peak within ±5%, half-amplitude within ±0.5 pt, tail within ±1 pt, corner displacement within ±2 pt and flat-edge displacement under 1 pt. Record interior metrics and separate light/dark/accessibility runs.
- [ ] Existing `tst_dock` and `tst_materialpolicy` and compositor material coverage pass. Capture 1×/1.5×/2× where supported and report cache behavior during a sweep, then run `make gates`. Fill in Hand-off.

## Hand-off

_(Implementation session records measurements, backend/scale, and where tint, rim and shadow are drawn.)_
