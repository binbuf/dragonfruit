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

// The producer accounts for the attachments it asks Qt to allocate: RGBA8
// colour (4 bytes/pixel) plus the combined 24-bit depth/8-bit stencil render
// buffer (4 bytes/pixel). This is an accounting estimate for the A/B memory
// comparison, not a driver allocation report.
qint64 fboAttachmentBytes(const QSize &size)
{
    return qint64(size.width()) * qint64(size.height()) * 8;
}

QString producerLabel(QQuickWindow *window)
{
    return window ? window->objectName() : QString();
}

QSize physicalSize(const QSize &logical, qreal dpr)
{
    if (logical.isEmpty())
        return QSize();
    return QSize(qMax(1, qCeil(logical.width() * dpr)),
                 qMax(1, qCeil(logical.height() * dpr)));
}

// Mesa's software rasterizers are valid GL contexts but there is no GPU
// acceleration behind them. Treat them as "no advanced GPU": the caller then
// keeps the T185 software scene graph instead of pretending the GPU rail is
// active.
bool softwareGlRenderer(const char *name)
{
    if (!name)
        return false;
    const QString renderer = QString::fromLatin1(name).toLower();
    return renderer.contains(QLatin1String("llvmpipe"))
        || renderer.contains(QLatin1String("softpipe"))
        || renderer.contains(QLatin1String("swrast"))
        || renderer.contains(QLatin1String("software rasterizer"));
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
    // GPU-first: an unset switch means "use the GPU when the host can". The
    // producer's initialization is the capability check; a failure is a
    // recorded shm fallback, not a reason to start on the CPU unconditionally.
    const QByteArray value =
        qgetenv("DRAGONFRUIT_CHROME_GPU").trimmed().toLower();
    if (value.isEmpty())
        return true;
    return value != "0" && value != "off" && value != "false";
}

