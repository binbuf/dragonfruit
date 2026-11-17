# 0285 — Menu-bar rendering follows the chrome frame pacer

Status: accepted (2026-11-17)

## Context

The menu bar already had a `ChromeFrameProducer` bound through
`bindChromePacing`, but ordinary requests still called `render()` directly on
the next event-loop turn. Pointer movement, clock ticks, popup geometry,
material policy, and configure/startup paths therefore bypassed the producer's
pending-frame check and coalescing. A frame could commit while the compositor's
`wl_surface.frame` callback for the previous commit was still outstanding,
producing duplicate commits and decoupling content from presentation.

Popup geometry had a second decoupling: a zero-delay timer applied the
rectangle and then requested a render, so a layout transaction could produce a
geometry pass followed by a separate content pass.

## Decision

- `scheduleRender()` marks the `menubar` producer dirty through
  `markChromeDirty()` and no longer owns a separate pending flag. The existing
  pacer decides whether to render now or wait for the compositor's frame
  callback.
- Start-up, configure, focus/menu resolution, clock, material policy, menu-bar
  policy, and scale changes use the same request path.
- Pointer movement and pointer leave deliver the input event only. Hover,
  title, and row changes raise `sceneChanged`, which the pacer coalesces. A
  move over unchanged chrome requests no frame.
- `schedulePopupGeometry()` marks the geometry pending. `render()` applies it
  once, immediately before building the frame targets, so the rectangle, input
  regions, and content share one paced frame.
- Closing a menu still drops input interception immediately. The close paths
  clear a stale deferred open geometry so the measured visual tail is not cut
  short by an older request.

## Consequences

- Rendered frames track display opportunities; repeated pointer samples
  coalesce into one request.
- The QML state at render time is what commits, so the latest input state
  reaches the next eligible frame without a per-sample redraw.
- Popup opening and retargeting no longer need a geometry frame followed by a
  content frame.
- The Dock pacer is unchanged; the menu bar now follows the same contract.
- Acceptance compares rendered frames with committed frames, because
  shared-memory commit deduplication happens after rendering.

## Verification

The menu-bar, Dock, chrome frame producer, and dmabuf commit component suites
pass with the change. A live high-rate pointer sweep is the measurement
workload: rendered and committed frames should stay at the display cadence
while an unchanged hover settles without further requests.
