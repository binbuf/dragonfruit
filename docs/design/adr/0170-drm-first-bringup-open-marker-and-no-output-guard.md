# 0170 — DRM first bring-up reports a typed OPEN/READY marker and refuses to run with no output

## Status

accepted

## Context

T-03.2 runs the DRM/KMS backend for the first time. The backend was written but
never executed ([03-real-session-bringup-perf.md](../tracks/03-real-session-bringup-perf.md)),
and this host has **no free logind seat** — a KDE Wayland session owns DRM
master on `seat0` ([SLICING-REVIEW.md](../../SLICING-REVIEW.md)). The track's
rule is explicit: if hardware is unavailable the slice is *marked open, not
silently skipped*, and swept on the hardware rail.

Two things fell out of the first execution (via a no-op seat so the host
session is never disturbed):

1. `LibSeatSession::new()` failure and "opened the card but got no output" both
   surfaced as an opaque error, or worse — if every connector failed to
   initialize, the compositor could have entered its event loop with **zero
   outputs** and simply hung with no display and no diagnosis.
2. Nothing in the tree produced the explicit "open" record the track requires.

## Decision

- **One pure classification owns the decision.** `compositor/src/drm_bringup.rs`
  defines `DrmBringup::{Ready, Open}` over `DrmBringupBlocker::{NoSeat, NoGpu,
  NoOutput}` and a `classify(session, device, outputs)` constructor plus a
  single-line `marker()`. It is exposed through the `dragonfruit-compositor`
  library so `compositor/tests/drm_bringup.rs` (in `make e2e`) pins it without
  hardware, and the binary uses the same code.
- **The backend prints the marker and fails loudly.** `backend::drm::init`
  classifies at each of its three checkpoints — session acquisition, GPU
  resolution, and the initial connector scan — prints
  `DRM bring-up: READY device=<node> outputs=<n>` or
  `DRM bring-up: OPEN (<reason>)`, and returns an error when not ready. A
  session/GPU/device that produces no output is a bring-up failure, not a
  headless-ish success.
- **A script records the open, not the task.** `scripts/drm-bringup.sh`
  (`make drm-bringup`) probes cards with `libdrm` (`drmIsMaster` /
  `drmSetMaster`, released immediately — `scripts/drm-seat-probe.py`), starts
  the DRM backend when a card accepts master, and otherwise writes
  `docs/captures/t03-drm-loop.open.txt` with the probe evidence, the backend's
  own no-op-seat self-report, and the exact reproduction command. It always
  exits 0: a no-seat host must not fail the pipeline.
- **The no-op seat is the non-disruptive fallback.** `LIBSEAT_BACKEND=noop`
  opens the card directly (the ACL grants the dev user) and `DrmDeviceFd::new`
  warns "assuming unprivileged mode" rather than stealing master, so the DRM
  path runs far enough to report `OPEN (no connected output)` without touching
  the host compositor.

## Consequences

- The T-03.2 unit is marked **open** on this host, with a committed artifact;
  T-03.4's runbook runs the same backend on a free seat/VM and replaces the
  open artifact with `t03-drm-loop-trace.txt` and the loop capture.
- Any future DRM checkpoint should classify through `drm_bringup` so the
  marker stays one greppable line and the open reason stays honest.
- Refusing to run with zero outputs changes behavior only for a session that
  could not scan out; headless/nested are unaffected.