# 0358 — The T347 matrix report is a PASS/OPEN contract with a native intake

## Status

accepted

## Context

T347 is the native half of the daily-driver gate: a headless pass is not a
hardware pass. The task must produce a complete matrix even when the host has
no free DRM seat, and it must not turn a missing seat into a skipped check or
a headless pass into a native PASS. T348 consumes the matrix to decide whether
the README can claim a daily driver.

An ad-hoc transcript cannot carry that contract. The report needs one row per
check, a verdict, the hardware profile, the exact rerun command, and an
artifact path; a failed headless check must block the gate; and a human on the
reference laptop needs a place to record the native result without editing the
report by hand.

## Decision

- **One runner, two artifacts.** `make t347-daily-driver-matrix`
  (`scripts/t347-daily-driver-matrix.py`) writes
  `docs/captures/t347-daily-driver-matrix.md` and a machine-readable `.json`
  companion. The `.md` is the evidence; the `.json` is for T348 and tooling.
- **Every row has a verdict, a reason, a command, and an artifact.** The
  report rows are the design 27 §5.2 checks plus the explicit launch, data,
  post-resume, media-key, and latency sub-rows. A row without native evidence
  is `OPEN`, never skipped.
- **No seat is `OPEN`, not a failure.** A host without a free DRM seat gets
  `OPEN` rows with the exact reference-laptop command. A failed headless
  conformance check is `FAIL` and blocks the gate.
- **Native evidence is a local intake file.** `PASS`/`FAIL` native verdicts are
  recorded in the gitignored `docs/captures/t347-native-evidence.json`. A
  `PASS` or `FAIL` entry must name a note and an artifact; an incomplete
  `PASS` stays `OPEN`. A headless `FAIL` always wins over an intake `PASS`
  (fail closed).
- **The runner owns no product behavior.** It captures the hardware profile,
  probes the seat, runs the existing headless suites, and composes the report.
  It does not launch a real session, sleep the host, or fix a failure.
- **Failures get follow-ups, not waivers.** The report names every `FAIL` and
  its artifact; a P0 failure is never downgraded. Filing the follow-up task is
  the implementing session's job, not a silent edit to the matrix.
- **Artifacts stay local.** The report, the logs, and the intake are under the
  ignored `docs/captures/` tree; the runner, its regression tests, and the
  design text are tracked.

## Consequences

- T348 reads one report and one intake instead of reconstructing the native
  state from scattered transcripts.
- A no-seat CI run is green only when the headless half is green; the gate
  stays `OPEN` and names every missing native row.
- A native `PASS` cannot be claimed from a headless test or a nested capture.
- The report can grow new rows without a new script, but a row that references
  an undefined check fails the report-builder regression test.
- The intake is a deliberate manual step on the reference laptop; forgetting
  it leaves the row `OPEN`, which is the safe outcome.
