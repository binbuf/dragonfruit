// SPDX-License-Identifier: MIT
#include "chromestats.h"

#include <QDateTime>
#include <QDebug>
#include <QtGlobal>

#include <algorithm>
#include <cerrno>
#include <csignal>

#include <fcntl.h>
#include <unistd.h>

namespace {

int g_signalPipe[2] = {-1, -1};

extern "C" void chromeStatsSignalHandler(int)
{
    const int savedErrno = errno;
    if (g_signalPipe[1] >= 0) {
        const char byte = 1;
        const ssize_t ignored = ::write(g_signalPipe[1], &byte, 1);
        (void)ignored;
    }
    errno = savedErrno;
}

QString sizeText(const QSize &size)
{
    return QStringLiteral("%1x%2").arg(size.width()).arg(size.height());
}

QString timingText(const char *prefix, const QVector<qint64> &values)
{
    if (values.isEmpty()) {
        return QStringLiteral("%1_samples=0 %1_p50=0 %1_p95=0 %1_p99=0 %1_max=0")
            .arg(QLatin1String(prefix));
    }
    qint64 maximum = 0;
    for (qint64 value : values)
        maximum = qMax(maximum, value);
    return QStringLiteral("%1_samples=%2 %1_p50=%3 %1_p95=%4 %1_p99=%5 %1_max=%6")
        .arg(QLatin1String(prefix))
        .arg(values.size())
        .arg(chromeStatsPercentile(values, 50))
        .arg(chromeStatsPercentile(values, 95))
        .arg(chromeStatsPercentile(values, 99))
        .arg(maximum);
}

} // namespace

qint64 chromeStatsPercentile(const QVector<qint64> &values, int percent)
{
    if (values.isEmpty())
        return 0;
    QVector<qint64> sorted = values;
    std::sort(sorted.begin(), sorted.end());
    const int last = sorted.size() - 1;
    const int rank = (percent * last) / 100;
    return sorted[qBound(0, rank, last)];
}

ChromeStats &ChromeStats::instance()
{
    static ChromeStats stats;
    return stats;
}

ChromeStats::ChromeStats()
{
    const QByteArray value = qgetenv("DRAGONFRUIT_CHROME_STATS");
    m_enabled = !value.isEmpty() && value != "0";
    const QByteArray trace = qgetenv("DRAGONFRUIT_CHROME_TRACE");
    m_traceEnabled = !trace.isEmpty() && trace != "0";
    m_traceClock.start();
}

void ChromeStats::recordSceneFrame()
{
    if (!m_enabled)
        return;
    ++m_sceneFrames;
}

void ChromeStats::recordRender(const QString &surface)
{
    if (!m_enabled || surface.isEmpty())
        return;
    ++m_surfaces[surface].renders;
}

void ChromeStats::recordGrabRender()
{
    if (!m_enabled)
        return;
    ++m_grabRenders;
}

void ChromeStats::recordRenderComplete()
{
    if (!m_enabled)
        return;
    m_renderTimer.start();
}

qint64 ChromeStats::commitUsSinceRender() const
{
    if (!m_enabled || !m_renderTimer.isValid())
        return -1;
    return m_renderTimer.nsecsElapsed() / 1000;
}

void ChromeStats::setTransportPath(const QString &path)
{
    if (!m_enabled || path.isEmpty())
        return;
    m_transportPath = path;
}

void ChromeStats::setGpuPath(const QString &path)
{
    if (!m_enabled || path.isEmpty())
        return;
    m_gpuPath = path;
}

void ChromeStats::setGpuFallbackReason(const QString &reason)
{
    if (!m_enabled)
        return;
    // Keep the first reason: the interesting one is why the GPU rail was
    // abandoned, not every later frame that stayed on the fallback.
    if (m_gpuReason.isEmpty() && !reason.isEmpty())
        m_gpuReason = reason;
}

void ChromeStats::setSurfacePath(const QString &surface, const QString &path,
                                 const QString &reason)
{
    if (!m_enabled || surface.isEmpty() || path.isEmpty())
        return;
    SurfaceStats &stats = m_surfaces[surface];
    stats.path = path;
    if (path == QLatin1String("dmabuf")) {
        // A successful GPU frame is the answer; do not keep an old transient
        // fallback as the surface's reason.
        stats.reason.clear();
    } else if (stats.reason.isEmpty() && !reason.isEmpty()) {
        stats.reason = reason;
    }
}

