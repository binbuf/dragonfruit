# 0213 — Reference evidence is a hashed optional manifest; comparisons register, mask and score per metric

- Status: Accepted
- Date: 2026-11-05
- Task: T194

## Context

The Liquid Glass work starts from a macOS workstation capture set that is not
part of the Dragonfruit repository and must never become a build dependency.
The reference analysis scripts resize the candidate to the reference rectangle
and compare whole images, so a crop or a blur regression can pass as a colour
difference. The starting Dragonfruit appearance and timing also need a stable
record before any rendering work begins.

## Decision

1. **Pinned evidence is a committed manifest, not a checkout.** T194 records
   the exact reference revision, source PNGs, layout/AX dumps, JSON sections,
   analysis scripts, crop rectangles, ICC assumptions and SHA-256 hashes in
   `docs/captures/glass-reference-manifest.json`. A reference run resolves the
   root from `--reference-root`, `DRAGONFRUIT_REFERENCE_ROOT`, or the sibling
   `../macos-liquid-glass`; a requested run fails on a missing root, revision
   mismatch, or any changed source/crop/script hash. The structural check and
   all product gates work without the checkout.
2. **Colour inputs are converted once to sRGB.** Embedded ICC profiles are
   honoured (Display P3 on the workstation); files without a profile are
   treated as sRGB. Geometry is logical, with an explicit capture scale.
3. **Registration is bounded and precedes scoring.** A small integer
   translation aligns capture drift. A best shift on the search bound is a
   failure, so a real crop/geometry change cannot be hidden by registration.
4. **Metrics are independent and masked.** Colour, geometry, MTF and motion
   have separate case masks and thresholds; timing counters are compared
   separately. Every metric reports its coverage and `pass`/`fail`/`missing`/
   `not_requested` status. Zero or missing coverage is never a pass.
5. **Synthetic fixtures are the hardware-independent regression rail.**
   Neutral, checker, line-grid and saturated fixtures plus a two-frame motion
   pair live in `tests/glass/` with derived targets and provenance. The
   self-test proves detection of intentional colour, crop/geometry, MTF,
   motion, registration and timing regressions.
6. **The baseline is a record with explicit gaps.** T194 records the v0.1.1
   nested crops and T-21 counters, including backend, scale, refresh, scheme,
   material tier/path and transport. A host recapture that cannot run is an
   OPEN artifact with the exact command and reason; unmapped surfaces and
   missing settings are named gaps, not inferred passes.

## Consequences

- Later tasks compare against a stable, hashed baseline and can rerun the
  reference check only when the checkout is available.
- A capture whose alignment exceeds the bound or whose band is empty fails
  visibly; it cannot silently score as a colour match.
- The reference PNGs remain local and optional; only hashes and derived
  measurements are committed.
- T207 consumes the same masks and per-metric thresholds for the nested
  acceptance matrix; T208 owns native rows and default enablement.
