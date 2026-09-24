# 0006 — Zoom/fullscreen are render-only geometry motion on the shared `WindowMotion`

## Status

accepted

## Context

T-02.3 animates between floating, zoomed, and fullscreen geometries. Two
constraints shape the decision:

- The window state and the model geometry must change **immediately** so focus,
  input regions, configure flow, and the workspace layout are never mid-flight:
  a zoomed window is genuinely zoomed (and input-correct) from the first frame,
  not after the tween ends. Animation must therefore be a drawing effect, not a
  state change.
- Unlike appear/minimize, the client surface **resizes** during the transition:
  the compositor configures a new size but the client's committed buffer lags.
  A target-relative scale (the appear/minimize transform) would render a
  stale-sized buffer at the wrong size mid-flight.

## Decision

- Reuse the one `WindowMotion`/`MotionFrame` path ([ADR 0005](0005-minimize-restore-motion-and-ghost.md))
  with two new kinds, **zoom** and **fullscreen**. They interpolate
  `origin → target` with alpha fixed at `1.0`: the window is visible at both
  ends, so there is nothing to fade.
- The state transition (`DfState::zoom_window`/`unzoom_window`/`fullscreen_window`/`unfullscreen_window`)
  applies first and immediately; the motion is started from the geometry the
  window is leaving. A new request mid-flight starts from the current
  interpolated rect, so an interrupt retargets without waiting (the T-02
  progress semantics).
- `MotionFrame::scale_for(surface_size)` maps a surface of the *current* size
  onto the interpolated rect. Appear/minimize draw a buffer already at the
  target size, so it equals the target-relative `MotionFrame::scale`;
  zoom/fullscreen use it so the geometry stays continuous across a buffer-size
  change. Compositor-drawn chrome (the SSD titlebar) keeps using the
  target-relative `scale` because its natural size is always the target.
- Zoom and fullscreen share the `WINDOW_OPEN` motion token; no new token is
  introduced.

## Consequences

- T-04 composes scale/clip/blur onto the same `MotionFrame`; it must respect
  `scale_for` for client surfaces.
- T-16 extends these motions to multi-monitor geometry (the target is drawn
  from `output_bounds_for`, which is per-output already).
- The render transform lags the client's resize: until the client commits the
  new buffer, the old buffer is scaled to the interpolated size (correct), and
  once the motion completes the un-scaled draw relies on the client having
  committed. A slow client can show a brief size pop at completion; a snapshot
  path is a T-04/post-gate refinement.