void ChromeStats::recordShmAlloc(const QString &surface, qint64 bytes)
{
    if (!m_enabled)
        return;
    ++m_shmAllocs;
    m_shmBytes += quint64(qMax<qint64>(0, bytes));
    SurfaceStats &stats = m_surfaces[surface];
    ++stats.shmAllocs;
    stats.shmBytes += quint64(qMax<qint64>(0, bytes));
}

void ChromeStats::recordCommit(const QString &surface, const QSize &logicalSize,
                               const QSize &bufferSize, int bufferScale, qreal scale,
                               bool viewport, qint64 commitUs, qint64 copyBytes,
                               qint64 damagePixels)
{
    if (!m_enabled)
        return;
    ++m_commits;
    const quint64 copied = quint64(qMax<qint64>(0, copyBytes));
    if (copied > 0)
        ++m_copies;
    m_copyBytes += copied;
    m_damagePixels += quint64(qMax<qint64>(0, damagePixels));
    if (commitUs >= 0)
        m_commitUs.append(commitUs);
    SurfaceStats &stats = m_surfaces[surface];
    stats.logicalSize = logicalSize;
    stats.bufferSize = bufferSize;
    stats.bufferScale = bufferScale;
    stats.scale = scale;
    stats.viewport = viewport;
    ++stats.commits;
    if (copied > 0)
        ++stats.copies;
    stats.copyBytes += copied;
    stats.damagePixels += quint64(qMax<qint64>(0, damagePixels));
    if (commitUs >= 0)
        stats.commitUs.append(commitUs);
}

void ChromeStats::recordSkippedCommit(const QString &surface, const QSize &logicalSize,
                                      const QSize &bufferSize, int bufferScale,
                                      qreal scale, bool viewport)
{
    if (!m_enabled)
        return;
    ++m_skippedCommits;
    SurfaceStats &stats = m_surfaces[surface];
    stats.logicalSize = logicalSize;
    stats.bufferSize = bufferSize;
    stats.bufferScale = bufferScale;
    stats.scale = scale;
    stats.viewport = viewport;
    ++stats.skippedCommits;
}

void ChromeStats::recordDmabufCommit(const QString &surface, qint64 bytes)
{
    if (!m_enabled)
        return;
    ++m_dmabufCommits;
    m_dmabufBytes += quint64(qMax<qint64>(0, bytes));
    SurfaceStats &stats = m_surfaces[surface];
    ++stats.dmabufCommits;
    stats.dmabufBytes += quint64(qMax<qint64>(0, bytes));
}

void ChromeStats::recordDmabufDeferred(const QString &surface)
{
    if (!m_enabled)
        return;
    ++m_dmabufDeferred;
    ++m_surfaces[surface].dmabufDeferred;
}

void ChromeStats::recordDmabufFallback(const QString &surface, const QString &reason)
{
    if (!m_enabled)
        return;
    ++m_dmabufFallbacks;
    ++m_surfaces[surface].dmabufFallbacks;
    setGpuFallbackReason(reason);
}

void ChromeStats::recordDmabufBufferCreated(const QString &surface)
{
    if (!m_enabled)
        return;
    ++m_dmabufBuffers;
    ++m_dmabufAllocs;
    SurfaceStats &stats = m_surfaces[surface];
    ++stats.dmabufBuffers;
    ++stats.dmabufAllocs;
}

void ChromeStats::recordDmabufBufferDestroyed(const QString &surface)
{
    if (!m_enabled)
        return;
    if (m_dmabufBuffers > 0)
        --m_dmabufBuffers;
    if (surface.isEmpty())
        return;
    SurfaceStats &stats = m_surfaces[surface];
    if (stats.dmabufBuffers > 0)
        --stats.dmabufBuffers;
}

void ChromeStats::recordDmabufReuse(const QString &surface)
{
    if (!m_enabled)
        return;
    ++m_dmabufReuses;
    ++m_surfaces[surface].dmabufReuses;
}

void ChromeStats::setDmabufSyncMode(const QString &mode)
{
    if (!m_enabled || mode.isEmpty())
        return;
    m_dmabufSyncMode = mode;
}

