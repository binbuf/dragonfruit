// SPDX-License-Identifier: GPL-3.0-only
#include "compositorpolicy.h"

#include <cmath>

namespace {

constexpr double kDefaultTransparency = 0.5255;

QVariant readValue(const QVariantMap &values, const QString &key, const QVariant &fallback)
{
    const QVariant value = values.value(key);
    return value.isValid() ? value : fallback;
}

// A malformed or out-of-range slider is a safe local material, not a reason to
// forward a poisoned value: fall back to the measured schema midpoint.
double sanitizeTransparency(const QVariant &value)
{
    bool ok = false;
    const double number = value.toDouble(&ok);
    if (!ok || !std::isfinite(number))
        return kDefaultTransparency;
    return qBound(0.0, number, 1.0);
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
    policy.transparency = sanitizeTransparency(
        readValue(values, QStringLiteral("appearance.transparency"),
                  policy.transparency));
    policy.reduceTransparency =
        readValue(values, QStringLiteral("accessibility.reduceTransparency"),
                  policy.reduceTransparency)
            .toBool();
    policy.increaseContrast =
        readValue(values, QStringLiteral("accessibility.increaseContrast"),
                  policy.increaseContrast)
            .toBool();
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
    policy.pointerSpeed =
        readValue(values, QStringLiteral("input.pointerSpeed"), policy.pointerSpeed).toDouble();
    policy.naturalScroll =
        readValue(values, QStringLiteral("input.naturalScroll"), policy.naturalScroll).toBool();
    policy.tapToClick =
        readValue(values, QStringLiteral("input.tapToClick"), policy.tapToClick).toBool();
    policy.leftHanded =
        readValue(values, QStringLiteral("input.leftHanded"), policy.leftHanded).toBool();
    policy.scrollMethod =
        readValue(values, QStringLiteral("input.scrollMethod"), policy.scrollMethod).toString();
    policy.secondaryClick =
        readValue(values, QStringLiteral("input.secondaryClick"), policy.secondaryClick)
            .toString();
    policy.scrollSpeed =
        readValue(values, QStringLiteral("input.scrollSpeed"), policy.scrollSpeed).toDouble();
    policy.doubleClickMs =
        readValue(values, QStringLiteral("input.doubleClickSpeed"), policy.doubleClickMs).toInt();
    policy.haptics =
        readValue(values, QStringLiteral("input.haptics"), policy.haptics).toBool();
    // T320: the per-family pointer snapshots. A pane row writes the family key;
    // the legacy `input.*` pointer key is the fallback an upgraded store still
    // carries. The two families differ in their schema defaults (a mouse's
    // scroll method is `button`, a trackpad's is `two-finger`).
    policy.mousePointer.pointerSpeed =
        readValue(values, QStringLiteral("input.mouse.pointerSpeed"),
                  policy.mousePointer.pointerSpeed)
            .toDouble();
    policy.mousePointer.naturalScroll =
        readValue(values, QStringLiteral("input.mouse.naturalScroll"),
                  policy.naturalScroll)
            .toBool();
    policy.mousePointer.tapToClick = policy.tapToClick;
    policy.mousePointer.leftHanded =
        readValue(values, QStringLiteral("input.mouse.leftHanded"),
                  policy.leftHanded)
            .toBool();
    policy.mousePointer.scrollMethod =
        readValue(values, QStringLiteral("input.mouse.scrollMethod"),
                  QStringLiteral("button"))
            .toString();
    policy.mousePointer.accelProfile =
        readValue(values, QStringLiteral("input.mouse.pointerAcceleration"), true).toBool()
            ? QStringLiteral("adaptive")
            : QStringLiteral("flat");

    policy.trackpadPointer.pointerSpeed =
        readValue(values, QStringLiteral("input.trackpad.pointerSpeed"),
                  policy.trackpadPointer.pointerSpeed)
            .toDouble();
    policy.trackpadPointer.naturalScroll =
        readValue(values, QStringLiteral("input.trackpad.naturalScroll"),
                  policy.naturalScroll)
            .toBool();
    policy.trackpadPointer.tapToClick = policy.tapToClick;
    policy.trackpadPointer.leftHanded = policy.leftHanded;
    policy.trackpadPointer.scrollMethod =
        readValue(values, QStringLiteral("input.scrollMethod"),
                  policy.trackpadPointer.scrollMethod)
            .toString();
    policy.trackpadPointer.accelProfile = QStringLiteral("adaptive");
    // T275: the accessibility accommodations.
    policy.magnifierEnabled =
        readValue(values, QStringLiteral("accessibility.magnifier"),
                  policy.magnifierEnabled)
            .toBool();
    policy.magnifierZoom =
        readValue(values, QStringLiteral("accessibility.magnifierZoom"),
                  policy.magnifierZoom)
            .toDouble();
    policy.magnifierMode =
        readValue(values, QStringLiteral("accessibility.magnifierMode"),
                  policy.magnifierMode)
            .toString();
    policy.accessibilityShortcut =
        readValue(values, QStringLiteral("accessibility.shortcut"),
                  policy.accessibilityShortcut)
            .toString();
    // T276: the Overview/hot-corner snapshot. A malformed action/modifier id
    // is forwarded as-is; the compositor ignores unknown spellings and keeps
    // the current corner, matching the other policy setters.
    policy.hotCornerTopLeft =
        readValue(values, QStringLiteral("overview.hotCornerTopLeft"),
                  policy.hotCornerTopLeft)
            .toString();
    policy.hotCornerTopRight =
        readValue(values, QStringLiteral("overview.hotCornerTopRight"),
                  policy.hotCornerTopRight)
            .toString();
    policy.hotCornerBottomLeft =
        readValue(values, QStringLiteral("overview.hotCornerBottomLeft"),
                  policy.hotCornerBottomLeft)
            .toString();
    policy.hotCornerBottomRight =
        readValue(values, QStringLiteral("overview.hotCornerBottomRight"),
                  policy.hotCornerBottomRight)
            .toString();
    policy.hotCornerTopLeftModifier =
        readValue(values, QStringLiteral("overview.hotCornerTopLeftModifier"),
                  policy.hotCornerTopLeftModifier)
            .toString();
    policy.hotCornerTopRightModifier =
        readValue(values, QStringLiteral("overview.hotCornerTopRightModifier"),
                  policy.hotCornerTopRightModifier)
            .toString();
    policy.hotCornerBottomLeftModifier =
        readValue(values, QStringLiteral("overview.hotCornerBottomLeftModifier"),
                  policy.hotCornerBottomLeftModifier)
            .toString();
    policy.hotCornerBottomRightModifier =
        readValue(values, QStringLiteral("overview.hotCornerBottomRightModifier"),
                  policy.hotCornerBottomRightModifier)
            .toString();
    policy.workspaceCount =
        readValue(values, QStringLiteral("workspaces.count"), policy.workspaceCount)
            .toInt();
    policy.switchToAppSpace =
        readValue(values, QStringLiteral("overview.switchToAppSpace"),
                  policy.switchToAppSpace)
            .toBool();
    // The first input source is the active layout; a `layout:variant` spelling
    // carries the variant. Empty keeps the host XKB_DEFAULT_* configuration.
    const QStringList sources =
        readValue(values, QStringLiteral("input.sources"), QStringList()).toStringList();
    if (!sources.isEmpty()) {
        const QString first = sources.first();
        const qsizetype separator = first.indexOf(QLatin1Char(':'));
        if (separator >= 0) {
            policy.keyboardLayout = first.left(separator);
            policy.keyboardVariant = first.mid(separator + 1);
        } else {
            policy.keyboardLayout = first;
            policy.keyboardVariant.clear();
        }
    }
    return policy;
}