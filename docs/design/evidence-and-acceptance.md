# Evidence and acceptance

| | |
|---|---|
| **Status** | target process |
| **Date** | 2026-11-08 |
| **Scope** | how total-experience claims are captured, measured, reviewed, and gated |
| **Related** | [22-total-experience.md](22-total-experience.md), [motion/feedback](motion-and-feedback.md), [experience schema](experience-schema.md), `macos-design-review/docs/CAPTURE-PLAN.md`, [STATUS.md](STATUS.md) |

## 1. Why this process exists

The Liquid Glass campaign proved that measured values beat eyeballed values.
The expanded scope is larger and easier to fake: a screenshot of a menu is not
a menu, a still of the Dock is not a click tree, and a settings pane with a
toggle is not a setting.

This process makes each claim falsifiable:

- **Capture** the reference state or motion.
- **Measure** the geometry, colour, timing, or behaviour.
- **Implement** against the measurement.
- **Compare** the implementation against the reference with a committed script.
- **Record** the evidence and any `OPEN` rows.

## 2. Evidence classes

| Class | Example | Produced by |
|---|---|---|
| Still | Dock hover, Settings pane, menu | `screencapture`, manifest |
| Motion | Cmd+Tab, minimize, HUD | 120 fps recorder, frame extraction |
| AX tree | Labels, roles, order, frames | `uictl dump-ui` / AX dump |
| Geometry | Tile layout, window frame, hit targets | AX + edge detection |
| Measurement | Material fit, timing curve, MTF | analysis scripts |
| Behaviour trace | Event sequence, state machine, input log | headless tests, trace files |
| Host API trace | D-Bus calls, package manager output | adapter tests |
| Native proof | DRM scanout readback | T-21 native rail |

Nested screenshots are **reference evidence**, never native proof. A still of a
nested session does not prove the DRM path, HiDPI sharpness, or the cursor.

## 3. Artifact schema

The canonical machine-readable schema is
[experience-schema.md](experience-schema.md) and
`docs/captures/experience-schema.json`; the manifest is
`docs/captures/experience-manifest.json`. The example below is the shape the
contract preserves, not a second schema.

The 2026-11-08 structural review adds a second shareable reference artifact,
`macos-design-review/docs/experience_measurements.json` (title-bar taxonomy,
About, Spotlight Apps, Notification Center, Control Center and Finder
geometry). T230a maps it into the Dragonfruit manifest; the macOS screenshots
stay in that repository's gitignored `captures/` directory.

Every artifact has an entry in a manifest:

```json
{
  "id": "S-DOCK-05",
  "kind": "still",
  "path": "captures/experience/dock/hover-magnified__light__1x.png",
  "surface": "dock",
  "state": "hover-magnified",
  "axes": {
    "appearance": "light",
    "scale": "1x",
    "refresh": "120",
    "reduceTransparency": false,
    "increaseContrast": false,
    "reduceMotion": false,
    "display": "single",
    "locale": "en-US"
  },
  "trigger": "uictl move 900 1120 12; wait 700ms",
  "build": "27.0.1/26A434",
  "capturedAt": "2026-11-06T12:00:00-08:00",
  "script": "tools/dock_sweep.py --state hover-magnified",
  "hash": "sha256:…",
  "supersedes": null,
  "notes": ""
}
```

Rules:

- The manifest is append-only; a re-capture adds a new entry.
- `path` is relative to the reference repository root.
- `hash` is the SHA-256 of the source file.
- `trigger` is reproducible; a script path is preferred.
- `axes` uses the fixed vocabulary from the capture plan.
- `OPEN` rows are first-class entries with `"status": "open"` and a `reason`.

## 4. Measurement pipeline

### 4.1 Stills

1. Load the PNG with its embedded ICC profile.
2. Convert to sRGB for comparison; keep the original.
3. Crop the measured region; exclude text/icons where the metric is material.
4. Fit the material model (alpha/tint/blur/rim/lens/shadow).
5. Measure geometry (corner radius, padding, row height, hit target).
6. Record the result as JSON with residuals and sample counts.

Existing scripts: `analysis/measure.py`, `analysis/coverage_measure.py`,
`analysis/render_reference.py`, `analysis/compare.py`. The expanded campaign
adds `tools/experience_measure.py` for non-material surfaces.

### 4.2 Motion

