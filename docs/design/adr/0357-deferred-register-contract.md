# 0357 — Deferred items are a checked register

## Status

accepted

## Context

The 2026-11-22 stability review split defects into P0/P1 release blockers and
P2 deferrals. Design 27 §7 held a short summary table with only "Why" and
"Promote when". A deferral could lose its owner, its user impact, or its STATUS
row without any gate noticing, and several review items had no STATUS row at
all. That is a silent omission with extra prose.

The daily-driver gate is only honest if an accepted deferral is a recorded
decision. The record needs to say why the item is out, who owns it, what the
user loses while it is out, and what condition would promote it.

## Decision

- **One register.** Design 27 §7 is the only list of accepted P2 deferrals.
  Every row names the deferred item, the reason, the user impact, an owner
  document or task, a promotion trigger, and the open gap id in `STATUS.md`.
- **The STATUS row is part of the decision.** A new deferral adds a row in §7
  and an open gap row in `STATUS.md` in the same change. A STATUS id that is
  missing or already resolved fails.
- **The gate is a parser, not a roadmap.** `scripts/check-deferred-register.py`
  parses the table, requires all six fields, requires every review item, and
  runs in `make verify-stability`. It does not rank, schedule, or duplicate the
  task set.
- **No deferred item is implemented by the register task.** Promotion needs a
  task, a test, and a hardware evidence row; the row then leaves §7 and its
  STATUS row is closed.
- **The owner is concrete.** An owner must name a document (`.md`), an ADR, or
  a task id. "Someone" and "TBD" fail the check.

## Consequences

- A P2 item cannot disappear silently: the gate fails on a missing row, a
  missing field, a placeholder owner, a duplicate, or an unknown/resolved
  STATUS id.
- Adding a legitimate deferral is deliberately a two-file review point (design
  27 §7 and `STATUS.md`), not a sentence in an unrelated hand-off.
- The register stays small. It records the boundary of the daily-driver gate;
  it does not become a second backlog or a scheduling surface.
- Future docs that mention a deferral should link to its §7 row instead of
  inventing a second list.
