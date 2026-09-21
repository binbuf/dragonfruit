// SPDX-License-Identifier: MIT
// Dock core unit tests (T-10): the interim `.desktop` resolver/launcher, the
// `dock.pinned` persistence, and the pure pinned+running entry merge. Runs
// headless with no compositor, Wayland, or QML.
#include "desktopentry.h"
#include "dockdrops.h"
#include "dockmodel.h"
#include "dockpins.h"
#include "docksettings.h"
#include "downloadsmonitor.h"
#include "framecommitgate.h"
#include "trashmonitor.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>
#include <QUrl>

namespace {

void writeFile(const QString &path, const QString &contents)
{
    QFile file(path);
    QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Text));
    file.write(contents.toUtf8());
    file.close();
}

QString makeAppDir(QTemporaryDir &dir)
{
    const QString apps = dir.path() + QStringLiteral("/applications");
    QDir().mkpath(apps);
    return apps;
}
} // namespace

class TestDockCore : public QObject
{
    Q_OBJECT

private slots:
    // -- .desktop parsing ------------------------------------------------

    void parseReadsDesktopEntryFields()
    {
        const QString contents = QStringLiteral(
            "# a comment\n"
            "[Desktop Entry]\n"
            "Type=Application\n"
            "Name=Files\n"
            "Name[de]=Dateien\n"
            "Icon=system-file-manager\n"
            "Exec=nautilus %U\n"
            "StartupWMClass=org.example.Nautilus\n"
            "Categories=System;FileManager;\n"
            "Terminal=false\n"
            "\n"
            "[Desktop Action new-window]\n"
            "Name=New Window\n");
        const DesktopEntry entry =
            DesktopEntryIndex::parse(QStringLiteral("org.example.Nautilus.desktop"), contents);
        QVERIFY(entry.valid);
        QCOMPARE(entry.name, QStringLiteral("Files")); // unlocalized wins
        QCOMPARE(entry.icon, QStringLiteral("system-file-manager"));
        QCOMPARE(entry.exec, QStringLiteral("nautilus %U"));
        QCOMPARE(entry.startupWmClass, QStringLiteral("org.example.Nautilus"));
        QCOMPARE(entry.categories,
                 (QStringList{QStringLiteral("System"), QStringLiteral("FileManager")}));
        QCOMPARE(entry.terminal, false);
    }

    void parseIgnoresHiddenAndNoDisplay()
    {
        const DesktopEntry entry = DesktopEntryIndex::parse(
            QStringLiteral("hidden.desktop"),
            QStringLiteral("[Desktop Entry]\nName=Hidden\nExec=x\nNoDisplay=true\n"));
        QVERIFY(entry.valid);
        QVERIFY(entry.noDisplay);
        QVERIFY(!DesktopEntryIndex::isLaunchable(entry));
    }

    // -- resolution ------------------------------------------------------

    void resolveByIdSuffixAndWmClass()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString apps = makeAppDir(dir);
        writeFile(apps + QStringLiteral("/org.dragonfruit.Files.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Files\nExec=df-files\n"));
        writeFile(apps + QStringLiteral("/org.example.Nautilus.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Files\nExec=nautilus\n"
                                 "StartupWMClass=org.example.Nautilus\n"));
        writeFile(apps + QStringLiteral("/firefox.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Firefox\nExec=firefox %u\n"
                                 "StartupWMClass=firefox\n"));

        DesktopEntryIndex index;
        index.scan({apps});

