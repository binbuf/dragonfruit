# 0227 — T208 keeps the glass rollout staged until a native replacement cycle passes

**Status:** accepted (2026-11-07)

## Context

T207 closed the nested replacement acceptance: 50 rows, 16/16 legacy/
replacement A/B pairs, `make nested-acceptance-check` PASS. T208 then ran the
native DRM baseline on the live session (`seat0`, VT2, `card1`, amdgpu
Lucienne, DP-1 3840×2160 @ 143.999 Hz, scale 1.5):

- The KMS stills and `SIGUSR1` transcript re-confirm the legacy sampled path
  (`path=gpu`, `blur_draws=101498`, `backdrop_skipped=0`), live backdrop
  sampling (light Dock 80.9→169.7, dark Dock 42.5→135.0), bounded idle/load
  resources and fullscreen chrome suppression.
- The running release is **pre-T195/T196**: no `glass rollout` code, schema 17,
  no glass accessibility keys. The replacement cannot be exercised on it.
- The compositor is the session anchor; restarting it ends the session. This
  host has no free VT/second seat or operator login, so the replacement native
  visual/perf, native lock-exit and the native rollback render cycle cannot be
  produced from inside the session.
- Fullscreen entry works, but the eligible client is composited:
  `direct_scanout_candidates=4019→11261`, `direct_scanouts=0`. This is the
  existing T190d/T192 gap, not a replacement regression.
- The prior GPU-overview teardown report (T192b) did not reproduce in a
  shortened full-rail main run (14/14 scenarios, including Mission Control and
  close); the run was stopped during the refresh matrix, so it remains a T193
  watch item.

## Decision

The shipped glass rollout default stays `legacy`. T208 does **not** flip
`glass.rollout.defaultMode` until a free DRM seat runs the current build
through the native replacement rows and a rollback cycle with no new
presentation, resource, fullscreen, scanout or lock regression.

The one rollback/selection switch is `DRAGONFRUIT_GLASS_ROLLOUT`. The native
session helper now persists it (like `DRAGONFRUIT_DRM_MATERIAL` and
`DRAGONFRUIT_DRM_GLASS`), so a rollback survives `native-rebuild`:

```bash
DRAGONFRUIT_GLASS_ROLLOUT=legacy make native-rebuild
DRAGONFRUIT_GLASS_ROLLOUT=replacement make native-rebuild
```

The evidence boundary, missing seat/GPU/display and exact rerun commands are in
[`docs/captures/t208-native-acceptance.md`](../../captures/t208-native-acceptance.md)
and [`t208-native-stats.txt`](../../captures/t208-native-stats.txt).

## Consequences

- The replacement is still a staged release candidate; nested T207 remains its
  acceptance evidence. The default flip is an operator follow-up on a free
  seat, not an unattended agent change.
- A future task must not treat the native baseline as replacement proof. The
  native replacement stills/perf, native accessibility overrides, direct
  scanout, lock-exit and hotplug/mixed-refresh rows stay OPEN.
- The rollback route is now persistent, so a failed post-flip session can be
  recovered with one explicit `native-rebuild`.
- T192b's GPU-overview teardown finding stays open as a watch item until the
  full teardown path is rerun; T208 did not attribute it to the glass change.
