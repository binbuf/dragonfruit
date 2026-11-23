# 0328 — Lock before sleep is a shell gate with a logind delay inhibitor

## Status

Accepted (2026-11-23).

## Context

T318 has to make every sleep request present the lock UI before the machine
sleeps, including a lid close the shell did not initiate. logind emits
`PrepareForSleep(true)` for a menu suspend, an idle suspend, and a lid close
alike, so the signal is the only common trigger. Two details decide the shape:

- logind emits `PrepareForSleep(true)` **before** it checks for delay
  inhibitors and before it starts the sleep target. A client that only locks
  inside the signal handler can lose the race: without a delay inhibitor
  logind may suspend before the compositor has confirmed the lock.
- The compositor confirms `ext_session_lock_v1` asynchronously, after the shell
  sends the lock request. Waiting on the GUI thread is acceptable here — the
  machine is about to sleep and the compositor must still render — but the
  wait needs a bound.

## Decision

- **`shell/src/suspendlock.*` owns the hook.** It subscribes to
  `org.freedesktop.login1.Manager.PrepareForSleep` on the system bus and takes
  one `Inhibit("sleep", "dragonfruit-shell", "Lock the session before sleep",
  "delay")` at startup. The fd is held while awake and released after the
  bounded lock wait (confirmed or timed out), so logind's
  `InhibitDelayMaxSec` window is the time the shell has to lock. A wake
  re-takes the inhibitor for the next cycle.
- **Entering locks once and waits for the compositor.** The handler calls the
  existing `lockScreen()` path, then pumps the GUI loop until
  `ShellProtocol::isSessionLocked()` (or 2 s, shorter than logind's 5 s
  default). A duplicate `PrepareForSleep(true)` while asleep is ignored. The
  inhibitor is released after the wait whether or not the lock confirmed; it
  is never what keeps the session locked.
- **Leaving is a refresh, never an unlock.** `PrepareForSleep(false)` updates
  the lock scene's clock and repaints every configured lock surface, wakes
  DPMS (`df_output.set_dpms(true)`), and re-takes the inhibitor. The gate has
  no unlock effect, so a wake cannot clear the lock.
- **A missing lock path is a logged failure, not a block.** If there is no
  protocol or the compositor refuses, the handler still returns and logind may
  proceed. If the shell is gone while locked, T306/ADR 0312 ends the session;
  the gate never unlocks on its behalf.

Rejected alternatives:

- **No delay inhibitor.** It is the smallest patch, but logind's signal is
  emitted before its inhibitor check, so the machine can sleep before the lock
  is confirmed.
- **A separate lock-before-sleep daemon.** The shell already owns the lock UI,
  the protocol, and logind; a second process adds a failure domain and a bus
  round trip on the critical path.
- **Blocking in the signal handler without a bound.** A hung compositor must
  not hold the machine awake for the whole logind delay allowance.

## Consequences

- `tst_locksuspend` runs a fake logind on a private bus: it asserts one lock
  request, a confirmed lock before the handler returns, one inhibitor release,
  duplicate idempotence, and a wake that only refreshes and wakes DPMS.
- A logind that rejects the inhibitor (polkit, or a daemon that is not up yet)
  weakens only the ordering guarantee; the gate logs it once and retries on the
  next wake. A logind service restart drops the old fd, so the next wake's
  re-take is the recovery point.
- The compositor's own suspend quiesce remains its DRM-session concern; the
  shell does not send a second suspend request for a suspend it did not start.
