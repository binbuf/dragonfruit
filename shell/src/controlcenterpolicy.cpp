// SPDX-License-Identifier: MIT
#include "controlcenterpolicy.h"

namespace {

// The four corner keys, in `HotCorner::ALL` order.
const char *const kCornerKeys[] = {
    "overview.hotCornerTopLeft",
    "overview.hotCornerTopRight",
    "overview.hotCornerBottomLeft",
    "overview.hotCornerBottomRight",
};

} // namespace

QString focusModeForToggle(bool enabled)
{
    return enabled ? QStringLiteral("dnd") : QStringLiteral("off");
}

QString colorSchemeForDarkToggle(bool dark)
{
    return dark ? QStringLiteral("dark") : QStringLiteral("light");
}

QVariantMap missionControlView(const QVariantMap &values)
{
    const bool gesture = values.value(QStringLiteral("gestures.enabled"), true).toBool()
        && values.value(QStringLiteral("gestures.missionControl"), true).toBool();
    int corners = 0;
    for (const char *key : kCornerKeys) {
        if (values.value(QLatin1String(key)).toString()
                == QLatin1String("mission-control"))
            ++corners;
    }
    const bool reachable = gesture || corners > 0;
    QString label;
    if (gesture && corners == 0)
        label = QStringLiteral("Gesture");
    else if (gesture)
        label = QStringLiteral("Gesture, %1 corner(s)").arg(corners);
    else if (corners == 0)
        label = QStringLiteral("No trigger");
    else
        label = QStringLiteral("%1 corner(s)").arg(corners);

    QVariantMap view;
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("overview"));
    view.insert(QStringLiteral("label"), label);
    view.insert(QStringLiteral("reachable"), reachable);
    view.insert(QStringLiteral("gesture"), gesture);
    view.insert(QStringLiteral("cornerCount"), corners);
    return view;
}