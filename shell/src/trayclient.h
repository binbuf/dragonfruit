// SPDX-License-Identifier: MIT
// Client for the StatusNotifier/AppIndicator tray surface app-index hosts
// (T-14.3).
//
// app-index owns `org.kde.StatusNotifierWatcher` and exposes the live items'
// resolved identity, icon, and DBusMenu over `org.dragonfruit.AppIndex1`. The
// shell renders each item in the menu bar's unified status slots and opens its
// menu with the design-system `ContextMenu`.
//
// Every call is blocking and returns exactly what the service's flat JSON view
// carries. When the service is absent every call is a no-op miss, so a session
// without app-index simply shows no tray items.
#pragma once

#include <QList>
#include <QString>
#include <QVariantList>

struct TrayItem {
    // The stable registered name the watcher reports (`bus.name`, or
    // `sender:/path` for a path registration).
    QString name;
    QString id;
    QString title;
    QString iconName;
    // The themed icon app-index already resolved to a file, when any.
    QString iconPath;
    QString tooltip;
    QString menuPath;
    bool itemIsMenu = false;
    bool needsAttention = false;
    bool hasPixmap = false;
    bool valid = false;
};

class TrayClient
{
public:
    TrayClient() = default;

    // True when `org.dragonfruit.AppIndex1` is on the session bus.
    bool available() const;

    // The live tray items, or an empty list when the service is absent.
    QList<TrayItem> items() const;

    // One item's DBusMenu as a design-system row list (nested `submenu`
    // arrays included), or an empty list on a miss.
    QVariantList menu(const QString &name) const;

    // Activate one menu row by its DBusMenu id.
    bool menuEvent(const QString &name, int id) const;

    // A direct item activation: `activate`, `secondary`, or `context`.
    bool activate(const QString &name, const QString &kind) const;

    // Pure decoders, exposed for headless tests.
    static QList<TrayItem> parseItems(const QString &json);
    static QVariantList parseMenu(const QString &json);

private:
    QString call(const QString &method, const QList<QVariant> &arguments) const;
    bool callBool(const QString &method, const QList<QVariant> &arguments) const;
};