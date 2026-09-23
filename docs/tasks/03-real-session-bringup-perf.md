# T-03 — Real Session: DRM Bring-Up and Performance Baseline

> **Track, not a single slice.** This file is the design reference. It is executed as 4 one-session units: [T-03.1](units/011-t-03.1-nested-performance-budgets-host-rail.md) · [T-03.2](units/012-t-03.2-drm-first-bring-up.md) · [T-03.3](units/013-t-03.3-hardware-input-validation.md) · [T-03.4](units/014-t-03.4-drm-soak-teardown-runbook.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 3 of 17 — the whole loop on real hardware |
| **Area** | `compositor/` (DRM backend) · `tools/dragonfruit-dev/` · CI/perf harness |
| **Depends on** | T-01 (T-02 optional; can run in parallel with T-04) |
| **Blocks** | T-12 (real session), T-17 (gate) |
| **Legacy detail** | [legacy/02-compositor-core.md](legacy/02-compositor-core.md) · [legacy/03-input-keymaps-shortcuts.md](legacy/03-input-keymaps-shortcuts.md) · [legacy/01-repo-scaffolding-ci-licensing.md](legacy/01-repo-scaffolding-ci-licensing.md) (soak) · [testing-ladder.md](../testing-ladder.md) |

## Demo

The T-01 loop, run in a **DRM/logind session on real hardware** (a spare seat,
a second VT with a dedicated dev user, or a VM with a spare GPU):

```
log in to the Dragonfruit session → menu bar + Dock appear on the real display
→ launch an app from the Dock → titlebar, drag, zoom, minimize, restore, close
→ workspace switch → clean exit back to the display manager
```

Capture: `docs/captures/t03-drm-loop.*`, plus the frame-trace output.

## Why now

The DRM backend has **never run**. Every slice after this one (materials,
Mission Control, session, lock, portals) will be judged on real hardware, and
the performance budgets are meaningless until measured there. Finding this out
at T-17 is the "too late" risk in its purest form. T-03 is deliberately
scheduled right after the loop exists.

**This slice does not block the nested critical path.** If no seat is
available, record it as open, proceed with T-04, and carry the open item to
T-17. Do not let hardware access stall the plan.

## Inherited and reused

- DRM backend implementation (LibSeat session, udev hotplug, per-crtc
  `DrmOutput`, GbmGles backend, vblank scheduling, direct scanout, hardware
  cursor, multi-GPU `GpuManager`) — written, never run
  (`legacy/02-compositor-core.md`).
- Render-path counters and `dump_stats` on SIGUSR1/clean exit.
- Nested 100-cycle soak and teardown verification.
- Synthetic-input harness for the headless half; real libinput devices on DRM.

## Scope

### In

1. **DRM first bring-up**: LibSeat, udev hotplug, per-crtc outputs, vblank
   frame scheduling, direct scanout, hardware cursor plane, multi-GPU
   import/fallback. Fix whatever falls out; expect real bugs.
2. **Input on hardware**: mouse, keyboard, touchpad gestures, hot corners,
   tablet/pen pressure, one non-US layout.
3. **Frame budgets measured and recorded** (nested + DRM):
   - 60 s nested idle trace: zero damage, zero client wakeups;
   - input-to-photon latency under one frame;
   - one frame of compositor work per animation frame;
   - direct-scanout counter advances under the expected conditions.
4. **Teardown soak**: 100-cycle nested + a DRM session cycle with no leaked VT
   master, socket, token, or orphaned client.
5. **Documented runbook**: the exact commands for the dedicated-user
   second-VT workflow and the VM path, linked from
   [../testing-ladder.md](../testing-ladder.md).

### Out / explicitly deferred

- Suspend/resume and 100-cycle suspend soak (T-16).
- Multi-monitor transitions and fractional scaling (T-16).
- NVIDIA-specific validation (post-gate backlog).

## Acceptance

- [ ] The demo runs on DRM and the capture + frame trace are committed.
- [ ] The four budget measurements are recorded with raw numbers in
      PROGRESS.md (or a linked report), pass/fail stated honestly.
- [ ] Nested 100-cycle soak and one DRM session cycle pass with clean
      teardown.
- [ ] The runbook lets a second person reproduce the run without help.
- [ ] If hardware is unavailable: the slice is explicitly marked open, not
      silently skipped.

## Test plan

- Automated: nested soak, idle trace, latency harness (headless where
  possible).
- Manual/instrumented: DRM bring-up matrix on the available hardware.
- Regression: `make e2e` stays green on the headless backend.

## Risks

- **Hardware access** is the main risk; keep the slice parallel and
  non-blocking.
- **DRM bugs may be deep** (session/seat/vblank). Timebox the first bring-up;
  if it needs more than the slice allows, split a T-03b follow-up rather than
  hiding the failure.
- **Latency measurement methodology** matters; document the instrument and the
  conditions with the numbers.

## Hand-off

- T-12 consumes the seat/session knowledge and the runbook.
- T-16 owns suspend, multi-monitor, scaling, and the driver matrix.
- T-17 gates on the DRM run.