void ChromeStats::recordDmabufSyncWait(qint64 waitUs, bool timeout)
{
    if (!m_enabled)
        return;
    ++m_dmabufSyncWaits;
    if (timeout)
        ++m_dmabufSyncTimeouts;
    Q_UNUSED(waitUs);
}

void ChromeStats::recordSceneResize(const QString &surface, const QSize &from,
                                    const QSize &to)
{
    if (!m_enabled || surface.isEmpty())
        return;
    ++m_sceneResizes;
    ++m_surfaces[surface].sceneResizes;
    qInfo().noquote() << QStringLiteral(
        "chrome resize: surface=%1 from=%2x%3 to=%4x%5")
        .arg(surface)
        .arg(from.width())
        .arg(from.height())
        .arg(to.width())
        .arg(to.height());
}

void ChromeStats::recordFboAlloc(const QString &surface, const QSize &size,
                                 qint64 bytes, qint64 createUs)
{
    if (!m_enabled)
        return;
    const quint64 accounted = quint64(qMax<qint64>(0, bytes));
    ++m_fboAllocs;
    m_fboBytesLive += accounted;
    m_fboBytesPeak = qMax(m_fboBytesPeak, m_fboBytesLive);
    if (createUs >= 0)
        m_fboCreateUs.append(createUs);
    if (surface.isEmpty())
        return;
    SurfaceStats &stats = m_surfaces[surface];
    ++stats.fboAllocs;
    stats.fboBytesLive += accounted;
    stats.fboBytesPeak = qMax(stats.fboBytesPeak, stats.fboBytesLive);
    qInfo().noquote() << QStringLiteral(
        "chrome fbo: surface=%1 trace_id=%2 size=%3x%4 bytes=%5 create_us=%6 "
        "t_ms=%7")
        .arg(surface)
        .arg(activeTraceId())
        .arg(size.width())
        .arg(size.height())
        .arg(accounted)
        .arg(createUs)
        .arg(QDateTime::currentMSecsSinceEpoch());
}

void ChromeStats::recordFboFree(const QString &surface, qint64 bytes)
{
    if (!m_enabled)
        return;
    const quint64 accounted = quint64(qMax<qint64>(0, bytes));
    m_fboBytesLive -= qMin(m_fboBytesLive, accounted);
    if (surface.isEmpty())
        return;
    SurfaceStats &stats = m_surfaces[surface];
    stats.fboBytesLive -= qMin(stats.fboBytesLive, accounted);
}

void ChromeStats::recordDmabufImport(const QString &surface, qint64 roundtripUs,
                                     qint64 totalUs, bool ok)
{
    if (!m_enabled)
        return;
    ++m_dmabufImports;
    if (!ok)
        ++m_dmabufImportFailures;
    if (roundtripUs >= 0)
        m_dmabufRoundtripUs.append(roundtripUs);
    if (totalUs >= 0)
        m_dmabufImportUs.append(totalUs);
    if (surface.isEmpty())
        return;
    SurfaceStats &stats = m_surfaces[surface];
    ++stats.dmabufImports;
    if (!ok)
        ++stats.dmabufImportFailures;
    qInfo().noquote() << QStringLiteral(
        "chrome import: surface=%1 trace_id=%2 roundtrip_us=%3 total_us=%4 "
        "ok=%5 t_ms=%6")
        .arg(surface)
        .arg(activeTraceId())
        .arg(roundtripUs)
        .arg(totalUs)
        .arg(ok ? 1 : 0)
        .arg(QDateTime::currentMSecsSinceEpoch());
}

void ChromeStats::recordDmabufHold(const QString &surface, qint64 holdUs,
                                   qint64 sharedHoldUs, bool lastUser)
{
    if (!m_enabled)
        return;
    if (holdUs >= 0)
        m_dmabufHoldUs.append(holdUs);
    if (lastUser && sharedHoldUs >= 0)
        m_dmabufSharedHoldUs.append(sharedHoldUs);
    if (surface.isEmpty())
        return;
    SurfaceStats &stats = m_surfaces[surface];
    if (holdUs >= 0)
        stats.dmabufHoldUs.append(holdUs);
    if (lastUser && sharedHoldUs >= 0)
        stats.dmabufSharedHoldUs.append(sharedHoldUs);
}

