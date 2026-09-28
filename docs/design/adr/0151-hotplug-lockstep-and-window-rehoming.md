# 0151 — Hotplug joins lockstep, and detach re-homes windows

## Status

Accepted (T-16.2).

## Context

The workspace model is lockstep: a switch advances the active Space on every
display together (macOS semantics). Two gaps surfaced once hotplug was
scriptable headlessly:

- `WorkspaceModel::add_output` gave a newly attached display a fresh three-Space
  list with `active: 0`. If the other displays were on a non-zero Space, the new
  display was desynchronised, and `switch_all` (which moves each display by
  `delta` from its own index) kept them desynchronised forever.
- `DfState::on_output_removed` migrated a detached display's window
  **assignments** to the primary's active Space, but left each window's geometry
  at its old location on the dead display. The window stayed tracked yet was
  off every remaining output — a lost window to the user.

## Decision

1. `add_output` mirrors the existing displays' Space *shape* (list length and
   `fullscreen_for` markers) and adopts their active index (clamped to the list
   length). The first output still creates the fresh three-Space list.
2. Before a detached output leaves the `Space`, `relocate_windows_off_output`
   re-homes every window that was on it: a window is "on" the removed output
   when the scene maps it there or its stored geometry still overlaps the
   removed output. Floating windows cascade into the primary's usable area,
   zoomed windows re-fit it through the decoration insets, and fullscreen
   windows fill the primary. `WindowStateMachine::relocate` preserves each
   window's state (a minimized window updates the geometry of the state it will
   restore to).

## Consequences

- A switch after a hotplug advances every display to the same Space index; the
  headless gate `workspace_switch_stays_lockstep_after_hotplug` fails if either
  half regresses.
- No window is lost on detach; `hotplug_under_load_keeps_every_window_on_a_live_display`
  runs repeated add/remove cycles under mapped windows and asserts each window
  stays inside the remaining display.
- Imaginary fullscreen Spaces are mirrored on hotplug; `exit_fullscreen` already
  removes every instance, so the round-trip stays correct.
- Relocation is geometry policy only: it does not change the window's Space
  assignment, which `remove_output` still migrates.