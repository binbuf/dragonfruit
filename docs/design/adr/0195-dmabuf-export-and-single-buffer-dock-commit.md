# 0195 — dmabuf export and the single-buffer Dock commit

## Status

accepted (T-21.1d, T186b)

## Context

T186a proved the capability half: the nested compositor advertises
`zwp_linux_dmabuf_v1` feedback, EGL can export a Qt-rendered texture, and
`GpuChromeFrameProducer` can render the QML scene through
`QQuickRenderControl` into a reusable FBO. T186b has to turn that into a real
Dock transport without changing the private layer protocol or making a failed
GPU path a black surface.

The shell normally forces `QT_QPA_PLATFORM=offscreen` and owns its chrome
surfaces on its own libwayland connection. The offscreen QPA has no GL context,
so the opt-in GPU rail needs a GL-capable Qt platform. The Qt Wayland platform
plugin cannot own the chrome surfaces: they are private `df_layer_surface`
roles, not `xdg_toplevel`. It is used only for the Qt-side GL context, and no
QQuickWindow is shown.

## Decision

- **Opt-in GL rail.** `DRAGONFRUIT_CHROME_GPU=1` plus a host
  `WAYLAND_DISPLAY` selects the Wayland QPA before `QGuiApplication`; unset or
  `0` keeps the offscreen QPA. The chrome surfaces stay on the shell's own
  libwayland connection. A missing GL context records `no-gl-context` and the
  whole shell stays on the T185 producer.
- **One GPU producer per chrome scene.** The scene-graph API is process-global,
  so when the GPU rail is chosen every chrome scene uses
  `GpuChromeFrameProducer`. In T186b only the Dock commits dmabuf; the other
  surfaces read the FBO back and keep the T185 pooled-shm commit. T186d moves
  them to the same transport.
- **Export.** `ChromeDmabufExporter` creates an `EGLImage` from the FBO texture
  with `EGL_GL_TEXTURE_2D_KHR`, queries it with
  `eglExportDMABUFImageQueryMESA`, and exports plane 0 with
  `eglExportDMABUFImageMESA`. The exported FourCC and modifier are checked
  against the compositor's feedback table; only an exact `(format, modifier)`
  pair is committed, and the exported stride/offset are what the buffer
  params carry. The export is y-inverted relative to the compositor
  convention, so `zwp_linux_buffer_params_v1.create` carries `y_invert`.
- **One reusable buffer, commit per frame.** The Dock surface owns one
  `wl_buffer` created from the exported dmabuf. The producer keeps the FBO and
  export alive and re-renders into it; `glFinish()` runs before each commit so
  the first frame (and the exported buffer) is complete when the compositor
  samples it. There is no ring and no release-driven reuse yet: T186c adds
  those, plus the explicit fence/sync path. A new FBO export gets a monotonic
  serial; a size/format/serial change recreates the `wl_buffer` instead of
  reusing storage that no longer exists.
- **Fallback is a first-class outcome.** Any export, format/modifier,
  viewporter, import, or buffer-creation failure abandons the dmabuf rail for
  the session, records the reason, and commits the already-rendered frame
  through the T185 shm path. `chrome stats` gains
  `gpu=off|shm|dmabuf`, `gpu_reason=<token>`, and
  `dmabuf_commits/bytes/deferred/fallbacks`; the compositor already reports
  `chrome_dmabuf_imports`.

## Consequences

- A nested `DRAGONFRUIT_CHROME_GPU=1` run reports `path=render-control+dmabuf`,
  `gpu=dmabuf`, `grab_renders=0`, and Dock `copy_bytes=0`; the compositor's
  `chrome_dmabuf_imports` advances. `DRAGONFRUIT_CHROME_GPU=0` stays on
  `render-control+shm` with zero dmabuf imports.
- The Wayland QPA opens a second Wayland connection for the GL context only.
  That is a deliberate deviation from the parent spec's "one display" wording;
  the private chrome protocol and surface roles are unchanged, and the shell
  remains independently restartable.
- T186c must add the 2–3 buffer ring, release handling, and explicit sync.
  Until then a single buffer is re-rendered while the compositor may still be
  sampling the previous frame; the nested rail is visually correct but the
  final sync contract is not yet in place.
- The compositor still imports and composites the dmabuf through the normal
  `WaylandSurfaceRenderElement` path; there is no compositor-side CPU upload
  fallback.
