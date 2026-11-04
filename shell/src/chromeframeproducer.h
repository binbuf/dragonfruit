// SPDX-License-Identifier: MIT
// One chrome frame producer per shell surface (T-21.1c).
//
// On the confirmed software scene graph this owns a QQuickRenderControl and a
// pair of offscreen QImages. Every renderFrame() sets a new paint-device
// render target (which forces the software renderer to repaint the whole
// scene), polishes/syncs/renders exactly once, and returns the target image.
// It never calls QQuickWindow::grabWindow(). The pair is ping-ponged so the
// image handed to the transport is not the one the next frame writes into;
// the transport can keep it as its previous-frame damage baseline without a
// deep copy or a detach.
//
// On a non-software backend (T186's territory) the producer falls back to a
// normal visible QQuickWindow and records a grab render, so the shell still
// works while the GPU transport is absent.
#pragma once

#include <QImage>
#include <QObject>
#include <QSize>
#include <QString>

class QQuickRenderControl;
class QQuickWindow;

class ChromeFrameProducer : public QObject
{
    Q_OBJECT

public:
    explicit ChromeFrameProducer(QObject *parent = nullptr);
    ~ChromeFrameProducer() override;

    virtual QQuickWindow *window() const { return m_window; }
    virtual bool renderControlActive() const { return m_renderControl != nullptr; }
    virtual bool softwareSceneGraph() const { return m_software; }
    // True when this producer renders through a GPU context. The T185 shm
    // producer is always false; GpuChromeFrameProducer overrides it.
    virtual bool gpuActive() const { return false; }
    // A short reason the GPU path fell back, or an empty string.
    virtual QString fallbackReason() const { return QString(); }

    // Resize the offscreen window. A render-control window is never shown; a
    // fallback window is shown once so its render loop can drive it.
    virtual void resize(const QSize &bufferSize);

    // Render one frame. The returned image is a shallow reference to the
    // producer's ping-pong target and stays valid until the same target is
    // reused two frames later. Null when no frame could be produced.
    virtual QImage renderFrame();

signals:
    // QML asked for a new frame (render control) or the fallback window
    // finished one. Coalesced by the controller; this is not a timer.
    void sceneChanged();

protected:
    // External lets a subclass own the window/context instead of the T185
    // software producer. It creates no window and probes no backend.
    enum class Construction { Default, External };
    explicit ChromeFrameProducer(Construction construction, QObject *parent = nullptr);

private:
    void ensureImages();
    void invalidateImages();

    QQuickRenderControl *m_renderControl = nullptr;
    QQuickWindow *m_window = nullptr;
    QImage m_images[2];
    int m_nextImage = 0;
    bool m_software = false;
    bool m_rendering = false;
};
