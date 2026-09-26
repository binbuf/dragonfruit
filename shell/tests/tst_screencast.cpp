// SPDX-License-Identifier: MIT
// ScreenCast source picker bridge tests (T-13.4a): the pure request lifecycle
// (options decode, selection modes, the chosen-source hand-off, cancellation)
// and the live `ScreenCastBridge` against a fake `org.dragonfruit.Portal1`
// presenter surface on the private session bus that `dbus-run-session`
// provides. The D-Bus half skips when no bus is available, so the test is safe
// on a bus-less host.
#include "screencastbridge.h"

#include <QDBusConnection>
#include <QDBusMetaType>
#include <QSignalSpy>
#include <QTest>
#include <QVariantMap>

namespace {

const QString kService = QStringLiteral("org.freedesktop.impl.portal.desktop.dragonfruit");
const QString kPath = QStringLiteral("/org/freedesktop/portal/desktop");

// A minimal stand-in for the backend's presenter surface (T-13.4a): the two
// methods the picker calls and the `ScreenCastOpened` signal it watches.
class FakePortal : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Portal1")

public:
    QString completedHandle() const { return m_completedHandle; }
    QList<ScreenCastSelection> completedSelections() const { return m_completedSelections; }
    QString cancelledHandle() const { return m_cancelledHandle; }

    void emitOpened(const QString &handle, const QString &sessionHandle, uint types,
                    bool multiple, const QVariantMap &options)
    {
        emit ScreenCastOpened(handle, sessionHandle, QStringLiteral("org.example.App"), types,
                              multiple, options);
    }

public slots:
    bool CompleteScreenCast(const QString &handle, const QList<ScreenCastSelection> &selections)
    {
        m_completedHandle = handle;
        m_completedSelections = selections;
        return true;
    }

    bool CancelScreenCast(const QString &handle)
    {
        m_cancelledHandle = handle;
        return true;
    }

    QVariantList PendingScreenCasts() const { return {}; }

signals:
    void ScreenCastOpened(const QString &handle, const QString &sessionHandle,
                          const QString &appId, uint types, bool multiple,
                          const QVariantMap &options);

private:
    QString m_completedHandle;
    QList<ScreenCastSelection> m_completedSelections;
    QString m_cancelledHandle;
};

QVariantList source(const QString &kind, const QString &id, const QString &label)
{
    return QVariantList{QVariantMap{
        {QStringLiteral("id"), id},
        {QStringLiteral("kind"), kind},
        {QStringLiteral("label"), label},
        {QStringLiteral("detail"), QStringLiteral("1920 × 1080")},
    }};
}

QVariantList monitor(const QString &id, const QString &label)
{
    return source(QStringLiteral("monitor"), id, label);
}

QVariantList window(const QString &id, const QString &label)
{
    return source(QStringLiteral("window"), id, label);
}

} // namespace

