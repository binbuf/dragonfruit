// SPDX-License-Identifier: MIT
// GPU chrome frame producer (T-21.1d).
//
// This is the T185 `ChromeFrameProducer` with a GPU render target instead of
// the software ping-pong QImage pair: one shared QOpenGLContext, one
// QOffscreenSurface, a hidden QQuickWindow driven by QQuickRenderControl, and
// a reusable QOpenGLFramebufferObject wrapped in a QQuickRenderTarget. The
// target's device-pixel-ratio keeps QML layout logical while rasterization is
// physical (T184/ADR 0192).
//
// T186b adds the EGLImage/dmabuf export of the rendered FBO. `renderFrame()`
// remains the readback path used by the non-Dock surfaces (they still commit
// through T185 shm); `exportFrame()` renders once and exports the FBO without
// a CPU readback. A failed export is a recorded fallback, never a black
// surface.
#pragma once

#include <QHash>
#include <QImage>
#include <QQuickRenderTarget>
#include <QSize>
#include <QVector>

#include "chromedmabuf.h"
#include "chromedmabufexport.h"
#include "chromeframeproducer.h"
#include "gpuchromecapabilities.h"

class QOffscreenSurface;
class QOpenGLContext;
class QOpenGLFramebufferObject;
class QQuickRenderControl;
class QTimer;

class GpuChromeFrameProducer : public ChromeFrameProducer
{
    Q_OBJECT

public:
    explicit GpuChromeFrameProducer(QObject *parent = nullptr);
    ~GpuChromeFrameProducer() override;

    // Two independently sized render targets share one QQuickWindow: the
    // strip (or bar) scene and, when a popup is open, a popup-only scene. The
    // popup target is sized by its content, so opening a narrow menu no longer
    // enlarges every attachment in the strip ring. The controller sets the
    // window size and the root item offset before rendering a view.
    enum View { PrimaryView = 0, PopupView = 1 };

    // Create the GL context, offscreen surface, render control, and hidden
    // window. On failure returns false, sets `fallbackReason()`, and leaves
    // the object inert; the caller keeps the T185 shm producer.
    bool initialize(QString *reason = nullptr);
    bool initialized() const { return m_initialized; }
    bool hasWindow() const { return m_window != nullptr; }

    QQuickWindow *window() const override { return m_window; }
    QQuickRenderControl *renderControl() const { return m_renderControl; }
    bool renderControlActive() const override { return m_renderControl != nullptr; }
    bool softwareSceneGraph() const override { return false; }
    bool gpuActive() const override { return m_initialized; }
    QString fallbackReason() const override { return m_reason; }

    // Logical -> physical factor for the render target. The T185 contract
    // passes a physical buffer size to resize(); this producer divides it by
    // the DPR to keep the QML scene in logical units.
    void setDevicePixelRatio(qreal dpr);
    qreal devicePixelRatio() const { return m_dpr; }

    // The compositor's linux-dmabuf feedback table. An exported frame is only
    // committed when its exact format/modifier pair appears here; T186b sets
    // this once from `ShellProtocol::gpuCapabilities()`.
    void setDmabufFormats(
        const QVector<GpuChromeCapabilities::FormatModifier> &formats);
    QVector<GpuChromeCapabilities::FormatModifier> dmabufFormats() const
    {
        return m_dmabufFormats;
    }

    void resize(const QSize &bufferSize) override;
    // Renders once into the reusable FBO and returns a verification/readback
    // image (the non-Dock surfaces still use the T185 shm commit path).
    QImage renderFrame() override;
    // Render one view for the shm fallback. `view` is `PrimaryView` or
    // `PopupView`; the image comes from that view's ring.
    QImage renderFrame(int view);
    // Renders once into a free ring slot and exports that slot as a dmabuf.
    // No CPU readback. The returned frame names its producer slot; the fd
    // stays owned by this producer and is valid until the slot is retired.
    // The slot remains busy until `releaseDmabufFrame()` (from
    // `wl_buffer.release`) or `abandonDmabufFrame()` (deferred/failed commit).
    // This is the synchronous API (the smoke rail): it waits for the fence
    // before exporting. The controller uses `submitFrame()` instead.
    bool exportFrame(ChromeDmabufFrame *frame, QString *reason = nullptr);
    // Render/export one view. `view` is `PrimaryView` or `PopupView`; each has
    // its own ring, so a popup allocation never occupies a strip slot.
    bool exportFrame(int view, ChromeDmabufFrame *frame,
                     QString *reason = nullptr);

