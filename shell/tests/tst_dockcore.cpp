// SPDX-License-Identifier: MIT
// Dock core unit tests (T-10): the interim `.desktop` resolver/launcher, the
// `dock.pinned` persistence, and the pure pinned+running entry merge. Runs
// headless with no compositor, Wayland, or QML.
#include "controlcenterpolicy.h"
#include "desktopentry.h"
#include "dockdrops.h"
#include "dockmodel.h"
#include "dockprojection.h"
#include "downloadsmonitor.h"
#include "filestarget.h"
#include "focusstatus.h"
#include "framecommitgate.h"
#include "launchfailure.h"
#include "notificationclient.h"
#include "settingsclient.h"
#include "trashbridge.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>
#include <QUrl>

#include <cstdlib>

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

QVariantList makeOverflowEntries(int pinned, int temporary, int recent, int minimized)
{
    QVariantList entries;
    const auto add = [&entries](const QString &kind, const QString &id) {
        entries.append(QVariantMap{{QStringLiteral("id"), id},
                                   {QStringLiteral("kind"), kind},
                                   {QStringLiteral("running"),
                                    kind != QLatin1String("recent")}});
    };
    for (int i = 0; i < pinned; ++i)
        add(QStringLiteral("pinned"), QStringLiteral("pin%1").arg(i));
    for (int i = 0; i < temporary; ++i)
        add(QStringLiteral("temporary"), QStringLiteral("tmp%1").arg(i));
    for (int i = 0; i < recent; ++i)
        add(QStringLiteral("recent"), QStringLiteral("recent%1").arg(i));
    for (int i = 0; i < minimized; ++i)
        add(QStringLiteral("minimized"), QStringLiteral("min%1").arg(i));
    return entries;
}

int countKind(const QVariantList &entries, const QString &kind)
{
    int count = 0;
    for (const QVariant &value : entries) {
        if (value.toMap().value(QStringLiteral("kind")).toString() == kind)
            ++count;
    }
    return count;
}

// The projection's app entry `appId` (`__unknown__` for an empty id).
QVariantMap entryForAppId(const QVariantList &entries, const QString &appId)
{
    const QString key = appId.isEmpty() ? QStringLiteral("__unknown__") : appId;
    for (const QVariant &value : entries) {
        const QVariantMap map = value.toMap();
        if (map.value(QStringLiteral("id")).toString() == key)
            return map;
    }
    return {};
}

