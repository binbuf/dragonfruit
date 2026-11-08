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
#include <QSet>

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
                refreshSecurity();
            });
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            [this](const QString &) {
                m_available = false;
                emit availableChanged(false);
                if (!m_view.isEmpty()) {
                    m_view.clear();
                    emit changed(m_view);
                }
                if (!m_securityView.isEmpty()) {
                    m_securityView.clear();
                    emit securityChanged(m_securityView);
                }
            });
    refresh();
    refreshSecurity();
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

void DbusPrivacyClient::refreshSecurity()
{
    call(QStringLiteral("RefreshSecurity"), {});
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

void DbusPrivacyClient::setSecuritySourceEnabled(const QString &id, bool enabled)
{
    call(QStringLiteral("SetSecuritySourceEnabled"), {id, enabled});
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
    const bool isSecurityState = method == QStringLiteral("SecurityState")
        || method == QStringLiteral("RefreshSecurity");
    const bool isSecurityWrite = method == QStringLiteral("SetSecuritySourceEnabled");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, isSecurityState, isSecurityWrite, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isState)
                    applyReply(reply.value().toUtf8());
                else if (isSecurityState)
                    applySecurityReply(reply.value().toUtf8());
                else {
                    // The write reply is the outcome report; the new state is
                    // the host's re-read, never invented here.
                    if (isSecurityWrite)
                        refreshSecurity();
                    else
                        refresh();
                }
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

void DbusPrivacyClient::applySecurityReply(const QByteArray &json)
{
    QJsonParseError parseError{};
    const QJsonDocument document = QJsonDocument::fromJson(json, &parseError);
    QVariantMap view;
    if (parseError.error == QJsonParseError::NoError && document.isObject())
        view = document.object().toVariantMap();
    if (view == m_securityView)
        return;
    m_securityView = view;
    emit securityChanged(m_securityView);
}

// --- MockPrivacyClient -----------------------------------------------------

MockPrivacyClient::MockPrivacyClient(QObject *parent)
    : PrivacyClient(parent)
{
    resetForTest();
}

void MockPrivacyClient::resetForTest()
{
    m_privacyAvailable = true;
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
    m_sources = {
        { QStringLiteral("distro:dnf"), QStringLiteral("Fedora repositories"),
          QStringLiteral("distro"), true, false,
          QStringLiteral("Distribution repositories are managed by system policy.") },
        { QStringLiteral("flatpak:flathub:system"), QStringLiteral("Flathub (system)"),
          QStringLiteral("flatpak"), true, true,
          QStringLiteral("System Flatpak remote") },
    };
    rebuild();
    rebuildSecurity();
}

void MockPrivacyClient::refresh()
{
    rebuild();
    rebuildSecurity();
}

void MockPrivacyClient::refreshSecurity()
{
    rebuildSecurity();
}

void MockPrivacyClient::setPrivacyAvailableFixture(bool available)
{
    if (m_privacyAvailable == available)
        return;
    m_privacyAvailable = available;
    emit changed(m_privacyAvailable ? m_view : QVariantMap());
    emit availableChanged(m_privacyAvailable);
}

void MockPrivacyClient::setSecuritySourceEnabled(const QString &id, bool enabled)
{
    for (SoftwareSource &source : m_sources) {
        if (source.id != id)
            continue;
        if (!source.writable)
            return;
        source.enabled = enabled;
        break;
    }
    rebuildSecurity();
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
    QHash<QString, QJsonObject> categoryById;
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
        categoryById.insert(category.table, categoryEntry);
        categories.append(categoryEntry);
    }

    // T228: the macOS rows mapped to the closest Linux permission. The
    // mapping mirrors `MAC_PRIVACY_CATEGORIES` in the privacy adapter.
    struct MacRow {
        const char *id;
        const char *label;
        const char *owner;
        const char *difference;
        const char *table;
        const char *resource;
    };
    const MacRow macRows[] = {
        { "location", "Location", "Portal Location/Geoclue",
          "Apps ask the Location portal per request; the portal store keeps the answer.",
          "location", "" },
        { "contacts", "Contacts", "Address-book service (EDS)",
          "Contacts access is owned by the address-book service; no portal table records it per app.",
          "", "" },
        { "calendars", "Calendars", "Calendar service (EDS)",
          "Calendar access is owned by the calendar service; no portal table records it per app.",
          "", "" },
        { "reminders", "Reminders", "Task service (EDS)",
          "Reminders are a task-service capability; Linux has no TCC-style reminders permission.",
          "", "" },
        { "photos", "Photos", "Document portal",
          "The document portal records file access; there is no separate Photos permission.",
          "documents", "" },
        { "camera", "Camera", "xdg-desktop-portal Device",
          "Camera and microphone share the portal devices table; the camera resource is shown here.",
          "devices", "camera" },
        { "microphone", "Microphone", "xdg-desktop-portal Device",
          "Microphone is the portal devices table's microphone resource, not a separate TCC service.",
          "devices", "microphone" },
        { "speech", "Speech recognition", "No dedicated portal",
          "Apps use the microphone permission and their own speech service; there is no speech-recognition permission.",
          "", "" },
        { "accessibility", "Accessibility", "AT-SPI accessibility bus",
          "Accessibility is a bus-wide setting, not a per-app permission the portal store can revoke.",
          "", "" },
        { "fullDisk", "Full disk access", "polkit",
          "There is no full-disk-access switch; privileged file access is authorized by polkit per action.",
          "", "" },
        { "files", "Files and folders", "Document portal",
          "The document portal records per-app file access; it has no folder-level permission.",
          "documents", "" },
        { "screenRecording", "Screen recording", "Portal ScreenCast",
          "Screen recording is the ScreenCast portal permission; screenshots use a separate table.",
          "screencast", "" },
        { "automation", "Automation", "D-Bus policy + polkit",
          "There is no automation permission; D-Bus policy and polkit govern app-to-app calls.",
          "", "" },
        { "appManagement", "App management", "Package manager + polkit",
          "Installing or removing apps is authorized by polkit; the portal store keeps no app-management record.",
          "", "" },
        { "developerTools", "Developer tools", "Portal + polkit",
          "There is no developer-tools permission; debug interfaces are governed by the portal and polkit.",
          "", "" },
    };
    QJsonArray macCategories;
    QSet<QString> mappedTables;
    for (const MacRow &mac : macRows) {
        const QString table = QString::fromLatin1(mac.table);
        const QString resourceFilter = QString::fromLatin1(mac.resource);
        if (!table.isEmpty())
            mappedTables.insert(table);
        QJsonArray apps;
        for (const Table &category : ordered) {
            if (category.table != table)
                continue;
            for (const Resource &resource : category.resources) {
                if (!resourceFilter.isEmpty() && resource.id != resourceFilter)
                    continue;
                for (const AppPermission &permission : resource.apps) {
                    const QString state = stateForPermissions(permission.permissions);
                    QJsonObject appEntry;
                    appEntry.insert(QStringLiteral("app"), permission.app);
                    appEntry.insert(QStringLiteral("table"), category.table);
                    appEntry.insert(QStringLiteral("resourceId"), resource.id);
                    appEntry.insert(QStringLiteral("state"), state);
                    appEntry.insert(QStringLiteral("stateLabel"), stateLabel(state));
                    appEntry.insert(QStringLiteral("permissions"),
                                    QJsonArray::fromStringList(permission.permissions));
                    apps.append(appEntry);
                }
            }
        }
        const bool delegated = table.isEmpty();
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), QString::fromLatin1(mac.id));
        entry.insert(QStringLiteral("label"), QString::fromLatin1(mac.label));
        entry.insert(QStringLiteral("linuxOwner"), QString::fromLatin1(mac.owner));
        entry.insert(QStringLiteral("difference"), QString::fromLatin1(mac.difference));
        entry.insert(QStringLiteral("delegated"), delegated);
        entry.insert(QStringLiteral("appCount"), apps.size());
        entry.insert(QStringLiteral("summary"),
                     delegated ? QStringLiteral("Delegated")
                     : apps.size() == 0 ? QStringLiteral("None")
                     : apps.size() == 1 ? QStringLiteral("1 app")
                                        : QStringLiteral("%1 apps").arg(apps.size()));
        entry.insert(QStringLiteral("apps"), apps);
        macCategories.append(entry);
    }
    QJsonArray otherCategories;
    for (const Table &category : ordered) {
        if (mappedTables.contains(category.table))
            continue;
        if (categoryById.contains(category.table))
            otherCategories.append(categoryById.value(category.table));
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
    view.insert(QStringLiteral("macCategories"), macCategories.toVariantList());
    view.insert(QStringLiteral("otherCategories"), otherCategories.toVariantList());
    m_view = view;
    emit changed(m_view);
}

