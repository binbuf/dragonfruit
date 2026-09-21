// SPDX-License-Identifier: MIT
#include "dockprojection.h"

#include <algorithm>

#include <QHash>
#include <QVariantMap>

namespace {

QString unknownKey()
{
    return QStringLiteral("__unknown__");
}

} // namespace

QString displayNameForAppId(const QString &appId)
{
    if (appId.isEmpty())
        return QStringLiteral("Unknown");
    const qsizetype dot = appId.lastIndexOf(QLatin1Char('.'));
    return dot >= 0 ? appId.mid(dot + 1) : appId;
}

QVariantList buildDockProjection(const QList<DockWindow> &windows)
{
    struct AppGroup {
        QString name;
        QVariantList windows;
        int minimized = 0;
    };
    QHash<QString, AppGroup> groups;
    QList<DockWindow> minimizedWindows;

    const auto rowFor = [](const DockWindow &window) {
        QVariantMap row;
        // A decimal string: a 64-bit pointer does not survive a QML `number`.
        row.insert(QStringLiteral("windowId"), QString::number(window.windowId));
        row.insert(QStringLiteral("title"),
                   window.title.isEmpty() ? displayNameForAppId(window.appId) : window.title);
        row.insert(QStringLiteral("minimized"), window.minimized);
        row.insert(QStringLiteral("focused"), window.focused);
        if (window.workspaceIndex >= 0) {
            row.insert(QStringLiteral("workspaceIndex"), window.workspaceIndex);
            row.insert(QStringLiteral("workspaceName"),
                       window.workspaceName.isEmpty()
                           ? QStringLiteral("Space %1").arg(window.workspaceIndex + 1)
                           : window.workspaceName);
        }
        return row;
    };

    for (const DockWindow &window : windows) {
        const QString key = window.appId.isEmpty() ? unknownKey() : window.appId;
        AppGroup &group = groups[key];
        if (group.name.isEmpty())
            group.name = displayNameForAppId(window.appId);
        // Announcement order is oldest-first, so prepending yields a
        // most-recent-first window list.
        group.windows.prepend(rowFor(window));
        if (window.minimized) {
            group.minimized += 1;
            minimizedWindows.append(window);
        }
    }

    // The focused window leads its app's list (the chooser's checkmark).
    for (auto it = groups.begin(); it != groups.end(); ++it) {
        QVariantList &list = it.value().windows;
        for (int i = 1; i < list.size(); ++i) {
            if (list.at(i).toMap().value(QStringLiteral("focused")).toBool()) {
                list.move(i, 0);
                break;
            }
        }
    }

    QVariantList entries;
    for (auto it = groups.constBegin(); it != groups.constEnd(); ++it) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), it.key());
        entry.insert(QStringLiteral("appId"),
                     it.key() == unknownKey() ? QString() : it.key());
        entry.insert(QStringLiteral("name"), it.value().name);
        entry.insert(QStringLiteral("kind"), QStringLiteral("temporary"));
        entry.insert(QStringLiteral("running"), true);
        entry.insert(QStringLiteral("windows"), it.value().windows.size());
        entry.insert(QStringLiteral("windowList"), it.value().windows);
        entry.insert(QStringLiteral("minimized"),
                     it.value().minimized == it.value().windows.size());
        entries.append(entry);
    }
    std::sort(entries.begin(), entries.end(), [](const QVariant &a, const QVariant &b) {
        return a.toMap().value(QStringLiteral("name")).toString()
               < b.toMap().value(QStringLiteral("name")).toString();
    });

    // Per-window minimized entries carry their app's full window list so the
    // owning app's menu/chooser works from the minimized entry too (T-10
    // section 13).
    for (const DockWindow &window : minimizedWindows) {
        const QString key = window.appId.isEmpty() ? unknownKey() : window.appId;
        const AppGroup group = groups.value(key);
        QVariantMap entry;
        entry.insert(QStringLiteral("id"),
                     QStringLiteral("win:%1").arg(static_cast<qulonglong>(window.windowId)));
        entry.insert(QStringLiteral("windowId"), QString::number(window.windowId));
        entry.insert(QStringLiteral("appId"), window.appId);
        entry.insert(QStringLiteral("name"),
                     window.title.isEmpty() ? group.name : window.title);
        entry.insert(QStringLiteral("kind"), QStringLiteral("minimized"));
        entry.insert(QStringLiteral("running"), false);
        entry.insert(QStringLiteral("minimized"), true);
        entry.insert(QStringLiteral("windowList"), group.windows);
        entries.append(entry);
    }

    return entries;
}