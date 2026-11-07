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
- [x] If required native evidence is unavailable, leave the rollout gate open with the missing seat/GPU/display, exact rerun command and status in Hand-off. Keep the staged default in that case. Mark optional hardware variants separately.
- [x] `make gates` passes on the final tree. Update `docs/rendering.md`, `docs/design/STATUS.md` and `docs/captures/README.md`. Fill in Hand-off.

## Hand-off

### Default state

**Not flipped.** `design-system/tokens/tokens.json`
`glass.rollout.defaultMode = "legacy"`; `Rollout::default() = Legacy`. The
native replacement/rollback rows are OPEN, so the staged default is kept.

### Changes

- `scripts/native-session.sh` — persists `DRAGONFRUIT_GLASS_ROLLOUT` into
  `native.env` and both the compositor/shell unit drop-ins, so the rollback
  switch survives `native-rebuild`.
- `scripts/t208-kms-readback.c` — dev-only native KMS readback (T191b method:
  `pidfd_getfd` + `drmModeGetFB2` + three-plane dma-buf/EGL + GLES readback).
- `scripts/t208-shell-fullscreen.c` — dev-only trusted shell-protocol driver
  for the native fullscreen/direct-scanout row.
- `docs/captures/t208-native-acceptance.md`, `t208-native-stats.txt`, and 14
  `t208-native-*.png` stills.
- `docs/design/adr/0227-t208-native-rollout-gate.md`.
- `docs/rendering.md` (P6/R0/R6/R8.2/T208), `docs/design/STATUS.md` (T-20,
  G-03, chrome/portals rows), `docs/captures/README.md` (T208 section),
  `docs/runbook-drm-session.md` (glass switch + DRM capture boundary),
  `docs/tasks/glass/plan/progress/T208.md`.

### Results

- Native baseline re-verified on the real seat: `seat0`, VT2, `card1`/amdgpu
  Lucienne, DP-1 3840x2160 @ 143.999 Hz, scale 1.5. KMS stills show live
  backdrop sampling: light Dock 80.9→169.7, dark Dock 42.5→135.0; light menu
  bar 224.9→238.8, dark 48.6→74.9.
- Idle 60 s: shell chrome 1 frame, no animation, presentation p50 6.946 ms;
  p95/p99 ~1.00 s is the 1 Hz idle VRR cadence; fd/RSS flat.
- vkcube load 35 s: frame p50/p95/p99 6179/8400/8662 us; presentation
  6947/12716/13157 us; budget 6944 us; missed +984; `path=gpu`,
  `blur_draws=101498`, `blur_panels=202702`, `backdrop_skipped=0`,
  allocations 232→235; shell roles `chrome/dock/popup/drawer tier=0 blur=1`;
  fd/memory returned to baseline after the client exited.
- Fullscreen entry PASS with chrome suppressed; `direct_scanout_candidates`
  4019→11261 but `direct_scanouts=0` — direct scanout remains OPEN
  (T190d/T192). Native lock entry/privacy PASS; native lock-exit OPEN (the
  pre-T207 shell restarted before its timed unlock; recovery unlocked cleanly).
- Nested replacement evidence unchanged: T207 50 rows / 46 pass / 4 approved
  adaptations / 16/16 A/B; `make nested-acceptance-check` PASS;
  `make t21-gpu-rollout` PASS (re-run; dmabuf Dock/menu/popovers/lock, SHM fallback; committed T207 artifacts left unchanged).
- Prior GPU-overview teardown: a shortened full-rail main run completed 14/14
  scenarios including Mission Control/close with no TypeError/SIGSEGV; it was
  stopped during the refresh matrix, so the full teardown path remains a T193
  watch item.
- Running native build is pre-T195/T196 (no glass rollout code, schema 17, no
  glass accessibility keys), so the native replacement visual/perf,
  accessibility overrides, hotplug/mixed refresh, GPU timer queries and the
  native rollback render cycle remain OPEN.

### Rollback

The default is already the safe state. The persistent route is:

```bash
DRAGONFRUIT_GLASS_ROLLOUT=legacy make native-rebuild
DRAGONFRUIT_GLASS_ROLLOUT=replacement make native-rebuild  # pre-rollout A/B
```

The config half was exercised with `rebuild --no-build --no-install
--no-logout`: the value appeared in `~/.config/dragonfruit/native.env` and both
unit drop-ins, then the original files were restored byte-for-byte and
`systemctl --user daemon-reload` ran. The native render cycle that reads it is
OPEN.

### Checks

- `make native-status` — PASS; release selected/installed, all units active.
- `make native-capture` — PASS; `~/.local/state/dragonfruit/native-captures/20261106-234833-420595`.
- `make native-perf` — PASS; `20261106-235222-422306`; vkcube 318.30 fps.
- `cargo test -p dragonfruit-compositor --lib material_preset::tests::default_rollout_is_the_existing_appearance` — 1 passed.
- `cargo test -p dragonfruit-compositor --test material_presets rollout_defaults_to_the_existing_appearance` — 1 passed.
- `make nested-acceptance-check` — PASS.
- `make t21-gpu-rollout` — PASS (re-run; committed T207 artifacts left unchanged).
- `bash -n scripts/native-session.sh` — OK; both new C tools compile.
- `make gates` — PASS on the final tree (rc=0; nested acceptance PASS, e2e,
  84/84 ctest, qml-test; log tail shows `glass rollout=legacy`).

### Remaining / blockers

**Blocking:** the native replacement rows and the native rollback render cycle
need the current build installed and a fresh native session. Missing: a free
DRM seat (or VM/second VT) with `card1`/`DP-1` and an operator login; this
session owns DRM master, the compositor is the session anchor, and the host
has no free VT/operator password. The running native binaries predate
T195/T196.

Exact rerun on a free seat / after the operator restarts the session with the
current build:

```bash
cd /home/user/repos/binbuf/dragonfruit
make native-status
make native-capture
make native-perf
make build   # generates build/protocols/* for the shell driver
gcc -O2 -D_GNU_SOURCE -o /tmp/df-kms-readback scripts/t208-kms-readback.c \
    $(pkg-config --cflags --libs libdrm egl glesv2)
/tmp/df-kms-readback --info
/tmp/df-kms-readback --capture /tmp/df-native.ppm
gcc -O2 -D_GNU_SOURCE -Ibuild/protocols -o /tmp/df-shell-fullscreen \
    scripts/t208-shell-fullscreen.c \
    build/protocols/dragonfruit-core-protocol.c \
    build/protocols/dragonfruit-toplevel-protocol.c \
    $(pkg-config --cflags --libs wayland-client)
systemctl --user stop dragonfruit-shell.service
TOKEN=$(cat /run/user/$UID/dragonfruit-wayland.launch-token)
WAYLAND_DISPLAY=dragonfruit-wayland /tmp/df-shell-fullscreen "$TOKEN" vkcube 14
systemctl --user start dragonfruit-shell.service
DRAGONFRUIT_GLASS_ROLLOUT=legacy make native-rebuild   # rollback cycle
```

Optional/OPEN variants: multi-GPU/rotated (single Lucienne, normal transform),
hotplug/mixed refresh (one connected output), native GPU timer queries
(`DRAGONFRUIT_GPU_TIMING=1` needs a restart), cursor/overlay planes (not in the
KMS readback), and the full T192b GPU-overview teardown rerun.
