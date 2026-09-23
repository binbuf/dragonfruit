# T-12 — Session Lifecycle, Lock Screen, and Idle

> **Track, not a single slice.** This file is the design reference. It is executed as 10 one-session units: [T-12.1a](units/081-t-12.1a-session-manager-and-restart-policy.md) · [T-12.1b](units/082-t-12.1b-session-environment-and-units.md) · [T-12.2](units/083-t-12.2-display-manager-entry-and-logout-teardown.md) · [T-12.3a](units/084-t-12.3a-lock-protocol-and-ui.md) · [T-12.3b](units/085-t-12.3b-lock-pam-authentication.md) · [T-12.3c](units/086-t-12.3c-lock-input-capture-and-kill-resistance.md) · [T-12.4a](units/087-t-12.4a-idle-timers.md) · [T-12.4b](units/088-t-12.4b-idle-inhibitors-and-wake.md) · [T-12.5a](units/089-t-12.5a-suspend-resume-cycle.md) · [T-12.5b](units/090-t-12.5b-session-policy-keys-and-kill-matrix.md). Strict order and prerequisites live in [ROADMAP.md](../ROADMAP.md).

| | |
|---|---|
| **Slice** | 12 of 17 — the desktop becomes a real session |
| **Area** | `services/session` · `compositor/` (idle, lock) · display-manager integration |
| **Depends on** | T-03, T-08 |
| **Blocks** | T-13, T-16, T-17 |
| **Legacy detail** | [legacy/24-session-lifecycle.md](legacy/24-session-lifecycle.md) · [legacy/26-lock-screen-idle.md](legacy/26-lock-screen-idle.md) · [11-session-and-dev-workflow.md](../design/11-session-and-dev-workflow.md) |

## Demo

```
pick Dragonfruit at the display manager → the session starts (compositor,
shell, services, apps) on real hardware
→ lock the screen (Cmd+Ctrl+Q) → the lock screen; input is captured; unlock
→ idle the session → the screen blanks; a wake input restores it
→ suspend/resume once (hardware permitting)
→ log out → clean teardown, back to the greeter; Plasma/host unaffected
→ kill the shell/services → they restart; kill the compositor → the session
  ends by design
```

Capture: `docs/captures/t12-session.*` (login → lock → idle → logout).

## Why now

This is the first slice where the desktop is a *session* rather than a nested
window. It is sequenced after the loop, materials, and settings because those
are what make the session worth entering — and before portals/compatibility
because those need a real session to be validated. **Safety gate:** no real
session is handed to anyone outside the project before lock enforcement
exists.

## Inherited and reused

- `$XDG_RUNTIME_DIR/<socket>.launch-token` and `.x11-display` hand-off
  contracts (dev tool parity).
- `ChildGuard`/teardown verification patterns.
- The dev-tool nested workflow, soak, and `docs/testing-ladder.md`.
- Compositor output API for display power, and the T-08 settings provider for
  lock/idle policy.

## Scope

### In

1. **Session services**: a session manager (launch order, restart policy,
   environment: `XDG_CURRENT_DESKTOP`, `WAYLAND_DISPLAY`, `DISPLAY`,
   `DRAGONFRUIT_LAUNCH_TOKEN`), systemd user units, and a display-manager
   session entry.
2. **Session lifecycle**: startup, logout, crash behavior (compositor death
   ends the session; shell/services restart), and the dedicated-user
   second-VT workflow for development.
3. **Lock screen**: session lock via `ext-session-lock-v1` (or the sanctioned
   path), a first-party lock UI with authentication (PAM via a small helper,
   never a custom credential store), input capture, and kill-resistance.
4. **Idle**: idle timers (dim, blank, lock, suspend) from policy keys; idle
   inhibitors honored; wake restores.
5. **Suspend/resume** (one cycle; the 100-cycle soak is T-16) with clean
   recovery of outputs, input, and clients.
6. **Policy keys** in settingsd: lock delay, idle delays, suspend behavior.

### Out / explicitly deferred

- 100-cycle suspend/resume soak, multi-monitor transitions, fractional scaling
  (T-16).
- Fingerprint/other auth methods beyond the basic path (T-15).
- Fast user switching.

## Acceptance

- [ ] The demo runs on DRM and the capture is committed.
- [ ] Lock enforcement: input cannot reach clients while locked; a kill test
      of the lock UI does not unlock the session.
- [ ] Idle → blank → wake works; idle inhibitors prevent it.
- [ ] One suspend/resume cycle recovers cleanly.
- [ ] Logout teardown is clean; the host DE/display manager is unaffected.
- [ ] The session entry appears next to the host DE in the display manager.
- [ ] `make soak` and the nested demos stay green.

## Test plan

- Unit: idle policy, session env, unit files.
- Headless/nested: shell/service restart, token hand-off.
- DRM: login → lock → idle → suspend → logout, repeated.
- Kill tests: lock UI, shell, settingsd, notification service.

## Risks

- **Lock correctness** is a security boundary; treat it as such and do not
  ship a real session before it passes.
- **Two graphical sessions for one user collide** through shared services; the
  dedicated-user workflow is the mitigation (testing-ladder rung 2).
- **PAM integration** must reuse the host stack, never a custom store.

## Hand-off

- T-13 validates portals in this session.
- T-16 runs the suspend/multi-monitor soak here.
- T-17's DRM gate uses this session.
