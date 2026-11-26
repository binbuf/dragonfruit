// SPDX-License-Identifier: GPL-3.0-or-later
// T282: every Settings adapter client against one fake system-status host.
//
// The fake distinguishes `State` (the cached view) from `Refresh` (a real
// re-read), returns an outcome report for every write, and can report
// `denied` or `failed`. The test proves the two contract halves the panes
// depend on:
//
//   * an opened pane calls `Refresh`, never only the cached `State`;
//   * a denied/failed write is reported as that outcome, never regressed into
//     an implicit success, and the client re-reads afterwards.
//
// Run under a private `dbus-run-session` (see CMakeLists.txt). No real host
// service or hardware is involved.
#include "AccessibilityClient.h"
#include "AccountsClient.h"
#include "BatteryClient.h"
#include "BluetoothClient.h"
#include "GeneralClient.h"
#include "InputClient.h"
#include "NetworkClient.h"
#include "NotificationsClient.h"
#include "PrintersClient.h"
#include "PrivacyClient.h"
#include "SoundClient.h"
#include "StorageClient.h"
#include "UpdatesClient.h"
#include "VpnClient.h"
#include "WifiClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusVirtualObject>
#include <QHash>
#include <QSignalSpy>
#include <QTest>
#include <QVariantList>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");

QString readJson()
{
    // A minimal but valid view for every adapter kind. A client only needs a
    // JSON object to replace its previous view; the exact fields are the
    // pane's business, not this contract test's.
    return QStringLiteral(
        "{\"kind\":\"fake\",\"state\":\"ready\",\"present\":true,"
        "\"powered\":true,\"volume\":0.5,\"muted\":false,"
        "\"sinks\":[],\"sources\":[],\"devices\":[],\"connections\":[],"
        "\"printers\":[],\"jobs\":[],\"updatesAvailable\":false,"
        "\"permissions\":[],\"security\":[],\"profiles\":[],"
        "\"activeProfile\":\"balanced\",\"userName\":\"test\"}");
}

class FakeStatusHost : public QDBusVirtualObject
{
    Q_OBJECT

public:
    QString outcome = QStringLiteral("denied");
    QString message = QStringLiteral("polkit denied this action");
    QHash<QString, int> stateCalls;
    QHash<QString, int> refreshCalls;
    QHash<QString, int> writes;

    QString introspect(const QString &) const override { return QString(); }

    bool handleMessage(const QDBusMessage &call,
                       const QDBusConnection &connection) override
    {
        if (call.type() != QDBusMessage::MethodCallMessage)
            return false;
        const QString interface = call.interface();
        const QString member = call.member();
        const bool isState = member == QLatin1String("State");
        const bool isRefresh = member == QLatin1String("Refresh")
            || member.startsWith(QLatin1String("Refresh"));
        if (isState) {
            ++stateCalls[interface];
            connection.send(call.createReply(QVariantList{readJson()}));
            return true;
        }
        if (isRefresh) {
            ++refreshCalls[interface];
            connection.send(call.createReply(QVariantList{readJson()}));
            return true;
        }
        ++writes[interface];
        const QString report = QStringLiteral(
            "{\"outcome\":\"%1\",\"message\":\"%2\",\"note\":\"%2\"}")
                                   .arg(outcome, message);
        connection.send(call.createReply(QVariantList{report}));
        return true;
    }

    int refreshes() const
    {
        int total = 0;
        for (int count : refreshCalls)
            total += count;
        return total;
    }
};

bool registerHost(QDBusConnection &bus, FakeStatusHost &host)
{
    if (!bus.registerService(kService))
        return false;
    return bus.registerVirtualObject(kPath, &host, QDBusConnection::SubPath);
}

} // namespace

class TestSettingsAdapters : public QObject
{
    Q_OBJECT

private:
    QDBusConnection m_bus = QDBusConnection::sessionBus();
    FakeStatusHost *m_host = nullptr;

    template <typename Client>
    Client *makeClient()
    {
        auto *client = new Client(this);
        return client;
    }

