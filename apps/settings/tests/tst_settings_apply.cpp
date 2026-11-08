// SPDX-License-Identifier: MIT
// Settings common apply path tests (T224): client-side range/enum validation,
// optimistic apply with revert on an owner failure, service disappearance, and
// restart resync (including a write made while the daemon was away).
//
// Run under a private `dbus-run-session` (see CMakeLists.txt). The fake
// `org.dragonfruit.Settings1` owner uses the real wire signatures; the live
// settingsd binary is not required.
#include "settingsclient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusContext>
#include <QDBusVariant>
#include <QSignalSpy>
#include <QTest>
#include <QVariantMap>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.Settings1");
const QString kPath = QStringLiteral("/org/dragonfruit/Settings1");
const QString kInterface = QStringLiteral("org.dragonfruit.Settings1");

class FakeSettingsService : public QObject, protected QDBusContext
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Settings1")

public:
    explicit FakeSettingsService(QObject *parent = nullptr)
        : QObject(parent)
        , m_values(settingsSchemaDefaults())
    {
    }

    QVariant value(const QString &key) const { return m_values.value(key); }
    void setValue(const QString &key, const QVariant &value)
    {
        m_values.insert(key, value);
        emit Changed(key, QDBusVariant(value));
    }
    void failNextSet(const QString &message)
    {
        m_failNext = true;
        m_failMessage = message;
    }
    int setCount() const { return m_setCount; }

public slots:
    QDBusVariant Get(const QString &key) const { return QDBusVariant(m_values.value(key)); }

    void Set(const QString &key, const QDBusVariant &value)
    {
        if (m_failNext) {
            m_failNext = false;
            sendErrorReply(QStringLiteral("org.dragonfruit.Settings1.Error.Failed"),
                           m_failMessage);
            return;
        }
        ++m_setCount;
        setValue(key, value.variant());
    }

    QVariantMap GetAll() const { return m_values; }
    QStringList ListKeys() const { return m_values.keys(); }

signals:
    void Changed(const QString &key, const QDBusVariant &value);

private:
    QVariantMap m_values;
    bool m_failNext = false;
    QString m_failMessage;
    int m_setCount = 0;
};

bool registerService(QDBusConnection &bus, FakeSettingsService &service)
{
    if (!bus.registerService(kService))
        return false;
    return bus.registerObject(kPath, &service,
                              QDBusConnection::ExportAllSlots
                                  | QDBusConnection::ExportAllSignals);
}

void unregisterService(QDBusConnection &bus)
{
    bus.unregisterObject(kPath);
    bus.unregisterService(kService);
}

} // namespace

