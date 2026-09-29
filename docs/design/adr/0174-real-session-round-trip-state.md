# 0174 — The real-session round trip is one state file, a session-ready autologin beat, and an exact file snapshot

## Status

accepted

## Context

T-12.6b must close the host session, log into Dragonfruit as a primary session,
and restore the host desktop **exactly** — from the system menu's "Quit to
\<previous desktop\>" item or, after a crash, from the next login. There is no
live compositor handoff ([ADR 0053](0053-real-session-dev-harness.md)), so the
only way to make the return exact is to persist what the arm changed and to
have a single owner of the restore.

Two problems need a decision:

1. **Crash-loop protection.** If the harness arms display-manager autologin
   immediately and the Dragonfruit compositor crashes during startup, the next
   login autologins straight back into the crashing session. Autologin must be
   *committed* only once the session is known to be up.
2. **Byte-identical restore.** T-12.6a's per-DM keyfile edits are minimal but
   restore by reconstructing values, so a key the display manager did not have
   (e.g. GDM's `AutomaticLoginEnable`) can be left behind. "Exactly as it was"
   demands a stronger guarantee.

## Decision

- **One state file is the source of truth.** `$XDG_STATE_HOME/dragonfruit/
  dev-session.json` (`dragonfruit_session::dev_session`) records the previous
  default session, the display manager, the autologin request and prior
  snapshot, the dev-tool path, timestamps, and — crucially — **exact byte
  snapshots** of the display-manager files the arm may edit. The harness is the
  only writer; the file is cleared on return and recovery.
- **The session entry reads the state file**, not the harness, to export
  `DRAGONFRUIT_DEV_RETURN` (and `DRAGONFRUIT_DEV_BIN`) into the login
  environment. The shell shows the return item only when that variable is set.
- **Autologin is gated by the session-ready beat.** `--round-trip` never arms
  autologin itself; `--ready`, run once the session is up, arms it and marks
  `session_ready`. A crash before the beat leaves the display manager
  untouched, so it cannot loop back in.
- **Restore writes the snapshots back verbatim** (creating a file that did not
  exist is recorded as absence and removed again). When a state file predates
  snapshots, the T-12.6a semantic `restore`/`restore_autologin` seam is the
  fallback, so the round trip still works.
- **Logout asks the session manager.** GNOME's `gnome-session-quit --logout`,
  KDE's KSMserver D-Bus logout, or the logind fallback — never a `SIGKILL`, so
  unsaved-work prompts fire normally.

## Consequences

- A clean round trip is byte-identical on GDM, SDDM, and LightDM, including a
  display-manager file that did not exist before the arm.
- The state file carries DM file contents and is user-readable; it is dev-only
  and cleared on return, and the autologin files are already world-readable.
- The second-VT mode (T-12.6c) is unchanged and does not use this file.
- The real display-manager write and the real logout still need a seat/spare
  GPU/VM (the T-159…T-161 rail); the headless suites pin the arm/ready/restore
  state machine, the byte snapshots, and the environment propagation.