// SPDX-License-Identifier: MIT
// Opt-in chrome transport counters (T-21.1a, extended by T-21.1c).
//
// `DRAGONFRUIT_CHROME_STATS=1` turns on the counters below and a greppable
// `chrome stats` line on SIGUSR1/exit. When the variable is unset the class is
// inert (every recorder returns immediately), so the normal path keeps its
// allocation and timing behaviour.
#pragma once

#include <QElapsedTimer>
#include <QHash>
#include <QSize>
#include <QString>
#include <QVector>

#include <cstdint>

class ChromeStats
{
public:
    static ChromeStats &instance();

    // True when `DRAGONFRUIT_CHROME_STATS` is set to a non-zero value.
    bool enabled() const { return m_enabled; }

    // The transport label in the summary line (`render-control+shm` or the
    // pre-T-21.1c fallback `offscreen-qpa+shm`).
    void setTransportPath(const QString &path);

    // T-21.1d: the GPU rail state in the summary line. `path` is `off`,
    // `dmabuf`, or `shm` (GPU requested but the Dock fell back); `reason` is
    // the first fallback token (`no-egl-export`, `no-format`, ...).
    void setGpuPath(const QString &path);
    void setGpuFallbackReason(const QString &reason);
    // T-21.1d: the per-surface transport actually used for the last frame
    // (`dmabuf` or `shm`) and the first reason the surface fell back. This is
    // the `path=`/`gpu_reason=` pair on each `chrome surface` line.
    void setSurfacePath(const QString &surface, const QString &path,
                        const QString &reason = QString());

    // The QML scene-graph produced a frame (one renderFrame()).
    void recordSceneFrame();
    // One producer rendered a frame for `surface` (the stable label).
    void recordRender(const QString &surface);
    // `grabWindow()` is about to re-render the scene for a commit. Stays zero
    // on the render-control path.
    void recordGrabRender();
    // The render finished; `commit_us` is measured from here.
    void recordRenderComplete();
    // A new pooled shm buffer was allocated (not a reuse).
    void recordShmAlloc(const QString &surface, qint64 bytes);
    // One committed chrome frame. `copyBytes` is the bytes copied into the
    // pooled buffer, `damagePixels` the changed region the compositor is told
    // about. Allocation is recorded separately, so a steady-state frame has
    // `recordShmAlloc` count 0 and a non-zero `copyBytes`.
    void recordCommit(const QString &surface, const QSize &logicalSize,
                      const QSize &bufferSize, int bufferScale, qreal scale,
                      bool viewport, qint64 commitUs, qint64 copyBytes,
                      qint64 damagePixels);
    // A frame whose pixels were identical to the previous commit: no buffer,
    // copy, or commit was needed.
    void recordSkippedCommit(const QString &surface, const QSize &logicalSize,
                             const QSize &bufferSize, int bufferScale, qreal scale,
                             bool viewport);
    // T-21.1d: one dmabuf-backed Dock frame was committed, deferred because
    // the ring had no free buffer, or fell back to shm.
    void recordDmabufCommit(const QString &surface, qint64 bytes);
    void recordDmabufDeferred(const QString &surface);
    void recordDmabufFallback(const QString &surface, const QString &reason);
    // T186c: the live dmabuf buffer ring. `dmabuf_buffers` is the current
    // count (flat after warm-up); `dmabuf_allocs`/`dmabuf_reuses` are
    // cumulative, so a steady-state frame allocates nothing.
    void recordDmabufBufferCreated(const QString &surface);
    void recordDmabufBufferDestroyed(const QString &surface);
    void recordDmabufReuse(const QString &surface);
    // The producer's pre-sample sync mode (`gl-fence` or `gl-finish`) and one
    // fence wait. `timeout` counts a wait that did not complete in time; such
    // a frame is not committed.
    void setDmabufSyncMode(const QString &mode);
    void recordDmabufSyncWait(qint64 waitUs, bool timeout);

    // A `wl_surface.frame` callback was delivered for `surface`.
    void recordFrameCallback(const QString &surface, qint64 callbackUs);
    // A render driven by the bounded safety tick because no frame callback
    // arrived (a degraded compositor).
    void recordFallbackRender();

    // Microseconds since the last render completion, or -1 when no readback
    // has completed yet.
    qint64 commitUsSinceRender() const;

    // Emit the current counters. `label` is `SIGUSR1-<n>` or `exit`. The
    // timing rings are cleared after the line is written, so each dump's
    // `commit_us_*` percentiles describe the interval since the previous dump;
    // the counters themselves stay cumulative.
    void emitStats(const QString &label);
    void emitSignalStats();

    static void installSignalHandler();
    // Read end of the self-pipe the SIGUSR1 handler writes to, or -1.
    static int signalFd();
    // Drain the self-pipe; true when at least one signal was pending.
    static bool takeSignal();

private:
    struct SurfaceStats {
        QString path = QStringLiteral("shm");
        QString reason;
        QSize logicalSize;
        QSize bufferSize;
        int bufferScale = 1;
        qreal scale = 1.0;
        bool viewport = false;
        quint64 commits = 0;
        quint64 renders = 0;
        quint64 shmAllocs = 0;
        quint64 shmBytes = 0;
        quint64 copyBytes = 0;
        quint64 damagePixels = 0;
        quint64 skippedCommits = 0;
        quint64 fallbackRenders = 0;
        quint64 dmabufCommits = 0;
        quint64 dmabufBytes = 0;
        quint64 dmabufDeferred = 0;
        quint64 dmabufFallbacks = 0;
        quint64 dmabufBuffers = 0;
        quint64 dmabufAllocs = 0;
        quint64 dmabufReuses = 0;
        QVector<qint64> commitUs;
        QVector<qint64> frameCallbackUs;
    };

    ChromeStats();

    bool m_enabled = false;
    QString m_transportPath = QStringLiteral("offscreen-qpa+shm");
    QString m_gpuPath = QStringLiteral("off");
    QString m_gpuReason;
    quint64 m_sceneFrames = 0;
    quint64 m_commits = 0;
    quint64 m_grabRenders = 0;
    quint64 m_shmAllocs = 0;
    quint64 m_shmBytes = 0;
    quint64 m_copyBytes = 0;
    quint64 m_damagePixels = 0;
    quint64 m_skippedCommits = 0;
    quint64 m_fallbackRenders = 0;
    quint64 m_dmabufCommits = 0;
    quint64 m_dmabufBytes = 0;
    quint64 m_dmabufDeferred = 0;
    quint64 m_dmabufFallbacks = 0;
    quint64 m_dmabufBuffers = 0;
    quint64 m_dmabufAllocs = 0;
    quint64 m_dmabufReuses = 0;
    quint64 m_dmabufSyncWaits = 0;
    quint64 m_dmabufSyncTimeouts = 0;
    QString m_dmabufSyncMode = QStringLiteral("none");
    quint64 m_frameCallbacks = 0;
    quint64 m_signalDumps = 0;
    QVector<qint64> m_commitUs;
    QVector<qint64> m_frameCallbackUs;
    QHash<QString, SurfaceStats> m_surfaces;
    QElapsedTimer m_renderTimer;
};

// Percentile helpers shared with the summary (nearest-rank, 0-based).
qint64 chromeStatsPercentile(const QVector<qint64> &values, int percent);