        QCOMPARE(index.resolve(QStringLiteral("org.dragonfruit.Files")).id,
                 QStringLiteral("org.dragonfruit.Files.desktop"));
        QCOMPARE(index.resolve(QStringLiteral("org.dragonfruit.Files.desktop")).id,
                 QStringLiteral("org.dragonfruit.Files.desktop"));
        // StartupWMClass, case-insensitively.
        QCOMPARE(index.resolve(QStringLiteral("ORG.EXAMPLE.NAUTILUS")).id,
                 QStringLiteral("org.example.Nautilus.desktop"));
        QCOMPARE(index.resolve(QStringLiteral("firefox")).id, QStringLiteral("firefox.desktop"));
        // A miss is invalid, never a crash.
        QVERIFY(!index.resolve(QStringLiteral("does.not.Exist")).valid);
    }

    void resolveUsesFirstDirectoryForDuplicateIds()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString user = dir.path() + QStringLiteral("/user");
        const QString system = dir.path() + QStringLiteral("/system");
        QDir().mkpath(user);
        QDir().mkpath(system);
        writeFile(user + QStringLiteral("/dup.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=User\nExec=user\n"));
        writeFile(system + QStringLiteral("/dup.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=System\nExec=system\n"));

        DesktopEntryIndex index;
        index.scan({user, system});
        QCOMPARE(index.byId(QStringLiteral("dup.desktop")).name, QStringLiteral("User"));
    }

    // -- launch command --------------------------------------------------

    void buildLaunchCommandExpandsFileCodes()
    {
        DesktopEntry entry;
        entry.id = QStringLiteral("app.desktop");
        entry.name = QStringLiteral("App");
        entry.icon = QStringLiteral("app-icon");
        entry.exec = QStringLiteral("app --flag %U %i");
        entry.valid = true;

        const QStringList argv = DesktopEntryIndex::buildLaunchCommand(
            entry, {QStringLiteral("/a.txt"), QStringLiteral("/b.txt")});
        QCOMPARE(argv, (QStringList{QStringLiteral("app"), QStringLiteral("--flag"),
                                    QStringLiteral("/a.txt"), QStringLiteral("/b.txt"),
                                    QStringLiteral("--icon"), QStringLiteral("app-icon")}));
    }

    void buildLaunchCommandSingleFileCodeWithNoFiles()
    {
        DesktopEntry entry;
        entry.id = QStringLiteral("app.desktop");
        entry.name = QStringLiteral("App");
        entry.exec = QStringLiteral("app %u");
        entry.valid = true;
        QCOMPARE(DesktopEntryIndex::buildLaunchCommand(entry),
                 (QStringList{QStringLiteral("app")}));
    }

    void buildLaunchCommandHandlesQuotesAndPercent()
    {
        DesktopEntry entry;
        entry.id = QStringLiteral("app.desktop");
        entry.name = QStringLiteral("App");
        entry.exec = QStringLiteral("\"my app\" --label \"100%%\" %c");
        entry.valid = true;
        QCOMPARE(DesktopEntryIndex::buildLaunchCommand(entry),
                 (QStringList{QStringLiteral("my app"), QStringLiteral("--label"),
                              QStringLiteral("100%"), QStringLiteral("App")}));
    }

    // -- dock.pinned persistence ----------------------------------------

    void pinsRoundTripAndPreserveUnknownKeys()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString path = dir.path() + QStringLiteral("/dragonfruit/settings.json");

        DockPins pins(path);
        QVERIFY(!pins.fileExists());
        QVERIFY(pins.load());
        QVERIFY(pins.isEmpty());
        pins.setIds({QStringLiteral("a.desktop"), QStringLiteral("b.desktop")});
        QVERIFY(pins.save());

        DockPins reloaded(path);
        QVERIFY(reloaded.fileExists());
        QVERIFY(reloaded.load());
        QCOMPARE(reloaded.ids(),
                 (QStringList{QStringLiteral("a.desktop"), QStringLiteral("b.desktop")}));

        // An unrelated key written by another owner survives a Dock write.
        writeFile(path, QStringLiteral("{\n"
                                       "  \"schema\": 1,\n"
                                       "  \"future\": true,\n"
                                       "  \"keys\": { \"dock.pinned\": [\"a.desktop\"] }\n"
                                       "}\n"));

        DockPins third(path);
        QVERIFY(third.load());
        QVERIFY(third.add(QStringLiteral("c.desktop")));
        QVERIFY(third.save());
        QFile check(path);
        QVERIFY(check.open(QIODevice::ReadOnly | QIODevice::Text));
        const QByteArray written = check.readAll();
        QVERIFY(written.contains("\"future\": true"));
        QVERIFY(written.contains("c.desktop"));
    }

    void pinsMoveAndRemove()
    {
        DockPins pins(QStringLiteral("/nonexistent/settings.json"));
        pins.setIds({QStringLiteral("a"), QStringLiteral("b"), QStringLiteral("c")});
        QVERIFY(pins.move(0, 2));
        QCOMPARE(pins.ids(), (QStringList{QStringLiteral("b"), QStringLiteral("c"),
                                          QStringLiteral("a")}));
        QVERIFY(!pins.move(5, 0));
        QVERIFY(pins.remove(QStringLiteral("c")));
        QVERIFY(!pins.remove(QStringLiteral("c")));
    }

    void defaultPinsPickFirstPartyAndRegisteredCategories()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString apps = makeAppDir(dir);
        writeFile(apps + QStringLiteral("/org.dragonfruit.Files.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Files\nExec=df-files\n"));
        writeFile(apps + QStringLiteral("/org.dragonfruit.Settings.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Settings\nExec=df-settings\n"));
        writeFile(apps + QStringLiteral("/term.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Term\nExec=term\n"
                                 "Categories=System;TerminalEmulator;\n"));
        writeFile(apps + QStringLiteral("/browse.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Browser\nExec=browser\n"
                                 "Categories=Network;WebBrowser;\n"));
        // A hidden WebBrowser entry sorts first but must not be pinned.
        writeFile(apps + QStringLiteral("/aaa-hidden.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Hidden Browser\nExec=hidden\n"
                                 "NoDisplay=true\nCategories=Network;WebBrowser;\n"));

        DesktopEntryIndex index;
        index.scan({apps});
        QCOMPARE(DockPins::resolveDefaultPins(index),
                 (QStringList{QStringLiteral("org.dragonfruit.Files.desktop"),
                              QStringLiteral("org.dragonfruit.Settings.desktop"),
                              QStringLiteral("term.desktop"),
                              QStringLiteral("browse.desktop")}));
    }

    // -- dock.* settings (T-10 section 19) -------------------------------

    void settingsDefaultsWhenFileMissing()
    {
        DockSettings settings(QStringLiteral("/nonexistent/settings.json"));
        QVERIFY(settings.load());
        QCOMPARE(settings.size(), 0.5);
        QCOMPARE(settings.magnification(), 0.5);
        QCOMPARE(settings.position(), QStringLiteral("bottom"));
        QCOMPARE(settings.autohide(), false);
        QCOMPARE(settings.animateOpening(), true);
        QCOMPARE(settings.showIndicators(), true);
        QCOMPARE(settings.minimizeIntoTileIcon(), false);
        QCOMPARE(settings.minimizedAnimation(), QStringLiteral("scale"));
        QCOMPARE(settings.titlebarDoubleClick(), QStringLiteral("zoom"));
        QCOMPARE(settings.showRecentApps(), false);
        QCOMPARE(settings.reduceMotion(), false);
    }

    void settingsRoundTripAndValidate()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString path = dir.path() + QStringLiteral("/dragonfruit/settings.json");

        DockSettings settings(path);
        settings.setSize(2.0);          // clamps to 1.0
        settings.setMagnification(-1.0); // clamps to 0.0
        settings.setPosition(QStringLiteral("sideways")); // -> bottom
        settings.setAutohide(true);
        settings.setAnimateOpening(false);
        settings.setShowIndicators(false);
        settings.setMinimizeIntoTileIcon(true);
        settings.setMinimizedAnimation(QStringLiteral("warp")); // -> scale
        settings.setTitlebarDoubleClick(QStringLiteral("fill")); // -> zoom
        settings.setShowRecentApps(true);
        settings.setReduceMotion(true);
        QVERIFY(settings.save());

        DockSettings reloaded(path);
        QVERIFY(reloaded.load());
        QCOMPARE(reloaded.size(), 1.0);
        QCOMPARE(reloaded.magnification(), 0.0);
        QCOMPARE(reloaded.position(), QStringLiteral("bottom"));
        QCOMPARE(reloaded.autohide(), true);
        QCOMPARE(reloaded.animateOpening(), false);
        QCOMPARE(reloaded.showIndicators(), false);
        QCOMPARE(reloaded.minimizeIntoTileIcon(), true);
        QCOMPARE(reloaded.minimizedAnimation(), QStringLiteral("scale"));
        QCOMPARE(reloaded.titlebarDoubleClick(), QStringLiteral("zoom"));
        QCOMPARE(reloaded.showRecentApps(), true);
        QCOMPARE(reloaded.reduceMotion(), true);

        // A malformed file keeps the defaults and reports why.
        writeFile(path, QStringLiteral("{ not json"));
        DockSettings broken(path);
        QVERIFY(!broken.load());
        QVERIFY(!broken.lastError().isEmpty());
        QCOMPARE(broken.size(), 0.5);
    }

    void settingsAndPinsShareTheFileWithoutClobbering()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString path = dir.path() + QStringLiteral("/dragonfruit/settings.json");

        // The settings write lands first, then a pin write; both survive.
        DockSettings settings(path);
        settings.setMagnification(0.8);
        QVERIFY(settings.save());
        DockPins pins(path);
        QVERIFY(pins.load());
        QVERIFY(pins.add(QStringLiteral("a.desktop")));
        QVERIFY(pins.save());

        DockSettings reloadedSettings(path);
        QVERIFY(reloadedSettings.load());
        QCOMPARE(reloadedSettings.magnification(), 0.8);
        DockPins reloadedPins(path);
        QVERIFY(reloadedPins.load());
        QCOMPARE(reloadedPins.ids(), QStringList{QStringLiteral("a.desktop")});

        // And the reverse order.
        DockSettings second(path);
        QVERIFY(second.load());
        second.setAutohide(true);
        QVERIFY(second.save());
        DockPins third(path);
        QVERIFY(third.load());
        QVERIFY(third.add(QStringLiteral("b.desktop")));
        QVERIFY(third.save());
        DockSettings finalSettings(path);
        QVERIFY(finalSettings.load());
        QCOMPARE(finalSettings.autohide(), true);
        QCOMPARE(finalSettings.magnification(), 0.8);
    }

    void settingsEqualsDetectsAChange()
    {
        DockSettings a(QStringLiteral("/nonexistent/settings.json"));
        DockSettings b(QStringLiteral("/nonexistent/settings.json"));
        QVERIFY(a.equals(b));
        b.setAutohide(true);
        QVERIFY(!a.equals(b));
    }

    // -- entry merge -----------------------------------------------------

    void mergePinsAndRunning()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString apps = makeAppDir(dir);
        writeFile(apps + QStringLiteral("/org.dragonfruit.Files.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Files\nExec=df-files\n"));
        writeFile(apps + QStringLiteral("/org.dragonfruit.Settings.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Settings\nExec=df-settings\n"));
        writeFile(apps + QStringLiteral("/org.mozilla.firefox.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Firefox\nExec=firefox\n"));

        DesktopEntryIndex index;
        index.scan({apps});

        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("files")},
                        {QStringLiteral("appId"), QStringLiteral("org.dragonfruit.Files")},
                        {QStringLiteral("name"), QStringLiteral("Files")},
                        {QStringLiteral("kind"), QStringLiteral("temporary")},
                        {QStringLiteral("running"), true},
                        {QStringLiteral("windows"), 2},
                        {QStringLiteral("windowList"),
                         QVariantList{QVariantMap{{QStringLiteral("windowId"), QStringLiteral("1")},
                                                  {QStringLiteral("title"), QStringLiteral("A")}},
                                      QVariantMap{{QStringLiteral("windowId"), QStringLiteral("2")},
                                                  {QStringLiteral("title"), QStringLiteral("B")}}}}},
            QVariantMap{{QStringLiteral("id"), QStringLiteral("win:1")},
                        {QStringLiteral("appId"), QStringLiteral("org.dragonfruit.Files")},
                        {QStringLiteral("name"), QStringLiteral("Doc")},
                        {QStringLiteral("kind"), QStringLiteral("minimized")}},
        };
        const QStringList pinned{QStringLiteral("org.dragonfruit.Files.desktop"),
                                 QStringLiteral("org.dragonfruit.Settings.desktop"),
                                 QStringLiteral("org.dragonfruit.Missing.desktop")};
        QHash<QString, QString> launchStates;
        launchStates.insert(QStringLiteral("org.dragonfruit.Settings.desktop"),
                            QStringLiteral("launching"));

        const QVariantList entries =
            buildDockEntries(pinned, index, running, launchStates);
        // 3 pinned + 1 minimized (the running Files merged into its pin).
        QCOMPARE(entries.size(), 4);

        const QVariantMap files = entries.at(0).toMap();
        QCOMPARE(files.value(QStringLiteral("kind")).toString(), QStringLiteral("pinned"));
        QCOMPARE(files.value(QStringLiteral("name")).toString(), QStringLiteral("Files"));
        QCOMPARE(files.value(QStringLiteral("running")).toBool(), true);
        QCOMPARE(files.value(QStringLiteral("windows")).toInt(), 2);
        // The per-window list survives the pin merge for the chooser.
        const QVariantList filesWindows = files.value(QStringLiteral("windowList")).toList();
        QCOMPARE(filesWindows.size(), 2);
        QCOMPARE(filesWindows.at(0).toMap().value(QStringLiteral("windowId")).toString(),
                 QStringLiteral("1"));
        QCOMPARE(files.value(QStringLiteral("appId")).toString(),
                 QStringLiteral("org.dragonfruit.Files"));

        const QVariantMap settings = entries.at(1).toMap();
        QCOMPARE(settings.value(QStringLiteral("running")).toBool(), false);
        QCOMPARE(settings.value(QStringLiteral("launch")).toString(),
                 QStringLiteral("launching"));

        const QVariantMap missing = entries.at(2).toMap();
        QCOMPARE(missing.value(QStringLiteral("missing")).toBool(), true);
        QCOMPARE(missing.value(QStringLiteral("name")).toString(), QStringLiteral("Missing"));

        QCOMPARE(entries.at(3).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("minimized"));
    }

    void mergeKeepsUnpinnedRunningAsTemporary()
    {
        DesktopEntryIndex index; // empty
        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("term")},
                        {QStringLiteral("appId"), QStringLiteral("org.example.Terminal")},
                        {QStringLiteral("name"), QStringLiteral("Terminal")},
                        {QStringLiteral("kind"), QStringLiteral("temporary")},
                        {QStringLiteral("running"), true},
                        {QStringLiteral("windows"), 1}},
        };
        const QVariantList entries =
            buildDockEntries({}, index, running, QHash<QString, QString>());
        QCOMPARE(entries.size(), 1);
        QCOMPARE(entries.at(0).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("temporary"));
    }

    void mergeTemporaryCarriesDesktopIdForPromotion()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString apps = makeAppDir(dir);
        writeFile(apps + QStringLiteral("/org.example.Terminal.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Terminal\nExec=term\n"));

        DesktopEntryIndex index;
        index.scan({apps});
        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("term")},
                        {QStringLiteral("appId"), QStringLiteral("org.example.Terminal")},
                        {QStringLiteral("name"), QStringLiteral("Terminal")},
                        {QStringLiteral("kind"), QStringLiteral("temporary")},
                        {QStringLiteral("running"), true},
                        {QStringLiteral("windows"), 1}},
        };
        const QVariantList entries =
            buildDockEntries({}, index, running, QHash<QString, QString>());
        QCOMPARE(entries.size(), 1);
        const QVariantMap entry = entries.at(0).toMap();
        // The drag "Keep in Dock" promotion needs the resolved desktop id.
        QCOMPARE(entry.value(QStringLiteral("desktopId")).toString(),
                 QStringLiteral("org.example.Terminal.desktop"));
    }

    // -- trash monitor ---------------------------------------------------

    void trashMonitorReadsEmptyAndFull()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        QVERIFY(QDir().mkpath(root + QStringLiteral("/info")));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));

        TrashMonitor monitor(root);
        monitor.start();
        QVERIFY(!monitor.isFull());
        QCOMPARE(monitor.itemCount(), 0);

        writeFile(root + QStringLiteral("/info/foo.txt.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/foo.txt\n"));
        writeFile(root + QStringLiteral("/files/foo.txt"), QStringLiteral("hello"));
        monitor.refresh();
        QVERIFY(monitor.isFull());
        QCOMPARE(monitor.itemCount(), 1);

        // A payload with no info record still counts (partial state).
        writeFile(root + QStringLiteral("/files/bar.txt"), QStringLiteral("bar"));
        monitor.refresh();
        QCOMPARE(monitor.itemCount(), 2);

        // Removing the info record and payload drops the count back.
        QVERIFY(QFile::remove(root + QStringLiteral("/info/foo.txt.trashinfo")));
        QVERIFY(QFile::remove(root + QStringLiteral("/files/foo.txt")));
        monitor.refresh();
        QCOMPARE(monitor.itemCount(), 1);
    }

    void trashMonitorWatchesForThirdPartyChanges()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        QVERIFY(QDir().mkpath(root + QStringLiteral("/info")));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));

        TrashMonitor monitor(root);
        QSignalSpy spy(&monitor, &TrashMonitor::changed);
        QVERIFY(spy.isValid());
        monitor.start();

        // A deletion by another application appears as a new info record.
        writeFile(root + QStringLiteral("/info/third.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/third\n"));
        QTRY_COMPARE(monitor.itemCount(), 1);
        QVERIFY(spy.count() >= 1);
    }

    void trashMonitorEmptyRemovesItems()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        QVERIFY(QDir().mkpath(root + QStringLiteral("/info")));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));
        writeFile(root + QStringLiteral("/info/a.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/a\n"));
        writeFile(root + QStringLiteral("/files/a"), QStringLiteral("a"));
        writeFile(root + QStringLiteral("/info/b.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/b\n"));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files/b")));
        writeFile(root + QStringLiteral("/files/b/nested"), QStringLiteral("n"));

        TrashMonitor monitor(root);
        monitor.start();
        QCOMPARE(monitor.itemCount(), 2);

        const int removed = monitor.empty();
        QCOMPARE(removed, 4); // 2 info records + a + the b tree
        QVERIFY(!monitor.isFull());
        QCOMPARE(monitor.itemCount(), 0);
        QVERIFY(QDir(root + QStringLiteral("/files")).entryList(
                    QDir::AllEntries | QDir::NoDotAndDotDot).isEmpty());
        QVERIFY(QDir(root + QStringLiteral("/info")).entryList(
                    QDir::AllEntries | QDir::NoDotAndDotDot).isEmpty());
    }

    void trashMonitorRefusesUnsafeRoot()
    {
        TrashMonitor monitor(QStringLiteral("/"));
        QCOMPARE(monitor.empty(), -1);
        QVERIFY(!monitor.lastError().isEmpty());
    }

    void trashMonitorTrashMovesFilesAndWritesInfo()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        const QString source = dir.path() + QStringLiteral("/note.txt");
        writeFile(source, QStringLiteral("hello"));

        TrashMonitor monitor(root);
        monitor.start();
        QCOMPARE(monitor.itemCount(), 0);

        const int trashed = monitor.trash(QStringList{source});
        QCOMPARE(trashed, 1);
        QVERIFY(monitor.isFull());
        QCOMPARE(monitor.itemCount(), 1);
        QVERIFY(!QFileInfo::exists(source));
        QVERIFY(QFileInfo::exists(root + QStringLiteral("/files/note.txt")));

        QFile record(root + QStringLiteral("/info/note.txt.trashinfo"));
        QVERIFY(record.open(QIODevice::ReadOnly));
        const QString contents = QString::fromUtf8(record.readAll());
        QVERIFY(contents.startsWith(QStringLiteral("[Trash Info]\n")));
        QVERIFY(contents.contains(QStringLiteral("DeletionDate=")));
        // The original path is percent-encoded in the record.
        const QString encoded = QString::fromLatin1(QUrl::toPercentEncoding(source));
        QVERIFY(contents.contains(QStringLiteral("Path=") + encoded));

        // A second item with the same basename gets a unique name.
        writeFile(source, QStringLiteral("again"));
        QCOMPARE(monitor.trash(QStringList{source}), 1);
        QCOMPARE(monitor.itemCount(), 2);
        QVERIFY(QFileInfo::exists(root + QStringLiteral("/files/note.1.txt")));
    }

    void trashMonitorRefusesTrashingItself()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));
        TrashMonitor monitor(root);
        monitor.start();
        QCOMPARE(monitor.trash(QStringList{root + QStringLiteral("/files")}), 0);
        QVERIFY(!monitor.lastError().isEmpty());
    }

    // -- external drops --------------------------------------------------

    void parseUriListDecodesFileUris()
    {
        const QByteArray payload =
            "# comment\r\n"
            "file:///home/x/My%20File.txt\r\n"
            "\r\n"
            "https://example.com/not-a-file\r\n"
            "file:///home/x/plain\r\n";
        const QStringList paths = parseUriList(payload);
        QCOMPARE(paths, (QStringList{QStringLiteral("/home/x/My File.txt"),
                                     QStringLiteral("/home/x/plain")}));
    }

    void uriListClassifiesApplicationAlias()
    {
        QVERIFY(uriListIsApplication(QStringList{QStringLiteral("/tmp/org.example.App.desktop")}));
        QVERIFY(!uriListIsApplication(QStringList{QStringLiteral("/tmp/readme.txt")}));
        QVERIFY(!uriListIsApplication(
            QStringList{QStringLiteral("/tmp/a.desktop"), QStringLiteral("/tmp/b.desktop")}));
        QCOMPARE(desktopIdForFile(QStringLiteral("/tmp/org.example.App.desktop")),
                 QStringLiteral("org.example.App.desktop"));
        QCOMPARE(desktopIdForFile(QStringLiteral("/tmp/readme.txt")), QString());
    }

    void dropActionsFollowTargetAndPayload()
    {
        using Payload = DockDropPayload;
        using Action = DockDropAction;
        // An app alias pins on the app region / empty Dock, no-op elsewhere.
        QCOMPARE(dockDropActionFor(QString(), Payload::Application), Action::PinApp);
        QCOMPARE(dockDropActionFor(QStringLiteral("pinned"), Payload::Application),
                 Action::PinApp);
        QCOMPARE(dockDropActionFor(QStringLiteral("temporary"), Payload::Application),
                 Action::PinApp);
        QCOMPARE(dockDropActionFor(QStringLiteral("trash"), Payload::Application),
                 Action::None);
        QCOMPARE(dockDropActionFor(QStringLiteral("divider"), Payload::Application),
                 Action::None);
        // Files follow the target.
        QCOMPARE(dockDropActionFor(QStringLiteral("pinned"), Payload::Files),
                 Action::OpenWithApp);
        QCOMPARE(dockDropActionFor(QStringLiteral("temporary"), Payload::Files),
                 Action::OpenWithApp);
        QCOMPARE(dockDropActionFor(QStringLiteral("trash"), Payload::Files),
                 Action::TrashFiles);
        QCOMPARE(dockDropActionFor(QStringLiteral("stack"), Payload::Files),
                 Action::MoveToDownloads);
        QCOMPARE(dockDropActionFor(QStringLiteral("divider"), Payload::Files), Action::None);
        QCOMPARE(dockDropActionFor(QStringLiteral("minimized"), Payload::Files), Action::None);
    }

    // -- downloads monitor (T-10 section 17) -----------------------------

    void downloadsMonitorListsAndBadgesNewItems()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString downloads = dir.path() + QStringLiteral("/Downloads");
        QVERIFY(QDir().mkpath(downloads));
        writeFile(downloads + QStringLiteral("/existing.txt"), QStringLiteral("x"));

        DownloadsMonitor monitor(downloads);
        monitor.start();
        QCOMPARE(monitor.itemCount(), 1);
        // A pre-existing item is not a badge on login.
        QCOMPARE(monitor.newCount(), 0);

        writeFile(downloads + QStringLiteral("/fresh.txt"), QStringLiteral("y"));
        QTRY_COMPARE(monitor.itemCount(), 2);
        QTRY_COMPARE(monitor.newCount(), 1);

        const QVariantList items = monitor.items();
        QCOMPARE(items.size(), 2);
        bool foundFresh = false;
        for (const QVariant &value : items) {
            const QVariantMap item = value.toMap();
            QVERIFY(item.value(QStringLiteral("path")).toString().startsWith(downloads));
            if (item.value(QStringLiteral("name")).toString() == QStringLiteral("fresh.txt"))
                foundFresh = true;
        }
        QVERIFY(foundFresh);

        monitor.markSeen();
        QCOMPARE(monitor.newCount(), 0);
    }

    void downloadsMonitorMoveInMovesFiles()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString downloads = dir.path() + QStringLiteral("/Downloads");
        const QString source = dir.path() + QStringLiteral("/note.txt");
        writeFile(source, QStringLiteral("hello"));

        DownloadsMonitor monitor(downloads);
        monitor.start();
        QCOMPARE(monitor.itemCount(), 0);

        QCOMPARE(monitor.moveIn(QStringList{source}), 1);
        QVERIFY(!QFileInfo::exists(source));
        QVERIFY(QFileInfo::exists(downloads + QStringLiteral("/note.txt")));
        QCOMPARE(monitor.itemCount(), 1);

        // Moving a file already in the folder is refused, never recursive.
        QCOMPARE(monitor.moveIn(QStringList{downloads + QStringLiteral("/note.txt")}), 0);
        QVERIFY(!monitor.lastError().isEmpty());
    }

    void downloadsMonitorRefusesUnsafeRoot()
    {
        DownloadsMonitor monitor(QStringLiteral("/"));
        QCOMPARE(monitor.moveIn(QStringList{QStringLiteral("/tmp/whatever")}), 0);
        QVERIFY(!monitor.lastError().isEmpty());
    }

    // -- recent/suggested apps (T-10 section 17) -------------------------

    void recentEntriesSkipPinnedRunningAndMisses()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString apps = makeAppDir(dir);
        writeFile(apps + QStringLiteral("/a.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Alpha\nExec=a\n"));
        writeFile(apps + QStringLiteral("/b.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Beta\nExec=b\n"));
        writeFile(apps + QStringLiteral("/c.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Gamma\nExec=c\n"));
        writeFile(apps + QStringLiteral("/d.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Delta\nExec=d\n"));

        DesktopEntryIndex index;
        index.scan({apps});

        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("b")},
                        {QStringLiteral("appId"), QStringLiteral("b")},
                        {QStringLiteral("kind"), QStringLiteral("temporary")},
                        {QStringLiteral("running"), true}},
        };
        // Recency order: running B, pinned A, C, D, then an unresolved miss.
        const QVariantList recents = buildRecentEntries(
            {QStringLiteral("b"), QStringLiteral("a"), QStringLiteral("c"),
             QStringLiteral("d"), QStringLiteral("gone")},
            {QStringLiteral("a.desktop")}, running, index);
        QCOMPARE(recents.size(), 2);
        QCOMPARE(recents.at(0).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("recent"));
        QCOMPARE(recents.at(0).toMap().value(QStringLiteral("desktopId")).toString(),
                 QStringLiteral("c.desktop"));
        QCOMPARE(recents.at(0).toMap().value(QStringLiteral("running")).toBool(), false);
        QCOMPARE(recents.at(1).toMap().value(QStringLiteral("desktopId")).toString(),
                 QStringLiteral("d.desktop"));

        // The limit is honored.
        const QVariantList limited = buildRecentEntries(
            {QStringLiteral("c"), QStringLiteral("d")}, {}, {}, index, 1);
        QCOMPARE(limited.size(), 1);
        QCOMPARE(limited.at(0).toMap().value(QStringLiteral("desktopId")).toString(),
                 QStringLiteral("c.desktop"));
    }

    void mergeAppendsRecentsBeforeMinimized()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString apps = makeAppDir(dir);
        writeFile(apps + QStringLiteral("/pin.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Pin\nExec=pin\n"));
        writeFile(apps + QStringLiteral("/recent.desktop"),
                  QStringLiteral("[Desktop Entry]\nName=Recent\nExec=recent\n"));

        DesktopEntryIndex index;
        index.scan({apps});
        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("win:1")},
                        {QStringLiteral("appId"), QStringLiteral("pin")},
                        {QStringLiteral("name"), QStringLiteral("Doc")},
                        {QStringLiteral("kind"), QStringLiteral("minimized")}},
        };
        const QVariantList entries = buildDockEntries(
            {QStringLiteral("pin.desktop")}, index, running, {}, {QStringLiteral("recent")});
        QCOMPARE(entries.size(), 3);
        QCOMPARE(entries.at(0).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("pinned"));
        QCOMPARE(entries.at(1).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("recent"));
        QCOMPARE(entries.at(1).toMap().value(QStringLiteral("desktopId")).toString(),
                 QStringLiteral("recent.desktop"));
        QCOMPARE(entries.at(2).toMap().value(QStringLiteral("kind")).toString(),
                 QStringLiteral("minimized"));
    }

    // -- bounce clocks ---------------------------------------------------

    void launchBounceIsThreeHopsThenDone()
    {
        // Hop phase 0 at the start and at each hop boundary, peak mid-hop.
        QCOMPARE(dockLaunchBouncePhase(0), 0.0);
        QVERIFY(qAbs(dockLaunchBouncePhase(100) - 0.5) < 1e-9);
        QCOMPARE(dockLaunchBouncePhase(200), 0.0);
        QVERIFY(dockLaunchBouncePhase(300) > 0.4);
        // The third hop ends exactly at the total duration.
        QCOMPARE(dockLaunchBouncePhase(kLaunchBounceMs), -1.0);
        QCOMPARE(dockLaunchBouncePhase(kLaunchBounceMs + 100), -1.0);
        // A negative clock is never a valid bounce.
        QCOMPARE(dockLaunchBouncePhase(-1), -1.0);
    }

    void attentionBounceRepeats()
    {
        QCOMPARE(dockAttentionBouncePhase(0), 0.0);
        // The hop phase peaks at the half-period; the caller's sin(pi*phase)
        // turns that into the full attention amplitude.
        QVERIFY(qAbs(dockAttentionBouncePhase(kAttentionBounceHopMs / 2) - 0.5) < 1e-9);
        // One full period wraps back to the start of the next hop.
        QCOMPARE(dockAttentionBouncePhase(kAttentionBounceHopMs), 0.0);
        QVERIFY(qAbs(dockAttentionBouncePhase(kAttentionBounceHopMs + kAttentionBounceHopMs / 2)
                     - 0.5) < 1e-9);
    }

    // -- scene-graph frame gate (FR-14) ----------------------------------

    void frameGateSchedulesEachSceneGraphFrame()
    {
        FrameCommitGate gate;
        // A rendered scene-graph frame schedules a commit; the readback's own
        // re-render (bracketed by begin/endCommit) must not schedule another.
        QVERIFY(gate.frameRendered());
        gate.beginCommit();
        QVERIFY(gate.committing());
        QVERIFY(!gate.frameRendered());
        gate.endCommit();
        QVERIFY(!gate.committing());
        QVERIFY(gate.frameRendered());
        QCOMPARE(gate.sceneFrames(), quint64(3));
        QCOMPARE(gate.committedFrames(), quint64(1));
    }

    void frameGateResetClearsCounters()
    {
        FrameCommitGate gate;
        gate.frameRendered();
        gate.beginCommit();
        gate.endCommit();
        gate.reset();
        QVERIFY(!gate.committing());
        QCOMPARE(gate.sceneFrames(), quint64(0));
        QCOMPARE(gate.committedFrames(), quint64(0));
        QVERIFY(gate.frameRendered());
    }
};

QTEST_GUILESS_MAIN(TestDockCore)
#include "tst_dockcore.moc"
