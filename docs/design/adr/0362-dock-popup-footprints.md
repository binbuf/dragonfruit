# 0362 — Dock popups publish separate paint, card and picking bounds

- Status: accepted
- Date: 2026-11-25

## Context

A Dock popover was treated as one rectangle. The shell cropped its overlay
surface to the popover's laid-out box, which stopped at the card and did not
include the QML shadow ring. The window chooser's 45-degree arrow also
extended about 1.66 logical pixels below that box, so the arrow tip and every
shadow pixel outside the card were clipped.

The same rectangle was the picking region: while open, the whole popup surface
accepted pointer input, including the empty band beside the arrow and the
transparent rounded-corner pixels. The material descriptor also used the full
box, so the compositor frosted the empty arrow-side band.

## Decision

A Dock popover publishes three separate bounds:

- **Card/arrow coverage** (`activePopoverRect`, `popoverCardRect`): the
  laid-out popover box used for placement and regression checks.
- **Paint extent** (`popoverRect`): the card box plus the rotated arrow tip and
  the full permitted QML `Shadow` reach (blur and elevation offset), before the
  existing output/headroom clamp. The shell commits this as the overlay crop.
- **Picking region** (`activePopoverInputRects`): the card bodies only, mapped
  into overlay-local coordinates. The Wayland input region is built from a few
  rounded spans per card so the shadow ring, empty arrow-side bands and corner
  cutouts stay click-through. The input region is emptied immediately at
  logical close, preserving outside dismissal, submenu traversal, drag and the
  existing immediate input release.

The material regions stop at the card body. The arrow keeps its QML
`Material.menuFill` treatment, matching the card, so the existing
rounded-rectangle material protocol does not need an arrow silhouette. Submenu
cards remain separate material and input regions.

Popup footprint math lives entirely in `Dock.qml` and the shell's popup
geometry path. The persistent Dock's `plateRect`, `panelRect`, `paintedBounds`
and input rects are not inputs to it; the plate is only the popover anchor.

## Consequences

- The chooser and every arrowed Dock popover keep their whole arrow and shadow
  at 1x, 1.5x and 2x. The patterned-underlay fixture shows clear card corners
  and arrow-side bands.
- Pointer input falls through the shadow and empty bands to the surface below;
  the card body remains fully interactive.
- The persistent plate's geometry and its pixels below the popup shadow are
  unchanged across an open/close interaction.
- The Menu Bar popup family (T352) can adopt the same three-bound split; it is
  not changed here.
