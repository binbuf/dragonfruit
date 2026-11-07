---
verify: make check-tokens && make visual-test && make gates
---
# T207 — Verify the replacement in nested and fallback sessions

## Goal

Verify the integrated replacement against the pinned evidence and the existing behavior before hardware rollout.

## Context (read first)

- Depends on T194–T206. Use the manifest and baseline from T194 and the per-surface acceptance criteria.
- `docs/testing-ladder.md`, `docs/rendering.md`, `scripts/t21-presentation-bench.sh`, and `docs/v0.1.1/progress/T192b.md`.

## Scope

- [x] Run a repeatable nested comparison against the old rail using matched scenes, same logical geometry, settings and available output scales. Include stationary and moving backgrounds, with light/dark, slider, both accessibility overrides and Reduce Motion.
- [x] Exercise 1×/1.5×/2×, software/headless, forced Full/Reduced/Minimal, SHM/dmabuf where supported, overlays and output change. Record safe fallback reason and effective preset. Check lock/fullscreen transitions, idle settle, service restarts, and layer teardown.
- [x] Publish matched interior/rim/MTF/lens/timing scores and target deviations. For reliable registered flat bands, target mean |ΔsRGB| <0.02, p95 <0.05 and SSIM >0.90. Use surface-specific metrics for patterned or underconstrained bands. Report approved adaptations separately from missed measured criteria.
- [x] Compare refresh-relative timing, padded area, cache misses and allocation growth, transport/copies, and idle behavior on the same host. Preserve the current release rail for an A/B and rollback check.

## Out of scope

- Native DRM sign-off and changing the default (T208). Extending the release to unmeasured surfaces.

## Design notes

Keep comparison masks and tolerance definitions checked in. Include surface, scheme, tier and backend in each result. Scores must derive from actual compositor output. Existing nested refresh overrides verify scheduling policy and do not establish the physical panel rate. Track release-baseline failures separately and investigate new regressions before sign-off. Run reusable hardware-independent checks through existing gates and keep optional host capture workflows separate.

## Done when

- [x] The evidence table has pass/fail/adaptation/OPEN with a command and reason for every row. Required nested measured and interaction gates pass, including live setting changes and no recurring idle redraw.
- [x] `make check-tokens`, `make visual-test` and `make gates` pass. Record fixture and report paths in `docs/captures/README.md` and fill in Hand-off.

## Hand-off

### Changes

- `tests/glass/t207-cases.json` — 50-row matrix, flat-band mask and the
  tolerance contract (mean |ΔsRGB| < 0.02, p95 < 0.05, SSIM > 0.90).
- `scripts/t207-nested-acceptance.py` (`plan`/`score`/`check`) and
  `scripts/capture-t207-nested-acceptance.sh`; `scripts/t207-moving-panel.c`
  for the moving-backdrop rows.
- `scripts/t21-live-capture.sh` grew `lock-exit`, `fullscreen-exit`,
  `CAPTURE_DELAY`, `REDUCE_MOTION`, `PRE_CAPTURE_CMD`, configurable SIGUSR1
  snapshots, shell counters, `query lock`/`session`/`policy`/`scanout`, and a
  post-capture query. `shell/src/shellcontroller.cpp` retries the capture-only
  timed unlock until the lock protocol reports locked.
- `Makefile`: `nested-acceptance-check` is in `gates`;
  `nested-acceptance-capture` runs the live matrix.
- Evidence: `docs/captures/t207-nested-acceptance.{json,md}`,
  `t207-nested-acceptance-capture.json`, 50 compositor stills plus query and
  stats dumps, `t207-logs/`, and `t207-nested-acceptance.open.txt`.
- Docs: ADR 0226, `docs/captures/README.md`, `docs/rendering.md` R8.2,
  `docs/design/STATUS.md` T-20 row, progress note
  `docs/tasks/glass/plan/progress/T207.md`.

### Results

- 50 nested rows: 46 `pass`, 4 `adaptation`; 16/16 legacy/replacement A/B
  pairs pass. The adaptations are the legacy rail's expected reference miss
  (it is the old appearance), the 1.5x/2x legacy reference rows, and the
  offscreen/SHM dmabuf capture boundary (the dmabuf rail is the fresh
  `make t21-gpu-rollout` PASS).
- Dock flat band against the macOS `dock_interior` crop: replacement mean
  0.0053, p95 0.0039, SSIM 0.9495 at 1x; 0.0053/0.0039/0.9502 at 1.5x;
  0.0053/0.0039/0.9495 at 2x. Legacy mean 0.0862–0.0863, SSIM 0.9345–0.9372.
- Moving backdrop: the white override-redirect panel moves the Dock flat band
  from 0.5294 to 0.8776 (light, Δ0.3482) and 0.4589 to 0.8588 (dark,
  Δ0.3998).
- A/B p50 2.8–5.5 ms, replacement allocations 8 vs legacy 9–10, transport
  `render-control+shm` on both, idle rows 0 frames / 0 wakeups. The live
  output-change row reports scale 1.5.
- Per-surface reports: T198 1x light/dark/Reduce Transparency/Increase
  Contrast pass (1.5x/2x MTF is the approved nested scene-texture
  adaptation); T200 veil max LUT delta 0.0039; T202 light attached alpha
  +0.0020; T203 max outer/module fit 0.0227; T205 max banner/stack/lower fit
  0.0372; T206 max fit 0.0051 across 22 cases.
- External: headless demo PASS; settingsd kill/restart PASS; 10-cycle soak
  PASS; `make t21-gpu-rollout` PASS. The shipped default remains `legacy`.

### Checks

- `make nested-acceptance-capture` — 50/50 rows captured.
- `make nested-acceptance-check` — PASS.
- `make check-tokens` — current.
- `make visual-test` — 87 snapshots passed.
- `make gates` — PASS (nested acceptance, e2e, 84/84 ctest).
- `make demo DEMO_ARGS=--headless` — PASS.
- `cargo test -p dragonfruit-settingsd --test restart` — 1 passed.
- `make soak SOAK_CYCLES=10` — 10 clean cycles, zero strays.
- `make t21-gpu-rollout` — PASS.

### Remaining / blockers

- Native DRM, panel-mode, mixed-output and direct-scanout sign-off remain
  OPEN for T208; the exact rerun commands and evidence boundary are in
  `docs/captures/t207-nested-acceptance.open.txt`.
- The 1.5x/2x nested checker MTF gap and the controlled Dark recaptures
  (notification banner/lower panel, Settings chrome, attached File menu)
  remain approved adaptations/follow-ups; they do not block the nested
  acceptance.
- No release blocker for the nested rail. T208 still owns the default flip
  and the native rollback cycle.
