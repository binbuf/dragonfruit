// SPDX-License-Identifier: MIT
#include "gpuchromeframeproducer.h"

#include <QDebug>
#include <QElapsedTimer>
#include <QOffscreenSurface>
#include <QOpenGLContext>
#include <QOpenGLExtraFunctions>
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
    // The ring is reconfigured on the next frame. Busy slots stay alive until
    // the compositor releases them; only then may their FBO be recreated.
    m_geometryDirty = true;
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

bool GpuChromeFrameProducer::configureRing(const QSize &physical)
{
    if (physical.isEmpty())
        return false;
    const bool sizeRetired = m_ring.setSize(physical);
    m_geometryDirty = false;
    if (sizeRetired)
        collectRetiredSlots();
    return true;
}

bool GpuChromeFrameProducer::ensureFramebuffer(int slot, const QSize &physical)
{
    if (!m_window || !m_context || physical.isEmpty())
        return false;
    SlotResources &resources = m_slots[slot];
    if (resources.fbo && resources.fbo->size() == physical)
        return true;

    if (resources.fbo) {
        ChromeDmabufExporter::closeFrame(&resources.frame);
        m_window->setRenderTarget(QQuickRenderTarget());
        delete resources.fbo;
        resources.fbo = nullptr;
        resources.rendered = false;
        if (m_lastRenderedSlot == slot)
            m_lastRenderedSlot = -1;
    }

    QOpenGLFramebufferObjectFormat format;
    format.setAttachment(QOpenGLFramebufferObject::CombinedDepthStencil);
    format.setInternalTextureFormat(GL_RGBA8);
    format.setSamples(0);
    format.setMipmap(false);
    resources.fbo = new QOpenGLFramebufferObject(physical, format);
    if (!resources.fbo->isValid()) {
        delete resources.fbo;
        resources.fbo = nullptr;
        setFailure(QStringLiteral("fbo-create-failed"));
        return false;
    }
    // A fresh render target starts transparent, not with driver garbage. Qt
    // clears the target on a normal frame; this covers the first one.
    resources.fbo->bind();
    if (QOpenGLFunctions *functions = m_renderContext->functions()) {
        functions->glClearColor(0.0f, 0.0f, 0.0f, 0.0f);
        functions->glClear(GL_COLOR_BUFFER_BIT);
    }
    resources.fbo->release();
    resources.rendered = false;
    return true;
}

bool GpuChromeFrameProducer::syncGpuWork()
{
    if (!m_renderContext)
        return false;
    QOpenGLExtraFunctions *functions = m_renderContext->extraFunctions();
    if (!functions) {
        m_renderContext->functions()->glFinish();
        ChromeStats::instance().setDmabufSyncMode(QStringLiteral("gl-finish"));
        return true;
    }

    GLsync sync = functions->glFenceSync(GL_SYNC_GPU_COMMANDS_COMPLETE, 0);
    if (!sync) {
        m_renderContext->functions()->glFinish();
        ChromeStats::instance().setDmabufSyncMode(QStringLiteral("gl-finish"));
        return true;
    }

    QElapsedTimer timer;
    timer.start();
    GLenum status = GL_TIMEOUT_EXPIRED;
    constexpr qint64 kFenceTimeoutUs = 100000;
    while (timer.nsecsElapsed() / 1000 < kFenceTimeoutUs) {
        status = functions->glClientWaitSync(
            sync, GL_SYNC_FLUSH_COMMANDS_BIT, 10000000);
        if (status != GL_TIMEOUT_EXPIRED)
            break;
    }
    functions->glDeleteSync(sync);

    const qint64 waitUs = timer.nsecsElapsed() / 1000;
    const bool ok = status == GL_ALREADY_SIGNALED || status == GL_CONDITION_SATISFIED;
    ChromeStats::instance().setDmabufSyncMode(QStringLiteral("gl-fence"));
    ChromeStats::instance().recordDmabufSyncWait(waitUs, !ok);
    if (!ok) {
        // Do not hand an unfinished buffer to the compositor. The controller
        // commits the last complete frame through shm instead.
        m_exportReason = QStringLiteral("sync-timeout");
        return false;
    }
    return true;
}