    // Asynchronous GPU completion (T238). `submitFrame()` renders into a free
    // ring slot and retains its fence without waiting; the producer polls that
    // fence from its own event-loop timer on the render context and emits
    // `frameReady(view)` when the slot may be exported and committed. A fence
    // that exceeds `completionBudgetUs()` emits
    // `frameFailed(view, "sync-timeout")`; the slot is abandoned and the last
    // completed frame stays the readback fallback. Frames per view are kept in
    // submission order, so commits stay in order even when several surfaces
    // share one producer.
    bool submitFrame(int view, QString *reason = nullptr);
    // T238: submit with a conservative change report. `damage` is in logical
    // scene coordinates; the producer maps it through the device pixel ratio,
    // clips it to the render target, and carries it on the exported frame. An
    // unknown report (the default above) damages every target in full.
    bool submitFrame(int view, const ChromeSceneDamage &damage,
                     QString *reason = nullptr);
    // Export the completed slot for `view` and mark it compositor-owned.
    // False when no frame is ready. The slot stays busy until its release.
    bool takeCompletedFrame(int view, ChromeDmabufFrame *frame,
                            QString *reason = nullptr);
    bool hasPendingFrame(int view) const;
    bool hasReadyFrame(int view) const;
    // Return a completed but uncommitted frame to the ring (the scene was
    // discarded before its commit).
    void abandonReadyFrame(int view);
    // The bounded completion budget for one submitted frame.
    static qint64 completionBudgetUs();

    // The stats label for a view (`dock`/`dock-popup`, `menubar`/...). The
    // controller sets the popup label so allocation/peak accounting names the
    // surface that owns the bytes.
    void setViewLabel(int view, const QString &label);
    QString viewLabel(int view) const;
    // Return a committed slot to the ring. `serial` must match the frame that
    // was committed; stale releases are ignored. The view is resolved by the
    // serial, so the release path does not need to know which target it was.
    void releaseDmabufFrame(int slot, uint64_t serial);
    // Return an exported but not committed slot to the ring.
    void abandonDmabufFrame(int slot);
    void abandonDmabufFrame(int view, int slot);
    // Release every slot in a view's ring. Free slots are destroyed now; a
    // slot the compositor still holds is destroyed on its release. Used when
    // the popup interaction session ends and its warm target is returned.
    void discardView(int view);
    // Read the FBO last rendered by exportFrame()/renderFrame() without
    // rendering again. Used for the one-frame SHM fallback after a failed
    // export/import.
    QImage readbackCurrentFrame();
    QImage readbackCurrentFrame(int view);
    QImage readbackFrame() const { return m_lastFrame; }
    QString exportReason() const { return m_exportReason; }

    QSize framebufferSize() const;
    QSize framebufferSize(int view) const;
    unsigned int framebufferTexture() const;
    unsigned int framebufferTexture(int view) const;
    int dmabufRingCapacity() const { return 3; }
    int dmabufRingLiveCount() const;
    int dmabufRingLiveCount(int view) const;
    quint64 dmabufRingAllocations() const;
    quint64 dmabufRingAllocations(int view) const;

