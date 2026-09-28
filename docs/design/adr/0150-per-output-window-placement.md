# 0150 — New windows open on the focused output

## Status

accepted

## Context

Since T-04 the placement policy in `DfState::map_pending_windows` resolved
the target from `primary_output()` — `space.outputs().next()` — so every new
window, on any display, opened on the first output. The per-output cascade
counter and transient-dialog centering already existed, but the *output*
choice did not. T-16.1a then made reserved zones and chrome geometry
per-output and explicitly deferred the placement half here, noting that
`usable_geometry_for` being per-output was the contract T-16.1b builds on.

The open question was what "focused output" means. The compositor tracks
keyboard focus (`active_window`) and has `output_name_under_pointer` (already
used for per-output chrome), but no persistent "focused output" field. A
window-focus-only rule cannot direct a window to a display that has no windows
yet, because focusing a display requires a window already there.

## Decision

- **Focused output = output under the pointer, else the active window's
  output, else the primary output.** `DfState::focused_output()` is the one
  resolver; it returns the output name and its full logical geometry. Pointer
  first makes new windows open where the user is working and lets an empty
  display receive its first window; the active-window and primary steps keep
  the policy total when no pointer is known.
- **Ordinary windows cascade inside the focused output's *usable* area.**
  `cascaded_geometry(zones.usable(output), size, index, step)` subtracts that
  output's reserved zones (`ShellProtocolState::reserved_zones_for`, T-16.1a),
  so a menu bar or Dock pushes windows clear of the chrome. The per-output
  cascade index still keys on the output name.
- **Workspace assignment follows the placement output.**
  `assign_new_window_space` now takes the output name (focused output for an
  ordinary window, the transient parent's output for a dialog) and assigns to
  that output's active Space instead of the primary's. The Xwayland path and
  `user_positioned_geometry` follow the same output choice.

## Consequences

- A new window opens on the display under the pointer; moving the pointer to
  another display and launching opens there. Both Wayland and Xwayland
  windows follow one rule.
- The headless gate is `new_windows_land_on_the_focused_output` in
  `compositor/tests/shell_protocol_conformance.rs`: two outputs, a 28 px bar
  pinned to the primary, and three mapped windows whose `query decorations`
  rects land on the primary (below the reserve), the second display (no
  reserve), and the primary again as the pointer moves.
- The rule is "pointer first", not "active window first": a keyboard
  launch while the pointer rests on another display opens there. This is the
  documented tradeoff — it matches the existing per-output-chrome pointer
  signal and is the only rule that can seed an empty display.
- T-16.2 (hotplug under load) inherits this resolver; on output removal the
  focused output naturally falls back because the removed output is no longer
  in the space.