bool GpuChromeFrameProducer::renderToFramebuffer(int slot)
{
    if (!m_initialized || !m_window || !m_context || !m_surface || !m_renderContext)
        return false;
    if (QOpenGLContext::currentContext() != m_renderContext
        && !m_renderContext->makeCurrent(m_surface)) {
        setFailure(QStringLiteral("no-current-context"));
        return false;
    }

    const QSize physical = physicalSize(m_window->size(), m_dpr);
    if (!ensureFramebuffer(slot, physical)) {
        m_renderContext->doneCurrent();
        return false;
    }
    SlotResources &resources = m_slots[slot];

    QQuickRenderTarget target =
        QQuickRenderTarget::fromOpenGLTexture(resources.fbo->texture(), physical);
    target.setDevicePixelRatio(m_dpr);
    m_window->setRenderTarget(target);
    if (!resources.rendered)
        m_window->update();
    m_renderControl->polishItems();
    m_renderControl->beginFrame();
    const bool synced = m_renderControl->sync();
    if (synced || !resources.rendered)
        m_renderControl->render();
    m_renderControl->endFrame();

    if (!syncGpuWork()) {
        // The target may hold an unfinished frame. Force a full render into it
        // on the next attempt instead of treating it as a valid static frame.
        resources.rendered = false;
        m_renderContext->doneCurrent();
        return false;
    }
    resources.rendered = true;
    // Only a fence-complete frame becomes the readback fallback; a timed-out
    // render must not be handed to the compositor or to the shm fallback.
    m_lastRenderedSlot = slot;
    ChromeStats::instance().recordSceneFrame();
    ChromeStats::instance().recordRenderComplete();
    return true;
}

QImage GpuChromeFrameProducer::renderFrame()
{
    if (!m_initialized || !m_window) {
        m_exportReason = m_reason.isEmpty() ? QStringLiteral("render-failed") : m_reason;
        return QImage();
    }
    const QSize physical = physicalSize(m_window->size(), m_dpr);
    if (!configureRing(physical)) {
        m_exportReason = QStringLiteral("bad-size");
        return QImage();
    }
    const int slot = m_ring.acquire();
    if (slot < 0) {
        m_exportReason = QStringLiteral("ring-exhausted");
        return QImage();
    }
    if (!renderToFramebuffer(slot)) {
        m_ring.abandon(slot);
        collectRetiredSlots();
        return QImage();
    }

    QImage image = readbackSlot(slot);
    m_ring.abandon(slot);
    collectRetiredSlots();
    m_renderContext->doneCurrent();
    if (!image.isNull())
        m_lastFrame = image;
    return image;
}

QImage GpuChromeFrameProducer::readbackSlot(int slot)
{
    auto it = m_slots.find(slot);
    if (it == m_slots.end() || !it.value().fbo || !m_renderContext || !m_surface)
        return QImage();
    const bool wasCurrent = QOpenGLContext::currentContext() == m_renderContext;
    if (!wasCurrent && !m_renderContext->makeCurrent(m_surface))
        return QImage();

    QImage image = it.value().fbo->toImage(true);
    if (!image.isNull()) {
        image = image.convertToFormat(QImage::Format_ARGB32_Premultiplied);
        image.setDevicePixelRatio(m_dpr);
    }
    if (!wasCurrent)
        m_renderContext->doneCurrent();
    return image;
}

QImage GpuChromeFrameProducer::readbackCurrentFrame()
{
    return readbackSlot(m_lastRenderedSlot);
}

