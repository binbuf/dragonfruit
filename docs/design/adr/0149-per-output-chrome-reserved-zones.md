# 0149 — Reserved zones and chrome geometry are resolved per output

## Status

accepted

## Context

Since T-07 the private shell protocol has placed chrome surfaces and reported
reserved zones, but `DfState.reserved_zones` was a **single global union**:
`refresh_reserved_zones` folded every chrome surface into one `ReservedZones`
and `send_output_properties`/`broadcast_reserved_zones` sent that union to
every display. `usable_geometry_for` (the Zoom target) subtracted it from the
window's output. With one display this was invisible; with two it was wrong —
a Dock pinned to one monitor reserved space on the other, and an output's
usable area depended on chrome it did not own. The legacy T-09/T-10 notes
called this out and named `LayerSurfaceState::matches_output` as the filter
hook for the fix (T-16.1a).

## Decision

- **Aggregate reserved zones per output.** `aggregate_reserved_for(output_name,
  layers, zones)` folds only the surfaces for which
  [`matches_output`](../../../compositor/src/shell/layer.rs) is true. Each edge
  keeps the deepest reserve, so two surfaces may reserve the same edge on one
  display without summing. The global union (`aggregate_reserved`) is kept only
  as the fallback for an output not yet resolved.
- **A pinned surface reserves only where it is pinned.** A surface created with
  a `wl_output` reserves only on that display; a surface created with `NULL`
  (the menu bar, the Dock) targets every display and reserves on each. This is
  the `wlr-layer-shell` all-output rule the protocol already documented in
  code — the `dragonfruit-shell.xml` description, which still said "the primary
  output", is corrected to match.
- **`ShellProtocolState` stores the per-output map** (`output_reserved:
  HashMap<String, ReservedZones>`); `refresh_reserved_zones` rebuilds it from
  the live outputs, and `reserved_zones_for(name)` is the one lookup. Both the
  `df_output.reserved_zone` replay on output announce and the live broadcast
  use it.
- **Chrome geometry is per output.** `chrome_surfaces(output)` already resolves
  each surface's anchor/margin geometry and reserved strip against that
  output's logical rectangle; the per-output reserve now feeds the same path.
  Reserved thicknesses are **logical** pixels, so a display's scale changes the
  geometry they are subtracted from but not the reserve itself.
- **`usable_geometry_for` uses the window's own output.** Zoom/inset reflow
  subtracts only that output's zones, which is the contract T-16.1b's window
  placement builds on.

## Consequences

- T-16.1b places new windows on the focused output and can rely on
  `usable_geometry_for` being per-output; a window on the secondary display is
  no longer zoomed around a Dock that lives on the primary.
- The headless gate is
  `per_output_chrome_reserves_are_scoped_and_scale_aware` in
  `compositor/tests/shell_protocol_conformance.rs`: two outputs at different
  scales, a primary-pinned bar (28) and an all-output bar (24), asserting the
  primary keeps 28 while the secondary receives only 24 (never 28).
- A chrome surface created without an output is still one `wl_surface`, so its
  client `configure` size is resolved against the primary output; the composited
  geometry, anchor, and reserve are per output, but a genuinely per-output
  client buffer needs one layer surface per output in the shell. That remains a
  follow-up and is not claimed here.
- Earlier ADRs that describe reserved zones (0089/0102 Dock, 0108 metrics) keep
  their geometry; only the ownership of the reserve changes from global to
  per-output.