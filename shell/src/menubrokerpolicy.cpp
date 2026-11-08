// SPDX-License-Identifier: MIT
#include "menubrokerpolicy.h"

#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QList>
#include <QVariantMap>

namespace {

QVariantMap entryWithReason(const QString &label, const QString &shortcut,
                            const QString &action, bool enabled,
                            const QString &disabledReason)
{
    QVariantMap map;
    map.insert(QStringLiteral("label"), label);
    if (!shortcut.isEmpty())
        map.insert(QStringLiteral("shortcut"), shortcut);
    map.insert(QStringLiteral("action"), action);
    map.insert(QStringLiteral("enabled"), enabled);
    if (!enabled && !disabledReason.isEmpty())
        map.insert(QStringLiteral("disabledReason"), disabledReason);
    return map;
}

QVariantMap entry(const QString &label, const QString &shortcut,
                  const QString &action, bool enabled)
{
    return entryWithReason(label, shortcut, action, enabled, QString());
}

QVariantMap submenuEntry(const QString &label, const QVariantList &submenu)
{
    QVariantMap map;
    map.insert(QStringLiteral("label"), label);
    map.insert(QStringLiteral("type"), QStringLiteral("submenu"));
    map.insert(QStringLiteral("submenu"), submenu);
    return map;
}

QVariantMap separator()
{
    QVariantMap map;
    map.insert(QStringLiteral("type"), QStringLiteral("separator"));
    return map;
}

QVariantMap servicesMenu()
{
    return submenuEntry(
        QCoreApplication::translate("ShellController", "Services"),
        {entryWithReason(
            QCoreApplication::translate("ShellController", "No Services Available"),
            QString(), QStringLiteral("services.none"), false,
            QCoreApplication::translate("ShellController",
                                        "No application has registered a service."))});
}

// The live flag a row's action maps to, or -1 when the row is not a hide verb.
// The action is authoritative; the label is a tolerant fallback.
int hideFlagIndex(const QVariantMap &row)
{
    const QString action = row.value(QStringLiteral("action")).toString();
    if (action == QLatin1String("hide"))
        return 0;
    if (action == QLatin1String("hide-others"))
        return 1;
    if (action == QLatin1String("show-all"))
        return 2;
    const QString label = row.value(QStringLiteral("label")).toString();
    if (label == QLatin1String("Hide Others"))
        return 1;
    if (label == QLatin1String("Show All"))
        return 2;
    if (label.startsWith(QLatin1String("Hide ")))
        return 0;
    return -1;
}

} // namespace

