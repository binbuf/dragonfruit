# 0315 — One output global and output-scoped input

## Status

Accepted (2026-11-22). Corrects the DRM output-global ownership and the
first-output input assumptions in [`02-compositor.md`](../02-compositor.md).

## Context

The multi-monitor model is per-output in the workspace and shell layers, but
two compositor paths assume one output:

1. **Duplicate `wl_output` globals.** `backend::add_output` calls
   `Output::create_global` and discards the `GlobalId`; the DRM
   `connector_connected` path calls it again and stores the second id. Each
   DRM output therefore advertises two `wl_output` globals, and unplug removes
   only the second. Clients see the output twice and a stale output survives
   hotplug. The first global also leaks for the compositor’s lifetime.
2. **Phantom output on init failure.** `add_output` maps the output, creates
   its Space list, and registers the workspace before `planes()` and
   `initialize_output()` can fail. On failure there is no `SurfaceData`, so
   `connector_disconnected`/`device_removed` can never remove the output; the
   user gets an invisible monitor that windows can be placed on.
3. **First-output input.** Popup constraint uses `space.outputs().next()`;
   absolute pointer, locked absolute pointer, touch, and tablet mapping do the
   same. A menu on the second monitor flips against the first, and a touch on
   the second monitor moves the pointer on the first. The lock surface on a
   secondary output cannot be clicked.

These are daily-driver defects for the reference laptop + external monitor
setup, not visual polish.

## Decision

1. **Exactly one `wl_output` global per output.** `add_output` returns the
   `GlobalId` (or the caller creates it); the backend stores it in
   `SurfaceData` and removes that exact global on disconnect. The second
   `create_global` in `connector_connected` is deleted. Nested and headless
   create their one global at startup.
2. **Connector init is transactional.** On any failure after `add_output`
   that prevents the output from becoming usable (planes, DRM output init),
   call `on_output_removed`, `space.unmap_output`, remove the global, and
   clear the workspace before returning. No output survives without
   `SurfaceData`. The VRR probe is not such a failure: an unsupported,
   modeset-required, or refused VRR property keeps the fixed presentation
   period and the working output.
3. **Popups are constrained to the parent output.** Resolve the output from
   the parent window’s geometry (fall back to the focused output, then the
   primary) and pass that geometry to `constrained_popup_geometry`.
4. **Absolute input is output-scoped.** Map `PointerMotionAbsolute`, touch,
   and tablet events to the output under the device/pointer, using that
   output's geometry. The device's normalized position is mapped across the
   output-layout bounding box; the output containing that point is the
   authority, the output under the current pointer is the fallback when the
   layout has a gap, and the nearest output is the last resort. The locked
   absolute path uses the lock surface's output: a single live lock surface is
   always the target, and with several the event position (then the pointer)
   selects one.
5. **Mixed-DPI chrome is not part of this decision.** The shell’s single
   `m_outputScale` remains; per-output chrome is deferred in design 27 §7.
   This ADR fixes geometry and globals, not raster scale.

Rejected alternatives:

- **Keep two globals and dedupe in the shell.** Clients still see duplicates;
  the stale global still survives unplug.
- **Create the global in `add_output` only and never in DRM.** That is the
  chosen direction; the caller must store and remove it.
- **Use the focused output for popups.** The parent window’s output is the
  protocol-correct anchor; focus can move while the popup is open.
- **Map absolute input to the primary output.** It is wrong on every
  multi-monitor host.

## Consequences

- Hotplug is clean: one global in, one global out; no phantom outputs.
- Secondary-monitor menus, touch, tablets, and the lock surface work.
- `add_output`’s signature changes; nested/headless and the synthetic output
  tests update.
- The T347 hotplug row is the hardware proof; the headless tests prove the
  global count and the geometry mapping.
- The output-scoping geometry is a pure module
  (`compositor/src/output_input.rs`) shared by the router and the headless
  conformance tests, so a two-output run proves popup bounds, absolute motion,
  touch, and the locked path without hardware.
- A per-device `map-to-output` override is a follow-up. Until it exists, a
  device is scoped by the position it reports (the wlroots layout-mapping
  default), which is correct for the synthetic/libinput normalized model.
