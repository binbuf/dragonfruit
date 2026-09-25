# 0053 — Full-session testing is a logout round-trip or a second VT, never a compositor handoff

## Status

proposed

## Context

`make dev`/`make demo` run the compositor **nested** in the host session and
cover 80–90% of daily work (`docs/testing-ladder.md` rung 1), but they never
exercise the seat: a real login, DRM/KMS, udev input, logind, session services,
or a display-manager entry. T-12 makes the desktop a real session
([11-session-and-dev-workflow.md](../11-session-and-dev-workflow.md)), and the
hardware rail (T-159…T-161) brings up the DRM backend, so the remaining gap is
a repeatable way to "fully test on the dev workstation".

The tempting shape — stop the host DE, launch Dragonfruit as the primary
compositor, then start the host DE again — is not possible to do safely.
Wayland clients are bound to one compositor and there is no standardized
migration, so **there is no live compositor handoff**
([11-session-and-dev-workflow.md](../11-session-and-dev-workflow.md)), and
scripting per-DE stop/start (`systemctl stop gdm`, killing GNOME/KDE) is
fragile and dangerous on a workstation.

## Decision

Add a **dev-session harness** with two modes, both DM-agnostic and both
crossing a fixed session boundary rather than swapping compositors:

- **Second VT (no logout, the daily mode).** The host DE stays on its VT; the
  harness starts a real DRM session on a free VT for a **dedicated user**, so
  the two graphical sessions do not collide over user services. Returning is a
  VT switch; the host session is never closed.
- **Display-manager round-trip (same user, the realism mode).** The harness
  *arms* the next DM login (select the Dragonfruit session, optionally arm
  autologin), logs out through the host session manager so unsaved-work prompts
  still fire, and restores the previous default session on return. A state file
  under `$XDG_STATE_HOME/dragonfruit/` makes return exact even after a crash,
  with a "session-ready" beat before autologin commits to prevent a crash loop.

Supporting decisions:

- **We never stop, restart, or introspect the host compositor.** VT switching
  and DM session selection are kernel/logind/DM mechanisms that work for every
  DE; this is what makes "all popular DEs" achievable without per-DE logic.
- **Session preselection is a per-DM adapter** (GDM via AccountsService
  `XSession`, SDDM via its state file, LightDM via `~/.dmrc`), degrading to
  "the human picks in the greeter". Autologin is optional, needs root, is armed
  only for the round trip, and is force-disabled on return or recovery.
- **No app continuity is promised.** No client survives the boundary; the
  harness may relaunch an explicit allowlist captured before logout, but never
  resurrects unsaved document state.

Alternatives rejected: live compositor handoff (not standardized), per-DE
stop/start scripts (fragile, invasive), and same-user second graphical session
(shared `XDG_RUNTIME_DIR`/portals collide).

## Consequences

- T-12 gains T-12.6a–c; they depend on the DM entry (T-12.2) and are exercised
  on the hardware rail (T-159…T-161), so they are scheduled after it.
- A user-visible **"Quit to <previous desktop>"** item appears only when the
  session was started by the harness (`DRAGONFRUIT_DEV_RETURN` set).
- The second-VT mode needs a dedicated development user, documented in
  `docs/testing-ladder.md` rung 2.
- The harness must remain a *dev* tool: it ships no autologin default and no
  permanent change to the host's DM configuration.