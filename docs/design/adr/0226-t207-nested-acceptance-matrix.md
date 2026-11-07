# 0226 — T207 nested acceptance is a checked-in A/B matrix

**Status:** accepted (2026-11-06)

## Context

The liquid-glass replacement had per-surface captures (T198–T206) but no
single acceptance run that compares the replacement with the legacy rail on
the same host, scene and settings. T208 needs nested evidence for the
rollback path, the flat-band targets, and the interaction/fallback matrix
before it can enable the replacement by default.

## Decision

T207 is one checked-in matrix, `tests/glass/t207-cases.json`, plus a runner
and scorer:

- `make nested-acceptance-capture` runs every case on the nested compositor.
  A/B cases run twice with `DRAGONFRUIT_GLASS_ROLLOUT=legacy` and
  `=replacement`; the scene, output scale, scheme, slider, accessibility
  overrides, tier and material path are identical within a pair.
- `make nested-acceptance-check` is the hardware-independent gate. It
  validates the case coverage, the evidence table, the per-surface reports
  and the tolerance contract.
- The flat-band comparison registers the live Dock panel rect, normalizes
  both crops to the same size, and applies mean |ΔsRGB| < 0.02, p95 < 0.05
  and SSIM > 0.90 against the macOS `dock_interior` crop. Patterned or
  underconstrained bands use the per-surface T198/T200/T202/T203/T205/T206
  scores instead.
- The A/B gate compares geometry/padded area, p50 timing against the refresh
  interval, allocation and blur-cache growth, transport/copies, idle
  frames/wakeups and the reference error. The legacy rail is expected to miss
  the reference target; that is recorded as an adaptation, not a pair
  failure.
- The shipped default stays `legacy` until T208. `DRAGONFRUIT_GLASS_ROLLOUT`
  is the A/B and rollback switch.

## Consequences

- The acceptance is repeatable without a human choosing scenes, and a failed
  required row fails `make gates` through the checked-in report.
- The nested refresh override remains scheduling-policy evidence; it is not a
  physical panel rate. Native DRM rows stay OPEN and belong to T208.
- Approved adaptations (1.5x/2x nested checker MTF, underconstrained Dark
  notification surfaces, adapted Dark Settings chrome, the Dark attached-menu
  baseline, and the offscreen/SHM dmabuf capture boundary) are reported
  separately from missed measured criteria.