void ChromeStats::recordRingExhausted(const QString &surface)
{
    if (!m_enabled)
        return;
    ++m_ringExhausted;
    if (!surface.isEmpty())
        ++m_surfaces[surface].ringExhausted;
}

void ChromeStats::recordCommitFlush(qint64 flushUs)
{
    if (!m_enabled || flushUs < 0)
        return;
    m_commitFlushUs.append(flushUs);
}

void ChromeStats::recordPopoverOpen(const QString &kind, qint64 openUs)
{
    if (!m_enabled)
        return;
    ++m_popoverOpens;
    if (openUs >= 0)
        m_popoverOpenUs.append(openUs);
    if (kind.isEmpty())
        return;
    SurfaceStats &stats = m_surfaces[kind];
    ++stats.popoverOpens;
    if (openUs >= 0)
        stats.popoverOpenUs.append(openUs);
}

quint64 ChromeStats::activeTraceId() const
{
    return m_traceStack.isEmpty() ? 0 : m_traceStack.constLast();
}

quint64 ChromeStats::beginTrace(const QString &kind, const QString &detail)
{
    if (!m_traceEnabled)
        return 0;
    const quint64 id = ++m_traceSeq;
    m_traceStartNs.insert(id, m_traceClock.nsecsElapsed());
    m_traceStack.append(id);
    QString line = QStringLiteral(
        "chrome trace id=%1 stage=begin kind=%2 t_ms=%3")
        .arg(id)
        .arg(kind)
        .arg(QDateTime::currentMSecsSinceEpoch());
    if (!detail.isEmpty())
        line += QStringLiteral(" detail=%1").arg(detail);
    qInfo().noquote() << line;
    return id;
}

void ChromeStats::traceEvent(quint64 id, const QString &stage,
                             const QString &detail)
{
    if (!m_traceEnabled || id == 0)
        return;
    const auto it = m_traceStartNs.constFind(id);
    const qint64 startNs = it == m_traceStartNs.constEnd() ? m_traceClock.nsecsElapsed()
                                                          : it.value();
    const qint64 elapsedUs = (m_traceClock.nsecsElapsed() - startNs) / 1000;
    QString line = QStringLiteral(
        "chrome trace id=%1 stage=%2 dt_us=%3 t_ms=%4")
        .arg(id)
        .arg(stage)
        .arg(elapsedUs)
        .arg(QDateTime::currentMSecsSinceEpoch());
    if (!detail.isEmpty())
        line += QStringLiteral(" detail=%1").arg(detail);
    qInfo().noquote() << line;
}

qint64 ChromeStats::endTrace(quint64 id, const QString &detail)
{
    if (!m_traceEnabled || id == 0)
        return -1;
    const auto it = m_traceStartNs.constFind(id);
    if (it == m_traceStartNs.constEnd())
        return -1;
    const qint64 elapsedUs = (m_traceClock.nsecsElapsed() - it.value()) / 1000;
    m_traceStartNs.erase(it);
    m_traceStack.removeAll(id);
    QString line = QStringLiteral(
        "chrome trace id=%1 stage=end dt_us=%2 t_ms=%3")
        .arg(id)
        .arg(elapsedUs)
        .arg(QDateTime::currentMSecsSinceEpoch());
    if (!detail.isEmpty())
        line += QStringLiteral(" detail=%1").arg(detail);
    qInfo().noquote() << line;
    return elapsedUs;
}

void ChromeStats::recordFrameCallback(const QString &surface, qint64 callbackUs)
{
    if (!m_enabled)
        return;
    ++m_frameCallbacks;
    if (callbackUs >= 0)
        m_frameCallbackUs.append(callbackUs);
    if (!surface.isEmpty() && callbackUs >= 0)
        m_surfaces[surface].frameCallbackUs.append(callbackUs);
}

void ChromeStats::recordFallbackRender()
{
    if (!m_enabled)
        return;
    ++m_fallbackRenders;
}

