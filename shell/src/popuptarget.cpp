// SPDX-License-Identifier: GPL-3.0-only
#include "popuptarget.h"

#include <QtGlobal>

QSize popupTargetSize(const QSize &content, const QSize &output)
{
    if (content.isEmpty())
        return QSize();
    QSize size = content;
    if (output.isValid())
        size = size.boundedTo(output);
    return size;
}

QSize PopupTargetBudget::update(const QSize &content, const QSize &output,
                                bool sessionActive, qint64 nowMs)
{
    const QSize required = popupTargetSize(content, output);
    if (sessionActive) {
        m_lastActiveMs = nowMs;
        m_warm = false;
        if (!required.isEmpty()) {
            if (m_size.isEmpty()) {
                m_size = required;
            } else {
                // Grow-only within one session: retargeting to a smaller menu
                // (or a hover label) reuses the allocation instead of retiring
                // the ring.
                m_size.setWidth(qMax(m_size.width(), required.width()));
                m_size.setHeight(qMax(m_size.height(), required.height()));
            }
        }
    } else if (!m_size.isEmpty()) {
        const bool expired = m_lastActiveMs < 0
            || nowMs - m_lastActiveMs >= kWarmRetentionMs;
        if (expired)
            reset();
        else
            m_warm = true;
    }

    // An output change can shrink the space the retained target describes.
    if (!m_size.isEmpty() && output.isValid())
        m_size = m_size.boundedTo(output);
    return m_size;
}

void PopupTargetBudget::reset()
{
    m_size = QSize();
    m_lastActiveMs = -1;
    m_warm = false;
}
