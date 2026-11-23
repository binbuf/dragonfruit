// SPDX-License-Identifier: MIT
// Opening a file through the real platform seam: the MIME default is resolved
// from the desktop database, a stale/missing default falls back to an
// installed handler, and the handler receives the file path.
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QProcess>
#include <QStandardPaths>
#include <QTemporaryDir>
#include <QUrl>
#include <QtTest>

#include "FilesBridge.h"
#include "OpenHandler.h"

class FilesOpenTest : public QObject
{
    Q_OBJECT

private slots:
    void opens_with_the_configured_default_handler()
    {
        if (QStandardPaths::findExecutable(QStringLiteral("gtk-launch")).isEmpty()
            && QStandardPaths::findExecutable(QStringLiteral("gio")).isEmpty()) {
            QSKIP("neither gtk-launch nor gio is installed");
        }

        QTemporaryDir root;
        QVERIFY(root.isValid());
        const QString data = root.path() + QStringLiteral("/data");
        const QString config = root.path() + QStringLiteral("/config");
        const QString cache = root.path() + QStringLiteral("/cache");
        QVERIFY(QDir().mkpath(data + QStringLiteral("/applications")));
        QVERIFY(QDir().mkpath(config));
        QVERIFY(QDir().mkpath(cache));

        const QString marker = root.path() + QStringLiteral("/opened");
        const QString envMarker = root.path() + QStringLiteral("/opened-env");
        const QString script = root.path() + QStringLiteral("/handler.sh");
        {
            QFile handle(script);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write(QStringLiteral("#!/bin/sh\n"
                                        "printf '%s\\n' \"$1\" > %1\n"
                                        "printf '%s\\n' \"$DISPLAY\" > %2\n")
                                 .arg(marker, envMarker)
                                 .toUtf8());
        }
        QVERIFY(QFile::setPermissions(
                script, QFileDevice::ReadOwner | QFileDevice::WriteOwner
                                | QFileDevice::ExeOwner));

        const QString desktop =
                data + QStringLiteral("/applications/df-test-handler.desktop");
        {
            QFile handle(desktop);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write(QStringLiteral(
                                 "[Desktop Entry]\n"
                                 "Type=Application\n"
                                 "Name=Dragonfruit Test Handler\n"
                                 "Exec=%1 %f\n"
                                 "MimeType=text/plain;\n"
                                 "NoDisplay=true\n")
                                 .arg(script)
                                 .toUtf8());
        }
        {
            QFile handle(data + QStringLiteral("/applications/mimeapps.list"));
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write(QByteArray(
                    "[Default Applications]\n"
                    "text/plain=df-test-handler.desktop\n"));
        }

        const QString file = root.path() + QStringLiteral("/sample.txt");
        {
            QFile handle(file);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write("hello");
        }

        // The compositor's Xwayland hand-off, plus a stale DISPLAY in the
        // parent environment. The handler must receive the file's value, not
        // the inherited one (T309).
        const QString runtime = root.path() + QStringLiteral("/runtime");
        QVERIFY(QDir().mkpath(runtime));
        const QString socket = QStringLiteral("df-test-wayland");
        {
            QFile handle(runtime + QLatin1Char('/') + socket
                         + QStringLiteral(".x11-display"));
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write(":42\n");
        }

        const QByteArray oldDataHome = qgetenv("XDG_DATA_HOME");
        const QByteArray oldConfigHome = qgetenv("XDG_CONFIG_HOME");
        const QByteArray oldCacheHome = qgetenv("XDG_CACHE_HOME");
        const QByteArray oldRuntimeDir = qgetenv("XDG_RUNTIME_DIR");
        const QByteArray oldWaylandDisplay = qgetenv("WAYLAND_DISPLAY");
        const QByteArray oldDisplay = qgetenv("DISPLAY");
        qputenv("XDG_DATA_HOME", data.toUtf8());
        qputenv("XDG_CONFIG_HOME", config.toUtf8());
        qputenv("XDG_CACHE_HOME", cache.toUtf8());
        qputenv("XDG_RUNTIME_DIR", runtime.toUtf8());
        qputenv("WAYLAND_DISPLAY", socket.toUtf8());
        qputenv("DISPLAY", ":99");

        FilesBridge bridge;
        const bool opened = bridge.open(QUrl::fromLocalFile(file).toString());

        // Restore the environment even if the assertions below fail.
        if (oldDataHome.isNull())
            qunsetenv("XDG_DATA_HOME");
        else
            qputenv("XDG_DATA_HOME", oldDataHome);
        if (oldConfigHome.isNull())
            qunsetenv("XDG_CONFIG_HOME");
        else
            qputenv("XDG_CONFIG_HOME", oldConfigHome);
        if (oldCacheHome.isNull())
            qunsetenv("XDG_CACHE_HOME");
        else
            qputenv("XDG_CACHE_HOME", oldCacheHome);
        if (oldRuntimeDir.isNull())
            qunsetenv("XDG_RUNTIME_DIR");
        else
            qputenv("XDG_RUNTIME_DIR", oldRuntimeDir);
        if (oldWaylandDisplay.isNull())
            qunsetenv("WAYLAND_DISPLAY");
        else
            qputenv("WAYLAND_DISPLAY", oldWaylandDisplay);
        if (oldDisplay.isNull())
            qunsetenv("DISPLAY");
        else
            qputenv("DISPLAY", oldDisplay);

        QVERIFY(opened);
        QTRY_VERIFY_WITH_TIMEOUT(QFileInfo::exists(marker), 5000);
        QFile handle(marker);
        QVERIFY(handle.open(QIODevice::ReadOnly));
        QCOMPARE(QString::fromUtf8(handle.readAll()).trimmed(), file);

        QTRY_VERIFY_WITH_TIMEOUT(QFileInfo::exists(envMarker), 5000);
        QFile envHandle(envMarker);
        QVERIFY(envHandle.open(QIODevice::ReadOnly));
        QCOMPARE(QString::fromUtf8(envHandle.readAll()).trimmed(),
                 QStringLiteral(":42"));
    }

