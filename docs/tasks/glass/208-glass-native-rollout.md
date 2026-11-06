---
verify: make gates
---
# T208 — Verify native DRM and enable the replacement by default

## Goal

Verify the replacement on native Dragonfruit and enable it by default once the release evidence supports the switch.

## Context (read first)

- Depends on T207 and a usable native baseline. Read `docs/runbook-drm-session.md`, `docs/testing-ladder.md`, `docs/rendering.md`, and `docs/v0.1.1/progress/{T191b,T192b}.md`.
- Use the T194 fixtures and the T207 evidence matrix. Confirm which v0.1.1 issues remain at the release candidate, including the prior GPU-overview teardown report, before attributing faults to this change.

## Scope

- [ ] Capture native KMS or another trusted native compositor output for the core shell surfaces over a changing client, covering Light/Dark, accessibility overrides, foreground readability and opacity/blur transitions. Match old/new hardware, settings, modes and scenarios for comparison.
- [ ] Record per-output refresh/pacing, p50/p95/p99, missed presentations, GPU timings when supported, copied bytes, transport and fallback mix, cache churn, idle wakeups and fd/memory trend. Exercise Dock sweep, menus/panels, settled idle, hotplug/mixed refresh, and fullscreen entry/exit. Check default fullscreen can direct-scan out when eligible, while a deliberately visible overlay requires composition.
- [ ] On supported native hardware, confirm sampled material remains stable at the persistent floor under load, and that software, driver or cross-GPU fallback stays usable. Exercise version compatibility and lock/privacy boundaries.
- [ ] Flip the rollout default only after required nested and native rows pass. Verify the safe rollback route in a real session cycle, and document settings, supported variants and remaining reference gaps in the rendering/design status and capture docs.

## Out of scope

- Moving to a different renderer or chrome transport, or expanding the set to additional macOS surfaces.

## Design notes

Use the existing `make native-status`, `make native-capture`, `make native-perf`, `make t21-presentation-bench` and `make t21-direct-scanout-native` entry points where applicable. Use the actual refresh deadline. Evaluate baseline versus replacement at matched load and record regressions explicitly. Use the workstation's proposed <2 ms Dock and <4 ms Dock plus bar targets as diagnostic goals when measured with comparable GPU timing. Keep switch-off available while the default change is validated. Keep the reference slider and other missing Mac measurements clearly marked.

## Done when

- [ ] Native visual and performance artifacts identify real seat/GPU/driver/output and the capture method. Required primary native rows and rollback pass, with no new presentation, resource, fullscreen, scanout, or lock regression.
- [ ] If required native evidence is unavailable, leave the rollout gate open with the missing seat/GPU/display, exact rerun command and status in Hand-off. Keep the staged default in that case. Mark optional hardware variants separately.
- [ ] `make gates` passes on the final tree. Update `docs/rendering.md`, `docs/design/STATUS.md` and `docs/captures/README.md`. Fill in Hand-off.

## Hand-off

_(Implementation session records the default state, evidence links, rollback and all outstanding rows.)_
