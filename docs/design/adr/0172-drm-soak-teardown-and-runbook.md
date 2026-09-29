# 0172 — The DRM teardown soak covers tokens, and the runbook owns the hardware rail

## Status

accepted

## Context

T-03.4 closes the T-03 hardware slice: prove the DRM session tears down cleanly
and document a reproducible run ([03-real-session-bringup-perf.md](../tracks/03-real-session-bringup-perf.md)).
Two facts shape the work:

1. The teardown soak (`dragonfruit dev --soak`) asserted only that no socket,
   lock, or orphaned process leaked. Since T-07/T-19.3 the compositor also
   writes trusted-role hand-off files (`<socket>.launch-token`,
   `<socket>.desktop-launch-token`); a leaked token is a leaked capability and
   was not checked.
2. Multi-GPU import/fallback was implemented but never validated: a secondary
   with a render node imports buffers, a secondary without one falls back to
   the primary renderer with linear-only import. This host has no free logind
   seat (the KDE Wayland session owns DRM master), so the DRM cycle and the
   multi-GPU run are *open, not skipped*.

## Decision

- **The soak gate owns all four leak classes.** `tools/dragonfruit-dev/src/soak.rs`
  gains a pure `teardown_artifacts` list — socket, lock, and both launch
  tokens — and `run` fails on any leaked artifact, alongside the existing stray
  process scan. `dragonfruit dev --soak [N]` defaults to the headless backend
  and accepts `--nested` / `--drm` so the same check runs on those backends.
- **One DRM cycle is scripted, not hand-waved.** `scripts/drm-soak.sh`
  (`make drm-soak`) runs the automated soak and then `dragonfruit dev --soak 1
  --drm`, re-probing the card with `scripts/drm-seat-probe.py` to prove DRM
  master (the VT) was released. With no free seat it writes
  `docs/captures/t03-drm-soak.open.txt` with the no-op-seat self-report; with a
  free seat, `t03-drm-soak.txt`.
- **The multi-GPU decision is pure and mirrored.** `compositor/src/multi_gpu.rs`
  defines `GpuNode` and `MultiGpuOutcome::{NoGpu, Single, Imported, Fallback}`
  with a one-line `marker()`, plus the `format_allowed` dmabuf rule. The backend
  prints the marker after its initial scan and filters formats through the same
  rule; `compositor/tests/multi_gpu.rs` (in `make e2e`) pins it without
  hardware, and `scripts/multi-gpu-probe.py` mirrors it from sysfs. A host with
  one card records `docs/captures/t03-multigpu.open.txt`; two or more cards
  record `t03-multigpu.txt`.
- **The runbook owns the hardware rail.** `docs/runbook-drm-session.md` is the
  one place with the dedicated-user second-VT workflow, the libvirt/virtio-GPU
  VM path, the four acceptance commands, the demo walkthrough, teardown
  expectations, and troubleshooting. `docs/testing-ladder.md` links it from
  rungs 2 and 3.

## Consequences

- A leaked launch token now fails `make soak` / `make check`, not just a leaked
  socket. Future trusted-role hand-off files must be added to
  `soak::teardown_artifacts` and `shell::remove_token_file`.
- On this host T-03.4 is **open**: the automated 100-cycle soak passes and the
  three hardware artifacts are explicit `.open.txt` records. A free seat /
  spare GPU / clean VM replaces them with the validated forms via the runbook.
- The `Multi-GPU:` marker is the stable line the probe greps; new multi-GPU
  shapes classify through `multi_gpu` and keep it one line.
- T-12/T-16/T-17 build on the runbook; suspend/resume, multi-monitor scaling,
  and NVIDIA stay with T-16.