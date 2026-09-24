# 0004 — Window appear: origin hand-off and per-window render transform

## Status

accepted

## Context

T-02.1b makes a newly mapped window scale and fade in from its Dock entry.
The Dock owns the tile geometry, so the origin can only come from the shell;
the compositor must still work with no shell (headless, CI, a non-Dock
launch). Applying the effect by moving the window's model geometry would
desync the client's input region from what is drawn and repeatedly reflow the
client, and re-implementing Smithay's `Space` render pass per backend would
fork the render path.

## Decision

- The shell hands the origin over the existing private manager as an additive
  request, `df_toplevel_manager.set_launch_origin(app_id, x, y, w, h)`
  (`since=4`, manager interface bumped to 4). It is keyed by `app_id` and
  consumed by that app's first mapped window. Absent, the compositor derives
  a **centered origin**: the final rectangle shrunk about its own center
  (`WindowMotion::centered_origin`, generalized for minimize/restore in
  [ADR 0005](0005-minimize-restore-motion-and-ghost.md)).
- The transition is window-model state (`compositor/src/window/motion.rs`):
  an origin/target pair plus the shared clock's `Tween`. The model geometry
  and the `Space` location stay the **final** geometry, so input, layout, and
  the shell's view never move.
- The effect is a **per-window render transform**: `render::window_render_elements`
  walks the `Space` in the same order as Smithay's space pass and, for a
  window mid-appear, wraps each surface element in
  `RescaleRenderElement` + `RelocateRenderElement` and fades it via the
  surface-tree alpha; the SSD titlebar applies the same frame to its rects.
  Backends render this element list directly through their damage tracker
  instead of `desktop::space::render_output`.
- Reduced motion is the clock's flag at tween construction: a zero-duration
  tween is the target on its first (and only) step, still through
  `step_window_appearances`.

## Consequences

- T-02.2 (minimize/restore) reuses the origin hand-off and the per-window
  transform; T-02.4 (close) reuses the transform for the ghost.
- T-04 composes scale/clip/blur onto the same per-window transform rather
  than adding a second effect path.
- The shell must bind `df_toplevel_manager` at v4 to send the request; the
  compositor degrades cleanly for any client that binds lower.
- Backends no longer call `space::render_output` for windows; the space's
  layer-surface handling is irrelevant here because the shell's chrome is
  composited as custom elements (T-09).