QStringList ChromeStats::summaryLines(const QString &label) const
{
    QStringList lines;
    if (!m_enabled)
        return lines;
    // One greppable global line; the per-surface lines follow it so the
    // logical/buffer sizes of each chrome surface stay readable.
    lines << QStringLiteral(
                 "chrome stats (%1): path=%2 scene_frames=%3 commits=%4 "
                 "grab_renders=%5 shm_allocs=%6 shm_bytes=%7 copy_bytes=%8 "
                 "damage_pixels=%9 frame_callbacks=%10 fallback_renders=%11 "
                 "skipped_commits=%12 dmabuf_commits=%13 dmabuf_bytes=%14 "
                 "dmabuf_deferred=%15 dmabuf_fallbacks=%16 gpu=%17 "
                 "gpu_reason=%18 %19 %20 dmabuf_buffers=%21 dmabuf_allocs=%22 "
                 "dmabuf_reuses=%23 dmabuf_sync=%24 dmabuf_sync_waits=%25 "
                 "dmabuf_sync_timeouts=%26 copies=%27")
                 .arg(label)
                 .arg(m_transportPath)
                 .arg(m_sceneFrames)
                 .arg(m_commits)
                 .arg(m_grabRenders)
                 .arg(m_shmAllocs)
                 .arg(m_shmBytes)
                 .arg(m_copyBytes)
                 .arg(m_damagePixels)
                 .arg(m_frameCallbacks)
                 .arg(m_fallbackRenders)
                 .arg(m_skippedCommits)
                 .arg(m_dmabufCommits)
                 .arg(m_dmabufBytes)
                 .arg(m_dmabufDeferred)
                 .arg(m_dmabufFallbacks)
                 .arg(m_gpuPath)
                 .arg(m_gpuReason.isEmpty() ? QStringLiteral("none") : m_gpuReason)
                 .arg(timingText("commit_us", m_commitUs))
                 .arg(timingText("frame_callback_us", m_frameCallbackUs))
                 .arg(m_dmabufBuffers)
                 .arg(m_dmabufAllocs)
                 .arg(m_dmabufReuses)
                 .arg(m_dmabufSyncMode)
                 .arg(m_dmabufSyncWaits)
                 .arg(m_dmabufSyncTimeouts)
                 .arg(m_copies);

    // T238: the A/B stage line. Kept separate from `chrome stats` so the
    // existing report parsers keep reading the same prefix and field set.
    lines << QStringLiteral(
                 "chrome stage stats (%1): scene_resizes=%2 fbo_allocs=%3 "
                 "fbo_bytes_live=%4 fbo_bytes_peak=%5 dmabuf_imports=%6 "
                 "dmabuf_import_failures=%7 ring_exhausted=%8 popover_opens=%9 "
                 "%10 %11 %12 %13 %14 %15 %16")
                 .arg(label)
                 .arg(m_sceneResizes)
                 .arg(m_fboAllocs)
                 .arg(m_fboBytesLive)
                 .arg(m_fboBytesPeak)
                 .arg(m_dmabufImports)
                 .arg(m_dmabufImportFailures)
                 .arg(m_ringExhausted)
                 .arg(m_popoverOpens)
                 .arg(timingText("fbo_create_us", m_fboCreateUs))
                 .arg(timingText("dmabuf_import_us", m_dmabufImportUs))
                 .arg(timingText("dmabuf_roundtrip_us", m_dmabufRoundtripUs))
                 .arg(timingText("dmabuf_hold_us", m_dmabufHoldUs))
                 .arg(timingText("dmabuf_shared_hold_us", m_dmabufSharedHoldUs))
                 .arg(timingText("commit_flush_us", m_commitFlushUs))
                 .arg(timingText("popover_open_us", m_popoverOpenUs));

    QStringList names = m_surfaces.keys();
    names.sort();
    for (const QString &name : names) {
        const SurfaceStats &stats = m_surfaces.value(name);
        lines << QStringLiteral(
                     "chrome surface (%1): surface=%2 path=%3 gpu_reason=%4 "
                     "logical_size=%5 buffer_size=%6 "
                     "buffer_scale=%7 scale=%8 viewport=%9 commits=%10 shm_allocs=%11 "
                     "shm_bytes=%12 copy_bytes=%13 damage_pixels=%14 skipped=%15 "
                     "fallback=%16 renders=%17 dmabuf_commits=%18 dmabuf_bytes=%19 "
                     "dmabuf_deferred=%20 dmabuf_fallbacks=%21 %22 %23 "
                     "dmabuf_buffers=%24 dmabuf_allocs=%25 dmabuf_reuses=%26 "
                     "copies=%27 scene_resizes=%28 fbo_allocs=%29 "
                     "fbo_bytes_live=%30 dmabuf_imports=%31 "
                     "ring_exhausted=%32 popover_opens=%33 %34 %35")
                     .arg(label)
                     .arg(name)
                     .arg(stats.path)
                     .arg(stats.reason.isEmpty() ? QStringLiteral("none") : stats.reason)
                     .arg(sizeText(stats.logicalSize))
                     .arg(sizeText(stats.bufferSize))
                     .arg(stats.bufferScale)
                     .arg(stats.scale, 0, 'g', 4)
                     .arg(stats.viewport ? 1 : 0)
                     .arg(stats.commits)
                     .arg(stats.shmAllocs)
                     .arg(stats.shmBytes)
                     .arg(stats.copyBytes)
                     .arg(stats.damagePixels)
                     .arg(stats.skippedCommits)
                     .arg(stats.fallbackRenders)
                     .arg(stats.renders)
                     .arg(stats.dmabufCommits)
                     .arg(stats.dmabufBytes)
                     .arg(stats.dmabufDeferred)
                     .arg(stats.dmabufFallbacks)
                     .arg(timingText("commit_us", stats.commitUs))
                     .arg(timingText("frame_callback_us", stats.frameCallbackUs))
                     .arg(stats.dmabufBuffers)
                     .arg(stats.dmabufAllocs)
                     .arg(stats.dmabufReuses)
                     .arg(stats.copies)
                     .arg(stats.sceneResizes)
                     .arg(stats.fboAllocs)
                     .arg(stats.fboBytesLive)
                     .arg(stats.dmabufImports)
                     .arg(stats.ringExhausted)
                     .arg(stats.popoverOpens)
                     .arg(timingText("dmabuf_hold_us", stats.dmabufHoldUs))
                     .arg(timingText("dmabuf_shared_hold_us",
                                     stats.dmabufSharedHoldUs));
    }
    return lines;
}

