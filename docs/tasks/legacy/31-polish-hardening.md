# T-31 — Polish and Hardening

| | |
|---|---|
| **Phase** | 7 · Polish |
| **Area** | cross-cutting (compositor, shell, services) |
| **Depends on** | All prior phases; feeds from every ticket's risk list |
| **Blocks** | Daily-driver bar (final) · "confidently given to arbitrary users" |
| **Estimate** | XL (continuous) |
| **Design docs** | [ROADMAP.md](../../ROADMAP.md) · [14-risks.md](../../design/14-risks.md) · [02-compositor.md](../../design/02-compositor.md) |

## Summary

The reliability-and-polish phase: multi-monitor transitions and hotplug,
fractional-scale edge cases, suspend/resume soak, graphics-driver testing
(Intel/AMD baseline → NVIDIA validation), accessibility, localization,
performance-budget enforcement, and crash recovery — with scripted soak
tests in a VM matrix as the exit gate.

## Background

The daily-driver bar
([ROADMAP.md](../../ROADMAP.md)) spans displays, windowing,
input, session, security, power, desktop services, portals, networking,
Bluetooth, audio, storage, clipboard, accessibility, applications, and
hardware (Intel/AMD baseline, then NVIDIA). Phase-7 exit: "multi-monitor
hotplug, 100 suspend/resume cycles, and fractional scaling pass scripted
soak tests in a VM matrix." The biggest technical risk is not drawing
macOS-looking controls — it is making the environment **boringly
reliable** ([14-risks.md](../../design/14-risks.md)).

## Scope

### 1. Displays / GPU

- Multi-monitor transitions (lockstep switches under load — extends T-05/
  T-11), hotplug while fullscreen, mixed-DPI setups.
- Fractional-scale edge cases: `viewporter`+`fractional-scale` clients,
  Xwayland policy (from T-06), chrome alignment at 125%/150%.
- GPU matrix: Intel/AMD baseline drivers; **NVIDIA validation** (proprietary
  + NVK as available) — direct scanout, VRR, multi-GPU fallback drills
  (T-02 FR-6 under soak).
- GPU/driver failure injection: device loss, modeset failure, CRTC
  exhaustion — fail-safe output config, never a hang.

### 2. Power / lifecycle

- 100-cycle suspend/resume soak: `PrepareForSleep` freeze → quiesce →
  resume re-init, outputs, cursors, streams, clocks (shares harness with
  T-24/T-26/T-27).
- Suspend during recording/capture (documented + hardened teardown).
- Idle chains at boundaries (dim/off/lock ordering).

### 3. Input / accessibility

- Input-method (`text-input`/input-method) real-world validation
  (CJK IMEs) — early-integration promise from
  [02-compositor.md](../../design/02-compositor.md) realized here as
  long-tail polish.
- Accessibility pass: keyboard-only full-desktop walkthrough, AT-SPI
  audit of shell + flagship apps, compositor magnification
  (follow-focus/follow-caret) shipped to Settings.
- Gesture quality tuning on real trackpads (T-03 constants).

### 4. Performance

- Enforce the budgets from [ROADMAP.md](../../ROADMAP.md) in
  the dev loop: 60 Hz workspace/Mission Control; input-to-photon < 1
  frame; idle zero-damage/zero-wakeup; one frame of work per animation
  frame; Files budgets (T-17/T-18).
- Continuous perf regression tracking (frame-time traces in CI on
  headless; hardware runs on a cadence).

### 5. Crash recovery / robustness

- Protocol fuzzing of compositor and private protocols (robustness-first:
  never crash on malformed requests —
  [14-risks.md](../../design/14-risks.md)).
- Restart drills for every restartable component under load (the crash
  policy table from
  [01-architecture.md](../../design/01-architecture.md)).
- Journal/telemetry (opt-in) for post-mortem compositor crashes.

### 6. Localization

- String externalization for shell + apps; RTL layout pass; locale
  matrix for collation (Files) and date/time formats (menu bar).

### Out of scope

- New features — polish phase freezes scope; anything new goes to a
  post-1.0 board (scope-creep guardrail,
  [14-risks.md](../../design/14-risks.md)).

## Requirements

- FR-1: **Phase-7 exit**: scripted soak suite in a VM matrix —
  multi-monitor hotplug, 100 suspend/resume cycles, fractional-scale
  cases — passes unattended.
- FR-2: NVIDIA matrix report: baseline feature table (scanout, VRR,
  multi-GPU) with pass/fail per driver, issues filed upstream or gated in
  code.
- FR-3: IME validation matrix (at least one CJK input method) passes in
  flagship apps and the shell.
- FR-4: Accessibility audit passes for the full desktop walkthrough.
- FR-5: Perf regression gates active in CI (no silent regressions on the
  budget table).
- FR-6: Fuzz corpus runs clean over compositor + private protocols
  (continuous fuzzing job).

## Acceptance criteria

- [ ] Soak suite green on the VM matrix (hotplug × suspend × scaling).
- [ ] Hardware-matrix reports archived per release.
- [ ] Zero known crash-class bugs from the drills open at release gate.

## Test plan

- This ticket *is* the test plan: build the soak harness, run it
  continuously, and treat its findings as the work queue.

## Risks / open questions

- NVIDIA is the historical pain point — time-box per driver release; do
  not let it consume the polish budget (it's the tail, not the dog).
- Fuzz findings in private protocols may force additive-version bumps —
  follow the versioning policy, never break the lockstep contract.
