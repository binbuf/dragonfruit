// SPDX-License-Identifier: MIT
// FileChooser picker tests (T-13.2b): the pure request model (listing through
// files-core, selection, navigation, the SaveFile name) and the live
// `ChooserBridge` against a fake `org.dragonfruit.Portal1` presenter surface
// on the private session bus that `dbus-run-session` provides. The D-Bus half
// skips when no bus is available, so the test is safe on a bus-less host.
#include "chooserbridge.h"

#include <QDBusConnection>
#include <QDir>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>
#include <QUrl>
#include <QVariantMap>

namespace {

const QString kService = QStringLiteral("org.freedesktop.impl.portal.desktop.dragonfruit");
const QString kPath = QStringLiteral("/org/freedesktop/portal/desktop");
const QString kInterface = QStringLiteral("org.dragonfruit.Portal1");

// A minimal stand-in for the backend's presenter surface (T-13.2a): the three
// methods the picker calls and the `FileChooserOpened` signal it watches.
class FakePortal : public QObject
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.dragonfruit.Portal1")

public:
    QString completedHandle() const { return m_completedHandle; }
    QStringList completedUris() const { return m_completedUris; }
    QString cancelledHandle() const { return m_cancelledHandle; }

    void emitOpened(const QString &handle, const QString &kind, const QString &title,
                    const QVariantMap &options)
    {
        emit FileChooserOpened(handle, kind, QStringLiteral("org.example.App"), QString(), title,
                               options);
    }

public slots:
    bool CompleteFileChooser(const QString &handle, const QStringList &selections)
    {
        m_completedHandle = handle;
        m_completedUris = selections;
        return true;
    }

    bool CancelFileChooser(const QString &handle)
    {
        m_cancelledHandle = handle;
        return true;
    }

    QVariantList PendingFileChoosers() const { return {}; }

signals:
    void FileChooserOpened(const QString &handle, const QString &kind, const QString &appId,
                           const QString &parentWindow, const QString &title,
                           const QVariantMap &options);

private:
    QString m_completedHandle;
    QStringList m_completedUris;
    QString m_cancelledHandle;
};

// A temp folder with one directory and one file, sorted folders-first.
struct SampleFolder {
    QTemporaryDir dir;

    SampleFolder()
    {
        dir.setAutoRemove(true);
        QDir(dir.path()).mkpath(QStringLiteral("alpha"));
        QFile file(dir.filePath(QStringLiteral("zeta.txt")));
        (void)file.open(QIODevice::WriteOnly);
        file.write("hello");
    }

    QString uri() const { return QUrl::fromLocalFile(dir.path()).toString(); }
    QString fileUri() const
    {
        return QUrl::fromLocalFile(dir.filePath(QStringLiteral("zeta.txt"))).toString();
    }
};

} // namespace

class TestChooser : public QObject
{
    Q_OBJECT

private slots:
    void beginListsThroughFilesCoreFoldersFirst()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy started(&bridge, &ChooserBridge::started);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QStringLiteral("Open File"),
                     QString(), false, false, folder.uri());
        QCOMPARE(started.count(), 1);
        QVERIFY(bridge.active());
        QCOMPARE(bridge.entries().size(), 2);

        const QVariantMap first = bridge.entries().at(0).toMap();
        QCOMPARE(first.value(QStringLiteral("name")).toString(), QStringLiteral("alpha"));
        QVERIFY(first.value(QStringLiteral("directory")).toBool());
        const QVariantMap second = bridge.entries().at(1).toMap();
        QCOMPARE(second.value(QStringLiteral("name")).toString(), QStringLiteral("zeta.txt"));
        QVERIFY(!second.value(QStringLiteral("directory")).toBool());
    }

    void selectingAFileAndAcceptingFinishes()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri());
        bridge.select(1);
        QCOMPARE(bridge.selectedIndex(), 1);
        QVERIFY(bridge.canGoUp());
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), true);
        QVERIFY(!bridge.active());
    }

    void aFolderCanBeChosenInDirectoryMode()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     true, folder.uri());
        QVERIFY(bridge.directoryMode());
        // No row selected: the current folder is the selection.
        bridge.accept();
        QVERIFY(!bridge.active());
    }

    void saveModeJoinsTheTypedName()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/2"), QStringLiteral("save"), QStringLiteral("Save File"),
                     QStringLiteral("Save"), false, false, folder.uri());
        QVERIFY(bridge.saveMode());
        bridge.accept(); // no name yet
        QCOMPARE(finished.count(), 0);
        QVERIFY(bridge.active());
        QVERIFY(!bridge.error().isEmpty());
        bridge.setSaveName(QStringLiteral("report.txt"));
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QVERIFY(!bridge.active());
    }

    void goUpNavigatesToTheParent()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri() + QStringLiteral("/alpha"));
        QCOMPARE(bridge.entries().size(), 0);
        bridge.goUp();
        QTRY_COMPARE(bridge.entries().size(), 2);
    }

    void cancelResolvesTheRequest()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri());
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

        // Register on a separate connection so the bridge sees the portal as a
        // remote peer (the same arrangement the real backend uses).
        const QString connectionName = QStringLiteral("dragonfruit_test_portal");
        QDBusConnection serviceBus =
            QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
        QVERIFY(serviceBus.isConnected());

        FakePortal portal;
        QVERIFY(serviceBus.registerService(kService));
        QVERIFY(serviceBus.registerObject(kPath, &portal,
                                          QDBusConnection::ExportAllSlots
                                              | QDBusConnection::ExportAllSignals));

        SampleFolder folder;
        ChooserBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());

        QVariantMap options;
        // The real portal carries `current_folder` as a NUL-terminated `ay`;
        // Qt hands the shell that trailing NUL, which the presenter must trim
        // (else the URI grows a `%00` and the listing fails).
        QByteArray folderBytes = folder.dir.path().toUtf8();
        folderBytes.append('\0');
        options.insert(QStringLiteral("current_folder"), folderBytes);
        options.insert(QStringLiteral("multiple"), false);
        options.insert(QStringLiteral("directory"), false);
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/1"),
                          QStringLiteral("open"), QStringLiteral("Open File"), options);

        QTRY_VERIFY(bridge.active());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.select(1);
        bridge.accept();

        QTRY_COMPARE(portal.completedHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/1"));
        QCOMPARE(portal.completedUris(), QStringList{folder.fileUri()});

        // A second request cancelled through the seam reaches the portal too.
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/2"),
                          QStringLiteral("open"), QStringLiteral("Open File"), options);
        QTRY_VERIFY(bridge.active());
        bridge.cancel();
        QTRY_COMPARE(portal.cancelledHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/2"));

        serviceBus.unregisterObject(kPath);
        serviceBus.unregisterService(kService);
        QDBusConnection::disconnectFromBus(connectionName);
    }
};

QTEST_GUILESS_MAIN(TestChooser)
#include "tst_chooser.moc"