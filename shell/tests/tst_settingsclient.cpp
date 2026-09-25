// SPDX-License-Identifier: MIT
// Shell settings-client tests (T-08.2a): the typed value store, the
// `Changed` de-duplication, and the live `DbusSettingsClient` against a fake
// `org.dragonfruit.Settings1` service on the private session bus that
// `dbus-run-session` provides. When no bus is available the live half skips
// (the mock half still runs), so the test is safe on a bus-less host.
#include "settingsclient.h"

#include "dockmodel.h"

#include <QDBusConnection>
#include <QDBusVariant>
#include <QSignalSpy>
#include <QTest>
#include <QVariantMap>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.Settings1");
const QString kPath = QStringLiteral("/org/dragonfruit/Settings1");

// A minimal stand-in for settingsd's `org.dragonfruit.Settings1` object, with
// the same method/signal signatures the real daemon exports.
class FakeSettingsService : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Settings1")
    Q_PROPERTY(uint SchemaVersion READ schemaVersion CONSTANT)

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

public slots:
    QDBusVariant Get(const QString &key) const
    {
        return QDBusVariant(m_values.value(key));
    }

    void Set(const QString &key, const QDBusVariant &value)
    {
        setValue(key, value.variant());
    }

    QVariantMap GetAll() const { return m_values; }

    QStringList ListKeys() const { return m_values.keys(); }

signals:
    void Changed(const QString &key, const QDBusVariant &value);

private:
    uint schemaVersion() const { return 1; }
    QVariantMap m_values;
};

} // namespace

class TestSettingsClient : public QObject
{
    Q_OBJECT

private slots:
    void mockSeedsSchemaDefaultsAndEmitsOnRealChange()
    {
        MockSettingsClient client;
        QVERIFY(client.isAvailable());
        QCOMPARE(client.real(QStringLiteral("dock.size"), -1.0), 0.5);
        QCOMPARE(client.real(QStringLiteral("dock.magnification"), -1.0), 0.5);
        QCOMPARE(client.string(QStringLiteral("dock.position"), QString()),
                 QStringLiteral("bottom"));
        QCOMPARE(client.boolean(QStringLiteral("dock.autohide"), true), false);
        QCOMPARE(client.stringList(QStringLiteral("dock.pinned")), QStringList());

        QSignalSpy spy(&client, &SettingsClient::changed);
        client.set(QStringLiteral("dock.size"), 0.9);
        QCOMPARE(spy.count(), 1);
        QCOMPARE(client.real(QStringLiteral("dock.size"), -1.0), 0.9);
        // A repeat write is a silent no-op (no double reaction).
        client.set(QStringLiteral("dock.size"), 0.9);
        QCOMPARE(spy.count(), 1);

        // The pinned list round-trips as the `as` shape.
        client.set(QStringLiteral("dock.pinned"),
                   QStringList{QStringLiteral("a.desktop"), QStringLiteral("b.desktop")});
        QCOMPARE(client.stringList(QStringLiteral("dock.pinned")),
                 (QStringList{QStringLiteral("a.desktop"), QStringLiteral("b.desktop")}));

        QSignalSpy refreshed(&client, &SettingsClient::refreshed);
        client.refresh();
        QCOMPARE(refreshed.count(), 1);
    }

    void dockKeysChangeViaSettingsAndReLayOut()
    {
        // The acceptance path, headless: a key flipped through the client (as
        // settingsd would) becomes visible and re-lays-out the Dock.
        MockSettingsClient client;
        const int beforeIcon =
            dockIconSize(client.real(QStringLiteral("dock.size"), 0.5), 32, 64);
        QSignalSpy spy(&client, &SettingsClient::changed);
        client.set(QStringLiteral("dock.size"), 1.0);
        QCOMPARE(spy.count(), 1);
        QCOMPARE(spy.at(0).at(0).toString(), QStringLiteral("dock.size"));
        const int afterIcon =
            dockIconSize(client.real(QStringLiteral("dock.size"), 0.5), 32, 64);
        QVERIFY(afterIcon > beforeIcon);
    }

    void dbusClientRoundTripsAgainstTheService()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakeSettingsService service;
        QVERIFY(bus.registerService(kService));
        QVERIFY(bus.registerObject(kPath, &service,
                                   QDBusConnection::ExportAllSlots
                                       | QDBusConnection::ExportAllSignals
                                       | QDBusConnection::ExportAllProperties));

