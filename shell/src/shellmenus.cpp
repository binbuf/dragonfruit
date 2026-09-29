// SPDX-License-Identifier: MIT
#include "shellmenus.h"

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

QVariantList systemMenu(const QString &userName, const QString &devReturn)
{
    QVariantList menu;
    menu << entry(QStringLiteral("About This System"), QString(),
                  QStringLiteral("about-system"));
    menu << separator();
    menu << entry(QStringLiteral("System Settings\u2026"), QString(),
                  QStringLiteral("settings"));
    menu << entry(QStringLiteral("App Store"), QString(),
                  QStringLiteral("app-store"), false);
    menu << separator();
    menu << entry(QStringLiteral("Sleep"), QString(), QStringLiteral("sleep"));
    menu << entry(QStringLiteral("Restart\u2026"), QString(),
                  QStringLiteral("restart"));
    menu << entry(QStringLiteral("Shut Down\u2026"), QString(),
                  QStringLiteral("shut-down"));
    menu << separator();
    // T-12.6b: the real-session return row, only when the harness armed this
    // session. It ends the session and restores the host desktop.
    if (!devReturn.isEmpty()) {
        menu << entry(QStringLiteral("Quit to %1").arg(devReturn), QString(),
                      QStringLiteral("quit-to-return"));
    }
    menu << entry(QStringLiteral("Lock Screen"), QStringLiteral("Super+Ctrl+Q"),
                  QStringLiteral("lock-screen"));
    menu << entry(QStringLiteral("Log Out %1\u2026").arg(userName),
                  QStringLiteral("Super+Shift+Q"), QStringLiteral("log-out"));
    return menu;
}

} // namespace ShellMenus