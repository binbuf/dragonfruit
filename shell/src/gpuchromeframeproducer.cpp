// SPDX-License-Identifier: MIT
#include "gpuchromeframeproducer.h"

#include <QDebug>
#include <QOffscreenSurface>
#include <QOpenGLContext>
#include <QOpenGLFramebufferObject>
#include <QOpenGLFunctions>
#include <QQuickItem>
#include <QQuickRenderControl>
#include <QQuickWindow>
#include <QSGRendererInterface>
#include <QSurfaceFormat>
#include <QtMath>

#include "chromestats.h"

namespace {

QSize physicalSize(const QSize &logical, qreal dpr)
{
    if (logical.isEmpty())
        return QSize();
    return QSize(qMax(1, qCeil(logical.width() * dpr)),
                 qMax(1, qCeil(logical.height() * dpr)));
}

} // namespace

GpuChromeFrameProducer::GpuChromeFrameProducer(QObject *parent)
    : ChromeFrameProducer(Construction::External, parent)
{
}

GpuChromeFrameProducer::~GpuChromeFrameProducer()
{
    releaseGpu();
}

bool GpuChromeFrameProducer::environmentRequested()
{
    const QByteArray value = qgetenv("DRAGONFRUIT_CHROME_GPU").trimmed();
    return !value.isEmpty() && value != "0";
}

QString GpuChromeFrameProducer::environmentValue()
{
    return QString::fromLocal8Bit(qgetenv("DRAGONFRUIT_CHROME_GPU"));
}

void GpuChromeFrameProducer::setFailure(const QString &reason)
{
    m_reason = reason;
    m_initialized = false;
    qWarning().noquote() << QStringLiteral(
        "chrome gpu producer: fallback reason=%1").arg(reason);
}

bool GpuChromeFrameProducer::initialize(QString *reason)
{
    if (m_initialized) {
        if (reason)
            reason->clear();
        return true;
    }
    m_reason.clear();

    // The scene-graph adaptation is process-global. The smoke rail creates no
    // QQuickWindow before this point, so the API selection takes effect. In a
    // process that already created a window this may be too late; initialize
    // then fails cleanly and the caller stays on the T185 shm path.
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);

    m_context = new QOpenGLContext(this);
    QSurfaceFormat format;
    format.setRenderableType(QSurfaceFormat::OpenGL);
    format.setVersion(3, 3);
    format.setProfile(QSurfaceFormat::CoreProfile);
    m_context->setFormat(format);
    if (!m_context->create()) {
        setFailure(QStringLiteral("no-gl-context"));
        if (reason)
            *reason = m_reason;
        return false;
    }

    m_surface = new QOffscreenSurface(nullptr, this);
    m_surface->setFormat(m_context->format());
    m_surface->create();
    if (!m_surface->isValid()) {
        setFailure(QStringLiteral("no-offscreen-surface"));
        if (reason)
            *reason = m_reason;
        return false;
    }
    if (!m_context->makeCurrent(m_surface)) {
        setFailure(QStringLiteral("no-current-context"));
        if (reason)
            *reason = m_reason;
        return false;
    }

    m_renderControl = new QQuickRenderControl(this);
    m_window = new QQuickWindow(m_renderControl);
    m_window->setColor(Qt::transparent);
    connect(m_renderControl, &QQuickRenderControl::sceneChanged, this,
            &ChromeFrameProducer::sceneChanged);
    connect(m_renderControl, &QQuickRenderControl::renderRequested, this,
            &ChromeFrameProducer::sceneChanged);

    if (!m_renderControl->initialize()) {
        m_context->doneCurrent();
        setFailure(QStringLiteral("render-control-init-failed"));
        if (reason)
            *reason = m_reason;
        return false;
    }
    // Qt's OpenGL RHI backend creates a share context and leaves it current.
    // That is the context every FBO/render target must live on.
    m_renderContext = QOpenGLContext::currentContext();
    if (!m_renderContext) {
        m_context->doneCurrent();
        setFailure(QStringLiteral("no-render-context"));
        if (reason)
            *reason = m_reason;
        return false;
    }

    const char *version = reinterpret_cast<const char *>(
        m_renderContext->functions()->glGetString(GL_VERSION));
    const char *renderer = reinterpret_cast<const char *>(
        m_renderContext->functions()->glGetString(GL_RENDERER));
    // Keep the context current until the first frame. QQuickRenderControl's
    // initialization and the first sync() must stay on the same current
    // context; releasing it here produces a valid but empty first frame.
    m_initialized = true;
    qInfo().noquote() << QStringLiteral(
        "chrome gpu producer: initialized api=OpenGL version=%1 renderer=%2 "
        "dpr=%3 gpu_env=%4")
        .arg(QString::fromLatin1(version ? version : "unknown"),
             QString::fromLatin1(renderer ? renderer : "unknown"))
        .arg(m_dpr, 0, 'g', 4)
        .arg(environmentValue().isEmpty() ? QStringLiteral("unset")
                                          : environmentValue());
    if (reason)
        reason->clear();
    return true;
}

