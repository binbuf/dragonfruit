# T-17 — The Premium Experience Gate

> **Track, not a single slice.** This file is the design reference. It is executed as 9 one-session tasks: [T-17.1a](../../tasks/152-t-17.1a-nested-window-loop-verification.md) · [T-17.1b](../../tasks/153-t-17.1b-workspace-overview-switcher-verification.md) · [T-17.1c](../../tasks/154-t-17.1c-flatpak-browser-verification.md) · [T-17.3](../../tasks/155-t-17.3-visual-floor-and-reduced-motion-sign-off.md) · [T-17.5a](../../tasks/156-t-17.5a-absent-daemon-and-crash-matrix.md) · [T-17.5b](../../tasks/157-t-17.5b-leak-and-lock-enforcement-verification.md) · [T-17.6](../../tasks/158-t-17.6-unfamiliar-user-test-and-sign-off-report.md) · [T-17.2](../../tasks/167-t-17.2-drm-full-loop-verification.md) · [T-17.4](../../tasks/168-t-17.4-performance-budget-verification.md). Strict order and prerequisites live in [ROADMAP.md](../../ROADMAP.md).

| | |
|---|---|
| **Slice** | 17 of 17 — the full loop, measured and reviewed |
| **Area** | integration: compositor + shell + apps + session + third-party |
| **Depends on** | all previous slices |
| **Blocks** | the "premium macOS-like desktop" milestone |
| **Legacy detail** | [legacy/34-mvp-vertical-slice-gate.md](../../tasks/legacy/34-mvp-vertical-slice-gate.md) · [ROADMAP.md](../../ROADMAP.md) · [00-overview.md](../00-overview.md) |

## Demo

The 30-second loop, performed end to end on **nested and DRM** sessions, by
someone who has never seen the build:

```
launch app → Dock animation → window appears with traffic lights →
workspace switching → Mission Control (live surfaces) → minimize →
restore from Dock → app switch → close
```

with a third-party Qt app and an X11 app in the same loop, plus a Flatpak
browser, at the visual floor, reduced motion passing, and zero dropped frames
for the gestures.

Deliverable: `docs/captures/t17-premium-gate.*` (the recorded loop, light +
dark + reduced motion) and a sign-off report.

## Why this is a slice and not a phase

The legacy plan deferred this judgment to a gate behind a twelve-ticket fan-in
and risked discovering the "functional but looks bad" state at the end. In
this plan every slice has already produced a reviewed demo; T-17 is therefore
a **confirmation and a measurement**, not a discovery. If T-17 finds a
surprise, that is a planning failure to fix in the slice that owns it, not
new scope here.

## The checklist

### Loop

- [ ] Full loop completes on nested and DRM.
- [ ] Launch, appear, traffic lights, move, zoom, minimize, restore, close,
      workspace switch, Mission Control, app switch all work by pointer and
      keyboard.
- [ ] A third-party Qt app (SSD) and an X11 app behave correctly; a CSD app
      is unaffected.
- [ ] A Flatpak browser can file-choose, screenshot, and screen-share.

### Feel and visual floor

- [ ] Materials present (blur/rounding/shadows) and signed off against the
      design-system reference, light and dark.
- [ ] Every animation has a reduced-motion variant that passes.
- [ ] No flat approximations remain in shipped chrome.

### Performance

- [ ] 60 Hz, zero dropped frames for workspace switch and Mission Control
      gestures on baseline Intel/AMD (frame trace attached).
- [ ] Input-to-photon latency under one frame, nested and DRM.
- [ ] Idle desktop: zero damage, zero client wakeups from our shell.
- [ ] Folder open and large-list scroll budgets from T-10.

### Robustness

- [ ] Every absent-daemon case degrades; nothing blocks session start.
- [ ] Crash/kill matrix: shell, settingsd, notification service, portal
      backend, apps; compositor death ends the session by design.
- [ ] Teardown: repeated loops leak no socket/token/`DISPLAY` file, no
      orphaned client, no VT master.
- [ ] Lock enforcement verified again on the packaged build.

### Product judgment

- [ ] The unfamiliar-user test: a person who has not read the docs performs
      the loop without instructions; failures are legible.
- [ ] Side-by-side against the design docs and
      [`../reference/System_Preferences.md`](../../reference/System_Preferences.md):
      the interaction model reads as intended, the assets are original.
- [ ] The post-gate backlog is explicitly listed, not silently implied.

## Acceptance

- [ ] Every checklist item is ticked or explicitly waived with a recorded
      reason.
- [ ] The captured loop and the sign-off report are committed.
- [ ] `make check` (lint + test + soak) and `make e2e` are green on the
      release commit.
- [ ] `README` and `docs/design/11-session-and-dev-workflow.md` carry the
      exact reproduction commands.

## Risks

- **Hardware access** for the DRM half; if unavailable, the gate is marked
  incomplete, not passed (the legacy T-34 rule).
- **Third-party app drift**: pin the zoo versions used for the gate and
  record them.
- **Scope pressure**: this slice adds no features. Any surprise becomes a
  follow-up slice, and the backlog list is the honest record.
