// SPDX-License-Identifier: GPL-3.0-or-later
// Terminal fallback and `Terminal=true` launching (T340). The resolver prefers
// `$TERMINAL`, then `xdg-terminal-exec`, then `x-terminal-emulator`, then a
// common terminal list; a terminal entry with no emulator returns no command
// and the shell raises the visible launch-failure notification. Pure: no
// Wayland, QML, or compositor.
#include "desktopentry.h"
#include "dockmodel.h"
#include "launchfailure.h"
#include "notificationclient.h"

#include <QDir>
#include <QFile>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QProcess>
#include <QProcessEnvironment>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QtTest>

namespace {

bool writeExecutable(const QString &dirPath, const QString &name,
                     const QByteArray &body = QByteArrayLiteral("#!/bin/sh\nexit 0\n"))
{
    QDir dir(dirPath);
    if (!dir.exists() && !QDir().mkpath(dirPath))
        return false;
    QFile file(dir.filePath(name));
    if (!file.open(QIODevice::WriteOnly | QIODevice::Truncate))
        return false;
    if (file.write(body) != body.size())
        return false;
    file.close();
    return file.setPermissions(QFileDevice::ReadOwner | QFileDevice::WriteOwner
                               | QFileDevice::ExeOwner | QFileDevice::ReadGroup
                               | QFileDevice::ExeGroup | QFileDevice::ReadOther
                               | QFileDevice::ExeOther);
}

DesktopEntry terminalEntry(const QString &exec = QStringLiteral("myapp --flag"))
{
    DesktopEntry entry;
    entry.id = QStringLiteral("org.example.Terminal.desktop");
    entry.name = QStringLiteral("Terminal App");
    entry.exec = exec;
    entry.terminal = true;
    entry.valid = true;
    return entry;
}

DesktopEntry plainEntry()
{
    DesktopEntry entry;
    entry.id = QStringLiteral("org.example.Plain.desktop");
    entry.name = QStringLiteral("Plain");
    entry.exec = QStringLiteral("plainapp --flag");
    entry.valid = true;
    return entry;
}

} // namespace

