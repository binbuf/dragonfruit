// SPDX-License-Identifier: MIT
// Popup visual transaction and lifetime owner (T354).
//
// One popup is more than a sequence of independent geometry, material and
// input requests. A GPU frame can complete after the popup retargeted or
// closed, a transport can fall back from dmabuf to shm mid-frame, and the
// visual exit must outlive the input owner. This class is the single owner for
// that state: it joins the accepted content with its generation, owning output,
// anchor/crop/paint extents, material coverage and input policy, fences stale
// frames and transport switches, and retains the last accepted snapshot for the
// compositor's bounded exit lifetime.
//
// The class is pure (QtCore + QtGui image ownership) and unit-tested by
// `tst_popupvisualtransaction`. The protocol side is `df_layer_surface
// .set_popup_transaction` (since v5); a client that never sends it keeps the
// pre-T354 behavior.
#pragma once

#include <QImage>
#include <QList>
#include <QRect>
#include <QSize>
#include <QString>
#include <QVariantList>

class PopupVisualTransaction
{
public:
    enum class Phase { Idle, Opening, Open, Closing, Complete, Cancelled };
    enum class Transport { None, SharedMemory, Dmabuf };

    // The documented retained-snapshot lifetime. Long enough for the menu-bar
    // exit tail plus a compositor-owned animation (T355), short enough that an
    // idle desktop returns to the strip-only allocation.
    static constexpr qint64 kSnapshotLifetimeMs = 750;
    // A client cannot pin a full-size buffer indefinitely.
    static constexpr qint64 kMaxRetainMs = 5000;
    // Bumped when the wire contract changes; mirrored in the ADR and protocol.
    static constexpr int kContractVersion = 1;

    struct Snapshot {
        QImage image;              // shm fallback pixels; null for dmabuf
        Transport transport = Transport::None;
        quint32 generation = 0;
        quint32 contentSerial = 0;
        QSize logicalSize;
        QRect crop;
        QRect paint;
        bool valid = false;
    };

    void open(quint32 generation, const QString &outputName, qint64 nowMs);
    bool retarget(quint32 generation, const QString &outputName,
                  const QRect &crop, const QRect &paint,
                  const QVariantList &materialRegions,
                  const QList<QRect> &inputRects, bool inputActive,
                  qint64 nowMs);
    // Fence one content frame before it is committed. A stale generation, an
    // older content serial, or a frame after completion is rejected. A
    // transport change is accepted only when the serial is newer (the
    // documented dmabuf -> shm fallback).
    bool beginFrame(quint32 generation, quint32 contentSerial,
                    Transport transport);
    // Retain the pixels of the frame that was just committed. QImage is
    // implicitly shared, so this holds one frame, not a per-frame copy.
    void storeSnapshot(const QImage &image, const QSize &logicalSize,
                       const QRect &sourceRect, qint64 nowMs);
    void dismiss(quint32 generation, qint64 retainMs, qint64 nowMs);
    bool complete(quint32 generation, qint64 nowMs);
    bool cancel(quint32 generation, const QString &reason, qint64 nowMs);
    bool outputRemoved(const QString &outputName, qint64 nowMs);
    bool releaseExpired(qint64 nowMs);

    Phase phase() const { return m_phase; }
    bool active() const { return m_phase != Phase::Idle; }
    bool inputActive() const;
    bool hasRetainedSnapshot() const;
    bool retained(qint64 nowMs) const;
    qint64 retainedUntilMs() const { return m_retainedUntilMs; }
    const Snapshot &snapshot() const { return m_snapshot; }
    quint32 generation() const { return m_generation; }
    quint32 contentSerial() const { return m_contentSerial; }
    Transport transport() const { return m_lastAcceptedTransport; }
    QString outputName() const { return m_outputName; }
    QRect crop() const { return m_crop; }
    QRect paint() const { return m_paint; }
    QVariantList materialRegions() const { return m_materialRegions; }
    QList<QRect> inputRects() const { return m_inputRects; }
    qint64 retainedBytes() const;
    qint64 lastReleasedBytes() const { return m_lastReleasedBytes; }
    QString lastReleaseReason() const { return m_lastReleaseReason; }
    quint64 transportSwitches() const { return m_transportSwitches; }

    static qint64 clampRetainMs(qint64 ms);
    static QString phaseName(Phase phase);
    static QString transportName(Transport transport);

private:
    void release(const QString &reason);

    Phase m_phase = Phase::Idle;
    quint32 m_generation = 0;
    quint32 m_contentSerial = 0;
    Transport m_lastAcceptedTransport = Transport::None;
    bool m_inputActive = false;
    QString m_outputName;
    QRect m_crop;
    QRect m_paint;
    QVariantList m_materialRegions;
    QList<QRect> m_inputRects;
    Snapshot m_snapshot;
    qint64 m_retainedUntilMs = 0;
    QString m_lastReleaseReason;
    qint64 m_lastReleasedBytes = 0;
    quint64 m_transportSwitches = 0;
};
