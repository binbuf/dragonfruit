# v0.1.1 Roadmap — T-21 Presentation foundation

This is the executable plan for the v0.1.1 release. It is a dedicated
symphony task set so the release can run independently of the completed
T-01…T-20 work in [`docs/ROADMAP.md`](../ROADMAP.md).

- Run with `./.symphony/symphony run --set 0.1.1`.
- Check readiness with `./.symphony/symphony doctor --set 0.1.1`.
- Inspect progress with `./.symphony/symphony status --set 0.1.1`.

The release is deliberately narrow: **fix the presentation path before adding
more UI surface area.** The compositor/material architecture stays; the shell
transport, refresh scheduling, material cost/floor, and native evidence are the
deliverables. Design reference:
[Track 21 — Presentation foundation](../design/tracks/21-presentation-foundation.md).

## Gates and evidence rules

- **Baseline gate** — the T183 measurement must land before any transport
  rewrite starts; record the scene-graph backend and the shell's share of the
  frame budget.
- **HiDPI gate** — do not judge material sharpness before T184 renders chrome
  at physical resolution.
- **Fallback contract** — the GPU path in T186 must degrade to the T185 pooled
  SHM path, not to a black surface or a new renderer.
- **Scheduling gate** — T187/T188 judge a 144 Hz panel against 6.94 ms, not
  16 ms; mixed refresh is tested per output.
- **Evidence gate** — T191/T192 must commit native visual proof and
  quantitative presentation numbers before T193 closes the release.
- **No renderer switch** — Skia/Impeller/Flutter are out of scope.

Every task reports p50/p95/p99 and missed deadlines, not only averages. Nested
captures are reference evidence; they are never labeled native proof. If
hardware is unavailable, write an explicit `OPEN` artifact naming the missing
seat/GPU/panel and the exact command to rerun; do not mark the task silently
done.

## Phase 21.1 — Chrome transport

- [x] T183 — T-21.1a Chrome transport baseline and decision → [../tasks/0.1.1/183-t-21.1a-chrome-transport-baseline.md](../tasks/0.1.1/183-t-21.1a-chrome-transport-baseline.md)
- [x] T184 — T-21.1b Chrome HiDPI and fractional scale → [../tasks/0.1.1/184-t-21.1b-chrome-hidpi-and-fractional-scale.md](../tasks/0.1.1/184-t-21.1b-chrome-hidpi-and-fractional-scale.md)
- [x] T185 — T-21.1c Chrome SHM transport hygiene and frame pacing → [../tasks/0.1.1/185-t-21.1c-chrome-shm-transport-hygiene.md](../tasks/0.1.1/185-t-21.1c-chrome-shm-transport-hygiene.md) ⟵ accepted
- [x] T185a1 — T-21.1c close-out verification → [../tasks/0.1.1/185a1-t-21.1c-closeout-verify.md](../tasks/0.1.1/185a1-t-21.1c-closeout-verify.md)
- [x] T186a — T-21.1d GPU transport: capability probe and render-control spike → [../tasks/0.1.1/186a-t-21.1d-gpu-probe.md](../tasks/0.1.1/186a-t-21.1d-gpu-probe.md)
- [x] T186b — T-21.1d GPU transport: dmabuf export and single-buffer Dock commit → [../tasks/0.1.1/186b-t-21.1d-dmabuf-export-dock.md](../tasks/0.1.1/186b-t-21.1d-dmabuf-export-dock.md)
- [x] T186c — T-21.1d GPU transport: dmabuf ring, sync, and fallback → [../tasks/0.1.1/186c-t-21.1d-dmabuf-ring-sync.md](../tasks/0.1.1/186c-t-21.1d-dmabuf-ring-sync.md)
- [x] T186d — T-21.1d GPU transport: rollout, stats, and docs → [../tasks/0.1.1/186d-t-21.1d-gpu-rollout.md](../tasks/0.1.1/186d-t-21.1d-gpu-rollout.md)

## Phase 21.2 — Refresh-aware scheduling and material floor

- [x] T187a — T-21.2a Per-output refresh model (pure) → [../tasks/0.1.1/187a-t-21.2a-refresh-model.md](../tasks/0.1.1/187a-t-21.2a-refresh-model.md)
- [x] T187b — T-21.2a Wire the refresh model into the clock and budget → [../tasks/0.1.1/187b-t-21.2a-refresh-clock-wiring.md](../tasks/0.1.1/187b-t-21.2a-refresh-clock-wiring.md)
- [x] T188a — T-21.2b Per-output presentation clock (pure) → [../tasks/0.1.1/188a-t-21.2b-presentation-clock.md](../tasks/0.1.1/188a-t-21.2b-presentation-clock.md)
- [x] T188b — T-21.2b Per-output frame requests and timer arming → [../tasks/0.1.1/188b-t-21.2b-frame-requests.md](../tasks/0.1.1/188b-t-21.2b-frame-requests.md)
- [x] T188c — T-21.2b VRR, chrome callbacks, and mixed-refresh evidence → [../tasks/0.1.1/188c-t-21.2b-vrr-mixed-refresh.md](../tasks/0.1.1/188c-t-21.2b-vrr-mixed-refresh.md)
- [x] T189a — T-21.2c Persistent/transient role classification and per-role state → [../tasks/0.1.1/189a-t-21.2c-role-classification.md](../tasks/0.1.1/189a-t-21.2c-role-classification.md)
- [x] T189b — T-21.2c Shell per-role application: Dock and menu bar → [../tasks/0.1.1/189b-t-21.2c-shell-role-application.md](../tasks/0.1.1/189b-t-21.2c-shell-role-application.md)
- [x] T189c — T-21.2c Overlay and popover fallback coverage → [../tasks/0.1.1/189c-t-21.2c-overlay-fallback-coverage.md](../tasks/0.1.1/189c-t-21.2c-overlay-fallback-coverage.md)
- [x] T189d — T-21.2c Refresh-aware hysteresis and load tests → [../tasks/0.1.1/189d-t-21.2c-refresh-aware-hysteresis.md](../tasks/0.1.1/189d-t-21.2c-refresh-aware-hysteresis.md)

