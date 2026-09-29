# 0181 — T-16.4 suspend/resume soak: the cycle count is the contract, the hardware half is a probe

- **Status:** Accepted (T-16.4, Phase 18 / T-16 platform polish)
- **Date:** 2026-09-29
- **Context:** T-12.5a proved one suspend/resume cycle recovers outputs, input,
  and clients without a restart, and [ADR 0072](0072-suspend-resume-cycle.md)
  left the T-16 obligation explicit: "the later 100-cycle soak asserts
  `suspended=0 cycles=N` and a live scene; the counter is the contract." The
  task is the last hardening unit before the T-16 graphics-driver matrix
  (T-16.5). A suspend is destructive to the machine running it, and no
  production logind `SuspendBackend` exists yet (the `SuspendController` seam
  still has only its recording `MockSuspend`), so the real 100-sleep soak
  cannot run on this host without ending the developer's own session.

- **Decision:** The 100-cycle soak is proven in two headless halves plus a
  recorded hardware probe.
  1. **Compositor.** `compositor/tests/suspend_resume_conformance.rs` gains
     `one_hundred_suspend_resume_cycles_leak_no_state`: one headless session,
     one live Wayland client, 100 `suspend`/`resume` round trips. Each cycle
     asserts `outputs=1 windows=1` both asleep and awake and the cycle counter
     reaches `cycle`; after the 100th wake the client connection round-trips
     and input routes; then a clean `SIGTERM` asserts no socket, lock, launch
     token, or `DISPLAY` file leaked. The suite is already in `make e2e`.
  2. **Session.** `services/session/tests/suspend.rs` gains
     `one_hundred_cycles_keep_the_supervised_session_alive`: 100 cycles
     through the real `Supervisor` and `MockSuspend`, asserting one platform
     request per cycle, an unchanged child pid, and zero restarts.
  3. **Hardware probe.** `scripts/t16-suspend-resume-soak.sh`
     (`make t16-suspend-resume-soak`) runs both halves under an isolated
     `XDG_RUNTIME_DIR` (asserted empty after) and writes
     `docs/captures/t16-suspend-resume-soak.txt`. The real-machine row —
     logind actually sleeping the box 100 times — is recorded **OPEN**, not
     skipped, because it needs a disposable machine and the logind backend.
  4. `compositor/tests/common/mod.rs` exports `compositor_artifacts(socket)`,
     the one leak list the cleanup helper and the soak assert against.

- **Consequences:** The suspend/resume contract is now a repeatable count, not
  a one-shot: a leaked output, window, connection, or hand-off file fails the
  soak, and the counter must reach exactly 100. The real 100-sleep soak stays
  on the hardware rail (T-159…T-161 / the T-16 VM matrix) until the logind
  `SuspendBackend` lands and a clean VM is available; replacing the OPEN
  artifact is `make t16-suspend-resume-soak` on that machine. New per-session
  hand-off files must be added to `compositor_artifacts` and
  `soak::teardown_artifacts` together.