        DbusSettingsClient client;
        QVERIFY(client.isAvailable());

        // The constructor's GetAll resync lands the service's snapshot.
        QSignalSpy refreshed(&client, &SettingsClient::refreshed);
        QTRY_VERIFY(refreshed.count() >= 1);
        QCOMPARE(client.real(QStringLiteral("dock.size"), -1.0), 0.5);
        QCOMPARE(client.string(QStringLiteral("dock.position"), QString()),
                 QStringLiteral("bottom"));

        // A daemon-side change arrives as a `Changed` signal, not a poll.
        service.setValue(QStringLiteral("dock.size"), 0.9);
        QTRY_COMPARE(client.real(QStringLiteral("dock.size"), -1.0), 0.9);

        // A client write is mirrored to the service, which persists it and
        // echoes `Changed`; the echo is de-duplicated.
        client.set(QStringLiteral("dock.autohide"), true);
        QTRY_VERIFY(service.value(QStringLiteral("dock.autohide")).toBool());
        QCOMPARE(client.boolean(QStringLiteral("dock.autohide"), false), true);

        bus.unregisterObject(kPath);
        bus.unregisterService(kService);
    }

    void dbusClientResyncsWhenTheServiceRestarts()
    {
        // T-08.3: kill/restart resync. The client subscribes to the
        // well-known name; when the daemon disappears it keeps its last-known
        // values (nothing is visibly lost), and when the name reappears it
        // re-reads the daemon's snapshot (`GetAll`). A write attempted while
        // the daemon was down is in-memory only, so the re-sync reconciles it
        // away instead of silently keeping an unpersisted change.
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        // The fake daemon lives on a *separate* connection to the same bus,
        // which makes it remote from the client's `sessionBus()` exactly like
        // a real settingsd process. This matters: `QDBusConnectionInterface`'s
        // serviceRegistered/serviceUnregistered only fire for unique names, so
        // a client built on those signals resynced only for an in-process
        // service and silently stopped for a real restart. T-08.3 fixed that
        // with a `QDBusServiceWatcher`; this test guards the fix.
        const QString kServiceConnection = QStringLiteral("dragonfruit_test_settingsd");
        QDBusConnection serviceBus =
            QDBusConnection::connectToBus(QDBusConnection::SessionBus, kServiceConnection);
        QVERIFY(serviceBus.isConnected());

        FakeSettingsService service;
        QVERIFY(serviceBus.registerService(kService));
        QVERIFY(serviceBus.registerObject(kPath, &service,
                                          QDBusConnection::ExportAllSlots
                                              | QDBusConnection::ExportAllSignals
                                              | QDBusConnection::ExportAllProperties));

        DbusSettingsClient client;
        QVERIFY(client.isAvailable());
        QTRY_COMPARE(client.real(QStringLiteral("dock.size"), -1.0), 0.5);

        // A write through the client reaches the durable owner.
        client.set(QStringLiteral("dock.autohide"), true);
        QTRY_VERIFY(service.value(QStringLiteral("dock.autohide")).toBool());

        // kill -9: the name goes away; the desktop keeps the last-known state.
        QVERIFY(serviceBus.unregisterService(kService));
        QTRY_VERIFY(!client.isAvailable());
        QCOMPARE(client.boolean(QStringLiteral("dock.autohide"), false), true);

        // A change made while the daemon is down cannot be persisted; it is
        // only the optimistic in-memory value.
        client.set(QStringLiteral("dock.size"), 0.9);
        QCOMPARE(client.real(QStringLiteral("dock.size"), -1.0), 0.9);

        // The daemon restarts under the same name; its snapshot is authoritative.
        QSignalSpy refreshed(&client, &SettingsClient::refreshed);
        QVERIFY(serviceBus.registerService(kService));
        QTRY_VERIFY(refreshed.count() >= 1);
        QTRY_VERIFY(client.isAvailable());
        // The while-down write is reconciled away; the persisted write survives.
        QTRY_COMPARE(client.real(QStringLiteral("dock.size"), -1.0), 0.5);
        QCOMPARE(client.boolean(QStringLiteral("dock.autohide"), false), true);

        serviceBus.unregisterObject(kPath);
        serviceBus.unregisterService(kService);
        QDBusConnection::disconnectFromBus(kServiceConnection);
    }
};

QTEST_GUILESS_MAIN(TestSettingsClient)
#include "tst_settingsclient.moc"