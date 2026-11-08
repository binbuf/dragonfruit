// SPDX-License-Identifier: MIT
#include "VpnClient.h"

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

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Vpn");

} // namespace

// --- DbusVpnClient ---------------------------------------------------------

DbusVpnClient::DbusVpnClient(QObject *parent)
    : VpnClient(parent)
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

DbusVpnClient::~DbusVpnClient() = default;

bool DbusVpnClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusVpnClient::refresh()
{
    // `Refresh` re-reads NetworkManager before returning the view; `State`
    // would only replay the host's startup snapshot.
    call(QStringLiteral("Refresh"), {});
}

void DbusVpnClient::connectVpn(const QString &uuid)
{
    call(QStringLiteral("Connect"), {uuid});
}

void DbusVpnClient::disconnectVpn(const QString &uuid)
{
    call(QStringLiteral("Deactivate"), {uuid});
}

void DbusVpnClient::addVpn(const QString &plugin, const QString &name,
                           const QString &gateway)
{
    call(QStringLiteral("Add"), {plugin, name, gateway});
}

void DbusVpnClient::setAutoconnect(const QString &uuid, bool enabled)
{
    call(QStringLiteral("SetAutoconnect"), {uuid, enabled});
}

void DbusVpnClient::call(const QString &method, const QVariantList &arguments)
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
    const bool isRead = method == QStringLiteral("State")
        || method == QStringLiteral("Refresh");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isRead, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isRead)
                    applyReply(reply.value().toUtf8());
                else
                    // The write reply is the outcome report; the new state is
                    // the host's re-read, never invented here.
                    refresh();
            });
}

void DbusVpnClient::applyReply(const QByteArray &json)
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

// --- MockVpnClient ---------------------------------------------------------

MockVpnClient::MockVpnClient(QObject *parent)
    : VpnClient(parent)
{
    resetForTest();
}

void MockVpnClient::setFixture(const QString &mode)
{
    resetForTest();
    if (mode == QLatin1String("no-plugin")) {
        m_plugins.clear();
        rebuild();
    }
}

void MockVpnClient::resetForTest()
{
    m_connections = {
        { QStringLiteral("Work VPN"), QStringLiteral("11111111-1111-1111-1111-111111111111"),
          QStringLiteral("openvpn"), QStringLiteral("OpenVPN"), true, true },
        { QStringLiteral("Home"), QStringLiteral("22222222-2222-2222-2222-222222222222"),
          QStringLiteral("wireguard"), QStringLiteral("WireGuard"), false, false },
    };
    // `DF_VPN_FIXTURE=no-plugin` models a host with no VPN plugin installed:
    // the saved connections still read, but the Add sheet has nothing to
    // offer. The default fixture has OpenVPN and WireGuard.
    if (qEnvironmentVariable("DF_VPN_FIXTURE") == QLatin1String("no-plugin"))
        m_plugins.clear();
    else
        m_plugins = {
            { QStringLiteral("org.freedesktop.NetworkManager.openvpn"),
              QStringLiteral("OpenVPN"), false },
            { QStringLiteral("org.freedesktop.NetworkManager.wireguard"),
              QStringLiteral("WireGuard"), true },
        };
    rebuild();
}

void MockVpnClient::addVpn(const QString &plugin, const QString &name,
                           const QString &)
{
    if (plugin.isEmpty() || name.isEmpty())
        return;
    const QString kind = plugin.endsWith(QLatin1String("wireguard"))
        ? QStringLiteral("wireguard") : QStringLiteral("vpn");
    const QString kindLabel = plugin.endsWith(QLatin1String("openvpn"))
        ? QStringLiteral("OpenVPN")
        : (plugin.endsWith(QLatin1String("wireguard"))
               ? QStringLiteral("WireGuard") : QStringLiteral("VPN"));
    m_connections.append({
        name,
        QStringLiteral("mock-vpn-%1").arg(m_connections.size() + 1),
        kind,
        kindLabel,
        false,
        true,
    });
    rebuild();
}

