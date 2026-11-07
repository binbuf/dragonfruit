# 0197 — GPU chrome rollout: every surface, per-surface stats, and the shm contract

## Status

accepted (T-21.1d, T186d)

## Context

T186c made the Dock's dmabuf path safe with a three-slot ring, release-driven
reuse, and a GL fence. The other chrome scenes still rendered through the GPU
producer but read the FBO back and committed through the T185 pooled shm path.
That left the rollout half-done: `chrome stats` could not say which surfaces
were actually on dmabuf, and a mixed scene could silently pay a CPU readback
even when its producer was GPU-backed. The parent spec also requires the
fallback to be explicit, never a black surface or a double render.

## Decision

- **One commit contract for every chrome surface.** `ShellProtocol` gains
  `commitDmabufFor(label, ...)` (the stable `chrome stats` label) and
  `commitLockDmabuf(id, ...)` (lock surfaces are keyed by the opaque configure
  id). Every render path in `ShellController` builds one
  `ChromeSceneTarget` list and calls `commitChromeScene()`; the Dock, menu bar,
  popovers, overview, switcher, banner, Control Center, OSD, chooser,
  screenshot, screencast, polkit, apps drawer, and lock all use the same
  producer/commit path.
- **A shared scene is all-or-nothing.** The menu bar + dropdown and Dock +
  popover are one render committed to two surfaces. If any target cannot use
  dmabuf (missing viewporter, hard import failure, or a capture-only surface
  filter), the whole scene renders once through T185 shm. This is the only way
  to avoid reading back an FBO that the compositor is already sampling.
- **No readback after a dmabuf commit.** A successful export/commit never
  calls `readbackCurrentFrame()`. The readback is reserved for the case where
  no target committed at all; a failed target is retried on the next frame or
  on `wl_buffer.release`.
- **Release bookkeeping is per producer.** A shared scene has one wl_buffer
  per surface. `ChromeDmabufReleaseTracker` holds the producer ring slot until
  the last surface release; a popover release must not let the next frame
  overwrite the still-visible bar buffer.
- **Stats are per surface.** Each `chrome surface` line carries `path=dmabuf`
  or `path=shm`, `gpu_reason`, `dmabuf_buffers`/`dmabuf_allocs`/
  `dmabuf_reuses`, and `copy_bytes`. A successful dmabuf frame has
  `copy_bytes=0`; a fallback frame is counted and names its reason
  (`ring-exhausted`, `sync-timeout`, `no-viewporter`, `import-failed`, ...).
- **The default is now GPU-first.** An unset `DRAGONFRUIT_CHROME_GPU` requests
  the rail; `0`/`off`/`false` forces the pooled-shm fallback. The capability
  probe/producer decides availability, including rejecting software GL
  renderers. (This supersedes the original T186d "default stays off"
  decision.) The capture-only `DRAGONFRUIT_CHROME_GPU_SURFACES` filter isolates one surface
  per nested run; it does not change the shipping default.
- **One fallback contract.** A hard export/format failure abandons the rail
  for the session; a transient ring/fence failure keeps it enabled and retries
  on the bounded safety tick or the next release. Nothing renders twice.

## Consequences

- A GPU-enabled nested run reports dmabuf for every mapped chrome surface and
  zero copy bytes on the dmabuf frames; a `DRAGONFRUIT_CHROME_GPU=0` run
  reports `gpu=off` and zero dmabuf imports.
- The compositor still offers only implicit dma-buf sync. A shared scene can
  fill its three-slot ring before the compositor releases the previous frame;
  that frame is a counted `ring-exhausted` shm fallback. On a shared host the
  same path can record `sync-timeout`. Explicit sync (`linux-drm-syncobj-v1`)
  remains the T192/T193 acceptance boundary, and the default-on switch stays
  with those tasks.
- The private layer protocol and surface roles are unchanged; the shell
  remains independently restartable.
