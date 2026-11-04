// SPDX-License-Identifier: MIT
// Opt-in chrome transport counters (T-21.1a).
//
// The shell's chrome presentation path is deliberately measured, not guessed:
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

    // The QML scene-graph produced a frame (one `afterRendering`).
    void recordSceneFrame();
    // `grabWindow()` is about to re-render the scene for a commit.
    void recordGrabRender();
    // The readback finished; `commit_us` is measured from here.
    void recordRenderComplete();
    // One chrome buffer commit. `logicalSize` is the surface contract the
    // compositor configured; `bufferSize` is the physical buffer committed;
    // `bufferScale` is the wl_surface integer scale (1 when a wp_viewport
    // carries a fractional mapping). `shmBytes` is the fresh shared-memory
    // buffer this commit allocated and copied (0 for a non-shm path).
    void recordCommit(const QString &surface, const QSize &logicalSize,
                      const QSize &bufferSize, int bufferScale, qreal scale,
                      bool viewport, qint64 commitUs, qint64 shmBytes);
    // A `wl_surface.frame` callback was delivered (none before T-21.1c).
    void recordFrameCallback(qint64 callbackUs);

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
        QSize logicalSize;
        QSize bufferSize;
        int bufferScale = 1;
        qreal scale = 1.0;
        bool viewport = false;
        quint64 commits = 0;
        quint64 shmAllocs = 0;
        quint64 shmBytes = 0;
        QVector<qint64> commitUs;
        QVector<qint64> frameCallbackUs;
    };

    ChromeStats();

    bool m_enabled = false;
    quint64 m_sceneFrames = 0;
    quint64 m_commits = 0;
    quint64 m_grabRenders = 0;
    quint64 m_shmAllocs = 0;
    quint64 m_shmBytes = 0;
    quint64 m_frameCallbacks = 0;
    quint64 m_signalDumps = 0;
    QVector<qint64> m_commitUs;
    QVector<qint64> m_frameCallbackUs;
    QHash<QString, SurfaceStats> m_surfaces;
    QElapsedTimer m_renderTimer;
};

// Percentile helpers shared with the summary (nearest-rank, 0-based).
qint64 chromeStatsPercentile(const QVector<qint64> &values, int percent);
