# T-24 — Session Lifecycle and systemd Units

| | |
|---|---|
| **Phase** | 5 · Desktop infrastructure |
| **Area** | `services/session/` + `packaging/` unit files |
| **Depends on** | [T-02](02-compositor-core.md) · [T-01](01-repo-scaffolding-ci-licensing.md) · [T-20](20-system-service-adapters.md) (logind adapter A11 — implemented **as part of this ticket**, not gated on the full adapter roster) |
| **Blocks** | [T-34](34-mvp-vertical-slice-gate.md) (real-session run) · Daily-driver bar (clean startup/shutdown/crash behavior) · real-hardware testing of everything |
| **Estimate** | L |
| **Design docs** | [11-session-and-dev-workflow.md](../../design/11-session-and-dev-workflow.md) · [01-architecture.md](../../design/01-architecture.md) · [12-packaging.md](../../design/12-packaging.md) |

## Summary

The systemd user-session composition: `dragonfruit-session.target` under
`graphical-session.target`, ordered startup (compositor first; parallel
shell/services; portal last), restart policies, session environment, GDM
Wayland-session registration, and logind integration (locking, sleep, VT).

## Background

Sessions start and end as a unit; **no live compositor handoff**
([00-overview.md](../../design/00-overview.md)). Startup order, restart
policies, and the environment contract are specified once here
([01-architecture.md](../../design/01-architecture.md),
[11-session-and-dev-workflow.md](../../design/11-session-and-dev-workflow.md)).

## Dependency and safety notes

- **Do not gate T-24 on the full T-20 roster.** The session needs only the
  logind adapter (A11: session lifetime/VT/`LockSession`/sleep hooks). The
  MVP critical path runs `session → shell/services → portal`; bring A11 in
  with this ticket and let the remaining adapters follow.
- **Security gate (pre-handoff).** A real DRM session without lock
  enforcement ([T-26](26-lock-screen-idle.md)) must not be handed to anyone
  outside the project. For T-34 the DRM run is a controlled, attended test;
  a general session requires T-26 first. Record this explicitly rather than
  treating an unlocked session as acceptable.

## Scope

### In scope

1. **Unit composition** (from
   [11-session-and-dev-workflow.md](../../design/11-session-and-dev-workflow.md)):
   ```text
   graphical-session.target (systemd --user)
     └── dragonfruit-session.target
           ├── dragonfruit-compositor.service     # starts first, owns the seat
           ├── dragonfruit-shell.service          # After=compositor, Restart=always
           ├── dragonfruit-settingsd.service      # Restart=on-failure
           ├── dragonfruit-menu-broker.service    # Restart=on-failure
           ├── dragonfruit-app-index.service      # Restart=on-failure
           ├── dragonfruit-notifications.service  # Restart=on-failure
           └── dragonfruit-portal.service         # Restart=on-failure
   ```
2. **Startup order**: compositor first (acquires seat/DRM master); shell
   and services start in parallel **once the compositor's private protocol
   socket exists**; the portal backend registers with `xdg-desktop-portal`
   last.
3. **Shutdown** is the reverse: services stop, the shell unmaps chrome,
   the compositor is last to exit. Only the compositor's death ends the
   session (GDM returns the user to the login screen — the display-server
   failure behavior we want).
4. **Launch-token distribution**: the session provisions the shell (and,
   later, the Files desktop surface) with one-time launch tokens
   out-of-band at startup (T-07's trust model).
5. **Session environment** exported at startup: `XDG_CURRENT_DESKTOP=
   dragonfruit`, `XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY` pointing at
   the session socket — the public contract toolkits, `portals.conf`, and
   DE-sensitive libraries use.
6. **GDM integration**: Wayland-session descriptor
   (`dragonfruit.desktop`); **we ship a session entry, not a greeter** —
   login stays with GDM ([01-architecture.md](../../design/01-architecture.md)).
   Parallel-install rule: install alongside GNOME, never replace it.
7. **logind integration**:
   - `LockSession`/`UnlockSession` requests drive the lock screen (T-26).
   - `PrepareForSleep` / `PrepareForShutdown` hooks: freeze animations,
     flush persisted state, quiesce rendering before suspend; resume
     re-inits outputs and resumes render loops.
   - VT management for session switching.
8. **Second-VT workflow documentation + tooling** with a dedicated
   development user (avoid user-session service collisions while running
   GNOME on another VT).
9. **Crash policy table** from
   [01-architecture.md](../../design/01-architecture.md) enforced by unit
   config: shell always-restarts; services on-failure; portal fails-soft;
   compositor death = session end.

### Out of scope

- Lock screen internals (T-26); portal registration logic (T-27);
  notifications service (T-25).
- Packaging the units into RPMs (T-32) — here we author and test them.

## Requirements

- FR-1: Clean startup: no service starts before the compositor's private
  socket exists; portal registers last; total startup race-free under
  stress (10 rapid login cycles).
- FR-2: Clean teardown: **no leaked VT master, no orphaned clients**
  (Foundation/Phase-1 exit test operationalized here at session level);
  shutdown is the documented reverse order.
- FR-3: Kill each non-compositor service during a live session: the
  session continues; per-service degradation matches its policy row.
- FR-4: Compositor kill ends the session and returns to GDM within a
  bounded, pleasant delay; no core dump UI, no hang.
- FR-5: `PrepareForSleep` round-trip: animations freeze, state flushes,
  render quiesces; resume re-inits outputs and resumes loops (no black
  screens, no stuck CRTCs) — first-pass, soak-tested further in T-31.
- FR-6: Environment contract verified: a client launched from the session
  sees the documented env; `portals.conf` selects our backend by desktop
  name (with T-27/T-32).
- FR-7: Session works with GNOME still installed (parallel install; no
  file or service-name conflicts).

## Acceptance criteria

- [ ] Login → full desktop in a real (VM first) session; logout/reboot
      cycles clean.
- [ ] Restart drills for each service pass in-session.
- [ ] Sleep/resume basic pass (T-31 extends to 100-cycle soak).
- [ ] Session units + descriptor land in `packaging/fedora/` shape ready
      for T-32.

## Test plan

- VM: startup/teardown instrumentation (unit ordering graphs via systemd
  analytics); service-kill drills; sleep/resume cycles.
- Second-VT workflow: documented + tested with a dedicated dev user.

## Risks / open questions

- systemd user-bus vs session-bus edge cases on older setups — Fedora 44
  reference target only; Debian deferred (T-32).
- Startup ordering "socket exists" needs a robust readiness signal from
  the compositor — define it as part of the private protocol handshake
  (coordinate with T-07).
