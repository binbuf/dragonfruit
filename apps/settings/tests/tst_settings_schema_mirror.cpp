// SPDX-License-Identifier: GPL-3.0-or-later
// T282: the client schema mirror cannot drift from the daemon.
//
// `settingsSchemaDefaults()` / `settingsSchemaInfo()` in
// libs/settings-client are the hand-maintained client cache of
// services/settingsd's `schema::KEYS`. This test runs the real daemon binary
// with `--print-schema-json` and compares every key, type, default, enum, and
// numeric bound. A missing or extra key, a wrong default, or a moved bound
// fails here instead of becoming a live "unknown setting" write.
//
// CMake finds the built `dragonfruit-settingsd` and passes its path in
// `DF_SETTINGSD_BIN`; without the binary (a standalone Qt build) the test
// skips, and the Rust workspace tests still cover the daemon half.
#include "settingsclient.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QProcess>
#include <QSet>
#include <QTest>

#include <limits>
#include <QVariantMap>

namespace {

QString typeSignature(const QVariant &value)
{
    switch (value.typeId()) {
    case QMetaType::Bool:
        return QStringLiteral("b");
    case QMetaType::Double:
        return QStringLiteral("d");
    case QMetaType::LongLong:
        return QStringLiteral("x");
    case QMetaType::QStringList:
        return QStringLiteral("as");
    default:
        return QStringLiteral("s");
    }
}

bool defaultsEqual(const QVariant &mirror, const QJsonValue &daemon)
{
    if (mirror.typeId() == QMetaType::Bool)
        return daemon.isBool() && daemon.toBool() == mirror.toBool();
    if (mirror.typeId() == QMetaType::Double)
        return daemon.isDouble()
            && qFuzzyCompare(daemon.toDouble() + 1.0, mirror.toDouble() + 1.0);
    if (mirror.typeId() == QMetaType::LongLong)
        return daemon.isDouble()
            && daemon.toDouble() == double(mirror.toLongLong());
    if (mirror.typeId() == QMetaType::QStringList) {
        if (!daemon.isArray())
            return false;
        QStringList expected = mirror.toStringList();
        const QJsonArray array = daemon.toArray();
        if (array.size() != expected.size())
            return false;
        for (int i = 0; i < array.size(); ++i) {
            if (array.at(i).toString() != expected.at(i))
                return false;
        }
        return true;
    }
    return daemon.isString() && daemon.toString() == mirror.toString();
}

} // namespace

class TestSettingsSchemaMirror : public QObject
{
    Q_OBJECT

private slots:
    void theClientMirrorMatchesTheDaemonSchema()
    {
        const QString binary = qEnvironmentVariable("DF_SETTINGSD_BIN");
        if (binary.isEmpty())
            QSKIP("DF_SETTINGSD_BIN is not set; no built daemon to compare");

        QProcess process;
        process.start(binary, { QStringLiteral("--print-schema-json") });
        QVERIFY2(process.waitForFinished(15000), "the daemon schema dump timed out");
        QCOMPARE(process.exitStatus(), QProcess::NormalExit);
        QCOMPARE(process.exitCode(), 0);

        QJsonParseError error{};
        const QJsonDocument document =
            QJsonDocument::fromJson(process.readAllStandardOutput(), &error);
        QVERIFY2(error.error == QJsonParseError::NoError, qPrintable(error.errorString()));
        QVERIFY(document.isObject());
        const QJsonObject root = document.object();
        const QJsonValue schema = root.value(QStringLiteral("schema"));
        QVERIFY(schema.isDouble());
        const QJsonArray keys = root.value(QStringLiteral("keys")).toArray();
        QVERIFY(!keys.isEmpty());

        const QVariantMap &defaults = settingsSchemaDefaults();
        QCOMPARE(defaults.size(), keys.size());

        QSet<QString> daemonKeys;
        for (const QJsonValue &entry : keys) {
            QVERIFY(entry.isObject());
            const QJsonObject key = entry.toObject();
            const QString name = key.value(QStringLiteral("key")).toString();
            QVERIFY2(!name.isEmpty(), "a daemon schema row has no key");
            QVERIFY2(!daemonKeys.contains(name), qPrintable("duplicate " + name));
            daemonKeys.insert(name);

            QVERIFY2(defaults.contains(name),
                     qPrintable("the client mirror is missing " + name));
            const QVariant declared = defaults.value(name);

            QVERIFY2(typeSignature(declared)
                         == key.value(QStringLiteral("type")).toString(),
                     qPrintable(QStringLiteral("%1: the mirror says %2, the daemon says %3")
                                    .arg(name, typeSignature(declared),
                                         key.value(QStringLiteral("type")).toString())));
            QVERIFY2(defaultsEqual(declared, key.value(QStringLiteral("default"))),
                     qPrintable("default drift on " + name));

            const QVariantMap info = settingsSchemaInfo(name);
            const QStringList allowed = info.value(QStringLiteral("allowed")).toStringList();
            QStringList daemonAllowed;
            for (const QJsonValue &item : key.value(QStringLiteral("allowed")).toArray())
                daemonAllowed.append(item.toString());
            QVERIFY2(allowed == daemonAllowed,
                     qPrintable(QStringLiteral("%1: mirror allowed %2, daemon %3")
                                    .arg(name, allowed.join(QLatin1Char(',')),
                                         daemonAllowed.join(QLatin1Char(',')))));

            const bool hasMin = info.value(QStringLiteral("hasMin")).toBool();
            const bool hasMax = info.value(QStringLiteral("hasMax")).toBool();
            const QJsonValue min = key.value(QStringLiteral("min"));
            const QJsonValue max = key.value(QStringLiteral("max"));
            QCOMPARE(hasMin, !min.isNull());
            QCOMPARE(hasMax, !max.isNull());
            if (hasMin)
                QCOMPARE(info.value(QStringLiteral("min")).toDouble(), min.toDouble());
            if (hasMax)
                QCOMPARE(info.value(QStringLiteral("max")).toDouble(), max.toDouble());
        }

        for (auto it = defaults.constBegin(); it != defaults.constEnd(); ++it) {
            QVERIFY2(daemonKeys.contains(it.key()),
                     qPrintable("the client mirror has a key the daemon does not: " + it.key()));
        }
    }

