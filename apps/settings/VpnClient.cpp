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
    call(QStringLiteral("State"), {});
}

void DbusVpnClient::connectVpn(const QString &uuid)
{
    call(QStringLiteral("Connect"), {uuid});
}

void DbusVpnClient::disconnectVpn(const QString &uuid)
{
    call(QStringLiteral("Deactivate"), {uuid});
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

void MockVpnClient::resetForTest()
{
    m_connections = {
        { QStringLiteral("Work VPN"), QStringLiteral("11111111-1111-1111-1111-111111111111"),
          QStringLiteral("openvpn"), QStringLiteral("OpenVPN"), true, true },
        { QStringLiteral("Home"), QStringLiteral("22222222-2222-2222-2222-222222222222"),
          QStringLiteral("wireguard"), QStringLiteral("WireGuard"), false, false },
    };
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
    m_view = view;
    emit changed(m_view);
}