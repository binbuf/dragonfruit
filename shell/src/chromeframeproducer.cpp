// SPDX-License-Identifier: MIT
#include "chromeframeproducer.h"

#include <QQuickRenderControl>
#include <QQuickRenderTarget>
#include <QQuickWindow>
#include <QSGRendererInterface>

#include "chromestats.h"

ChromeFrameProducer::ChromeFrameProducer(QObject *parent)
    : QObject(parent)
{
    // The scene-graph adaptation is chosen when the first QQuickWindow is
    // constructed. A throwaway probe window answers "software or not" without
    // forcing a backend: on a host with no GPU Qt falls back to software here
    // exactly as it would for the real windows.
    {
        QQuickWindow probe;
        m_software = probe.rendererInterface()->graphicsApi()
            == QSGRendererInterface::Software;
    }

    if (m_software) {
        m_renderControl = new QQuickRenderControl(this);
        m_window = new QQuickWindow(m_renderControl);
        connect(m_renderControl, &QQuickRenderControl::sceneChanged, this,
                &ChromeFrameProducer::sceneChanged);
        connect(m_renderControl, &QQuickRenderControl::renderRequested, this,
                &ChromeFrameProducer::sceneChanged);
    } else {
        m_window = new QQuickWindow;
        connect(m_window, &QQuickWindow::afterRendering, this,
                &ChromeFrameProducer::sceneChanged);
    }
    m_window->setColor(Qt::transparent);
}

ChromeFrameProducer::~ChromeFrameProducer()
{
    // The render control is a child and the window is manually owned; delete
    // the window first so the control's windowDestroyed() path is not reached
    // twice.
    delete m_window;
    m_window = nullptr;
}

void ChromeFrameProducer::invalidateImages()
{
    m_images[0] = QImage();
    m_images[1] = QImage();
    m_nextImage = 0;
}

void ChromeFrameProducer::ensureImages()
{
    if (!m_window)
        return;
    const qreal dpr = m_window->effectiveDevicePixelRatio();
    const QSize expected = m_window->size() * dpr;
    if (expected.isEmpty())
        return;
    if (m_images[0].size() == expected && m_images[1].size() == expected)
        return;
    for (QImage &image : m_images) {
        image = QImage(expected, QImage::Format_ARGB32_Premultiplied);
        image.setDevicePixelRatio(dpr);
        // The first target is not guaranteed to repaint every pixel (the
        // renderer may be created after the target-dirty check), so start
        // from transparent. Subsequent targets are forced full by the target
        // change in QQuickWindowPrivate::syncSceneGraph().
        image.fill(Qt::transparent);
    }
    m_nextImage = 0;
}

void ChromeFrameProducer::resize(const QSize &bufferSize)
{
    if (!m_window || bufferSize.isEmpty())
        return;
    if (m_window->width() != bufferSize.width()
        || m_window->height() != bufferSize.height()) {
        m_window->resize(bufferSize);
        invalidateImages();
    }
    if (!m_renderControl && !m_window->isVisible())
        m_window->show();
}

QImage ChromeFrameProducer::renderFrame()
{
    if (!m_window || m_rendering)
        return QImage();
    m_rendering = true;
    QImage result;
    if (m_renderControl) {
        ensureImages();
        QImage &target = m_images[m_nextImage];
        if (target.isNull()) {
            m_rendering = false;
            return QImage();
        }
        m_nextImage ^= 1;
        // A different paint device is a render-target change, which makes the
        // software renderer mark the whole scene dirty. Reusing the same
        // device would leave stale pixels in the ping-pong image.
        m_window->setRenderTarget(QQuickRenderTarget::fromPaintDevice(&target));
        m_renderControl->polishItems();
        m_renderControl->sync();
        m_renderControl->render();
        result = target;
    } else {
        // Non-software fallback: T186 replaces this with a GPU readback.
        ChromeStats::instance().recordGrabRender();
        result = m_window->grabWindow();
    }
    m_rendering = false;
    ChromeStats::instance().recordSceneFrame();
    ChromeStats::instance().recordRenderComplete();
    return result;
}