    // The child environment itself, independent of which opener is installed:
    // present/absent/empty hand-off files and the offscreen-QPA scrub.
    void desktop_child_environment_reads_the_display_handoff()
    {
        QTemporaryDir runtime;
        QVERIFY(runtime.isValid());
        const QString socket = QStringLiteral("df-test-wayland");
        const QString handoff =
            runtime.path() + QLatin1Char('/') + socket + QStringLiteral(".x11-display");

        // Set the parent environment, read the child environment, and restore
        // the parent before returning, so a failed assertion cannot leak the
        // fake hand-off into another test.
        const auto childEnvironment = [&](const QByteArray &contents, bool write) {
            const QByteArray oldRuntimeDir = qgetenv("XDG_RUNTIME_DIR");
            const QByteArray oldWaylandDisplay = qgetenv("WAYLAND_DISPLAY");
            const QByteArray oldDisplay = qgetenv("DISPLAY");
            const QByteArray oldPlatform = qgetenv("QT_QPA_PLATFORM");
            qputenv("XDG_RUNTIME_DIR", runtime.path().toUtf8());
            qputenv("WAYLAND_DISPLAY", socket.toUtf8());
            qputenv("DISPLAY", ":99");
            qputenv("QT_QPA_PLATFORM", "offscreen");
            if (write) {
                QFile handle(handoff);
                if (handle.open(QIODevice::WriteOnly))
                    handle.write(contents);
            } else {
                QFile::remove(handoff);
            }
            const QProcessEnvironment environment = filesDesktopChildEnvironment();
            if (oldRuntimeDir.isNull())
                qunsetenv("XDG_RUNTIME_DIR");
            else
                qputenv("XDG_RUNTIME_DIR", oldRuntimeDir);
            if (oldWaylandDisplay.isNull())
                qunsetenv("WAYLAND_DISPLAY");
            else
                qputenv("WAYLAND_DISPLAY", oldWaylandDisplay);
            if (oldDisplay.isNull())
                qunsetenv("DISPLAY");
            else
                qputenv("DISPLAY", oldDisplay);
            if (oldPlatform.isNull())
                qunsetenv("QT_QPA_PLATFORM");
            else
                qputenv("QT_QPA_PLATFORM", oldPlatform);
            return environment;
        };

        QProcessEnvironment environment = childEnvironment(":5\n", true);
        QCOMPARE(environment.value(QStringLiteral("DISPLAY")), QStringLiteral(":5"));
        QCOMPARE(environment.value(QStringLiteral("WAYLAND_DISPLAY")), socket);
        QVERIFY(!environment.contains(QStringLiteral("QT_QPA_PLATFORM")));

        environment = childEnvironment({}, false);
        QVERIFY(!environment.contains(QStringLiteral("DISPLAY")));

        environment = childEnvironment("\n", true);
        QVERIFY(!environment.contains(QStringLiteral("DISPLAY")));
    }

    void an_empty_target_is_rejected()
    {
        FilesBridge bridge;
        QVERIFY(!bridge.open(QString()));
    }
};

QTEST_GUILESS_MAIN(FilesOpenTest)

#include "tst_files_open.moc"
