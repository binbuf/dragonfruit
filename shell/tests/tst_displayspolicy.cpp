// SPDX-License-Identifier: MIT
// T-09.5: the pure mapping from the settingsd display keys onto what the
// shell forwards over `df_output.set_scale` / `df_output.set_transform`
// (`set_brightness` added in T-11.3a). No bus, Wayland, or QML.
#include <QtTest>
#include <QVariantMap>

#include "displaycapabilities.h"
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
    void advancedRowsOnlyAdvertiseSupportedControls();
    void advancedRowsCarryAnOpenReason();
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

void TestDisplaysPolicy::advancedRowsOnlyAdvertiseSupportedControls()
{
    const QList<DisplayAdvancedRow> rows = displayAdvancedRows();
    QCOMPARE(rows.size(), 8);
    QCOMPARE(rows[0].id, QStringLiteral("brightness"));
    QVERIFY(rows[0].supported);
    QCOMPARE(rows[0].control, QStringLiteral("slider"));
    QCOMPARE(rows[1].id, QStringLiteral("rotation"));
    QVERIFY(rows[1].supported);
    QCOMPARE(rows[1].control, QStringLiteral("popup"));

    // No other row is advertised as a live control.
    for (const DisplayAdvancedRow &row : rows) {
        if (row.id == QLatin1String("brightness") || row.id == QLatin1String("rotation"))
            continue;
        QVERIFY2(!row.supported, qPrintable(row.id));
        QCOMPARE(row.control, QStringLiteral("info"));
    }
}

void TestDisplaysPolicy::advancedRowsCarryAnOpenReason()
{
    const QVariantList rows = displayAdvancedRowsVariant();
    QCOMPARE(rows.size(), 8);
    for (const QVariant &entry : rows) {
        const QVariantMap row = entry.toMap();
        if (row.value(QStringLiteral("supported")).toBool()) {
            QVERIFY(row.value(QStringLiteral("reason")).toString().isEmpty());
            continue;
        }
        QVERIFY2(!row.value(QStringLiteral("reason")).toString().isEmpty(),
                 qPrintable(row.value(QStringLiteral("id")).toString()));
    }
}

QTEST_MAIN(TestDisplaysPolicy)
#include "tst_displayspolicy.moc"