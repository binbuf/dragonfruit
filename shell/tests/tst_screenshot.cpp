// SPDX-License-Identifier: GPL-3.0-or-later
// Screenshot selection bridge tests (T-13.3a): the pure request lifecycle (the
// mode, the selection hand-off, cancellation) and the live `ScreenshotBridge`
// against a fake `org.dragonfruit.Portal1` presenter surface on the private
// session bus that `dbus-run-session` provides. The D-Bus half skips when no
// bus is available, so the test is safe on a bus-less host.
#include "screenshotbridge.h"

#include "portalrow.h"

#include <QDBusConnection>
#include <QDBusMetaType>
#include <QSignalSpy>
#include <QTest>
#include <QVariantMap>

namespace {

const QString kService = QStringLiteral("org.freedesktop.impl.portal.desktop.dragonfruit");
const QString kPath = QStringLiteral("/org/freedesktop/portal/desktop");

// A minimal stand-in for the backend's presenter surface (T-13.3a): the two
// methods the overlay calls and the `ScreenshotOpened` signal it watches.
class FakePortal : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Portal1")

public:
    QString completedHandle() const { return m_completedHandle; }
    QString completedUri() const { return m_completedUri; }
    QString cancelledHandle() const { return m_cancelledHandle; }

    void emitOpened(const QString &handle, const QString &mode, const QVariantMap &options)
    {
        emit ScreenshotOpened(handle, mode, QStringLiteral("org.example.App"), QString(),
                              options);
    }

    // The handles a previous shell left in the backend's registry. Each row is
    // the `(handle, mode)` pair the real `PendingScreenshots` returns.
    void setPending(const QStringList &handles)
    {
        m_pending.clear();
        for (const QString &handle : handles)
            m_pending.append(PortalPendingRow{handle, QStringLiteral("region")});
    }

public slots:
    bool CompleteScreenshot(const QString &handle, const QString &uri)
    {
        m_completedHandle = handle;
        m_completedUri = uri;
        return true;
    }

    bool CancelScreenshot(const QString &handle)
    {
        m_cancelledHandle = handle;
        return true;
    }

    QList<PortalPendingRow> PendingScreenshots() const { return m_pending; }

signals:
    void ScreenshotOpened(const QString &handle, const QString &mode, const QString &appId,
                          const QString &parentWindow, const QVariantMap &options);

private:
    QString m_completedHandle;
    QString m_completedUri;
    QString m_cancelledHandle;
    QList<PortalPendingRow> m_pending;
};

QString connectFakePortal(FakePortal &portal, const QString &connectionName)
{
    QDBusConnection serviceBus =
        QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
    if (!serviceBus.isConnected())
        return QString();
    if (!serviceBus.registerService(kService)) {
        QDBusConnection::disconnectFromBus(connectionName);
        return QString();
    }
    if (!serviceBus.registerObject(kPath, &portal,
                                   QDBusConnection::ExportAllSlots
                                       | QDBusConnection::ExportAllSignals)) {
        serviceBus.unregisterService(kService);
        QDBusConnection::disconnectFromBus(connectionName);
        return QString();
    }
    return connectionName;
}

void disconnectFakePortal(const QString &connectionName)
{
    if (connectionName.isEmpty())
        return;
    QDBusConnection serviceBus =
        QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
    serviceBus.unregisterObject(kPath);
    serviceBus.unregisterService(kService);
    QDBusConnection::disconnectFromBus(connectionName);
}

} // namespace

