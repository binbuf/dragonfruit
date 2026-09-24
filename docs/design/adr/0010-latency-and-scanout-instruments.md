# 0010 — Input-to-photon latency instrument and the direct-scanout template

## Status

accepted

## Context

T-03.1b must measure the two remaining T-03 budgets that T-03.1a did not:
**input-to-photon latency under one frame** in nested runs (FR-3) and a
**direct-scanout counter** the DRM rail (T-03.2/T-03.4) can reuse (FR-5).
[ADR 0009](0009-idle-trace-instrumentation.md) reserved the `SIGUSR1`
render-stats line as a positional, append-only contract and named T-03.1b as
the next appender; this record fixes the method and the fields so T-03.2,
T-03.4, T-16.11 and T-17 all read the same trace.

There is no hardware photon sensor available to the compositor. The only
honest thing the compositor can measure is the delay from an input event
entering its router to the next frame it actually presents — for nested, to
`WinitGraphicsBackend::submit`; later, for DRM, to `DrmCompositor::queue_frame`.
The instrument must be backend-agnostic so the headless backend (CI) exercises
the identical code path.

## Decision

- **`instrument::LatencyInstrument`** lives on `RenderStats`. Input routing
  (`input::process_input_event`) calls `note_input` (the **earliest** input
  since the last presented frame is kept, so a batch reports its worst leg);
  the presenting backend calls `note_present` — nested `render::post_repaint`,
  headless `render::post_repaint_headless`, DRM `backend::drm::render_surface`.
  A pending input older than **250 ms** when a frame presents is **discarded**,
  not credited (`latency_dropped`), so an idle pointer move that produced no
  damage cannot pollute an unrelated later redraw.
- The render-stats line appends, after the T-03.1a `client_wakeups`:
  `latency_us_last`, `latency_us_max`, `latency_samples`, `latency_dropped`.
  Order is frozen; later instruments append only ([ADR 0009](0009-idle-trace-instrumentation.md)).
- A dedicated `latency stats` line and a `query latency` synthetic reply expose
  `last_us`, `max_us`, `samples`, `dropped` and the 60 Hz pass/fail; the
  `scripts/latency-trace.sh` + `scripts/latency-probe.py` capture records the
  nested raw samples to `docs/captures/t03-latency-nested.txt`.
- **`instrument::ScanoutCounter`** is the direct-scanout counter template: a
  read-only snapshot of `direct_scanouts`, `frames_rendered` and
  `frames_skipped_no_damage` from `RenderStats`, with `advanced_since` /
  `engaged_since`. DRM already advances `direct_scanouts` when
  `DrmCompositor` returns a primary-plane client buffer; T-03.2/T-03.4 take a
  before/after snapshot around the expected conditions (an unobstructed
  fullscreen client advances it; a composited window/chrome does not). A
  `scanout stats` line and `query scanout` reply emit the snapshot.

## Consequences

- The T-03 budget "input-to-photon under one frame" is asserted as
  `latency_us_last <= 1_000_000 / 60` by the headless `latency_trace.rs`
  integration test and the nested capture's verdict; the nested run is a real
  measurement (input router → winit submit), not a synthetic one.
- The measurement deliberately excludes display scanout latency and vblank; it
  is a compositor event-loop latency, which is what the compositor controls.
  T-03.4 documents that limitation when it fills the DRM run.
- `latency_samples` is the count of credited inputs; a flat/zero count while
  idle is the honesty check (the headless test asserts it).
- `ScanoutCounter` owns no state and reads only `DfState::stats`; a future
  counter (VRR, per-plane) appends a field/report rather than changing these.
