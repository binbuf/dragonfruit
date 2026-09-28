# 0159 — The T-17.1a nested window-loop capture is a scripted active-window harness

## Status

accepted

## Context

T-17.1a verifies the 30-second window loop on the nested session and commits a
capture (launch → appear → traffic lights → move → zoom → minimize → restore →
close) for a third-party Qt SSD client, an X11 client, and a CSD client. The
per-behaviour conformance already exists headlessly
(`window_conformance`, `xwayland_conformance`, `milestone_e2e` in `make e2e`),
but a capture must show the loop on a real nested session, where the demo maps
several clients at once.

Three properties of the live nested session make the older T-01 capture driver
(`scripts/capture-demo-driver.py`) unusable for T-17: the nested window is one
host window among others (so it must be captured by active-window, not by a
solid wallpaper colour, since T-18 ships a photo wallpaper); the first-party Qt
apps are CSD by design (`apps/settings/SettingsWindow.qml`), so the loop's SSD
traffic lights need a real third-party Qt client; and the demo maps overlapping
windows, so a window's titlebar can be occluded.

## Decision

- **Capture by active window.** `scripts/t17-window-loop-driver.py` raises the
  nested window through the host compositor's scripting D-Bus and crops the
  alpha-trimmed active-window still to the 1920x1200 nested output (the T-16.7
  pattern). The capture does not depend on the wallpaper pixels.
- **Classify windows from the compositor.** The driver reads `query
  decorations` and `query identity`; SSD vs CSD comes from the decoration
  report, and the loop window is the server-side-decorated third-party Qt
  client (`kcalc`) launched against the private socket. The first-party CSD
  Settings and the X11 `xmessage` client are the CSD and X11 arms.
- **Clear the stage before the loop.** A window that intersects the loop
  titlebar is dragged out of the way first, using the same titlebar-drag path
  (the drag is stepped and pauses after the press so a CSD client's
  `xdg_toplevel.move` round trip is not lost).
- **Restore runs through the compositor primitive.** After the real yellow-light
  minimize, the agent capture restores via the compositor's `restore <id>`
  primitive — the same lifecycle primitive a Dock tile invokes. The Dock-tile
  click itself is *not* synthesized in the agent capture: `kcalc` is an
  unpinned running app, so its tile appears and the pinned tiles re-center,
  which makes a Dock-band pixel diff ambiguous and can launch a pinned app
  instead of restoring. The Dock-tile geometry is covered by the headless
  T-01/T-02 suites and by the human walkthrough; the minimized still records
  the Dock state.
- **The headless suites stay the conformance.** T-17.1a adds no new headless
  test because the whole loop is already pinned per behaviour
  (`traffic_lights_drive_zoom_minimize_and_close`,
  `titlebar_drag_and_double_click_move_and_zoom`,
  `window_menu_runs_zoom_minimize_close_and_move_to_space`,
  `x11_traffic_lights_drive_zoom_minimize_and_close`,
  `ssd_toplevel_carries_a_titlebar_and_csd_does_not`,
  `decoration_tier_matrix_default_explicit_ssd_and_csd_side_by_side`, and
  `milestone_e2e`).

## Consequences

- The capture is reproducible non-interactively on any host Wayland session
  with `spectacle`, Pillow, and (optionally) `kcalc`; without `kcalc` the loop
  runs on the X11 window and the deviation is recorded.
- T-17.1b/T-17.1c and T-17.2 (the DRM loop) reuse the active-window capture,
  window classification, and clear-the-stage helpers rather than re-deriving
  them.
- The Dock-tile click remains a human-verified part of the gate; if automation
  is wanted later it needs a stable Dock-tile locator (the running/minimized
  tile is not at a fixed x), which is a separate tooling task.