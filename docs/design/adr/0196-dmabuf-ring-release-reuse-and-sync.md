# 0196 — dmabuf ring, release-driven reuse, and pre-sample sync

## Status

accepted (T-21.1d, T186c)

## Context

T186b proved the Dock can export its rendered FBO as a dmabuf and commit it
through the private layer surface, but it kept one buffer and re-rendered into
it every frame. The compositor may still be sampling that buffer until
`wl_buffer.release`, so the single-buffer path is not safe under animation and
allocates no ring. T186c has to make the Dock transport production-safe without
changing the visual result or the private protocol.

## Decision

- **Bounded per-surface ring.** `ChromeDmabufRing` owns three slots per
  producer/surface. Each slot has its own FBO and cached EGL/dmabuf export; the
  shell keeps one reusable `wl_buffer` per `(surface, slot)`. A slot is
  acquired only while free, so the producer never renders into a buffer the
  compositor still holds. `ChromeDmabufFrame::slot` carries the identity; the
  monotonic `serial` identifies the committed generation of that slot.
- **Release returns the slot.** `ShellProtocol::onBufferRelease()` clears the
  slot's busy flag and emits `dmabufReleased(surface, slot, serial)`. The
  controller forwards it to `GpuChromeFrameProducer::releaseDmabufFrame()`,
  which makes the slot reusable. A stale serial is ignored. An export that is
  deferred or rejected is returned with `abandonDmabufFrame()`.
- **Resize/format recreation.** A size or export-format change retires the
  current generation. Free retired slots are destroyed immediately; busy
  retired slots stay alive until their release and are then destroyed. The
  ring never exceeds three live buffers; new-generation allocation waits for
  the old generation to drain instead of growing without bound.
- **Sync before sampling.** After rendering, the producer waits on a GL fence
  (`glFenceSync` + `glClientWaitSync`, 100 ms bound). If fences are unavailable
  it uses `glFinish`. A timed-out fence is a recorded `sync-timeout` and the
  frame is not committed. The compositor only offers implicit dma-buf sync
  (no `linux-drm-syncobj-v1` global), so the exported buffer carries the
  driver's implicit dma-fence; the GL fence guarantees the producer's work is
  complete before that buffer is handed over. This implicit-only limitation is
  recorded in `docs/rendering.md` and `docs/design/04-shell.md`.
- **Fallback is explicit.** Ring exhaustion reads back the last complete frame
  and commits it through the T185 shm pool for that frame, then keeps the
  dmabuf rail enabled and retries on release. Export/import/format failures
  abandon the rail for the session and use the same shm path. Every case is
  counted as `dmabuf_fallbacks` with a `gpu_reason`; the surface never goes
  black.
- **Stats.** `chrome stats` gains `dmabuf_buffers`, `dmabuf_allocs`,
  `dmabuf_reuses`, `dmabuf_sync`, `dmabuf_sync_waits`, and
  `dmabuf_sync_timeouts`. The first three are per surface as well as global, so
  a warmed-up run shows a flat live-buffer count and no new allocation.

## Consequences

- A nested GPU run reports `dmabuf_buffers=3` (or the number the Dock actually
  warmed) and `dmabuf_allocs` unchanged across two stats dumps; reuse and
  commits advance. The fallback run remains `gpu=off` with zero dmabuf imports.
- Fast animation can fill all three slots before the compositor releases them.
  That frame is a counted shm fallback (`gpu_reason=ring-exhausted`) rather than
  a black surface or a write into a busy buffer.
- T186d can move the remaining surfaces onto the same commit path; the ring is
  per producer/surface, not global. Default-on and native acceptance remain
  T191/T192.

T186d extends this decision to every chrome surface and adds the per-surface
`path=` stats; the shared-scene release bookkeeping and the capture-only
surface filter are recorded in
[ADR 0197](0197-gpu-chrome-rollout-and-per-surface-stats.md).
