// SPDX-License-Identifier: MIT
// GPU chrome frame producer (T-21.1d).
//
// This is the T185 `ChromeFrameProducer` with a GPU render target instead of
// the software ping-pong QImage pair: one shared QOpenGLContext, one
// QOffscreenSurface, a hidden QQuickWindow driven by QQuickRenderControl, and
// a reusable QOpenGLFramebufferObject wrapped in a QQuickRenderTarget. The
// target's device-pixel-ratio keeps QML layout logical while rasterization is
// physical (T184/ADR 0192).
//
// T186b adds the EGLImage/dmabuf export of the rendered FBO. `renderFrame()`
// remains the readback path used by the non-Dock surfaces (they still commit
// through T185 shm); `exportFrame()` renders once and exports the FBO without
// a CPU readback. A failed export is a recorded fallback, never a black
// surface.
#pragma once

#include <QImage>
#include <QQuickRenderTarget>
#include <QSize>
#include <QVector>

#include "chromedmabuf.h"
#include "chromedmabufexport.h"
#include "chromeframeproducer.h"
#include "gpuchromecapabilities.h"

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
    bool hasWindow() const { return m_window != nullptr; }

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

    // The compositor's linux-dmabuf feedback table. An exported frame is only
    // committed when its exact format/modifier pair appears here; T186b sets
    // this once from `ShellProtocol::gpuCapabilities()`.
    void setDmabufFormats(
        const QVector<GpuChromeCapabilities::FormatModifier> &formats);
    QVector<GpuChromeCapabilities::FormatModifier> dmabufFormats() const
    {
        return m_dmabufFormats;
    }

    void resize(const QSize &bufferSize) override;
    // Renders once into the reusable FBO and returns a verification/readback
    // image (the non-Dock surfaces still use the T185 shm commit path).
    QImage renderFrame() override;
    // Renders once and exports the current FBO as a dmabuf. No CPU readback.
    // The returned frame's fd stays owned by this producer and is valid until
    // the FBO is resized/recreated or the producer is destroyed.
    bool exportFrame(ChromeDmabufFrame *frame, QString *reason = nullptr);
    // Read the FBO last rendered by exportFrame()/renderFrame() without
    // rendering again. Used for the one-frame SHM fallback after a failed
    // export/import.
    QImage readbackCurrentFrame();
    QImage readbackFrame() const { return m_lastFrame; }
    QString exportReason() const { return m_exportReason; }

    QSize framebufferSize() const;
    unsigned int framebufferTexture() const;

    // `DRAGONFRUIT_CHROME_GPU` is opt-in; unset or "0" keeps the T185 path.
    static bool environmentRequested();
    static QString environmentValue();

private:
    bool renderToFramebuffer();
    void ensureFramebuffer();
    bool ensureDmabufExport();
    void releaseDmabuf();
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
    ChromeDmabufExporter m_exporter;
    ChromeDmabufFrame m_dmabufFrame;
    uint64_t m_dmabufSerial = 0;
    QVector<GpuChromeCapabilities::FormatModifier> m_dmabufFormats;
    qreal m_dpr = 1.0;
    bool m_initialized = false;
    bool m_exporterInitialized = false;
    QString m_reason;
    QString m_exportReason;
    QImage m_lastFrame;
};
