// SPDX-License-Identifier: GPL-3.0-only
// Client for `org.dragonfruit.AppIndex1` (T-14.1a).
//
// The shell queries app-index for application identity and themed icons; it
// never scans `.desktop` directories itself. The catalog loads and decodes
// installed records on a worker. Window-menu resolution runs on the caller's
// menu worker.
#pragma once

#include "desktopentry.h"

#include <QList>
#include <QObject>
#include <QThreadPool>
#include <QString>
#include <QVariantList>

class QDBusPendingCall;

// One window registered with `com.canonical.AppMenu.Registrar`, as app-index's
// `AppMenuWindows` view carries it (T-14.4/T230).
struct AppMenuWindow {
    quint32 windowId = 0;
    QString appId;
    QString owner;
    QString menuPath;
};

// Decode the `AppMenuWindows` JSON array. Malformed rows are skipped so one
// bad registration can never hide the rest.
QList<AppMenuWindow> parseAppMenuWindows(const QString &json);

// The registration for a focused app. Matches the raw compositor id, the
// canonical `.desktop` id, and the two with the suffix normalized, in that
// order. Returns nullptr when no registration names the app.
const AppMenuWindow *findAppMenuWindow(const QList<AppMenuWindow> &windows,
                                       const QString &rawAppId,
                                       const QString &desktopId);

class AppIndexClient
{
public:
    AppIndexClient() = default;

    // True when `org.dragonfruit.AppIndex1` is on the session bus.
    bool available() const;

    // Resolve a Wayland `app_id` (or desktop id) to an installed record.
    // Returns an invalid entry on a miss.
    DesktopEntry resolve(const QString &identity) const;

    // Resolve a window's identifiers: the Wayland `appId` (may be empty) and
    // the X11 `WM_CLASS` instance/class (may be empty). Returns an invalid
    // entry on a miss.
    DesktopEntry resolveWindow(const QString &appId, const QString &instance,
                               const QString &wmClass) const;

    // Resolve an icon name to a themed file path for `size` (default: the
    // service's record size, 256). Empty when no theme provides it.
    QString iconPath(const QString &name, int size = 256) const;

    // T230: bind a focused window's DBusMenu registration to the compositor's
    // app identity. app-index re-keys and re-publishes the bridge model so the
    // menu-broker resolves it for the focused app. A miss is false.
    bool focusWindow(const QString &windowId, const QString &appId) const;

    // T230: the AppMenu.Registrar windows, as `{windowId, appId, owner,
    // menuPath}` rows. Empty when the service is absent.
    QList<AppMenuWindow> appMenuWindows() const;

    // T230: the registered window's projected DBusMenu as the menu-broker's
    // `[{title, items}]` list. Empty when the window has no registration.
    QVariantList windowMenu(const QString &windowId) const;

    // T230: forward a projected `dbusmenu:<id>` click to the owning app as
    // `Event(id, "clicked")`. Returns whether the owner accepted it.
    QDBusPendingCall windowMenuEvent(quint32 windowId, int id) const;

private:
    QString call(const QString &method, const QList<QVariant> &arguments) const;
    bool callBool(const QString &method, const QList<QVariant> &arguments) const;
    static DesktopEntry parseRecord(const QString &json);
    static QVariantList parseMenus(const QString &json);
};

// The last complete identity snapshot shared by the Dock, picker and drawer.
// One worker reads the current service owner. Changes arriving during a read
// request at most one follow-up, and a restarted owner gets a fresh subscription.
class AppIndexCatalog : public QObject
{
    Q_OBJECT
public:
    explicit AppIndexCatalog(QObject *parent = nullptr);
    ~AppIndexCatalog() override;

    void start();
    void refresh();
    void refreshIfNeeded();
    bool available() const { return !m_owner.isEmpty(); }
    const DesktopEntryIndex &index() const { return m_index; }

signals:
    void ownerChanged();
    void catalogChanged();

private slots:
    void onChanged(const QString &interests);

private:
    void setOwner(const QString &owner);

    DesktopEntryIndex m_index;
    QThreadPool m_worker;
    QString m_owner;
    quint64 m_ownerRevision = 0;
    bool m_subscribed = false;
    bool m_hasCurrentSnapshot = false;
    bool m_refreshInFlight = false;
    bool m_refreshPending = false;
};
