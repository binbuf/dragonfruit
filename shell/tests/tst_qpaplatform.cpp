// SPDX-License-Identifier: GPL-3.0-or-later
// Qt platform-plugin (QPA) lookup (T342). The shell checks for the Wayland
// plugin before creating QGuiApplication and falls back to the offscreen QPA
// when it is absent. These tests use temporary directories; no plugin is
// loaded and no display is required.
#include "qpaplatform.h"

#include <QDir>
#include <QFile>
#include <QProcessEnvironment>
#include <QTemporaryDir>
#include <QtTest>

namespace {

bool writeFile(const QString &directory, const QString &name)
{
    QDir dir(directory);
    if (!dir.exists() && !QDir().mkpath(directory))
        return false;
    QFile file(dir.filePath(name));
    return file.open(QIODevice::WriteOnly | QIODevice::Truncate);
}

} // namespace

class TestQpaPlatform : public QObject
{
    Q_OBJECT

private slots:
    void pluginNamesCoverQtPackaging()
    {
        const QStringList names = waylandQpaPluginNames();
        QVERIFY(names.contains(QStringLiteral("libqwayland.so")));
        QVERIFY(names.contains(QStringLiteral("libqwayland-generic.so")));
        QVERIFY(names.contains(QStringLiteral("libqwayland-egl.so")));
    }

    void directoriesFollowQtLookupOrder()
    {
        QTemporaryDir temp;
        QVERIFY(temp.isValid());
        const QString first = QDir(temp.path()).filePath(QStringLiteral("first"));
        const QString second = QDir(temp.path()).filePath(QStringLiteral("second"));
        const QString qtPlugins = QDir(temp.path()).filePath(QStringLiteral("qt/plugins"));
        const QString pluginRoot = QDir(temp.path()).filePath(QStringLiteral("root"));

        QProcessEnvironment environment;
        environment.insert(QStringLiteral("QT_QPA_PLATFORM_PLUGIN_PATH"),
                           first + QDir::listSeparator() + second);
        environment.insert(QStringLiteral("QT_PLUGIN_PATH"), pluginRoot);

        const QStringList directories =
            qpaPluginDirectories(qtPlugins, environment);
        QCOMPARE(directories,
                 (QStringList{QDir::cleanPath(first), QDir::cleanPath(second),
                              QDir::cleanPath(qtPlugins + QStringLiteral("/platforms")),
                              QDir::cleanPath(pluginRoot + QStringLiteral("/platforms"))}));
    }

    void duplicateDirectoriesAreRemoved()
    {
        QTemporaryDir temp;
        QVERIFY(temp.isValid());
        const QString platforms =
            QDir(temp.path()).filePath(QStringLiteral("qt/plugins/platforms"));

        QProcessEnvironment environment;
        environment.insert(QStringLiteral("QT_QPA_PLATFORM_PLUGIN_PATH"), platforms);

        const QStringList directories = qpaPluginDirectories(
            QDir(temp.path()).filePath(QStringLiteral("qt/plugins")),
            environment);
        QCOMPARE(directories.count(QDir::cleanPath(platforms)), 1);
        QCOMPARE(directories.size(), 1);
    }

    void anyWaylandPluginFileNameCounts()
    {
        const QStringList names = waylandQpaPluginNames();
        for (const QString &name : names) {
            QTemporaryDir temp;
            QVERIFY(temp.isValid());
            QVERIFY(writeFile(temp.path(), name));
            QVERIFY2(waylandQpaPluginPresent({temp.path()}),
                     qPrintable(QStringLiteral("missing %1").arg(name)));
        }
    }

    void unrelatedFileIsNotAPlugin()
    {
        QTemporaryDir temp;
        QVERIFY(temp.isValid());
        QVERIFY(writeFile(temp.path(), QStringLiteral("libqoffscreen.so")));
        QVERIFY(!waylandQpaPluginPresent({temp.path()}));
        QVERIFY(!waylandQpaPluginPresent({}));
    }

    void platformValueSelectsWayland()
    {
        QVERIFY(qpaPlatformIsWayland(QStringLiteral("wayland")));
        QVERIFY(qpaPlatformIsWayland(QStringLiteral("Wayland")));
        QVERIFY(qpaPlatformIsWayland(QStringLiteral("wayland;xcb")));
        QVERIFY(qpaPlatformIsWayland(QStringLiteral("xcb;wayland")));
        QVERIFY(qpaPlatformIsWayland(QStringLiteral("wayland-egl")));
        QVERIFY(!qpaPlatformIsWayland(QString()));
        QVERIFY(!qpaPlatformIsWayland(QStringLiteral("offscreen")));
        QVERIFY(!qpaPlatformIsWayland(QStringLiteral("xcb")));
        QVERIFY(!qpaPlatformIsWayland(QStringLiteral("waylandish")));
    }
};

QTEST_GUILESS_MAIN(TestQpaPlatform)
#include "tst_qpaplatform.moc"
