# 0178 — T-17.5b leak and lock enforcement: an isolated soak tripwire and test-harness cleanup

- **Status:** Accepted (T-17.5b, Phase 17)
- **Date:** 2026-09-29
- **Context:** T-17's robustness checklist requires that repeated loops leak
  no socket/token/`DISPLAY` file/orphan/VT master, and that lock enforcement
  holds on the packaged build. T-17.5a verified absent-daemon/crash behavior;
  T-17.5b owns the leak and lock half. The existing teardown soak
  (`tools/dragonfruit-dev/src/soak.rs`) already ran N compositor sessions and
  checked the socket, lock, and both launch tokens, but not the Xwayland
  `DISPLAY` hand-off file, and it ran against the shared `$XDG_RUNTIME_DIR`,
  where pre-existing junk could hide a regression. Separately, every
  compositor integration test hard-kills its child (`SIGKILL` skips the
  compositor's own teardown) and most harnesses did not remove the `DISPLAY`
  file, so `$XDG_RUNTIME_DIR` had accumulated hundreds of stale
  `dragonfruit-test-*` files — an actual leak across repeated test loops.

- **Decision:** Treat the leak tripwire as a first-class artifact and make it
  hermetic and complete:
  1. `soak::teardown_artifacts` includes the `.x11-display` file, so the
     repeated-loop gate checks every per-session hand-off file.
  2. `compositor/tests/common/mod.rs` exports
     `cleanup_compositor_artifacts(socket)`, which removes the socket, its
     lock, both launch tokens, and the `DISPLAY` file; every compositor
     integration-test harness calls it in `Drop` on all paths, including a
     hard kill.
  3. `scripts/t17-leak-lock-soak.sh` (`make t17-leak-lock-soak`) runs the soak
     and the `session_lock_conformance` suite each under a scratch
     `XDG_RUNTIME_DIR` and asserts the directory is empty afterwards, and runs
     the lock suite with `--release` so the conformance proof is against the
     optimized artifact. The known VT-master hardware half is recorded OPEN
     and referenced to `make drm-soak`.

- **Consequences:** The leak contract is now checked, not implied: a leaked
  `DISPLAY` file fails the soak, and a leaked lock-surface or lock state fails
  `repeated_lock_cycles_never_leak_lock_state` (25 cycles asserting
  `locked=0 surfaces=0` after each unlock). New compositor tests must use the
  shared cleanup helper in `Drop`. The installed RPM/DEB re-run stays with
  T-16.9/T-16.10 (not yet landed) and the T-17.6 human sign-off; this unit
  verifies the release build those packages are built from.