1. Extract frames with timestamps (`ffmpeg -vsync 0`, `ffprobe`).
2. Identify the trigger frame and the first/last visible change.
3. Track a known element (edge, centroid, bounding box).
4. Fit duration, curve, overshoot, and settle.
5. Record dropped/duplicated frames; discard runs with >5% loss.
6. Repeat at 60/120 Hz where possible; record the resolution.

Output:

```json
{
  "id": "A-DOCK-01",
  "durationMs": 135,
  "curve": "smoothstep",
  "overshoot": 0.0,
  "triggerToFirstChangeMs": 8.3,
  "frames": 17,
  "fps": 120,
  "fitResidual": 0.012,
  "reducedMotion": {"durationMs": 0, "stateChange": "immediate"}
}
```

### 4.3 Behaviour

- Headless tests pin state transitions, input routing, and failure paths.
- A trace records the event sequence with timestamps.
- A behaviour claim without a test is design-only, not implemented.

### 4.4 Host services

- Adapter tests use a mock service and a real-service integration test where
  available.
- Absence tests stop/unown the service and assert the pane/tile state.
- Distro adapter tests run in the distro VM.

## 5. Acceptance gates

### 5.1 Per task

A task is agent-done when:

- [ ] The scoped behaviour is implemented.
- [ ] The headless tests are in `make e2e` and green.
- [ ] `make check`/`make soak` are unchanged.
- [ ] The capture/measurement artifact is committed or an `OPEN` row exists.
- [ ] The task's `## Hand-off` records what landed, what deviated, and what the
      next task must know.
- [ ] Docs touched in the same change.
- [ ] No silently deferred user-facing half.

### 5.2 Per surface

A surface is accepted when:

- [ ] Every state in its inventory row has a still or motion artifact.
- [ ] Geometry/material/motion is within the tolerance.
- [ ] Keyboard and AT-SPI paths are tested.
- [ ] Light/dark, reduced motion, reduce transparency, increase contrast are
      tested.
- [ ] 1×/1.5×/2× are tested where the surface renders.
- [ ] Multi-display behaviour is tested or explicitly `OPEN`.
- [ ] Performance is within the per-surface budget.
- [ ] The absence path is tested.

### 5.3 Per release

A release is accepted when:

- [ ] All required tasks are agent-done.
- [ ] The regression matrix passes on each supported distro.
- [ ] Native evidence exists for the supported hardware boundary.
- [ ] `OPEN` rows are listed with a rerun command and an owner.
- [ ] The human sign-off has watched the release demo and compared it to the
      design docs and capture goldens.
- [ ] `STATUS.md`, `ROADMAP.md`, and the design docs are reconciled.

## 6. Tolerances

Starting points; refine in the implementing task. The resolved per-surface
contract is `docs/captures/experience-thresholds.json` (profiles plus
per-surface overrides); this table is the source for those values. Tighten a
value with a design note; never silently loosen one.

| Dimension | Tolerance |
|---|---|
| Material flat region | mean abs sRGB < 0.02, p95 < 0.05, SSIM > 0.90 |
| Material checker | MTF within ±0.05 per frequency |
| Rim/lens | peak ±5%, half-height ±0.5 pt, displacement ±2 pt |
| Geometry | ±1 pt for radius/padding/row height; hit targets ≥ 24×24 pt |
| Window chrome | height 0/32/40/48/52/66/88 pt ±1 pt; traffic lights 16 pt AX frame / 14 pt drawn circle ±1 pt; 8/18 pt insets ±1 pt; 23 pt spacing ±1 pt |
| Structural surfaces | About 280×503/740×820 pt ±1 pt; drawer 7 columns/115×92 pt cell/77 pt icon; Notification Center 344 pt width; Control Center 64 pt cells/12 pt gaps/292 pt field; Finder 52/198/28/30/28 pt bars and Get Info 328×1034 pt |
| Typography | exact size/weight/line height; truncation matches |
| Motion duration | ±10% or ±1 frame, whichever is larger |
| Motion curve | fit residual < 5% of travel; no unmeasured overshoot |
| Input latency | within one frame of reference |
| Reduced motion | no unmeasured animation; state change immediate |
| Contrast | WCAG AA for text/controls; visible focus |
| Performance | per-surface budget; p95/p99 reported |