void GpuChromeFrameProducer::setDevicePixelRatio(qreal dpr)
{
    if (dpr <= 0.0)
        return;
    if (qFuzzyCompare(m_dpr, dpr))
        return;
    m_dpr = dpr;
    // The target is rebuilt on the next frame; the window keeps its logical
    // size and only the physical rasterization changes.
    if (m_fbo) {
        QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
        if (context && m_surface)
            context->makeCurrent(m_surface);
        delete m_fbo;
        m_fbo = nullptr;
        m_target = QQuickRenderTarget();
        if (context)
            context->doneCurrent();
    }
}

void GpuChromeFrameProducer::resize(const QSize &bufferSize)
{
    if (!m_window || bufferSize.isEmpty())
        return;
    const qreal dpr = m_dpr > 0.0 ? m_dpr : 1.0;
    const QSize logical(qMax(1, qRound(bufferSize.width() / dpr)),
                        qMax(1, qRound(bufferSize.height() / dpr)));
    if (m_window->size() != logical)
        m_window->resize(logical);
    // A hidden render-control window does not expose, so its content item does
    // not always follow QWindow::resize. Size it explicitly; QML layout then
    // has a logical root rect even though the window is never shown.
    if (m_window->contentItem()
        && m_window->contentItem()->size() != QSizeF(logical)) {
        m_window->contentItem()->setSize(QSizeF(logical));
    }
}

void GpuChromeFrameProducer::ensureFramebuffer()
{
    if (!m_window || !m_context)
        return;
    const QSize physical = physicalSize(m_window->size(), m_dpr);
    if (physical.isEmpty())
        return;
    if (m_fbo && m_fbo->size() == physical)
        return;

    delete m_fbo;
    m_fbo = nullptr;
    QOpenGLFramebufferObjectFormat format;
    format.setAttachment(QOpenGLFramebufferObject::CombinedDepthStencil);
    format.setInternalTextureFormat(GL_RGBA8);
    format.setSamples(0);
    format.setMipmap(false);
    m_fbo = new QOpenGLFramebufferObject(physical, format);
    if (!m_fbo->isValid()) {
        delete m_fbo;
        m_fbo = nullptr;
        setFailure(QStringLiteral("fbo-create-failed"));
        return;
    }
    m_target = QQuickRenderTarget::fromOpenGLTexture(m_fbo->texture(), physical);
    m_target.setDevicePixelRatio(m_dpr);
}

QImage GpuChromeFrameProducer::renderFrame()
{
    if (!m_initialized || !m_window || !m_context || !m_surface || !m_renderContext)
        return QImage();
    if (QOpenGLContext::currentContext() != m_renderContext
        && !m_renderContext->makeCurrent(m_surface)) {
        setFailure(QStringLiteral("no-current-context"));
        return QImage();
    }

    ensureFramebuffer();
    if (!m_fbo) {
        m_renderContext->doneCurrent();
        return QImage();
    }

    m_window->setRenderTarget(m_target);
    m_renderControl->polishItems();
    m_renderControl->beginFrame();
    const bool synced = m_renderControl->sync();
    if (synced)
        m_renderControl->render();
    m_renderControl->endFrame();

    // The readback is verification-only in T186a. T186b replaces it with an
    // EGL fence + export, so no production frame copies pixels to the CPU.
    m_renderContext->functions()->glFinish();
    QImage image = m_fbo->toImage(true);
    if (!image.isNull()) {
        image = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        image.setDevicePixelRatio(m_dpr);
    }
    m_renderContext->doneCurrent();

    if (!image.isNull())
        m_lastFrame = image;
    ChromeStats::instance().recordSceneFrame();
    ChromeStats::instance().recordRenderComplete();
    return image;
}

QSize GpuChromeFrameProducer::framebufferSize() const
{
    return m_fbo ? m_fbo->size() : QSize();
}

unsigned int GpuChromeFrameProducer::framebufferTexture() const
{
    return m_fbo ? m_fbo->texture() : 0;
}

void GpuChromeFrameProducer::releaseGpu()
{
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    if (context && m_surface && context->isValid())
        context->makeCurrent(m_surface);

    if (m_renderControl)
        m_renderControl->invalidate();
    delete m_fbo;
    m_fbo = nullptr;
    m_target = QQuickRenderTarget();
    delete m_window;
    m_window = nullptr;
    if (context && context->isValid())
        context->doneCurrent();
    delete m_renderControl;
    m_renderControl = nullptr;
    m_renderContext = nullptr;

    delete m_surface;
    m_surface = nullptr;
    delete m_context;
    m_context = nullptr;
    m_initialized = false;
}