class TestScreenshot : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        qDBusRegisterMetaType<PortalPendingRow>();
        qDBusRegisterMetaType<QList<PortalPendingRow>>();
    }

    void beginPresentsTheRequestedMode()
    {
        ScreenshotBridge bridge;
        QSignalSpy started(&bridge, &ScreenshotBridge::started);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("window"),
                     QStringLiteral("org.example.App"), QString());
        QCOMPARE(started.count(), 1);
        QVERIFY(bridge.active());
        QCOMPARE(bridge.mode(), QStringLiteral("window"));
        QCOMPARE(bridge.handle(), QStringLiteral("/req/1"));
        QCOMPARE(bridge.appId(), QStringLiteral("org.example.App"));
    }

    void acceptingHandsTheSelectionToTheCaptureSeam()
    {
        ScreenshotBridge bridge;
        QSignalSpy captured(&bridge, &ScreenshotBridge::captureRequested);
        QSignalSpy finished(&bridge, &ScreenshotBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("region"),
                     QStringLiteral("org.example.App"), QString());
        bridge.accept(12, 34, 300, 200);
        QCOMPARE(captured.count(), 1);
        const QList<QVariant> args = captured.at(0);
        QCOMPARE(args.at(0).toString(), QStringLiteral("region"));
        QCOMPARE(args.at(1).toInt(), 12);
        QCOMPARE(args.at(2).toInt(), 34);
        QCOMPARE(args.at(3).toInt(), 300);
        QCOMPARE(args.at(4).toInt(), 200);
        // A portal request stays active until the capture returns a URI.
        QCOMPARE(finished.count(), 0);
        QVERIFY(bridge.active());

        bridge.complete(QStringLiteral("file:///tmp/shot.png"));
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), true);
        QVERIFY(!bridge.active());
    }

    void anEmptySelectionIsRefused()
    {
        ScreenshotBridge bridge;
        QSignalSpy captured(&bridge, &ScreenshotBridge::captureRequested);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("region"), QString(), QString());
        bridge.accept(1, 2, 0, 0);
        QCOMPARE(captured.count(), 0);
        QVERIFY(bridge.active());
        QVERIFY(!bridge.error().isEmpty());
    }

    void beginLocalFinishesWithoutAPortal()
    {
        ScreenshotBridge bridge;
        QSignalSpy captured(&bridge, &ScreenshotBridge::captureRequested);
        QSignalSpy finished(&bridge, &ScreenshotBridge::finished);
        bridge.beginLocal(QStringLiteral("fullscreen"));
        QVERIFY(bridge.active());
        QCOMPARE(bridge.mode(), QStringLiteral("fullscreen"));
        QVERIFY(bridge.handle().isEmpty());
        bridge.accept(0, 0, 1920, 1080);
        QCOMPARE(captured.count(), 1);
        QCOMPARE(finished.count(), 1);
        QVERIFY(!bridge.active());
    }

    void cancelResolvesTheRequest()
    {
        ScreenshotBridge bridge;
        QSignalSpy finished(&bridge, &ScreenshotBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("region"), QString(), QString());
        bridge.cancel();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), false);
        QVERIFY(!bridge.active());
    }

    void aSecondRequestCancelsTheFirstPortalHandle()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        const QString connectionName =
            connectFakePortal(portal, QStringLiteral("dragonfruit_test_screenshot_replace"));
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        ScreenshotBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());
        // Let the signal match rule settle before the fake emits.
        QTest::qWait(50);

        portal.emitOpened(QStringLiteral("/req/first"), QStringLiteral("region"), QVariantMap());
        QTRY_VERIFY(bridge.active());
        QCOMPARE(bridge.handle(), QStringLiteral("/req/first"));

        // A second screenshot request replaces the first; the first caller
        // must be answered rather than left to the portal's deadline.
        portal.emitOpened(QStringLiteral("/req/second"), QStringLiteral("window"), QVariantMap());
        QTRY_COMPARE(portal.cancelledHandle(), QStringLiteral("/req/first"));
        QCOMPARE(bridge.handle(), QStringLiteral("/req/second"));
        QVERIFY(bridge.active());

        disconnectFakePortal(connectionName);
    }

    void theLocalShortcutCancelsAPortalRequest()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        const QString connectionName =
            connectFakePortal(portal, QStringLiteral("dragonfruit_test_screenshot_local"));
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        ScreenshotBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());
        // Let the signal match rule settle before the fake emits.
        QTest::qWait(50);

        portal.emitOpened(QStringLiteral("/req/portal"), QStringLiteral("region"), QVariantMap());
        QTRY_VERIFY(bridge.active());
        bridge.beginLocal(QStringLiteral("fullscreen"));
        QTRY_COMPARE(portal.cancelledHandle(), QStringLiteral("/req/portal"));
        QVERIFY(bridge.active());
        QVERIFY(bridge.handle().isEmpty());

        disconnectFakePortal(connectionName);
    }

    void startupCancelsTheBackendsPendingRequests()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        portal.setPending({QStringLiteral("/req/stale")});
        const QString connectionName =
            connectFakePortal(portal, QStringLiteral("dragonfruit_test_screenshot_pending"));
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        // A restarted shell asks what a previous instance left behind and
        // answers it instead of silently dropping the handle.
        ScreenshotBridge bridge;
        bridge.connectService();
        QTRY_COMPARE(portal.cancelledHandle(), QStringLiteral("/req/stale"));

        disconnectFakePortal(connectionName);
    }

    void theSelectionRoundTripsToThePortalPresenter()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        const QString connectionName = QStringLiteral("dragonfruit_test_screenshot_portal");
        QDBusConnection serviceBus =
            QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
        QVERIFY(serviceBus.isConnected());

        FakePortal portal;
        QVERIFY(serviceBus.registerService(kService));
        QVERIFY(serviceBus.registerObject(kPath, &portal,
                                          QDBusConnection::ExportAllSlots
                                              | QDBusConnection::ExportAllSignals));

        ScreenshotBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());

        QVariantMap options;
        options.insert(QStringLiteral("mode"), QStringLiteral("window"));
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/1"),
                          QStringLiteral("window"), options);

        QTRY_VERIFY(bridge.active());
        QCOMPARE(bridge.mode(), QStringLiteral("window"));
        bridge.accept(500, 400, 1, 1);
        bridge.complete(QStringLiteral("file:///tmp/shot.png"));

        QTRY_COMPARE(portal.completedHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/1"));
        QCOMPARE(portal.completedUri(), QStringLiteral("file:///tmp/shot.png"));

        // A second request cancelled through the seam reaches the portal too.
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/2"),
                          QStringLiteral("region"), QVariantMap());
        QTRY_VERIFY(bridge.active());
        bridge.cancel();
        QTRY_COMPARE(portal.cancelledHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/2"));

        serviceBus.unregisterObject(kPath);
        serviceBus.unregisterService(kService);
        QDBusConnection::disconnectFromBus(connectionName);
    }
};

QTEST_GUILESS_MAIN(TestScreenshot)
#include "tst_screenshot.moc"