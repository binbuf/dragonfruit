# v0.1.1 — Presentation foundation tasks

This directory is the v0.1.1 task set. It continues the repository-wide task
number sequence at **T183** and implements
[Track 21 — Presentation foundation](../../design/tracks/21-presentation-foundation.md).

The release is deliberately narrow: **fix the presentation path before adding
more UI surface area.** The compositor/material architecture stays; the shell
transport, refresh scheduling, material cost/floor, and native evidence are the
deliverables.

## Order

| Task | File | Purpose | Hardware |
|---|---|---|---|
| T183 | [183-t-21.1a-chrome-transport-baseline.md](183-t-21.1a-chrome-transport-baseline.md) | Measure the current offscreen `grabWindow`/`wl_shm` path and record the transport decision. | optional |
| T184 | [184-t-21.1b-chrome-hidpi-and-fractional-scale.md](184-t-21.1b-chrome-hidpi-and-fractional-scale.md) | Render chrome at physical resolution; correct buffer scale/viewport at 1x/1.5x/2x. | no |
| T185 | [185-t-21.1c-chrome-shm-transport-hygiene.md](185-t-21.1c-chrome-shm-transport-hygiene.md) | Remove the double render and per-frame SHM allocation; consume frame callbacks. | no |
| T186 | [186-t-21.1d-gpu-backed-chrome-transport.md](186-t-21.1d-gpu-backed-chrome-transport.md) | GPU-render chrome and commit dmabuf buffers; keep pooled SHM as fallback. | optional |
| T187 | [187-t-21.2a-per-output-refresh-clock.md](187-t-21.2a-per-output-refresh-clock.md) | Replace the global 16 ms constant with a per-output refresh/deadline model. | no |
| T188 | [188-t-21.2b-presentation-paced-mixed-refresh.md](188-t-21.2b-presentation-paced-mixed-refresh.md) | Pace animation from actual presentation timing; mixed-refresh correctness. | yes for mixed |
| T189 | [189-t-21.2c-persistent-chrome-material-floor.md](189-t-21.2c-persistent-chrome-material-floor.md) | Keep Dock/menu-bar material under load; refresh-aware degrade hysteresis. | no |
| T190 | [190-t-21.3-panel-region-material-and-scanout.md](190-t-21.3-panel-region-material-and-scanout.md) | Panel-region scene texture, stable IDs, direct-scanout preservation. | optional |
| T191 | [191-t-21.4a-native-material-acceptance.md](191-t-21.4a-native-material-acceptance.md) | Accept the default-on native material on real DRM hardware. | yes |
| T192 | [192-t-21.4b-presentation-benchmark-and-soak.md](192-t-21.4b-presentation-benchmark-and-soak.md) | Measure p95/p99, missed presentations, copies, GPU time, and soak stability. | yes |
| T193 | [193-t-21.5-regression-matrix-and-closeout.md](193-t-21.5-regression-matrix-and-closeout.md) | Run the release matrix, update docs/STATUS, and close v0.1.1. | optional |

## Gates

1. **T183 is a gate.** No transport rewrite starts until the scene-graph backend
   and the shell's share of the frame budget are measured.
2. **T184 is a correctness gate.** Do not judge material sharpness before
   chrome is rendered at physical resolution.
3. **T185 is the fallback contract.** The GPU path in T186 must degrade to it,
   not to a black surface or a new renderer.
4. **T187/T188 are the scheduling gate.** A 144 Hz panel is judged against
   6.94 ms, not 16 ms; mixed refresh is tested per output.
5. **T191/T192 are the evidence gate.** Native visual proof and quantitative
   presentation numbers must be committed before T193 closes the release.
6. **No renderer switch.** Skia/Impeller/Flutter are out of scope. The question
   is whether Qt Quick + Smithay is configured and paced correctly.

## Evidence rules

- Report p50/p95/p99 and missed deadlines, not only averages.
- Record CPU copies/bytes, SHM vs dmabuf, GPU render time (or an explicit
  "timer query unavailable" note), idle wakeups, and memory/fd growth.
- Nested captures are reference evidence; they are never labeled native proof.
- If hardware is unavailable, write an explicit `OPEN` artifact naming the
  missing seat/GPU/panel and the exact command to rerun. Do not mark the task
  silently done.

## Commands

```bash
make gates                       # full gate: e2e + qml-test (run once on the final tree)
make e2e                         # required global verify
make qml-test                    # shell/QML tests
make t21-doctor                  # preflight a T-21 nested/host capture session
cargo test -p dragonfruit-compositor
make idle-trace                  # idle zero-wakeup evidence
make native-rebuild              # release session install/relogin
make native-capture              # native diagnostics snapshot
make native-perf                 # capture + client frame-rate probe
make check                       # lint + test + soak
```

New measurement targets added by this track (`make t21-chrome-baseline`,
`make t21-presentation-bench`) must be documented in the task that adds them
and must not be wired into `make e2e` if they need a host session or hardware.

## Harness

This directory is the task-file source for the dedicated `0.1.1` symphony task
set. The set's roadmap, progress notes and run logs live in
[`docs/v0.1.1/`](../../v0.1.1/ROADMAP.md); the task files stay here.

```bash
./.symphony/symphony doctor --set 0.1.1   # preflight this release
./.symphony/symphony lint --set 0.1.1     # check the docs contract
./.symphony/symphony status --set 0.1.1   # progress for this release
./.symphony/symphony run --set 0.1.1      # run T183…T193 in order
```

The harness uses `make e2e` as the independent verify command. Hardware-only
halves must be committed as explicit `OPEN` evidence, never accepted silently.
