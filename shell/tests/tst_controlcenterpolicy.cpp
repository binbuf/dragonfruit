// SPDX-License-Identifier: MIT
// T-11.3b / T-15.5b: the pure Control Center tile -> owner mapping. The Focus
// and dark-mode toggle spellings, and the Mission Control / hot corners trigger
// summary the shell projects from settingsd values. No bus, Wayland, or QML.
#include <QtTest>

#include "controlcenterpolicy.h"
#include "settingsclient.h"

class TestControlCenterPolicy : public QObject
{
    Q_OBJECT

private slots:
    void focusToggleMapsToTheNotificationMode();
    void darkToggleMapsToTheScheme();
    void missionControlSummaryMatchesTheSchemaDefaults();
    void missionControlCountsOnlyMissionControlCorners();
    void missionControlGestureRequiresBothSwitches();
    void missionControlNoTriggerIsValid();
};

void TestControlCenterPolicy::focusToggleMapsToTheNotificationMode()
{
    QCOMPARE(focusModeForToggle(true), QStringLiteral("dnd"));
    QCOMPARE(focusModeForToggle(false), QStringLiteral("off"));
}

void TestControlCenterPolicy::darkToggleMapsToTheScheme()
{
    QCOMPARE(colorSchemeForDarkToggle(true), QStringLiteral("dark"));
    QCOMPARE(colorSchemeForDarkToggle(false), QStringLiteral("light"));
}

void TestControlCenterPolicy::missionControlSummaryMatchesTheSchemaDefaults()
{
    // The shell always holds the schema defaults, so the corner keys are
    // present: the gesture trio is on and the top-left corner is Mission
    // Control.
    const QVariantMap view = missionControlView(settingsSchemaDefaults());
    QCOMPARE(view.value(QStringLiteral("state")).toString(), QStringLiteral("available"));
    QCOMPARE(view.value(QStringLiteral("glyph")).toString(), QStringLiteral("overview"));
    QCOMPARE(view.value(QStringLiteral("label")).toString(),
             QStringLiteral("Gesture, 1 corner(s)"));
    QCOMPARE(view.value(QStringLiteral("reachable")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("gesture")).toBool(), true);
    QCOMPARE(view.value(QStringLiteral("cornerCount")).toInt(), 1);

    // A bare map (no corner keys at all) has no corner, so the gesture is the
    // only trigger.
    const QVariantMap bare = missionControlView({});
    QCOMPARE(bare.value(QStringLiteral("label")).toString(), QStringLiteral("Gesture"));
    QCOMPARE(bare.value(QStringLiteral("cornerCount")).toInt(), 0);
}

void TestControlCenterPolicy::missionControlCountsOnlyMissionControlCorners()
{
    QVariantMap values;
    values.insert(QStringLiteral("overview.hotCornerTopLeft"), QStringLiteral("none"));
    values.insert(QStringLiteral("overview.hotCornerTopRight"), QStringLiteral("desktop-reveal"));
    values.insert(QStringLiteral("overview.hotCornerBottomLeft"), QStringLiteral("mission-control"));
    values.insert(QStringLiteral("overview.hotCornerBottomRight"), QStringLiteral("mission-control"));
    const QVariantMap view = missionControlView(values);
    QCOMPARE(view.value(QStringLiteral("cornerCount")).toInt(), 2);
    QCOMPARE(view.value(QStringLiteral("label")).toString(),
             QStringLiteral("Gesture, 2 corner(s)"));
}

void TestControlCenterPolicy::missionControlGestureRequiresBothSwitches()
{
    QVariantMap values;
    values.insert(QStringLiteral("overview.hotCornerTopLeft"), QStringLiteral("none"));
    values.insert(QStringLiteral("gestures.missionControl"), true);
    values.insert(QStringLiteral("gestures.enabled"), false);
    const QVariantMap view = missionControlView(values);
    QCOMPARE(view.value(QStringLiteral("gesture")).toBool(), false);
    QCOMPARE(view.value(QStringLiteral("label")).toString(), QStringLiteral("No trigger"));
    QCOMPARE(view.value(QStringLiteral("reachable")).toBool(), false);
}

void TestControlCenterPolicy::missionControlNoTriggerIsValid()
{
    // The schema defaults seed all four keys, so a real map always carries
    // them; a bare map is the all-defaults case. Moving the corner off Mission
    // Control and disabling the gesture leaves no trigger at all.
    QVariantMap values = settingsSchemaDefaults();
    values.insert(QStringLiteral("overview.hotCornerTopLeft"), QStringLiteral("lock-screen"));
    values.insert(QStringLiteral("gestures.missionControl"), false);
    const QVariantMap view = missionControlView(values);
    QCOMPARE(view.value(QStringLiteral("label")).toString(), QStringLiteral("No trigger"));
    QCOMPARE(view.value(QStringLiteral("reachable")).toBool(), false);
    QCOMPARE(view.value(QStringLiteral("cornerCount")).toInt(), 0);
}

QTEST_MAIN(TestControlCenterPolicy)
#include "tst_controlcenterpolicy.moc"