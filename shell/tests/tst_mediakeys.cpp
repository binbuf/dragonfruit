// SPDX-License-Identifier: GPL-3.0-only
// T321: the pure media-key step policy. The compositor routes the XF86 keys as
// input actions; this test fixes the 5% steps, the mute toggle, the clamping,
// and the absent-path no-op without a bus, Wayland, or QML.
#include <QtTest>

#include "mediakeys.h"

class TestMediaKeys : public QObject
{
    Q_OBJECT

private slots:
    void handlesOnlyTheFiveMediaKeys();
    void volumeStepsFivePercentPerPress();
    void volumeClampsAtBothEnds();
    void volumeStepsDoNotAccumulateFloatNoise();
    void volumeKeepsTheMuteState();
    void muteTogglesAndPreservesTheLevel();
    void brightnessStepsFivePercentPerPress();
    void brightnessClampsAtBothEnds();
    void absentAudioPathIsASilentNoOp();
    void absentBrightnessPathIsASilentNoOp();
    void unknownActionIsNotHandled();
};

void TestMediaKeys::handlesOnlyTheFiveMediaKeys()
{
    const QStringList actions = {
        QStringLiteral("volume-up"),   QStringLiteral("volume-down"),
        QStringLiteral("mute"),        QStringLiteral("brightness-up"),
        QStringLiteral("brightness-down"),
    };
    for (const QString &action : actions)
        QVERIFY2(MediaKeys::handles(action), qPrintable(action));
    QVERIFY(!MediaKeys::handles(QStringLiteral("workspace-next")));
    QVERIFY(!MediaKeys::handles(QStringLiteral("volume")));
    QVERIFY(!MediaKeys::handles(QString()));
}

void TestMediaKeys::volumeStepsFivePercentPerPress()
{
    const MediaKeys::Plan up =
        MediaKeys::plan(QStringLiteral("volume-up"), true, 0.5, false, false, 1.0);
    QCOMPARE(up.write, MediaKeys::Write::SetVolume);
    QCOMPARE(up.value, 0.55);
    QCOMPARE(up.muted, false);

    const MediaKeys::Plan down =
        MediaKeys::plan(QStringLiteral("volume-down"), true, 0.5, false, false, 1.0);
    QCOMPARE(down.write, MediaKeys::Write::SetVolume);
    QCOMPARE(down.value, 0.45);
}

void TestMediaKeys::volumeClampsAtBothEnds()
{
    QCOMPARE(MediaKeys::plan(QStringLiteral("volume-up"), true, 1.0, false, false, 1.0).value,
             1.0);
    QCOMPARE(MediaKeys::plan(QStringLiteral("volume-up"), true, 0.98, false, false, 1.0).value,
             1.0);
    QCOMPARE(MediaKeys::plan(QStringLiteral("volume-down"), true, 0.0, false, false, 1.0).value,
             0.0);
    QCOMPARE(MediaKeys::plan(QStringLiteral("volume-down"), true, 0.03, false, false, 1.0).value,
             0.0);
}

void TestMediaKeys::volumeStepsDoNotAccumulateFloatNoise()
{
    // Twenty 5% steps from zero land exactly on 1.0; the hundredth snap keeps
    // the OSD label and the written level stable.
    double level = 0.0;
    for (int i = 0; i < 20; ++i) {
        level = MediaKeys::plan(QStringLiteral("volume-up"), true, level, false, false, 1.0)
                    .value;
    }
    QCOMPARE(level, 1.0);
}

void TestMediaKeys::volumeKeepsTheMuteState()
{
    // The volume keys only change the level; mute stays the explicit key's
    // job. The OSD therefore reports the resulting (still muted) state.
    const MediaKeys::Plan plan =
        MediaKeys::plan(QStringLiteral("volume-up"), true, 0.4, true, false, 1.0);
    QCOMPARE(plan.write, MediaKeys::Write::SetVolume);
    QCOMPARE(plan.value, 0.45);
    QCOMPARE(plan.muted, true);
}

void TestMediaKeys::muteTogglesAndPreservesTheLevel()
{
    const MediaKeys::Plan muted =
        MediaKeys::plan(QStringLiteral("mute"), true, 0.35, false, false, 1.0);
    QCOMPARE(muted.write, MediaKeys::Write::SetMute);
    QCOMPARE(muted.value, 0.35);
    QCOMPARE(muted.muted, true);

    const MediaKeys::Plan unmuted =
        MediaKeys::plan(QStringLiteral("mute"), true, 0.35, true, false, 1.0);
    QCOMPARE(unmuted.write, MediaKeys::Write::SetMute);
    QCOMPARE(unmuted.value, 0.35);
    QCOMPARE(unmuted.muted, false);
}

void TestMediaKeys::brightnessStepsFivePercentPerPress()
{
    const MediaKeys::Plan up =
        MediaKeys::plan(QStringLiteral("brightness-up"), false, 0.0, false, true, 0.7);
    QCOMPARE(up.write, MediaKeys::Write::SetBrightness);
    QCOMPARE(up.value, 0.75);
    QCOMPARE(up.muted, false);

    const MediaKeys::Plan down =
        MediaKeys::plan(QStringLiteral("brightness-down"), false, 0.0, false, true, 0.7);
    QCOMPARE(down.write, MediaKeys::Write::SetBrightness);
    QCOMPARE(down.value, 0.65);
}

void TestMediaKeys::brightnessClampsAtBothEnds()
{
    QCOMPARE(MediaKeys::plan(QStringLiteral("brightness-up"), false, 0.0, false, true, 1.0)
                 .value,
             1.0);
    QCOMPARE(MediaKeys::plan(QStringLiteral("brightness-down"), false, 0.0, false, true, 0.0)
                 .value,
             0.0);
    QCOMPARE(MediaKeys::plan(QStringLiteral("brightness-down"), false, 0.0, false, true, 0.02)
                 .value,
             0.0);
}

void TestMediaKeys::absentAudioPathIsASilentNoOp()
{
    for (const QString &action :
         {QStringLiteral("volume-up"), QStringLiteral("volume-down"),
          QStringLiteral("mute")}) {
        const MediaKeys::Plan plan =
            MediaKeys::plan(action, false, 0.5, false, true, 0.5);
        QCOMPARE(plan.write, MediaKeys::Write::None);
    }
}

void TestMediaKeys::absentBrightnessPathIsASilentNoOp()
{
    for (const QString &action :
         {QStringLiteral("brightness-up"), QStringLiteral("brightness-down")}) {
        const MediaKeys::Plan plan =
            MediaKeys::plan(action, true, 0.5, false, false, 0.5);
        QCOMPARE(plan.write, MediaKeys::Write::None);
    }
}

void TestMediaKeys::unknownActionIsNotHandled()
{
    const MediaKeys::Plan plan =
        MediaKeys::plan(QStringLiteral("screenshot"), true, 0.5, false, true, 0.5);
    QCOMPARE(plan.write, MediaKeys::Write::None);
}

QTEST_GUILESS_MAIN(TestMediaKeys)
#include "tst_mediakeys.moc"
