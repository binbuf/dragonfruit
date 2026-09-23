# T-26 — Lock Screen and Idle Policy

| | |
|---|---|
| **Phase** | 5 · Desktop infrastructure |
| **Area** | `compositor/` (ext-session-lock) + shell lock surface |
| **Depends on** | [T-02](02-compositor-core.md) (`ext-session-lock-v1`, `ext-idle-notify`) · [T-07](07-private-shell-protocols.md) · [T-24](24-session-lifecycle.md) (logind LockSession) · [T-08](08-design-system.md) |
| **Blocks** | Daily-driver bar (trustworthy lock screen) · Phase-5 exit (kill tests) |
| **Estimate** | L |
| **Design docs** | [11-session-and-dev-workflow.md](../../design/11-session-and-dev-workflow.md) · [01-architecture.md](../../design/01-architecture.md) · [08-settings.md](../../design/08-settings.md) |

## Summary

Fail-secure locking: compositor-enforced via `ext-session-lock-v1`
semantics with **no code path — including a crash — that unlocks a locked
session**, plus idle-driven auto-lock, and a lock surface (authenticator)
rendered by the shell but enforced by the compositor.

## Background

Locking is **fail-secure: no code path, including a crash, unlocks a
locked session** ([01-architecture.md](../../design/01-architecture.md)).
`LockSession`/`UnlockSession` requests drive the lock screen; idle locking
uses `ext-idle-notify` thresholds owned by the compositor
([11-session-and-dev-workflow.md](../../design/11-session-and-dev-workflow.md)).

## Scope

### In scope

1. **Compositor lock enforcement**:
   - `ext-session-lock-v1` server semantics: while locked, only the lock
     surface renders; other clients receive no input, no new surfaces
     map above, screen capture is refused (portal requests while locked
     follow the consent path — capture cannot bypass lock).
   - Lock/Unlock requests from logind (`LockSession`/`UnlockSession`),
     hot corners (T-14), idle thresholds (`ext-idle-notify`), and a
     user-facing shortcut.
   - **Crash policy**: compositor crash while locked → session ends
     (never an unlocked desktop). Lock surface crash → the session stays
     locked (the lock surface is restarted or the compositor re-renders a
     minimal lock) — the invariant is the lock, not any one process.
2. **Idle policy** (compositor-owned):
   - Thresholds from settingsd (Lock Screen pane, T-16): idle→lock delay,
     display-off delay, `idle-inhibit` honored (video playback keeps the
     screen on via the protocol from T-02).
   - Suspend interaction with logind (with T-24 hooks).
3. **Lock surface (shell-rendered)**: authenticator UI (password first;
   biometrics via fprintd where present, T-20 A10), clock/wallpaper
   presentation, unlock animation per motion tokens, reduced-motion
   variant.
4. **Authentication**: PAM-based unlock through the host's infrastructure
   (we never roll our own); auth failure handling (delays, message),
   session-switch affordance (return to GDM) — that path ends the session,
   by design.
5. **Kill-test harness**: scripted attempts to unlock by crashing
   compositor, lock surface, settingsd, shell, and by protocol abuse
   (malicious client attempts during lock) — the Phase-5 exit drill.

### Out of scope

- Screen dimming/DPMS details beyond thresholds (T-31 polish).
- GDM/greeter (reused, [01-architecture.md](../../design/01-architecture.md)).

## Requirements

- FR-1: Locked-state invariant test: while locked, no client input flows,
  no surfaces appear above, no capture path succeeds; enforced by the
  compositor regardless of shell state.
- FR-2: Kill matrix: every plausible crash during lock leaves the session
  locked or ends it — **never unlocked**. Includes: kill lock surface,
  kill shell, kill settingsd, protocol-fuzz the lock compositor.
- FR-3: Unlock success path: PAM auth round trip; failure path with
  delay/rate-limit.
- FR-4: Idle chain honors `idle-inhibit` (video playback test) and
  thresholds apply live from settingsd changes.
- FR-5: Hot corner / shortcut / logind requests all lock within one frame
  budget; unlock animation interruptible.
- FR-6: Lock works on multi-monitor (all outputs locked; any monitor can
  host the authenticator).

## Acceptance criteria

- [ ] **Phase-5 exit: the lock screen survives kill tests.**
- [ ] Kill matrix green in the VM (deliberate crash tooling).
- [ ] Idle-inhibit + threshold round-trip green.
- [ ] PAM unlock works on the reference Fedora config; failure states
      designed, not accidental.

## Test plan

- Automated kill drills (the testing-ladder VM stage —
  [11-session-and-dev-workflow.md](../../design/11-session-and-dev-workflow.md)).
- Malicious-client suite during lock (protocol fuzz).
- Multi-monitor lock/unlock matrix.

## Risks / open questions

- The lock surface's visual polish vs. the invariant: if in doubt, ugly
  and locked beats beautiful and bypassable.
- PAM stack specifics (fedora pam + fprintd) — keep auth in a small
  isolated component with clear boundaries.
