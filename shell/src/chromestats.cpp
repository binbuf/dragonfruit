// SPDX-License-Identifier: MIT
#include "chromestats.h"

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
    m_copyBytes += quint64(qMax<qint64>(0, copyBytes));
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
    stats.copyBytes += quint64(qMax<qint64>(0, copyBytes));
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

void ChromeStats::emitStats(const QString &label)
{
    if (!m_enabled)
        return;
    // One greppable global line; the per-surface lines follow it so the
    // logical/buffer sizes of each chrome surface stay readable.
    qInfo().noquote()
        << QStringLiteral(
               "chrome stats (%1): path=%2 scene_frames=%3 commits=%4 "
               "grab_renders=%5 shm_allocs=%6 shm_bytes=%7 copy_bytes=%8 "
               "damage_pixels=%9 frame_callbacks=%10 fallback_renders=%11 "
               "skipped_commits=%12 dmabuf_commits=%13 dmabuf_bytes=%14 "
               "dmabuf_deferred=%15 dmabuf_fallbacks=%16 gpu=%17 "
               "gpu_reason=%18 %19 %20 dmabuf_buffers=%21 dmabuf_allocs=%22 "
               "dmabuf_reuses=%23 dmabuf_sync=%24 dmabuf_sync_waits=%25 "
               "dmabuf_sync_timeouts=%26")
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
               .arg(m_dmabufSyncTimeouts);

    QStringList names = m_surfaces.keys();
    names.sort();
    for (const QString &name : names) {
        const SurfaceStats &stats = m_surfaces.value(name);
        qInfo().noquote()
            << QStringLiteral(
                   "chrome surface (%1): surface=%2 path=%3 gpu_reason=%4 "
                   "logical_size=%5 buffer_size=%6 "
                   "buffer_scale=%7 scale=%8 viewport=%9 commits=%10 shm_allocs=%11 "
                   "shm_bytes=%12 copy_bytes=%13 damage_pixels=%14 skipped=%15 "
                   "fallback=%16 renders=%17 dmabuf_commits=%18 dmabuf_bytes=%19 "
                   "dmabuf_deferred=%20 dmabuf_fallbacks=%21 %22 %23 "
                   "dmabuf_buffers=%24 dmabuf_allocs=%25 dmabuf_reuses=%26")
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
                   .arg(stats.dmabufReuses);
    }

    // The next dump describes the interval after this one. Counters stay
    // cumulative; only the timing rings are windowed.
    m_commitUs.clear();
    m_frameCallbackUs.clear();
    for (auto it = m_surfaces.begin(); it != m_surfaces.end(); ++it) {
        it.value().commitUs.clear();
        it.value().frameCallbackUs.clear();
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
