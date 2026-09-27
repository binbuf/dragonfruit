// SPDX-License-Identifier: MIT
#include "menubrokerpolicy.h"

#include <QCoreApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QList>
#include <QVariantMap>

namespace {

QVariantMap entry(const QString &label, const QString &shortcut,
                  const QString &action, bool enabled)
{
    QVariantMap map;
    map.insert(QStringLiteral("label"), label);
    if (!shortcut.isEmpty())
        map.insert(QStringLiteral("shortcut"), shortcut);
    map.insert(QStringLiteral("action"), action);
    map.insert(QStringLiteral("enabled"), enabled);
    return map;
}

QVariantMap separator()
{
    QVariantMap map;
    map.insert(QStringLiteral("type"), QStringLiteral("separator"));
    return map;
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
        apps.append(app);
    }
    return QString::fromUtf8(QJsonDocument(apps).toJson(QJsonDocument::Compact));
}

QVariantList fixedApplicationMenu(const QString &appName, const AppMenuLiveState &state)
{
    QVariantList menu;
    menu << entry(QCoreApplication::translate("ShellController", "About %1").arg(appName),
                  QString(), QStringLiteral("about"), true);
    menu << entry(QCoreApplication::translate("ShellController", "Settings\u2026"),
                  QStringLiteral("Super+,"), QStringLiteral("settings"), true);
    menu << separator();
    menu << entry(QCoreApplication::translate("ShellController", "Hide %1").arg(appName),
                  QStringLiteral("Super+H"), QStringLiteral("hide"), state.hide);
    menu << entry(QCoreApplication::translate("ShellController", "Hide Others"),
                  QStringLiteral("Super+Alt+H"), QStringLiteral("hide-others"),
                  state.hideOthers);
    menu << entry(QCoreApplication::translate("ShellController", "Show All"),
                  QString(), QStringLiteral("show-all"), state.showAll);
    menu << separator();
    menu << entry(QCoreApplication::translate("ShellController", "Quit %1").arg(appName),
                  QStringLiteral("Super+Q"), QStringLiteral("quit"), true);
    return menu;
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