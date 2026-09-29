# 0175 — The second-VT harness decides from `loginctl` and refuses a colliding user

## Status

accepted

## Context

T-12.6c is the low-friction "fully test on the workstation" command: the host
desktop stays on its VT and Dragonfruit starts a real DRM session on a **free
VT** for a **dedicated user**, with its own `XDG_RUNTIME_DIR` and user services.
It differs from the T-12.6b round trip in that it never logs the host out — the
return is a VT switch — so it needs no display manager, no autologin, and no
state file ([ADR 0053](0053-real-session-dev-harness.md),
[ADR 0174](0174-real-session-round-trip-state.md)).

Two graphical sessions for the same Unix user collide over shared user-session
state (portals, `XDG_RUNTIME_DIR`, environment), so the mode requires a
dedicated user. Three questions need a decision: how to know a VT is free,
when to refuse, and how to start and end the session without touching the host.

## Decision

- **Everything is read from `loginctl`, and nothing is guessed.** The harness
  snapshots sessions with `list-sessions` plus one `show-session` per id
  (`Name`, `Seat`, `VTNr`, `Type`, `Class`, `Active`, `State`). The free VT is
  the smallest VT in `2..=12` that no session occupies — **VT1 is never
  assumed** to be the host desktop; a host may run it on any VT.
- **Refuse a colliding user with an explanation.** The preflight refuses when
  the target user already holds an active seat graphical session (`Active=yes`,
  `Class=user`, a seat, and `Type` `wayland`/`x11`) and prints the
  dedicated-user rationale and the exact `sudo useradd -m dfdev` bootstrap. A
  console (`tty`) or user-manager session is not a collision.
- **Start through a PAM login session on the free VT.** The plan is
  `chvt <vt>` then
  `systemd-run --uid <uid> --gid <gid> --property PAMName=login
  --property TTYPath=/dev/tty<vt> …` running the shipped session entry, so the
  dedicated user gets its own logind session, user manager, `XDG_RUNTIME_DIR`,
  and `systemd --user`. Teardown is `loginctl terminate-session <id>`, and the
  harness verifies no orphaned `dragonfruit*` process survives.
- **`--plan` is the safe, hardware-free form.** It runs the full preflight and
  VT selection and prints the start/switch/return/teardown commands without
  starting anything, so it is safe next to the host desktop and is what the
  validation script and the mocked-`loginctl` tests exercise.
- **The command seams are injectable.** `DF_LOGINCTL`/`DF_PASSWD` point the
  harness at a mock, `DF_SESSION_ENTRY` overrides the entry script, so the
  header logic is tested with no DM, no seat, and no dedicated user.

## Consequences

- The pure preflight, VT selection, plan commands, and `loginctl` parsing are
  pinned by `second_vt::tests` (a mocked `loginctl` included), and
  `make second-vt-validation` records the refusal plus the mocked selection
  everywhere; the one real start → switch → return → teardown cycle needs a
  free seat and the dedicated user (the T-159…T-161 rail) and is recorded
  OPEN on a host with neither.
- The harness is a dev tool: it creates no user, ships no autologin default,
  and never stops, restarts, or introspects the host compositor. The host
  session is left exactly as it was; returning is `chvt <host-vt>`.
- The second-VT mode and the display-manager round trip share `dragonfruit
  dev --real`; the presence of a round-trip flag selects T-12.6b, its absence
  selects this mode.