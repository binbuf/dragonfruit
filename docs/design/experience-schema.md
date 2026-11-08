# Experience schema and acceptance thresholds

| | |
|---|---|
| **Status** | implemented contract (T219) |
| **Date** | 2026-11-07 |
| **Release** | v0.2.0 |
| **Scope** | every surface id, state, axis, evidence class, trigger, acceptance metric, and per-surface threshold used by the total-experience track |
| **Related** | [evidence and acceptance](evidence-and-acceptance.md), [total experience](22-total-experience.md), [motion and feedback](motion-and-feedback.md), [T260](../tasks/0.2.0/260-t-22.8-acceptance-harness.md), [ADR 0228](adr/0228-experience-schema-and-thresholds.md) |

## 1. Why this contract exists

The total-experience track has hundreds of surface states, captures, tests, and
`OPEN` rows. Without one machine-readable contract, every task invents its own
id, axis vocabulary, tolerance, and definition of "captured", and the
acceptance harness in T260 has nothing stable to read.

This document is the human-readable contract. The machine-readable files are:

| File | Purpose |
|---|---|
| `docs/captures/experience-schema.json` | JSON Schema (draft 2020-12) for a manifest entry: surface id, state, axes, evidence class, trigger, acceptance metric, verification, status. |
| `docs/captures/experience-manifest.json` | The inventory and the mapping for every surface state to a Dragonfruit capture, an automated test, or an explicit `OPEN` row. |
| `docs/captures/experience-thresholds.json` | The per-surface material, geometry, typography, motion, latency, contrast, and performance thresholds, as reusable profiles plus per-surface overrides. |
| `scripts/t219-experience-schema.py` | The validator and coverage reporter. |
| `tests/experience/test_experience_schema.py` | Regression tests that prove the validator rejects missing axes, unknown ids, bad hashes, malformed `OPEN` rows, and dropped thresholds. |

The schema is an **evidence/CI contract**, not a runtime dependency. No
shipping binary reads these files; no macOS-only field enters the Dragonfruit
runtime.

## 2. Commands

```sh
make experience-schema-check      # validate + print coverage (part of make gates)
make experience-schema-coverage   # coverage summary only
make experience-schema-open       # every OPEN row with reason, rerun, owner
python3 scripts/t219-experience-schema.py --validate-artifact PATH
python3 scripts/t219-experience-schema.py --json
```

`make experience-schema-check` is hardware-independent: it needs Python 3 and
the checked-in captures/tests, not a Mac, a GPU, or a nested session.

## 3. Surface inventory and ids

Ids are stable and append-only. A later task may add a new id; it must not
rename one that another task cites. A state is addressed as `<surface>/<state>`
and has a row id `<surface>-NN` (for example `S-DOCK-13`).

| Family | Surfaces | State rows |
|---|---|---:|
| Always-visible shell | `S-MB`, `S-SYS`, `S-APP`, `S-ST`, `S-DOCK`, `S-DESK`, `S-CC`, `S-NC` | 107 |
| Navigation and windows | `N-MC`, `N-SW`, `N-WIN`, `N-STAGE`, `N-PREV`, `N-LOCK` | 71 |
| Search and launching | `Q-SP`, `Q-AD`, `Q-AP` | 30 |
| Menus and popovers | `S-MENU`, `S-POP`, `S-SHEET`, `S-DLG` | 24 |
| HUDs and transients | `H-VOL` … `H-PRIV` (14 HUDs) | 42 |
| System Settings | 23 shipped pane surfaces, 13 adapted/omitted panes, `SET-OMIT`, `SET-SEARCH`, `SET-AUTH` | 99 |
| Motion | `A-MENU` … `A-DRAWER` (14 transition groups) | 45 |

The full state list is in the manifest. The inventory source is
[`docs/reference/desktop-experience-inventory.md`](../reference/desktop-experience-inventory.md);
the reference campaign's `S-*`, `N-*`, `Q-*`, `H-*`, `SET-*`, and `A-*`
prefixes are preserved. The optional macOS reference checkout
(`macos-design-review`) was not present in this tree when T219 landed;
the local inventory is the id source and the reference checkout remains
optional evidence, not a build dependency.

## 4. Entry schema

Every state row has this shape:

