// SPDX-License-Identifier: MIT
// T-08.2c: the pure mapping from the settingsd motion/input keys onto the
// compositor policy the shell forwards over the private protocol. No bus,
// Wayland, or QML.
#include <QtTest>

#include "compositorpolicy.h"

class TestCompositorPolicy : public QObject
{
    Q_OBJECT

private slots:
    void defaultsMatchTheSchema();
    void everyKeyMapsThrough();
    void autoFollowsTheHost();
    void explicitSchemeIgnoresTheHost();
    void unknownSchemeFollowsTheHost();
    void liveFlipChangesThePolicy();
};

void TestCompositorPolicy::defaultsMatchTheSchema()
{
    const CompositorPolicy policy = compositorPolicyFromValues({}, /*hostDark=*/false);
    QCOMPARE(policy.reducedMotion, false);
    QCOMPARE(policy.colorScheme, QStringLiteral("light"));
    QCOMPARE(policy.titlebarDoubleClick, QStringLiteral("zoom"));
    QCOMPARE(policy.minimizedAnimation, QStringLiteral("scale"));
    QCOMPARE(policy.repeatDelayMs, 200);
    QCOMPARE(policy.repeatRateHz, 25);
    QCOMPARE(policy.gesturesEnabled, true);
    QCOMPARE(policy.gestureSpaceSwitch, true);
    QCOMPARE(policy.gestureMissionControl, true);
}

void TestCompositorPolicy::everyKeyMapsThrough()
{
    QVariantMap values;
    values.insert(QStringLiteral("appearance.colorScheme"), QStringLiteral("dark"));
    values.insert(QStringLiteral("accessibility.reduceMotion"), true);
    values.insert(QStringLiteral("dock.titlebarDoubleClick"), QStringLiteral("minimize"));
    values.insert(QStringLiteral("dock.minimizedAnimation"), QStringLiteral("none"));
    values.insert(QStringLiteral("input.repeatDelay"), 350);
    values.insert(QStringLiteral("input.repeatRate"), 40);
    values.insert(QStringLiteral("gestures.enabled"), false);
    values.insert(QStringLiteral("gestures.spaceSwitch"), false);
    values.insert(QStringLiteral("gestures.missionControl"), true);

    const CompositorPolicy policy = compositorPolicyFromValues(values, /*hostDark=*/false);
    QCOMPARE(policy.reducedMotion, true);
    QCOMPARE(policy.colorScheme, QStringLiteral("dark"));
    QCOMPARE(policy.titlebarDoubleClick, QStringLiteral("minimize"));
    QCOMPARE(policy.minimizedAnimation, QStringLiteral("none"));
    QCOMPARE(policy.repeatDelayMs, 350);
    QCOMPARE(policy.repeatRateHz, 40);
    QCOMPARE(policy.gesturesEnabled, false);
    QCOMPARE(policy.gestureSpaceSwitch, false);
    QCOMPARE(policy.gestureMissionControl, true);
}

void TestCompositorPolicy::autoFollowsTheHost()
{
    QVariantMap values;
    values.insert(QStringLiteral("appearance.colorScheme"), QStringLiteral("auto"));
    QCOMPARE(compositorPolicyFromValues(values, true).colorScheme, QStringLiteral("dark"));
    QCOMPARE(compositorPolicyFromValues(values, false).colorScheme, QStringLiteral("light"));
}

void TestCompositorPolicy::explicitSchemeIgnoresTheHost()
{
    QCOMPARE(resolveCompositorColorScheme(QStringLiteral("light"), true),
             QStringLiteral("light"));
    QCOMPARE(resolveCompositorColorScheme(QStringLiteral("dark"), false),
             QStringLiteral("dark"));
}

void TestCompositorPolicy::unknownSchemeFollowsTheHost()
{
    QCOMPARE(resolveCompositorColorScheme(QStringLiteral("sepia"), true),
             QStringLiteral("dark"));
    QCOMPARE(resolveCompositorColorScheme(QString(), false), QStringLiteral("light"));
}

void TestCompositorPolicy::liveFlipChangesThePolicy()
{
    // The same path a settingsd `Changed` takes: re-derive from the client's
    // values and the forward only fires when the struct differs.
    QVariantMap values;
    values.insert(QStringLiteral("appearance.colorScheme"), QStringLiteral("dark"));
    const CompositorPolicy before = compositorPolicyFromValues(values, false);

    values.insert(QStringLiteral("appearance.colorScheme"), QStringLiteral("light"));
    const CompositorPolicy after = compositorPolicyFromValues(values, false);
    QVERIFY(before != after);

    // Re-deriving an unchanged map is a no-op (`operator==`), so the shell
    // does not re-send the protocol request.
    QVERIFY(after == compositorPolicyFromValues(values, false));
}

QTEST_MAIN(TestCompositorPolicy)
#include "tst_compositorpolicy.moc"