// SPDX-License-Identifier: MIT
// Dock core unit tests (T-10): the app-index entry cache and launcher,
// `dock.pinned` persistence, and the pure pinned+running entry merge. The
// interim `.desktop` resolver was retired in T-14.7 (app-index owns parsing).
// Runs headless with no compositor, Wayland, or QML.
#include "apppicker.h"
#include "controlcenterpolicy.h"
#include "desktopentry.h"
#include "dockdrops.h"
#include "dockmodel.h"
#include "dockprojection.h"
#include "folderstacks.h"
#include "filestarget.h"
#include "focusstatus.h"
#include "framecommitgate.h"
#include "launchfailure.h"
#include "menubrokerclient.h"
#include "menubrokerpolicy.h"
#include "notificationclient.h"
#include "osdmodel.h"
#include "settingsclient.h"
#include "trayclient.h"
#include "trashbridge.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcessEnvironment>
#include <QSet>
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

// One cached application record. app-index is the only production `.desktop`
// parser (T-14.1a/T-14.7); the shell tests supply the records the service
// enumerates instead of parsing files themselves.
DesktopEntry makeEntry(const QString &id, const QString &name, const QString &exec,
                       const QString &startupWmClass = {}, const QStringList &categories = {},
                       bool terminal = false, bool noDisplay = false)
{
    DesktopEntry entry;
    entry.id = id;
    entry.name = name;
    entry.exec = exec;
    entry.startupWmClass = startupWmClass;
    entry.categories = categories;
    entry.terminal = terminal;
    entry.noDisplay = noDisplay;
    entry.valid = true;
    return entry;
}
} // namespace

class TestDockCore : public QObject
{
    Q_OBJECT

private slots:
    // -- launchability ---------------------------------------------------

    void launchableRejectsNoDisplay()
    {
        const DesktopEntry entry =
            makeEntry(QStringLiteral("hidden.desktop"), QStringLiteral("Hidden"),
                      QStringLiteral("x"), {}, {}, false, true);
        QVERIFY(entry.valid);
        QVERIFY(entry.noDisplay);
        QVERIFY(!DesktopEntryIndex::isLaunchable(entry));
    }

    // -- resolution (over the app-index cache) ---------------------------