class TestDesktopEntry : public QObject
{
    Q_OBJECT

private slots:
    void configuredTerminalWins()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("customterm")));
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("xdg-terminal-exec")));
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("x-terminal-emulator")));

        QProcessEnvironment environment;
        environment.insert(QStringLiteral("TERMINAL"),
                           QStringLiteral("customterm --class Terminal"));
        const TerminalLaunch terminal = resolveTerminal(environment, {dir.path()});
        QVERIFY(terminal.found());
        QCOMPARE(terminal.program, QStringLiteral("customterm"));
        QCOMPARE(terminal.arguments,
                 (QStringList{QStringLiteral("--class"), QStringLiteral("Terminal"),
                              QStringLiteral("-e")}));
        QCOMPARE(terminal.source, QStringLiteral("TERMINAL"));

        QCOMPARE(DesktopEntryIndex::buildLaunchCommand(terminalEntry(), {}, terminal),
                 (QStringList{QStringLiteral("customterm"), QStringLiteral("--class"),
                              QStringLiteral("Terminal"), QStringLiteral("-e"),
                              QStringLiteral("myapp"), QStringLiteral("--flag")}));
    }

    void missingConfiguredTerminalFallsThroughToXdgTerminalExec()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("xdg-terminal-exec")));
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("x-terminal-emulator")));

        QProcessEnvironment environment;
        environment.insert(QStringLiteral("TERMINAL"),
                           QStringLiteral("not-installed --class Stale"));
        const TerminalLaunch terminal = resolveTerminal(environment, {dir.path()});
        QVERIFY(terminal.found());
        QCOMPARE(terminal.program, QStringLiteral("xdg-terminal-exec"));
        QCOMPARE(terminal.arguments, QStringList{QStringLiteral("--")});
        QCOMPARE(terminal.source, QStringLiteral("xdg-terminal-exec"));
        // xdg-terminal-exec owns the terminal choice and takes the command
        // directly, so the launcher does not add `-e`.
        QCOMPARE(DesktopEntryIndex::buildLaunchCommand(terminalEntry(), {}, terminal),
                 (QStringList{QStringLiteral("xdg-terminal-exec"), QStringLiteral("--"),
                              QStringLiteral("myapp"), QStringLiteral("--flag")}));
    }

    void legacyXTerminalEmulatorIsTheNextFallback()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("x-terminal-emulator")));
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("foot")));

        const TerminalLaunch terminal =
            resolveTerminal(QProcessEnvironment(), {dir.path()});
        QVERIFY(terminal.found());
        QCOMPARE(terminal.program, QStringLiteral("x-terminal-emulator"));
        QCOMPARE(terminal.arguments, QStringList{QStringLiteral("-e")});
        QCOMPARE(terminal.source, QStringLiteral("x-terminal-emulator"));
    }

    void commonTerminalListCoversEveryCandidate_data()
    {
        QTest::addColumn<QString>("name");
        QTest::addColumn<QString>("prefix");
        QTest::newRow("ptyxis") << QStringLiteral("ptyxis") << QStringLiteral("--");
        QTest::newRow("gnome-terminal")  // df-allow-desktop-name
            << QStringLiteral("gnome-terminal") << QStringLiteral("--");  // df-allow-desktop-name
        QTest::newRow("konsole") << QStringLiteral("konsole") << QStringLiteral("-e");
        QTest::newRow("alacritty")
            << QStringLiteral("alacritty") << QStringLiteral("-e");
        QTest::newRow("kitty") << QStringLiteral("kitty") << QStringLiteral("-e");
        QTest::newRow("foot") << QStringLiteral("foot") << QStringLiteral("-e");
        QTest::newRow("xterm") << QStringLiteral("xterm") << QStringLiteral("-e");
    }

    void commonTerminalListCoversEveryCandidate()
    {
        QFETCH(QString, name);
        QFETCH(QString, prefix);

        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        QVERIFY(writeExecutable(dir.path(), name));

        const TerminalLaunch terminal =
            resolveTerminal(QProcessEnvironment(), {dir.path()});
        QVERIFY(terminal.found());
        QCOMPARE(terminal.program, name);
        QCOMPARE(terminal.arguments, QStringList{prefix});
        QCOMPARE(terminal.source, QStringLiteral("common-list"));
    }

    void commonTerminalListKeepsItsPreferenceOrder()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("foot")));
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("konsole")));
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("xterm")));

        const TerminalLaunch terminal =
            resolveTerminal(QProcessEnvironment(), {dir.path()});
        QCOMPARE(terminal.program, QStringLiteral("konsole"));
    }

    void noTerminalNeverRunsTheCommandUnwrapped()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());

        QProcessEnvironment environment;
        environment.insert(QStringLiteral("TERMINAL"),
                           QStringLiteral("not-installed"));
        const TerminalLaunch terminal = resolveTerminal(environment, {dir.path()});
        QVERIFY(!terminal.found());
        QVERIFY(terminal.program.isEmpty());
        QVERIFY(terminal.arguments.isEmpty());

        QCOMPARE(DesktopEntryIndex::buildLaunchCommand(terminalEntry(), {}, terminal),
                 QStringList());
        // Only terminal entries are gated: a normal app still launches.
        QCOMPARE(DesktopEntryIndex::buildLaunchCommand(plainEntry(), {}, terminal),
                 (QStringList{QStringLiteral("plainapp"), QStringLiteral("--flag")}));

        const QString error = terminalLaunchError(terminal);
        QVERIFY(!error.isEmpty());
        QVERIFY(error.contains(QStringLiteral("terminal emulator")));
        QVERIFY(writeExecutable(dir.path(), QStringLiteral("xterm")));
        QVERIFY(terminalLaunchError(resolveTerminal(QProcessEnvironment(), {dir.path()}))
                    .isEmpty());
    }

    void terminalFixtureLaunchesTheChosenEmulator()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        // A fixture terminal records the command line it was asked to run.
        QVERIFY(writeExecutable(
            dir.path(), QStringLiteral("foot"),
            QByteArrayLiteral("#!/bin/sh\nprintf 'TERMINAL\\n'\n"
                              "for arg in \"$@\"; do printf 'ARG:%s\\n' \"$arg\"; done\n")));

        const TerminalLaunch terminal =
            resolveTerminal(QProcessEnvironment(), {dir.path()});
        QCOMPARE(terminal.program, QStringLiteral("foot"));
        const QStringList argv =
            DesktopEntryIndex::buildLaunchCommand(terminalEntry(), {}, terminal);
        QCOMPARE(argv, (QStringList{QStringLiteral("foot"), QStringLiteral("-e"),
                                    QStringLiteral("myapp"), QStringLiteral("--flag")}));

        QProcess process;
        // Run the fixture by absolute path; the argv shape is what the
        // launcher produced, and this keeps the test independent of the
        // host PATH.
        process.setProgram(QDir(dir.path()).filePath(argv.first()));
        process.setArguments(argv.mid(1));
        process.start();
        QVERIFY2(process.waitForStarted(2000), qPrintable(process.errorString()));
        QVERIFY2(process.waitForFinished(2000), qPrintable(process.errorString()));
        QCOMPARE(process.exitStatus(), QProcess::NormalExit);
        QCOMPARE(process.exitCode(), 0);
        const QString output = QString::fromLocal8Bit(process.readAllStandardOutput());
        const QStringList lines = output.split(QLatin1Char('\n'), Qt::SkipEmptyParts);
        QVERIFY(lines.contains(QStringLiteral("TERMINAL")));
        QVERIFY(lines.contains(QStringLiteral("ARG:-e")));
        QVERIFY(lines.contains(QStringLiteral("ARG:myapp")));
        QVERIFY(lines.contains(QStringLiteral("ARG:--flag")));
    }

    void absentTerminalRaisesAVisibleFailure()
    {
        QTemporaryDir dir;
        QVERIFY(dir.isValid());
        const TerminalLaunch terminal =
            resolveTerminal(QProcessEnvironment(), {dir.path()});
        QVERIFY(!terminal.found());

        MockNotificationClient client(nullptr, /*seedFixture=*/false);
        QSignalSpy banners(&client, &NotificationClient::bannersChanged);
        raiseDockLaunchFailure(&client, QStringLiteral("Terminal"),
                               terminalLaunchError(terminal));
        QCOMPARE(banners.count(), 1);
        const QJsonArray view =
            QJsonDocument::fromJson(banners.takeFirst().at(0).toByteArray()).array();
        QCOMPARE(view.size(), 1);
        const QJsonObject banner = view.first().toObject();
        QCOMPARE(banner.value(QStringLiteral("summary")).toString(),
                 QStringLiteral("Could not launch Terminal"));
        QVERIFY(banner.value(QStringLiteral("body"))
                    .toString()
                    .contains(QStringLiteral("No terminal emulator is installed")));
    }

    void defaultPinsOmitTheTerminalWithoutAnEmulator()
    {
        DesktopEntryIndex index;
        DesktopEntry files;
        files.id = QStringLiteral("org.dragonfruit.Files.desktop");
        files.name = QStringLiteral("Files");
        files.valid = true;
        DesktopEntry settings;
        settings.id = QStringLiteral("org.dragonfruit.Settings.desktop");
        settings.name = QStringLiteral("Settings");
        settings.valid = true;
        DesktopEntry terminal;
        terminal.id = QStringLiteral("term.desktop");
        terminal.name = QStringLiteral("Terminal");
        terminal.categories = {QStringLiteral("System"),
                               QStringLiteral("TerminalEmulator")};
        terminal.valid = true;
        DesktopEntry browser;
        browser.id = QStringLiteral("browse.desktop");
        browser.name = QStringLiteral("Browser");
        browser.categories = {QStringLiteral("Network"),
                              QStringLiteral("WebBrowser")};
        browser.valid = true;
        index.loadFromRecords({files, settings, terminal, browser});

        QCOMPARE(resolveDefaultDockPins(index, /*terminalAvailable=*/true),
                 (QStringList{QStringLiteral("org.dragonfruit.Files.desktop"),
                              QStringLiteral("org.dragonfruit.Settings.desktop"),
                              QStringLiteral("term.desktop"),
                              QStringLiteral("browse.desktop")}));
        QCOMPARE(resolveDefaultDockPins(index, /*terminalAvailable=*/false),
                 (QStringList{QStringLiteral("org.dragonfruit.Files.desktop"),
                              QStringLiteral("org.dragonfruit.Settings.desktop"),
                              QStringLiteral("browse.desktop")}));
    }
};

QTEST_GUILESS_MAIN(TestDesktopEntry)
#include "tst_desktopentry.moc"