void ChromeStats::emitStats(const QString &label)
{
    if (!m_enabled)
        return;
    for (const QString &line : summaryLines(label))
        qInfo().noquote() << line;

    // The next dump describes the interval after this one. Counters stay
    // cumulative; only the timing rings are windowed.
    m_commitUs.clear();
    m_frameCallbackUs.clear();
    m_fboCreateUs.clear();
    m_dmabufImportUs.clear();
    m_dmabufRoundtripUs.clear();
    m_dmabufHoldUs.clear();
    m_dmabufSharedHoldUs.clear();
    m_commitFlushUs.clear();
    m_popoverOpenUs.clear();
    for (auto it = m_surfaces.begin(); it != m_surfaces.end(); ++it) {
        it.value().commitUs.clear();
        it.value().frameCallbackUs.clear();
        it.value().dmabufHoldUs.clear();
        it.value().dmabufSharedHoldUs.clear();
        it.value().popoverOpenUs.clear();
    }
}

void ChromeStats::emitSignalStats()
{
    if (!m_enabled)
        return;
    ++m_signalDumps;
    emitStats(QStringLiteral("SIGUSR1-%1").arg(m_signalDumps));
}

void ChromeStats::installSignalHandler()
{
    if (g_signalPipe[0] >= 0)
        return;
    if (::pipe(g_signalPipe) != 0) {
        g_signalPipe[0] = -1;
        g_signalPipe[1] = -1;
        return;
    }
    for (int fd : g_signalPipe) {
        ::fcntl(fd, F_SETFD, FD_CLOEXEC);
        ::fcntl(fd, F_SETFL, ::fcntl(fd, F_GETFL) | O_NONBLOCK);
    }
    std::signal(SIGUSR1, chromeStatsSignalHandler);
}

int ChromeStats::signalFd()
{
    return g_signalPipe[0];
}

bool ChromeStats::takeSignal()
{
    if (g_signalPipe[0] < 0)
        return false;
    bool pending = false;
    char buffer[64];
    for (;;) {
        const ssize_t read = ::read(g_signalPipe[0], buffer, sizeof(buffer));
        if (read > 0) {
            pending = true;
            continue;
        }
        break;
    }
    return pending;
}
