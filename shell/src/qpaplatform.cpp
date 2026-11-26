// SPDX-License-Identifier: GPL-3.0-only
#include "qpaplatform.h"

#include <QDir>

QStringList waylandQpaPluginNames()
{
    return {
        QStringLiteral("libqwayland.so"),
        QStringLiteral("libqwayland-generic.so"),
        QStringLiteral("libqwayland-egl.so"),
    };
}

QStringList qpaPluginDirectories(const QString &qtPluginsPath,
                                 const QProcessEnvironment &environment)
{
    QStringList directories;
    const auto addDirectory = [&directories](const QString &path) {
        if (path.isEmpty())
            return;
        const QString clean = QDir::cleanPath(path);
        if (!directories.contains(clean))
            directories.append(clean);
    };
    const auto splitPathList = [](const QString &value) {
        return value.split(QDir::listSeparator(), Qt::SkipEmptyParts);
    };

    for (const QString &directory :
         splitPathList(environment.value(
             QStringLiteral("QT_QPA_PLATFORM_PLUGIN_PATH")))) {
        addDirectory(directory);
    }
    if (!qtPluginsPath.isEmpty())
        addDirectory(QDir(qtPluginsPath).filePath(QStringLiteral("platforms")));
    for (const QString &root :
         splitPathList(environment.value(QStringLiteral("QT_PLUGIN_PATH")))) {
        addDirectory(QDir(root).filePath(QStringLiteral("platforms")));
    }
    return directories;
}

bool waylandQpaPluginPresent(const QStringList &directories)
{
    const QStringList names = waylandQpaPluginNames();
    for (const QString &directory : directories) {
        const QDir dir(directory);
        for (const QString &name : names) {
            if (dir.exists(name))
                return true;
        }
    }
    return false;
}

bool qpaPlatformIsWayland(const QString &platform)
{
    const QStringList keys =
        platform.split(QLatin1Char(';'), Qt::SkipEmptyParts);
    for (const QString &key : keys) {
        const QString normalized = key.trimmed().toLower();
        if (normalized == QLatin1String("wayland")
            || normalized.startsWith(QLatin1String("wayland-"))) {
            return true;
        }
    }
    return false;
}
