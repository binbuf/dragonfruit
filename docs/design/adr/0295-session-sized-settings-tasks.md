# 0295 — Session-sized Settings tasks

## Status

Accepted 2026-11-19. The task-sizing clause (item 8) of
[ADR 0287](0287-system-settings-visual-parity.md) and the old session-sizing
prose in the `system-settings-2` README/roadmap are **superseded by 0295**.
This ADR does not supersede the visual or IA target in
[`25-system-settings-visual-parity.md`](../25-system-settings-visual-parity.md)
or any pane decision in ADR 0287.

## Context

T289 (Battery + Displays) was accepted after the code landed in `3b31604` and
the targeted tests passed 3/3. The harness had parked it on the 180-minute
wall-clock ceiling after a session hit the provider's 30-image limit while
chasing the optional live `make demo` check. That was an infrastructure block,
not a code failure; re-running the task is the wrong move.

T290 was already split into `T290a`–`T290g`; that split is good and stays. The
remaining plan still contained tasks that spanned two or more panes or
surfaces, so the same failure could repeat. The roadmap needed a session-sized
stopping rule instead of an open-ended chase for screenshots.

## Decision

1. **One pane or one coherent surface per task.** Split a task before starting
   if it spans more than one pane or a distinct sub-pane. Do not discover the
   split mid-session. Use the existing letter-suffix scheme and the parent's
   number.
2. **Target roughly 90 minutes of active work.** If the task is not converging
   by then, stop with a hand-off and split it; do not start a second marathon.
3. **A targeted verify per task.** Each task names a deterministic test
   command and ends when that command is green. The full `make gates`/
   `make verify-fast` is the release gate in T302b, not a per-task requirement.
4. **Bounded captures.** Use the task's deterministic capture target for
   stills. Limit interactive screenshots to at most 6 per session; do not
   accumulate a 30-image request.
5. **No Wayland, no block.** If the host has no Wayland session, record the
   live `make demo` check as `OPEN` with the reason and hand off. The
   deterministic capture and the targeted tests are the agent-done evidence.
6. **Agent-done bar.** A task is agent-done when its targeted verify is green,
   its capture artifact exists (or is `OPEN` with a reason), its docs are
   updated, and its `## Hand-off` is filled in.
7. **Infrastructure blocks are accepted after green code.** A harness block on
   the wall-clock ceiling or the provider's image limit, after the code is
   committed and the targeted verify is green, is accepted with a note; it is
   not a reason to re-run the task.
8. **Human sign-off is batched.** The per-pane review checklist is evidence for
   the release boundary; T302b records the human verdict. A checked task is
   agent-done, not human-signed-off.

## Consequences

- The remaining `system-settings-2` work is split into session-sized children:
  T291a–T291e, T292a–T292b, T293a–T293b, T295a–T295b, T296a–T296b,
  T297a–T297b, T298a–T298b, T299a–T299c, T300a–T300d, and T302a–T302b.
  T294 (Search) and T301 (Printers & Scanners) stay single tasks because each
  is one pane.
- T290a–T290g are unchanged. T290g's Appearance ADR is renumbered from 0295 to
  0296 because this ADR takes 0295.
- The design targets and pane inventory in
  [`25-system-settings-visual-parity.md`](../25-system-settings-visual-parity.md)
  and ADR 0287 are unchanged. This is a planning and session-sizing pivot, not
  an architecture change.
- A task that ends `OPEN` for the live check is still accepted when its
  deterministic evidence is green and the hand-off names the missing
  host/seat.
- The release gate and the human sign-off move to T302b; the earlier tasks do
  not re-run the full gate.
