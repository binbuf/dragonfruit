// SPDX-License-Identifier: MIT
// Client for `org.dragonfruit.MenuBroker1` (T-14.7).
//
// The menu-broker owns the global-menu resolution (T-14.2a): first-party apps
// publish their design-system model, the DBusMenu bridge publishes third-party
// models (T-14.4), and the shell pushes the focus and window state and reads
// the resolved model back. This replaced the shell's interim `demoAppMenu()`
// stand-in.
//
// Every call is blocking and returns exactly what the service's flat JSON view
// carries. When the service is absent every call is a no-op miss, so a session
// without the broker degrades to the shell's own fixed application menu.
#pragma once

#include <QList>
#include <QString>
#include <QVariantList>

// One `ResolveFocused` reply. `valid` is false on a miss (the broker is absent
// or answered nothing).
struct ResolvedMenu {
    QString appId;
    QString appName;
    QString tier; // "native" | "dbusmenu" | "none"
    QVariantList applicationMenuItems; // the fixed application menu
    QVariantList menus;                // the app's exported top-level menus
    bool valid = false;
};

class MenuBrokerClient
{
public:
    MenuBrokerClient() = default;

    // True when `org.dragonfruit.MenuBroker1` is on the session bus.
    bool available() const;

    // Push the focused app id (empty for the desktop). Returns whether the
    // broker accepted the call.
    bool setFocusedApp(const QString &appId) const;

    // Push the app window-state list (`[{appId, windows, minimized}]`) as a
    // JSON string. Returns whether the broker accepted the payload.
    bool setWindowStates(const QString &json) const;

    // Resolve the focused app's menu. `valid` is false when the broker is
    // absent or the reply does not parse.
    ResolvedMenu resolveFocused() const;

    // Pure decoder, exposed for headless tests.
    static ResolvedMenu parseResolved(const QString &json);

private:
    QString call(const QString &method, const QList<QVariant> &arguments) const;
    bool callBool(const QString &method, const QList<QVariant> &arguments) const;
};