    void resolveByIdSuffixAndWmClass()
    {
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("org.dragonfruit.Files.desktop"), QStringLiteral("Files"),
                      QStringLiteral("df-files")),
            makeEntry(QStringLiteral("org.example.Nautilus.desktop"), QStringLiteral("Files"),
                      QStringLiteral("nautilus"), QStringLiteral("org.example.Nautilus")),
            makeEntry(QStringLiteral("firefox.desktop"), QStringLiteral("Firefox"),
                      QStringLiteral("firefox %u"), QStringLiteral("firefox")),
        });

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

    void loadFromRecordsUsesFirstDuplicateId()
    {
        // app-index already applies desktop-file precedence when it scans; the
        // cache must not let a later duplicate override the first record.
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("dup.desktop"), QStringLiteral("User"),
                      QStringLiteral("user")),
            makeEntry(QStringLiteral("dup.desktop"), QStringLiteral("System"),
                      QStringLiteral("system")),
        });
        QCOMPARE(index.byId(QStringLiteral("dup.desktop")).name, QStringLiteral("User"));
    }

    // T-14.1a: the Dock's identity corpus now comes from app-index, which
    // resolves the themed icon to a file path the Dock can render.
    void appIndexRecordsCarryThemedIconPaths()
    {
        DesktopEntry files;
        files.id = QStringLiteral("org.dragonfruit.Files.desktop");
        files.name = QStringLiteral("Files");
        files.icon = QStringLiteral("system-file-manager");
        files.iconPath =
            QStringLiteral("/usr/share/icons/hicolor/scalable/apps/system-file-manager.svg");
        files.exec = QStringLiteral("dragonfruit-files %U");
        files.valid = true;

        DesktopEntryIndex index;
        index.loadFromRecords({files});
        QCOMPARE(index.byId(QStringLiteral("org.dragonfruit.Files.desktop")).iconPath,
                 files.iconPath);
        QCOMPARE(index.resolve(QStringLiteral("org.dragonfruit.Files")).iconPath,
                 files.iconPath);
        // A record with no themed path is still valid; the Dock draws the
        // initial tile.
        DesktopEntry plain;
        plain.id = QStringLiteral("plain.desktop");
        plain.name = QStringLiteral("Plain");
        plain.valid = true;
        index.loadFromRecords({plain});
        QVERIFY(index.byId(QStringLiteral("plain.desktop")).valid);
        QVERIFY(index.byId(QStringLiteral("plain.desktop")).iconPath.isEmpty());
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

    // -- launched-app environment (T-09 follow-up) -----------------------

    void appLaunchEnvironmentScrubsTheShellsOffscreenQpa()
    {
        QProcessEnvironment base = QProcessEnvironment::systemEnvironment();
        base.insert(QStringLiteral("QT_QPA_PLATFORM"), QStringLiteral("offscreen"));
        base.insert(QStringLiteral("DF_SENTINEL"), QStringLiteral("keep"));

        const QProcessEnvironment environment = appLaunchEnvironment(base);
        // The session is Wayland; the app must not inherit the shell's
        // offscreen QPA (nor fall back to X11 via the inherited DISPLAY).
        QCOMPARE(environment.value(QStringLiteral("QT_QPA_PLATFORM")),
                 QStringLiteral("wayland"));
        QCOMPARE(environment.value(QStringLiteral("DF_SENTINEL")), QStringLiteral("keep"));
    }

    void appLaunchEnvironmentPreservesADeliberatePlatform()
    {
        QProcessEnvironment base;
        base.insert(QStringLiteral("QT_QPA_PLATFORM"), QStringLiteral("xcb"));
        const QProcessEnvironment environment = appLaunchEnvironment(base);
        QCOMPARE(environment.value(QStringLiteral("QT_QPA_PLATFORM")),
                 QStringLiteral("xcb"));
    }

    void appLaunchEnvironmentToleratesNoPlatformKey()
    {
        QProcessEnvironment base;
        base.insert(QStringLiteral("WAYLAND_DISPLAY"), QStringLiteral("dragonfruit-wayland"));
        const QProcessEnvironment environment = appLaunchEnvironment(base);
        QCOMPARE(environment.contains(QStringLiteral("QT_QPA_PLATFORM")), false);
        QCOMPARE(environment.value(QStringLiteral("WAYLAND_DISPLAY")),
                 QStringLiteral("dragonfruit-wayland"));
    }

    // T-14.7g: the shell connected with `--socket-name`, so the environment it
    // was started in never held `WAYLAND_DISPLAY`. A launched child must still
    // receive the actual socket or it maps no window ("clicking does nothing").
    void appLaunchEnvironmentExportsTheShellsSocketName()
    {
        QProcessEnvironment base;
        base.insert(QStringLiteral("QT_QPA_PLATFORM"), QStringLiteral("offscreen"));
        QVERIFY(!base.contains(QStringLiteral("WAYLAND_DISPLAY")));

        const QProcessEnvironment environment =
            appLaunchEnvironment(base, QStringLiteral("dragonfruit-wayland-7"));
        QCOMPARE(environment.value(QStringLiteral("WAYLAND_DISPLAY")),
                 QStringLiteral("dragonfruit-wayland-7"));
        QCOMPARE(environment.value(QStringLiteral("QT_QPA_PLATFORM")),
                 QStringLiteral("wayland"));
    }

    // The socket the shell connected on wins over a stale inherited name (the
    // nested backend may publish a different socket than the shell's env).
    void appLaunchEnvironmentUsesTheShellsSocketOverAStaleOne()
    {
        QProcessEnvironment base;
        base.insert(QStringLiteral("WAYLAND_DISPLAY"), QStringLiteral("wayland-0"));
        const QProcessEnvironment environment =
            appLaunchEnvironment(base, QStringLiteral("dragonfruit-nested"));
        QCOMPARE(environment.value(QStringLiteral("WAYLAND_DISPLAY")),
                 QStringLiteral("dragonfruit-nested"));
    }

    // With no socket supplied (an empty socket name) the inherited display is
    // left untouched.
    void appLaunchEnvironmentKeepsTheInheritedDisplayWithoutASocket()
    {
        QProcessEnvironment base;
        base.insert(QStringLiteral("WAYLAND_DISPLAY"), QStringLiteral("wayland-0"));
        const QProcessEnvironment environment = appLaunchEnvironment(base, QString());
        QCOMPARE(environment.value(QStringLiteral("WAYLAND_DISPLAY")),
                 QStringLiteral("wayland-0"));
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
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            // The hidden WebBrowser record comes first but must not be pinned.
            makeEntry(QStringLiteral("aaa-hidden.desktop"), QStringLiteral("Hidden Browser"),
                      QStringLiteral("hidden"), {},
                      {QStringLiteral("Network"), QStringLiteral("WebBrowser")}, false, true),
            makeEntry(QStringLiteral("org.dragonfruit.Files.desktop"), QStringLiteral("Files"),
                      QStringLiteral("df-files")),
            makeEntry(QStringLiteral("org.dragonfruit.Settings.desktop"),
                      QStringLiteral("Settings"), QStringLiteral("df-settings")),
            makeEntry(QStringLiteral("term.desktop"), QStringLiteral("Term"),
                      QStringLiteral("term"),
                      {}, {QStringLiteral("System"), QStringLiteral("TerminalEmulator")}),
            makeEntry(QStringLiteral("browse.desktop"), QStringLiteral("Browser"),
                      QStringLiteral("browser"),
                      {}, {QStringLiteral("Network"), QStringLiteral("WebBrowser")}),
        });
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
        QCOMPARE(config.chooserOnHover, false);
        QCOMPARE(config.minimizeReaction, false);
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
        values.insert(QStringLiteral("dock.chooserOnHover"), true);
        values.insert(QStringLiteral("dock.minimizeReaction"), true);
        const DockConfig config = dockConfigFromValues(values);
        QCOMPARE(config.size, 0.9);
        QCOMPARE(config.autohide, true);
        QCOMPARE(config.position, QStringLiteral("left"));
        QCOMPARE(config.reduceMotion, true);
        QCOMPARE(config.chooserOnHover, true);
        QCOMPARE(config.minimizeReaction, true);
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
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("org.dragonfruit.Files.desktop"), QStringLiteral("Files"),
                      QStringLiteral("df-files")),
            makeEntry(QStringLiteral("org.dragonfruit.Settings.desktop"),
                      QStringLiteral("Settings"), QStringLiteral("df-settings")),
            makeEntry(QStringLiteral("org.mozilla.firefox.desktop"), QStringLiteral("Firefox"),
                      QStringLiteral("firefox")),
        });

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
        // The merged published entry carries the derived count too (T-14.7o).
        QCOMPARE(files.value(QStringLiteral("windowCount")).toInt(), 2);
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
        QCOMPARE(entries.at(0).toMap().value(QStringLiteral("windowCount")).toInt(), 1);
    }

    void mergeTemporaryCarriesDesktopIdForPromotion()
    {
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("org.example.Terminal.desktop"), QStringLiteral("Terminal"),
                      QStringLiteral("term")),
        });
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

    // -- fixed-menu app open plan (T-09 follow-up) -----------------------

    void planAppOpenLaunchesWhenNotRunning()
    {
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("org.dragonfruit.Settings.desktop"),
                      QStringLiteral("Settings"), QStringLiteral("df-settings")),
        });

        const AppOpenPlan plan = planAppOpen(
            index, {}, QStringLiteral("org.dragonfruit.Settings.desktop"));
        QVERIFY(plan.resolved);
        QCOMPARE(plan.running, false);
        QVERIFY(plan.appId.isEmpty());
        QCOMPARE(plan.desktopId, QStringLiteral("org.dragonfruit.Settings.desktop"));
    }

    void planAppOpenActivatesARunningWindow()
    {
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("org.dragonfruit.Settings.desktop"),
                      QStringLiteral("Settings"), QStringLiteral("df-settings")),
        });

        // The compositor's raw identity resolves to the Settings desktop id.
        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("settings")},
                        {QStringLiteral("appId"), QStringLiteral("org.dragonfruit.Settings")},
                        {QStringLiteral("kind"), QStringLiteral("temporary")},
                        {QStringLiteral("running"), true},
                        {QStringLiteral("windows"), 1}},
        };
        const AppOpenPlan plan = planAppOpen(
            index, running, QStringLiteral("org.dragonfruit.Settings.desktop"));
        QVERIFY(plan.resolved);
        QCOMPARE(plan.running, true);
        QCOMPARE(plan.appId, QStringLiteral("org.dragonfruit.Settings"));
        QCOMPARE(plan.desktopId, QStringLiteral("org.dragonfruit.Settings.desktop"));
    }

    void planAppOpenIgnoresAnUnrelatedRunningWindow()
    {
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("org.dragonfruit.Settings.desktop"),
                      QStringLiteral("Settings"), QStringLiteral("df-settings")),
        });

        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("firefox")},
                        {QStringLiteral("appId"), QStringLiteral("firefox")},
                        {QStringLiteral("kind"), QStringLiteral("temporary")},
                        {QStringLiteral("running"), true},
                        {QStringLiteral("windows"), 1}},
        };
        const AppOpenPlan plan = planAppOpen(
            index, running, QStringLiteral("org.dragonfruit.Settings.desktop"));
        QVERIFY(plan.resolved);
        QCOMPARE(plan.running, false);
    }

    void planAppOpenReportsAnUnresolvedIdentity()
    {
        DesktopEntryIndex index; // empty corpus
        const AppOpenPlan plan = planAppOpen(
            index, {}, QStringLiteral("org.dragonfruit.Settings.desktop"));
        QCOMPARE(plan.resolved, false);
        QCOMPARE(plan.running, false);
        QVERIFY(plan.desktopId.isEmpty());
    }

    // T-14.7g identity fallback: a projection row with no compositor app id
    // cannot be activated, so the plan stays "not running" and the caller
    // launches the installed entry ("treat an empty app id as a launch").
    void planAppOpenTreatsAnEmptyAppIdAsANonMatch()
    {
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("org.dragonfruit.Settings.desktop"),
                      QStringLiteral("Settings"), QStringLiteral("df-settings")),
        });
        const QVariantList running{
            QVariantMap{{QStringLiteral("id"), QStringLiteral("unknown")},
                        {QStringLiteral("appId"), QString()},
                        {QStringLiteral("kind"), QStringLiteral("temporary")},
                        {QStringLiteral("running"), true},
                        {QStringLiteral("windows"), 1}},
        };
        const AppOpenPlan plan = planAppOpen(
            index, running, QStringLiteral("org.dragonfruit.Settings.desktop"));
        QVERIFY(plan.resolved);
        QCOMPARE(plan.desktopId, QStringLiteral("org.dragonfruit.Settings.desktop"));
        QCOMPARE(plan.running, false);
        QVERIFY(plan.appId.isEmpty());
    }

    // The shipped first-party `.desktop` entries are verified by app-index's
    // own tests (the only production parser): see
    // `services/app-index/src/index.rs`'s
    // `shipped_first_party_entries_are_launchable`.

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

    // T-14.7o: the projection exposes the app's window count directly, so the
    // Dock's badge never has to infer it from the raw list.
    void projectionExposesTheWindowCount()
    {
        const QVariantList entries = buildDockProjection({
            window(1, QStringLiteral("org.example.Alpha"), QStringLiteral("a1")),
            window(2, QStringLiteral("org.example.Alpha"), QStringLiteral("a2")),
            window(3, QStringLiteral("org.example.Beta"), QStringLiteral("b1"), true),
        });

        const QVariantMap alpha = entryForAppId(entries, QStringLiteral("org.example.Alpha"));
        QCOMPARE(alpha.value(QStringLiteral("windowCount")).toInt(), 2);
        QCOMPARE(dockWindowCount(alpha), 2);

        // A minimized entry carries its owning app's full count too, so a
        // click-to-choose from the minimized row still badges correctly.
        const QVariantMap beta = entryForAppId(entries, QStringLiteral("org.example.Beta"));
        QCOMPARE(beta.value(QStringLiteral("windowCount")).toInt(), 1);
        for (const QVariant &value : entries) {
            const QVariantMap map = value.toMap();
            if (map.value(QStringLiteral("kind")).toString() != QLatin1String("minimized"))
                continue;
            QCOMPARE(map.value(QStringLiteral("windowCount")).toInt(), 1);
        }
    }

    // The helper is the single source of the count: it prefers `windowList` and
    // falls back to the legacy scalar, and never invents a count.
    void windowCountDerivesFromTheWindowListOrTheScalar()
    {
        QCOMPARE(dockWindowCount(QVariantMap{}), 0);
        QCOMPARE(dockWindowCount(QVariantMap{{QStringLiteral("windows"), 4}}), 4);
        QCOMPARE(dockWindowCount(QVariantMap{
                     {QStringLiteral("windows"), 1},
                     {QStringLiteral("windowList"),
                      QVariantList{QVariantMap{{QStringLiteral("windowId"),
                                                QStringLiteral("1")}},
                                   QVariantMap{{QStringLiteral("windowId"),
                                                QStringLiteral("2")}}}}}),
                 2);
        // An entry with neither field is a zero, not a crash.
        QCOMPARE(dockWindowCount(QVariantMap{{QStringLiteral("id"), QStringLiteral("x")}}), 0);
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

    // T-14.7r: the async empty runs off the UI thread, reports Emptying while
    // in flight, then a one-shot success with the removed count and a re-read
    // store.
    void trashBridgeAsyncEmptyReportsTheRemovedCount()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        setenv("XDG_DATA_HOME", dir.path().toUtf8().constData(), 1);
        QVERIFY(QDir().mkpath(root + QStringLiteral("/info")));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));
        writeFile(root + QStringLiteral("/info/a.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/a\n"));
        writeFile(root + QStringLiteral("/files/a"), QStringLiteral("a"));
        writeFile(root + QStringLiteral("/info/b.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/b\n"));
        writeFile(root + QStringLiteral("/files/b"), QStringLiteral("b"));

        TrashBridge trash;
        trash.start();
        QCOMPARE(trash.itemCount(), 2);

        QSignalSpy finished(&trash, &TrashBridge::emptyFinished);
        QVERIFY(finished.isValid());

        QVERIFY(trash.emptyAsync());
        QVERIFY(trash.isEmptying());
        QVERIFY(trash.emptyState() == TrashBridge::EmptyState::Emptying);

        QTRY_VERIFY(!trash.isEmptying());
        QVERIFY(trash.emptyState() == TrashBridge::EmptyState::Succeeded);
        QCOMPARE(trash.emptyRemoved(), 2);
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), true);
        QCOMPARE(finished.at(0).at(1).toInt(), 2);
        QVERIFY(trash.emptyError().isEmpty());
        QVERIFY(!trash.isFull());
        QCOMPARE(trash.itemCount(), 0);
    }

    // T-14.7r: one operation at a time. A second request while the first is
    // still in flight (the finish is a queued call, so it has not run yet) is
    // ignored rather than started.
    void trashBridgeSecondEmptyWhileEmptyingIsIgnored()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString root = dir.path() + QStringLiteral("/Trash");
        setenv("XDG_DATA_HOME", dir.path().toUtf8().constData(), 1);
        QVERIFY(QDir().mkpath(root + QStringLiteral("/info")));
        QVERIFY(QDir().mkpath(root + QStringLiteral("/files")));
        writeFile(root + QStringLiteral("/info/only.trashinfo"),
                  QStringLiteral("[Trash Info]\nPath=/home/x/only\n"));
        writeFile(root + QStringLiteral("/files/only"), QStringLiteral("x"));

        TrashBridge trash;
        trash.start();
        QVERIFY(trash.emptyAsync());
        QVERIFY(trash.isEmptying());
        QVERIFY(!trash.emptyAsync());
        QVERIFY(trash.isEmptying());

        QTRY_VERIFY(!trash.isEmptying());
        QVERIFY(trash.emptyState() == TrashBridge::EmptyState::Succeeded);
        QCOMPARE(trash.emptyRemoved(), 1);
    }

    // T-14.7r: the absent-backend case (service down). A bridge that never
    // started has no monitor; emptyAsync reports a failure immediately instead
    // of blocking or hanging.
    void trashBridgeEmptyOnAnAbsentBackendFailsWithoutHanging()
    {
        TrashBridge trash;
        QSignalSpy finished(&trash, &TrashBridge::emptyFinished);
        QVERIFY(finished.isValid());

        QVERIFY(trash.emptyAsync());
        QVERIFY(!trash.isEmptying());
        QVERIFY(trash.emptyState() == TrashBridge::EmptyState::Failed);
        QCOMPARE(finished.count(), 1);
        QCOMPARE(finished.at(0).at(0).toBool(), false);
        QCOMPARE(finished.at(0).at(1).toInt(), -1);
        QVERIFY(!trash.emptyError().isEmpty());

        trash.resetEmptyState();
        QVERIFY(trash.emptyState() == TrashBridge::EmptyState::Idle);
        QVERIFY(trash.emptyRemoved() == 0);
        QVERIFY(trash.emptyError().isEmpty());
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
                 Action::MoveToFolder);
        QCOMPARE(dockDropActionFor(QStringLiteral("divider"), Payload::Files), Action::None);
        QCOMPARE(dockDropActionFor(QStringLiteral("minimized"), Payload::Files), Action::None);

        // A single folder pins on the app region, moves into a stack, and
        // trashes (T-14.7k).
        QCOMPARE(dockDropActionFor(QString(), Payload::Folder), Action::PinFolder);
        QCOMPARE(dockDropActionFor(QStringLiteral("pinned"), Payload::Folder),
                 Action::PinFolder);
        QCOMPARE(dockDropActionFor(QStringLiteral("temporary"), Payload::Folder),
                 Action::PinFolder);
        QCOMPARE(dockDropActionFor(QStringLiteral("stack"), Payload::Folder),
                 Action::MoveToFolder);
        QCOMPARE(dockDropActionFor(QStringLiteral("trash"), Payload::Folder),
                 Action::TrashFiles);
        QCOMPARE(dockDropActionFor(QStringLiteral("divider"), Payload::Folder),
                 Action::None);
        QCOMPARE(dockDropActionFor(QStringLiteral("minimized"), Payload::Folder),
                 Action::None);
    }

    void uriListClassifiesASingleDirectoryAsAFolder()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString folder = dir.path() + QStringLiteral("/Documents");
        QVERIFY(QDir().mkpath(folder));
        writeFile(dir.path() + QStringLiteral("/readme.txt"), QStringLiteral("x"));

        QVERIFY(uriListIsFolder(QStringList{folder}));
        QVERIFY(!uriListIsFolder(QStringList{dir.path() + QStringLiteral("/readme.txt")}));
        QVERIFY(!uriListIsFolder(QStringList{folder, folder}));
        QVERIFY(!uriListIsFolder(QStringList{QStringLiteral("/does/not/exist")}));
        // A relative path is never a pin candidate.
        QVERIFY(!uriListIsFolder(QStringList{QStringLiteral("Documents")}));
    }

    void parseDockDropPayloadClassifiesASingleFolder()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString folder = dir.path() + QStringLiteral("/Documents");
        QVERIFY(QDir().mkpath(folder));
        const QByteArray payload =
            QUrl::fromLocalFile(folder).toString().toUtf8() + "\r\n";
        const DockDropPayloadData data =
            parseDockDropPayload(QStringLiteral("text/uri-list"), payload);
        QVERIFY(data.valid);
        QCOMPARE(data.kind, DockDropPayload::Folder);
        QCOMPARE(data.paths, (QStringList{folder}));
        QVERIFY(data.desktopId.isEmpty());
    }

    // -- enter-time payload caching and drop affordances (T-14.7f) --------

    void parseDockDropPayloadClassifiesAppAliasAndFiles()
    {
        // The app-alias mime carries the identity directly.
        const DockDropPayloadData alias = parseDockDropPayload(
            QStringLiteral("application/x-dragonfruit-app"),
            QByteArrayLiteral("org.example.App\n"));
        QVERIFY(alias.valid);
        QCOMPARE(alias.kind, DockDropPayload::Application);
        QCOMPARE(alias.desktopId, QStringLiteral("org.example.App"));
        QVERIFY(alias.paths.isEmpty());

        // A single `.desktop` URI list is also an app alias.
        const DockDropPayloadData desktop = parseDockDropPayload(
            QStringLiteral("text/uri-list"),
            QByteArrayLiteral("file:///tmp/org.example.App.desktop\r\n"));
        QVERIFY(desktop.valid);
        QCOMPARE(desktop.kind, DockDropPayload::Application);
        QCOMPARE(desktop.desktopId, QStringLiteral("org.example.App.desktop"));
        QVERIFY(desktop.paths.isEmpty());

        // Several files stay files and keep their decoded paths.
        const DockDropPayloadData files = parseDockDropPayload(
            QStringLiteral("text/uri-list"),
            QByteArrayLiteral("file:///tmp/a.txt\r\nfile:///tmp/b.txt\r\n"));
        QVERIFY(files.valid);
        QCOMPARE(files.kind, DockDropPayload::Files);
        QCOMPARE(files.paths, (QStringList{QStringLiteral("/tmp/a.txt"),
                                            QStringLiteral("/tmp/b.txt")}));

        // An unsupported mime is not consumed.
        QVERIFY(!parseDockDropPayload(QStringLiteral("text/plain"),
                                      QByteArrayLiteral("hello")).valid);
        // The app-alias mime with an empty value resolves to nothing.
        QVERIFY(!parseDockDropPayload(QStringLiteral("application/x-dragonfruit-app"),
                                      QByteArrayLiteral("  \n")).valid);
    }

    void dropAffordanceMatchesTheActionPerTarget()
    {
        using Payload = DockDropPayload;
        QCOMPARE(dockDropAffordance(QString(), Payload::Application, QString(), true),
                 QStringLiteral("Add to Dock"));
        QCOMPARE(dockDropAffordance(QStringLiteral("divider"), Payload::Application,
                                    QString(), true),
                 QString());
        QCOMPARE(dockDropAffordance(QStringLiteral("trash"), Payload::Application,
                                    QString(), true),
                 QString());
        QCOMPARE(dockDropAffordance(QStringLiteral("pinned"), Payload::Files,
                                    QStringLiteral("Files"), true),
                 QStringLiteral("Open with Files"));
        QCOMPARE(dockDropAffordance(QStringLiteral("pinned"), Payload::Files, QString(), true),
                 QStringLiteral("Open with"));
        QCOMPARE(dockDropAffordance(QStringLiteral("trash"), Payload::Files, QString(), true),
                 QStringLiteral("Move to Trash"));
        QCOMPARE(dockDropAffordance(QStringLiteral("trash"), Payload::Files, QString(), false),
                 QStringLiteral("Trash unavailable"));
        QCOMPARE(dockDropAffordance(QStringLiteral("stack"), Payload::Files, QString(), true),
                 QStringLiteral("Move to Folder"));
        QCOMPARE(dockDropAffordance(QStringLiteral("stack"), Payload::Files,
                                    QStringLiteral("Documents"), true),
                 QStringLiteral("Move to Documents"));
        QCOMPARE(dockDropAffordance(QStringLiteral("divider"), Payload::Files, QString(), true),
                 QString());
        // A single folder offers a pin on the app region (T-14.7k).
        QCOMPARE(dockDropAffordance(QString(), Payload::Folder, QString(), true),
                 QStringLiteral("Pin Folder"));
        QCOMPARE(dockDropAffordance(QStringLiteral("pinned"), Payload::Folder, QString(), true),
                 QStringLiteral("Pin Folder"));
        QCOMPARE(dockDropAffordance(QStringLiteral("stack"), Payload::Folder,
                                    QStringLiteral("Documents"), true),
                 QStringLiteral("Move to Documents"));
        QCOMPARE(dockDropAffordance(QStringLiteral("divider"), Payload::Folder, QString(), true),
                 QString());
    }

    void duplicatePinIsDetectedByIdOrResolvedIdentity()
    {
        const QStringList pinned{QStringLiteral("org.example.App.desktop"),
                                 QStringLiteral("other.desktop")};
        QVERIFY(dockPinnedContains(pinned, QStringLiteral("org.example.App.desktop"), QString()));
        // A raw alias that resolves to the pinned id is still a duplicate.
        QVERIFY(dockPinnedContains(pinned, QStringLiteral("org.example.App"),
                                   QStringLiteral("org.example.App.desktop")));
        QVERIFY(!dockPinnedContains(pinned, QStringLiteral("new.desktop"), QString()));
        QVERIFY(!dockPinnedContains(pinned, QString(), QStringLiteral("org.example.App.desktop")));
    }

    // -- folder stacks (T-14.7k, generalizing T-10 section 17) -------------

    void folderStacksListAndBadgeNewItems()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString downloads = dir.path() + QStringLiteral("/Downloads");
        QVERIFY(QDir().mkpath(downloads));
        writeFile(downloads + QStringLiteral("/existing.txt"), QStringLiteral("x"));

        FolderStacks stacks;
        stacks.setFolders(QStringList{downloads});
        QCOMPARE(stacks.stacks().size(), 1);
        QVariantMap stack = stacks.stacks().first().toMap();
        QCOMPARE(stack.value(QStringLiteral("path")).toString(), downloads);
        QCOMPARE(stack.value(QStringLiteral("name")).toString(), QStringLiteral("Downloads"));
        QCOMPARE(stack.value(QStringLiteral("count")).toInt(), 1);
        // A pre-existing item is not a badge on login.
        QCOMPARE(stack.value(QStringLiteral("badge")).toInt(), 0);
        QCOMPARE(stack.value(QStringLiteral("missing")).toBool(), false);

        writeFile(downloads + QStringLiteral("/fresh.txt"), QStringLiteral("y"));
        QTRY_COMPARE(stacks.stacks().first().toMap().value(QStringLiteral("count")).toInt(), 2);
        QTRY_COMPARE(stacks.stacks().first().toMap().value(QStringLiteral("badge")).toInt(), 1);

        const QVariantList items = stacks.stacks().first().toMap()
                                       .value(QStringLiteral("items")).toList();
        bool foundFresh = false;
        for (const QVariant &value : items) {
            const QVariantMap item = value.toMap();
            QVERIFY(item.value(QStringLiteral("path")).toString().startsWith(downloads));
            if (item.value(QStringLiteral("name")).toString() == QStringLiteral("fresh.txt"))
                foundFresh = true;
        }
        QVERIFY(foundFresh);

        stacks.markSeen(downloads);
        QCOMPARE(stacks.stacks().first().toMap().value(QStringLiteral("badge")).toInt(), 0);
    }

    // A path that has vanished degrades to a missing entry with an empty
    // listing, never a crash (ADR 0092).
    void folderStacksDegradeAMissingPath()
    {
        FolderStacks stacks;
        stacks.setFolders(QStringList{QStringLiteral("/no/such/folder/here")});
        QCOMPARE(stacks.stacks().size(), 1);
        const QVariantMap stack = stacks.stacks().first().toMap();
        QVERIFY(stack.value(QStringLiteral("missing")).toBool());
        QCOMPARE(stack.value(QStringLiteral("count")).toInt(), 0);
        QVERIFY(stack.value(QStringLiteral("items")).toList().isEmpty());
        QCOMPARE(stacks.folders().size(), 1);
    }

    void folderStacksDeduplicateTheFolderSet()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        QVERIFY(QDir().mkpath(dir.path() + QStringLiteral("/A")));
        QVERIFY(QDir().mkpath(dir.path() + QStringLiteral("/B")));
        FolderStacks stacks;
        stacks.setFolders(QStringList{dir.path() + QStringLiteral("/A"),
                                      dir.path() + QStringLiteral("/A"),
                                      dir.path() + QStringLiteral("/B")});
        QCOMPARE(stacks.folders().size(), 2);
        QVERIFY(stacks.contains(dir.path() + QStringLiteral("/A")));
        QVERIFY(!stacks.contains(dir.path() + QStringLiteral("/C")));
    }

    void moveFilesIntoFolderMovesAndRefusesUnsafeRoot()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString downloads = dir.path() + QStringLiteral("/Downloads");
        const QString source = dir.path() + QStringLiteral("/note.txt");
        writeFile(source, QStringLiteral("hello"));

        QString error;
        QCOMPARE(moveFilesIntoFolder(downloads, QStringList{source}, &error), 1);
        QVERIFY(!QFileInfo::exists(source));
        QVERIFY(QFileInfo::exists(downloads + QStringLiteral("/note.txt")));
        QVERIFY(error.isEmpty());

        // Moving a file already in the folder is refused, never recursive.
        QCOMPARE(moveFilesIntoFolder(downloads,
                                     QStringList{downloads + QStringLiteral("/note.txt")},
                                     &error),
                 0);
        QVERIFY(!error.isEmpty());
        QVERIFY(QFileInfo::exists(downloads + QStringLiteral("/note.txt")));

        // A name collision is de-duplicated rather than overwriting.
        writeFile(dir.path() + QStringLiteral("/note.txt"), QStringLiteral("again"));
        QCOMPARE(moveFilesIntoFolder(downloads,
                                     QStringList{dir.path() + QStringLiteral("/note.txt")}),
                 1);
        QVERIFY(QFileInfo::exists(downloads + QStringLiteral("/note.1.txt")));

        QCOMPARE(moveFilesIntoFolder(QStringLiteral("/"),
                                     QStringList{QStringLiteral("/tmp/whatever")}, &error),
                 0);
        QVERIFY(!error.isEmpty());
    }

    // T-14.7h/k: a folder's display name is its basename, so the Dock's hover
    // label and the stack popover header can name any folder without the
    // artwork carrying text. A root/empty basename falls back to "Downloads".
    void folderDisplayNameFallsBackToDownloads()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const QString folder = dir.path() + QStringLiteral("/My Stuff");
        QVERIFY(QDir().mkpath(folder));
        QCOMPARE(folderDisplayName(folder), QStringLiteral("My Stuff"));
        QCOMPARE(folderDisplayName(QStringLiteral("/")), QStringLiteral("Downloads"));
    }

    // -- recent/suggested apps (T-10 section 17) -------------------------

    void recentEntriesSkipPinnedRunningAndMisses()
    {
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("a.desktop"), QStringLiteral("Alpha"), QStringLiteral("a")),
            makeEntry(QStringLiteral("b.desktop"), QStringLiteral("Beta"), QStringLiteral("b")),
            makeEntry(QStringLiteral("c.desktop"), QStringLiteral("Gamma"), QStringLiteral("c")),
            makeEntry(QStringLiteral("d.desktop"), QStringLiteral("Delta"), QStringLiteral("d")),
        });

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
        DesktopEntryIndex index;
        index.loadFromRecords(QList<DesktopEntry>{
            makeEntry(QStringLiteral("pin.desktop"), QStringLiteral("Pin"),
                      QStringLiteral("pin")),
            makeEntry(QStringLiteral("recent.desktop"), QStringLiteral("Recent"),
                      QStringLiteral("recent")),
        });
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

        // At 250 the recent goes and the running groups no longer all fit, so
        // they fold into the terminal overflow cell (T-14.7q). Pinned are
        // never dropped, and the recent is still dropped silently.
        const DockOverflowResult two =
            applyDockOverflow(entries, 250, 48, 32, 64, 6, 1);
        QVERIFY(two.clamped);
        QVERIFY(!two.overflowed);
        QCOMPARE(two.iconSize, 32);
        QCOMPARE(two.hiddenRecent, 1);
        QCOMPARE(two.hiddenTemporary, 2);
        QCOMPARE(two.overflowShown, 2);
        QCOMPARE(countKind(two.entries, QStringLiteral("pinned")), 3);
        QCOMPARE(countKind(two.entries, QStringLiteral("temporary")), 0);
        QCOMPARE(countKind(two.entries, QStringLiteral("recent")), 0);
        QCOMPARE(countKind(two.entries, QStringLiteral("overflow")), 1);
    }

    // -- overflow cell (T-14.7q) -----------------------------------------

    // The hidden running groups a result reports, in the overflow entry.
    QVariantList overflowGroups(const DockOverflowResult &result)
    {
        for (const QVariant &value : result.entries) {
            const QVariantMap map = value.toMap();
            if (map.value(QStringLiteral("kind")).toString() == QLatin1String("overflow"))
                return map.value(QStringLiteral("groups")).toList();
        }
        return {};
    }

    void overflowCellCarriesTheHiddenRunningGroups()
    {
        // 3 pinned + 3 temporary, no room for all of them at the minimum: the
        // visible set keeps pinned only and the three running groups fold into
        // one terminal cell.
        const QVariantList entries = makeOverflowEntries(3, 3, 0, 0);
        const DockOverflowResult result =
            applyDockOverflow(entries, 250, 48, 32, 64, 6, 1);
        QVERIFY(result.clamped);
        QVERIFY(!result.overflowed);
        QCOMPARE(result.iconSize, 32);
        QCOMPARE(result.hiddenTemporary, 3);
        QCOMPARE(result.overflowShown, 3);
        QCOMPARE(countKind(result.entries, QStringLiteral("pinned")), 3);
        QCOMPARE(countKind(result.entries, QStringLiteral("temporary")), 0);
        QCOMPARE(countKind(result.entries, QStringLiteral("overflow")), 1);

        const QVariantList groups = overflowGroups(result);
        QCOMPARE(groups.size(), 3);
        // Every hidden group is reachable, each a running temporary entry
        // marked so the Dock can re-resolve the chooser after a projection.
        QSet<QString> ids;
        for (const QVariant &group : groups) {
            const QVariantMap map = group.toMap();
            QCOMPARE(map.value(QStringLiteral("kind")).toString(),
                     QStringLiteral("temporary"));
            QCOMPARE(map.value(QStringLiteral("overflowGroup")).toBool(), true);
            ids.insert(map.value(QStringLiteral("id")).toString());
        }
        QCOMPARE(ids.size(), 3);
    }

    void overflowCellNeverIncludesPinned()
    {
        // 4 pinned + 4 temporary: the pinned prefix always survives whole, and
        // no group in the cell is a pinned entry.
        const QVariantList entries = makeOverflowEntries(4, 4, 0, 0);
        const DockOverflowResult result =
            applyDockOverflow(entries, 300, 48, 32, 64, 6, 1);
        QCOMPARE(countKind(result.entries, QStringLiteral("pinned")), 4);
        const QVariantList groups = overflowGroups(result);
        QVERIFY(groups.size() > 0);
        for (const QVariant &group : groups) {
            const QVariantMap map = group.toMap();
            QVERIFY(map.value(QStringLiteral("kind")).toString()
                    != QLatin1String("pinned"));
        }
    }

    void overflowCellIsTheLastAppRegionEntry()
    {
        // The cell sits after the last app-region entry and before the first
        // minimized entry (the Dock inserts the divider after it).
        const QVariantList entries = makeOverflowEntries(2, 3, 0, 2);
        const DockOverflowResult result =
            applyDockOverflow(entries, 300, 48, 32, 64, 6, 1);
        int overflowIndex = -1;
        int minimizedIndex = -1;
        int lastAppIndex = -1;
        for (int i = 0; i < result.entries.size(); ++i) {
            const QString kind =
                result.entries.at(i).toMap().value(QStringLiteral("kind")).toString();
            if (kind == QLatin1String("overflow"))
                overflowIndex = i;
            else if (kind == QLatin1String("minimized") && minimizedIndex < 0)
                minimizedIndex = i;
            else if (kind == QLatin1String("pinned") || kind == QLatin1String("temporary")
                     || kind == QLatin1String("recent"))
                lastAppIndex = i;
        }
        QVERIFY(overflowIndex > lastAppIndex);
        QVERIFY(overflowIndex < minimizedIndex);
    }

    void overflowFallsBackWhenNoCellFits()
    {
        // 10 pinned with no droppable room: even one cell cannot fit, so the
        // legacy error state stands (no cell, nothing silently reachable).
        const QVariantList entries = makeOverflowEntries(10, 0, 0, 0);
        const DockOverflowResult result =
            applyDockOverflow(entries, 100, 48, 32, 64, 6, 1);
        QVERIFY(result.overflowed);
        QCOMPARE(result.overflowShown, 0);
        QCOMPARE(countKind(result.entries, QStringLiteral("overflow")), 0);
        QCOMPARE(countKind(result.entries, QStringLiteral("pinned")), 10);
    }

    void overflowNothingHiddenWhenEverythingFits()
    {
        // No clamp and no cell when the running groups fit.
        const QVariantList entries = makeOverflowEntries(2, 2, 0, 0);
        const DockOverflowResult result =
            applyDockOverflow(entries, 600, 48, 32, 64, 6, 1);
        QVERIFY(!result.clamped);
        QCOMPARE(result.overflowShown, 0);
        QCOMPARE(countKind(result.entries, QStringLiteral("overflow")), 0);
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

    // -- minimize-to-icon reaction (T-14.7s) -----------------------------

    void minimizeReactionPhaseIsOneHopThenDone()
    {
        // A single linear hop in [0,1); the caller's sin(pi*phase) makes the
        // round trip. It ends exactly at the token-mirrored duration.
        QCOMPARE(dockMinimizeReactionPhase(0), 0.0);
        QVERIFY(qAbs(dockMinimizeReactionPhase(kMinimizeReactionMs / 2) - 0.5) < 1e-9);
        QCOMPARE(dockMinimizeReactionPhase(kMinimizeReactionMs), -1.0);
        QCOMPARE(dockMinimizeReactionPhase(kMinimizeReactionMs + 10), -1.0);
        QCOMPARE(dockMinimizeReactionPhase(-1), -1.0);
    }

    void minimizedPulsesDetectOnePulsePerIncreaseAndCoalesce()
    {
        const auto entriesFor = [](int minimized) {
            QVariantList windows;
            for (int i = 0; i < 3; ++i)
                windows.append(QVariantMap{{QStringLiteral("minimized"), i < minimized}});
            QVariantList entries;
            entries.append(QVariantMap{{QStringLiteral("id"), QStringLiteral("a")},
                                       {QStringLiteral("kind"), QStringLiteral("pinned")},
                                       {QStringLiteral("windowList"), windows}});
            entries.append(QVariantMap{{QStringLiteral("id"), QStringLiteral("b")},
                                       {QStringLiteral("kind"), QStringLiteral("temporary")},
                                       {QStringLiteral("windowList"), QVariantList{}}});
            return entries;
        };
        const QHash<QString, int> counts = dockMinimizedCounts(entriesFor(0));
        QCOMPARE(counts.value("a"), 0);
        QCOMPARE(counts.value("b"), 0);
        // No increase: no pulse (the seeding build and an idle projection).
        QVERIFY(dockMinimizedPulses(entriesFor(0), counts).isEmpty());
        // One window minimizes: one pulse for the owning entry.
        QCOMPARE(dockMinimizedPulses(entriesFor(1), counts), QStringList{QStringLiteral("a")});
        // A second window while the first pulse is in flight: still one pulse,
        // keyed to the same entry (the shell restarts its clock).
        const QHash<QString, int> afterFirst = dockMinimizedCounts(entriesFor(1));
        QCOMPARE(dockMinimizedPulses(entriesFor(2), afterFirst), QStringList{QStringLiteral("a")});
        // A restore lowers the count and never pulses.
        const QHash<QString, int> afterSecond = dockMinimizedCounts(entriesFor(2));
        QVERIFY(dockMinimizedPulses(entriesFor(1), afterSecond).isEmpty());
    }

    void minimizedCountsSkipThePerWindowRows()
    {
        QVariantList entries;
        entries.append(QVariantMap{{QStringLiteral("id"), QStringLiteral("a")},
                                   {QStringLiteral("kind"), QStringLiteral("pinned")},
                                   {QStringLiteral("windowList"),
                                    QVariantList{QVariantMap{{QStringLiteral("minimized"), true}}}}});
        // A per-window `minimized` row is skipped so the owning app is not
        // double-counted even though it also carries a windowList.
        entries.append(QVariantMap{{QStringLiteral("id"), QStringLiteral("win:1")},
                                   {QStringLiteral("kind"), QStringLiteral("minimized")},
                                   {QStringLiteral("windowList"),
                                    QVariantList{QVariantMap{{QStringLiteral("minimized"), true}}}}});
        const QHash<QString, int> counts = dockMinimizedCounts(entries);
        QCOMPARE(counts.value("a"), 1);
        QVERIFY(!counts.contains(QStringLiteral("win:1")));
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

    void aDockNoticeRaisesThroughTheSameNotificationPath()
    {
        MockNotificationClient client(nullptr, /*seedFixture=*/false);
        QSignalSpy banners(&client, &NotificationClient::bannersChanged);
        raiseDockNotice(&client, QStringLiteral("Could not move to Downloads"),
                        QStringLiteral("1 of 2 items could not be moved."));
        const QJsonArray view =
            QJsonDocument::fromJson(banners.takeFirst().at(0).toByteArray()).array();
        const QJsonObject banner = view.first().toObject();
        QCOMPARE(banner.value(QStringLiteral("appName")).toString(), QStringLiteral("Dock"));
        QCOMPARE(banner.value(QStringLiteral("summary")).toString(),
                 QStringLiteral("Could not move to Downloads"));
        QCOMPARE(banner.value(QStringLiteral("body")).toString(),
                 QStringLiteral("1 of 2 items could not be moved."));
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

    // -- OSD model (T-11.4a) ---------------------------------------------

    void osdVolumeAndBrightnessPresentWithTheRightKind()
    {
        OsdModel osd;
        QVERIFY(osd.presentVolume(0.4, false, 1000));
        QVERIFY(osd.visible());
        QCOMPARE(osd.kind(), OsdModel::Kind::Volume);
        QCOMPARE(osd.kindName(), QStringLiteral("volume"));
        QVERIFY(qAbs(osd.value() - 0.4) < 1e-9);
        QCOMPARE(osd.muted(), false);
        QCOMPARE(osd.deadline(), qint64(1000) + OsdModel::kDismissMs);

        QVERIFY(osd.presentBrightness(0.75, 2000));
        QCOMPARE(osd.kind(), OsdModel::Kind::Brightness);
        QCOMPARE(osd.kindName(), QStringLiteral("brightness"));
        QVERIFY(qAbs(osd.value() - 0.75) < 1e-9);

        // Values are clamped to the unit range.
        osd.presentBrightness(3.0, 3000);
        QCOMPARE(osd.value(), 1.0);
        osd.presentVolume(-2.0, false, 4000);
        QCOMPARE(osd.value(), 0.0);

        // A present with no kind is refused, never visible.
        OsdModel fresh;
        QVERIFY(!fresh.present(OsdModel::Kind::None, 0.5, false, 0));
        QVERIFY(!fresh.visible());
    }

    void osdAutoDismissesAtTheDeadline()
    {
        OsdModel osd;
        osd.presentVolume(0.5, false, 1000);
        // Still visible just before the deadline.
        QVERIFY(osd.tick(1000 + OsdModel::kDismissMs - 1));
        QVERIFY(osd.visible());
        // Dismissed at and after the deadline.
        QVERIFY(!osd.tick(1000 + OsdModel::kDismissMs));
        QVERIFY(!osd.visible());
    }

    void osdFullscreenSuppressesAndHidesImmediately()
    {
        OsdModel osd;
        osd.setFullscreen(true);
        // A change while fullscreen is refused and never shown.
        QVERIFY(!osd.presentVolume(0.5, false, 1000));
        QVERIFY(!osd.visible());

        // Leaving fullscreen lets the next change show.
        osd.setFullscreen(false);
        QVERIFY(osd.presentVolume(0.5, false, 2000));
        QVERIFY(osd.visible());

        // Entering fullscreen while visible hides it at once.
        osd.setFullscreen(true);
        QVERIFY(!osd.visible());
    }

    void osdReducedMotionJumpsTheFade()
    {
        OsdModel osd;
        osd.presentVolume(0.5, false, 1000);
        // No reduced motion: the fade ramps in from zero, holds, ramps out.
        QCOMPARE(osd.fade(1000, false), 0.0);
        QVERIFY(osd.fade(1000 + OsdModel::kFadeInMs / 2, false) > 0.0);
        QCOMPARE(osd.fade(1000 + OsdModel::kFadeInMs, false), 1.0);
        QCOMPARE(osd.fade(1000 + OsdModel::kDismissMs, false), 0.0);
        QVERIFY(osd.fade(1000 + OsdModel::kDismissMs - 1, false) > 0.0);

        // Reduced motion is immediately and always fully visible.
        QCOMPARE(osd.fade(1000, true), 1.0);
        QCOMPARE(osd.fade(1000 + OsdModel::kDismissMs - 1, true), 1.0);

        // A hidden OSD fades to nothing on either path.
        osd.hide();
        QCOMPARE(osd.fade(1000 + OsdModel::kFadeInMs, true), 0.0);
        QCOMPARE(osd.fade(1000 + OsdModel::kFadeInMs, false), 0.0);
    }

    void osdConcurrentTriggersCoalesceToTheLast()
    {
        OsdModel osd;
        osd.presentVolume(0.2, false, 1000);
        // The brightness change re-arms the same display: one OSD, not two.
        osd.presentBrightness(0.9, 1000);
        QCOMPARE(osd.kind(), OsdModel::Kind::Brightness);
        QVERIFY(qAbs(osd.value() - 0.9) < 1e-9);
        QCOMPARE(osd.deadline(), qint64(1000) + OsdModel::kDismissMs);
    }

    // -- Menu-broker fixed application menu (T-14.2a) -------------------

    // An app-level running entry, the shape `buildDockProjection` emits.
    static QVariantMap runningApp(const QString &appId, bool allMinimized)
    {
        return QVariantMap{
            { QStringLiteral("kind"), QStringLiteral("temporary") },
            { QStringLiteral("appId"), appId },
            { QStringLiteral("running"), true },
            { QStringLiteral("windows"), 1 },
            { QStringLiteral("minimized"), allMinimized },
        };
    }

    void theFixedApplicationMenuCarriesTheStandardRows()
    {
        const QVariantList menu = fixedApplicationMenu(
            QStringLiteral("Settings"),
            AppMenuLiveState{ false, false, false });
        QCOMPARE(menu.size(), 8);
        QCOMPARE(menu[0].toMap().value(QStringLiteral("label")).toString(),
                 QStringLiteral("About Settings"));
        QCOMPARE(menu[2].toMap().value(QStringLiteral("type")).toString(),
                 QStringLiteral("separator"));
        QCOMPARE(menu[3].toMap().value(QStringLiteral("label")).toString(),
                 QStringLiteral("Hide Settings"));
        QCOMPARE(menu[3].toMap().value(QStringLiteral("shortcut")).toString(),
                 QStringLiteral("Super+H"));
        QCOMPARE(menu[7].toMap().value(QStringLiteral("label")).toString(),
                 QStringLiteral("Quit Settings"));
        // A disabled verb is published explicitly so the bar dims it.
        QCOMPARE(menu[3].toMap().value(QStringLiteral("enabled")).toBool(), false);
        QCOMPARE(menu[0].toMap().value(QStringLiteral("enabled")).toBool(), true);
    }

    void theHideVerbsFollowTheLiveWindowProjection()
    {
        // Two visible apps, focused on the first: Hide + Hide Others.
        QVariantList entries{ runningApp(QStringLiteral("a"), false),
                              runningApp(QStringLiteral("b"), false) };
        AppMenuLiveState state =
            appMenuLiveState(QStringLiteral("a"), entries);
        QCOMPARE(state.hide, true);
        QCOMPARE(state.hideOthers, true);
        QCOMPARE(state.showAll, false);

        // Hide everything: only Show All stays meaningful.
        entries = { runningApp(QStringLiteral("a"), true),
                    runningApp(QStringLiteral("b"), true) };
        state = appMenuLiveState(QStringLiteral("a"), entries);
        QCOMPARE(state.hide, false);
        QCOMPARE(state.hideOthers, false);
        QCOMPARE(state.showAll, true);

        // Per-window minimized entries must not double-count the app state.
        QVariantList withWindows{
            runningApp(QStringLiteral("a"), false),
            QVariantMap{ { QStringLiteral("kind"), QStringLiteral("minimized") },
                         { QStringLiteral("appId"), QStringLiteral("a") },
                         { QStringLiteral("minimized"), true } },
        };
        state = appMenuLiveState(QStringLiteral("a"), withWindows);
        QCOMPARE(state.hide, true);
        QCOMPARE(state.showAll, false);

        // The empty desktop has nothing focused to hide.
        state = appMenuLiveState(QString(), entries);
        QCOMPARE(state.hide, false);
        QCOMPARE(state.hideOthers, false);
        QCOMPARE(state.showAll, true);
    }

    void applyingLiveStateRewritesOnlyTheHideRows()
    {
        const QVariantList published{
            QVariantMap{ { QStringLiteral("label"), QStringLiteral("About X") },
                         { QStringLiteral("action"), QStringLiteral("about") } },
            QVariantMap{ { QStringLiteral("label"), QStringLiteral("Hide X") },
                         { QStringLiteral("action"), QStringLiteral("hide") } },
            QVariantMap{ { QStringLiteral("label"), QStringLiteral("Hide Others") },
                         { QStringLiteral("action"), QStringLiteral("hide-others") } },
            QVariantMap{ { QStringLiteral("label"), QStringLiteral("Show All") },
                         { QStringLiteral("action"), QStringLiteral("show-all") },
                         { QStringLiteral("checked"), true } },
        };
        const QVariantList menu =
            applyAppMenuLiveState(published, AppMenuLiveState{ true, false, true });
        QCOMPARE(menu.size(), 4);
        // About keeps its declaration untouched (no enabled flag).
        QCOMPARE(menu[0].toMap().contains(QStringLiteral("enabled")), false);
        QCOMPARE(menu[1].toMap().value(QStringLiteral("enabled")).toBool(), true);
        QCOMPARE(menu[2].toMap().value(QStringLiteral("enabled")).toBool(), false);
        QCOMPARE(menu[3].toMap().value(QStringLiteral("enabled")).toBool(), true);
        // An unrelated field survives the rewrite.
        QCOMPARE(menu[3].toMap().value(QStringLiteral("checked")).toBool(), true);
    }

    // T-14.7: the shell pushes this payload to SetWindowStates. Only app-level
    // entries are apps; per-window minimized entries are skipped.
    void windowStatesJsonCarriesOneRowPerRunningApp()
    {
        const QVariantList entries{
            runningApp(QStringLiteral("org.dragonfruit.Settings"), false),
            QVariantMap{ { QStringLiteral("kind"), QStringLiteral("minimized") },
                         { QStringLiteral("appId"), QStringLiteral("org.dragonfruit.Settings") },
                         { QStringLiteral("minimized"), true } },
            runningApp(QStringLiteral("firefox"), true),
        };
        const QString json = windowStatesJson(entries);
        const QJsonArray array = QJsonDocument::fromJson(json.toUtf8()).array();
        QCOMPARE(array.size(), 2);
        QCOMPARE(array[0].toObject().value(QStringLiteral("appId")).toString(),
                 QStringLiteral("org.dragonfruit.Settings"));
        QCOMPARE(array[0].toObject().value(QStringLiteral("minimized")).toBool(), false);
        QCOMPARE(array[1].toObject().value(QStringLiteral("appId")).toString(),
                 QStringLiteral("firefox"));
        QCOMPARE(array[1].toObject().value(QStringLiteral("minimized")).toBool(), true);
    }

    // T-14.7: the broker's `ResolveFocused` reply decodes to the fixed menu
    // plus the app's exported top-level menus.
    void resolvedMenuDecodesTheBrokerReply()
    {
        const QByteArray json = R"({
            "appId": "org.dragonfruit.Settings",
            "appName": "Settings",
            "tier": "native",
            "applicationMenuItems": [
                { "label": "Hide Settings", "action": "hide", "enabled": true },
                { "label": "Quit Settings", "action": "quit" }
            ],
            "menus": [ { "title": "File", "items": [ { "label": "Close",
                       "action": "close" } ] } ]
        })";
        const ResolvedMenu resolved = MenuBrokerClient::parseResolved(QString::fromUtf8(json));
        QVERIFY(resolved.valid);
        QCOMPARE(resolved.appId, QStringLiteral("org.dragonfruit.Settings"));
        QCOMPARE(resolved.appName, QStringLiteral("Settings"));
        QCOMPARE(resolved.tier, QStringLiteral("native"));
        QCOMPARE(resolved.applicationMenuItems.size(), 2);
        QCOMPARE(resolved.menus.size(), 1);
        QCOMPARE(resolved.menus[0].toMap().value(QStringLiteral("title")).toString(),
                 QStringLiteral("File"));

        // A miss (empty reply or garbage) is invalid, never a phantom menu.
        QVERIFY(!MenuBrokerClient::parseResolved(QString()).valid);
        QVERIFY(!MenuBrokerClient::parseResolved(QStringLiteral("not json")).valid);
        QVERIFY(!MenuBrokerClient::parseResolved(QStringLiteral("[1,2]")).valid);
    }

    // -- Menu-broker accelerators (T-14.2b) ------------------------------

    void acceleratorsFlattenFixedAndOwnMenusThroughSubmenus()
    {
        const QVariantList applicationMenu{
            QVariantMap{ { QStringLiteral("label"), QStringLiteral("Settings") },
                         { QStringLiteral("shortcut"), QStringLiteral("Super+,") },
                         { QStringLiteral("action"), QStringLiteral("settings") } },
            QVariantMap{ { QStringLiteral("type"), QStringLiteral("separator") } },
            QVariantMap{ { QStringLiteral("label"), QStringLiteral("Quit") },
                         { QStringLiteral("shortcut"), QStringLiteral("Super+Q") },
                         { QStringLiteral("action"), QStringLiteral("quit") } },
        };
        const QVariantList appMenu{
            QVariantMap{
                { QStringLiteral("title"), QStringLiteral("Edit") },
                { QStringLiteral("items"),
                  QVariantList{
                      QVariantMap{ { QStringLiteral("label"), QStringLiteral("Undo") },
                                   { QStringLiteral("shortcut"), QStringLiteral("Super+Z") },
                                   { QStringLiteral("action"), QStringLiteral("edit.undo") } },
                      // A shortcut without an action does not register.
                      QVariantMap{ { QStringLiteral("label"), QStringLiteral("Redo") },
                                   { QStringLiteral("shortcut"), QStringLiteral("Super+Shift+Z") } },
                  } } },
            QVariantMap{
                { QStringLiteral("title"), QStringLiteral("View") },
                { QStringLiteral("items"),
                  QVariantList{
                      QVariantMap{ { QStringLiteral("label"), QStringLiteral("Sort") },
                                   { QStringLiteral("type"), QStringLiteral("submenu") },
                                   { QStringLiteral("submenu"),
                                     QVariantList{ QVariantMap{
                                         { QStringLiteral("label"), QStringLiteral("Name") },
                                         { QStringLiteral("shortcut"), QStringLiteral("Ctrl+N") },
                                         { QStringLiteral("action"),
                                           QStringLiteral("sort.name") } } } } },
                  } } },
        };

        const QList<MenuAccelerator> accelerators =
            publishedAccelerators(applicationMenu, appMenu);
        QCOMPARE(accelerators.size(), 4);
        QCOMPARE(accelerators[0].action, QStringLiteral("settings"));
        QCOMPARE(accelerators[0].chord, QStringLiteral("Super+,"));
        QCOMPARE(accelerators[2].action, QStringLiteral("edit.undo"));
        // The nested submenu action is not dropped.
        QCOMPARE(accelerators[3].action, QStringLiteral("sort.name"));
        QCOMPARE(accelerators[3].chord, QStringLiteral("Ctrl+N"));

        // The wire form is one `action<TAB>chord` per line, the shape
        // `df_toplevel_manager.set_app_accelerators` expects.
        QCOMPARE(acceleratorWireTable(accelerators),
                 QStringLiteral("settings\tSuper+,\nquit\tSuper+Q\n"
                                "edit.undo\tSuper+Z\nsort.name\tCtrl+N"));
        QCOMPARE(acceleratorWireTable({}), QString());
    }

    // T-14.3: the tray item and DBusMenu views decode from the service's flat
    // JSON, and a malformed payload never produces a phantom item.
    void tray_items_and_menus_decode_from_the_service_view()
    {
        const QString itemsJson = QStringLiteral(R"([
            { "name": "org.kde.StatusNotifierItem-1-1", "id": "telegram",
              "title": "Telegram", "iconName": "telegram", "iconPath": "/tmp/telegram.png",
              "menuPath": "/MenuBar", "itemIsMenu": true, "needsAttention": true,
              "hasPixmap": false },
            { "name": ":1.9:/SelfItem", "id": "self", "title": "", "iconName": "",
              "iconPath": "", "menuPath": "", "itemIsMenu": false,
              "needsAttention": false, "hasPixmap": true }
        ])");
        const QList<TrayItem> items = TrayClient::parseItems(itemsJson);
        QCOMPARE(items.size(), 2);
        QCOMPARE(items[0].name, QStringLiteral("org.kde.StatusNotifierItem-1-1"));
        QCOMPARE(items[0].title, QStringLiteral("Telegram"));
        QCOMPARE(items[0].iconPath, QStringLiteral("/tmp/telegram.png"));
        QCOMPARE(items[0].itemIsMenu, true);
        QCOMPARE(items[0].needsAttention, true);
        QCOMPARE(items[1].name, QStringLiteral(":1.9:/SelfItem"));
        QCOMPARE(items[1].hasPixmap, true);

        // A missing/empty name is not an item; a malformed payload is empty.
        QCOMPARE(TrayClient::parseItems(QStringLiteral("[ { \"id\": \"x\" } ]")).size(), 0);
        QCOMPARE(TrayClient::parseItems(QStringLiteral("{ not json")).size(), 0);
        QCOMPARE(TrayClient::parseItems(QString()).size(), 0);

        // The menu view is the design-system row shape, nested `submenu`
        // intact, and the click id survives the round trip.
        const QString menuJson = QStringLiteral(R"([
            { "id": 1, "type": "item", "label": "Show Window", "enabled": true },
            { "id": 5, "type": "submenu", "label": "Tools", "enabled": true,
              "submenu": [ { "id": 6, "type": "item", "label": "Preferences",
                             "checked": true, "checkable": true } ] },
            { "id": 9, "type": "separator", "label": "" }
        ])");
        const QVariantList menu = TrayClient::parseMenu(menuJson);
        QCOMPARE(menu.size(), 3);
        const QVariantMap first = menu.at(0).toMap();
        QCOMPARE(first.value(QStringLiteral("id")).toInt(), 1);
        QCOMPARE(first.value(QStringLiteral("label")).toString(), QStringLiteral("Show Window"));
        const QVariantMap submenu = menu.at(1).toMap();
        QCOMPARE(submenu.value(QStringLiteral("type")).toString(), QStringLiteral("submenu"));
        const QVariantList children =
            submenu.value(QStringLiteral("submenu")).toList();
        QCOMPARE(children.size(), 1);
        QCOMPARE(children.at(0).toMap().value(QStringLiteral("id")).toInt(), 6);
        QCOMPARE(children.at(0).toMap().value(QStringLiteral("checked")).toBool(), true);

        QCOMPARE(TrayClient::parseMenu(QStringLiteral("[]")).size(), 0);
        QCOMPARE(TrayClient::parseMenu(QStringLiteral("not json")).size(), 0);
    }

    // -- Add Application picker (T-14.7e) --------------------------------

    void appPickerDropsHiddenAndNonLaunchableAndDedupes()
    {
        DesktopEntry hidden = makeEntry(QStringLiteral("hidden.desktop"),
                                        QStringLiteral("Hidden"), QStringLiteral("x"), {}, {},
                                        false, true);
        DesktopEntry noExec = makeEntry(QStringLiteral("noexec.desktop"),
                                        QStringLiteral("No Exec"), QString());
        DesktopEntry dupFirst = makeEntry(QStringLiteral("dup.desktop"),
                                          QStringLiteral("First"), QStringLiteral("first"));
        DesktopEntry dupSecond = makeEntry(QStringLiteral("dup.desktop"),
                                           QStringLiteral("Second"), QStringLiteral("second"));
        const QList<AppPickerRow> rows = buildAppPickerList(
            {hidden, noExec, dupFirst, dupSecond}, {});
        QCOMPARE(rows.size(), 1);
        QCOMPARE(rows[0].desktopId, QStringLiteral("dup.desktop"));
        QCOMPARE(rows[0].name, QStringLiteral("First"));
    }

    void appPickerSortsByNameWithIdTiebreakAndTagsPinned()
    {
        DesktopEntry gamma = makeEntry(QStringLiteral("g.desktop"), QStringLiteral("Gamma"),
                                       QStringLiteral("g"));
        DesktopEntry alpha = makeEntry(QStringLiteral("a.desktop"), QStringLiteral("Alpha"),
                                       QStringLiteral("a"));
        DesktopEntry sameB = makeEntry(QStringLiteral("b.desktop"), QStringLiteral("Same"),
                                       QStringLiteral("b"));
        DesktopEntry sameA = makeEntry(QStringLiteral("a-same.desktop"), QStringLiteral("Same"),
                                       QStringLiteral("sa"));
        const QList<AppPickerRow> rows = buildAppPickerList(
            {gamma, alpha, sameB, sameA}, {QStringLiteral("g.desktop")});
        QCOMPARE(rows.size(), 4);
        QCOMPARE(rows[0].name, QStringLiteral("Alpha"));
        QCOMPARE(rows[0].pinned, false);
        QCOMPARE(rows[1].name, QStringLiteral("Gamma"));
        QCOMPARE(rows[1].pinned, true);
        // Name tie breaks on the desktop id, lexicographically.
        QCOMPARE(rows[2].desktopId, QStringLiteral("a-same.desktop"));
        QCOMPARE(rows[3].desktopId, QStringLiteral("b.desktop"));
    }

    void appPickerFiltersByQueryOverNameAndIdCaseInsensitively()
    {
        DesktopEntry files = makeEntry(QStringLiteral("org.dragonfruit.Files.desktop"),
                                       QStringLiteral("Files"), QStringLiteral("df-files"));
        DesktopEntry settings = makeEntry(QStringLiteral("org.dragonfruit.Settings.desktop"),
                                          QStringLiteral("Settings"), QStringLiteral("df-settings"));
        const QList<DesktopEntry> corpus = {files, settings};

        QCOMPARE(buildAppPickerList(corpus, {}, QStringLiteral("set")).size(), 1);
        QCOMPARE(buildAppPickerList(corpus, {}, QStringLiteral("SET")).size(), 1);
        QCOMPARE(buildAppPickerList(corpus, {}, QStringLiteral("dragonfruit")).size(), 2);
        // A query over the id, not the name.
        QCOMPARE(buildAppPickerList(corpus, {}, QStringLiteral("files.desktop")).size(), 1);
        QCOMPARE(buildAppPickerList(corpus, {}, QStringLiteral("  files  "))[0].desktopId,
                 QStringLiteral("org.dragonfruit.Files.desktop"));
        QCOMPARE(buildAppPickerList(corpus, {}, QStringLiteral("nothing")).size(), 0);
        // Empty/whitespace query returns the whole corpus.
        QCOMPARE(buildAppPickerList(corpus, {}).size(), 2);
        QCOMPARE(buildAppPickerList(corpus, {}, QStringLiteral("   ")).size(), 2);
    }

    void appPickerPinToggleAddsRemovesAndNeverDuplicates()
    {
        const QStringList base = {QStringLiteral("a.desktop"), QStringLiteral("b.desktop")};
        QCOMPARE(toggleAppPickerPin(base, QStringLiteral("c.desktop"), true),
                 (QStringList{QStringLiteral("a.desktop"), QStringLiteral("b.desktop"),
                              QStringLiteral("c.desktop")}));
        // Re-pinning is a no-op (the canonical single-writer value).
        QCOMPARE(toggleAppPickerPin(base, QStringLiteral("a.desktop"), true), base);
        QCOMPARE(toggleAppPickerPin(base, QStringLiteral("a.desktop"), false),
                 QStringList{QStringLiteral("b.desktop")});
        // Unpinning an absent id is a no-op.
        QCOMPARE(toggleAppPickerPin(base, QStringLiteral("z.desktop"), false), base);
        QCOMPARE(toggleAppPickerPin(base, QString(), true), base);
    }

    // -- launch-origin tile hand-off (T-14.7l) ----------------------------

    void dockLaunchAppIdPrefersStartupWmClass()
    {
        const DesktopEntry files = makeEntry(QStringLiteral("org.dragonfruit.Files.desktop"),
                                             QStringLiteral("Files"), QStringLiteral("df-files"),
                                             QStringLiteral("dragonfruit-files"));
        QCOMPARE(dockLaunchAppId(files), QStringLiteral("dragonfruit-files"));
    }

    void dockLaunchAppIdFallsBackToTheIdStem()
    {
        DesktopEntry gtk = makeEntry(QStringLiteral("org.example.Nautilus.desktop"),
                                     QStringLiteral("Files"), QStringLiteral("nautilus"));
        QCOMPARE(dockLaunchAppId(gtk), QStringLiteral("org.example.Nautilus"));
        // A suffixless id is its own stem.
        DesktopEntry bare = makeEntry(QStringLiteral("xterm"), QStringLiteral("XTerm"),
                                      QStringLiteral("xterm"));
        QCOMPARE(dockLaunchAppId(bare), QStringLiteral("xterm"));
        // A blank record yields no key; the launch keeps the centered fallback.
        QVERIFY(dockLaunchAppId(DesktopEntry{}).isEmpty());
        // Whitespace-only StartupWMClass falls through to the id.
        DesktopEntry padded = makeEntry(QStringLiteral("a.desktop"), QStringLiteral("A"),
                                        QStringLiteral("a"), QStringLiteral("   "));
        QCOMPARE(dockLaunchAppId(padded), QStringLiteral("a"));
    }

    void dockTileRectsKeepTheLastRectPerIdentityAndStayBounded()
    {
        DockTileRects tiles;
        QVERIFY(!tiles.rectFor(QStringLiteral("a")).isValid());
        tiles.record(QStringLiteral("a"), QRect(1, 2, 3, 4));
        QCOMPARE(tiles.rectFor(QStringLiteral("a")), QRect(1, 2, 3, 4));
        // Re-recording refreshes in place, without growing the map.
        tiles.record(QStringLiteral("a"), QRect(10, 20, 30, 40));
        QCOMPARE(tiles.rectFor(QStringLiteral("a")), QRect(10, 20, 30, 40));
        QCOMPARE(tiles.size(), 1);
        // A garbage rect is ignored rather than handed to the compositor.
        tiles.record(QStringLiteral("empty"), QRect(5, 5, 0, 10));
        QCOMPARE(tiles.size(), 1);

        for (int i = 0; i < DockTileRects::kCapacity + 5; ++i)
            tiles.record(QStringLiteral("id%1").arg(i), QRect(i, i, 8, 8));
        QCOMPARE(tiles.size(), DockTileRects::kCapacity);
        // The oldest keys were evicted; the newest survive.
        QVERIFY(!tiles.rectFor(QStringLiteral("id0")).isValid());
        QVERIFY(tiles.rectFor(QStringLiteral("id%1").arg(DockTileRects::kCapacity + 4)).isValid());
        tiles.clear();
        QCOMPARE(tiles.size(), 0);
    }

    void clampDockTileRectBoundsToTheOutput()
    {
        const QRect output(0, 0, 1280, 800);
        // Inside is untouched.
        QCOMPARE(clampDockTileRect(QRect(100, 700, 48, 48), output), QRect(100, 700, 48, 48));
        // Partly outside is cropped.
        QCOMPARE(clampDockTileRect(QRect(1260, 780, 48, 48), output), QRect(1260, 780, 20, 20));
        // Fully outside is dropped so the caller keeps the centered fallback.
        QVERIFY(!clampDockTileRect(QRect(2000, 2000, 48, 48), output).isValid());
        // An unknown output leaves the rect alone.
        QCOMPARE(clampDockTileRect(QRect(5, 6, 7, 8), QRect()), QRect(5, 6, 7, 8));
    }
};

QTEST_GUILESS_MAIN(TestDockCore)
#include "tst_dockcore.moc"
