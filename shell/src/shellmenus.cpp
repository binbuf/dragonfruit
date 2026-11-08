// SPDX-License-Identifier: MIT
#include "shellmenus.h"

#include <QCoreApplication>
#include <QtGlobal>
#include <QVariantMap>

namespace {

QVariantMap entry(const QString &label, const QString &shortcut = QString(),
                  const QString &action = QString(), bool enabled = true)
{
    QVariantMap map;
    map.insert(QStringLiteral("label"), label);
    if (!shortcut.isEmpty())
        map.insert(QStringLiteral("shortcut"), shortcut);
    if (!action.isEmpty())
        map.insert(QStringLiteral("action"), action);
    if (!enabled)
        map.insert(QStringLiteral("enabled"), false);
    return map;
}

QVariantMap separator()
{
    QVariantMap map;
    map.insert(QStringLiteral("type"), QStringLiteral("separator"));
    return map;
}

} // namespace

namespace ShellMenus {

QString devReturnDesktop()
{
    return qEnvironmentVariable("DRAGONFRUIT_DEV_RETURN");
}

QString devBin()
{
    return qEnvironmentVariable("DRAGONFRUIT_DEV_BIN");
}

QVariantList recentItemsMenu(const QVariantList &recentApps)
{
    QVariantList submenu;
    for (const QVariant &value : recentApps) {
        const QVariantMap app = value.toMap();
        const QString desktopId = app.value(QStringLiteral("desktopId")).toString();
        const QString name = app.value(QStringLiteral("name")).toString();
        if (desktopId.isEmpty() || name.isEmpty())
            continue;
        submenu << entry(name, QString(),
                         QStringLiteral("recent-app:%1").arg(desktopId));
    }
    if (submenu.isEmpty()) {
        QVariantMap none;
        none.insert(QStringLiteral("label"),
                    QCoreApplication::translate("ShellMenus", "No Recent Items"));
        none.insert(QStringLiteral("action"), QStringLiteral("recent.none"));
        none.insert(QStringLiteral("enabled"), false);
        none.insert(QStringLiteral("disabledReason"),
                    QCoreApplication::translate(
                        "ShellMenus", "No applications have been used yet."));
        submenu << none;
    }
    QVariantMap row;
    row.insert(QStringLiteral("label"), QCoreApplication::translate("ShellMenus", "Recent Items"));
    row.insert(QStringLiteral("type"), QStringLiteral("submenu"));
    row.insert(QStringLiteral("submenu"), submenu);
    return {row};
}

QVariantList systemMenu(const QString &userName, const QString &devReturn,
                        const QVariantList &recentApps)
{
    QVariantList menu;
    menu << entry(QCoreApplication::translate("ShellMenus", "About This System"),
                  QString(), QStringLiteral("about-system"));
    menu << separator();
    menu << entry(QCoreApplication::translate("ShellMenus", "Settings\u2026"),
                  QString(), QStringLiteral("settings"));
    menu << entry(QCoreApplication::translate("ShellMenus", "Software Center"),
                  QString(), QStringLiteral("app-store"), false);
    const QVariantList recent = recentItemsMenu(recentApps);
    if (!recent.isEmpty())
        menu << recent.first();
    menu << separator();
    menu << entry(QCoreApplication::translate("ShellMenus", "Sleep"), QString(),
                  QStringLiteral("sleep"));
    menu << entry(QCoreApplication::translate("ShellMenus", "Restart\u2026"),
                  QString(), QStringLiteral("restart"));
    menu << entry(QCoreApplication::translate("ShellMenus", "Shut Down\u2026"),
                  QString(), QStringLiteral("shut-down"));
    menu << separator();
    // T-12.6b: the real-session return row, only when the harness armed this
    // session. It ends the session and restores the host desktop.
    if (!devReturn.isEmpty()) {
        menu << entry(QCoreApplication::translate("ShellMenus", "Quit to %1").arg(devReturn),
                      QString(), QStringLiteral("quit-to-return"));
    }
    menu << entry(QCoreApplication::translate("ShellMenus", "Lock Screen"),
                  QStringLiteral("Super+Ctrl+Q"), QStringLiteral("lock-screen"));
    menu << entry(QCoreApplication::translate("ShellMenus", "Log Out %1\u2026").arg(userName),
                  QStringLiteral("Super+Shift+Q"), QStringLiteral("log-out"));
    return menu;
}

} // namespace ShellMenus