bool GpuChromeFrameProducer::ensureDmabufExport(int slot)
{
    auto it = m_slots.find(slot);
    if (it == m_slots.end() || !it.value().fbo || !m_renderContext)
        return false;
    SlotResources &resources = it.value();
    if (resources.frame.valid())
        return true;

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
        resources.fbo->texture(), resources.fbo->size(), &m_exportReason);
    if (!frame.valid())
        return false;

    // The compositor accepts only an exact pair from its feedback table. Check
    // it before creating a wl_buffer so an unsupported export is a clean
    // shm fallback rather than an import failure.
    const ChromeDmabufCheck check = ChromeDmabuf::validate(frame, m_dmabufFormats);
    if (!check.valid) {
        ChromeDmabufExporter::closeFrame(&frame);
        m_exportReason = check.reason;
        return false;
    }

    resources.frame = frame;
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
    if (!m_initialized || !m_window) {
        if (reason)
            *reason = m_reason.isEmpty() ? QStringLiteral("render-failed") : m_reason;
        return false;
    }

    const QSize physical = physicalSize(m_window->size(), m_dpr);
    if (!configureRing(physical)) {
        if (reason)
            *reason = QStringLiteral("bad-size");
        return false;
    }

    const int slot = m_ring.acquire();
    if (slot < 0) {
        if (reason)
            *reason = QStringLiteral("ring-exhausted");
        return false;
    }
    if (!renderToFramebuffer(slot)) {
        m_ring.abandon(slot);
        collectRetiredSlots();
        if (reason)
            *reason = m_exportReason.isEmpty()
                ? (m_reason.isEmpty() ? QStringLiteral("render-failed") : m_reason)
                : m_exportReason;
        return false;
    }
    if (!ensureDmabufExport(slot)) {
        m_ring.abandon(slot);
        collectRetiredSlots();
        if (reason)
            *reason = m_exportReason.isEmpty() ? QStringLiteral("no-egl-export")
                                               : m_exportReason;
        m_renderContext->doneCurrent();
        return false;
    }

    SlotResources &resources = m_slots[slot];
    resources.frame.slot = slot;
    resources.frame.serial = ++m_dmabufSerial;
    if (!m_ring.setSerial(slot, resources.frame.serial)) {
        m_ring.abandon(slot);
        if (reason)
            *reason = QStringLiteral("ring-serial");
        m_renderContext->doneCurrent();
        return false;
    }
    // The first export establishes the ring format. A later change retires
    // the old generation; the just-exported frame is still committed below.
    if (m_ring.setFormat(resources.frame.format, resources.frame.modifier))
        collectRetiredSlots();

    *frame = resources.frame;
    m_renderContext->doneCurrent();
    if (reason)
        reason->clear();
    return true;
}

void GpuChromeFrameProducer::releaseDmabufFrame(int slot, uint64_t serial)
{
    if (!m_ring.release(serial))
        return;
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    const bool madeCurrent = context && m_surface
        && QOpenGLContext::currentContext() != context
        && context->makeCurrent(m_surface);
    collectRetiredSlots();
    if (madeCurrent)
        context->doneCurrent();
    Q_UNUSED(slot);
}

void GpuChromeFrameProducer::abandonDmabufFrame(int slot)
{
    if (!m_ring.abandon(slot))
        return;
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    const bool madeCurrent = context && m_surface
        && QOpenGLContext::currentContext() != context
        && context->makeCurrent(m_surface);
    collectRetiredSlots();
    if (madeCurrent)
        context->doneCurrent();
}

QSize GpuChromeFrameProducer::framebufferSize() const
{
    auto it = m_slots.constFind(m_lastRenderedSlot);
    return it != m_slots.constEnd() && it.value().fbo ? it.value().fbo->size()
                                                      : QSize();
}

unsigned int GpuChromeFrameProducer::framebufferTexture() const
{
    auto it = m_slots.constFind(m_lastRenderedSlot);
    return it != m_slots.constEnd() && it.value().fbo ? it.value().fbo->texture()
                                                      : 0;
}

void GpuChromeFrameProducer::collectRetiredSlots()
{
    const QVector<int> retired = m_ring.takeRetired();
    for (int slot : retired)
        destroySlotResources(slot);
}

void GpuChromeFrameProducer::destroySlotResources(int slot)
{
    auto it = m_slots.find(slot);
    if (it == m_slots.end())
        return;
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    const bool madeCurrent = context && m_surface && context->isValid()
        && QOpenGLContext::currentContext() != context
        && context->makeCurrent(m_surface);
    SlotResources &resources = it.value();
    ChromeDmabufExporter::closeFrame(&resources.frame);
    if (resources.fbo) {
        if (m_window)
            m_window->setRenderTarget(QQuickRenderTarget());
        delete resources.fbo;
        resources.fbo = nullptr;
    }
    if (m_lastRenderedSlot == slot)
        m_lastRenderedSlot = -1;
    m_slots.erase(it);
    if (madeCurrent)
        context->doneCurrent();
}

void GpuChromeFrameProducer::releaseGpu()
{
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    if (context && m_surface && context->isValid())
        context->makeCurrent(m_surface);

    if (m_renderControl)
        m_renderControl->invalidate();
    const QList<int> slotIds = m_slots.keys();
    for (int slot : slotIds)
        destroySlotResources(slot);
    m_ring.reset();
    m_lastRenderedSlot = -1;
    m_exportReason.clear();
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
