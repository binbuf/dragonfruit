# 0311 — Stability is the daily-driver release gate

## Status

Accepted (2026-11-22). Defines the `stability` task set (T304–T348) and the
contract in [`27-stability-and-daily-driver.md`](../27-stability-and-daily-driver.md).

## Context

The 2026-11-22 review found that the tree builds, `cargo test --workspace` and
the visual suite pass, and the feature breadth is real. It also found defects
that make a normal workday unsafe or frustrating:

- an active popup grab can receive keystrokes while the session is locked;
- a shell crash while locked leaves a frozen, inputless session;
- keyboard focus is lost after a Space round-trip, minimize, client crash, or
  shell crash;
- the session has no idle timer, no DPMS, and no lock-on-suspend;
- X11 apps launch without `DISPLAY`;
- the Mouse/Trackpad settings and the brightness slider have no hardware
  consumer;
- Log Out is a live no-op;
- a clipboard client that never reads can freeze the shell;
- portal requests hang when the shell restarts;
- a cross-filesystem directory move can partially move data.

Most of these are already named in [`STATUS.md`](../STATUS.md) as partials or
`OPEN` rows. The project has been treating them as background polish while
adding Settings panes and visual parity. That ordering no longer matches the
product claim: a desktop with a large surface area that cannot lock, dim, or
launch an X11 app is not a daily driver.

## Decision

1. **A new `stability` task set owns the work.** It continues the repository
   task number sequence at **T304** and ends at **T348**. The task files live
   in the local, gitignored `docs/tasks/stability/`; the contract and ADRs are
   tracked in `docs/design/`.
2. **No new surface area while the gate is open.** Do not add Settings rows,
   Settings panes, shell chrome, material variants, or new animations as part
   of this set. Fixes may add a private protocol request, a test, or a
   diagnostic; they may not expand the product surface.
3. **The contract is the bar.** The invariants in
   [`27-stability-and-daily-driver.md`](../27-stability-and-daily-driver.md) §3
   are the acceptance criteria. A task is not done because a test passes in
   isolation; it is done when its invariant holds.
4. **`make gates` is necessary, not sufficient.** The release gate also
   requires the reference-hardware matrix in T347 and the batched human
   verdict in T348. Nested/headless evidence cannot prove a panel turned off,
   a touchpad tapped, or a lid closed.
5. **The README status flips only at T348.** Until then the note that
   Dragonfruit is “not yet intended as a stable daily-driver desktop” stays.
   If T347 has an `OPEN` P0 row, the note stays and the blocker is named.
6. **Settings truth is a gate, not a preference.** A key marked
   `stored_only: false` has a named consumer and a test that observes the
   effect, or it becomes `stored_only: true` and the pane says so
   ([ADR 0316](0316-settings-consumer-conformance.md)).

Rejected alternatives:

- **Keep fixing by area.** The review already found the defects; a new
  feature set would defer them again.
- **Call the gate “performance.”** Most blockers are correctness, security,
  and wiring, not frame time.
- **Require all P2 items.** IME, AT-SPI, HDR/VRR, multi-GPU, and mixed-DPI are
  real gaps but not the minimum for one reference laptop; they are recorded in
  the deferred register.

## Consequences

- The task set is local; the design contract and ADRs are tracked. Do not
  force-add `docs/tasks/stability/`.
- Every landed task updates its row in design 27 §4 and the matching
  `STATUS.md` gap row. A task that changes an invariant updates §3.
- A P0/P1 defect found during the set gets a task id before it gets a fix.
- The daily-driver claim is a tested hardware profile, not a general promise.
  The README records the profile and the known limits.
- New surface work resumes after T348; the deferred register is the input to
  the next set.
