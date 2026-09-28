// SPDX-License-Identifier: MIT
#include "PrivacyClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

#include <algorithm>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Privacy");

// The portal permission tables the adapter projects, in the pane's order. The
// bridge host already injects every one; the mock mirrors the vocabulary so a
// fixture view has the same shape.
struct KnownTable {
    const char *table;
    const char *label;
};

const KnownTable kKnownTables[] = {
    { "devices", "Camera" },
    { "location", "Location Services" },
    { "notifications", "Notifications" },
    { "screencast", "Screen Recording" },
    { "remote-desktop", "Remote Desktop" },
    { "screenshot", "Screenshot" },
    { "background", "Background" },
    { "usb", "USB Devices" },
    { "input-capture", "Input Capture" },
    { "gamemode", "Game Mode" },
    { "inhibit", "Inhibit" },
    { "realtime", "Realtime" },
    { "wallpaper", "Wallpaper" },
    { "desktop-used-apps", "Default Apps" },
};

QStringList permissionsForState(const QString &permission)
{
    if (permission == QStringLiteral("allowed"))
        return { QStringLiteral("yes") };
    if (permission == QStringLiteral("denied"))
        return { QStringLiteral("no") };
    if (permission == QStringLiteral("ask"))
        return { QStringLiteral("ask") };
    return {};
}

QString stateForPermissions(const QStringList &permissions)
{
    if (permissions.size() == 1) {
        if (permissions[0] == QStringLiteral("yes"))
            return QStringLiteral("allowed");
        if (permissions[0] == QStringLiteral("no"))
            return QStringLiteral("denied");
        if (permissions[0] == QStringLiteral("ask"))
            return QStringLiteral("ask");
    }
    return QStringLiteral("unset");
}

QString stateLabel(const QString &state)
{
    if (state == QStringLiteral("allowed"))
        return QStringLiteral("Allowed");
    if (state == QStringLiteral("denied"))
        return QStringLiteral("Denied");
    if (state == QStringLiteral("ask"))
        return QStringLiteral("Ask");
    return QStringLiteral("Not Set");
}

} // namespace

// --- DbusPrivacyClient -----------------------------------------------------

DbusPrivacyClient::DbusPrivacyClient(QObject *parent)
    : PrivacyClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_available = available();
    m_watcher = new QDBusServiceWatcher(
        m_service, bus, QDBusServiceWatcher::WatchForRegistration
                          | QDBusServiceWatcher::WatchForUnregistration,
        this);
    connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
            [this](const QString &) {
                m_available = true;
                emit availableChanged(true);
                refresh();
            });
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                m_available = false;
                emit availableChanged(false);
                if (!m_view.isEmpty()) {
                    m_view.clear();
                    emit changed(m_view);
                }
            });
    refresh();
}

DbusPrivacyClient::~DbusPrivacyClient() = default;

bool DbusPrivacyClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusPrivacyClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusPrivacyClient::setPermission(const QString &table, const QString &resourceId,
                                      const QString &app, const QString &permission)
{
    call(QStringLiteral("SetPermission"), {table, resourceId, app, permission});
}

void DbusPrivacyClient::deletePermission(const QString &table, const QString &resourceId,
                                         const QString &app)
{
    call(QStringLiteral("DeletePermission"), {table, resourceId, app});
}

void DbusPrivacyClient::call(const QString &method, const QVariantList &arguments)
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        return;
    }
    const bool isState = method == QStringLiteral("State");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isState)
                    applyReply(reply.value().toUtf8());
                else
                    // The write reply is the outcome report; the new state is
                    // the host's re-read, never invented here.
                    refresh();
            });
}

void DbusPrivacyClient::applyReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_view)
        return;
    m_view = view;
    emit changed(m_view);
}

// --- MockPrivacyClient -----------------------------------------------------

MockPrivacyClient::MockPrivacyClient(QObject *parent)
    : PrivacyClient(parent)
{
    resetForTest();
}

void MockPrivacyClient::resetForTest()
{
    m_tables = {
        { QStringLiteral("devices"),
          { { QStringLiteral("camera"),
              { { QStringLiteral("org.example.Snapshot"), { QStringLiteral("no") } },
                { QStringLiteral("org.mozilla.firefox"), { QStringLiteral("yes") } } } } } },
        { QStringLiteral("location"),
          { { QStringLiteral("location"),
              { { QStringLiteral("org.example.Maps"),
                  { QStringLiteral("exact"), QStringLiteral("2026-09-01T00:00:00Z") } } } } } },
        { QStringLiteral("notifications"),
          { { QStringLiteral("notification"),
              { { QStringLiteral("org.example.Calendar"), { QStringLiteral("ask") } } } } } },
        { QStringLiteral("screencast"),
          { { QStringLiteral("screencast"),
              { { QStringLiteral("com.obsproject.Studio"), { QStringLiteral("yes") } } } } } },
    };
    rebuild();
}

void MockPrivacyClient::refresh()
{
    rebuild();
}