AppMenuLiveState appMenuLiveState(const QString &focusedAppId, const QVariantList &entries)
{
    AppMenuLiveState state;
    const bool hasFocus = !focusedAppId.isEmpty();
    for (const QVariant &value : entries) {
        const QVariantMap row = value.toMap();
        // Only the app-level running entries describe an app's visibility;
        // the per-window minimized entries would double-count.
        if (row.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        const QString appId = row.value(QStringLiteral("appId")).toString();
        const bool hidden = row.value(QStringLiteral("minimized")).toBool();
        if (!hidden) {
            if (hasFocus && appId == focusedAppId)
                state.hide = true;
            if (hasFocus && appId != focusedAppId)
                state.hideOthers = true;
        } else {
            state.showAll = true;
        }
    }
    return state;
}

QString windowStatesJson(const QVariantList &entries)
{
    QJsonArray apps;
    for (const QVariant &value : entries) {
        const QVariantMap row = value.toMap();
        // Only app-level running entries describe an app's window state; the
        // per-window minimized entries would be mistaken for apps.
        if (row.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        const QString appId = row.value(QStringLiteral("appId")).toString();
        if (appId.isEmpty())
            continue;
        QJsonObject app;
        app.insert(QStringLiteral("appId"), appId);
        app.insert(QStringLiteral("windows"),
                   row.value(QStringLiteral("windows")).toInt());
        app.insert(QStringLiteral("minimized"),
                   row.value(QStringLiteral("minimized")).toBool());
        QJsonArray windowList;
        for (const QVariant &windowValue : row.value(QStringLiteral("windowList")).toList()) {
            const QVariantMap window = windowValue.toMap();
            const QString windowId = window.value(QStringLiteral("windowId")).toString();
            if (windowId.isEmpty())
                continue;
            QJsonObject entry;
            entry.insert(QStringLiteral("windowId"), windowId);
            entry.insert(QStringLiteral("title"),
                         window.value(QStringLiteral("title")).toString());
            entry.insert(QStringLiteral("focused"),
                         window.value(QStringLiteral("focused")).toBool());
            entry.insert(QStringLiteral("minimized"),
                         window.value(QStringLiteral("minimized")).toBool());
            windowList.append(entry);
        }
        app.insert(QStringLiteral("windowList"), windowList);
        apps.append(app);
    }
    return QString::fromUtf8(QJsonDocument(apps).toJson(QJsonDocument::Compact));
}

QStringList visibleWindowIds(const QVariantList &entries, const QString &appId,
                             bool others)
{
    QStringList ids;
    if (appId.isEmpty())
        return ids;
    for (const QVariant &value : entries) {
        const QVariantMap row = value.toMap();
        if (row.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        const QString rowApp = row.value(QStringLiteral("appId")).toString();
        if (rowApp.isEmpty())
            continue;
        if (others ? rowApp == appId : rowApp != appId)
            continue;
        for (const QVariant &windowValue : row.value(QStringLiteral("windowList")).toList()) {
            const QVariantMap window = windowValue.toMap();
            const QString windowId = window.value(QStringLiteral("windowId")).toString();
            if (!windowId.isEmpty() && !window.value(QStringLiteral("minimized")).toBool())
                ids << windowId;
        }
    }
    return ids;
}

QStringList minimizedWindowIds(const QVariantList &entries)
{
    QStringList ids;
    for (const QVariant &value : entries) {
        const QVariantMap row = value.toMap();
        if (row.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        for (const QVariant &windowValue : row.value(QStringLiteral("windowList")).toList()) {
            const QVariantMap window = windowValue.toMap();
            const QString windowId = window.value(QStringLiteral("windowId")).toString();
            if (!windowId.isEmpty() && window.value(QStringLiteral("minimized")).toBool()
                && !ids.contains(windowId))
                ids << windowId;
        }
    }
    return ids;
}

QStringList retainWindowIds(const QVariantList &entries, const QSet<QString> &windowIds)
{
    QSet<QString> live;
    for (const QVariant &value : entries) {
        const QVariantMap row = value.toMap();
        if (row.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        for (const QVariant &windowValue : row.value(QStringLiteral("windowList")).toList()) {
            const QString windowId = windowValue.toMap()
                                         .value(QStringLiteral("windowId"))
                                         .toString();
            if (!windowId.isEmpty())
                live.insert(windowId);
        }
    }
    QStringList retained;
    for (const QString &windowId : windowIds) {
        if (live.contains(windowId))
            retained << windowId;
    }
    retained.sort();
    return retained;
}

QVariantList fixedApplicationMenu(const QString &appName, const AppMenuLiveState &state)
{
    QVariantList menu;
    menu << entry(QCoreApplication::translate("ShellController", "About %1").arg(appName),
                  QString(), QStringLiteral("about"), true);
    menu << entry(QCoreApplication::translate("ShellController", "Settings\u2026"),
                  QStringLiteral("Super+,"), QStringLiteral("settings"), true);
    menu << servicesMenu();
    menu << separator();
    menu << entryWithReason(
        QCoreApplication::translate("ShellController", "Hide %1").arg(appName),
        QStringLiteral("Super+H"), QStringLiteral("hide"), state.hide,
        QCoreApplication::translate("ShellController", "The app has no visible window."));
    menu << entryWithReason(
        QCoreApplication::translate("ShellController", "Hide Others"),
        QStringLiteral("Super+Alt+H"), QStringLiteral("hide-others"), state.hideOthers,
        QCoreApplication::translate("ShellController", "No other app has a visible window."));
    menu << entryWithReason(
        QCoreApplication::translate("ShellController", "Show All"), QString(),
        QStringLiteral("show-all"), state.showAll,
        QCoreApplication::translate("ShellController", "No hidden app to show."));
    menu << separator();
    menu << entry(QCoreApplication::translate("ShellController", "Quit %1").arg(appName),
                  QStringLiteral("Super+Q"), QStringLiteral("quit"), true);
    return menu;
}

QVariantList windowMenu(const QVariantList &entries, const QString &appId,
                        const QString &appName)
{
    if (appId.isEmpty())
        return {};
    QVariantMap app;
    for (const QVariant &value : entries) {
        const QVariantMap row = value.toMap();
        if (row.value(QStringLiteral("kind")).toString() != QLatin1String("temporary"))
            continue;
        if (row.value(QStringLiteral("appId")).toString() == appId) {
            app = row;
            break;
        }
    }
    if (app.isEmpty())
        return {};

    const QVariantList windows = app.value(QStringLiteral("windowList")).toList();
    bool hasVisible = false;
    for (const QVariant &value : windows) {
        if (!value.toMap().value(QStringLiteral("minimized")).toBool()) {
            hasVisible = true;
            break;
        }
    }

    QVariantList menu;
    menu << entryWithReason(
        QCoreApplication::translate("ShellController", "Minimize"),
        QStringLiteral("Super+M"), QStringLiteral("window.minimize"), hasVisible,
        QCoreApplication::translate("ShellController",
                                    "Every window is already minimized."));
    menu << separator();
    if (windows.isEmpty()) {
        menu << entryWithReason(
            QCoreApplication::translate("ShellController", "No Windows"), QString(),
            QStringLiteral("window.none"), false,
            QCoreApplication::translate("ShellController",
                                        "This app has no open windows."));
    } else {
        for (const QVariant &value : windows) {
            const QVariantMap window = value.toMap();
            const QString windowId = window.value(QStringLiteral("windowId")).toString();
            if (windowId.isEmpty())
                continue;
            QString label = window.value(QStringLiteral("title")).toString();
            if (label.trimmed().isEmpty())
                label = appName;
            QVariantMap row = entry(label, QString(),
                                    QStringLiteral("window.activate:%1").arg(windowId), true);
            if (window.value(QStringLiteral("focused")).toBool()) {
                row.insert(QStringLiteral("checked"), true);
                row.insert(QStringLiteral("checkable"), true);
            }
            menu << row;
        }
    }
    menu << separator();
    menu << entryWithReason(
        QCoreApplication::translate("ShellController", "Bring All to Front"), QString(),
        QStringLiteral("window.bring-all-to-front"), !windows.isEmpty(),
        QCoreApplication::translate("ShellController",
                                    "This app has no open windows."));
    return menu;
}

bool hasMenuTitle(const QVariantList &menus, const QString &title)
{
    for (const QVariant &value : menus) {
        if (value.toMap().value(QStringLiteral("title")).toString().compare(
                title, Qt::CaseInsensitive) == 0)
            return true;
    }
    return false;
}

QVariantList applyAppMenuLiveState(const QVariantList &published, const AppMenuLiveState &state)
{
    const bool flags[3] = { state.hide, state.hideOthers, state.showAll };
    QVariantList menu;
    menu.reserve(published.size());
    for (const QVariant &value : published) {
        QVariantMap row = value.toMap();
        const int index = hideFlagIndex(row);
        if (index >= 0)
            row.insert(QStringLiteral("enabled"), flags[index]);
        menu << row;
    }
    return menu;
}

namespace {

void appendAccelerators(const QVariantList &rows, QList<MenuAccelerator> &out)
{
    for (const QVariant &value : rows) {
        const QVariantMap row = value.toMap();
        if (row.value(QStringLiteral("type")).toString() == QLatin1String("separator"))
            continue;
        const QString shortcut = row.value(QStringLiteral("shortcut")).toString();
        const QString action = row.value(QStringLiteral("action")).toString();
        if (!shortcut.isEmpty() && !action.isEmpty())
            out.append({action, shortcut});
        // A submenu carries accelerators too; recurse so a nested action is
        // never dropped.
        const QVariantList submenu = row.value(QStringLiteral("submenu")).toList();
        if (!submenu.isEmpty())
            appendAccelerators(submenu, out);
    }
}

} // namespace

QList<MenuAccelerator> menuAccelerators(const QVariantList &rows)
{
    QList<MenuAccelerator> out;
    appendAccelerators(rows, out);
    return out;
}

QList<MenuAccelerator> publishedAccelerators(const QVariantList &applicationMenuItems,
                                             const QVariantList &appMenuModel)
{
    QList<MenuAccelerator> out = menuAccelerators(applicationMenuItems);
    for (const QVariant &value : appMenuModel) {
        const QVariantMap menu = value.toMap();
        out += menuAccelerators(menu.value(QStringLiteral("items")).toList());
    }
    return out;
}

QString acceleratorWireTable(const QList<MenuAccelerator> &accelerators)
{
    QString table;
    for (const MenuAccelerator &accelerator : accelerators) {
        if (accelerator.action.isEmpty() || accelerator.chord.isEmpty())
            continue;
        if (!table.isEmpty())
            table += QLatin1Char('\n');
        table += accelerator.action + QLatin1Char('\t') + accelerator.chord;
    }
    return table;
}