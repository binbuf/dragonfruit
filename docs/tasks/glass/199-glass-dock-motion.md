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

- [x] Fit a continuous raised-cosine profile over resting tile centres with a bounded influence radius; calibrate both the diagnostic and default reference cases. Preserve live `dock.size`, `dock.magnification`, and `dock.position` across all supported ranges.
- [x] Match the 135 ms smoothstep enter/exit envelope. Retarget from the current visual state during quick reversals. Keep the plate, hit targets and popup anchors in step and preserve reduce-motion and existing presentation pacing.
- [x] Match the tooltip's shape, relative offset and dwell from the controlled reference. Retain drag/reorder, chooser, focus, launch and minimize ownership and avoid firing a tooltip on stale tile identity.
- [x] Measure launch lift and genie timings through the existing settings options. If a timing needs tuning, do it only in the matching animation, preserving the selected mode and the existing window target mapping.

## Out of scope

- Inventing a launch multi-bounce or interactive auto-hide curve from the incomplete 60 fps references. Changing default minimize animation mode.

## Design notes

Use ~125.1 pt in the diagnostic fixture and 1.217× in the default fixture. Treat 2.047× as the observed diagnostic peak. Preserve current stable anchor and input arbitration during motion. Tooltip width remains label-driven. Treat the observed restart hard cut separately from pointer-triggered auto-hide. Reduced motion collapses animation but retains deliberate dwell and dismissal behavior.

## Done when

- [x] Measured peak within ±3%, influence radius within ±5 pt and enter/exit 125–145 ms. Verify settle without sustained overshoot, and no idle repaint afterward. Exercise direct hover sweeps, re-entry, click while moving, edges, non-1× output, keyboard/drag, disabled magnification, and Reduce Motion.
- [x] `tst_dock`, `tst_dockcore`, `make dock-magnify-sweep-capture` on a suitable host, and `make gates` pass, with capture limitations recorded. Fill in Hand-off.

## Hand-off

### What landed

- **Spatial profile.** `Dock.qml` uses the measured raised cosine over resting
  tile centres with `component.dock.magnifyInfluence = 125.1` pt. Peak anchors
  are `magnifyPeak = 1.217` (default 0.5) and `magnifyPeakMax = 2.047`
  (diagnostic 1.0), joined by a monotone smoothstep mapping. Live `dock.size`,
  `dock.magnification` and `dock.position` stay finite and bounded
  (`dockConfigFromValues` clamps them).
- **Temporal envelope.** `motion.dockHover` is 135 ms with Bezier control
  points `(1/3, 0)` / `(2/3, 1)` (exact `3u² − 2u³`). The Behavior retargets
  from the current value, so quick reversals continue from the visible state;
  plate, input rects and popup/tooltip anchors all read the same live layout.
  Reduced motion still collapses to zero.
- **Tooltip.** 1400 ms dwell, label-driven pill ≈33 pt high (8 pt vertical
  padding), half-height radius, 8 pt above the plate, no Dock tail. A stale
  projection (reorder/pin change moving another tile under the pointer)
  dismisses the pending/open label instead of firing the old identity.
- **Launch/genie.** Launch lift is the measured 15 pt
  (`component.dock.launchLift`); the existing three-hop shape is unchanged.
  `motion.windowGenie` is 450 ms and is selected only for minimize/restore
  while `dock.minimizedAnimation=genie`; scale keeps 100/160 ms and none keeps
  the collapse. The window target/origin mapping is unchanged.
- **Evidence.** `scripts/t199-dock-motion.py` fits the reference sweep
  (A 1.0676, R 125.1 pt, observed 2.0469×) and checks the tokens;
  `docs/captures/t199-dock-motion.md` is the generated report. The sweep driver
  now detects the nested window by non-black content, crops bottom-relative,
  and closes the demo's client windows before sweeping.
- **Docs.** ADR 0218, `docs/design/04-shell.md`, `10-design-system.md`,
  `02-compositor.md`, `docs/dock.md`, `advisor-briefing.md`, the amended ADRs
  0034/0162/0186, and `docs/captures/README.md`.

### Check results (final tree)

- `python3 scripts/t199-dock-motion.py check` — all 8 token checks pass;
  `make dock-motion-calibration` pass.
- `cargo test -p dragonfruit-compositor --lib` — 76 passed.
- `cargo test -p dragonfruit-compositor --bin dragonfruit-compositor` — 413
  passed (includes the new genie envelope/mode tests).
- `cargo test -p dragonfruit-compositor --test reduced_motion_sweep` — 3
  passed.
- `ctest --test-dir build -R '^(tst_dock|tst_dockcore|qmllint_shell-dock)$'` —
  3/3 passed; `tst_dock` 314 passed (raised cosine, both peaks, influence
  bound, 135 ms enter/exit, reversal, tooltip shape/dwell/stale identity,
  click-while-moving, keyboard/drag, live ranges, reduce motion).
- `make qml-test` — 83/83 passed.
- `make dock-magnify-sweep-capture` — pass under a virtual KWin host:
  `docs/captures/t14-dock-magnify-sweep-{light,dark}.png` and
  `sweep-row-{light,dark}-after.png`.
- `make gates` — PASS (glass self-test 120/0, presets 151/0, e2e, QML 83/83).

### Capture limitations

- The sweep ran under `kwin_wayland --virtual --scale 1`; the nested
  compositor reports a 1920×1200 physical output while the shell overview is
  1280×800, so the driver uses normalized pointer positions and a
  bottom-relative crop. The strip is 1× only.
- The driver closes the demo's Settings/X11 windows so the crops show the Dock
  over wallpaper rather than an app pane behind the glass.
- 1.5×/2× Dock optics MTF remains T198's open scale-path item; T199 does not
  re-open it.

### Remaining observational gaps

- Genie still renders as the existing per-window scale transform; the warp is
  a material-pass follow-up, but its 450 ms timing and mode selection are now
  pinned.
- The reference launch multi-bounce is unresolved; only the 15 pt first lift
  is adopted, not a new bounce curve.
- The Dock no longer uses the Tooltip tail; the component keeps the opt-in
  tail for other consumers.
