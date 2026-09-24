# 0009 — Idle/animation trace instrumentation and the render-stats line

## Status

accepted

## Context

T-03.1a must prove two budgets with the same instrument: a 60 s idle window
with **zero damage and zero client wakeups** (FR-2), and **one compositor
frame per animation frame** while a transition runs (asserted by
[ADR 0003](0003-shared-animation-clock.md)). Earlier tasks already emitted
`frames_rendered`, `frames_skipped_no_damage`, `direct_scanouts` and
`animation_frames_stepped` on `SIGUSR1`/clean exit, but "client wakeups" had
no counter — a no-damage frame skip was only *argued* not to wake clients.
T-03.1b (latency, scanout), T-03.4 (on-hardware budgets) and T-16.11
(packaged re-measure) all need to read the same numbers, so the shape of the
line is a contract.

## Decision

- `DfState::dump_stats`'s `render stats` line is the single trace contract.
  Fields are positional and append-only, in this order:
  `frames_rendered`, `frames_skipped_no_damage`, `direct_scanouts`,
  `animation_frames_stepped`, `client_wakeups`.
- `client_wakeups` (`RenderStats::client_wakeups`) counts **frame-callback
  batches handed to mapped windows** — one per live window per presented
  frame. It is incremented in `render::post_repaint`/`post_repaint_headless`
  (nested/headless) and, on DRM, by `render::count_output_windows` because
  Smithay's `DrmCompositor` queues those callbacks itself. It must stay flat
  at zero while the desktop is idle.
- The trace window is `DF_IDLE_TRACE_SECS` (1..=600 s; default 10 s in-suite).
  `scripts/idle-trace.sh` / `make idle-trace` run the **60 s acceptance**
  window and record the raw line to `docs/captures/t03-idle-trace.txt`.

## Consequences

- Later instruments (T-03.1b latency, scanout template) **append** fields;
  they never reorder or rename existing ones, because tests and the capture
  log parse the line positionally.
- A rendered frame that presents to clients necessarily advances
  `client_wakeups`; a change that makes an idle desktop render (or wake a
  client) shows up as a non-flat counter in `idle_trace.rs`, not as a silent
  regression.
- On DRM, `client_wakeups` is an upper bound until the `DrmCompositor`
  callback path is hooked directly (T-03.2).
