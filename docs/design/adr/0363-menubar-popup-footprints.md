# 0363 — Menu Bar popups publish a settled crop, card material and picking bounds

- Status: accepted
- Date: 2026-11-25

## Context

The Menu Bar popup overlay was placed and cropped from the dropdown's
laid-out card box only. The QML `Shadow` ring therefore painted outside the
committed buffer and was clipped, while the popup surface still accepted
pointer input over the whole crop. The menu-bar exit also used a fixed 210 ms
timer, so a reduced-motion close or a short status-popup exit kept a stale
frosted surface mapped after the matching visual had ended.

Attached app menus and status popovers have different corner policies, and a
submenu is a separate card with a gap between it and its parent. A single
bounding rectangle cannot represent the card, the shadow paint reach, and the
clickable bodies at once.

## Decision

`MenuBar.qml` publishes one settled layout snapshot for the open popup:

- **Card coverage** (`dropdownX/Y/Width/Height`, `_dropdownRect`) is the
  laid-out dropdown (including an open submenu's union) used for placement,
  anchoring and regression checks.
- **Paint extent** (`dropdownPaintRect`) adds the full permitted QML `Shadow`
  reach (blur and elevation offset). **Committed crop**
  (`dropdownCropRect`/`dropdownCropX/Y/Width/Height`) clamps that extent to
  the owning output's visible bounds.
- **Material regions** (`dropdownSurfaceRegions`) stay the separate card
  bodies, translated to crop-local coordinates. The attached menu keeps
  square top corners; detached and status cards are all-rounded; the gap
  between a menu and its submenu receives no material.
- **Picking footprint** (`dropdownSurfaceInputRects`) is a few rounded spans
  per card body in crop-local coordinates. The shadow ring, the menu/submenu
  gap and the transparent corner tips stay click-through.

The controller commits the crop for placement and content in the independent,
shared and shared-memory paths, and sends the card material and explicit
picking rects on the same content commit. Logical close empties the input
region immediately and keeps the material through the visual tail only.

The exit interval comes from the QML close motion token. Reduced motion (or
any zero-duration exit) unmaps immediately; a normal menu close keeps its
183 ms tail, and a status popover its shorter popup token. The previous fixed
210 ms frost tail is retired.

## Consequences

- The shadow and rounded corners are inside the committed buffer at 1x,
  1.5x and 2x, while the card bodies remain the only interactive area.
- A changed model width settles into the crop, material and picking rects on
  the next rendered frame, and a warm reopen reuses the settled card.
- Reduced-motion and short exits release the frosted surface with their
  visual lifetime instead of a fixed extra interval.
- The visual transaction and retained whole-chain exit remain T354–T356.
  Large-model performance remains T360.
- The persistent Dock plate is not an input to this math; it keeps its own
  regression contract.
