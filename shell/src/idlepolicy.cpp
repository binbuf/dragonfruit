// SPDX-License-Identifier: MIT
#include "idlepolicy.h"

#include <QtGlobal>

#include <utility>

namespace {

constexpr int kMaxIdleSeconds = 86'400;

int readSeconds(const QVariantMap &values, const QString &key, int fallback)
{
    const QVariant value = values.value(key);
    if (!value.isValid())
        return fallback;
    bool ok = false;
    const int seconds = value.toInt(&ok);
    if (!ok)
        return fallback;
    return qBound(0, seconds, kMaxIdleSeconds);
}

} // namespace

IdleSettings idleSettingsFromValues(const QVariantMap &values)
{
    IdleSettings settings;
    settings.dim = readSeconds(values, QStringLiteral("idle.dim"), settings.dim);
    settings.blank = readSeconds(values, QStringLiteral("idle.blank"), settings.blank);
    settings.lock = readSeconds(values, QStringLiteral("idle.lock"), settings.lock);
    settings.suspend = readSeconds(values, QStringLiteral("idle.suspend"), settings.suspend);
    settings.preventSleep =
        values.value(QStringLiteral("power.preventSleep"), settings.preventSleep).toBool();
    return settings;
}

IdleCompositorPolicy idleCompositorPolicy(const IdleSettings &settings)
{
    IdleCompositorPolicy policy;
    policy.dim = settings.dim;
    policy.blank = settings.blank;
    policy.lock = settings.lock;
    // `power.preventSleep` is the one consumer of the suspend stage here: a
    // disabled stage never arms in the compositor, so an inhibitor race cannot
    // suspend a session that asked to stay awake.
    policy.suspend = settings.preventSleep ? 0 : settings.suspend;
    return policy;
}

IdleController::IdleController(IdleEffects effects)
    : m_effects(std::move(effects))
{
}

void IdleController::setSettings(const IdleSettings &settings)
{
    const IdleCompositorPolicy policy = idleCompositorPolicy(settings);
    m_settings = settings;
    if (m_policySent && policy == m_policy)
        return;
    m_policy = policy;
    m_policySent = true;
    if (m_effects.setPolicy)
        m_effects.setPolicy(policy);
}

void IdleController::handleAction(const QString &action)
{
    if (action == QLatin1String("idle-dim")) {
        if (m_settings.dim <= 0 || m_dimmed)
            return;
        m_dimmed = true;
        if (m_effects.setDimmed)
            m_effects.setDimmed(true);
    } else if (action == QLatin1String("idle-blank")) {
        if (m_settings.blank <= 0 || m_blanked)
            return;
        m_blanked = true;
        if (m_effects.dpmsOn)
            m_preBlankDpmsOn = m_effects.dpmsOn();
        if (m_effects.setDpms)
            m_effects.setDpms(false);
    } else if (action == QLatin1String("idle-lock")) {
        if (m_settings.lock <= 0)
            return;
        if (m_effects.lock)
            m_effects.lock();
    } else if (action == QLatin1String("idle-suspend")) {
        if (m_settings.suspend <= 0 || m_settings.preventSleep)
            return;
        if (m_effects.suspend)
            m_effects.suspend();
    } else if (action == QLatin1String("idle-awake")) {
        // Restore exactly what the stages changed: the pre-dim brightness
        // factor and the pre-blank DPMS state, not an assumed default.
        if (m_dimmed) {
            m_dimmed = false;
            if (m_effects.setDimmed)
                m_effects.setDimmed(false);
        }
        if (m_blanked) {
            m_blanked = false;
            if (m_effects.setDpms)
                m_effects.setDpms(m_preBlankDpmsOn);
        }
    }
}
