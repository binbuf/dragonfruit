# 0288 — Conservative per-surface damage on the GPU chrome path

Status: accepted (2026-11-17)

## Context

Every GPU chrome commit damaged the whole exported framebuffer of every target
it was attached to. The shared-memory path already compares the frame against
the previous one, skips an identical commit, and copies only the accumulated
damage into the selected pool slot; the dmabuf path had no equivalent, so a
clock tick, a menu-row highlight, or a magnified entry repainted the full strip
or bar in the compositor even though the visible change was small.

The producer cannot compare pixels: the whole point of the GPU rail is to avoid
the readback that comparison needs. It also cannot ask Qt Quick what changed;
`QQuickRenderControl` reports that the scene graph is dirty, not which items
moved. The only component that can bound a change is the scene owner — the
controller and the QML it drives — and only for a scene whose painted content is
completely described by geometry it already publishes.

The Dock strip is such a scene. `Dock.qml` now publishes `paintedBounds`: the
plate group (fill, rim, glow, and the shadow the group clips to the anchored
edge room), every magnified or bouncing entry, and the rects the Dock already
declares visible for input — which widen to the whole surface while a drag or
an external drop paints its affordance labels. Nothing else is painted into the
strip scene. The menu bar is not such a scene: its bar background, app-menu
titles, and status items span the whole strip, so a partial bound would be the
full strip anyway.

## Decision

- **The frame carries a change report.** `ChromeDmabufFrame` gains a
  `ChromeFrameDamage`: `Unknown` keeps the old full-target contract and is the
  safe default; `Region` carries a bounding box in framebuffer pixels that
  covers every changed pixel, and an empty box means the target already shows
  the frame.
- **The controller reports in logical scene coordinates.** `ChromeSceneDamage`
  is what QML can express; `GpuChromeFrameProducer::submitFrame()` maps it
  through the device pixel ratio with the same outward rounding as the source
  crop (`ChromeDmabuf::frameDamage()`), so a fractional scale never leaves a
  changed pixel outside the box. The report is stored with the submitted slot
  and re-applied after the exporter rebuilds the frame struct.
- **Each target translates through its own crop.** A shared scene commits one
  export to the strip and the popup; `ChromeDmabuf::targetDamage()` intersects
  the frame report with that target's viewport source rect. A target the change
  does not touch has nothing to send.
- **The Dock strip unions the previous and current painted bounds.** A moving
  plate, a magnified entry, or a launch bounce damages where it was and where it
  is. The union also covers content changes inside the plate, because the plate
  is part of the bounds, and the whole surface while a drag affordance is
  visible, because the input rects widen there. In the shared-scene popover
  mode the overlay rect is part of the strip; with independent popup targets it
  is not.
- **A moved scene has no baseline.** A scene-size, scale, item-origin, or
  target-geometry change (logical size, viewport crop, buffer size) forces a
  full damage. So does any frame the compositor did not receive: a deferred,
  timed-out, or ring-exhausted frame invalidates the Dock baseline, and the next
  frame damages the whole strip. A transport change is therefore never a
  partial commit against content the compositor does not have.
- **Every ring slot tracks content validity.** A slot is only a partial-damage
  baseline after the producer rendered a complete frame of the current
  generation into it; allocation, resize, format retirement, and FBO recreation
  clear that flag, and a retired slot never regains it.
- **A target is committed when its pixels or its protocol geometry change.** An
  unchanged target sends nothing and returns the producer slot; an unchanged
  target whose input region or panel rect moved commits only that surface state,
  without attaching a buffer.
- **Everything else stays full.** The menu bar, both popup views, the overview,
  switcher, banners, OSD, chooser, and lock surfaces report `Unknown`. The
  initial implementation keeps full-scene rasterization and reduces only the
  downstream damage the compositor is asked to repair.

## Alternatives

- **Compare pixels before the commit.** Correct and simple, but it needs the
  readback the GPU rail exists to avoid, and it stalls the shell thread on the
  fence.
- **Derive the region from Qt's dirty-item list.** Not public API, and it
  describes items, not the pixels they paint (a text change can overflow its
  item).
- **Publish per-item damage from every menu and status component.** Would cover
  the clock and row-highlight cases, but requires each component to prove its
  own completeness; a missed change is silent visual corruption. Left for a
  later increment with a shared menu damage model.
- **Damage only the current painted bounds.** Would leave the pixels an element
  moved away from undamaged, so a magnified entry or a moving plate would smear.
  The union with the previous frame is the conservative form.
- **Damage the full strip on every frame.** The status quo: never wrong, never
  smaller. Retained as the fallback for every scene without complete bounds.

## Consequences

- A pointer sweep, magnification, launch bounce, or Dock hide/reveal damages
  the plate and entry extents instead of the whole strip. `chrome stats` now
  records the actual damaged pixels per dmabuf commit and counts skipped dmabuf
  commits, so the reduction is measurable against the shared-memory path.
- The Dock strip's correctness now depends on `Dock.qml::paintedBounds` being
  complete. Any new item painted into the strip scene must be added to it, or
  the shell must fall back to a full damage for that frame. `tst_dock` pins the
  plate group and every input rect inside the published bounds, including a
  magnified entry.
- The menu bar's clock tick still damages the full bar. It is the same contract
  as before this change; narrowing it needs the per-component damage model
  above.
- A skipped commit leaves the surface's current buffer attached, exactly like
  the shared-memory skip path: the compositor keeps showing the frame it
  already has.
- The producer slot is held only by targets that actually attached it, so a
  frame that only re-commits surface geometry does not wait for a release that
  will never arrive.

## Verification

- `tst_dmabufcommit` covers the logical-to-framebuffer mapping at 1× and 1.5×,
  clipping, the known-empty region, per-target crop translation, and ring
  content validity across reuse and resize.
- `tst_chromestats` covers the per-commit damage area and the skipped-commit
  counter; `tst_chromeframeproducer` covers the inert producer's damaged submit.
- `tst_dock` checks that the plate group and every input rect stay inside
  `paintedBounds`, and that a magnified entry grows the bounds.
- The Dock, Dock-core, menu-bar, frame-producer, buffer-pool, stats, scale, and
  dmabuf-commit suites pass with the change.
- Comparing damage area and compositor GPU time between the GPU and
  shared-memory paths on a host GPU remains the acceptance measurement; this
  change provides the counters for it.