DockWindow window(quintptr id, const QString &appId, const QString &title,
                  bool minimized = false, bool focused = false, int workspace = -1,
                  const QString &workspaceName = {})
{
    DockWindow w;
    w.windowId = id;
    w.appId = appId;
    w.title = title;
    w.minimized = minimized;
    w.focused = focused;
    w.workspaceIndex = workspace;
    w.workspaceName = workspaceName;
    return w;
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

    // -- reveal target (T-10.6c) -----------------------------------------

    void revealExecutableResolvesAnAbsoluteProgram()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString program = dir.path() + QStringLiteral("/df-files");
        {
            QFile handle(program);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write("#!/bin/sh\n");
        }
        QVERIFY(QFile::setPermissions(
            program, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                         | QFileDevice::ExeOwner));

        DesktopEntry entry;
        entry.id = QStringLiteral("app.desktop");
        entry.name = QStringLiteral("App");
        entry.exec = program;
        entry.valid = true;
        QCOMPARE(revealExecutable(entry),
                 QFileInfo(program).absoluteFilePath());
    }

    void revealExecutableFindsAProgramOnPath()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString program = dir.path() + QStringLiteral("/df-tool");
        {
            QFile handle(program);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write("#!/bin/sh\n");
        }
        QVERIFY(QFile::setPermissions(
            program, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                         | QFileDevice::ExeOwner));

        const QByteArray previous = qgetenv("PATH");
        qputenv("PATH", dir.path().toUtf8() + ':' + previous);
        DesktopEntry entry;
        entry.id = QStringLiteral("tool.desktop");
        entry.name = QStringLiteral("Tool");
        entry.exec = QStringLiteral("df-tool --flag");
        entry.valid = true;
        const QString revealed = revealExecutable(entry);
        qputenv("PATH", previous);
        QCOMPARE(revealed, QFileInfo(program).absoluteFilePath());
    }

    void revealExecutableIsEmptyWhenItCannotResolve()
    {
        DesktopEntry entry;
        entry.id = QStringLiteral("ghost.desktop");
        entry.name = QStringLiteral("Ghost");
        entry.exec = QStringLiteral("df-definitely-not-on-path");
        entry.valid = true;
        QVERIFY(revealExecutable(entry).isEmpty());
    }

    // -- default pins and the typed settings view (T-08.2a) --------------

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
        QCOMPARE(resolveDefaultDockPins(index),
                 (QStringList{QStringLiteral("org.dragonfruit.Files.desktop"),
                              QStringLiteral("org.dragonfruit.Settings.desktop"),
                              QStringLiteral("term.desktop"),
                              QStringLiteral("browse.desktop")}));
    }

    void dockConfigReadsTheSchemaDefaults()
    {
        const DockConfig config = dockConfigFromValues(settingsSchemaDefaults());
        QCOMPARE(config.size, 0.5);
        QCOMPARE(config.magnification, 0.5);
        QCOMPARE(config.position, QStringLiteral("bottom"));
        QCOMPARE(config.autohide, false);
        QCOMPARE(config.animateOpening, true);
        QCOMPARE(config.showIndicators, true);
        QCOMPARE(config.minimizeIntoTileIcon, false);
        QCOMPARE(config.minimizedAnimation, QStringLiteral("scale"));
        QCOMPARE(config.titlebarDoubleClick, QStringLiteral("zoom"));
        QCOMPARE(config.showRecentApps, false);
        QCOMPARE(config.reduceMotion, false);
        QCOMPARE(config.pinned, QStringList());
    }

    void dockConfigReadsOverridesAndFallsBackOnMissingKeys()
    {
        QVariantMap values;
        values.insert(QStringLiteral("dock.size"), 0.9);
        values.insert(QStringLiteral("dock.autohide"), true);
        values.insert(QStringLiteral("dock.position"), QStringLiteral("left"));
        values.insert(QStringLiteral("accessibility.reduceMotion"), true);
        const DockConfig config = dockConfigFromValues(values);
        QCOMPARE(config.size, 0.9);
        QCOMPARE(config.autohide, true);
        QCOMPARE(config.position, QStringLiteral("left"));
        QCOMPARE(config.reduceMotion, true);
        // Keys absent from the map keep the schema default.
        QCOMPARE(config.magnification, 0.5);
        QCOMPARE(config.showRecentApps, false);
    }

    void dockIconSizeMapsTheRange()
    {
        QCOMPARE(dockIconSize(0.0, 32, 64), 32);
        QCOMPARE(dockIconSize(1.0, 32, 64), 64);
        QCOMPARE(dockIconSize(0.5, 32, 64), 48);
        // Out-of-range input is clamped to the token range.
        QCOMPARE(dockIconSize(-1.0, 32, 64), 32);
        QCOMPARE(dockIconSize(2.0, 32, 64), 64);
    }

    void settingsChangeReLaysOutTheDock()
    {
        // The headless half of the T-08.2a acceptance: a `dock.size` change
        // re-lays-out the Dock. At a fixed output length the small setting
        // keeps the minimum icon and fits every entry; the large one clamps
        // the icon up to the largest that fits.
        const QVariantList entries = makeOverflowEntries(4, 3, 2, 0);
        const DockOverflowResult smallFit =
            applyDockOverflow(entries, 500, dockIconSize(0.0, 32, 64), 32, 64, 6, 1);
        const DockOverflowResult largeFit =
            applyDockOverflow(entries, 500, dockIconSize(1.0, 32, 64), 32, 64, 6, 1);
        QCOMPARE(smallFit.iconSize, 32);
        QVERIFY(largeFit.iconSize > smallFit.iconSize);
        QVERIFY(largeFit.iconSize <= 64);
        QVERIFY(!smallFit.clamped);
        // Both keep every pinned entry; the layout (not the entry set) changed.
        QCOMPARE(countKind(largeFit.entries, QStringLiteral("pinned")), 4);
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

    // -- running-app projection (section 22 lifecycle matrix) ------------

    void projectionGroupsByAppAndLeadsWithFocused()
    {
        const QVariantList entries = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("one")),
            window(2, QStringLiteral("org.example.Alpha"), QStringLiteral("two"), false, true),
            window(3, QStringLiteral("org.example.Beta"), QStringLiteral("beta")),
        });

        QCOMPARE(countKind(entries, QStringLiteral("temporary")), 2);
        const QVariantMap alpha = entryForAppId(entries, QStringLiteral("org.example.Alpha"));
        QCOMPARE(alpha.value(QStringLiteral("name")).toString(), QStringLiteral("Alpha"));
        QCOMPARE(alpha.value(QStringLiteral("windows")).toInt(), 2);
        QCOMPARE(alpha.value(QStringLiteral("minimized")).toBool(), false);
        const QVariantList alphaWindows = alpha.value(QStringLiteral("windowList")).toList();
        QCOMPARE(alphaWindows.size(), 2);
        // The focused window leads its app's list (the chooser checkmark).
        QCOMPARE(alphaWindows.at(0).toMap().value(QStringLiteral("title")).toString(),
                 QStringLiteral("two"));
        QCOMPARE(alphaWindows.at(1).toMap().value(QStringLiteral("title")).toString(),
                 QStringLiteral("one"));

        const QVariantMap beta = entryForAppId(entries, QStringLiteral("org.example.Beta"));
        QCOMPARE(beta.value(QStringLiteral("windows")).toInt(), 1);
    }

    void projectionMinimizedFlagOnlyWhenAllWindowsMinimized()
    {
        const QVariantList entries = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("a1"), true),
            window(2, QStringLiteral("org.example.Alpha"), QStringLiteral("a2"), false),
            window(3, QStringLiteral("org.example.Beta"), QStringLiteral("b1"), true),
        });

        // Alpha still has a visible window: its app entry is not minimized,
        // but the minimized window still gets its own row.
        const QVariantMap alpha = entryForAppId(entries, QStringLiteral("org.example.Alpha"));
        QCOMPARE(alpha.value(QStringLiteral("minimized")).toBool(), false);
        QCOMPARE(alpha.value(QStringLiteral("windowList")).toList().size(), 2);
        QCOMPARE(countKind(entries, QStringLiteral("minimized")), 2);

        // Beta's only window is minimized: the app entry reports minimized.
        const QVariantMap beta = entryForAppId(entries, QStringLiteral("org.example.Beta"));
        QCOMPARE(beta.value(QStringLiteral("minimized")).toBool(), true);

        // The minimized entry carries its app's full window list so the menu
        // works from it (section 13).
        for (const QVariant &value : entries) {
            const QVariantMap map = value.toMap();
            if (map.value(QStringLiteral("kind")).toString() != QLatin1String("minimized"))
                continue;
            if (map.value(QStringLiteral("appId")).toString() == QLatin1String("org.example.Alpha"))
                QCOMPARE(map.value(QStringLiteral("windowList")).toList().size(), 2);
        }
    }

    void projectionIdentityChangeRehomesAndMerges()
    {
        // Before the change: two apps, one window each.
        const QVariantList before = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("a")),
            window(2, QStringLiteral("org.example.Beta"), QStringLiteral("b")),
        });
        QCOMPARE(countKind(before, QStringLiteral("temporary")), 2);

        // window 2's app_id changes to Alpha: its row must move into the
        // Alpha entry and no stale Beta entry may remain.
        const QVariantList after = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("a")),
            window(2, QStringLiteral("org.example.Alpha"), QStringLiteral("b")),
        });
        QCOMPARE(countKind(after, QStringLiteral("temporary")), 1);
        const QVariantMap alpha = entryForAppId(after, QStringLiteral("org.example.Alpha"));
        QCOMPARE(alpha.value(QStringLiteral("windows")).toInt(), 2);
        QVERIFY(entryForAppId(after, QStringLiteral("org.example.Beta")).isEmpty());
    }

    void projectionCarriesTheWorkspaceForCrossSpaceWindows()
    {
        const QVariantList entries = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("a"), false, false,
                   2, QStringLiteral("Code")),
            window(2, QStringLiteral("org.example.Alpha"), QStringLiteral("b"), false, false, 0),
        });
        const QVariantList windows =
            entryForAppId(entries, QStringLiteral("org.example.Alpha"))
                .value(QStringLiteral("windowList"))
                .toList();
        QCOMPARE(windows.size(), 2);
        // Newest first: window 2 (Space index 0, a generated name).
        QCOMPARE(windows.at(0).toMap().value(QStringLiteral("workspaceIndex")).toInt(), 0);
        QCOMPARE(windows.at(0).toMap().value(QStringLiteral("workspaceName")).toString(),
                 QStringLiteral("Space 1"));
        QCOMPARE(windows.at(1).toMap().value(QStringLiteral("workspaceIndex")).toInt(), 2);
        QCOMPARE(windows.at(1).toMap().value(QStringLiteral("workspaceName")).toString(),
                 QStringLiteral("Code"));
    }

    void projectionRapidOpenCloseHasNoStaleRows()
    {
        const QVariantList many = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("a")),
            window(2, QStringLiteral("org.example.Alpha"), QStringLiteral("b")),
            window(3, QStringLiteral("org.example.Alpha"), QStringLiteral("c")),
        });
        QCOMPARE(entryForAppId(many, QStringLiteral("org.example.Alpha"))
                     .value(QStringLiteral("windows"))
                     .toInt(),
                 3);

        // Two windows close before the next projection: only the survivor is
        // reported, with no stale window ids or rows.
        const QVariantList after = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("a")),
        });
        const QVariantMap alpha = entryForAppId(after, QStringLiteral("org.example.Alpha"));
        QCOMPARE(alpha.value(QStringLiteral("windows")).toInt(), 1);
        QCOMPARE(alpha.value(QStringLiteral("windowList")).toList().size(), 1);
        QCOMPARE(alpha.value(QStringLiteral("windowList"))
                     .toList()
                     .at(0)
                     .toMap()
                     .value(QStringLiteral("windowId"))
                     .toString(),
                 QStringLiteral("1"));
        QCOMPARE(countKind(after, QStringLiteral("minimized")), 0);
    }

    void projectionLastWindowClosedRemovesTheApp()
    {
        QVERIFY(buildDockProjection({}).isEmpty());
    }

    void projectionUnknownAppIdUsesOneGenericGroup()
    {
        const QVariantList entries = buildDockProjection({
            window(1, QString(), QStringLiteral("no identity")),
            window(2, QString(), QStringLiteral("also no identity")),
        });
        QCOMPARE(countKind(entries, QStringLiteral("temporary")), 1);
        const QVariantMap unknown = entryForAppId(entries, QString());
        QVERIFY(!unknown.isEmpty());
        QCOMPARE(unknown.value(QStringLiteral("appId")).toString(), QString());
        QCOMPARE(unknown.value(QStringLiteral("name")).toString(), QStringLiteral("Unknown"));
        QCOMPARE(unknown.value(QStringLiteral("windows")).toInt(), 2);
    }

    // -- trash bridge (T-10.6a) ------------------------------------------

    // Point the files-core Trash store at a temp dir so a test never touches
    // the real user trash. The Rust side reads XDG_DATA_HOME at construction.
    void trashBridgeReadsEmptyAndFull()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        setenv("XDG_DATA_HOME", dir.path().toUtf8().constData(), 1);
        QVERIFY(QDir().mkpath(root + QStringLiteral("/info")));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));

        TrashBridge trash;
        trash.start();
        QVERIFY(!trash.isFull());
        QCOMPARE(trash.itemCount(), 0);
        QVERIFY(trash.isAvailable());

        writeFile(root + QStringLiteral("/info/foo.txt.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/foo.txt\n"));
        writeFile(root + QStringLiteral("/files/foo.txt"), QStringLiteral("hello"));
        trash.refresh();
        QVERIFY(trash.isFull());
        QCOMPARE(trash.itemCount(), 1);

        QVERIFY(QFile::remove(root + QStringLiteral("/info/foo.txt.trashinfo")));
        QVERIFY(QFile::remove(root + QStringLiteral("/files/foo.txt")));
        trash.refresh();
        QCOMPARE(trash.itemCount(), 0);
    }

    void trashBridgeWatchesForThirdPartyChanges()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        setenv("XDG_DATA_HOME", dir.path().toUtf8().constData(), 1);
        QVERIFY(QDir().mkpath(root + QStringLiteral("/info")));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));

        TrashBridge trash;
        QSignalSpy spy(&trash, &TrashBridge::changed);
        QVERIFY(spy.isValid());
        trash.start();

        // A deletion by another application appears as a new info record and
        // wakes the files-core watch.
        writeFile(root + QStringLiteral("/info/third.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/third\n"));
        QTRY_COMPARE(trash.itemCount(), 1);
        QVERIFY(spy.count() >= 1);
    }

    void trashBridgeTrashAndEmptyRouteThroughFilesCore()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        setenv("XDG_DATA_HOME", dir.path().toUtf8().constData(), 1);
        const QString source = dir.path() + QStringLiteral("/note.txt");
        writeFile(source, QStringLiteral("hello"));

        TrashBridge trash;
        trash.start();
        QCOMPARE(trash.itemCount(), 0);

        QCOMPARE(trash.trash(QStringList{source}), 1);
        QVERIFY(trash.isFull());
        QCOMPARE(trash.itemCount(), 1);
        QVERIFY(!QFileInfo::exists(source));
        QVERIFY(QFileInfo::exists(root + QStringLiteral("/files/note.txt")));

        QFile record(root + QStringLiteral("/info/note.txt.trashinfo"));
        QVERIFY(record.open(QIODevice::ReadOnly));
        const QString contents = QString::fromUtf8(record.readAll());
        QVERIFY(contents.startsWith(QStringLiteral("[Trash Info]\n")));
        QVERIFY(contents.contains(QStringLiteral("DeletionDate=")));

        QCOMPARE(trash.empty(), 1);
        QVERIFY(!trash.isFull());
        QCOMPARE(trash.itemCount(), 0);
        QVERIFY(QDir(root + QStringLiteral("/files")).entryList(
                    QDir::AllEntries | QDir::NoDotAndDotDot).isEmpty());
        QVERIFY(QDir(root + QStringLiteral("/info")).entryList(
                    QDir::AllEntries | QDir::NoDotAndDotDot).isEmpty());
    }

    void trashBridgeCreatesAMissingStoreOnDemand()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        setenv("XDG_DATA_HOME", dir.path().toUtf8().constData(), 1);
        QVERIFY(!QFileInfo::exists(root));

        TrashBridge trash;
        trash.start();
        QVERIFY(trash.isAvailable());
        QVERIFY(!trash.isFull());
        QCOMPARE(trash.itemCount(), 0);
        QVERIFY(QFileInfo::exists(root + QStringLiteral("/info")));
        QVERIFY(QFileInfo::exists(root + QStringLiteral("/files")));
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

    // -- overflow clamp (T-10 section 5.1) --------------------------------

    void overflowFitsLeavesTheLayoutAlone()
    {
        const QVariantList entries = makeOverflowEntries(3, 0, 0, 0);
        const DockOverflowResult result =
            applyDockOverflow(entries, 400, 48, 32, 64, 6, 1);
        QVERIFY(!result.clamped);
        QVERIFY(!result.overflowed);
        QCOMPARE(result.iconSize, 48);
        QCOMPARE(result.hiddenTemporary, 0);
        QCOMPARE(result.hiddenRecent, 0);
        QCOMPARE(result.entries.size(), entries.size());
    }

    void overflowClampsIconSizeDownToFit()
    {
        // 6 items at icon 48 need 5*(48+6)+1 = 271 px; at 250 the icon is
        // clamped to the largest fitting size above the minimum (43 px).
        const QVariantList entries = makeOverflowEntries(3, 0, 0, 0);
        const DockOverflowResult result =
            applyDockOverflow(entries, 250, 48, 32, 64, 6, 1);
        QVERIFY(result.clamped);
        QVERIFY(!result.overflowed);
        QCOMPARE(result.iconSize, 43);
        QCOMPARE(result.hiddenTemporary, 0);
        QCOMPARE(result.hiddenRecent, 0);
        QCOMPARE(result.entries.size(), entries.size());
    }

    void overflowHidesRecentsBeforeTemporaries()
    {
        // 3 pinned + 2 temporary + 1 recent = 9 items; at the minimum icon
        // (32) 9 items need 305 px. At 280 only the recent must go.
        const QVariantList entries = makeOverflowEntries(3, 2, 1, 0);
        const DockOverflowResult one =
            applyDockOverflow(entries, 280, 48, 32, 64, 6, 1);
        QVERIFY(one.clamped);
        QVERIFY(!one.overflowed);
        QCOMPARE(one.iconSize, 32);
        QCOMPARE(one.hiddenRecent, 1);
        QCOMPARE(one.hiddenTemporary, 0);
        QCOMPARE(one.entries.size(), entries.size() - 1);
        // The removed entry is the recent one; every pinned entry survives.
        QCOMPARE(countKind(one.entries, QStringLiteral("pinned")), 3);
        QCOMPARE(countKind(one.entries, QStringLiteral("temporary")), 2);
        QCOMPARE(countKind(one.entries, QStringLiteral("recent")), 0);

        // At 250 both droppable regions shrink: the recent first, then one
        // temporary. Pinned are never dropped.
        const DockOverflowResult two =
            applyDockOverflow(entries, 250, 48, 32, 64, 6, 1);
        QVERIFY(two.clamped);
        QVERIFY(!two.overflowed);
        QCOMPARE(two.iconSize, 32);
        QCOMPARE(two.hiddenRecent, 1);
        QCOMPARE(two.hiddenTemporary, 1);
        QCOMPARE(two.entries.size(), entries.size() - 2);
        QCOMPARE(countKind(two.entries, QStringLiteral("pinned")), 3);
        QCOMPARE(countKind(two.entries, QStringLiteral("temporary")), 1);
        QCOMPARE(countKind(two.entries, QStringLiteral("recent")), 0);
    }

    void overflowNeverDropsPinned()
    {
        // 13 items with no droppable entries: even at the minimum the pinned
        // set overflows, so nothing is hidden and the error is reported.
        const QVariantList entries = makeOverflowEntries(10, 0, 0, 0);
        const DockOverflowResult result =
            applyDockOverflow(entries, 100, 48, 32, 64, 6, 1);
        QVERIFY(result.clamped);
        QVERIFY(result.overflowed);
        QCOMPARE(result.iconSize, 32);
        QCOMPARE(result.hiddenTemporary, 0);
        QCOMPARE(result.hiddenRecent, 0);
        QCOMPARE(result.entries.size(), entries.size());
        QCOMPARE(countKind(result.entries, QStringLiteral("pinned")), 10);
    }

    void overflowHonorsHiddenMinimizedEntries()
    {
        // 3 pinned + 5 minimized: with the minimized region hidden the content
        // fits at 300 px; when it is visible the same layout overflows.
        const QVariantList entries = makeOverflowEntries(3, 0, 0, 5);
        const DockOverflowResult visible =
            applyDockOverflow(entries, 300, 48, 32, 64, 6, 1, 2, true);
        QVERIFY(visible.clamped);
        QCOMPARE(visible.iconSize, 32);
        QVERIFY(visible.overflowed);

        const DockOverflowResult hidden =
            applyDockOverflow(entries, 300, 48, 32, 64, 6, 1, 2, false);
        QVERIFY(!hidden.clamped);
        QVERIFY(!hidden.overflowed);
        QCOMPARE(hidden.iconSize, 48);
    }

    void overflowIgnoresZeroLength()
    {
        // Before the first configure the output length is unknown: no clamp.
        const QVariantList entries = makeOverflowEntries(20, 5, 5, 5);
        const DockOverflowResult result =
            applyDockOverflow(entries, 0, 48, 32, 64, 6, 1);
        QVERIFY(!result.clamped);
        QVERIFY(!result.overflowed);
        QCOMPARE(result.iconSize, 48);
        QCOMPARE(result.entries.size(), entries.size());
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

    // -- launch-failure notification + action round-trip (T-11.1b) --------

    void aLaunchFailureRaisesANotificationWithTheAppAndReason()
    {
        MockNotificationClient client(nullptr, /*seedFixture=*/false);
        QSignalSpy banners(&client, &NotificationClient::bannersChanged);
        QSignalSpy notified(&client, &NotificationClient::notified);

        raiseDockLaunchFailure(&client, QStringLiteral("Files"),
                               QStringLiteral("QProcess::startDetached failed"));

        QCOMPARE(notified.count(), 1);
        QCOMPARE(notified.takeFirst().at(0).toUInt(), 1u);
        QCOMPARE(banners.count(), 1);
        const QJsonArray view =
            QJsonDocument::fromJson(banners.takeFirst().at(0).toByteArray()).array();
        QCOMPARE(view.size(), 1);
        const QJsonObject banner = view.first().toObject();
        QCOMPARE(banner.value(QStringLiteral("appName")).toString(), QStringLiteral("Dock"));
        QCOMPARE(banner.value(QStringLiteral("summary")).toString(),
                 QStringLiteral("Could not launch Files"));
        QCOMPARE(banner.value(QStringLiteral("body")).toString(),
                 QStringLiteral("QProcess::startDetached failed"));
        QVERIFY(banner.value(QStringLiteral("actions")).toArray().isEmpty());
    }

    void aLaunchFailureFallsBackToAGenericAppName()
    {
        MockNotificationClient client(nullptr, false);
        QSignalSpy banners(&client, &NotificationClient::bannersChanged);
        raiseDockLaunchFailure(&client, QString(), QString());
        const QJsonArray view =
            QJsonDocument::fromJson(banners.takeFirst().at(0).toByteArray()).array();
        QCOMPARE(view.first().toObject().value(QStringLiteral("summary")).toString(),
                 QStringLiteral("Could not launch app"));
        QCOMPARE(view.first().toObject().value(QStringLiteral("body")).toString(),
                 QStringLiteral("The app did not start."));
    }

    void aMockNotifyCarriesItsActionsAndInvokeDismisses()
    {
        MockNotificationClient client(nullptr, false);
        QSignalSpy banners(&client, &NotificationClient::bannersChanged);
        client.notify(QStringLiteral("Mail"), QStringLiteral("New message"),
                      QStringLiteral("From Ada"), QStringLiteral("critical"),
                      { QStringLiteral("reply") }, { QStringLiteral("Reply") });
        const QJsonArray view =
            QJsonDocument::fromJson(banners.takeFirst().at(0).toByteArray()).array();
        const QJsonObject banner = view.first().toObject();
        QCOMPARE(banner.value(QStringLiteral("urgency")).toString(),
                 QStringLiteral("critical"));
        const QJsonArray actions = banner.value(QStringLiteral("actions")).toArray();
        QCOMPARE(actions.size(), 1);
        QCOMPARE(actions.first().toObject().value(QStringLiteral("key")).toString(),
                 QStringLiteral("reply"));
        QCOMPARE(actions.first().toObject().value(QStringLiteral("label")).toString(),
                 QStringLiteral("Reply"));

        // Invoking the action removes the banner; the history records the
        // dismissal (the live service also emits ActionInvoked to the app).
        client.invoke(1, QStringLiteral("reply"));
        const QJsonArray cleared =
            QJsonDocument::fromJson(banners.takeFirst().at(0).toByteArray()).array();
        QVERIFY(cleared.isEmpty());
    }

    // -- Focus/DND menu-bar reflection (T-11.2b) --------------------------

    void focusStatusItemHidesForOffAndShowsTheCrescentOtherwise()
    {
        const QVariantMap off = focusStatusItem(
            QVariantMap{ { QStringLiteral("mode"), QStringLiteral("off") } });
        QCOMPARE(off.value(QStringLiteral("id")).toString(), QStringLiteral("focus"));
        QCOMPARE(off.value(QStringLiteral("available")).toBool(), false);
        QCOMPARE(off.value(QStringLiteral("selected")).toBool(), false);

        // An absent/unknown policy is the same hidden default.
        QCOMPARE(focusStatusItem({}).value(QStringLiteral("available")).toBool(), false);

        const QVariantMap focus = focusStatusItem(
            QVariantMap{ { QStringLiteral("mode"), QStringLiteral("focus") } });
        QCOMPARE(focus.value(QStringLiteral("available")).toBool(), true);
        QCOMPARE(focus.value(QStringLiteral("selected")).toBool(), false);
        QCOMPARE(focus.value(QStringLiteral("icon")).toString(), QStringLiteral("focus"));

        const QVariantMap dnd = focusStatusItem(
            QVariantMap{ { QStringLiteral("mode"), QStringLiteral("dnd") } });
        QCOMPARE(dnd.value(QStringLiteral("available")).toBool(), true);
        QCOMPARE(dnd.value(QStringLiteral("selected")).toBool(), true);
        QCOMPARE(dnd.value(QStringLiteral("accessibleName")).toString(),
                 QStringLiteral("Do Not Disturb"));
    }

    void theFocusStatusItemSurfacesTheSuppressedBatchCount()
    {
        const QVariantMap item = focusStatusItem(
            QVariantMap{ { QStringLiteral("mode"), QStringLiteral("dnd") },
                         { QStringLiteral("batchedCount"), 4 } });
        QCOMPARE(item.value(QStringLiteral("label")).toString(), QStringLiteral("4"));
        QVERIFY(item.value(QStringLiteral("accessibleName")).toString().contains(
            QStringLiteral("4")));

        const QVariantMap quiet = focusStatusItem(
            QVariantMap{ { QStringLiteral("mode"), QStringLiteral("focus") },
                         { QStringLiteral("batchedCount"), 0 } });
        QCOMPARE(quiet.value(QStringLiteral("label")).toString(), QString());
    }

    void theMockClientFocusModeRoundTripsThroughThePolicySignal()
    {
        MockNotificationClient client(nullptr, /*seedFixture=*/false);
        QSignalSpy policy(&client, &NotificationClient::focusPolicyChanged);
        client.refresh();
        QCOMPARE(policy.count(), 1);
        QCOMPARE(QJsonDocument::fromJson(policy.takeFirst().at(0).toByteArray())
                     .object()
                     .value(QStringLiteral("mode"))
                     .toString(),
                 QStringLiteral("off"));

        client.setFocusMode(QStringLiteral("dnd"));
        QCOMPARE(policy.count(), 1);
        const QJsonObject dnd = QJsonDocument::fromJson(policy.takeFirst().at(0).toByteArray())
                                     .object();
        QCOMPARE(dnd.value(QStringLiteral("mode")).toString(), QStringLiteral("dnd"));

        // Unknown names are rejected and leave the mode unchanged.
        client.setFocusMode(QStringLiteral("nonsense"));
        QCOMPARE(policy.count(), 0);

        // The T-11.1a compat bool maps onto the three-way policy.
        client.setDoNotDisturb(false);
        QCOMPARE(policy.count(), 1);
        QCOMPARE(QJsonDocument::fromJson(policy.takeFirst().at(0).toByteArray())
                     .object()
                     .value(QStringLiteral("mode"))
                     .toString(),
                 QStringLiteral("off"));
    }

    void theControlCenterFocusToggleMapsToTheServiceMode()
    {
        // On is Do Not Disturb; off clears the policy. The toggle never
        // selects the middle `focus` mode.
        QCOMPARE(focusModeForToggle(true), QStringLiteral("dnd"));
        QCOMPARE(focusModeForToggle(false), QStringLiteral("off"));

        // The mapped mode is accepted by the service's own vocabulary (the
        // notification client rejects anything else).
        MockNotificationClient client(nullptr, /*seedFixture=*/false);
        QSignalSpy policy(&client, &NotificationClient::focusPolicyChanged);
        client.setFocusMode(focusModeForToggle(true));
        QCOMPARE(policy.count(), 1);
        QCOMPARE(QJsonDocument::fromJson(policy.takeFirst().at(0).toByteArray())
                     .object()
                     .value(QStringLiteral("mode"))
                     .toString(),
                 QStringLiteral("dnd"));
        client.setFocusMode(focusModeForToggle(false));
        QCOMPARE(policy.count(), 1);
        QCOMPARE(QJsonDocument::fromJson(policy.takeFirst().at(0).toByteArray())
                     .object()
                     .value(QStringLiteral("mode"))
                     .toString(),
                 QStringLiteral("off"));
    }

    void theControlCenterDarkToggleMapsToAnAbsoluteScheme()
    {
        // On writes `dark`, off writes `light`; the toggle never writes `auto`.
        QCOMPARE(colorSchemeForDarkToggle(true), QStringLiteral("dark"));
        QCOMPARE(colorSchemeForDarkToggle(false), QStringLiteral("light"));
    }
};

QTEST_GUILESS_MAIN(TestDockCore)
#include "tst_dockcore.moc"