```json
{
  "id": "S-DOCK-13",
  "surface": "S-DOCK",
  "state": "hover-magnified",
  "summary": "Hover magnification",
  "evidence_class": "still",
  "axes": "light-1x",
  "trigger": "bash scripts/capture-dock-magnify.sh",
  "acceptance": {
    "dimensions": ["material", "geometry", "typography", "contrast"],
    "metric": "Hover magnification matches the spatial profile and the 135 ms envelope."
  },
  "verification": {
    "kind": "capture",
    "path": "docs/captures/t14-dock-magnify-center-light.png",
    "sha256": "f818…",
    "command": "bash scripts/capture-dock-magnify.sh"
  },
  "status": "captured"
}
```

Field rules:

| Field | Rule |
|---|---|
| `id` | `<surface>-NN`, unique, stable. |
| `surface` | Must exist in the manifest's `surfaces` list. |
| `state` | Must exist in that surface's `states` list. |
| `axes` | An axis-set id (or an inline axis object) using the vocabulary in §5. |
| `evidence_class` | One of `still`, `motion`, `ax`, `geometry`, `measurement`, `behaviour`, `host_api`, `native_proof`. |
| `trigger` | Reproducible command or input path; never empty. For a test row it is the test command; for an `OPEN` row it is the rerun command. |
| `acceptance.metric` | The falsifiable claim, in words. |
| `acceptance.dimensions` | One or more of the seven threshold dimensions; each must be applicable for that surface. |
| `verification` | `capture` (path + SHA-256 + command), `test` (test path + command), or `open` (reason + rerun + owner). |
| `status` | `captured`, `verified`, or `open`; must match `verification.kind`. |

The JSON Schema enforces the shape. The validator additionally enforces the
semantic rules the schema cannot express: unique ids, surface/state
references, the axis vocabulary, the threshold dimensions, capture hashes,
test paths, and `OPEN` completeness.

## 5. Axes vocabulary

Every axis set carries all eight required axes. `backend` is optional.

| Axis | Values |
|---|---|
| `appearance` | `light`, `dark`, `light+dark` |
| `scale` | `1x`, `1.5x`, `2x`, `1x+1.5x+2x`, `n/a` |
| `refresh` | `60`, `120`, `144`, `60+120+144`, `n/a` |
| `reduceMotion` | `true`, `false`, `both` |
| `reduceTransparency` | `true`, `false`, `both` |
| `increaseContrast` | `true`, `false`, `both` |
| `display` | `single`, `multi`, `single+multi`, `n/a` |
| `locale` | `en-US`, `es-ES`, `en-US+es-ES`, `n/a` |
| `backend` | `nested`, `native`, `headless`, `any` |

An axis set is a named combination (`light-1x`, `dark-1x`, `light-dark-1x`,
`reduced-motion`, `reduced-transparency`, `increase-contrast`,
`multi-display`, `motion-120`, `behaviour`, `open-intended`, …). A row that
covers both schemes or both booleans uses `light+dark`/`both`; a row that is
not display- or refresh-specific uses `n/a`. `open-intended` records the axes
the future evidence must cover, so an `OPEN` row is not an excuse to forget
the variants.

## 6. Evidence classes and verification

| Class | What it can prove | Typical verification |
|---|---|---|
| `still` | Material, geometry, typography, contrast at a settled state | Capture PNG + hash |
| `motion` | Duration, curve, overshoot, interruption | Capture clip/trace |
| `ax` | Labels, roles, order, frames | AX dump |
| `geometry` | Tile/row/hit-target geometry | AX or edge measurement |
| `measurement` | Fitted material/timing value | Analysis JSON |
| `behaviour` | State transition, input routing, failure path | Headless test or trace |
| `host_api` | D-Bus/host-service call sequence | Adapter test |
| `native_proof` | DRM scanout pixels | Native readback |

A nested capture is reference evidence, not native proof. A test without a
capture is behaviour evidence, not a visual match. The `evidence_class` and
`verification.kind` pair must be honest: a still cannot verify a motion
threshold, and a material report cannot verify a transition duration.

## 7. Acceptance thresholds

