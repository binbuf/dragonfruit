// SPDX-License-Identifier: MIT
#include "compositorpolicy.h"

namespace {

QVariant readValue(const QVariantMap &values, const QString &key, const QVariant &fallback)
{
    const QVariant value = values.value(key);
    return value.isValid() ? value : fallback;
}

} // namespace

QString resolveCompositorColorScheme(const QString &scheme, bool hostDark)
{
    if (scheme == QLatin1String("light"))
        return QStringLiteral("light");
    if (scheme == QLatin1String("dark"))
        return QStringLiteral("dark");
    // `auto` (or an unknown spelling) follows the host.
    return hostDark ? QStringLiteral("dark") : QStringLiteral("light");
}

CompositorPolicy compositorPolicyFromValues(const QVariantMap &values, bool hostDark)
{
    CompositorPolicy policy;
    policy.reducedMotion =
        readValue(values, QStringLiteral("accessibility.reduceMotion"), policy.reducedMotion)
            .toBool();
    const QString scheme = readValue(values, QStringLiteral("appearance.colorScheme"),
                                      QStringLiteral("auto"))
                               .toString();
    policy.colorScheme = resolveCompositorColorScheme(scheme, hostDark);
    policy.titlebarDoubleClick =
        readValue(values, QStringLiteral("dock.titlebarDoubleClick"),
                  policy.titlebarDoubleClick)
            .toString();
    policy.minimizedAnimation =
        readValue(values, QStringLiteral("dock.minimizedAnimation"),
                  policy.minimizedAnimation)
            .toString();
    policy.repeatDelayMs =
        readValue(values, QStringLiteral("input.repeatDelay"), policy.repeatDelayMs).toInt();
    policy.repeatRateHz =
        readValue(values, QStringLiteral("input.repeatRate"), policy.repeatRateHz).toInt();
    policy.gesturesEnabled =
        readValue(values, QStringLiteral("gestures.enabled"), policy.gesturesEnabled).toBool();
    policy.gestureSpaceSwitch =
        readValue(values, QStringLiteral("gestures.spaceSwitch"), policy.gestureSpaceSwitch)
            .toBool();
    policy.gestureMissionControl =
        readValue(values, QStringLiteral("gestures.missionControl"),
                  policy.gestureMissionControl)
            .toBool();
    return policy;
}