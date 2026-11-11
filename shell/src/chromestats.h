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

    // T238: stage timings for the Dock popover A/B. These are recorded even
    // when only `DRAGONFRUIT_CHROME_STATS` is set; the correlation trace is a
    // separate opt-in (`DRAGONFRUIT_CHROME_TRACE`).
    //
    // One scene-size change. The Dock popover currently grows the shared Dock
    // scene on open; the stable-popover flag removes this event.
    void recordSceneResize(const QString &surface, const QSize &from,
                           const QSize &to);
    // One render-target allocation. `bytes` is the attachment footprint the
    // producer accounts for (RGBA8 colour + combined depth/stencil), not a
    // driver-reported allocation. Live/peak make the extra resident cost of a
    // permanently reserved popover budget visible.
    void recordFboAlloc(const QString &surface, const QSize &size, qint64 bytes,
                        qint64 createUs);
    void recordFboFree(const QString &surface, qint64 bytes);
    // One first-use dmabuf import. `roundtripUs` is the blocking
    // `wl_display_roundtrip` wait inside `createDmabufSlot()`; `totalUs`
    // includes params setup, fd dup, and the round trip.
    void recordDmabufImport(const QString &surface, qint64 roundtripUs,
                            qint64 totalUs, bool ok);
    // One producer slot returned to the ring. `holdUs` is commit -> this
    // surface's release; `sharedHoldUs` (when `lastUser`) is first commit ->
    // last release for a scene shared by the Dock and its popover. This is the
    // shared-frame backpressure measurement.
    void recordDmabufHold(const QString &surface, qint64 holdUs,
                          qint64 sharedHoldUs, bool lastUser);
    // The producer had no free ring slot when a frame was requested.
    void recordRingExhausted(const QString &surface);
    // Time spent in `wl_display_flush` after a commit. A blocked socket is a
    // compositor-submission stall that the fence wait cannot explain.
    void recordCommitFlush(qint64 flushUs);
    // Popover-open request (the shell's `popoverChanged` signal) to the first
    // frame callback for that commit. This is request-to-visible, not hardware
    // input-to-visible; the trace carries the raw timestamps for that.
    void recordPopoverOpen(const QString &kind, qint64 openUs);

    // A `wl_surface.frame` callback was delivered for `surface`.
    void recordFrameCallback(const QString &surface, qint64 callbackUs);
    // A render driven by the bounded safety tick because no frame callback
    // arrived (a degraded compositor).
    void recordFallbackRender();

    // T238 correlation trace. `DRAGONFRUIT_CHROME_TRACE=1` emits one line per
    // stage with a monotonically increasing id. Timestamps carry both epoch
    // milliseconds (matching the compositor's `DFTRACE`) and a microsecond
    // delta from the interaction start, so individual openings can be lined
    // up with compositor frame/present traces without protocol changes.
    bool traceEnabled() const { return m_traceEnabled; }
    // The innermost open interaction, or 0. Import/allocation event lines use
    // this so a cold dma-buf wait can be attributed to one opening.
    quint64 activeTraceId() const;
    // Returns 0 when tracing is off.
    quint64 beginTrace(const QString &kind, const QString &detail = QString());
    void traceEvent(quint64 id, const QString &stage,
                    const QString &detail = QString());
    // Ends the interaction and returns its elapsed microseconds (or -1).
    qint64 endTrace(quint64 id, const QString &detail = QString());

    // The greppable `chrome stats` / `chrome stage stats` / `chrome surface`
    // lines for `label`. The emit path prints these; tests read them without
    // capturing qInfo.
    QStringList summaryLines(const QString &label) const;

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
        quint64 copies = 0;
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
        quint64 sceneResizes = 0;
        quint64 fboAllocs = 0;
        quint64 fboBytesLive = 0;
        quint64 fboBytesPeak = 0;
        quint64 dmabufImports = 0;
        quint64 dmabufImportFailures = 0;
        quint64 ringExhausted = 0;
        quint64 popoverOpens = 0;
        QVector<qint64> commitUs;
        QVector<qint64> frameCallbackUs;
        QVector<qint64> dmabufHoldUs;
        QVector<qint64> dmabufSharedHoldUs;
        QVector<qint64> popoverOpenUs;
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
    quint64 m_copies = 0;
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
    quint64 m_sceneResizes = 0;
    quint64 m_fboAllocs = 0;
    quint64 m_fboBytesLive = 0;
    quint64 m_fboBytesPeak = 0;
    quint64 m_dmabufImports = 0;
    quint64 m_dmabufImportFailures = 0;
    quint64 m_ringExhausted = 0;
    quint64 m_popoverOpens = 0;
    QVector<qint64> m_commitUs;
    QVector<qint64> m_frameCallbackUs;
    QVector<qint64> m_fboCreateUs;
    QVector<qint64> m_dmabufImportUs;
    QVector<qint64> m_dmabufRoundtripUs;
    QVector<qint64> m_dmabufHoldUs;
    QVector<qint64> m_dmabufSharedHoldUs;
    QVector<qint64> m_commitFlushUs;
    QVector<qint64> m_popoverOpenUs;
    QHash<QString, SurfaceStats> m_surfaces;
    QElapsedTimer m_renderTimer;

    bool m_traceEnabled = false;
    quint64 m_traceSeq = 0;
    QElapsedTimer m_traceClock;
    QHash<quint64, qint64> m_traceStartNs;
    QVector<quint64> m_traceStack;
};

// Percentile helpers shared with the summary (nearest-rank, 0-based).
qint64 chromeStatsPercentile(const QVector<qint64> &values, int percent);
