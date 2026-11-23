// SPDX-License-Identifier: MIT
// FileChooser picker tests (T-13.2b): the pure request model (listing through
// files-core, selection, navigation, the SaveFile name, overwrite
// confirmation, SaveFiles) and the live `ChooserBridge` against a fake
// `org.dragonfruit.Portal1` presenter surface on the private session bus that
// `dbus-run-session` provides. The D-Bus half skips when no bus is available,
// so the test is safe on a bus-less host.
#include "chooserbridge.h"

#include "portalrow.h"

#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusMetaType>
#include <QDir>
#include <QFile>
#include <QFileInfo>
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
    void clear()
    {
        m_completedHandle.clear();
        m_completedUris.clear();
        m_cancelledHandle.clear();
    }

    void emitOpened(const QString &handle, const QString &kind, const QString &title,
                    const QVariantMap &options)
    {
        emit FileChooserOpened(handle, kind, QStringLiteral("org.example.App"), QString(), title,
                               options);
    }

    // The handles a previous shell left in the backend's registry. Each row is
    // the `(handle, kind)` pair the real `PendingFileChoosers` returns.
    void setPending(const QStringList &handles)
    {
        m_pending.clear();
        for (const QString &handle : handles)
            m_pending.append(PortalPendingRow{handle, QStringLiteral("open")});
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

    QList<PortalPendingRow> PendingFileChoosers() const { return m_pending; }

signals:
    void FileChooserOpened(const QString &handle, const QString &kind, const QString &appId,
                           const QString &parentWindow, const QString &title,
                           const QVariantMap &options);

private:
    QString m_completedHandle;
    QStringList m_completedUris;
    QString m_cancelledHandle;
    QList<PortalPendingRow> m_pending;
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
    QString folderUri() const
    {
        return QUrl::fromLocalFile(dir.filePath(QStringLiteral("alpha"))).toString();
    }
};

// A folder with two files, for the multi-selection path.
struct MultiFolder {
    QTemporaryDir dir;

    MultiFolder()
    {
        dir.setAutoRemove(true);
        for (const char *name : { "one.txt", "two.txt" }) {
            QFile file(dir.filePath(QString::fromLatin1(name)));
            (void)file.open(QIODevice::WriteOnly);
            file.write("x");
        }
    }

    QString uri() const { return QUrl::fromLocalFile(dir.path()).toString(); }
    QString fileUri(const char *name) const
    {
        return QUrl::fromLocalFile(dir.filePath(QString::fromLatin1(name))).toString();
    }
};

