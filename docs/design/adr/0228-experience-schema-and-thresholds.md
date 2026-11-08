# 0228 — The experience schema is the evidence contract, and thresholds are tighten-only

## Status

accepted — defines the T219 contract consumed by T260 and every T222–T262
surface task.

## Context

The total-experience track spans 88 surfaces, 418 inventoried state rows, and
several evidence classes: stills, motion clips, AX dumps, geometry, material
measurements, behaviour traces, host-API traces, and native scanout proof.
Before T219 each task described its own state, axes, and tolerance, so a
capture could be called "done" without a reproducible trigger, a hash, or a
falsifiable metric. T260 cannot build a comparison harness on that.

The reference campaign lives in an optional `macos-design-review`
checkout. The local inventory
(`docs/reference/desktop-experience-inventory.md`) is the Dragonfruit-facing
id source; the checkout supplies reference measurements when present. Neither
may become a runtime or build dependency.

## Decision

- **Three checked-in files are the contract.**
  `docs/captures/experience-schema.json` is the JSON Schema for a state row;
  `docs/captures/experience-manifest.json` is the inventory and the
  capture/test/OPEN mapping; `docs/captures/experience-thresholds.json` is the
  per-surface threshold set. `docs/design/experience-schema.md` is the
  human-readable contract. No shipping binary reads them.
- **Ids are stable and append-only.** A surface id is `<prefix>-<name>`
  (`S-DOCK`, `N-WIN`, `SET-DD`, `A-MENU`); a state row is `<surface>-NN`.
  Renaming a cited id is forbidden; add a new id and mark the old one
  superseded.
- **Every state has one row.** A row carries surface, state, axes, evidence
  class, reproducible trigger, acceptance metric, threshold dimensions, and a
  verification of `capture`, `test`, or `open`. The axes vocabulary is fixed
  (appearance, scale, refresh, reduce-motion, reduce-transparency,
  increase-contrast, display, locale, backend).
- **`OPEN` is first-class.** An `OPEN` row names the missing capability, the
  exact rerun command, the owner task, and the intended axes. It is debt, not
  a pass.
- **Thresholds are tighten-only.** `experience-thresholds.json` defines six
  profiles (`glass-persistent`, `glass-transient`, `opaque-content`, `hud`,
  `settings-pane`, `motion`) and per-surface overrides. A task may tighten a
  value with a design note; it must not silently loosen one. The policy field
  is `loosen: forbidden`.
- **One validator is the gate.** `scripts/t219-experience-schema.py` checks
  the schema, ids, axis vocabulary, threshold dimensions, capture hashes, test
  paths, and `OPEN` completeness, then prints captured/verified/OPEN coverage
  by surface. `make experience-schema-check` runs it and its regression tests
  inside `make gates`. It is hardware-independent and standard-library-only.
- **T260 consumes, never duplicates.** The acceptance harness reads the
  thresholds file for its still/motion/geometry/typography tolerances.

## Consequences

- A task that lands a capture or test updates its manifest row, hash, and
  axes; the hash check fails if the capture changes without the row.
- A task that cannot land evidence leaves an `OPEN` row with a rerun command
  and an owner; the coverage summary keeps the debt visible.
- A tolerance change is a contract change: edit the thresholds file, record
  the reason, and run the T260 harness. Tightening is allowed; loosening
  needs a design note.
- The macOS reference checkout remains optional. When it is absent, reference
  measurements stay `OPEN`; the local inventory and Dragonfruit captures still
  validate.
- The contract is evidence/CI only. It does not add a runtime dependency,
  encode macOS-only fields, or change any shipped surface.
