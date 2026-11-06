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

_(Implementation session records evidence paths, commands, discrepancies, and open rows.)_