QString GpuChromeFrameProducer::environmentValue()
{
    const QByteArray value = qgetenv("DRAGONFRUIT_CHROME_GPU").trimmed();
    return value.isEmpty() ? QStringLiteral("auto")
                           : QString::fromLocal8Bit(value);
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

    // Check the renderer before creating any QQuickWindow: the scene-graph
    // adaptation is process-global, so a software GL renderer must be
    // rejected while the controller can still choose the T185 software path.
    const char *probeRenderer = reinterpret_cast<const char *>(
        m_context->functions()->glGetString(GL_RENDERER));
    if (softwareGlRenderer(probeRenderer)) {
        m_context->doneCurrent();
        setFailure(QStringLiteral("software-renderer"));
        if (reason)
            *reason = m_reason;
        return false;
    }

    // The scene-graph adaptation is process-global. The shell chooses this
    // rail at startup (before the first chrome window) when the GPU transport
    // is requested; a process that already created a software window fails
    // cleanly here and the caller stays on the T185 shm path.
    QQuickWindow::setGraphicsApi(QSGRendererInterface::OpenGL);

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
        .arg(environmentValue());
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

GpuChromeFrameProducer::ViewState &
GpuChromeFrameProducer::viewState(int view)
{
    return m_views[view == PopupView ? PopupView : PrimaryView];
}

const GpuChromeFrameProducer::ViewState *
GpuChromeFrameProducer::viewState(int view) const
{
    auto it = m_views.constFind(view == PopupView ? PopupView : PrimaryView);
    return it == m_views.constEnd() ? nullptr : &it.value();
}

void GpuChromeFrameProducer::setViewLabel(int view, const QString &label)
{
    viewState(view).label = label;
}

QString GpuChromeFrameProducer::viewLabel(int view) const
{
    const ViewState *state = viewState(view);
    if (state && !state->label.isEmpty())
        return state->label;
    return producerLabel(m_window);
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

bool GpuChromeFrameProducer::configureRing(int view, const QSize &physical)
{
    if (physical.isEmpty())
        return false;
    ViewState &state = viewState(view);
    const bool sizeRetired = state.ring.setSize(physical);
    m_geometryDirty = false;
    if (sizeRetired)
        collectRetiredSlots(view);
    return true;
}

bool GpuChromeFrameProducer::ensureFramebuffer(int view, int slot,
                                               const QSize &physical)
{
    if (!m_window || !m_context || physical.isEmpty())
        return false;
    SlotResources &resources = viewState(view).slotResources[slot];
    if (resources.fbo && resources.fbo->size() == physical)
        return true;

    if (resources.fbo) {
        ChromeDmabufExporter::closeFrame(&resources.frame);
        m_window->setRenderTarget(QQuickRenderTarget());
        ChromeStats::instance().recordFboFree(
            viewLabel(view), fboAttachmentBytes(resources.fbo->size()));
        delete resources.fbo;
        resources.fbo = nullptr;
        resources.rendered = false;
        ViewState &state = viewState(view);
        if (state.lastRenderedSlot == slot)
            state.lastRenderedSlot = -1;
    }

    QElapsedTimer createTimer;
    createTimer.start();
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
    ChromeStats::instance().recordFboAlloc(
        viewLabel(view), physical, fboAttachmentBytes(physical),
        createTimer.nsecsElapsed() / 1000);
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

bool GpuChromeFrameProducer::renderToFramebuffer(int view, int slot)
{
    if (!m_initialized || !m_window || !m_context || !m_surface || !m_renderContext)
        return false;
    if (QOpenGLContext::currentContext() != m_renderContext
        && !m_renderContext->makeCurrent(m_surface)) {
        setFailure(QStringLiteral("no-current-context"));
        return false;
    }

    const QSize physical = physicalSize(m_window->size(), m_dpr);
    if (!ensureFramebuffer(view, slot, physical)) {
        m_renderContext->doneCurrent();
        return false;
    }
    SlotResources &resources = viewState(view).slotResources[slot];

    QQuickRenderTarget target =
        QQuickRenderTarget::fromOpenGLTexture(resources.fbo->texture(), physical);
    target.setDevicePixelRatio(m_dpr);
    m_window->setRenderTarget(target);
    if (!resources.rendered)
        m_window->update();
    m_renderControl->polishItems();
    m_renderControl->beginFrame();
    m_renderControl->sync();
    // The ring can hand back a slot containing an older frame even when the
    // scene graph has no changes since its last sync.
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
    viewState(view).lastRenderedSlot = slot;
    ChromeStats::instance().recordSceneFrame();
    ChromeStats::instance().recordRenderComplete();
    return true;
}

QImage GpuChromeFrameProducer::renderFrame()
{
    return renderFrame(PrimaryView);
}

QImage GpuChromeFrameProducer::renderFrame(int view)
{
    if (!m_initialized || !m_window) {
        m_exportReason = m_reason.isEmpty() ? QStringLiteral("render-failed") : m_reason;
        return QImage();
    }
    const QSize physical = physicalSize(m_window->size(), m_dpr);
    if (!configureRing(view, physical)) {
        m_exportReason = QStringLiteral("bad-size");
        return QImage();
    }
    ViewState &state = viewState(view);
    const int slot = state.ring.acquire();
    if (slot < 0) {
        m_exportReason = QStringLiteral("ring-exhausted");
        ChromeStats::instance().recordRingExhausted(viewLabel(view));
        return QImage();
    }
    if (!renderToFramebuffer(view, slot)) {
        state.ring.abandon(slot);
        collectRetiredSlots(view);
        return QImage();
    }

    QImage image = readbackSlot(view, slot);
    state.ring.abandon(slot);
    collectRetiredSlots(view);
    m_renderContext->doneCurrent();
    if (!image.isNull())
        m_lastFrame = image;
    return image;
}

QImage GpuChromeFrameProducer::readbackSlot(int view, int slot)
{
    const ViewState &state = viewState(view);
    if (!m_renderContext || !m_surface)
        return QImage();
    auto it = state.slotResources.find(slot);
    if (it == state.slotResources.end() || !it.value().fbo)
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
    return readbackCurrentFrame(PrimaryView);
}

QImage GpuChromeFrameProducer::readbackCurrentFrame(int view)
{
    const ViewState &state = viewState(view);
    return readbackSlot(view, state.lastRenderedSlot);
}

bool GpuChromeFrameProducer::ensureDmabufExport(int view, int slot)
{
    ViewState &state = viewState(view);
    if (!m_renderContext)
        return false;
    auto it = state.slotResources.find(slot);
    if (it == state.slotResources.end() || !it.value().fbo)
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
    return exportFrame(PrimaryView, frame, reason);
}

bool GpuChromeFrameProducer::exportFrame(int view, ChromeDmabufFrame *frame,
                                         QString *reason)
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
    if (!configureRing(view, physical)) {
        if (reason)
            *reason = QStringLiteral("bad-size");
        return false;
    }

    ViewState &state = viewState(view);
    const int slot = state.ring.acquire();
    if (slot < 0) {
        ChromeStats::instance().recordRingExhausted(viewLabel(view));
        if (reason)
            *reason = QStringLiteral("ring-exhausted");
        return false;
    }
    if (!renderToFramebuffer(view, slot)) {
        state.ring.abandon(slot);
        collectRetiredSlots(view);
        if (reason)
            *reason = m_exportReason.isEmpty()
                ? (m_reason.isEmpty() ? QStringLiteral("render-failed") : m_reason)
                : m_exportReason;
        return false;
    }
    if (!ensureDmabufExport(view, slot)) {
        state.ring.abandon(slot);
        collectRetiredSlots(view);
        if (reason)
            *reason = m_exportReason.isEmpty() ? QStringLiteral("no-egl-export")
                                               : m_exportReason;
        m_renderContext->doneCurrent();
        return false;
    }

    SlotResources &resources = state.slotResources[slot];
    resources.frame.slot = slot;
    resources.frame.serial = ++m_dmabufSerial;
    if (!state.ring.setSerial(slot, resources.frame.serial)) {
        state.ring.abandon(slot);
        if (reason)
            *reason = QStringLiteral("ring-serial");
        m_renderContext->doneCurrent();
        return false;
    }
    // The first export establishes the ring format. A later change retires
    // the old generation; the just-exported frame is still committed below.
    if (state.ring.setFormat(resources.frame.format, resources.frame.modifier))
        collectRetiredSlots(view);

    *frame = resources.frame;
    m_renderContext->doneCurrent();
    if (reason)
        reason->clear();
    return true;
}

void GpuChromeFrameProducer::releaseDmabufFrame(int slot, uint64_t serial)
{
    Q_UNUSED(slot);
    int releasedView = -1;
    for (auto it = m_views.begin(); it != m_views.end(); ++it) {
        if (it.value().ring.release(serial)) {
            releasedView = it.key();
            break;
        }
    }
    if (releasedView < 0)
        return;
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    const bool madeCurrent = context && m_surface
        && QOpenGLContext::currentContext() != context
        && context->makeCurrent(m_surface);
    collectRetiredSlots(releasedView);
    if (madeCurrent)
        context->doneCurrent();
}

void GpuChromeFrameProducer::abandonDmabufFrame(int slot)
{
    abandonDmabufFrame(PrimaryView, slot);
}

void GpuChromeFrameProducer::abandonDmabufFrame(int view, int slot)
{
    ViewState &state = viewState(view);
    if (!state.ring.abandon(slot))
        return;
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    const bool madeCurrent = context && m_surface
        && QOpenGLContext::currentContext() != context
        && context->makeCurrent(m_surface);
    collectRetiredSlots(view);
    if (madeCurrent)
        context->doneCurrent();
}

void GpuChromeFrameProducer::discardView(int view)
{
    ViewState &state = viewState(view);
    state.lastRenderedSlot = -1;
    state.ring.retireAll();
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    const bool madeCurrent = context && m_surface && context->isValid()
        && QOpenGLContext::currentContext() != context
        && context->makeCurrent(m_surface);
    collectRetiredSlots(view);
    if (madeCurrent)
        context->doneCurrent();
}

QSize GpuChromeFrameProducer::framebufferSize() const
{
    return framebufferSize(PrimaryView);
}

QSize GpuChromeFrameProducer::framebufferSize(int view) const
{
    const ViewState *state = viewState(view);
    if (!state)
        return QSize();
    auto it = state->slotResources.constFind(state->lastRenderedSlot);
    return it != state->slotResources.constEnd() && it.value().fbo ? it.value().fbo->size()
                                                           : QSize();
}

unsigned int GpuChromeFrameProducer::framebufferTexture() const
{
    return framebufferTexture(PrimaryView);
}

unsigned int GpuChromeFrameProducer::framebufferTexture(int view) const
{
    const ViewState *state = viewState(view);
    if (!state)
        return 0;
    auto it = state->slotResources.constFind(state->lastRenderedSlot);
    return it != state->slotResources.constEnd() && it.value().fbo ? it.value().fbo->texture()
                                                           : 0;
}

int GpuChromeFrameProducer::dmabufRingLiveCount() const
{
    return dmabufRingLiveCount(PrimaryView);
}

int GpuChromeFrameProducer::dmabufRingLiveCount(int view) const
{
    const ViewState *state = viewState(view);
    return state ? state->ring.liveCount() : 0;
}

quint64 GpuChromeFrameProducer::dmabufRingAllocations() const
{
    return dmabufRingAllocations(PrimaryView);
}

quint64 GpuChromeFrameProducer::dmabufRingAllocations(int view) const
{
    const ViewState *state = viewState(view);
    return state ? state->ring.allocations() : 0;
}

void GpuChromeFrameProducer::collectRetiredSlots(int view)
{
    ViewState &state = viewState(view);
    const QVector<int> retired = state.ring.takeRetired();
    for (int slot : retired)
        destroySlotResources(view, slot);
}

void GpuChromeFrameProducer::destroySlotResources(int view, int slot)
{
    ViewState &state = viewState(view);
    auto it = state.slotResources.find(slot);
    if (it == state.slotResources.end())
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
        ChromeStats::instance().recordFboFree(
            viewLabel(view), fboAttachmentBytes(resources.fbo->size()));
        delete resources.fbo;
        resources.fbo = nullptr;
    }
    if (state.lastRenderedSlot == slot)
        state.lastRenderedSlot = -1;
    state.slotResources.erase(it);
    if (madeCurrent)
        context->doneCurrent();
}

void GpuChromeFrameProducer::releaseGpu()
{
    QOpenGLContext *context = m_renderContext ? m_renderContext : m_context;
    if (context && m_surface && context->isValid())
        context->makeCurrent(m_surface);

    // Destroy the slot render targets before invalidating the render control.
    // `QQuickRenderControl::invalidate()` tears down the scene graph/render
    // target state; calling `QQuickWindow::setRenderTarget()` afterwards
    // dereferences freed state and segfaults during shell teardown (T192b's
    // full-rail Mission Control finding, reproduced by a failed start).
    for (auto viewIt = m_views.begin(); viewIt != m_views.end(); ++viewIt) {
        const QList<int> slotIds = viewIt.value().slotResources.keys();
        for (int slot : slotIds)
            destroySlotResources(viewIt.key(), slot);
        viewIt.value().ring.reset();
        viewIt.value().lastRenderedSlot = -1;
    }
    m_views.clear();
    if (m_renderControl)
        m_renderControl->invalidate();
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