class TestScreenCast : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        qDBusRegisterMetaType<ScreenCastSelection>();
        qDBusRegisterMetaType<QList<ScreenCastSelection>>();
    }

    void beginPublishesTheRequestedOptions()
    {
        ScreenCastBridge bridge;
        QSignalSpy started(&bridge, &ScreenCastBridge::started);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("/session/1"),
                     QStringLiteral("org.example.App"), 1 | 2, true, 2);
        QCOMPARE(started.count(), 1);
        QVERIFY(bridge.active());
        QCOMPARE(bridge.handle(), QStringLiteral("/req/1"));
        QCOMPARE(bridge.sessionHandle(), QStringLiteral("/session/1"));
        QCOMPARE(bridge.appId(), QStringLiteral("org.example.App"));
        QCOMPARE(bridge.types(), 3u);
        QVERIFY(bridge.multiple());
        QCOMPARE(bridge.cursorMode(), 2u);
    }

    void theSourceListIsPublishedUnselected()
    {
        ScreenCastBridge bridge;
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("/session/1"), QString(),
                     3, true, 1);
        bridge.setSources(monitor(QStringLiteral("monitor:1"), QStringLiteral("Display")));
        QCOMPARE(bridge.sources().size(), 1);
        QCOMPARE(bridge.selectedCount(), 0);
        QCOMPARE(bridge.sources().at(0).toMap().value(QStringLiteral("selected")).toBool(),
                 false);
    }

    void singleSelectionReplacesThePreviousChoice()
    {
        ScreenCastBridge bridge;
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("/session/1"), QString(), 3,
                     false, 1);
        bridge.setSources(monitor(QStringLiteral("monitor:1"), QStringLiteral("One"))
                          + monitor(QStringLiteral("monitor:2"), QStringLiteral("Two")));
        bridge.select(QStringLiteral("monitor:1"));
        QCOMPARE(bridge.selectedCount(), 1);
        bridge.select(QStringLiteral("monitor:2"));
        QCOMPARE(bridge.selectedCount(), 1);
        QCOMPARE(bridge.sources().at(0).toMap().value(QStringLiteral("selected")).toBool(),
                 false);
        QCOMPARE(bridge.sources().at(1).toMap().value(QStringLiteral("selected")).toBool(),
                 true);
    }

    void multipleSelectionTogglesEachSource()
    {
        ScreenCastBridge bridge;
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("/session/1"), QString(), 3,
                     true, 1);
        bridge.setSources(monitor(QStringLiteral("monitor:1"), QStringLiteral("One"))
                          + monitor(QStringLiteral("monitor:2"), QStringLiteral("Two")));
        bridge.select(QStringLiteral("monitor:1"));
        bridge.select(QStringLiteral("monitor:2"));
        QCOMPARE(bridge.selectedCount(), 2);
        bridge.select(QStringLiteral("monitor:1"));
        QCOMPARE(bridge.selectedCount(), 1);
    }

    void acceptingWithNoSelectionIsRefused()
    {
        ScreenCastBridge bridge;
        QSignalSpy finished(&bridge, &ScreenCastBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("/session/1"), QString(), 3,
                     true, 1);
        bridge.setSources(monitor(QStringLiteral("monitor:1"), QStringLiteral("One")));
        bridge.accept();
        QCOMPARE(finished.count(), 0);
        QVERIFY(bridge.active());
        QVERIFY(!bridge.error().isEmpty());
    }

    void acceptingClosesTheRequest()
    {
        ScreenCastBridge bridge;
        QSignalSpy finished(&bridge, &ScreenCastBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("/session/1"), QString(), 3,
                     true, 1);
        bridge.setSources(monitor(QStringLiteral("monitor:1"), QStringLiteral("One")));
        bridge.select(QStringLiteral("monitor:1"));
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), true);
        QVERIFY(!bridge.active());
        QCOMPARE(bridge.selectedCount(), 0);
    }

    void cancelResolvesTheRequest()
    {
        ScreenCastBridge bridge;
        QSignalSpy finished(&bridge, &ScreenCastBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("/session/1"), QString(), 3,
                     false, 1);
        bridge.cancel();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), false);
        QVERIFY(!bridge.active());
    }

    void theSelectionRoundTripsToThePortalPresenter()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        const QString connectionName = QStringLiteral("dragonfruit_test_screencast_portal");
        QDBusConnection serviceBus =
            QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
        QVERIFY(serviceBus.isConnected());

        FakePortal portal;
        QVERIFY(serviceBus.registerService(kService));
        QVERIFY(serviceBus.registerObject(kPath, &portal,
                                          QDBusConnection::ExportAllSlots
                                              | QDBusConnection::ExportAllSignals));

        ScreenCastBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());

        QVariantMap options;
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/1"),
                          QStringLiteral("/org/freedesktop/portal/desktop/session/1"), 1 | 2,
                          true, options);

        QTRY_VERIFY(bridge.active());
        QCOMPARE(bridge.types(), 3u);
        QVERIFY(bridge.multiple());
        bridge.setSources(monitor(QStringLiteral("monitor:DP-1"), QStringLiteral("Display"))
                          + window(QStringLiteral("window:7"), QStringLiteral("Files")));
        bridge.select(QStringLiteral("monitor:DP-1"));
        bridge.select(QStringLiteral("window:7"));
        bridge.accept();

        QTRY_COMPARE(portal.completedHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/1"));
        const QList<ScreenCastSelection> chosen = portal.completedSelections();
        QCOMPARE(chosen.size(), 2);
        // The kind drives the type bit: monitor = 1, window = 2.
        QCOMPARE(chosen.at(0).id, QStringLiteral("monitor:DP-1"));
        QCOMPARE(chosen.at(0).sourceType, 1u);
        QCOMPARE(chosen.at(1).id, QStringLiteral("window:7"));
        QCOMPARE(chosen.at(1).sourceType, 2u);

        // A second request cancelled through the seam reaches the portal too.
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/2"),
                          QStringLiteral("/org/freedesktop/portal/desktop/session/1"), 1,
                          false, QVariantMap());
        QTRY_VERIFY(bridge.active());
        bridge.cancel();
        QTRY_COMPARE(portal.cancelledHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/2"));

        serviceBus.unregisterObject(kPath);
        serviceBus.unregisterService(kService);
        QDBusConnection::disconnectFromBus(connectionName);
    }
};

QTEST_GUILESS_MAIN(TestScreenCast)
#include "tst_screencast.moc"