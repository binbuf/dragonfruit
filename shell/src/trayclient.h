// SPDX-License-Identifier: GPL-3.0-only
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
//
// The service answers item and menu reads from its normalized snapshot cache,
// so these calls are cheap when warm; the shell still runs every read on a
// worker thread (see ShellController's inventory/interactive pools) so a cold
// or slow exporter can never block the scene/input thread.
#pragma once

#include <QList>
#include <QString>
#include <QVariantList>

struct TrayItem {
    // The stable registered name the watcher reports (`bus.name`, or
    // `sender:/path` for a path registration).
    QString name;
    // The exporter's bus destination when the snapshot was taken. An owner
    // change invalidates the item's cached menu and any action bound to it.
    QString destination;
    // The unique bus name that registered the item. This is the exporter
    // identity an open menu and its actions are bound to.
    QString owner;
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

// One item's menu reply. `owner` and `generation` bind the rows to the
// exporter snapshot they were projected from; a late reply for an older
// owner/generation is discarded by the shell.
struct TrayMenuSnapshot {
    bool ok = false;
    QString name;
    QString owner;
    quint64 generation = 0;
    QVariantList rows;
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

    // One item's menu plus the exporter owner and menu generation it belongs
    // to. Prefer this over `menu()`; it lets a caller discard a late reply
    // from a replaced exporter.
    TrayMenuSnapshot menuSnapshot(const QString &name) const;

    // Activate one menu row by its DBusMenu id.
    bool menuEvent(const QString &name, int id) const;

    // Activate one menu row only when `owner` still owns the item.
    bool menuEventFrom(const QString &name, const QString &owner, int id) const;

    // A direct item activation: `activate`, `secondary`, or `context`.
    bool activate(const QString &name, const QString &kind) const;

    // A direct activation only when `owner` still owns the item.
    bool activateFrom(const QString &name, const QString &owner, const QString &kind) const;

    // Pure decoders, exposed for headless tests.
    static QList<TrayItem> parseItems(const QString &json);
    static QVariantList parseMenu(const QString &json);
    static TrayMenuSnapshot parseSnapshot(const QString &json);

private:
    QString call(const QString &method, const QList<QVariant> &arguments) const;
    bool callBool(const QString &method, const QList<QVariant> &arguments) const;
};