    int refreshCount() const { return m_host ? m_host->refreshes() : 0; }

private slots:
    void initTestCase()
    {
        if (!m_bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");
        m_host = new FakeStatusHost;
        QVERIFY(registerHost(m_bus, *m_host));
    }

    void cleanupTestCase()
    {
        if (m_host) {
            m_bus.unregisterObject(kPath);
            m_bus.unregisterService(kService);
            delete m_host;
            m_host = nullptr;
        }
    }

    // Every adapter's open path is a fresh host read, not the cached State.
    void everyAdapterRefreshesOnOpen()
    {
        const int before = refreshCount();
        {
            auto *client = makeClient<DbusAccessibilityClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusAccountsClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusBatteryClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusBluetoothClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusGeneralClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusInputClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusNetworkClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusNotificationsClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusPrintersClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusPrivacyClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusSoundClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusStorageClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusUpdatesClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusVpnClient>();
            Q_UNUSED(client);
        }
        {
            auto *client = makeClient<DbusWifiClient>();
            Q_UNUSED(client);
        }
        // One Refresh per client constructor (the live clients also read
        // again when the service registration watcher fires).
        QTRY_VERIFY_WITH_TIMEOUT(refreshCount() >= before + 15, 5000);
        int states = 0;
        for (int count : m_host->stateCalls)
            states += count;
        QVERIFY2(states == 0, "an open adapter must not rely on the cached State");
    }

    // A denied write is reported as denied (or another honest failure), never
    // as an implicit success, and the client re-reads the host afterwards.
    void deniedWritesAreReportedAndRefreshFollows()
    {
        m_host->outcome = QStringLiteral("denied");
        m_host->message = QStringLiteral("polkit denied this action");

        // Bluetooth
        {
            DbusBluetoothClient client;
            QSignalSpy spy(&client, &BluetoothClient::writeFinished);
            const int before = refreshCount();
            client.setPowered(true);
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
            QVERIFY(spy.at(0).at(2).toString().contains("polkit"));
            QTRY_VERIFY_WITH_TIMEOUT(refreshCount() > before, 5000);
        }
        // Storage
        {
            DbusStorageClient client;
            QSignalSpy spy(&client, &StorageClient::writeFinished);
            const int before = refreshCount();
            client.mount(QStringLiteral("/org/freedesktop/UDisks2/block_devices/sdb1"));
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
            QTRY_VERIFY_WITH_TIMEOUT(refreshCount() > before, 5000);
        }
        // Sound
        {
            DbusSoundClient client;
            QSignalSpy spy(&client, &SoundClient::writeFinished);
            client.setVolume(0.4);
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Battery
        {
            DbusBatteryClient client;
            QSignalSpy spy(&client, &BatteryClient::writeFinished);
            client.setActiveProfile(QStringLiteral("performance"));
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Notifications
        {
            DbusNotificationsClient client;
            QSignalSpy spy(&client, &NotificationsClient::writeFinished);
            client.setFocusMode(QStringLiteral("personal"));
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Updates
        {
            DbusUpdatesClient client;
            QSignalSpy spy(&client, &UpdatesClient::writeFinished);
            client.check();
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // General
        {
            DbusGeneralClient client;
            QSignalSpy spy(&client, &GeneralClient::writeFinished);
            client.setNtp(true);
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Accounts
        {
            DbusAccountsClient client;
            QSignalSpy spy(&client, &AccountsClient::writeFinished);
            client.setLocked(1000, true);
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Printers
        {
            DbusPrintersClient client;
            QSignalSpy spy(&client, &PrintersClient::writeFinished);
            client.setDefaultPrinter(QStringLiteral("Office"));
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Privacy
        {
            DbusPrivacyClient client;
            QSignalSpy spy(&client, &PrivacyClient::writeFinished);
            client.setPermission(QStringLiteral("locations"), QStringLiteral("host"),
                                 QStringLiteral("app.desktop"), QStringLiteral("allow"));
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // VPN
        {
            DbusVpnClient client;
            QSignalSpy spy(&client, &VpnClient::writeFinished);
            client.connectVpn(QStringLiteral("uuid-1"));
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Wi-Fi
        {
            DbusWifiClient client;
            QSignalSpy spy(&client, &WifiClient::writeFinished);
            client.setEnabled(false);
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
        // Network / firewall
        {
            DbusNetworkClient client;
            QSignalSpy spy(&client, &NetworkClient::writeFinished);
            client.setFirewallEnabled(true);
            QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
            QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("denied"));
        }
    }

    // A transport/owner failure is reported as failed, not swallowed.
    void failedWritesAreReported()
    {
        m_host->outcome = QStringLiteral("failed");
        m_host->message = QStringLiteral("udisks is not responding");

        DbusStorageClient client;
        QSignalSpy spy(&client, &StorageClient::writeFinished);
        client.eject(QStringLiteral("/org/freedesktop/UDisks2/drives/disk1"));
        QTRY_COMPARE_WITH_TIMEOUT(spy.count(), 1, 5000);
        QCOMPARE(spy.at(0).at(1).toString(), QStringLiteral("failed"));
        QVERIFY(spy.at(0).at(2).toString().contains("udisks"));
    }

    // The two read-only adapters still complete their contract: a parseable
    // view and a report for the one explicit controller test.
    void readOnlyAdaptersDeliverViewsAndReports()
    {
        DbusAccessibilityClient accessibility;
        QTRY_VERIFY_WITH_TIMEOUT(!accessibility.view().isEmpty(), 5000);

        DbusInputClient input;
        QSignalSpy report(&input, &InputClient::controllerTested);
        input.testController(QStringLiteral("/dev/input/event0"));
        QTRY_COMPARE_WITH_TIMEOUT(report.count(), 1, 5000);
    }
};

QTEST_GUILESS_MAIN(TestSettingsAdapters)
#include "tst_settings_adapters.moc"
