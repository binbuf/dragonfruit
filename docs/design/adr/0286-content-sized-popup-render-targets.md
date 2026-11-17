# 0286 — Content-sized popup render targets

Status: accepted (2026-11-17)

## Context

The Dock rendered its strip and every popover into one shared scene, then
cropped the strip and the popover rectangle into two surfaces. The menu bar
did the same with a full-output-width bar scene that grew to the dropdown
height. A narrow menu therefore enlarged every attachment in the strip's
three-slot dma-buf ring, and a shared frame kept a slot busy until both
surfaces released it.

The earlier content-sized scene (`DockSceneBudget`) removed the always-resident
popover budget, and the T238 A/B showed that the stable budget spent 29–45 MiB
without improving open latency. Both arms still made the popup a sub-rect of
the strip scene, so a tall menu still resized the strip's render target and a
small tooltip still paid for the strip's attachment generation.

`ChromeDmabufRing` already supports multiple independent slot generations and
the GPU producer accounts for every attachment. The missing piece was a second
independently sized view in the same `QQuickWindow`, so the popup could be
rendered at its own size without a second QML scene or a second GL context.

## Decision

- **One window, two views.** `GpuChromeFrameProducer` owns a primary view (the
  strip or bar) and a popup view. Each view has its own `ChromeDmabufRing` and
  slot resources, so a popup allocation never occupies a strip slot and a
  strip commit never retires the popup target.
- **Content-sized popup targets.** `PopupTargetBudget` sizes the popup view to
  its content, clamped to the output. The target is grow-only for one
  interaction session so a submenu or hover label reuses the allocation, and
  it stays warm for 750 ms after the session ends before the controller
  releases it. The warm window covers the 183 ms close tail and a quick
  reopen without holding the largest menu for the rest of the session.
- **The surface still declares the real popup rect.** The render target may be
  larger than the current card; the commit crops the top-left content rect
  through the existing viewporter path. Popup state, anchor mapping, pointer
  transfer, input regions, material regions, and the close animation lifetime
  are unchanged.
- **The strip scene is the strip.** With independent targets the Dock scene is
  the resting strip and the menu-bar scene is the bar/veil height; neither
  grows when a menu opens. Input coordinates are unchanged because the
  controller restores the root item offset after the popup view renders.
- **Transport fallback is per frame.** The popup view uses the same dmabuf
  export/import/fallback rules as the primary view. A popup that cannot use
  dmabuf renders through the pooled shm path with its own readback image; a
  hard GPU failure returns the next frame to the shared content-sized scene.
- **The mode is measurable.** `DRAGONFRUIT_POPUP_TARGETS` selects
  `independent` (default when the GPU rail is active), `shared`, or `stable`.
  `DRAGONFRUIT_DOCK_STABLE_POPOVER=1` still selects `stable` for older runs.
  The `t238-dock-popover-ab` harness now runs all three arms.

## Alternatives

- **Keep the shared content-sized scene.** Lowest render work (one scene per
  frame) but the strip ring is resized by every popup size and a popup keeps a
  strip slot busy. Retained as `shared` for comparison and as the shm/fallback
  path.
- **Keep the full popover budget resident.** The T238 A/B rejected it: more
  memory, no measured open-latency gain. Retained as `stable` for comparison.
- **A second QML scene and producer.** Would isolate the popup completely but
  requires splitting `Dock.qml`/`MenuBar.qml`, duplicating interaction state,
  and a second GL context. The two-view approach gets the same attachment
  isolation without moving the QML tree.

## Consequences

- Opening a menu no longer resizes the strip/bar ring. Attachment memory is
  bounded by `strip area + popup area` per view instead of `scene area`, and
  each target is accounted under its own surface label (`dock-popup`,
  `menubar-popup`) in `chrome stage stats`.
- A frame that changes both the strip and the popup renders the scene twice
  (once per view). A frame that changes only the popup still renders the strip
  because the Dock/bar pacer commits the current scene; the strip render is
  small and the strip target does not resize.
- The popup target can be larger than the current card for the rest of a
  session. That is the bounded warm allocation the task calls for; the
  release timer returns it after the session and the retention window.
- The software producer keeps the shared scene path. On a host without the GPU
  rail the popup remains a cropped sub-rect of the strip scene, which is
  already content-sized on the shm copy path.

## Verification

- `tst_dockcore` covers the popup target clamp, grow-only session, warm
  retention, release, and output shrink.
- `tst_dock`, `tst_menubar`, `tst_design_system`, `tst_chromeframeproducer`,
  `tst_chromebufferpool`, `tst_chromestats`, `tst_chromescale`, and
  `tst_dmabufcommit` pass with the change.
- `scripts/t238-dock-popover-ab.sh` repeats 100 cold/warm open-close cycles for
  `shared`, `stable`, and `independent`. The report separates open latency,
  scene resizes, allocation counts, live/peak attachment bytes, imports, and
  release holds, so the independent arm can be selected on measured tail
  latency and bounded memory rather than assumption.
- Native 4K/144 Hz presentation timing remains a separate `OPEN` row in
  `t238-dock-popover-ab-native.sh`; this change does not claim native numbers.