void MockVpnClient::setAutoconnect(const QString &uuid, bool enabled)
{
    for (Connection &connection : m_connections) {
        if (connection.uuid == uuid) {
            connection.autoconnect = enabled;
            break;
        }
    }
    rebuild();
}

void MockVpnClient::refresh()
{
    rebuild();
}

void MockVpnClient::connectVpn(const QString &uuid)
{
    for (Connection &connection : m_connections) {
        if (connection.uuid != uuid)
            continue;
        // Only one tunnel is up in the fixture: connecting one disconnects the
        // other, the way a single active VPN reads on a workstation.
        for (Connection &other : m_connections)
            other.connected = (&other == &connection);
        rebuild();
        return;
    }
}

void MockVpnClient::disconnectVpn(const QString &uuid)
{
    for (Connection &connection : m_connections) {
        if (connection.uuid == uuid) {
            connection.connected = false;
            rebuild();
            return;
        }
    }
}

void MockVpnClient::rebuild()
{
    int connectedCount = 0;
    QString activeUuid;
    QString activeName;
    QJsonArray connections;
    for (const Connection &connection : m_connections) {
        if (connection.connected) {
            ++connectedCount;
            if (activeUuid.isEmpty()) {
                activeUuid = connection.uuid;
                activeName = connection.id;
            }
        }
        const QString state = connection.connected ? QStringLiteral("connected")
                                                   : QStringLiteral("disconnected");
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), connection.id);
        entry.insert(QStringLiteral("uuid"), connection.uuid);
        entry.insert(QStringLiteral("kind"), connection.kind);
        entry.insert(QStringLiteral("kindLabel"), connection.kindLabel);
        entry.insert(QStringLiteral("state"), state);
        entry.insert(QStringLiteral("stateLabel"), connection.connected
                       ? QStringLiteral("Connected") : QStringLiteral("Disconnected"));
        entry.insert(QStringLiteral("connected"), connection.connected);
        entry.insert(QStringLiteral("autoconnect"), connection.autoconnect);
        entry.insert(QStringLiteral("label"),
                     connection.kindLabel + QStringLiteral(" \u00b7 ")
                         + (connection.connected ? QStringLiteral("Connected")
                                                 : QStringLiteral("Disconnected")));
        connections.append(entry);
    }

    const bool present = !m_connections.isEmpty();
    QString label = QStringLiteral("No VPN");
    if (present) {
        if (connectedCount == 0)
            label = QStringLiteral("Not Connected");
        else if (connectedCount == 1)
            label = activeName;
        else
            label = QStringLiteral("%1 Connected").arg(connectedCount);
    }

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("vpn"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), connectedCount > 0
                    ? QStringLiteral("vpn") : QStringLiteral("vpn-off"));
    view.insert(QStringLiteral("label"), label);
    view.insert(QStringLiteral("present"), present);
    view.insert(QStringLiteral("connectionCount"), m_connections.size());
    view.insert(QStringLiteral("connectedCount"), connectedCount);
    view.insert(QStringLiteral("activeUuid"), activeUuid);
    view.insert(QStringLiteral("activeName"), activeName);
    view.insert(QStringLiteral("readOnly"), false);
    view.insert(QStringLiteral("note"), QString());
    view.insert(QStringLiteral("connections"), connections.toVariantList());

    QVariantList plugins;
    for (const Plugin &plugin : m_plugins) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), plugin.serviceType);
        entry.insert(QStringLiteral("serviceType"), plugin.serviceType);
        entry.insert(QStringLiteral("label"), plugin.label);
        entry.insert(QStringLiteral("builtin"), plugin.builtin);
        plugins.append(entry);
    }
    view.insert(QStringLiteral("plugins"), plugins);
    view.insert(QStringLiteral("pluginCount"), plugins.size());
    view.insert(QStringLiteral("pluginsPresent"), !plugins.isEmpty());

    m_view = view;
    emit changed(m_view);
}
