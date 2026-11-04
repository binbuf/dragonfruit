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
#include <QtGui/qopenglcontext_platform.h>

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

    // The scene-graph adaptation is process-global. The shell chooses this
    // rail at startup (before the first chrome window) when the GPU transport
    // is requested; a process that already created a software window fails
    // cleanly here and the caller stays on the T185 shm path.
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
        releaseDmabuf();
        delete m_fbo;
        m_fbo = nullptr;
        m_target = QQuickRenderTarget();
        if (context)
            context->doneCurrent();
    }
}

void GpuChromeFrameProducer::setDmabufFormats(
    const QVector<GpuChromeCapabilities::FormatModifier> &formats)
{
    m_dmabufFormats = formats;
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

    releaseDmabuf();
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

bool GpuChromeFrameProducer::renderToFramebuffer()
{
    if (!m_initialized || !m_window || !m_context || !m_surface || !m_renderContext)
        return false;
    if (QOpenGLContext::currentContext() != m_renderContext
        && !m_renderContext->makeCurrent(m_surface)) {
        setFailure(QStringLiteral("no-current-context"));
        return false;
    }

    ensureFramebuffer();
    if (!m_fbo) {
        m_renderContext->doneCurrent();
        return false;
    }

    m_window->setRenderTarget(m_target);
    m_renderControl->polishItems();
    m_renderControl->beginFrame();
    const bool synced = m_renderControl->sync();
    if (synced)
        m_renderControl->render();
    m_renderControl->endFrame();

    // T186b's first-frame contract: the GL work must be complete before the
    // compositor can sample the exported dmabuf. T186c replaces this with an
    // explicit fence/sync path.
    m_renderContext->functions()->glFinish();
    ChromeStats::instance().recordSceneFrame();
    ChromeStats::instance().recordRenderComplete();
    return true;
}

QImage GpuChromeFrameProducer::renderFrame()
{
    if (!renderToFramebuffer())
        return QImage();

    QImage image = readbackCurrentFrame();
    m_renderContext->doneCurrent();
    if (!image.isNull())
        m_lastFrame = image;
    return image;
}

QImage GpuChromeFrameProducer::readbackCurrentFrame()
{
    if (!m_fbo || !m_renderContext || !m_surface)
        return QImage();
    const bool wasCurrent = QOpenGLContext::currentContext() == m_renderContext;
    if (!wasCurrent && !m_renderContext->makeCurrent(m_surface))
        return QImage();

    QImage image = m_fbo->toImage(true);
    if (!image.isNull()) {
        image = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        image.setDevicePixelRatio(m_dpr);
    }
    if (!wasCurrent)
        m_renderContext->doneCurrent();
    return image;
}

bool GpuChromeFrameProducer::ensureDmabufExport()
{
    if (m_dmabufFrame.valid())
        return true;
    if (!m_fbo || !m_renderContext)
        return false;

    if (!m_exporterInitialized) {
        auto *native = m_renderContext->nativeInterface<QNativeInterface::QEGLContext>();
        if (!native) {
            m_exportReason = QStringLiteral("no-egl-context");
            return false;
        }
        if (!m_exporter.initialize(native->display(), native->nativeContext(),
                                   &m_exportReason)) {
            m_exporterInitialized = true;
            return false;
        }
        m_exporterInitialized = true;
    }
    if (!m_exporter.ready()) {
        if (m_exportReason.isEmpty())
            m_exportReason = QStringLiteral("no-egl-export");
        return false;
    }

    ChromeDmabufFrame frame = m_exporter.exportTexture(
        m_fbo->texture(), m_fbo->size(), &m_exportReason);
    if (!frame.valid())
        return false;
    frame.serial = ++m_dmabufSerial;

    // The compositor accepts only an exact pair from its feedback table. Check
    // it before creating a wl_buffer so an unsupported export is a clean
    // shm fallback rather than an import failure.
    const ChromeDmabufCheck check = ChromeDmabuf::validate(frame, m_dmabufFormats);
    if (!check.valid) {
        ChromeDmabufExporter::closeFrame(&frame);
        m_exportReason = check.reason;
        return false;
    }

    m_dmabufFrame = frame;
    m_exportReason.clear();
    return true;
}

bool GpuChromeFrameProducer::exportFrame(ChromeDmabufFrame *frame, QString *reason)
{
    if (!frame) {
        if (reason)
            *reason = QStringLiteral("bad-frame");
        return false;
    }
    if (!renderToFramebuffer()) {
        if (reason)
            *reason = m_reason.isEmpty() ? QStringLiteral("render-failed") : m_reason;
        return false;
    }
    if (!ensureDmabufExport()) {
        if (reason)
            *reason = m_exportReason.isEmpty() ? QStringLiteral("no-egl-export")
                                               : m_exportReason;
        m_renderContext->doneCurrent();
        return false;
    }
    *frame = m_dmabufFrame;
    m_renderContext->doneCurrent();
    if (reason)
        reason->clear();
    return true;
}

QSize GpuChromeFrameProducer::framebufferSize() const
{
    return m_fbo ? m_fbo->size() : QSize();
}

unsigned int GpuChromeFrameProducer::framebufferTexture() const
{
    return m_fbo ? m_fbo->texture() : 0;
}

void GpuChromeFrameProducer::releaseDmabuf()
{
    ChromeDmabufExporter::closeFrame(&m_dmabufFrame);
    m_exportReason.clear();
}

void GpuChromeFrameProducer::releaseGpu()
{
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    if (context && m_surface && context->isValid())
        context->makeCurrent(m_surface);

    if (m_renderControl)
        m_renderControl->invalidate();
    releaseDmabuf();
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