Thresholds are starting points from the reference campaign. A later task may
**tighten** a value with a design note; it must not silently **loosen** one.
The source table is [evidence-and-acceptance §6](evidence-and-acceptance.md#6-tolerances).

### 7.1 Profiles

| Profile | Used by | Material | Geometry | Typography | Motion | Latency | Contrast | Performance |
|---|---|---|---|---|---|---|---|---|
| `glass-persistent` | Menu bar, Dock | flat mean ≤ 0.02, p95 ≤ 0.05, SSIM ≥ 0.90; MTF ±0.05; rim ±5% / 0.5 pt / 2 pt | ±1 pt; hit ≥ 24 pt | exact size/weight; line ±0.5 pt | ±10% or 1 frame; residual < 5%; no unmeasured overshoot | 1 frame; p95 8.3 ms, p99 16.7 ms | WCAG AA (4.5:1 text, 3:1 control); focus visible | p95 8.3 ms, p99 16.7 ms; 0 dropped; 8 ms gesture |
| `glass-transient` | Menus, popovers, Control Center, notifications, overlays, windows | same material tolerances | ±1 pt; hit ≥ 24 pt | exact | same | 1 frame; p95 16.7 ms, p99 33.3 ms | same | p95 16.7 ms, p99 33.3 ms; 0 dropped; 16 ms gesture |
| `opaque-content` | Desktop, Preview | flat mean ≤ 0.01, p95 ≤ 0.02, SSIM ≥ 0.95 | ±1 pt; hit ≥ 24 pt | exact | same | same | same | same as transient |
| `hud` | Every `H-*` | same material tolerances | ±1 pt; hit ≥ 24 pt | exact | same | same | same | same as transient |
| `settings-pane` | Every `SET-*` | same material tolerances | ±1 pt; row 44 pt; hit ≥ 24 pt | exact 13 pt | same | same | same | same as transient |
| `motion` | Every `A-*` | not applicable | not applicable | not applicable | same | same | not applicable | same as transient |

The per-surface overrides in `experience-thresholds.json` make the contract
concrete where the profile is not enough:

- `S-MB`: 13 pt type, 24 pt hit target, 8.3/16.7 ms latency and frame budget.
- `S-DOCK`: 48 pt icon row, 44 pt hit target, 16/33 ms frame budget, 16 ms
  gesture budget.
- `S-CC`, `S-NC`, `N-MC`, `N-SW`: 44 pt hit target; `N-MC` keeps the 16 ms
  gesture budget.
- `SET-*`: 44 pt row height, 24 pt hit target, 13 pt type.
- `A-*`: motion/latency/performance only; material, geometry, typography, and
  contrast are measured on the owning surface.
- `N-WIN` / structural surfaces (T230a): chrome height 0/32/40/48/52/66/88 pt
  ±1 pt; traffic lights 16×16 pt ±1 pt; 8/18 pt insets ±1 pt; 23 pt spacing
  ±1 pt; About panel 280×503 pt ±1 pt; drawer 7 columns/115×92 pt cell/77 pt
  icon; Notification Center 344 pt width and three material bands; Control
  Center 64 pt cells/12 pt gaps/292 pt field; Finder 52/198/28/30/28 pt bars
  and Get Info 328×1034 pt.

### 7.3 Structural surface states (2026-11-08 review)

T230a extends the manifest with the structural review states. The new rows
cover:

| Family | New states |
|---|---|
| `N-WIN` | chrome styles 0/32/40/48/52/66/88, traffic-light geometry, accessory bars, fullscreen hidden/revealed, long title/RTL |
| `SET-GENERAL` / About | compact 280×503 pt panel, full 740×820 pt page, More Info transition, long/localized names |
| `Q-AD` | Spotlight Apps grid, list, search, category filter, suggestions, empty/no-results, launch |
| `S-NC` | 344 pt panel, banner stack, widget stack, gallery, empty/actions/Focus, motion |
| `S-CC` | 4-column grid, module spans, expanded sheets, Edit Controls catalogue, pages/resize/pinning, motion |
| `N-PREV` / Files | Finder chrome, four view modes, Get Info inspector, toolbar-hidden mode |

The reference geometry is in
`macos-design-review/docs/experience_measurements.json`. A row without a
Dragonfruit capture/test stays `OPEN`; the reference value goes in the row's
summary/threshold, not in a fake `captured` status. The OPEN ids are listed in
[23-macos-structural-surface-review.md](23-macos-structural-surface-review.md)
§9.

### 7.2 Threshold dimensions

| Dimension | Required fields |
|---|---|
| `material` | `flat_region.mean_abs_srgb_max`, `flat_region.p95_abs_srgb_max`, `flat_region.ssim_min`, `checker_mtf.delta_max`, `rim.peak_rel_max`, `rim.half_height_pt_max`, `rim.displacement_pt_max` |
| `geometry` | `position_pt_max`, `radius_pt_max`, `padding_pt_max`, `row_height_pt_max`, `hit_target_pt_min` |
| `typography` | `size_pt`, `size_pt_exact`, `weight_exact`, `line_height_pt_max`, `truncation` |
| `motion` | `duration_rel_max`, `duration_frame_max`, `curve_residual_rel_max`, `overshoot_unmeasured`, `reduced_motion_state_change` |
| `latency` | `input_to_photon_frames_max`, `p95_ms_max`, `p99_ms_max` |
| `contrast` | `text_wcag`, `text_ratio_min`, `large_text_ratio_min`, `control_ratio_min`, `focus_visible`, `focus_ratio_min` |
| `performance` | `p95_frame_ms_max`, `p99_frame_ms_max`, `dropped_frames_max`, `gesture_budget_ms` |

A dimension may be `"applicable": false` only with a `reason`. A row may not
claim a dimension that its surface marks not applicable; the validator
rejects that.

## 8. Mapping rules

1. **Every state has a row.** The validator walks each surface's `states` and
   fails if any state has no entry.
2. **Captured** rows carry a real file and a matching SHA-256. The file may be
   a still, a clip, a trace, or an analysis JSON.
3. **Verified** rows carry a real test path and the command that runs it. The
   test must be in the tree; the validator checks the path.
4. **OPEN** rows are first-class. They must name the missing capability, the
   exact rerun command, the owner task, and the intended axes. An `OPEN` row
   is not a pass; it is a visible debt with a next step.
5. **One state can have more than one row** when the still and the motion are
   separate artifacts. The coverage summary counts every row.
6. **No invented values.** If a capture is missing, the row is `OPEN`; the
   threshold is not relaxed to make it pass.

## 9. Coverage summary

`make experience-schema-check` prints the captured/verified/OPEN counts for
every surface and the overall total. The T219 landing snapshot is:

```text
TOTAL   169 captured   71 verified   178 open   418 rows
Covered (captured + verified): 240/418 (57.4%)
```

The all-OPEN surfaces are the explicit debt list: Stage/tiling, Preview,
Search, Sheets, most HUDs, the unshipped/omitted Settings panes, and the
unmeasured Quick Settings/notification/HUD/Search/drawer motion groups. They
are the input to T222–T262, not a silent gap.

The manifest is the source of truth for the count; the snapshot above is a
convenience for reviewers. Run `make experience-schema-coverage` for the live
table.

## 10. CI and update procedure

`make gates` runs `make experience-schema-check` before `make e2e` and
`make qml-test`. The check is hardware-independent.

When a task lands a capture or test:

1. Add or update the state row in `experience-manifest.json`.
2. Set `verification.kind` to `capture` or `test`; add the path, hash, and
   command.
3. Keep the axes honest (`light+dark`, `1x+1.5x+2x`, `both`, …).
4. If the task cannot land the evidence, leave `status: "open"` with the exact
   reason, rerun command, and owner.
5. Run `make experience-schema-check`; the hash check will fail if the capture
   changed without the manifest.

When a task tightens a threshold:

1. Change `experience-thresholds.json` and record the reason in the task's
   hand-off or a design note.
2. Do not loosen a value without a design note; the contract policy is
   `loosen: forbidden`.
3. Run the check and the T260 harness.

## 11. Relationship to T260

T260 ([acceptance harness](../tasks/0.2.0/260-t-22.8-acceptance-harness.md))
consumes `docs/captures/experience-thresholds.json` as its tolerance source.
It must not duplicate the values. Its still, motion, geometry, and typography
checks report the metric, crop, tolerance, and diff image against these
profiles. A T260 tolerance change is a change to this contract.

## 12. References

- [Evidence and acceptance](evidence-and-acceptance.md)
- [Total desktop experience](22-total-experience.md)
- [Motion, HUDs, sound, and feedback](motion-and-feedback.md)
- [macOS structural surface review](23-macos-structural-surface-review.md)
- [Desktop experience inventory](../reference/desktop-experience-inventory.md)
- [T260 acceptance harness](../tasks/0.2.0/260-t-22.8-acceptance-harness.md)
- [ADR 0228](adr/0228-experience-schema-and-thresholds.md)
- [ADR 0241](adr/0241-macos-structural-surface-review.md)
