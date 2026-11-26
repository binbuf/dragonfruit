// SPDX-License-Identifier: GPL-3.0-or-later
#include "popupvisualtransaction.h"

#include <QtGlobal>

qint64 PopupVisualTransaction::clampRetainMs(qint64 ms)
{
    return qBound<qint64>(0, ms, kMaxRetainMs);
}

QString PopupVisualTransaction::phaseName(Phase phase)
{
    switch (phase) {
    case Phase::Idle:
        return QStringLiteral("idle");
    case Phase::Opening:
        return QStringLiteral("opening");
    case Phase::Open:
        return QStringLiteral("open");
    case Phase::Closing:
        return QStringLiteral("closing");
    case Phase::Complete:
        return QStringLiteral("complete");
    case Phase::Cancelled:
        return QStringLiteral("cancelled");
    }
    return QStringLiteral("idle");
}

QString PopupVisualTransaction::transportName(Transport transport)
{
    switch (transport) {
    case Transport::None:
        return QStringLiteral("none");
    case Transport::SharedMemory:
        return QStringLiteral("shm");
    case Transport::Dmabuf:
        return QStringLiteral("dmabuf");
    }
    return QStringLiteral("none");
}

void PopupVisualTransaction::open(quint32 generation, const QString &outputName,
                                  qint64 nowMs)
{
    Q_UNUSED(nowMs);
    if (active())
        release(QStringLiteral("superseded"));
    m_phase = Phase::Opening;
    m_generation = generation;
    m_contentSerial = 0;
    m_lastAcceptedTransport = Transport::None;
    m_inputActive = false;
    m_outputName = outputName;
    m_crop = QRect();
    m_paint = QRect();
    m_materialRegions.clear();
    m_inputRects.clear();
    m_retainedUntilMs = 0;
}

bool PopupVisualTransaction::retarget(quint32 generation,
                                      const QString &outputName,
                                      const QRect &crop, const QRect &paint,
                                      const QVariantList &materialRegions,
                                      const QList<QRect> &inputRects,
                                      bool inputActive, qint64 nowMs)
{
    Q_UNUSED(nowMs);
    if (m_phase != Phase::Opening && m_phase != Phase::Open)
        return false;
    if (generation < m_generation)
        return false;
    m_generation = generation;
    m_outputName = outputName;
    m_crop = crop;
    m_paint = paint;
    m_materialRegions = materialRegions;
    m_inputRects = inputRects;
    m_inputActive = inputActive;
    return true;
}

bool PopupVisualTransaction::beginFrame(quint32 generation,
                                        quint32 contentSerial,
                                        Transport transport)
{
    if (m_phase != Phase::Opening && m_phase != Phase::Open
        && m_phase != Phase::Closing)
        return false;
    if (generation != m_generation)
        return false;
    // A closing snapshot may still receive its exit frames, but the content
    // sequence must always advance. This is what fences a late GPU frame or a
    // transport switch from overwriting newer pixels.
    if (contentSerial <= m_contentSerial)
        return false;
    if (transport != Transport::None && m_lastAcceptedTransport != Transport::None
        && transport != m_lastAcceptedTransport) {
        ++m_transportSwitches;
    }
    m_contentSerial = contentSerial;
    if (transport != Transport::None)
        m_lastAcceptedTransport = transport;
    if (m_phase == Phase::Opening)
        m_phase = Phase::Open;
    return true;
}

void PopupVisualTransaction::storeSnapshot(const QImage &image,
                                           const QSize &logicalSize,
                                           const QRect &sourceRect,
                                           qint64 nowMs)
{
    Q_UNUSED(nowMs);
    m_snapshot = Snapshot();
    m_snapshot.transport = m_lastAcceptedTransport;
    m_snapshot.generation = m_generation;
    m_snapshot.contentSerial = m_contentSerial;
    m_snapshot.logicalSize = logicalSize;
    m_snapshot.crop = QRect(QPoint(0, 0), logicalSize);
    m_snapshot.paint = m_paint;
    if (m_lastAcceptedTransport == Transport::SharedMemory && !image.isNull())
        m_snapshot.image = image;
    m_snapshot.valid = true;
    Q_UNUSED(sourceRect);
}

void PopupVisualTransaction::dismiss(quint32 generation, qint64 retainMs,
                                     qint64 nowMs)
{
    if (m_phase != Phase::Opening && m_phase != Phase::Open
        && m_phase != Phase::Closing)
        return;
    if (generation < m_generation)
        return;
    m_generation = generation;
    m_phase = Phase::Closing;
    m_inputActive = false;
    if (m_retainedUntilMs == 0)
        m_retainedUntilMs = nowMs + clampRetainMs(retainMs);
}

bool PopupVisualTransaction::complete(quint32 generation, qint64 nowMs)
{
    if (m_phase != Phase::Opening && m_phase != Phase::Open
        && m_phase != Phase::Closing && m_phase != Phase::Complete)
        return false;
    if (generation < m_generation)
        return false;
    m_generation = generation;
    m_phase = Phase::Complete;
    m_inputActive = false;
    if (m_retainedUntilMs <= nowMs)
        release(QStringLiteral("complete"));
    return true;
}

bool PopupVisualTransaction::cancel(quint32 generation, const QString &reason,
                                    qint64 nowMs)
{
    Q_UNUSED(nowMs);
    if (!active())
        return false;
    if (generation < m_generation)
        return false;
    release(reason);
    return true;
}

bool PopupVisualTransaction::outputRemoved(const QString &outputName,
                                           qint64 nowMs)
{
    Q_UNUSED(nowMs);
    if (!active())
        return false;
    // An empty name means "any output was removed"; an all-output popup may
    // have been shown on the detached display by the per-output overlay rule.
    if (!outputName.isEmpty() && !m_outputName.isEmpty()
        && outputName != m_outputName)
        return false;
    release(QStringLiteral("output"));
    return true;
}

bool PopupVisualTransaction::releaseExpired(qint64 nowMs)
{
    if (m_phase != Phase::Closing && m_phase != Phase::Complete)
        return false;
    if (m_retainedUntilMs <= 0 || nowMs < m_retainedUntilMs)
        return false;
    release(QStringLiteral("expired"));
    return true;
}

bool PopupVisualTransaction::inputActive() const
{
    return (m_phase == Phase::Opening || m_phase == Phase::Open) && m_inputActive;
}

bool PopupVisualTransaction::hasRetainedSnapshot() const
{
    return m_snapshot.valid;
}

bool PopupVisualTransaction::retained(qint64 nowMs) const
{
    if (!m_snapshot.valid)
        return false;
    if (m_phase != Phase::Closing && m_phase != Phase::Complete)
        return false;
    return m_retainedUntilMs > nowMs;
}

qint64 PopupVisualTransaction::retainedBytes() const
{
    return m_snapshot.valid ? m_snapshot.image.sizeInBytes() : 0;
}

void PopupVisualTransaction::release(const QString &reason)
{
    m_lastReleasedBytes = retainedBytes();
    m_lastReleaseReason = reason;
    m_snapshot = Snapshot();
    m_retainedUntilMs = 0;
    m_inputActive = false;
    m_materialRegions.clear();
    m_inputRects.clear();
    m_phase = Phase::Idle;
}