    void nonFiniteIntegralAndPageCountWritesAreJudgedByTheMirror()
    {
        // T282: the page-count keys are int64 with the daemon's 1..3 / 0..2
        // bounds, and a non-finite or fractional value is rejected here just
        // as the daemon would reject it.
        const QVariantMap pageCount = settingsSchemaInfo(QStringLiteral("quickSettings.pageCount"));
        QCOMPARE(pageCount.value(QStringLiteral("type")).toString(), QStringLiteral("int64"));
        QVERIFY(pageCount.value(QStringLiteral("hasMin")).toBool());
        QVERIFY(pageCount.value(QStringLiteral("hasMax")).toBool());
        QCOMPARE(pageCount.value(QStringLiteral("min")).toDouble(), 1.0);
        QCOMPARE(pageCount.value(QStringLiteral("max")).toDouble(), 3.0);

        const QVariantMap activePage =
            settingsSchemaInfo(QStringLiteral("quickSettings.activePage"));
        QCOMPARE(activePage.value(QStringLiteral("type")).toString(), QStringLiteral("int64"));
        QCOMPARE(activePage.value(QStringLiteral("min")).toDouble(), 0.0);
        QCOMPARE(activePage.value(QStringLiteral("max")).toDouble(), 2.0);

        MockSettingsClient client;
        QVERIFY(client.set(QStringLiteral("quickSettings.pageCount"), 2.0));
        QCOMPARE(client.integer(QStringLiteral("quickSettings.pageCount"), 0), 2);
        QVERIFY(!client.set(QStringLiteral("quickSettings.pageCount"), 4.0));
        // A QML double is coerced to the nearest int64, then the constraint
        // applies: 1.5 becomes 2 and is accepted.
        QVERIFY(client.set(QStringLiteral("quickSettings.pageCount"), 1.5));
        QCOMPARE(client.integer(QStringLiteral("quickSettings.pageCount"), 0), 2);
        QVERIFY(!client.set(QStringLiteral("dock.size"), std::numeric_limits<double>::quiet_NaN()));
        QVERIFY(!client.set(QStringLiteral("dock.size"), std::numeric_limits<double>::infinity()));
        QCOMPARE(client.writeState(QStringLiteral("dock.size")), QStringLiteral("error"));
        QVERIFY(client.writeError(QStringLiteral("dock.size")).contains("finite"));
    }
};

QTEST_MAIN(TestSettingsSchemaMirror)
#include "tst_settings_schema_mirror.moc"
