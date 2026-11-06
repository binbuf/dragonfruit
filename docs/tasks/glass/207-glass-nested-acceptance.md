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

- [ ] Run a repeatable nested comparison against the old rail using matched scenes, same logical geometry, settings and available output scales. Include stationary and moving backgrounds, with light/dark, slider, both accessibility overrides and Reduce Motion.
- [ ] Exercise 1×/1.5×/2×, software/headless, forced Full/Reduced/Minimal, SHM/dmabuf where supported, overlays and output change. Record safe fallback reason and effective preset. Check lock/fullscreen transitions, idle settle, service restarts, and layer teardown.
- [ ] Publish matched interior/rim/MTF/lens/timing scores and target deviations. For reliable registered flat bands, target mean |ΔsRGB| <0.02, p95 <0.05 and SSIM >0.90. Use surface-specific metrics for patterned or underconstrained bands. Report approved adaptations separately from missed measured criteria.
- [ ] Compare refresh-relative timing, padded area, cache misses and allocation growth, transport/copies, and idle behavior on the same host. Preserve the current release rail for an A/B and rollback check.

## Out of scope

- Native DRM sign-off and changing the default (T208). Extending the release to unmeasured surfaces.

## Design notes

Keep comparison masks and tolerance definitions checked in. Include surface, scheme, tier and backend in each result. Scores must derive from actual compositor output. Existing nested refresh overrides verify scheduling policy and do not establish the physical panel rate. Track release-baseline failures separately and investigate new regressions before sign-off. Run reusable hardware-independent checks through existing gates and keep optional host capture workflows separate.

## Done when

- [ ] The evidence table has pass/fail/adaptation/OPEN with a command and reason for every row. Required nested measured and interaction gates pass, including live setting changes and no recurring idle redraw.
- [ ] `make check-tokens`, `make visual-test` and `make gates` pass. Record fixture and report paths in `docs/captures/README.md` and fill in Hand-off.

## Hand-off

_(Implementation session records remaining native rows, commands and any release blockers.)_
