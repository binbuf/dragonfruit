// SPDX-License-Identifier: MIT
// Dock core unit tests (T-10): the interim `.desktop` resolver/launcher, the
// `dock.pinned` persistence, and the pure pinned+running entry merge. Runs
// headless with no compositor, Wayland, or QML.
#include "desktopentry.h"
#include "dockmodel.h"
#include "dockpins.h"

#include <QDir>
#include <QFile>
#include <QTemporaryDir>
#include <QTest>

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
                        {QStringLiteral("windows"), 2}},
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
};

QTEST_GUILESS_MAIN(TestDockCore)
#include "tst_dockcore.moc"
