// SPDX-License-Identifier: MIT
// The shell's launch-time Xwayland DISPLAY hand-off (T309). The compositor
// writes `$XDG_RUNTIME_DIR/<socket>.x11-display`; `appLaunchEnvironment` must
// read it at launch time, trim it, and remove an inherited DISPLAY when the
// file is absent or empty. The final case starts a real headless child, so the
// assertion is about the environment a launched X11 client receives, not just
// the in-memory map.
#include "desktopentry.h"

#include <QDir>
#include <QFile>
#include <QProcess>
#include <QProcessEnvironment>
#include <QTemporaryDir>
#include <QtTest>

namespace {

void writeHandoff(const QString &runtimeDir, const QString &socket,
                  const QByteArray &contents)
{
    QDir().mkpath(runtimeDir);
    QFile file(QDir(runtimeDir).filePath(socket + QStringLiteral(".x11-display")));
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write(contents);
}

QProcessEnvironment launchBase(const QString &runtimeDir, const QString &socket)
{
    QProcessEnvironment environment;
    environment.insert(QStringLiteral("XDG_RUNTIME_DIR"), runtimeDir);
    environment.insert(QStringLiteral("WAYLAND_DISPLAY"), socket);
    environment.insert(QStringLiteral("DISPLAY"), QStringLiteral(":99"));
    environment.insert(QStringLiteral("QT_QPA_PLATFORM"), QStringLiteral("offscreen"));
    environment.insert(QStringLiteral("DF_SENTINEL"), QStringLiteral("keep"));
    return environment;
}

// Run a tiny "X11 client" that prints the DISPLAY it received. This is the
// observable that matters: the child process, not the QProcessEnvironment map.
QString runHeadlessClient(const QProcessEnvironment &environment)
{
    QProcess child;
    child.setProcessEnvironment(environment);
    child.setProgram(QStringLiteral("/bin/sh"));
    child.setArguments({QStringLiteral("-c"), QStringLiteral("printf '%s' \"$DISPLAY\"")});
    child.start();
    if (!child.waitForStarted(2000))
        return QStringLiteral("<not-started>");
    if (!child.waitForFinished(2000)) {
        child.kill();
        child.waitForFinished(200);
        return QStringLiteral("<timeout>");
    }
    return QString::fromUtf8(child.readAllStandardOutput());
}

} // namespace

class TestShellLaunchEnv : public QObject
{
    Q_OBJECT

private slots:
    void handoffFileSetsDisplayAndKeepsWaylandAndQpa()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString socket = QStringLiteral("dragonfruit-wayland");
        writeHandoff(runtime.path(), socket, ":0\n");

        const QProcessEnvironment environment =
            appLaunchEnvironment(launchBase(runtime.path(), socket));
        QCOMPARE(environment.value(QStringLiteral("DISPLAY")), QStringLiteral(":0"));
        QCOMPARE(environment.value(QStringLiteral("WAYLAND_DISPLAY")), socket);
        // The shell's offscreen QPA is still scrubbed to the session platform.
        QCOMPARE(environment.value(QStringLiteral("QT_QPA_PLATFORM")),
                 QStringLiteral("wayland"));
        QCOMPARE(environment.value(QStringLiteral("DF_SENTINEL")), QStringLiteral("keep"));
    }

    void missingHandoffRemovesAnInheritedDisplay()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString socket = QStringLiteral("dragonfruit-wayland");

        const QProcessEnvironment environment =
            appLaunchEnvironment(launchBase(runtime.path(), socket));
        QVERIFY(!environment.contains(QStringLiteral("DISPLAY")));
    }

    void emptyHandoffRemovesAnInheritedDisplay()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString socket = QStringLiteral("dragonfruit-wayland");
        writeHandoff(runtime.path(), socket, "\n");

        const QProcessEnvironment environment =
            appLaunchEnvironment(launchBase(runtime.path(), socket));
        QVERIFY(!environment.contains(QStringLiteral("DISPLAY")));
    }

    void trailingWhitespaceIsTrimmed()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString socket = QStringLiteral("dragonfruit-wayland");
        writeHandoff(runtime.path(), socket, ":12\r\n\n");

        const QProcessEnvironment environment =
            appLaunchEnvironment(launchBase(runtime.path(), socket));
        QCOMPARE(environment.value(QStringLiteral("DISPLAY")), QStringLiteral(":12"));
    }

    void theConnectedSocketSelectsTheHandoffFile()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString staleSocket = QStringLiteral("wayland-0");
        const QString liveSocket = QStringLiteral("dragonfruit-nested");
        writeHandoff(runtime.path(), liveSocket, ":3\n");

        const QProcessEnvironment environment =
            appLaunchEnvironment(launchBase(runtime.path(), staleSocket), liveSocket);
        QCOMPARE(environment.value(QStringLiteral("DISPLAY")), QStringLiteral(":3"));
        QCOMPARE(environment.value(QStringLiteral("WAYLAND_DISPLAY")), liveSocket);
    }

    void theInheritedSocketIsUsedWithoutAnArgument()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString socket = QStringLiteral("dragonfruit-wayland");
        writeHandoff(runtime.path(), socket, ":4\n");

        const QProcessEnvironment environment =
            appLaunchEnvironment(launchBase(runtime.path(), socket), QString());
        QCOMPARE(environment.value(QStringLiteral("DISPLAY")), QStringLiteral(":4"));
    }

    void noSocketRemovesAnInheritedDisplay()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        QProcessEnvironment base;
        base.insert(QStringLiteral("XDG_RUNTIME_DIR"), runtime.path());
        base.insert(QStringLiteral("DISPLAY"), QStringLiteral(":99"));

        const QProcessEnvironment environment = appLaunchEnvironment(base, QString());
        QVERIFY(!environment.contains(QStringLiteral("DISPLAY")));
    }

    void aHeadlessClientReceivesTheHandoffAndNotAStaleDisplay()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString socket = QStringLiteral("dragonfruit-wayland");
        writeHandoff(runtime.path(), socket, ":7\n");

        const QProcessEnvironment environment =
            appLaunchEnvironment(launchBase(runtime.path(), socket));
        QCOMPARE(runHeadlessClient(environment), QStringLiteral(":7"));

        QVERIFY(QFile::remove(
            QDir(runtime.path()).filePath(socket + QStringLiteral(".x11-display"))));
        const QProcessEnvironment withoutHandoff =
            appLaunchEnvironment(launchBase(runtime.path(), socket));
        QCOMPARE(runHeadlessClient(withoutHandoff), QString());
    }
};

QTEST_GUILESS_MAIN(TestShellLaunchEnv)
#include "tst_shell_launch_env.moc"
