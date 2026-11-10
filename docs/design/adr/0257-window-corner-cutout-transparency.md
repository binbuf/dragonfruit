# 0257 — The window shadow cuts at the window bounds, not the rounded body

- Status: accepted
- Date: 2026-11-10

## Context

T206 introduced the window shadow ring: the shadow spans are cut with the
window body so a first-party window with translucent app-local sidebar/toolbar
chrome cannot sample its own shadow. The cutout used the window's **rounded
body** (`shadow_elements_outside` with `component.window.radius`).

That keeps the shadow out of the translucent surface, but it also leaves the
shadow in the square corner pixels between the rounded window arc and the
window's bounding box. On a first-party window those pixels are transparent in
the client surface, so the shadow shows through as a faint translucent
grey/white square. With the rounded titlebar drawn over it, the result reads
as a 90° corner artifact behind the round corner — most visible over a bright
wallpaper or at fractional scale.

## Decision

- Cut the window shadow ring at the **decorated window bounds**, not the
  rounded body: `shadow::shadow_elements_outside_bounds` passes a square
  occluder (`radius = 0`) to the shared span subtraction. The renderer's
  per-window shadow pass uses it.
- The rounded silhouette is supplied by the surface the compositor or QML
  already draws: the SSD titlebar's `CornerMask::titlebar()` spans, and the
  first-party `AppWindow`/`TitleBar`/`ChromeSheet` rounding. The shadow does
  not need to trace that arc; it only needs to stay outside the window.
- The ring still omits the whole window body, so T206's translucent-chrome
  rule is preserved. Only the corner cutout changes: it is now completely
  clear instead of shadowed.

## Consequences

- The square corner pixels outside a rounded titlebar are transparent again;
  the wallpaper shows through exactly, with no translucent shadow square.
- The shadow remains visible outside the window bounds. Its inner edge is
  square at the bounds; that is intentional, because a rounded inner cutout
  is what produced the artifact. Square-corner CSD windows (Electron, most
  custom toolkits) are unchanged: their opaque surface covers the same pixels.
- `window::shadow::tests::shadow_outside_bounds_keeps_the_rounded_corner_cutout_clear`
  guards every corner point and keeps the old rounded-body variant as the
  regression control.