    // GPU-first transport policy. `DRAGONFRUIT_CHROME_GPU` is an opt-out:
    // unset/"auto" (and "1"/"on"/"true") request the GPU rail, which falls
    // back to the T185 software/shm path if any prerequisite is missing.
    // "0"/"off"/"false" forces the T185 path (diagnostics and tests).
    static bool environmentRequested();
    static QString environmentValue();

signals:
    // A submitted frame completed and its dmabuf export succeeded. The
    // controller exports it with `takeCompletedFrame()` and commits it.
    void frameReady(int view);
    // A submitted frame could not be completed or exported. `reason` is the
    // same token the synchronous path reports (`sync-timeout`, `no-egl-export`,
    // ...); the slot has already been returned to the ring.
    void frameFailed(int view, const QString &reason);

private:
    struct SlotResources {
        QOpenGLFramebufferObject *fbo = nullptr;
        ChromeDmabufFrame frame;
        // The export serial assigned at submit. `ensureDmabufExport()` may
        // rebuild `frame` from the exporter, which does not carry the serial,
        // so it is kept here and re-applied before the commit.
        uint64_t serial = 0;
        // True once this slot held a frame whose fence signalled and whose
        // content may be reused without a forced full repaint.
        bool rendered = false;
        // The retained completion fence for a submitted frame, or null when
        // the render already finished synchronously.
        void *fence = nullptr;
        // Monotonic submit time, for the completion budget and the wait stats.
        qint64 submitNs = 0;
        // T238: the conservative change report supplied at submit, in
        // framebuffer pixels. `ensureDmabufExport()` rebuilds `frame` from the
        // exporter, which does not carry damage, so it is kept here and
        // re-applied with the serial before the commit.
        ChromeFrameDamage damage;
    };

    struct ViewState {
        ChromeDmabufRing ring;
        QHash<int, SlotResources> slotResources;
        int lastRenderedSlot = -1;
        // Slots whose fence is still being polled, oldest first. Several
        // surfaces can share one producer and view (one lock surface per
        // output), so a single in-flight slot is not enough.
        QVector<int> pendingSlots;
        // Completed + exported slots waiting for `takeCompletedFrame()`,
        // oldest first. Fences signal in submission order, so the commit
        // order matches the submit order.
        QVector<int> readySlots;
        QString label;
    };

    // The result of a non-blocking fence check for one slot.
    enum class Completion { Pending, Ready, Timeout };

    ViewState &viewState(int view);
    const ViewState *viewState(int view) const;

    bool configureRing(int view, const QSize &physical);
    // Render into `slot` and retain the completion fence. Does not wait.
    bool renderToFramebuffer(int view, int slot);
    // Blocking completion for the synchronous API only.
    bool waitForSlot(int view, int slot, qint64 budgetUs);
    // Zero-timeout fence check. Deletes the fence once it settles.
    Completion checkCompletion(int view, int slot);
    // Promote a fence-complete slot to ready and export its dmabuf.
    QString finishReadySlot(int view, int slot);
    void pollCompletion();
    void armCompletionPoll();
    void cancelPending(int view);
    void clearFence(SlotResources &resources);
    bool ensureFramebuffer(int view, int slot, const QSize &physical);
    bool ensureDmabufExport(int view, int slot);
    QImage readbackSlot(int view, int slot);
    void collectRetiredSlots(int view);
    void destroySlotResources(int view, int slot);
    void releaseGpu();
    void setFailure(const QString &reason);

    QOpenGLContext *m_context = nullptr;
    // QQuickRenderControl's RHI OpenGL backend creates its own context that
    // shares with m_context and makes it current during initialize(). Render
    // targets and FBOs must be created on that context, not on m_context.
    QOpenGLContext *m_renderContext = nullptr;
    QOffscreenSurface *m_surface = nullptr;
    QQuickRenderControl *m_renderControl = nullptr;
    QQuickWindow *m_window = nullptr;
    // One ring per view: the strip and popup targets never share a slot.
    QHash<int, ViewState> m_views;
    ChromeDmabufExporter m_exporter;
    uint64_t m_dmabufSerial = 0;
    QVector<GpuChromeCapabilities::FormatModifier> m_dmabufFormats;
    qreal m_dpr = 1.0;
    bool m_initialized = false;
    bool m_exporterInitialized = false;
    bool m_geometryDirty = true;
    QString m_reason;
    QString m_exportReason;
    QImage m_lastFrame;
    // The completion poll runs on the shell event loop while any view has a
    // submitted frame. It is stopped as soon as every fence has settled.
    QTimer *m_completionTimer = nullptr;
    int m_completionPolls = 0;
};
