# T-16.4 suspend/resume soak — reviewed state

This is the reviewed companion to the headless transcript
`t16-suspend-resume-soak.txt` (`make t16-suspend-resume-soak`). It covers the
T-16 acceptance row "100 suspend/resume cycles pass, no leaked state" and the
"output/input/client recovery" scope item.

The contract is [ADR 0181](../design/adr/0181-t16-suspend-resume-soak.md); the
one-cycle model it repeats is [ADR 0072](../design/adr/0072-suspend-resume-cycle.md).

## What ran

| Row | Command | Result |
|---|---|---|
| 100 cycles, one headless session, live client | `cargo test --release -p dragonfruit-compositor --test suspend_resume_conformance` | 2/2 pass; `cycles=100 suspended=0 outputs=1 windows=1` |
| Scene intact every cycle | same suite | pass (`outputs=1 windows=1` asleep and awake) |
| Client + input recovery | same suite | pass (round-trip after cycle 100; input routes after the final wake) |
| No leaked state on exit | same suite's clean `SIGTERM` check | pass (socket, lock, both tokens, `DISPLAY` file all removed) |
| Session survives 100 cycles | `cargo test --release -p dragonfruit-session --test suspend` | pass (same child pid, no restart, 100 backend requests) |
| Real machine sleeps 100 times | logind `PrepareForSleep` end to end | **OPEN** — needs a disposable machine + the logind backend |

Both suites run under an **isolated `XDG_RUNTIME_DIR`**, and the transcript
asserts that directory is empty afterwards instead of trusting the run's own
self-report. The compositor suite is already part of `make e2e`; the session
test is already part of `make e2e`'s `dragonfruit-session` row.

## Live nested check (run for this task)

A nested `make demo` session was launched, the desktop raised, and the active
window captured:

- `docs/captures/t16-suspend-resume-soak.png` — the nested Dragonfruit desktop
  (wallpaper, Dock, Settings window, X11 demo window), cropped from the
  active-window capture `/tmp/opencode/t162-active.png` (2115×1437) to the
  1920×1200 nested output.
- Demo log `/tmp/opencode/t162-demo3.log`; teardown reported
  `clean teardown — host session undisturbed`.

Vision read: the desktop renders correctly — full wallpaper with no banding,
a crisp translucent Dock, the Settings window and X11 demo window present, and
**no clipping or stray artifacts**. This task changes no user-visible surface,
so the capture confirms the desktop still renders; the soak itself is
headless.

## The hardware half

A true 100-cycle soak calls logind's suspend, actually sleeps the machine 100
times, and verifies output/input/client recovery on each wake. On this host
that would sleep the developer's own graphical session, and the production
logind `SuspendBackend` (the `SuspendBackend` trait's non-mock impl) is not yet
wired, so the row is recorded **OPEN**, not skipped, per
`docs/SLICING-REVIEW.md`. The hardware rail (T-159…T-161: free seat / spare
GPU / clean VM, plus the T-16 VM matrix) owns it. `make
t16-suspend-resume-soak` on a disposable machine is the replacement path.