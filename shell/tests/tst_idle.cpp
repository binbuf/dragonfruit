// SPDX-License-Identifier: GPL-3.0-only
// T317: the pure settingsd -> compositor idle policy mapping and the
// stage-to-effect controller. A mock effects sink stands in for the protocol
// and logind; no bus, Wayland, or QML.
#include <QtTest>

#include "idlepolicy.h"

namespace {

struct Recorded {
    QList<IdleCompositorPolicy> policies;
    QList<bool> dimmed;
    QList<bool> dpms;
    int locks = 0;
    int suspends = 0;
    bool dpmsState = true;
};

IdleEffects recordingEffects(Recorded *record)
{
    IdleEffects effects;
    effects.setPolicy = [record](const IdleCompositorPolicy &policy) {
        record->policies.append(policy);
    };
    effects.setDimmed = [record](bool dimmed) { record->dimmed.append(dimmed); };
    effects.dpmsOn = [record]() { return record->dpmsState; };
    effects.setDpms = [record](bool on) {
        record->dpms.append(on);
        record->dpmsState = on;
    };
    effects.lock = [record]() { ++record->locks; };
    effects.suspend = [record]() { ++record->suspends; };
    return effects;
}

} // namespace

class TestIdle : public QObject
{
    Q_OBJECT

private slots:
    void defaultsMatchTheSchema();
    void everyKeyMapsThrough();
    void delaysAreClamped();
    void preventSleepDisablesTheSuspendStage();
    void firstSnapshotAlwaysForwards();
    void unchangedSnapshotDoesNotResend();
    void changedPolicyIsForwarded();
    void livePreventSleepChangeDisablesSuspend();
    void stageSequenceDimsBlanksLocksSuspendsThenWakes();
    void awakeRestoresThePreBlankDpmsState();
    void disabledStagesDoNothing();
    void repeatedStagesAreIdempotent();
    void unknownActionIsIgnored();
    void suspendIsIgnoredWhilePreventSleepIsSet();
};

void TestIdle::defaultsMatchTheSchema()
{
    const IdleSettings settings = idleSettingsFromValues({});
    QCOMPARE(settings.dim, 150);
    QCOMPARE(settings.blank, 300);
    QCOMPARE(settings.lock, 600);
    QCOMPARE(settings.suspend, 0);
    QCOMPARE(settings.preventSleep, false);
    QCOMPARE(idleCompositorPolicy(settings).suspend, 0);
}

void TestIdle::everyKeyMapsThrough()
{
    QVariantMap values;
    values.insert(QStringLiteral("idle.dim"), 30);
    values.insert(QStringLiteral("idle.blank"), 60);
    values.insert(QStringLiteral("idle.lock"), 120);
    values.insert(QStringLiteral("idle.suspend"), 240);
    values.insert(QStringLiteral("power.preventSleep"), true);

    const IdleSettings settings = idleSettingsFromValues(values);
    QCOMPARE(settings.dim, 30);
    QCOMPARE(settings.blank, 60);
    QCOMPARE(settings.lock, 120);
    QCOMPARE(settings.suspend, 240);
    QCOMPARE(settings.preventSleep, true);

    const IdleCompositorPolicy policy = idleCompositorPolicy(settings);
    QCOMPARE(policy.dim, 30);
    QCOMPARE(policy.blank, 60);
    QCOMPARE(policy.lock, 120);
    QCOMPARE(policy.suspend, 0);
}

void TestIdle::delaysAreClamped()
{
    QVariantMap values;
    values.insert(QStringLiteral("idle.dim"), -5);
    values.insert(QStringLiteral("idle.blank"), 200000);
    const IdleSettings settings = idleSettingsFromValues(values);
    QCOMPARE(settings.dim, 0);
    QCOMPARE(settings.blank, 86400);

    // A non-numeric value falls back to the schema default rather than
    // silently disabling a stage.
    QVariantMap wrong;
    wrong.insert(QStringLiteral("idle.lock"), QStringLiteral("soon"));
    QCOMPARE(idleSettingsFromValues(wrong).lock, 600);
}

void TestIdle::preventSleepDisablesTheSuspendStage()
{
    IdleSettings settings;
    settings.suspend = 300;
    settings.preventSleep = true;
    QCOMPARE(idleCompositorPolicy(settings).suspend, 0);

    settings.preventSleep = false;
    QCOMPARE(idleCompositorPolicy(settings).suspend, 300);
}

void TestIdle::firstSnapshotAlwaysForwards()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    controller.setSettings(IdleSettings{});
    QVERIFY(controller.policySent());
    QCOMPARE(record.policies.size(), 1);
    QCOMPARE(record.policies.first().dim, 150);
    QCOMPARE(record.policies.first().blank, 300);
    QCOMPARE(record.policies.first().lock, 600);
    QCOMPARE(record.policies.first().suspend, 0);
}

void TestIdle::unchangedSnapshotDoesNotResend()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    IdleSettings settings;
    settings.dim = 20;
    settings.blank = 40;
    controller.setSettings(settings);
    controller.setSettings(settings);
    QCOMPARE(record.policies.size(), 1);
}

