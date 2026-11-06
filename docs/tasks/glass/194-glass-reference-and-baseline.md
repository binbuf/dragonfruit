---
verify: make gates
---
# T194 — Pin reference evidence and capture a baseline

## Goal

Create a repeatable, hardware-independent comparison workflow for compositor output and record the starting Dragonfruit appearance and timing.

## Context (read first)

- [Workstation evidence and integration map](plan/REFERENCE.md) records the pinned source, known contradictions, and surface coverage.
- In the reference checkout, read `analysis/{measure,coverage_measure,glasslib,render_reference,compare}.py` and `tools/build_material_schema.py`.
- `scripts/{capture-t20-liquid-glass.sh,t20-material-compare.py,t21-presentation-bench.sh}`, `docs/captures/README.md`, and `docs/testing-ladder.md` provide the existing capture and comparison entry points.

## Scope

- [ ] Record a small manifest of exact source PNGs, layout/AX dumps, JSON sections, analysis scripts, crops, ICC assumptions and hashes. Document a reference root override and fail clearly on missing or mismatched fixtures when a reference run is requested.
- [ ] Extend the existing comparison tooling for logical coordinate registration, ICC conversion, and per-metric masks. Add small synthetic deterministic fixtures for neutral, checker, line grid, and saturated tests. Store derived targets and source provenance with the tests.
- [ ] Record baseline captures for Dock, bar, at least one attached and detached menu, Control Center, and banner in both schemes where a baseline exists. Record the unmapped Notification Center and missing settings as baseline gaps. Capture a stationary backdrop and a separate moving test window, and record frame, region, allocation and transport counters for later comparison.

## Out of scope

- Rendering, token and settings changes. A separate native performance run is T208.

## Design notes

Keep the full reference checkout optional. Use its PNGs locally and record hashes and derived measurements. Register before scoring and bound the registration so alignment cannot hide a geometry regression. Exclude labels, icons and unrelated windows from material bands. Score the interior, rim, shadow, geometry, MTF and timing independently. Report zero or missing coverage per case explicitly. Verify no synthetic capture hooks activate in a regular session.

## Done when

- [ ] An automated self-test demonstrates detection of intentional colour, crop/geometry, MTF and motion regressions and runs through a documented command.
- [ ] Baseline and manifest identify backend, scale, refresh, applied settings and material/transport state. Record exact commands and any host capture that could not run.
- [ ] `make gates` passes. Document new commands in `docs/captures/README.md` and fill in Hand-off.

## Hand-off

### Changes

- Reference evidence: `docs/captures/glass-reference-manifest.json` (pinned
  revision, 15 source files, 33 JSON sections, 6 analysis scripts, 13 crops,
  ICC assumptions, hashes, 8 gaps). Tool:
  `scripts/t194-reference-manifest.py` (`--write`, `--verify`,
  `--check-manifest`). Root override: `--reference-root` →
  `DRAGONFRUIT_REFERENCE_ROOT` → sibling `../macos-liquid-glass`.
- Comparison: `scripts/glass_compare.py` + `scripts/t194-glass-compare.py`
  (ICC-aware sRGB, bounded logical registration, per-metric masks, independent
  colour/geometry/MTF/motion/timing scores, explicit missing coverage).
- Fixtures/self-test: `tests/glass/` with neutral, checker, line-grid and
  saturated fixtures, a motion pair, specs, `targets.json`, `provenance.json`,
  and `run_selftest.py` (120 checks).
- Baseline: `docs/captures/t194-glass-baseline.{json,md}`, twelve surface
  crops (`t194-glass-baseline-{dock,bar,menu_attached,menu_detached,
  control_center,banner}-{light,dark}.png`), stationary/moving backdrop
  evidence (including the native `t21-native-{dark,light}-live-under-dock-2x`
  moving test panel), T-21 counters, and `t194-glass-baseline.open.txt` with
  the probe logs.
- Runner: `scripts/capture-t194-glass-baseline.sh` and the Make targets
  `glass-selftest`, `glass-reference-manifest`, `glass-reference-verify`,
  `glass-baseline-capture`, `glass-baseline-check`. `make gates` now runs
  `glass-selftest`.
- Docs: `docs/captures/README.md` and
  `docs/design/adr/0213-glass-reference-manifest-and-comparison.md`.
- Progress note: `docs/tasks/glass/plan/progress/T194.md`.

### Checks

- `make glass-selftest` → 120 passed, 0 failed.
- `make glass-baseline-check` → PASS (12 surface crops, 6 gaps).
- `make glass-reference-verify` → PASS at
  `063a473cf06d0dba5cb83bd56c40b6fc88df2931`.
- Missing-root probe → exit 2 (`reference root is not a directory`); injected
  bad source hash → exit 2 (`REFERENCE FIXTURE MISMATCH`, source hash
  mismatch).
- `./scripts/check-no-capture-grab.sh` → PASS.
- `make gates` → PASS (glass-selftest, e2e, 83/83 qml tests, 384 s).

### Deviations / gaps

- Live host capture could not run in this session: `spectacle` exits non-zero
  with no PNG, and the nested compositor does not get its Wayland socket
  within 10 s (the host did not map the nested window). The exact probes and
  logs are in `t194-glass-baseline.open.txt`,
  `t194-glass-baseline-spectacle-probe.log` and
  `t194-glass-baseline-nested-probe.log`. The baseline is therefore recorded
  from the committed v0.1.1 `t20-chrome-*` stills, the T-21 benchmark, and the
  native live-content stills that move an override-redirect test panel behind
  the chrome. A fresh live recapture remains OPEN; the moving test panel
  itself is recorded.
- Notification Center is unmapped; `appearance.transparency`,
  `accessibility.reduceTransparency`, and `accessibility.increaseContrast` are
  absent from the settingsd schema; dark banner/lower-panel reference fits
  remain missing. These are explicit baseline gaps.
- Unrelated working-tree changes in `shell/src/shellcontroller.{cpp,h}` and
  `shell/src/shellprotocol.cpp` were present during this session and were not
  made or reverted by T194.

### Remaining work

- Rerun `make glass-baseline-capture` on a live host to replace the OPEN row
  with fresh stills. No T194 blocker remains for the hardware-independent
  workflow.
