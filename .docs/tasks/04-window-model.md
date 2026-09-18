# T-04 — Window Model: States, Focus, Placement, Regions

| | |
|---|---|
| **Phase** | 1 · Foundation |
| **Area** | `compositor/` (window management) |
| **Depends on** | [T-02](02-compositor-core.md) · [T-03](03-input-keymaps-shortcuts.md) |
| **Blocks** | [T-05](05-spaces-model.md) · [T-06](06-xwayland.md) · [T-07](07-private-shell-protocols.md) · [T-13](13-window-decorations-ssd.md) |
| **Estimate** | L |
| **Design docs** | [02-compositor.md](../design/02-compositor.md) · [03-workspaces.md](../design/03-workspaces.md) |

## Summary

The compositor's window state machine: floating/minimized/zoomed/fullscreen
with remembered restore geometry, macOS-style distinct Zoom vs Fullscreen,
click-to-focus, centered cascade placement, interactive move/resize,
transient dialogs, popups, and honored client regions. Windows survive
shell restarts untouched.

## Background

Windows, focus, stacking, and workspace assignment are **compositor state
the shell never co-owns** ([02-compositor.md](../design/02-compositor.md)).
Getting the state machine right is what lets the Dock, app switcher, and
Mission Control be mere observers.

## Scope

### In scope

1. **Window states**:
   - A window is **floating, minimized, zoomed, or fullscreen**, with its
     restore geometry remembered across each transition.
   - **Zoom and fullscreen are distinct states** (macOS-style):
     - Zoom grows the window to fill the Space **minus the menu bar and
       Dock**.
     - Fullscreen creates **its own Space** (with T-05).
   - **There is no separate "maximize" state.** Requests for maximize map
     to Zoom.
2. **Focus**:
   - **Click-to-focus only**; focus never follows pointer motion alone.
   - Focus changes are broadcast over the private protocol so shell, Dock,
     and menu-broker track the active application without polling.
3. **Placement**:
   - New windows open near the **center of the active Space** with a
     per-output cascade offset.
   - Transient dialogs center on their parent, stay above it, and
     **minimize with it**.
4. **Regions**:
   - Client-provided **opaque, translucent, and input regions** are honored:
     input outside the input region falls through; translucent regions
     participate in the blur pass rather than fighting it.
5. **Shell-restart independence**: workspace assignment, stacking, and
     focus live in the compositor; a shell crash/restart (systemd restarts
     it) leaves windows untouched.
6. **Window menu primitives** (consumed via private protocol and the SSD
   titlebar right-click in T-13): Move to Space, Minimize, Zoom, Close.
7. **Interactive move and resize**: pointer-driven move and edge-resize for
   floating windows, serving both SSD titlebar/edge input (T-13) and CSD
   clients' `xdg_toplevel` move/resize requests — the machinery behind the
   "focus / move / resize" rung of the vertical slice
   ([13-roadmap.md](../design/13-roadmap.md)).
8. **Popups** (`xdg_popup`): positioner-constraint placement, popup input
   grabs, dismissal (click-away, Escape, parent unfocus), and stacking
   above their toplevel — the substrate for app menus, context menus, and
   the window menu alike
   ([02-compositor.md](../design/02-compositor.md) lists popups with
   transient dialogs).

### Out of scope

- Spaces logic itself (T-05) — this ticket keeps the per-window state and
  geometry; T-05 assigns windows to Spaces.
- Mission Control transforms (T-11).
- Decorations/titlebar drawing (T-13).

## Requirements

- FR-1: All four states reachable via protocol requests, SSD controls, and
  window menu; restore geometry correct after every state round-trip
  (floating→zoomed→floating restores original geometry, etc.).
- FR-2: Maximize requests produce Zoom behavior; no code path stores a
  distinct maximize state.
- FR-3: Click on unfocused window focuses it; pointer motion alone never
  changes focus (property test).
- FR-4: Focus broadcasts arrive on the private protocol for: window mapped,
  focused, unfocused, title/app_id changes, minimized, restored.
- FR-5: New-window cascade: consecutive windows offset from center; per
  output; wraps rather than walking off-screen.
- FR-6: Transient dialog: centered on parent, above parent in stacking,
  minimizes/restores with parent, closes with parent.
- FR-7: Input passthrough honored for clients with shaped input regions
  (test: click-through over a client with empty input region in that area).
- FR-8: Translucency: translucent regions composite with the blur pass, no
  artifacts (visual test with a blur+translucent test client).
- FR-9: Kill -9 the shell process; windows keep position, stacking, focus
   (scripted restart test).
- FR-10: Pointer move and edge-resize work for floating windows from both
  SSD input and CSD client requests; client min/max-size and aspect hints
  clamp resizes; a dragged window never leaves its output.
- FR-11: Popups stay within their output (positioner flip/slide/resize
  constraints), dismiss on click-away/Escape/parent unfocus, and never
  desync from or stack below their parent.

## Acceptance criteria

- [ ] State-machine property tests pass (all reachable transitions preserve
      restore geometry).
- [ ] Transient-dialog behavior matches the spec matrix (center/above/
      minimize-with/close-with).
- [ ] Shell-restart test passes (FR-9) — this is also a Phase-1 exit
      contributor.
- [ ] Move/resize and popup conformance suites pass (headless, driving
      `xdg_toplevel` move/resize and `xdg_popup` requests, malformed
      positioners included).

## Test plan

- Headless protocol tests driving `xdg_toplevel` state, move, and resize
  requests plus `xdg_popup` placement matrices.
- Malformed-client suite: clients sending contradictory state requests never
  crash the compositor (protocol-robustness rule from
  [14-risks.md](../design/14-risks.md)).
- Nested-session UI test with the design-system gallery (T-08) as the
  window source.

## Risks / open questions

- Real applications (Electron popups, SDL fullscreen toggles, Java windows)
  will bend these expectations; log deviations into the compatibility zoo
  tracked by [T-30](30-compatibility-bridges.md).
- "Zoom fills Space minus menu bar and Dock" requires reserved-zone data
  from the private protocol (T-07) — coordinate with that ticket's anchor
  zones.
