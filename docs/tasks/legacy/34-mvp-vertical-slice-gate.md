# T-34 — MVP Vertical-Slice Gate (Working Desktop)

| | |
|---|---|
| **Phase** | MVP milestone (spans Phases 1–3) |
| **Area** | integration: compositor + shell + apps + session |
| **Depends on** | [T-33](33-compositor-effects-materials.md) · [T-35](35-window-lifecycle-animations.md) · [T-13](13-window-decorations-ssd.md) · [T-11](11-mission-control-workspace-ux.md) · [T-12](12-app-switcher.md) · [T-09](09-menu-bar.md) (with the [T-20](20-system-service-adapters.md) MVP slice) · [T-15](15-settingsd-settings-model.md)/[T-16](16-settings-app.md) · [T-17](17-files-core.md)/[T-18](18-files-app.md) · [T-24](24-session-lifecycle.md) · [T-01](01-repo-scaffolding-ci-licensing.md) nested workflow |
| **Blocks** | First user-facing "working desktop MVP" milestone · Phase-2/3 exit |
| **Estimate** | M |
| **Design docs** | [ROADMAP.md](../../ROADMAP.md) · [11-session-and-dev-workflow.md](../../design/11-session-and-dev-workflow.md) · [00-overview.md](../../design/00-overview.md) |

## Summary

The single acceptance gate that declares the **demo MVP** reachable: the
30-second interaction loop runs end to end on a nested session and a real DRM
session, with the compositor materials of T-33 present (not a flat
approximation), at 60 Hz, using our own apps plus at least one third-party Qt
app and one X11 app.

This ticket exists so the loop is an explicit, testable deliverable rather
than something we hope T-31 discovers. It is deliberately narrow: no new
features, only integration, measurement, and the visual floor.

## Background

The loop and the vertical slice are defined in
[ROADMAP.md](../../ROADMAP.md); the Experience phase exit is "the 30-second
interaction loop runs with zero dropped frames". The Dock slices (T-10) have
already scripted the compositor-observable half of the loop
(activate/restore/close, `Closed` for cross-Space windows). What remains is
to join the missing links — materials, traffic lights, workspace switching,
Mission Control, app switch, and real apps — and gate them together.

## Scope

### In scope

1. **The loop, end to end**:
   ```text
   launch app → Dock animation → window appears → traffic lights →
   workspace switching → Mission Control → minimize →
   restore from Dock → app switch → close
   ```
2. **Two target sessions**: nested (`dragonfruit dev --nested --shell`) for
   CI/daily dev, and a **real DRM/logind session** (via T-24 units or the
   second-VT workflow) for the milestone sign-off.
3. **The visual floor**: T-33 materials live on menu bar, Dock, popovers, and
   SSD titlebars — the "looks bad" state is explicitly not acceptable at this
   gate.
4. **Mixed clients**: one third-party Qt/Wayland app and one X11 app in the
   same loop, proving identity/Dock grouping and Xwayland integration.
5. **Scripted where possible**: extend `milestone_e2e.rs` / conformance
   suites for the compositor-observable steps; a recorded nested capture and
   a documented manual walkthrough carry the parts a headless test cannot
   (visual quality, feel).
6. **Teardown**: no leaked socket/token/`DISPLAY` file/orphaned client, no
   VT master leak on the DRM run.

### Out of scope

- Daily-driver features (multi-monitor, suspend, IME, NVIDIA) — T-31.
- System-service adapters/Control Center/notifications — T-20–T-25; the
  menu-bar status items may be stubbed here.

## Requirements

- **FR-1**: The full loop completes on nested and DRM sessions with our
  Settings and Files apps and at least one Qt and one X11 third-party app.
- **FR-2**: 60 Hz, zero dropped frames for workspace switch and Mission
  Control gestures on baseline Intel/AMD (the T-11 budget, measured here).
- **FR-3**: Reduced-motion variants of every loop animation pass.
- **FR-4**: The chrome materials of T-33 are visibly present (blur/rounding/
  shadows), verified by side-by-side capture against the design-system
  reference.
- **FR-5**: A person unfamiliar with the build can perform the loop from the
  session without instructions; failures are legible (badges/messages), never
  silent.
- **FR-6**: Clean teardown on both backends; repeated loops leak nothing.
- **FR-7**: Every app absent case degrades: an app that fails to launch
  leaves the Dock usable and the session alive.

## Acceptance criteria

- [ ] Scripted nested loop passes in CI (the compositor-observable steps).
- [ ] DRM session run of the same loop passes; teardown clean.
- [ ] Frame-time trace for the full gesture within budget.
- [ ] Nested capture review: materials present, traffic lights correct,
      light/dark + reduced motion.
- [ ] `README`/`11-session-and-dev-workflow` updated with the exact
      commands to reproduce the gate.

## Test plan

- `make e2e` / `milestone_e2e.rs`: attach a shell client, a Wayland app, and
  an X11 app; drive launch → focus → Space switch → overview → minimize →
  restore → app switch → close over the private protocol and synthetic input.
- Nested capture: visual-floor image assertions (blur token on/off).
- DRM: manual + instrumented run on baseline Intel/AMD with the frame-time
  trace; teardown soak (repeat loop N times).

## Risks / open questions

- The DRM run needs a spare seat/VM; if unavailable, gate on nested and file
  the DRM run as the one open item (mirrors the T-02 DRM status) rather than
  declaring the milestone green silently.
- Third-party app choice matters for traffic lights (Qt SSD vs GTK CSD); the
  gate should use a Qt app for the Tier-2 path and document the GTK/Electron
  outcome as best-effort.