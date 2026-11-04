# 0194 — GPU chrome probe and render-control spike

## Status

accepted (T-21.1d, T186a)

## Context

T185 left the shell on the pooled-shm path: every chrome surface renders
through `QQuickRenderControl` into a software `QImage` pair, then copies into a
`wl_shm` buffer. T186 needs the GPU path before it can remove that copy. The
first question is not export but capability: does this seat/compositor expose
`zwp_linux_dmabuf_v1` feedback, can EGL export a Qt-rendered texture, and can
the shell actually create a GL render-control scene without changing the
default software rail?

The shell forces `QT_QPA_PLATFORM=offscreen`; that platform has no GL context
(GLX needs an X display, and the offscreen plugin does not provide EGL). The
nested smoke rail therefore runs with `QT_QPA_PLATFORM=wayland`, which is a
Qt-side connection used only for the GL context; the shell's private chrome
surfaces stay on the shell's own libwayland connection.

## Decision

- **Probe is read-only and runs once at shell startup.**
  `GpuChromeProbe` opens a temporary registry on the shell's existing
  `wl_display`, binds `zwp_linux_dmabuf_v1` (v4/v5 feedback when advertised),
  copies the format/modifier table out of the fd, queries EGL client/display
  extensions on the same Wayland display, and destroys the probe objects. It
  creates no `wl_buffer` and commits nothing. `ShellProtocol` keeps the result
  for T186b and logs one greppable `chrome gpu probe` line plus a bounded
  format list.
- **One pure capability/format helper.** `GpuChromeCapabilities` parses the
  16-byte feedback records and the EGL extension strings, and selects a
  compositor format deterministically: `ABGR8888` then `ARGB8888`; within a
  format linear, then the first explicit modifier, then
  `DRM_FORMAT_MOD_INVALID`. The actual exported modifier is still queried from
  EGL in T186b and must appear in the feedback table. `tst_gpucapabilities`
  covers malformed tables, both extension lists, and the selection order.
- **`GpuChromeFrameProducer` is the T185 producer with a GPU target.**
  It subclasses `ChromeFrameProducer`, creates one `QOpenGLContext` +
  `QOffscreenSurface` per producer, a `QQuickRenderControl`, a hidden
  `QQuickWindow`, and a reusable `QOpenGLFramebufferObject` wrapped in a
  `QQuickRenderTarget`. The target's device-pixel-ratio keeps QML layout
  logical while the FBO is physical; `resize()` keeps the T185 physical-buffer
  contract. The readback in `renderFrame()` is verification-only for the
  spike; T186b replaces it with EGLImage/dmabuf export.
- **Render targets live on Qt's RHI share context.** `QQuickRenderControl`'s
  OpenGL RHI backend creates its own context sharing with the context passed
  at `initialize()` and leaves it current. Creating the FBO or forcing the
  original context current produces a valid but empty frame. The producer
  stores the context current after `initialize()` and uses that for every FBO,
  target, readback, and teardown.
- **The flag stays off and failure is explicit.** `DRAGONFRUIT_CHROME_GPU`
  is unset by default and still only parsed by the new producer. A missing
  dmabuf global, feedback, EGL export extension, compatible format, or GL
  context records a reason (`no-dmabuf`, `no-feedback`, `no-egl-export`,
  `no-format`, `no-gl-context`, …) and leaves the T185 shm producer in charge.
  No path falls back to a black surface.
- **No production commit path in T186a.** The private layer protocol is
  unchanged and the normal shell never instantiates the GPU producer. The
  spike is exercised by `build/shell/tests/t21-gpu-smoke` against a nested
  compositor; the committed evidence is `docs/captures/t21-gpu-probe.txt` and
  `t21-gpu-smoke.png`.

## Consequences

- T186b can consume `ShellProtocol::gpuCapabilities()` and the producer's FBO
  texture to build the EGLImage/dmabuf export. The probe already reports the
  advertised format/modifier set and the chosen pair.
- The offscreen shell rail records `no-gl-context` when asked for GPU output;
  a production GPU rail needs a GL-capable Qt platform/context path before
  T186d can roll the flag out. T186a deliberately does not switch the default
  shell QPA.
- The nested GL capture is evidence for the render-control/FBO/DPR half only;
  it is not native acceptance and contains no dmabuf commit. The compositor
  reports `chrome_dmabuf_imports=0` in that run, as expected.