## Phase 21.3 — Panel-region material

- [x] T190a — T-21.3 Pure MaterialRegion helper → [../tasks/0.1.1/190a-t-21.3-material-region.md](../tasks/0.1.1/190a-t-21.3-material-region.md)
- [x] T190b — T-21.3 Region-sized scene render and caching → [../tasks/0.1.1/190b-t-21.3-region-render.md](../tasks/0.1.1/190b-t-21.3-region-render.md)
- [x] T190c — T-21.3 Stable scene/panel IDs and damage → [../tasks/0.1.1/190c-t-21.3-stable-ids-damage.md](../tasks/0.1.1/190c-t-21.3-stable-ids-damage.md)
- [x] T190d — T-21.3 Direct-scanout preservation and material counters → [../tasks/0.1.1/190d-t-21.3-scanout-and-counters.md](../tasks/0.1.1/190d-t-21.3-scanout-and-counters.md)

## Phase 21.4 — Native evidence and benchmarks

- [x] T191a — T-21.4a Native rebuild, diagnostics, and rollback → [../tasks/0.1.1/191a-t-21.4a-native-diagnostics.md](../tasks/0.1.1/191a-t-21.4a-native-diagnostics.md)
- [x] T191b — T-21.4a Native visual sweep and evidence boundary → [../tasks/0.1.1/191b-t-21.4a-native-visual-sweep.md](../tasks/0.1.1/191b-t-21.4a-native-visual-sweep.md)
- [ ] T192a — T-21.4b GPU/chrome timing counters → [../tasks/0.1.1/192a-t-21.4b-timing-and-bench-harness.md](../tasks/0.1.1/192a-t-21.4b-timing-and-bench-harness.md)
- [ ] T192b — T-21.4b Benchmark script and nested scenario run → [../tasks/0.1.1/192b-t-21.4b-bench-harness.md](../tasks/0.1.1/192b-t-21.4b-bench-harness.md)
- [ ] T192c — T-21.4b Baseline comparison and native benchmark rows → [../tasks/0.1.1/192c-t-21.4b-baseline-and-native.md](../tasks/0.1.1/192c-t-21.4b-baseline-and-native.md)
- [ ] T192d — T-21.4b Presentation soak and final report → [../tasks/0.1.1/192d-t-21.4b-soak-and-report.md](../tasks/0.1.1/192d-t-21.4b-soak-and-report.md)

## Phase 21.5 — Release close-out

- [ ] T193a — T-21.5 Regression matrix: rendering and interactions → [../tasks/0.1.1/193a-t-21.5-regression-matrix.md](../tasks/0.1.1/193a-t-21.5-regression-matrix.md)
- [ ] T193b — T-21.5 Regression matrix: performance and compatibility → [../tasks/0.1.1/193b-t-21.5-performance-compatibility-matrix.md](../tasks/0.1.1/193b-t-21.5-performance-compatibility-matrix.md)
- [ ] T193c — T-21.5 Docs reconciliation and v0.1.1 close-out → [../tasks/0.1.1/193c-t-21.5-docs-and-closeout.md](../tasks/0.1.1/193c-t-21.5-docs-and-closeout.md)

## Release definition of done

A unit is agent-done when `make e2e` passes, `make check` and `make soak` are
unchanged, the task's capture/evidence artifact exists, any hardware-dependent
half is explicitly `OPEN`, docs and design references are updated, and the
task's `## Hand-off` is filled in. Human sign-off remains batched at the release
boundary and is not part of an agent task.

<!-- symphony:status -->
**Pipeline status** — updated 2026-11-05T17:15:00Z · 23/30 done

- Completed: T183, T184, T185, T185a1, T186a, T186b, T186c, T186d, T187a, T187b, T188a, T188b, T188c, T189a, T189b, T189c, T189d, T190a, T190b, T190c, T190d, T191a, T191b
- Blocked: none
- Failed: none
- Remaining: T192a, T192b, T192c, T192d, T193a, T193b, T193c
- Last finished: T191b — done · Captured 23 native DP-1 KMS-readback stills (light/dark Dock/menu over live content, menu, drawer, Control Center, OSD, notification, popover, magnification, Mission Control, lock, fullscreen) and recorded the evidence boundary; docs/rendering R6, STATUS G-03/G-08, captures README updated.
<!-- /symphony:status -->
