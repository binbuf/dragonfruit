# T-16 — Platform Polish and Packaging

> **Track, not a single slice.** This file is the design reference. It is executed as 15 one-session tasks: [T-16.1a](../../tasks/142-t-16.1a-per-output-chrome-sizing.md) · [T-16.1b](../../tasks/143-t-16.1b-per-output-window-placement.md) · [T-16.2](../../tasks/144-t-16.2-hotplug-under-load-and-lockstep.md) · [T-16.3a](../../tasks/145-t-16.3a-integer-scaled-xwayland.md) · [T-16.3b](../../tasks/146-t-16.3b-viewport-downscale-and-chrome.md) · [T-16.6a](../../tasks/147-t-16.6a-atspi-and-keyboard-audit.md) · [T-16.6b](../../tasks/148-t-16.6b-magnifier-and-reduced-motion-sweep.md) · [T-16.7](../../tasks/149-t-16.7-localization-and-i18n.md) · [T-16.8a](../../tasks/150-t-16.8a-crash-kill-matrix.md) · [T-16.8b](../../tasks/151-t-16.8b-compositor-death-and-restart-policy.md) · [T-16.4](../../tasks/162-t-16.4-suspend-resume-soak.md) · [T-16.5](../../tasks/163-t-16.5-graphics-driver-matrix.md) · [T-16.9](../../tasks/164-t-16.9-fedora-packaging-and-ci.md) · [T-16.10](../../tasks/165-t-16.10-debian-packaging-and-ci.md) · [T-16.11](../../tasks/166-t-16.11-packaged-build-performance-re-measure.md). Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 16 of 17 — the hardening and distribution pass |
| **Area** | `compositor/` (multi-monitor, scaling, drivers) · a11y/i18n · `packaging/` · CI |
| **Depends on** | T-12, T-15 |
| **Blocks** | T-17 |
| **Legacy detail** | [legacy/31-polish-hardening.md](../../tasks/legacy/31-polish-hardening.md) · [legacy/32-packaging-distribution.md](../../tasks/legacy/32-packaging-distribution.md) · [legacy/14-hot-corners-desktop-background.md](../../tasks/legacy/14-hot-corners-desktop-background.md) · [testing-ladder.md](../../testing-ladder.md) |

## Demo

```
hotplug a second monitor → chrome anchors per output, windows migrate, Zoom
  and Mission Control stay lockstep
→ fractional scaling on both outputs, no blurry/oversized chrome
→ 100 suspend/resume cycles, no leaked state
→ keyboard-only walkthrough of the whole desktop; AT-SPI tree dumped live
→ switch locale; the shell and first-party apps are translated
→ kill -9 each service and the shell; everything recovers; compositor death
  ends the session by design
→ install the Fedora and Debian packages on clean VMs; the session appears in
  the display manager
```

Capture: `docs/captures/t16-polish.*`, plus the soak/matrix reports.

## Why now

Everything feature-shaped is done; this slice makes it survive real hardware
and real users. It is the last step before the premium gate because the gate's
"unfamiliar user" test requires accessibility, localization, and stable
packages to be meaningful.

## Inherited and reused

- Per-output chrome limitation from the legacy T-09/T-10 work (`matches_output`
  is the filter change hook).
- The fractional-scaling policy decision (`docs/xwayland-scaling.md`).
- The soak harness and testing ladder.
- The design-system token gates and gallery goldens.

## Scope

### In

1. **Multi-monitor**: per-output chrome sizing and reserved zones, window
   placement per output, lockstep transitions, hotplug under load.
2. **Fractional scaling**: integer-scaled Xwayland + per-surface viewport
   downscale as documented; chrome sized per output.
3. **Suspend/resume**: 100-cycle soak, output/input/client recovery.
4. **Graphics drivers**: Intel/AMD baseline matrix; NVIDIA validation or a
   documented outcome; GPU-loss behavior.
5. **Accessibility**: live AT-SPI dump/walkthrough, keyboard-only operation,
   compositor magnifier if specified, reduced-motion across every animation.
6. **Localization/i18n**: shell and first-party apps translatable; locale
   formatting.
7. **Crash recovery**: kill tests for every restartable component; compositor
   death behavior documented.
8. **Packaging**: Fedora and Debian packages, session files, dependencies,
   CI packaging jobs, install/uninstall cleanly on fresh VMs.
9. **Performance**: re-run every budget on the packaged build; publish the
   numbers.

### Out / explicitly deferred

- HDR/color-management staging (post-gate backlog).
- NVIDIA-specific features beyond baseline validation.
- Additional distros beyond Fedora/Debian.

## Acceptance

- [ ] The demo runs and the capture/reports are committed.
- [ ] Multi-monitor hotplug + fractional scaling pass a scripted VM matrix.
- [ ] 100 suspend/resume cycles pass.
- [ ] Keyboard-only walkthrough and live AT-SPI dump pass.
- [ ] Packages install and run on clean Fedora and Debian VMs.
- [ ] All performance budgets are re-measured on the packaged build.
- [ ] `make soak` stays green; all previous demos still pass.

## Test plan

- VM matrix: hotplug, scaling, suspend, driver install.
- Scripted soak cycles; crash/kill matrix.
- a11y: AT-SPI dump diff, keyboard walkthrough.
- Packaging: clean-VM install/uninstall.

## Risks

- **VM/hardware access** gates the matrix; keep the automated parts in CI and
  record the manual parts honestly.
- **Fractional scaling edge cases** are a known multi-slice risk; the
  documented policy is the fallback.
- **Packaging drift**: build from the same pinned toolchains CI uses.

## Hand-off

- T-17 gates on this slice's reports.
