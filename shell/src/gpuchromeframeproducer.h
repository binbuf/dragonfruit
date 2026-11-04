// SPDX-License-Identifier: MIT
// GPU chrome frame producer skeleton (T-21.1d, spike only).
//
// This is the T185 `ChromeFrameProducer` with a GPU render target instead of
// the software ping-pong QImage pair: one shared QOpenGLContext, one
// QOffscreenSurface, a hidden QQuickWindow driven by QQuickRenderControl, and
// a reusable QOpenGLFramebufferObject wrapped in a QQuickRenderTarget. The
// target's device-pixel-ratio keeps QML layout logical while rasterization is
// physical (T184/ADR 0192).
//
// T186a stops at the rendered texture. `renderFrame()` reads it back only so
// the smoke rail can verify pixels/DPR; T186b replaces that readback with the
// EGLImage/dmabuf export and `wl_buffer` commit. Nothing in the normal shell
// path instantiates this class yet, and `DRAGONFRUIT_CHROME_GPU` stays unset
// by default.
#pragma once

#include <QImage>
#include <QQuickRenderTarget>
#include <QSize>

#include "chromeframeproducer.h"

class QOffscreenSurface;
class QOpenGLContext;
class QOpenGLFramebufferObject;
class QQuickRenderControl;

class GpuChromeFrameProducer : public ChromeFrameProducer
{
    Q_OBJECT

public:
    explicit GpuChromeFrameProducer(QObject *parent = nullptr);
    ~GpuChromeFrameProducer() override;

    // Create the GL context, offscreen surface, render control, and hidden
    // window. On failure returns false, sets `fallbackReason()`, and leaves
    // the object inert; the caller keeps the T185 shm producer.
    bool initialize(QString *reason = nullptr);
    bool initialized() const { return m_initialized; }

    QQuickWindow *window() const override { return m_window; }
    QQuickRenderControl *renderControl() const { return m_renderControl; }
    bool renderControlActive() const override { return m_renderControl != nullptr; }
    bool softwareSceneGraph() const override { return false; }
    bool gpuActive() const override { return m_initialized; }
    QString fallbackReason() const override { return m_reason; }

    // Logical -> physical factor for the render target. The T185 contract
    // passes a physical buffer size to resize(); this producer divides it by
    // the DPR to keep the QML scene in logical units.
    void setDevicePixelRatio(qreal dpr);
    qreal devicePixelRatio() const { return m_dpr; }

    void resize(const QSize &bufferSize) override;
    // Renders once into the reusable FBO and returns a verification readback.
    // T186b will render without this readback and export the texture instead.
    QImage renderFrame() override;
    QImage readbackFrame() const { return m_lastFrame; }

    QSize framebufferSize() const;
    unsigned int framebufferTexture() const;

    // `DRAGONFRUIT_CHROME_GPU` is opt-in; unset or "0" keeps the T185 path.
    static bool environmentRequested();
    static QString environmentValue();

private:
    void ensureFramebuffer();
    void releaseGpu();
    void setFailure(const QString &reason);

    QOpenGLContext *m_context = nullptr;
    // QQuickRenderControl's RHI OpenGL backend creates its own context that
    // shares with m_context and makes it current during initialize(). Render
    // targets and FBOs must be created on that context, not on m_context.
    QOpenGLContext *m_renderContext = nullptr;
    QOffscreenSurface *m_surface = nullptr;
    QQuickRenderControl *m_renderControl = nullptr;
    QQuickWindow *m_window = nullptr;
    QOpenGLFramebufferObject *m_fbo = nullptr;
    QQuickRenderTarget m_target;
    qreal m_dpr = 1.0;
    bool m_initialized = false;
    QString m_reason;
    QImage m_lastFrame;
};
