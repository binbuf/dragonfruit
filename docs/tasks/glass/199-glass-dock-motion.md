---
verify: make gates
---
# T199 — Match Dock magnification and tooltip behavior

## Goal

Match the measured spatial and temporal magnification response and keep Dock interactions stable through animated geometry changes.

## Context (read first)

- Depends on T198. See [reference motion and the default-peak discrepancy](plan/REFERENCE.md#how-to-resolve-conflicting-evidence).
- `shell/dock/{Dock,DockEntry}.qml`, `shell/src/{dockmodel,shellcontroller}.cpp`, `docs/dock.md`, `shell/tests/tst_dock.qml`, and `scripts/capture-dock-magnify-sweep-driver.py`.
- The sibling `analysis/{magnification_sweep,temporal_response}.json` and raw `*__dock_layout.json` are the spatial and temporal calibration inputs.

## Scope

- [ ] Fit a continuous raised-cosine profile over resting tile centres with a bounded influence radius; calibrate both the diagnostic and default reference cases. Preserve live `dock.size`, `dock.magnification`, and `dock.position` across all supported ranges.
- [ ] Match the 135 ms smoothstep enter/exit envelope. Retarget from the current visual state during quick reversals. Keep the plate, hit targets and popup anchors in step and preserve reduce-motion and existing presentation pacing.
- [ ] Match the tooltip's shape, relative offset and dwell from the controlled reference. Retain drag/reorder, chooser, focus, launch and minimize ownership and avoid firing a tooltip on stale tile identity.
- [ ] Measure launch lift and genie timings through the existing settings options. If a timing needs tuning, do it only in the matching animation, preserving the selected mode and the existing window target mapping.

## Out of scope

- Inventing a launch multi-bounce or interactive auto-hide curve from the incomplete 60 fps references. Changing default minimize animation mode.

## Design notes

Use ~125.1 pt in the diagnostic fixture and 1.217× in the default fixture. Treat 2.047× as the observed diagnostic peak. Preserve current stable anchor and input arbitration during motion. Tooltip width remains label-driven. Treat the observed restart hard cut separately from pointer-triggered auto-hide. Reduced motion collapses animation but retains deliberate dwell and dismissal behavior.

## Done when

- [ ] Measured peak within ±3%, influence radius within ±5 pt and enter/exit 125–145 ms. Verify settle without sustained overshoot, and no idle repaint afterward. Exercise direct hover sweeps, re-entry, click while moving, edges, non-1× output, keyboard/drag, disabled magnification, and Reduce Motion.
- [ ] `tst_dock`, `tst_dockcore`, `make dock-magnify-sweep-capture` on a suitable host, and `make gates` pass, with capture limitations recorded. Fill in Hand-off.

## Hand-off

_(Implementation session records motion and tooltip results and remaining observational gaps.)_