void TestIdle::changedPolicyIsForwarded()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    IdleSettings settings;
    settings.dim = 20;
    controller.setSettings(settings);

    settings.dim = 25;
    settings.lock = 90;
    controller.setSettings(settings);
    QCOMPARE(record.policies.size(), 2);
    QCOMPARE(record.policies.last().dim, 25);
    QCOMPARE(record.policies.last().lock, 90);
}

void TestIdle::livePreventSleepChangeDisablesSuspend()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    IdleSettings settings;
    settings.suspend = 120;
    controller.setSettings(settings);
    QCOMPARE(record.policies.last().suspend, 120);

    settings.preventSleep = true;
    controller.setSettings(settings);
    QCOMPARE(record.policies.size(), 2);
    QCOMPARE(record.policies.last().suspend, 0);
}

void TestIdle::stageSequenceDimsBlanksLocksSuspendsThenWakes()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    IdleSettings settings;
    settings.dim = 1;
    settings.blank = 2;
    settings.lock = 3;
    settings.suspend = 4;
    controller.setSettings(settings);

    controller.handleAction(QStringLiteral("idle-dim"));
    QVERIFY(controller.dimmed());
    controller.handleAction(QStringLiteral("idle-blank"));
    QVERIFY(controller.blanked());
    controller.handleAction(QStringLiteral("idle-lock"));
    controller.handleAction(QStringLiteral("idle-suspend"));

    QCOMPARE(record.dimmed, QList<bool>{true});
    QCOMPARE(record.dpms, QList<bool>{false});
    QCOMPARE(record.locks, 1);
    QCOMPARE(record.suspends, 1);

    controller.handleAction(QStringLiteral("idle-awake"));
    QVERIFY(!controller.dimmed());
    QVERIFY(!controller.blanked());
    QCOMPARE(record.dimmed, (QList<bool>{true, false}));
    QCOMPARE(record.dpms, (QList<bool>{false, true}));
}

void TestIdle::awakeRestoresThePreBlankDpmsState()
{
    Recorded record;
    // The panel was already off before the idle stage (a user-requested
    // blank); wake must not turn it on.
    record.dpmsState = false;
    IdleController controller(recordingEffects(&record));
    IdleSettings settings;
    settings.blank = 2;
    controller.setSettings(settings);

    controller.handleAction(QStringLiteral("idle-blank"));
    QCOMPARE(record.dpms, QList<bool>{false});
    controller.handleAction(QStringLiteral("idle-awake"));
    QCOMPARE(record.dpms, (QList<bool>{false, false}));
}

void TestIdle::disabledStagesDoNothing()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    IdleSettings settings;
    settings.dim = 0;
    settings.blank = 0;
    settings.lock = 0;
    settings.suspend = 0;
    controller.setSettings(settings);

    controller.handleAction(QStringLiteral("idle-dim"));
    controller.handleAction(QStringLiteral("idle-blank"));
    controller.handleAction(QStringLiteral("idle-lock"));
    controller.handleAction(QStringLiteral("idle-suspend"));
    controller.handleAction(QStringLiteral("idle-awake"));

    QVERIFY(record.dimmed.isEmpty());
    QVERIFY(record.dpms.isEmpty());
    QCOMPARE(record.locks, 0);
    QCOMPARE(record.suspends, 0);
}

void TestIdle::repeatedStagesAreIdempotent()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    controller.setSettings(IdleSettings{});

    controller.handleAction(QStringLiteral("idle-dim"));
    controller.handleAction(QStringLiteral("idle-dim"));
    controller.handleAction(QStringLiteral("idle-blank"));
    controller.handleAction(QStringLiteral("idle-blank"));
    QCOMPARE(record.dimmed, QList<bool>{true});
    QCOMPARE(record.dpms, QList<bool>{false});

    controller.handleAction(QStringLiteral("idle-awake"));
    controller.handleAction(QStringLiteral("idle-awake"));
    QCOMPARE(record.dimmed, (QList<bool>{true, false}));
    QCOMPARE(record.dpms, (QList<bool>{false, true}));
}

void TestIdle::unknownActionIsIgnored()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    controller.setSettings(IdleSettings{});
    controller.handleAction(QStringLiteral("lock-screen"));
    controller.handleAction(QString());
    QCOMPARE(record.locks, 0);
    QVERIFY(record.dimmed.isEmpty());
    QVERIFY(record.dpms.isEmpty());
    QCOMPARE(record.suspends, 0);
}

void TestIdle::suspendIsIgnoredWhilePreventSleepIsSet()
{
    Recorded record;
    IdleController controller(recordingEffects(&record));
    IdleSettings settings;
    settings.suspend = 10;
    settings.preventSleep = true;
    controller.setSettings(settings);
    controller.handleAction(QStringLiteral("idle-suspend"));
    QCOMPARE(record.suspends, 0);
}

QTEST_APPLESS_MAIN(TestIdle)
#include "tst_idle.moc"
