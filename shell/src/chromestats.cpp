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

void ChromeStats::recordCommit(const QString &surface, const QSize &bufferSize, int bufferScale,
                               qint64 commitUs, qint64 shmBytes)
{
    if (!m_enabled)
        return;
    ++m_commits;
    ++m_shmAllocs;
    m_shmBytes += quint64(qMax<qint64>(0, shmBytes));
    if (commitUs >= 0)
        m_commitUs.append(commitUs);
    SurfaceStats &stats = m_surfaces[surface];
    stats.logicalSize = QSize(bufferSize.width() / qMax(1, bufferScale),
                              bufferSize.height() / qMax(1, bufferScale));
    stats.bufferSize = bufferSize;
    stats.bufferScale = bufferScale;
    ++stats.commits;
    ++stats.shmAllocs;
    stats.shmBytes += quint64(qMax<qint64>(0, shmBytes));
    if (commitUs >= 0)
        stats.commitUs.append(commitUs);
}

void ChromeStats::recordFrameCallback(qint64 callbackUs)
{
    if (!m_enabled)
        return;
    ++m_frameCallbacks;
    if (callbackUs >= 0)
        m_frameCallbackUs.append(callbackUs);
}

void ChromeStats::emitStats(const QString &label)
{
    if (!m_enabled)
        return;
    // One greppable global line; the per-surface lines follow it so the
    // logical/buffer sizes of each chrome surface stay readable.
    qInfo().noquote()
        << QStringLiteral(
               "chrome stats (%1): path=offscreen-qpa+shm scene_frames=%2 commits=%3 "
               "grab_renders=%4 shm_allocs=%5 shm_bytes=%6 frame_callbacks=%7 "
               "%8 %9")
               .arg(label)
               .arg(m_sceneFrames)
               .arg(m_commits)
               .arg(m_grabRenders)
               .arg(m_shmAllocs)
               .arg(m_shmBytes)
               .arg(m_frameCallbacks)
               .arg(timingText("commit_us", m_commitUs))
               .arg(timingText("frame_callback_us", m_frameCallbackUs));

    QStringList names = m_surfaces.keys();
    names.sort();
    for (const QString &name : names) {
        const SurfaceStats &stats = m_surfaces.value(name);
        qInfo().noquote()
            << QStringLiteral(
                   "chrome surface (%1): surface=%2 logical_size=%3 buffer_size=%4 "
                   "buffer_scale=%5 commits=%6 shm_allocs=%7 shm_bytes=%8 %9 %10")
                   .arg(label)
                   .arg(name)
                   .arg(sizeText(stats.logicalSize))
                   .arg(sizeText(stats.bufferSize))
                   .arg(stats.bufferScale)
                   .arg(stats.commits)
                   .arg(stats.shmAllocs)
                   .arg(stats.shmBytes)
                   .arg(timingText("commit_us", stats.commitUs))
                   .arg(timingText("frame_callback_us", stats.frameCallbackUs));
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
