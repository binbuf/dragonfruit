# 0005 — Minimize/restore reuse the per-window motion transform; the ghost renders from the model

## Status

accepted

## Context

T-02.2 animates a window shrinking into and growing out of its Dock entry's
tile. T-02.1b already established the origin hand-off and a per-window render
transform for the appear transition. Two constraints shape minimize:

- The window must be **input-inert and out of the layout** the moment it is
  minimized (focus must not stay on an invisible surface, and a "ghost" must
  not be hit by the pointer). Moving the model geometry to animate would
  desync the client's input region, and keeping the minimized window mapped in
  `Space` would leave a phantom hit target.
- The shared clock must step each motion exactly once per frame. Registering
  one clock closure per motion (as appear did) steps the whole motion set once
  per closure per frame when several motions are live.

## Decision

- Generalize the appear transition into a single `WindowMotion`
  (`compositor/src/window/motion.rs`) with a kind: **appear**, **restore**, or
  **minimize**. Appear/restore interpolate `origin → target` with alpha
  `0 → 1`; minimize is the exact reverse (`target → origin`, alpha `1 → 0`).
  One `MotionFrame` serves all three; the render transform is always relative
  to the window's committed `target`, so input and layout never move.
- Minimize **unmaps the window immediately** and broadcasts its state; the
  surface is held as a **ghost rendered from the window model** (not from
  `Space`) until the motion completes. Restore maps the window immediately and
  animates it back out of the origin.
- **One motion driver** is registered with the clock; `DfState::window_motion_driver`
  prevents a second registration, and the driver steps every window motion
  once per clock frame.
- The `app_id` → Dock-tile map (`set_launch_origin`) is **remembered**, not
  consumed, so minimize/restore reuse the same tile as the appear; absent, the
  same centered fallback is derived.
- Reduced motion (or `dock.minimizedAnimation=none`) is a zero-duration tween:
  the motion completes on its first step through the same path.

## Consequences

- T-02.4 (close) reuses `WindowMotion` for the fade/scale-out ghost; the
  ghost-lifetime rule (removed from the model only at completion) is the same.
- T-04 composes scale/clip/blur onto the one `MotionFrame` rather than adding
  a second effect path.
- Backends render minimizing ghosts by walking `WindowModel::active_motions`
  in addition to `Space` (`render::window_render_elements`,
  `render::titlebar_render_elements`).
- A minimizing window is never consulted by hit-testing, hover, or the
  workspace layout.
- The shell must still resolve `desktopId` → compositor `app_id` (T-23) before
  it can send the real Dock tile; until then real launches use the centered
  fallback, exactly as appear did.
