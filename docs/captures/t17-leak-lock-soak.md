# T-17.5b leak and lock enforcement — reviewed state

This is the premium-gate companion to the headless verification transcript
`t17-leak-lock-soak.txt` (`make t17-leak-lock-soak`). It re-proves two
contracts from the T-17 checklist on the release tree:

- **Teardown/leaks.** Repeated loops leak no socket, lock, launch token,
  `DISPLAY` file, orphaned client, or VT master.
- **Lock enforcement.** Locking still holds on the release build, including
  against an untrusted client and a killed lock UI.

The contract is [ADR 0178](../design/adr/0178-t17-leak-lock-enforcement.md).

## What ran

| Row | Command | Result |
|---|---|---|
| Repeated loops + leak tripwire | `XDG_RUNTIME_DIR=<scratch> target/release/dragonfruit dev --soak 100` | 100 clean cycles; scratch dir empty |
| Lock covers/unlocks | `cargo test --release -p dragonfruit-compositor --test session_lock_conformance` | pass |
| Untrusted client cannot lock | same suite | pass |
| Lock UI killed while locked | same suite | pass |
| 25× lock/unlock leak loop | same suite, `repeated_lock_cycles_never_leak_lock_state` | pass |
| VT / DRM master | `python3 scripts/drm-seat-probe.py` | **OPEN** — no free logind seat |

Both soak and lock suite run with an **isolated `XDG_RUNTIME_DIR`**, so the
transcript can assert the directory is empty afterwards instead of trusting
the run's own self-report; and because the lock suite is run with
`--release`, the conformance proof is against the same optimized artifact the
soak uses (`CARGO_BIN_EXE_dragonfruit-compositor`).

## The leak fix that came out of it

The verification found a real accumulation: every compositor integration test
hard-kills its child (`SIGKILL` skips the compositor's own teardown) and most
harnesses did not remove the Xwayland `DISPLAY` hand-off file, so
`$XDG_RUNTIME_DIR` had accumulated **hundreds** of stale
`dragonfruit-test-*` sockets/tokens/`.x11-display` files across past runs (the
known stale-runtime-file follow-up in `docs/PROGRESS.md`). Two changes close
it:

- `tools/dragonfruit-dev/src/soak.rs` — `teardown_artifacts` now includes the
  Xwayland `DISPLAY` file, so the repeated-loop tripwire actually checks it.
- `compositor/tests/common/mod.rs` (new) —
  `cleanup_compositor_artifacts(&socket)` removes socket, socket.lock, both
  launch tokens, and the `DISPLAY` file; every compositor integration-test
  harness calls it in `Drop` on every path (including a hard kill).

After the change, running the suites leaves the runtime directory count
unchanged (measured: 2037 → 2037 entries around a lock + xdnd + xwayland run).

## Live nested loop check (run for this task)

A full nested `make demo` session was launched, the desktop raised and
captured, then **cleanly SIGTERMed**:

- `docs/captures/t17-leak-lock-soak.png` — the nested Dragonfruit desktop
  (wallpaper, menu bar, Dock, Settings window, X11 demo window), cropped from
  the active-window capture `/tmp/opencode/t157-active.png`.
- Host-screen capture `/tmp/opencode/t157-desktop.png`; demo log
  `/tmp/opencode/t157-live.log`.

Vision read: all components present and correctly rendered, "no tearing,
clipping, or stray pixels". Teardown left **no** `dragonfruit-t157` socket,
token, or `DISPLAY` file and no live Dragonfruit process; the demo reported
`clean teardown — host session undisturbed`. This task changes no
user-visible surface, so the capture confirms the desktop still renders.

## The VT master half

The host runs a graphical session that holds DRM master, so a real DRM cycle
is impossible here; the seat probe is recorded **OPEN**, not skipped. The
compositor's clean teardown removes its own `DISPLAY`/X11 files and releases
the backend, and `scripts/drm-soak.sh` (`make drm-soak`) re-probes DRM master
after one real cycle on the T-03.4 hardware rail. That re-probe is the
authoritative VT-master evidence and remains open with T-03.4/T-17.2.

## Packaged build

T-16.9/T-16.10 (Fedora/Debian packaging) have not landed, so there is no RPM
or DEB to install here. This unit verifies the **release build** the packages
are built from (`target/release/dragonfruit` + `target/release/dragonfruit-
compositor`); the installed-package re-run is deferred to T-16.9/T-16.10 and
the T-17.6 human sign-off, as the track design already scopes "on the packaged
build" to the final gate.