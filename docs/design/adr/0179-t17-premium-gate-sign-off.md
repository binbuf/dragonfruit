# 0179 — The T-17 premium-gate sign-off is a report over per-unit evidence, with two hardware rows left open

## Status

accepted

## Context

The premium gate ([17-premium-gate.md](../tracks/17-premium-gate.md)) is a
checklist, not an implementation: the loop, the visual floor, the performance
budget, robustness, and product judgment. Its nine units each landed their own
capture and conformance. T-17.6 is the last unit before the hardware rail: it
must tick or explicitly waive every checklist item, list the post-gate backlog,
and commit the report and reproduction commands
([SLICING-REVIEW.md](../../SLICING-REVIEW.md): the human half is batched at the
track boundary; the agent half is a report plus a capture).

Two checklist groups cannot be closed on this host. DRM is unavailable (the
host session owns DRM master; [14-risks.md](../14-risks.md)) and the baseline
Intel/AMD frame trace needs hardware this host does not have. Those are T-17.2
and T-17.4, which run *after* this unit in the execution order. The gate rule is
"if unavailable, the gate is marked incomplete, not passed".

The human unfamiliar-user test is un-automatable by definition. The slicing
review already moved it to the batched human sign-off; T-17.6 owns the protocol
and the agent-legibility evidence, not the human verdict.

## Decision

- **The sign-off is a report, not a new product surface.** It is
  `docs/captures/t17-premium-gate.md`, reproducing the track checklist with a
  verdict and an evidence link per item. It owns no new feature.
- **Agent-completable closure.** A checklist item is closed when a committed
  capture, a green conformance suite, or a headless transcript proves it. The
  report is the index; the per-unit captures stay authoritative.
- **Explicit open rows.** "Full loop on DRM" and "60 Hz zero dropped frames on
  baseline Intel/AMD" are marked **OPEN**, owned by T-17.2 and T-17.4. The gate
  is therefore **incomplete, not passed**, until those land on hardware.
- **The unfamiliar-user test is a protocol now and a human verdict at the
  boundary.** T-17.6 records the walkthrough steps, the legibility criteria, and
  an agent legibility review of the committed stills; the human test remains the
  batched boundary item.
- **The assembling capture is scripted.**
  `scripts/capture-t17-premium-gate.sh` (`make t17-premium-gate-capture`) runs
  the assembled nested desktop, captures `docs/captures/t17-premium-gate.png`,
  re-runs the fast headless rows, and writes
  `docs/captures/t17-premium-gate.txt`. The final acceptance is `make check`
  (lint + test + 100-cycle soak) and `make e2e` green on the tree.
- **The desktop-name gate is corrected** so `make check` can be green: the
  `org.<name>.<member>` namespaces are third-party identifiers, not
  XDG_CURRENT_DESKTOP hardcodes (see [0180](0180-third-party-identifier-desktop-name-gate.md)).

## Consequences

- The report is a stable index: future work cites `t17-premium-gate.md#<item>`
  instead of re-deriving a verdict from the raw captures.
- The gate is honestly incomplete: the DRM and Intel/AMD rows are follow-ups,
  not silent passes. A future reviewer can point at the two OPEN rows instead of
  re-reading the track.
- The post-gate backlog in the report is the list carried out of the gate;
  nothing else may be implied after it.
- The report changes no code; regenerating it needs only the host session tools
  the other T-17 captures already require.