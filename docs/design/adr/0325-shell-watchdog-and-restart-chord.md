# 0325 — A GUI-thread shell watchdog and the restart chord

| | |
|---|---|
| **Status** | accepted |
| **Date** | 2026-11-22 |
| **Scope** | Shell liveness supervision, the manual recovery chord, and the one-time token restart path |
| **Contract** | [27-stability-and-daily-driver.md](../27-stability-and-daily-driver.md) §3.4, [11-session-and-dev-workflow.md](../11-session-and-dev-workflow.md) session supervision |

## Context

The shipped shell unit used `Restart=always` but had no liveness signal. A
GUI-thread deadlock — a blocked Wayland dispatch, a synchronous D-Bus call, or
a QML render stall — leaves the process alive, so systemd never sees a failure
and the desktop keeps a frozen menu bar and Dock. The stability review records
this as S-09.

A heartbeat cannot live on a worker thread: it would keep beating through the
GUI-thread block that the watchdog exists to catch. The manual chord has the
same problem if it only delivers an event to the shell's queue, because a hung
GUI thread never drains it.

## Decision

- **The shell is a notify service.** `dragonfruit-shell.service` uses
  `Type=notify`, `NotifyAccess=main`, and `WatchdogSec=30`.
- **The heartbeat is a GUI-thread timer.** `ShellWatchdog`
  (`shell/src/shellwatchdog.*`) resolves `sd_notify` at runtime with
  `QLibrary`, so the shell does not gain a build-time libsystemd dependency.
  `ShellController::startShellWatchdog()` defers `READY=1` and the timer to the
  event loop; the timer sends `WATCHDOG=1` every 10 s (three beats per systemd
  window). A blocked GUI thread stops the beats and systemd kills and restarts
  the process. Outside systemd (`NOTIFY_SOCKET` absent) the timer stays off.
- **`Ctrl+Alt+Esc` is `InputAction::RestartShell`.** The shortcut engine binds
  the exact chord; `Ctrl+Esc` and `Alt+Esc` remain ordinary application keys.
- **The compositor drops the connection, not the shell.** On the action the
  compositor closes the authenticated shell client's Wayland connection
  (`DisconnectReason::ConnectionClosed`). This works even when the shell's GUI
  thread is hung. The normal disconnect path removes the chrome layers,
  restores window focus, and reissues the shell's one-time launch token; then
  `Restart=always` starts a fresh shell. The shell also handles a routed
  `restart-shell` action as a fallback for a compositor that only sends the
  event.
- **A locked session stays fail-secure.** Dropping the shell while locked
  follows [ADR 0312](0312-lock-loss-ends-the-session.md): the session ends
  cleanly rather than leaving a frozen lock. The chord never unlocks.

## Consequences

- One timer runs on the GUI thread for the shell's lifetime; the 10 s beat is
  the only new wakeup.
- The restart chord is a last resort, not a shell feature. It does not save
  shell state: windows and workspaces are compositor-owned and survive, while
  transient popups, menus, and unsaved shell-local UI state do not.
- A shell restart remains one process; splitting the shell or the lock UI into
  separate processes stays out of scope.
- Tests: `shell/tests/tst_shellwatchdog.cpp` (GUI-thread timer, blocked-thread
  behavior, action spelling), `services/session/tests/units.rs` (unit keys),
  and `compositor/src/input/shortcuts.rs` (chord mapping). The opt-in
  `scripts/t314-watchdog-check.sh` proves a real systemd user manager kills
  and restarts a blocked probe.
