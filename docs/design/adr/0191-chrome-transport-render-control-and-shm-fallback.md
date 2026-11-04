# 0191 — Chrome transport: QQuickRenderControl + GPU buffers, pooled shm fallback

## Status

accepted

## Context

T-21.1a measured the current shell chrome transport before T184–T186 change it.
The shell forces `QT_QPA_PLATFORM=offscreen`; `QSG_INFO=1` reports
`Loading backend software`, and the process maps only the EGL/GLX loader
libraries — no DRI driver. Every chrome frame is therefore rendered by the
software scene graph, read back with `QQuickWindow::grabWindow()`, cropped,
`memcpy`'d into a fresh `memfd`/`/dev/shm` buffer, and committed as a new
`wl_shm_buffer`. The compositor imports each fresh buffer through Smithay.

The committed baseline (`docs/captures/t21-chrome-baseline.txt`, nested,
private bus, 1920×1200) measured:

| Scenario | commits | shm bytes | commit_us p50 / p95 / p99 |
|---|---:|---:|---:|
| idle (5.3 s) | 0 | 0 | — (zero-wakeup) |
| slow Dock sweep (2.7 s) | 291 | 268,929,360 | 1069 / 1653 / 1943 µs |
| menu-bar dropdown open | 68 | 26,307,008 | 957 / 1190 / 1393 µs |
| Mission Control (13 windows) | 116 | 130,760,400 | 908 / 6766 / 7526 µs |

The Dock bar alone is 1920×125×4 = **960,000 bytes per commit**, and the
compositor reports the same `chrome_shm_imports`/`chrome_shm_bytes` for every
shell commit. `grab_renders` equals the commit count: the readback re-renders
the scene. `frame_callbacks` is 0 in both processes because the shell does not
request `wl_surface.frame`; nothing paces QML animation to presentation.
Compositor `frame timing` during the Dock sweep was p50 10,569 µs / p95 18,445 µs
/ p99 23,422 µs, while the 144 Hz panel's deadline is 6.94 ms. The nested run
has no hardware vblank, so it cannot report native `presentation stats`; the
native session's installed release compositor reports its own lifetime
presentation ring separately, and the scenario-driven native chrome half
remains OPEN for the hardware rail.

## Decision

- **Primary path: `QQuickRenderControl` plus reusable GPU buffers/dmabuf.**
  Render the QML scene into an offscreen target owned by the shell, keep the
  target and its textures across frames, and export/attach it to the private
  `df_layer_surface` as a dmabuf (or an equivalent Qt RHI native texture).
  Compositor frame callbacks pace the next render; buffer-release events gate
  reuse. This removes the per-frame `grabWindow()` readback and the 960 KB
  CPU copy/upload measured above.
- **Fallback: pooled `wl_shm` plus frame callbacks.** Where the GPU path is
  unavailable (software scene graph, headless, unsupported dmabuf/GPU, or a
  multi-GPU seat without a compatible import), keep the existing shm format
  but allocate a small pool, reuse released buffers, damage only the changed
  region, and pace on `wl_surface.frame`. This is the T185 slice.
- **The Qt Wayland platform plugin is not the transport.** It creates an
  unrelated `xdg_toplevel` for the shell window and has no way to attach the
  rendered scene to a private `df_layer_surface`; the shell's own libwayland
  connection is what owns the chrome surface and its reserved zone.
- **Do not build a custom QPA plugin or a second UI renderer.** The decision
  is about Qt Quick transport configuration, not a Skia/Impeller/Flutter
  replacement. A custom QPA plugin adds maintenance and still does not solve
  the private layer-surface attachment.

## Consequences

- T184 can keep the current render target and add the physical-resolution
  buffer scale; T185 can land the shm pool/frame-callback fallback and prove
  `grab_renders=0`; T186 can add the GPU/dmabuf path behind a runtime flag and
  report `path=dmabuf|shm` in `chrome stats`.
- The fallback remains shippable and testable without a GPU; the primary path
  must degrade to it explicitly, not silently.
- The baseline report's per-surface `logical_size`, `buffer_size`, and
  `buffer_scale=1` are the pre-T184 values; T184 changes the buffer size, not
  the surface layout or the private protocol.
- No default behavior changed in T-21.1a: the counters are opt-in and the
  normal `make demo`/`make e2e` paths are untouched.

**T186d update.** The GPU/dmabuf branch now covers every chrome surface behind
`DRAGONFRUIT_CHROME_GPU`, with per-surface `path=dmabuf|shm` stats and the
pooled shm path as the explicit fallback. The rollout, shared-scene release
rules, and measured implicit-sync limits are in
[ADR 0197](0197-gpu-chrome-rollout-and-per-surface-stats.md); the default
remains off until T192/T193.