void MockPrivacyClient::setPermission(const QString &table, const QString &resourceId,
                                      const QString &app, const QString &permission)
{
    const QStringList permissions = permissionsForState(permission);
    Table *entry = nullptr;
    for (Table &candidate : m_tables) {
        if (candidate.table == table) {
            entry = &candidate;
            break;
        }
    }
    if (!entry) {
        m_tables.append(Table { table, {} });
        entry = &m_tables.last();
    }
    Resource *resource = nullptr;
    for (Resource &candidate : entry->resources) {
        if (candidate.id == resourceId) {
            resource = &candidate;
            break;
        }
    }
    if (!resource) {
        entry->resources.append(Resource { resourceId, {} });
        resource = &entry->resources.last();
    }
    for (AppPermission &candidate : resource->apps) {
        if (candidate.app == app) {
            candidate.permissions = permissions;
            rebuild();
            return;
        }
    }
    resource->apps.append(AppPermission { app, permissions });
    rebuild();
}

void MockPrivacyClient::deletePermission(const QString &table, const QString &resourceId,
                                         const QString &app)
{
    for (Table &entry : m_tables) {
        if (entry.table != table)
            continue;
        for (Resource &resource : entry.resources) {
            if (resource.id != resourceId)
                continue;
            resource.apps.removeIf([&app](const AppPermission &candidate) {
                return candidate.app == app;
            });
        }
    }
    rebuild();
}

void MockPrivacyClient::rebuild()
{
    // Every known table is a category, in the curated order; a table the store
    // has an entry for overrides its (empty) resources; an unknown table is
    // kept after the known ones.
    QList<Table> ordered;
    for (const KnownTable &known : kKnownTables) {
        Table category;
        category.table = QString::fromLatin1(known.table);
        for (const Table &entry : m_tables) {
            if (entry.table == category.table) {
                category.resources = entry.resources;
                break;
            }
        }
        ordered.append(category);
    }
    for (const Table &entry : m_tables) {
        bool known = false;
        for (const KnownTable &knownTable : kKnownTables) {
            if (entry.table == QString::fromLatin1(knownTable.table)) {
                known = true;
                break;
            }
        }
        if (!known)
            ordered.append(entry);
    }

    auto labelFor = [](const QString &table) {
        for (const KnownTable &known : kKnownTables) {
            if (table == QString::fromLatin1(known.table))
                return QString::fromLatin1(known.label);
        }
        return table;
    };

    int appCount = 0;
    int grantedCount = 0;
    int deniedCount = 0;
    QJsonArray categories;
    for (Table &category : ordered) {
        // Sort resources by id and applications by app id (mirrors the model).
        std::stable_sort(category.resources.begin(), category.resources.end(),
                         [](const Resource &a, const Resource &b) { return a.id < b.id; });
        int categoryApps = 0;
        int categoryGranted = 0;
        QJsonArray resources;
        for (Resource &resource : category.resources) {
            std::stable_sort(resource.apps.begin(), resource.apps.end(),
                             [](const AppPermission &a, const AppPermission &b) {
                                 return a.app < b.app;
                             });
            QJsonArray apps;
            for (const AppPermission &permission : resource.apps) {
                const QString state = stateForPermissions(permission.permissions);
                ++categoryApps;
                if (state == QStringLiteral("allowed"))
                    ++categoryGranted;
                QJsonObject appEntry;
                appEntry.insert(QStringLiteral("app"), permission.app);
                appEntry.insert(QStringLiteral("state"), state);
                appEntry.insert(QStringLiteral("stateLabel"), stateLabel(state));
                appEntry.insert(QStringLiteral("permissions"),
                                QJsonArray::fromStringList(permission.permissions));
                apps.append(appEntry);
            }
            QJsonObject resourceEntry;
            resourceEntry.insert(QStringLiteral("id"), resource.id);
            resourceEntry.insert(QStringLiteral("appCount"), apps.size());
            resourceEntry.insert(QStringLiteral("apps"), apps);
            resources.append(resourceEntry);
        }
        appCount += categoryApps;
        grantedCount += categoryGranted;
        for (const Resource &resource : category.resources)
            for (const AppPermission &permission : resource.apps)
                if (stateForPermissions(permission.permissions) == QStringLiteral("denied"))
                    ++deniedCount;

        QJsonObject categoryEntry;
        categoryEntry.insert(QStringLiteral("id"), category.table);
        categoryEntry.insert(QStringLiteral("label"), labelFor(category.table));
        categoryEntry.insert(QStringLiteral("summary"), categoryApps == 0
                                 ? QStringLiteral("None")
                                 : categoryApps == 1
                                     ? QStringLiteral("1 app")
                                     : QStringLiteral("%1 apps").arg(categoryApps));
        categoryEntry.insert(QStringLiteral("appCount"), categoryApps);
        categoryEntry.insert(QStringLiteral("grantedCount"), categoryGranted);
        categoryEntry.insert(QStringLiteral("resources"), resources);
        categories.append(categoryEntry);
    }

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("privacy"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("privacy"));
    view.insert(QStringLiteral("label"),
                appCount == 0 ? QStringLiteral("No App Permissions")
                             : appCount == 1 ? QStringLiteral("1 App")
                                             : QStringLiteral("%1 Apps").arg(appCount));
    view.insert(QStringLiteral("present"), appCount > 0);
    view.insert(QStringLiteral("appCount"), appCount);
    view.insert(QStringLiteral("grantedCount"), grantedCount);
    view.insert(QStringLiteral("deniedCount"), deniedCount);
    view.insert(QStringLiteral("categoryCount"), categories.size());
    view.insert(QStringLiteral("categories"), categories.toVariantList());
    m_view = view;
    emit changed(m_view);
}