class TestSettingsApply : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
    }

    void validationRejectsRangesEnumsTypesAndUnknownKeys()
    {
        // No daemon: the client still validates against the schema before it
        // would touch the in-memory store.
        DbusSettingsClient client;
        QSignalSpy finished(&client, &SettingsClient::writeFinished);

        QCOMPARE(client.value("dock.size").toDouble(), 0.5);
        QVERIFY(!client.set("dock.size", 9.0));
        QCOMPARE(client.value("dock.size").toDouble(), 0.5);
        QCOMPARE(client.writeState("dock.size"), QStringLiteral("error"));
        QVERIFY(client.writeError("dock.size").contains("outside"));

        // A second rejection of the same key must replace the message even
        // though the state is already `error`.
        QSignalSpy stateChanges(&client, &SettingsClient::writeStateChanged);
        QVERIFY(!client.set("dock.size", -1.0));
        QVERIFY(client.writeError("dock.size").contains("-1"));
        QCOMPARE(stateChanges.count(), 1);
        QCOMPARE(client.writeState("dock.size"), QStringLiteral("error"));

        QVERIFY(!client.set("appearance.colorScheme", "purple"));
        QCOMPARE(client.value("appearance.colorScheme").toString(), QStringLiteral("auto"));
        QVERIFY(client.writeError("appearance.colorScheme").contains("not one of"));

        QVERIFY(!client.set("dock.autohide", "yes"));
        QVERIFY(client.writeError("dock.autohide").contains("expected bool"));

        QVERIFY(!client.set("nope.key", true));
        QVERIFY(client.writeError("nope.key").contains("unknown"));

        // A one-sided range reports the bound it violated.
        QVERIFY(!client.set("wallpaper.providerLastFetch", -1));
        QVERIFY(client.writeError("wallpaper.providerLastFetch").contains("below"));

        // A valid write on the absent-owner path is accepted in memory and
        // marked deferred, not failed.
        QVERIFY(client.set("dock.size", 0.8));
        QCOMPARE(client.value("dock.size").toDouble(), 0.8);
        QCOMPARE(client.writeState("dock.size"), QStringLiteral("deferred"));
        QCOMPARE(finished.count(), 7); // six rejections + the accepted write
        QVERIFY(finished.at(6).at(2).toBool());
    }

    void ownerFailureRevertsAndReportsTheMessage()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        FakeSettingsService service;
        QVERIFY(registerService(bus, service));

        DbusSettingsClient client;
        QTRY_VERIFY_WITH_TIMEOUT(client.isAvailable(), 5000);
        QTRY_COMPARE_WITH_TIMEOUT(client.value("dock.autohide").toBool(), false, 5000);

        QVERIFY(client.set("dock.autohide", true));
        QTRY_VERIFY_WITH_TIMEOUT(service.value("dock.autohide").toBool(), 5000);
        QTRY_COMPARE_WITH_TIMEOUT(client.writeState("dock.autohide"),
                                  QStringLiteral("idle"), 5000);

        // The owner rejects the next write. The optimistic value appears
        // immediately, then the client reverts and records the reason.
        service.failNextSet(QStringLiteral("settingsd: disk is read-only"));
        QSignalSpy finished(&client, &SettingsClient::writeFinished);
        QVERIFY(client.set("dock.autohide", false));
        QCOMPARE(client.value("dock.autohide").toBool(), false); // optimistic
        QTRY_COMPARE_WITH_TIMEOUT(finished.count(), 1, 5000);
        QCOMPARE(finished.at(0).at(2).toBool(), false);
        QCOMPARE(client.value("dock.autohide").toBool(), true); // reverted
        QCOMPARE(client.writeState("dock.autohide"), QStringLiteral("error"));
        QCOMPARE(client.writeError("dock.autohide"),
                 QStringLiteral("settingsd: disk is read-only"));

        // An owner change clears the stale error.
        service.setValue("dock.autohide", false);
        QTRY_COMPARE_WITH_TIMEOUT(client.value("dock.autohide").toBool(), false, 5000);
        QTRY_COMPARE_WITH_TIMEOUT(client.writeError("dock.autohide"), QString(), 5000);

        unregisterService(bus);
    }

    void serviceDisappearanceAndRestartResyncKeepTheLastValues()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        FakeSettingsService service;
        QVERIFY(registerService(bus, service));

        DbusSettingsClient client;
        QTRY_VERIFY_WITH_TIMEOUT(client.isAvailable(), 5000);
        QTRY_COMPARE_WITH_TIMEOUT(client.value("workspaces.count").toLongLong(), 3, 5000);

        // An external write reaches the client through `Changed`.
        service.setValue("workspaces.count", 5);
        QTRY_COMPARE_WITH_TIMEOUT(client.value("workspaces.count").toLongLong(), 5, 5000);

        // The daemon goes away. The last value is kept so the desktop does not
        // lose a setting visually; `available` is the absence signal.
        unregisterService(bus);
        QTRY_VERIFY_WITH_TIMEOUT(!client.isAvailable(), 5000);

        // A write made while it is away is applied in memory for this session
        // only; it is explicitly deferred, not a durable success.
        QVERIFY(client.set("workspaces.count", 7));
        QCOMPARE(client.value("workspaces.count").toLongLong(), 7);
        QCOMPARE(client.writeState("workspaces.count"), QStringLiteral("deferred"));

        // The daemon returns with its persisted value. GetAll is authoritative
        // (ADR 0032), so the while-down write is reconciled away and its
        // deferred state clears rather than leaving the UI claiming success.
        QVERIFY(registerService(bus, service));
        QTRY_VERIFY_WITH_TIMEOUT(client.isAvailable(), 5000);
        QTRY_COMPARE_WITH_TIMEOUT(client.value("workspaces.count").toLongLong(), 5, 5000);
        QCOMPARE(service.value("workspaces.count").toLongLong(), 5);
        QCOMPARE(client.writeState("workspaces.count"), QStringLiteral("idle"));
        QCOMPARE(client.writeError("workspaces.count"), QString());

        // A restart with a changed persisted value resyncs to the daemon's
        // state.
        unregisterService(bus);
        QTRY_VERIFY_WITH_TIMEOUT(!client.isAvailable(), 5000);
        service.setValue("workspaces.count", 9);
        QVERIFY(registerService(bus, service));
        QTRY_VERIFY_WITH_TIMEOUT(client.isAvailable(), 5000);
        QTRY_COMPARE_WITH_TIMEOUT(client.value("workspaces.count").toLongLong(), 9, 5000);

        unregisterService(bus);
    }
};

QTEST_MAIN(TestSettingsApply)
#include "tst_settings_apply.moc"
