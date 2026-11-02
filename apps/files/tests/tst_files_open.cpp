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
        const QString script = root.path() + QStringLiteral("/handler.sh");
        {
            QFile handle(script);
            QVERIFY(handle.open(QIODevice::WriteOnly));
            handle.write(QStringLiteral("#!/bin/sh\nprintf '%s\\n' \"$1\" > %1\n")
                                 .arg(marker)
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

        const QByteArray oldDataHome = qgetenv("XDG_DATA_HOME");
        const QByteArray oldConfigHome = qgetenv("XDG_CONFIG_HOME");
        const QByteArray oldCacheHome = qgetenv("XDG_CACHE_HOME");
        qputenv("XDG_DATA_HOME", data.toUtf8());
        qputenv("XDG_CONFIG_HOME", config.toUtf8());
        qputenv("XDG_CACHE_HOME", cache.toUtf8());

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

        QVERIFY(opened);
        QTRY_VERIFY_WITH_TIMEOUT(QFileInfo::exists(marker), 5000);
        QFile handle(marker);
        QVERIFY(handle.open(QIODevice::ReadOnly));
        QCOMPARE(QString::fromUtf8(handle.readAll()).trimmed(), file);
    }

    void an_empty_target_is_rejected()
    {
        FilesBridge bridge;
        QVERIFY(!bridge.open(QString()));
    }
};

QTEST_GUILESS_MAIN(FilesOpenTest)

#include "tst_files_open.moc"
