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
- [ ] T184 — T-21.1b Chrome HiDPI and fractional scale → [../tasks/0.1.1/184-t-21.1b-chrome-hidpi-and-fractional-scale.md](../tasks/0.1.1/184-t-21.1b-chrome-hidpi-and-fractional-scale.md)
- [ ] T185 — T-21.1c Chrome SHM transport hygiene and frame pacing → [../tasks/0.1.1/185-t-21.1c-chrome-shm-transport-hygiene.md](../tasks/0.1.1/185-t-21.1c-chrome-shm-transport-hygiene.md)
- [ ] T186 — T-21.1d GPU-backed chrome transport → [../tasks/0.1.1/186-t-21.1d-gpu-backed-chrome-transport.md](../tasks/0.1.1/186-t-21.1d-gpu-backed-chrome-transport.md)

## Phase 21.2 — Refresh-aware scheduling and material floor

- [ ] T187 — T-21.2a Per-output refresh model and animation clock → [../tasks/0.1.1/187-t-21.2a-per-output-refresh-clock.md](../tasks/0.1.1/187-t-21.2a-per-output-refresh-clock.md)
- [ ] T188 — T-21.2b Presentation-paced mixed-refresh scheduling → [../tasks/0.1.1/188-t-21.2b-presentation-paced-mixed-refresh.md](../tasks/0.1.1/188-t-21.2b-presentation-paced-mixed-refresh.md)
- [ ] T189 — T-21.2c Persistent-chrome material floor and degrade → [../tasks/0.1.1/189-t-21.2c-persistent-chrome-material-floor.md](../tasks/0.1.1/189-t-21.2c-persistent-chrome-material-floor.md)

## Phase 21.3 — Panel-region material

- [ ] T190 — T-21.3 Panel-region material and direct-scanout preservation → [../tasks/0.1.1/190-t-21.3-panel-region-material-and-scanout.md](../tasks/0.1.1/190-t-21.3-panel-region-material-and-scanout.md)

## Phase 21.4 — Native evidence and benchmarks

- [ ] T191 — T-21.4a Native DRM material acceptance → [../tasks/0.1.1/191-t-21.4a-native-material-acceptance.md](../tasks/0.1.1/191-t-21.4a-native-material-acceptance.md)
- [ ] T192 — T-21.4b Presentation benchmark and soak → [../tasks/0.1.1/192-t-21.4b-presentation-benchmark-and-soak.md](../tasks/0.1.1/192-t-21.4b-presentation-benchmark-and-soak.md)

## Phase 21.5 — Release close-out

- [ ] T193 — T-21.5 Regression matrix and v0.1.1 close-out → [../tasks/0.1.1/193-t-21.5-regression-matrix-and-closeout.md](../tasks/0.1.1/193-t-21.5-regression-matrix-and-closeout.md)

## Release definition of done

A unit is agent-done when `make e2e` passes, `make check` and `make soak` are
unchanged, the task's capture/evidence artifact exists, any hardware-dependent
half is explicitly `OPEN`, docs and design references are updated, and the
task's `## Hand-off` is filled in. Human sign-off remains batched at the release
boundary and is not part of an agent task.

<!-- symphony:status -->
**Pipeline status** — updated 2026-11-03T22:42:18Z · 1/11 done

- Completed: T183
- Blocked: none
- Failed: none
- Remaining: T184, T185, T186, T187, T188, T189, T190, T191, T192, T193
- Last finished: T183 — done · Chrome transport baseline, opt-in shell/compositor counters, report, ADR 0191, and live stills landed; qml-test and e2e pass.
<!-- /symphony:status -->
