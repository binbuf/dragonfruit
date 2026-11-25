# 0360 — Chrome stability fixtures are a manifest-gated expected-failure suite

## Status

accepted

## Context

T349 starts the stability-2 set. The set must capture repeatable pre-fix
behavior for the status anchors and the Dock chooser before T350–T352 change
them, and it must give every later task a deterministic chrome check that runs
without a native seat. A normal Qt test suite cannot hold both halves: the
reproducers are supposed to fail on the starting revision, while the gate must
be green.

The chrome findings also span evidence classes. A QML fixture runs on the
software scene graph and sees the declared geometry and the composed QML
silhouette. A GPU ring check reads back the producer's FBO. A nested capture
sees the compositor's composed output. Native scanout sees the DRM frame. Those
are not interchangeable, and a green fixture cannot stand in for a native
presentation claim.

## Decision

- **One deterministic gate.** `make verify-chrome-stability` runs the popup/menu
  Qt suites, `shell_protocol_conformance`, `animation_clock`, `latency_trace`,
  the material geometry checks, the fixture manifest unit tests, and the
  offscreen fixture suite. It needs no GPU and no native seat, so it runs in
  both evidence cases.
- **The manifest is the expected-failure contract.**
  `tests/chrome-stability/expected-failures.json` names every fixture case, its
  expected result, the owning task, the setup, and the symptom. A pre-fix
  reproducer is `expected: fail`; the gate is green only while it still fails.
  When the owner lands, that task flips the row to `pass` and the gate enforces
  the fix. An unexpected pass or an unlisted fixture fails closed.
- **The fixture suite is offscreen and separate.** `tst_chromestability` is
  built but is not a CTest target. `tests/chrome-stability/run_fixtures.py`
  parses its per-function results and compares them with the manifest. The
  report is written under `docs/captures/stability2/` and is local evidence,
  not a checked-in golden.
- **Evidence classes stay explicit.** The fixture report says Qt Quick
  Test/offscreen. `make t349-gpu-smoke` runs the producer ring check and writes
  its own nested-GPU artifact. `make t349-chrome-baselines` runs the real Dock
  chooser through the T238 driver for shared-memory and nested-GPU counters.
  A missing host session produces an explicit `OPEN` row with the exact command;
  it never skips the deterministic fixture gate.
- **The GPU ring check is a regression probe.** It paints three distinct slots,
  reuses two older slots with an unchanged scene and a 1×1 logical damage
  report, compares the whole frame and a pixel outside the damage box, and
  checks premultiplied alpha at 1×, 1.5×, and 2×. It does not claim native
  scanout or compositor import.

## Consequences

- T350/T351 must update their manifest rows; T352 may add the menu-bar crop
  rows it needs. A later task cannot silently leave a fixed reproducer in the
  expected-failure list, and a regression cannot hide behind an expected pass.
- New chrome fixtures must add a manifest row. The runner reports an unlisted
  test as a gate failure.
- A green `make verify-chrome-stability` is agent evidence only. Native
  presentation and scanout remain the T363/T365 rows with their own commands.
- Capture artifacts stay under the ignored `docs/captures/stability2/`
  directory; the manifest, runner, and scripts are tracked source.