// Register the fake portal on its own connection, as the real backend is a
// separate peer.
QString connectFakePortal(FakePortal &portal)
{
    const QString connectionName = QStringLiteral("dragonfruit_test_portal");
    QDBusConnection serviceBus =
        QDBusConnection::connectToBus(QDBusConnection::SessionBus, connectionName);
    if (!serviceBus.isConnected())
        return QString();
    if (!serviceBus.registerService(kService))
        return QString();
    if (!serviceBus.registerObject(kPath, &portal,
                                   QDBusConnection::ExportAllSlots
                                       | QDBusConnection::ExportAllSignals)) {
        serviceBus.unregisterService(kService);
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

class TestChooser : public QObject
{
    Q_OBJECT

private slots:
    void initTestCase()
    {
        qDBusRegisterMetaType<PortalPendingRow>();
        qDBusRegisterMetaType<QList<PortalPendingRow>>();
    }

    void beginListsThroughFilesCoreFoldersFirst()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy started(&bridge, &ChooserBridge::started);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QStringLiteral("Open File"),
                     QString(), false, false, folder.uri());
        QCOMPARE(started.count(), 1);
        QVERIFY(bridge.active());
        QTRY_COMPARE(bridge.entries().size(), 2);

        const QVariantMap first = bridge.entries().at(0).toMap();
        QCOMPARE(first.value(QStringLiteral("name")).toString(), QStringLiteral("alpha"));
        QVERIFY(first.value(QStringLiteral("directory")).toBool());
        const QVariantMap second = bridge.entries().at(1).toMap();
        QCOMPARE(second.value(QStringLiteral("name")).toString(), QStringLiteral("zeta.txt"));
        QVERIFY(!second.value(QStringLiteral("directory")).toBool());
        QCOMPARE(bridge.breadcrumbs().last().toMap().value(QStringLiteral("label")).toString(),
                 QFileInfo(folder.dir.path()).fileName());
    }

    void selectingAFileAndAcceptingFinishes()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.select(1, 0);
        QCOMPARE(bridge.selectedIndex(), 1);
        QCOMPARE(bridge.selectedIndices().size(), 1);
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
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     true, folder.uri());
        QVERIFY(bridge.directoryMode());
        QTRY_COMPARE(bridge.entries().size(), 2);
        // No row selected: the current folder is the selection.
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), true);
    }

    void aSelectedFolderIsTheDirectoryModeSelection()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     true, folder.uri());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.select(0, 0);
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), true);
    }

    void saveModeJoinsTheTypedName()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/2"), QStringLiteral("save"), QStringLiteral("Save File"),
                     QStringLiteral("Save"), false, false, folder.uri());
        QVERIFY(bridge.saveMode());
        QVERIFY(!bridge.saveFilesMode());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.accept(); // no name yet
        QCOMPARE(finished.count(), 0);
        QVERIFY(bridge.active());
        QVERIFY(!bridge.error().isEmpty());
        bridge.setSaveName(QStringLiteral("report.txt"));
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QVERIFY(!bridge.active());
    }

    void saveModeConfirmsBeforeReplacing()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/2"), QStringLiteral("save"), QString(), QString(), false,
                     false, folder.uri());
        QTRY_COMPARE(bridge.entries().size(), 2);

        bridge.setSaveName(QStringLiteral("zeta.txt"));
        bridge.accept();
        QVERIFY(bridge.confirmReplace());
        QCOMPARE(bridge.replaceName(), QStringLiteral("zeta.txt"));
        QCOMPARE(finished.count(), 0);

        // Keeping editing returns to the dialog without resolving it.
        bridge.dismissReplace();
        QVERIFY(!bridge.confirmReplace());
        QVERIFY(bridge.active());
        QCOMPARE(finished.count(), 0);

        // Replace resolves it.
        bridge.accept();
        QVERIFY(bridge.confirmReplace());
        bridge.acceptReplace();
        QCOMPARE(finished.count(), 1);
        QVERIFY(!bridge.active());
    }

    void saveModeAcceptsAnAbsolutePath()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/2"), QStringLiteral("save"), QString(), QString(), false,
                     false, folder.uri());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.setSaveName(folder.dir.filePath(QStringLiteral("absolute.txt")));
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QVERIFY(!bridge.active());
    }

    void saveFilesModeChoosesTheFolder()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        QSignalSpy finished(&bridge, &ChooserBridge::finished);
        bridge.begin(QStringLiteral("/req/3"), QStringLiteral("save-files"), QString(), QString(),
                     false, false, folder.uri());
        QVERIFY(bridge.saveFilesMode());
        QVERIFY(!bridge.saveMode());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.accept();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), true);
    }

    void multipleSelectionKeepsEveryChosenRow()
    {
        MultiFolder folder;
        ChooserBridge bridge;
        bridge.begin(QStringLiteral("/req/4"), QStringLiteral("open"), QString(), QString(), true,
                     false, folder.uri());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.select(0, 0);
        bridge.select(1, Qt::ControlModifier);
        QCOMPARE(bridge.selectedIndices().size(), 2);
        bridge.select(1, Qt::ControlModifier);
        QCOMPARE(bridge.selectedIndices().size(), 1);
        bridge.select(0, Qt::ShiftModifier);
        QCOMPARE(bridge.selectedIndices().size(), 2);
    }

    void selectAllSelectsEveryRowInAMultipleRequest()
    {
        MultiFolder folder;
        ChooserBridge bridge;
        bridge.begin(QStringLiteral("/req/4"), QStringLiteral("open"), QString(), QString(), true,
                     false, folder.uri());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.selectAll();
        QCOMPARE(bridge.selectedIndices().size(), 2);

        // A single-selection request does not grow a multi-selection.
        ChooserBridge single;
        single.begin(QStringLiteral("/req/5"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri());
        QTRY_COMPARE(single.entries().size(), 2);
        single.selectAll();
        QCOMPARE(single.selectedIndices().size(), 0);
    }

    void newFolderAddsARowAndKeepsTheListingAlive()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        bridge.begin(QStringLiteral("/req/6"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.newFolder();
        QTRY_COMPARE(bridge.entries().size(), 3);
        bool found = false;
        for (const QVariant &row : bridge.entries()) {
            const QVariantMap map = row.toMap();
            if (map.value(QStringLiteral("name")).toString()
                == QStringLiteral("untitled folder")) {
                found = true;
                QVERIFY(map.value(QStringLiteral("directory")).toBool());
            }
        }
        QVERIFY(found);
    }

    void requestsQueueBehindTheLiveDialog()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri());
        QCOMPARE(bridge.handle(), QStringLiteral("/req/1"));
        bridge.begin(QStringLiteral("/req/2"), QStringLiteral("save"), QString(), QString(), false,
                     false, folder.uri());
        // The first request stays live; the second waits.
        QCOMPARE(bridge.handle(), QStringLiteral("/req/1"));
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.cancel();
        QTRY_COMPARE(bridge.handle(), QStringLiteral("/req/2"));
        QVERIFY(bridge.active());
    }

    void goUpNavigatesToTheParent()
    {
        SampleFolder folder;
        ChooserBridge bridge;
        bridge.begin(QStringLiteral("/req/1"), QStringLiteral("open"), QString(), QString(), false,
                     false, folder.uri() + QStringLiteral("/alpha"));
        QTRY_COMPARE(bridge.entries().size(), 0);
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
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.cancel();
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), false);
        QVERIFY(!bridge.active());
    }

    void startupCancelsTheBackendsPendingRequests()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        portal.setPending({QStringLiteral("/req/stale")});
        const QString connectionName = connectFakePortal(portal);
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        // A stale chooser row carries only its handle and kind. Re-presenting
        // it would lose the caller's options (SaveFiles' names in particular),
        // so startup answers it with Cancel instead of dropping the handle.
        ChooserBridge bridge;
        bridge.connectService();
        QTRY_COMPARE(portal.cancelledHandle(), QStringLiteral("/req/stale"));
        QVERIFY(!bridge.active());

        disconnectFakePortal(connectionName);
    }

    void theSelectionRoundTripsToThePortalPresenter()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        const QString connectionName = connectFakePortal(portal);
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

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
        bridge.select(1, 0);
        bridge.accept();

        QTRY_COMPARE(portal.completedHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/1"));
        QCOMPARE(portal.completedUris(), QStringList{folder.fileUri()});

        // A second request cancelled through the seam reaches the portal too.
        portal.clear();
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/2"),
                          QStringLiteral("open"), QStringLiteral("Open File"), options);
        QTRY_VERIFY(bridge.active());
        bridge.cancel();
        QTRY_COMPARE(portal.cancelledHandle(),
                     QStringLiteral("/org/freedesktop/portal/desktop/request/2"));

        disconnectFakePortal(connectionName);
    }

    void theSaveRequestPrefillsTheNameAndConfirms()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        const QString connectionName = connectFakePortal(portal);
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        SampleFolder folder;
        ChooserBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());

        QVariantMap options;
        QByteArray fileBytes = folder.dir.filePath(QStringLiteral("zeta.txt")).toUtf8();
        fileBytes.append('\0');
        options.insert(QStringLiteral("current_file"), fileBytes);
        options.insert(QStringLiteral("current_name"), QStringLiteral("zeta.txt"));
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/3"),
                          QStringLiteral("save"), QStringLiteral("Save File"), options);

        QTRY_VERIFY(bridge.active());
        QCOMPARE(bridge.kind(), QStringLiteral("save"));
        QCOMPARE(bridge.saveName(), QStringLiteral("zeta.txt"));
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.accept();
        QVERIFY(bridge.confirmReplace());
        bridge.acceptReplace();
        QTRY_COMPARE(portal.completedUris(), QStringList{folder.fileUri()});

        disconnectFakePortal(connectionName);
    }

    void theSaveRequestUsesASelectedFolder()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        const QString connectionName = connectFakePortal(portal);
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        SampleFolder folder;
        ChooserBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());

        QVariantMap options;
        QByteArray folderBytes = folder.dir.path().toUtf8();
        folderBytes.append('\0');
        options.insert(QStringLiteral("current_folder"), folderBytes);
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/6"),
                          QStringLiteral("save"), QStringLiteral("Save File"), options);

        QTRY_VERIFY(bridge.active());
        QTRY_COMPARE(bridge.entries().size(), 2);
        // The `alpha` folder is the destination for this save.
        bridge.select(0, 0);
        bridge.setSaveName(QStringLiteral("report.txt"));
        bridge.accept();
        QTRY_COMPARE(portal.completedUris(),
                     QStringList{ QUrl::fromLocalFile(folder.dir.filePath(
                                     QStringLiteral("alpha/report.txt")))
                                      .toString() });

        disconnectFakePortal(connectionName);
    }

    void theSaveFilesRequestReturnsTheChosenFolder()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        const QString connectionName = connectFakePortal(portal);
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        SampleFolder folder;
        ChooserBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());

        QVariantMap options;
        QByteArray folderBytes = folder.dir.path().toUtf8();
        folderBytes.append('\0');
        options.insert(QStringLiteral("current_folder"), folderBytes);
        QDBusArgument filesArgument;
        filesArgument.beginArray(QMetaType::fromType<QByteArray>());
        for (const char *name : { "one.txt", "two.txt" }) {
            QByteArray bytes(name);
            bytes.append('\0');
            filesArgument << bytes;
        }
        filesArgument.endArray();
        options.insert(QStringLiteral("files"), QVariant::fromValue(filesArgument));
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/4"),
                          QStringLiteral("save-files"), QStringLiteral("Save Files"), options);

        QTRY_VERIFY(bridge.active());
        QVERIFY(bridge.saveFilesMode());
        QCOMPARE(bridge.saveFiles(), QStringList({ QStringLiteral("one.txt"),
                                                   QStringLiteral("two.txt") }));
        QTRY_COMPARE(bridge.entries().size(), 2);
        // A selected folder row is the destination.
        bridge.select(0, 0);
        bridge.accept();
        // The presenter returns the folder; the portal appends the names.
        QTRY_COMPARE(portal.completedUris(), QStringList{folder.folderUri()});

        disconnectFakePortal(connectionName);
    }

    void theOpenRequestReturnsEverySelectedFile()
    {
        QDBusConnection bus = QDBusConnection::sessionBus();
        if (!bus.isConnected())
            QSKIP("no session bus available (run under dbus-run-session)");

        FakePortal portal;
        const QString connectionName = connectFakePortal(portal);
        if (connectionName.isEmpty())
            QSKIP("the fake presenter could not own the portal name");

        MultiFolder folder;
        ChooserBridge bridge;
        bridge.connectService();
        QTRY_VERIFY(bridge.serviceAvailable());

        QVariantMap options;
        QByteArray folderBytes = folder.dir.path().toUtf8();
        folderBytes.append('\0');
        options.insert(QStringLiteral("current_folder"), folderBytes);
        options.insert(QStringLiteral("multiple"), true);
        portal.emitOpened(QStringLiteral("/org/freedesktop/portal/desktop/request/5"),
                          QStringLiteral("open"), QStringLiteral("Open Files"), options);

        QTRY_VERIFY(bridge.active());
        QTRY_COMPARE(bridge.entries().size(), 2);
        bridge.select(0, 0);
        bridge.select(1, Qt::ControlModifier);
        bridge.accept();
        QTRY_COMPARE(portal.completedUris(),
                     (QStringList{ folder.fileUri("one.txt"), folder.fileUri("two.txt") }));

        disconnectFakePortal(connectionName);
    }
};

QTEST_GUILESS_MAIN(TestChooser)
#include "tst_chooser.moc"
