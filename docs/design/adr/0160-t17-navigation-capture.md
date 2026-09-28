# 0160 — The T-17.1b nested navigation capture is a scripted state-machine harness

## Status

accepted

## Context

T-17.1b verifies the navigation half of the T-17 premium gate on the nested
session and commits a capture: workspace switching, Mission Control, and app
switching, each by pointer and keyboard
([17-premium-gate.md](../tracks/17-premium-gate.md)). The per-behaviour
conformance already exists headlessly (`window_conformance` and
`shell_protocol_conformance` in `make e2e`), but the capture must show the
paths on a live session, where the demo maps several clients and the shell's
overview/switcher chrome is composited over the compositor's live-surface
transforms.

T-17.1a established the active-window capture harness
([0159](0159-t17-nested-window-loop-capture.md)); this decision reuses it and
fixes how navigation states are observed and what is deliberately not
synthesized.

## Decision

- **Observe through the compositor's read-only introspection.** The driver
  asserts every path via `query spaces` (active Space and its window count),
  `query grid` (the Mission Control live-surface transform and strip),
  `query wallpaper` (the in-flight Space slide: direction, progress, slots),
  and `query switcher` (entries, selection, focus). A path that does not reach
  the documented state fails the capture run; a still alone is not evidence.
- **One trigger per column, same state machine.** Workspace switch: keyboard
  `Ctrl+Left`/`Ctrl+Right`, a three-finger swipe caught mid-slide, and a
  pointer click on a Mission Control workspace-strip card. Mission Control:
  keyboard `Ctrl+Up` and a top-left hot-corner dwell. App switch: keyboard
  `Cmd+Tab` (commit on modifier release) and a pointer click on a live
  preview. The strip-card geometry is reproduced from the shared tokens
  (`menu_bar::HEIGHT` + `overview::STRIP_MARGIN` + centered 132x84 cards) so
  the click lands on the same card the compositor's `strip_space_at` resolves.
- **Pointer Mission Control *selection* is out of scope for the capture.** The
  overview's pointer path here opens Mission Control (hot corner); selecting a
  window from it is the shell's centered title-card row, and the synthetic
  click did not reliably hit a card. The compositor selection round-trip is
  already pinned headlessly by
  `overview_click_selects_and_focuses_the_live_representation` (T-05.2), and
  pointer workspace switching (the strip card) is exercised live.
- **The headless suites stay the conformance.** No new headless test: every
  behaviour is already pinned by `overview_grid_transforms_live_surfaces_into_the_grid`,
  `wallpaper_follows_the_active_space_and_slides_with_it`,
  `overview_click_selects_and_focuses_the_live_representation`,
  `overview_drag_moves_the_live_representation_between_spaces`,
  `app_switcher_opens_cycles_commits_once_and_escape_cancels`,
  `app_switcher_cycles_windows_and_pointer_commits`, and the
  `synthetic_input_drives_shortcuts_hot_corners_and_gestures` /
  shortcut-walkthrough cases in `shell_protocol_conformance`.

## Consequences

- The capture is reproducible non-interactively on any host Wayland session
  with `spectacle`, python3+Pillow, and the built tree; it is not part of
  `make e2e`.
- T-17.1c and T-17.2 (the DRM loop) reuse the active-window capture and
  introspection assertions rather than re-deriving them.
- The switcher chrome (a faint scrim plus cards) is visually subtle; the
  authoritative evidence is the `query switcher` transcript and the frame
  change, with the still as the reviewer's aid.
- A stable synthetic locator for the Mission Control window-card row remains a
  tooling follow-up if pointer selection is wanted in a later capture.