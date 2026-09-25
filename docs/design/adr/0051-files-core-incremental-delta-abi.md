# 0051 — files-core streams incremental deltas, not whole snapshots

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-09-25 |
| **Task** | T-10.5 (extends [0049](0049-files-core-c-abi-bridge.md), [0050](0050-files-core-ffi-optimistic-ops.md)) |

## Context

[0049](0049-files-core-c-abi-bridge.md) made `df_files_poll` return the model's
**whole ordered snapshot** on every batch: the Qt facade cleared its
`QAbstractListModel` and rebuilt every row (`beginResetModel`). That is fine
for a small folder but quadratic for a large one — a 100k listing re-serialized
and re-copied ~10⁷ rows (each with three `CString`s), and reset the view once
per batch, so it could not hold the [09-files.md] "100k scrolls at 60 Hz with
flat memory" budget.

## Decision

The C ABI gains an **incremental delta** surface beside the snapshot one:

- `df_files_row` is `{ rank, df_files_node }`; `df_files_delta` is
  `{ status, error, reset, rows, row_count, total }`.
- `df_files_poll_delta` returns only the nodes the receiver has not seen, at
  their **final ranks** (`reset == 0`). The session keeps a `reported` id set so
  a streamed node is serialized exactly once.
- `df_files_snapshot_delta` returns a full ordered model (`reset == 1`). This is
  the path for the first paint, a sort change, and every optimistic
  begin/confirm/revert whose effect is not append-only — those also re-seed the
  reported set.
- The Qt facade applies an insertion delta by merging the new nodes into its
  display-ordered vector in one O(n+k) pass and emitting
  `beginInsertRows(oldCount, newCount-1)` (count growth) plus
  `dataChanged(0, oldCount-1)` (the existing rows shifted rank). It never
  re-serializes settled rows.
- The listing worker grows its batch geometrically (first batch stays
  `DEFAULT_BATCH`; cap `MAX_BATCH = 8192`), so the Rust sorted merge is
  O(n²/batch) with a small constant while the first frame stays prompt.

## Consequences

- A 100k listing now sends each node once (asserted headlessly) instead of
  ~100k×N rows; the Qt model's per-batch work is O(n) moves, and the view's
  paint work is bounded by the visible window because `GridView`/`ListView`
  virtualize delegates.
- The delta's ordering contract is "existing rows never reorder". If a delta
  ever violates the strictly-ascending-rank invariant, the facade falls back to
  `df_files_snapshot_delta` rather than paint a wrong order.
- The snapshot ABI (`df_files_poll`/`df_files_snapshot`) remains for the Rust
  unit tests; the Qt app uses only the delta functions.
- `SyntheticSource` (`df_files_begin_synthetic`) is a disk-free perf fixture
  gated to a `/synthetic` location, so the 100k budget is measurable without
  creating 100k inodes. It is not a shipping backend.

[09-files.md]: ../09-files.md