The structural review values are machine-readable in
`docs/captures/experience-thresholds.json` under `N-WIN`, `SET-GENERAL`,
`Q-AD`, `S-NC`, `S-CC`, and `N-FILES`. `N-FILES` is the Files window; `N-PREV`
remains the Quick Look overlay. The 14 review `OPEN` ids (`OPEN-CHROME-02`,
`OPEN-CHROME-03`, `OPEN-ABOUT-01`, `OPEN-ABOUT-02`, `OPEN-DRAWER-01`,
`OPEN-DRAWER-02`, `OPEN-NC-01`, `OPEN-NC-02`, `OPEN-CC-01`, `OPEN-CC-02`,
`OPEN-CC-03`, `OPEN-FINDER-01`, `OPEN-FINDER-02`, `OPEN-FINDER-03`) are
first-class manifest rows. Each names the missing evidence, the exact rerun
command, the owner task, and the impact in its `reason`; a later task replaces
it with a capture/test or a narrower `OPEN` row.

## 7. Evidence repository layout

In the macOS reference repo:

```text
captures/experience/
  manifest.json
  <surface>/...
analysis/experience/
  <surface>.json
docs/experience/
  <surface>.md
  OPEN.md
```

In Dragonfruit:

```text
docs/captures/experience/
  <task-id>-<surface>-<state>.png
  <task-id>-<surface>-<state>.txt
docs/design/
  ...design docs...
```

Dragonfruit captures are **our implementation**, not macOS reference images.
Never commit Apple screenshots to Dragonfruit.

## 8. Native vs nested

| Evidence | What it proves | What it does not prove |
|---|---|---|
| Nested capture | QML/layout/motion logic | DRM path, HiDPI sharpness, cursor |
| Headless test | state machine, input routing | visual quality |
| DRM readback | native pixels on the tested GPU | other GPUs, HDR, multi-GPU |
| External photo | cursor/overlay planes | exact colour |

Label every artifact with its class. Do not use a nested capture to claim a
native result.

## 9. Review process

### 9.1 Agent review

1. Run the task's tests.
2. Compare the capture to the reference measurement.
3. Check the `OPEN` rows and the hand-off.
4. Update the task checkboxes only then.

### 9.2 Human review (batched)

1. Watch the release/track demo in a live nested session.
2. Compare against the design docs and the capture goldens.
3. Perform the keyboard-only and screen-reader spot checks.
4. Tick the track acceptance boxes.
5. Record the verdict in `docs/design/STATUS.md` or the release report.

A green test suite is necessary but not sufficient. The canonical failure this
process prevents: a Dock with passing tests that nobody can use because the
click tree is wrong.

## 10. Open rows

An `OPEN` row is required when:

- the hardware is unavailable (second display, fingerprint reader, keyboard
  backlight, HDR, VRR);
- the macOS build does not expose the surface (Siri/Spotlight merge, Stage
  Manager onboarding);
- a third-party service is not installed;
- a native run cannot be performed on the current seat.

An `OPEN` row must include:

- the exact missing capability;
- the command to reproduce;
- the owner;
- the impact (which task/surface is blocked);
- a fallback used in the meantime.

Do not mark a task done with an unrecorded `OPEN` row.

## 11. CI

- **Headless:** Rust tests, QML tests, compositor conformance, state machines,
  input routing, absence tests.
- **Golden:** design-system gallery, material comparisons, geometry, motion
  traces where deterministic.
- **Distro:** build on Fedora/Ubuntu/Debian/Arch; install/smoke on VMs.
- **Native:** separate hardware rail; not required for every PR.
- **Docs:** link check, ADR uniqueness, `make check-naming` (user-facing
  strings, desktop entries, packaging metadata), and `make check-assets`
  (asset provenance). Both are in `make gates`.
- **Experience contract:** `make experience-schema-check` validates the
  surface/state/axis schema, the full capture/test/OPEN mapping, capture
  hashes, and the per-surface thresholds; it prints the coverage summary.

The CI must not require a Mac or a GPU for the ordinary test suite.

## 12. References

- `macos-design-review/docs/CAPTURE-PLAN.md`
- `macos-design-review/docs/SURFACE-INVENTORY.md`
- `macos-design-review/docs/SURFACE-REVIEW.md`
- [macOS structural surface review](23-macos-structural-surface-review.md)
- [Motion/feedback](motion-and-feedback.md)
- [Total experience](22-total-experience.md)
- [Existing STATUS](STATUS.md)
- [Existing roadmap](../ROADMAP.md)
