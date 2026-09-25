// SPDX-License-Identifier: MIT
#include "osdmodel.h"

#include <QtGlobal>

bool OsdModel::present(Kind kind, double value, bool muted, qint64 nowMs)
{
    if (kind == Kind::None)
        return false;
    m_kind = kind;
    m_value = qBound(0.0, value, 1.0);
    m_muted = muted;
    m_shownAt = nowMs;
    m_deadline = nowMs + kDismissMs;
    // A fullscreen surface suppresses the brief overlay: presentation must not
    // be disturbed. The state is still recorded, but nothing is shown.
    m_visible = !m_fullscreen;
    return m_visible;
}

bool OsdModel::presentVolume(double value, bool muted, qint64 nowMs)
{
    return present(Kind::Volume, value, muted, nowMs);
}

bool OsdModel::presentBrightness(double value, qint64 nowMs)
{
    return present(Kind::Brightness, value, false, nowMs);
}

bool OsdModel::tick(qint64 nowMs)
{
    if (m_visible && nowMs >= m_deadline)
        m_visible = false;
    return m_visible;
}

void OsdModel::hide()
{
    m_visible = false;
}

void OsdModel::setFullscreen(bool fullscreen)
{
    m_fullscreen = fullscreen;
    if (fullscreen)
        m_visible = false;
}

QString OsdModel::kindName() const
{
    switch (m_kind) {
    case Kind::Volume:
        return QStringLiteral("volume");
    case Kind::Brightness:
        return QStringLiteral("brightness");
    case Kind::None:
    default:
        return QString();
    }
}

double OsdModel::fade(qint64 nowMs, bool reducedMotion) const
{
    if (!m_visible)
        return 0.0;
    if (reducedMotion)
        return 1.0;
    const qint64 elapsed = nowMs - m_shownAt;
    if (elapsed < kFadeInMs)
        return qBound(0.0, static_cast<double>(elapsed) / kFadeInMs, 1.0);
    const qint64 remaining = m_deadline - nowMs;
    if (remaining < kFadeOutMs)
        return qBound(0.0, static_cast<double>(remaining) / kFadeOutMs, 1.0);
    return 1.0;
}