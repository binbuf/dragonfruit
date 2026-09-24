# 0018 — Mission Control drag drops onto the workspace-strip card under the pointer

## Status

accepted

## Context

T-05.3 drags a **live** Mission Control representation onto another Space and
assigns the window there. Only the active Space's surfaces are in the grid —
neighbour-Space reveal (drawing surfaces unmapped from the `Space`) is a later
T-05 slice — so the destination Space is not visible inside the grid. The shell
already draws the workspace strip (one card per Space) from shared design
tokens and already owns an accessible title-card drag whose drop target is the
card under the pointer. The question was how the compositor resolves the
destination for a drag of the live representation.

## Decision

- A left press on a live representation begins a `GridDrag`
  (`compositor/src/overview/grid.rs`): window, press point, current point, with
  `DRAG_THRESHOLD` (8 logical px, the shell grid's own
  `DragHandler.dragThreshold`) as the click/drag discriminator. Motion
  translates that one window's grid frame so the live surface follows the
  pointer; the SSD titlebar and shadow follow through the existing
  `window_render_frame` seam.
- The drop target is the **workspace-strip card under the pointer**. The
  compositor reproduces the shell's centered strip layout from the shared
  tokens (`strip_card_rect`/`strip_space_at`): menu-bar height + strip margin at
  the top, `CARD_WIDTH` cards `STRIP_GAP` apart, centered horizontally.
- A drop on a different Space calls `DfState::move_window_to_space` — the one
  assignment primitive, which also updates the app's Space memory and reflows
  the scene — and leaves the overview open. A drop off the strip or on the
  source Space is cancelled; a press that never crossed the threshold is the
  T-05.2 selection.
- The shell title-card grid stays the keyboard/accessible path; this is an
  additional pointer path over the live surfaces.

## Consequences

- There is one assignment primitive (`move_window_to_space`) and one hit/drop
  geometry model; T-05.4 (wallpaper slide) and T-05.6 (frame budget) do not
  grow a second drag resolver.
- The strip geometry is duplicated in Rust and QML from the same generated
  tokens, not a hand-copied constant; if the shell restyles the strip, the
  tokens move both. Multi-monitor drag edge cases remain T-16.
- When neighbour-Space reveal lands, the drop target can widen to the revealed
  Space regions without changing the drag state machine; only
  `overview_drop_space_at` grows.