# 0193 — Chrome frame producer: one render per frame, pooled shm, frame-callback pacing

## Status

accepted

## Context

T-21.1a measured the shell's fallback chrome transport. Each `afterRendering`
frame was followed by `QQuickWindow::grabWindow()`, which rendered the QML
scene a second time; the cropped image was copied into a fresh `memfd`/`shm`
buffer per commit, the whole buffer was damaged, and the shell never requested
`wl_surface.frame`. The Dock bar alone is 960,000 bytes per commit at 1x.

T-21.1b made the committed buffer physical-resolution while keeping the
surface contract logical. That raises the copy cost at 1.5x/2x, so the
fallback path needs to be cheap before T-21.1d adds GPU/dmabuf export.

## Decision

- **One `ChromeFrameProducer` per chrome scene.** It owns a
  `QQuickRenderControl` window and two `QImage` paint-device targets. Each
  requested frame sets the other target (`QQuickRenderTarget::fromPaintDevice`)
  and calls `polishItems()`, `sync()`, `render()` exactly once. The target
  change makes the software renderer repaint the whole scene; the ping-pong
  pair lets the transport keep the previous frame as its damage baseline
  without a deep copy or a detach. `QQuickWindow::grabWindow()` is not used on
  the render-control path. A non-software backend keeps a visible window and
  the old readback until T186 supplies the GPU path.
- **Per-surface three-buffer `wl_shm` pool.** A `ChromeBufferPool` tracks
  free/busy/retired slots. A `wl_buffer.release` returns the buffer to the
  pool; a size change retires the old generation and it is destroyed only when
  the compositor releases it. Allocation happens only on warm-up or resize, and
  the pool never exceeds three live buffers per surface. Each slot keeps one
  `memfd`/`mmap` for its lifetime, so a steady-state commit has no allocation.
- **One copy, changed-region damage.** `commitTo()` accepts a physical source
  rect and copies it once, row by row, into the pooled buffer. It compares the
  frame with the previous committed frame, damages only the changed pixel
  bounding box, and skips the commit entirely when the frame is identical.
  `ARGB32_Premultiplied` is not converted again.
- **Presentation pacing.** The shell requests `wl_surface.frame` before the
  commit and the controller renders the next QML state only from
  `framePresented`. A scene change with no callback in flight renders
  immediately (idle to animating); with a callback in flight it marks the
  producer dirty and waits. The compositor answers chrome callbacks in
  `post_repaint()`, the DRM/nested no-damage path, and headless. A 100 ms
  safety tick renders a dirty producer if no callback arrives and is counted
  as `fallback_renders`; it is not an animation source.
- **The private layer protocol and surface roles are unchanged.** The shell
  still authenticates and owns its layer surfaces through the same
  `df_shell`/`df_layer_surface` objects, so it remains independently
  restartable.

## Consequences

- The Dock/menu-bar paths make zero `grabWindow()` calls; `chrome stats`
  reports `grab_renders=0` and the before/after reports are
  `docs/captures/t21-chrome-baseline.txt` and
  `docs/captures/t21-chrome-transport-after.txt`.
- Steady-state animated frames keep `shm_allocs` flat and copy at most one
  full surface per committed frame. The damage bounding box is recorded as
  `damage_pixels`.
- A compositor that does not answer `wl_surface.frame` degrades to the counted
  safety tick instead of freezing the shell.
- T186 replaces the producer's software readback with a GPU target/dmabuf
  export while keeping the pool, pacing, and damage contract as the fallback.
