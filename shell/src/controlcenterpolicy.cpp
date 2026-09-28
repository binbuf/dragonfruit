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

// A compact duration for the Lock Screen tile subtitle: whole seconds, then
// whole minutes, then whole hours. The settings key is whole seconds
// (`idle.lock`), so this never needs sub-second precision.
QString lockDurationLabel(int seconds)
{
    if (seconds > 0 && seconds % 3600 == 0)
        return QStringLiteral("%1 h").arg(seconds / 3600);
    if (seconds > 0 && seconds % 60 == 0)
        return QStringLiteral("%1 min").arg(seconds / 60);
    return QStringLiteral("%1 s").arg(seconds);
}

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

QVariantMap lockPolicyView(const QVariantMap &values)
{
    // The schema default is 600 s (10 minutes); 0 disables the lock stage.
    const int lockSeconds = values.value(QStringLiteral("idle.lock"), 600).toInt();
    const bool requirePassword = lockSeconds > 0;
    const QString label = requirePassword
        ? QStringLiteral("Password after %1").arg(lockDurationLabel(lockSeconds))
        : QStringLiteral("No password required");

    QVariantMap view;
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("lock"));
    view.insert(QStringLiteral("label"), label);
    view.insert(QStringLiteral("requirePassword"), requirePassword);
    view.insert(QStringLiteral("lockSeconds"), lockSeconds);
    return view;
}

QVariantMap menuBarView(const QVariantMap &values)
{
    const QString autoHide =
        values.value(QStringLiteral("menu.autoHide"),
                     QStringLiteral("full-screen")).toString();
    QString label;
    if (autoHide == QLatin1String("never"))
        label = QStringLiteral("Never");
    else if (autoHide == QLatin1String("always"))
        label = QStringLiteral("Always");
    else
        label = QStringLiteral("In Full Screen Only");

    QVariantMap view;
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("menu-bar"));
    view.insert(QStringLiteral("label"), label);
    view.insert(QStringLiteral("autoHide"), autoHide);
    view.insert(QStringLiteral("showBackground"),
                values.value(QStringLiteral("menu.showBackground"), true).toBool());
    view.insert(QStringLiteral("globalMenu"),
                values.value(QStringLiteral("menu.global"), true).toBool());
    return view;
}