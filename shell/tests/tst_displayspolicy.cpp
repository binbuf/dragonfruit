// SPDX-License-Identifier: MIT
// T-09.5: the pure mapping from the settingsd display keys onto what the
// shell forwards over `df_output.set_scale` / `df_output.set_transform`
// (`set_brightness` added in T-11.3a). No bus, Wayland, or QML.
#include <QtTest>

#include "displayspolicy.h"

class TestDisplaysPolicy : public QObject
{
    Q_OBJECT

private slots:
    void defaultsMatchTheSchema();
    void everyKeyMapsThrough();
    void rotationNamesMapToTheWireEnum();
    void unknownRotationFallsBackToNormal();
    void liveFlipChangesTheSettings();
};

void TestDisplaysPolicy::defaultsMatchTheSchema()
{
    const DisplaySettings settings = displaySettingsFromValues({});
    QCOMPARE(settings.scale, 1.0);
    QCOMPARE(settings.rotation, QStringLiteral("normal"));
    QCOMPARE(settings.brightness, 1.0);
}

void TestDisplaysPolicy::everyKeyMapsThrough()
{
    QVariantMap values;
    values.insert(QStringLiteral("display.scale"), 1.25);
    values.insert(QStringLiteral("display.rotation"), QStringLiteral("270"));
    values.insert(QStringLiteral("display.brightness"), 0.35);

    const DisplaySettings settings = displaySettingsFromValues(values);
    QCOMPARE(settings.scale, 1.25);
    QCOMPARE(settings.rotation, QStringLiteral("270"));
    QCOMPARE(settings.brightness, 0.35);
}

void TestDisplaysPolicy::rotationNamesMapToTheWireEnum()
{
    // df_output.transform: normal=0, 90=1, 180=2, 270=3.
    QCOMPARE(outputTransformFromName(QStringLiteral("normal")), 0u);
    QCOMPARE(outputTransformFromName(QStringLiteral("90")), 1u);
    QCOMPARE(outputTransformFromName(QStringLiteral("180")), 2u);
    QCOMPARE(outputTransformFromName(QStringLiteral("270")), 3u);
}

void TestDisplaysPolicy::unknownRotationFallsBackToNormal()
{
    QCOMPARE(outputTransformFromName(QStringLiteral("45")), 0u);
    QCOMPARE(outputTransformFromName(QString()), 0u);
}

void TestDisplaysPolicy::liveFlipChangesTheSettings()
{
    // The same path a settingsd `Changed` takes: re-derive from the client's
    // values and the forward only fires when the struct differs.
    QVariantMap values;
    values.insert(QStringLiteral("display.scale"), 1.0);
    const DisplaySettings before = displaySettingsFromValues(values);

    values.insert(QStringLiteral("display.scale"), 1.5);
    const DisplaySettings after = displaySettingsFromValues(values);
    QVERIFY(before != after);

    // Re-deriving an unchanged map is a no-op (`operator==`), so the shell
    // does not re-send the protocol request.
    QVERIFY(after == displaySettingsFromValues(values));
}

QTEST_MAIN(TestDisplaysPolicy)
#include "tst_displayspolicy.moc"