void MockPrivacyClient::rebuildSecurity()
{
    QJsonArray sources;
    int enabledCount = 0;
    for (const SoftwareSource &source : m_sources) {
        if (source.enabled)
            ++enabledCount;
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), source.id);
        entry.insert(QStringLiteral("name"), source.name);
        entry.insert(QStringLiteral("kind"), source.kind);
        entry.insert(QStringLiteral("enabled"), source.enabled);
        entry.insert(QStringLiteral("writable"), source.writable);
        entry.insert(QStringLiteral("note"), source.note);
        sources.append(entry);
    }
    QJsonObject encryption;
    encryption.insert(QStringLiteral("available"), true);
    encryption.insert(QStringLiteral("encrypted"), true);
    encryption.insert(QStringLiteral("label"), QStringLiteral("Encrypted (LUKS)"));
    encryption.insert(QStringLiteral("note"),
                      QStringLiteral("The running tree is on a LUKS device. "
                                     "Unlocking and recovery stay with the "
                                     "distribution's cryptsetup tools."));
    QJsonArray devices;
    QJsonObject device;
    device.insert(QStringLiteral("name"), QStringLiteral("luks-8bb932c4"));
    device.insert(QStringLiteral("kind"), QStringLiteral("crypt"));
    device.insert(QStringLiteral("fstype"), QStringLiteral("btrfs"));
    device.insert(QStringLiteral("mountpoints"),
                  QJsonArray::fromStringList({QStringLiteral("/"),
                                              QStringLiteral("/home")}));
    devices.append(device);
    encryption.insert(QStringLiteral("devices"), devices);
    QJsonObject hardening;
    hardening.insert(QStringLiteral("available"), true);
    hardening.insert(QStringLiteral("mode"), QStringLiteral("enforcing"));
    hardening.insert(QStringLiteral("label"), QStringLiteral("Enforcing"));
    hardening.insert(QStringLiteral("detail"),
                     QStringLiteral("SELinux is enforcing; Secure Boot is disabled."));
    QJsonArray extensions;
    QJsonObject extension;
    extension.insert(QStringLiteral("id"), QStringLiteral("dragonfruit"));
    extension.insert(QStringLiteral("name"), QStringLiteral("Dragonfruit"));
    extension.insert(QStringLiteral("dbusName"),
                     QStringLiteral("org.freedesktop.impl.portal.desktop.dragonfruit"));
    extension.insert(QStringLiteral("interfaces"),
                     QJsonArray::fromStringList(
                         {QStringLiteral("org.freedesktop.impl.portal.Settings"),
                          QStringLiteral("org.freedesktop.impl.portal.FileChooser")}));
    extension.insert(QStringLiteral("useIn"), QStringLiteral("dragonfruit"));
    extension.insert(QStringLiteral("path"),
                     QStringLiteral("/usr/share/xdg-desktop-portal/portals/dragonfruit.portal"));
    extensions.append(extension);
    QJsonArray advanced;
    const auto advancedRow = [&advanced](const QString &id, const QString &label,
                                         const QString &value, const QString &detail,
                                         const QString &state) {
        QJsonObject row;
        row.insert(QStringLiteral("id"), id);
        row.insert(QStringLiteral("label"), label);
        row.insert(QStringLiteral("value"), value);
        row.insert(QStringLiteral("detail"), detail);
        row.insert(QStringLiteral("state"), state);
        advanced.append(row);
    };
    advancedRow(QStringLiteral("polkit"), QStringLiteral("Authorization"),
                QStringLiteral("polkit"),
                QStringLiteral("Privileged actions use the session polkit agent."),
                QStringLiteral("ok"));
    advancedRow(QStringLiteral("firewall"), QStringLiteral("Firewall"),
                QStringLiteral("firewalld"),
                QStringLiteral("The Network pane owns the firewall controls."),
                QStringLiteral("ok"));
    advancedRow(QStringLiteral("secure-boot"), QStringLiteral("Secure Boot"),
                QStringLiteral("Disabled"),
                QStringLiteral("Reported by mokutil."),
                QStringLiteral("off"));

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("security"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("shield-check"));
    view.insert(QStringLiteral("label"), QStringLiteral("Encrypted (LUKS)"));
    view.insert(QStringLiteral("enabledSourceCount"), enabledCount);
    QVariantMap software;
    software.insert(QStringLiteral("available"), true);
    software.insert(QStringLiteral("manager"), QStringLiteral("dnf"));
    software.insert(QStringLiteral("sources"), sources.toVariantList());
    view.insert(QStringLiteral("softwareSources"), software);
    view.insert(QStringLiteral("encryption"), encryption.toVariantMap());
    view.insert(QStringLiteral("hardening"), hardening.toVariantMap());
    view.insert(QStringLiteral("extensions"), extensions.toVariantList());
    view.insert(QStringLiteral("advanced"), advanced.toVariantList());
    m_securityView = view;
    emit securityChanged(m_securityView);
}
