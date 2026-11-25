# 0361 — Status popups clamp to the owning output in their host's coordinates

## Status

accepted

## Context

T350 fixes the four status popups that opened at x=0. Wi-Fi, Volume, Battery,
and Bluetooth are hosted by zero-sized `Item`s in the bar scene. The shared
`Popup` clamp used the popup parent's width, so a zero-width host resolved to
`0`. The Date & Time panel worked because its wrapper copied the bar's size.
Two of the four menus also assigned `popup.x` inside `onOpened`, which broke the
live anchor binding after a relayout.

The menu bar scene is output-local: its (0, 0) is the owning output's top-left.
The compositor applies the output's layout origin when it places the popup
layer surface. The committed dropdown target is still in window coordinates so
it includes the bar's reveal transform.

## Decision

- `MenuBar` publishes the owning output's logical bounds in each status host's
  parent coordinates (`popupBoundsIn(item)`) and passes them to the shared
  `Popup` as `availableRect`.
- Wi-Fi, Volume, Battery, Bluetooth, and the Date & Time panel all use the same
  `availableRect` contract. The manual `onOpened` x assignments are removed so
  the anchor stays a live binding.
- Pointer and keyboard activation both go through `activateStatusItem` ->
  `openStatusMenu`; the popup derives its anchor from the live status item, so
  a status-row relayout or an output resize re-runs the clamp.
- The committed dropdown target stays in window coordinates and keeps the bar
  reveal transform. QML popup geometry is output-local; the compositor owns the
  output's layout origin.
- The chrome fixture gate runs the status-anchor rows at 1x, 1.5x, and 2x, and
  covers secondary-output bounds, the right-edge clamp, relayout, resize, and
  pointer/keyboard parity.

## Consequences

- A zero-sized popup host is no longer a placement bug when it supplies valid
  `availableRect` bounds. Later popup work must pass bounds in the host's parent
  coordinates instead of relying on the host width or an `onOpened` assignment.
- T352 and T361 reuse this contract for the remaining menu-bar and popup
  families.
- A scale change does not alter logical popup geometry; it changes the
  physical buffer through the existing chrome-scale path.
- QML cannot simulate a compositor output-layout origin, so the deterministic
  contract is output-local placement. Native placement remains the hardware
  evidence row.
