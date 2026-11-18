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
    call(QStringLiteral("Connect"), {uuid}, QStringLiteral("connect"));
}

void DbusVpnClient::disconnectVpn(const QString &uuid)
{
    call(QStringLiteral("Deactivate"), {uuid}, QStringLiteral("disconnect"));
}

void DbusVpnClient::addVpn(const QString &plugin, const QString &name,
                           const QString &gateway)
{
    call(QStringLiteral("Add"), {plugin, name, gateway}, QStringLiteral("add"));
}

void DbusVpnClient::addWireguard(const QString &name, const QString &privateKey,
                                 const QString &address,
                                 const QString &peerPublicKey,
                                 const QString &endpoint, const QString &dns,
                                 int mtu)
{
    call(QStringLiteral("AddWireguard"),
         {name, privateKey, address, peerPublicKey, endpoint, dns, mtu},
         QStringLiteral("add-wireguard"));
}

void DbusVpnClient::editVpn(const QString &uuid)
{
    call(QStringLiteral("Edit"), {uuid}, QStringLiteral("edit"));
}

void DbusVpnClient::setAutoconnect(const QString &uuid, bool enabled)
{
    call(QStringLiteral("SetAutoconnect"), {uuid, enabled},
         QStringLiteral("autoconnect"));
}

void DbusVpnClient::call(const QString &method, const QVariantList &arguments,
                         const QString &operation)
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        if (!operation.isEmpty())
            emit writeFinished(operation, QStringLiteral("absent"), QString());
        return;
    }
    const bool isRead = method == QStringLiteral("State")
        || method == QStringLiteral("Refresh");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isRead, operation, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid()) {
                    if (!isRead && !operation.isEmpty())
                        emit writeFinished(operation, QStringLiteral("failed"),
                                           QString());
                    return;
                }
                if (isRead) {
                    applyReply(reply.value().toUtf8());
                    return;
                }
                // The write reply is the outcome report; the new state is the
                // host's re-read, never invented here.
                const QJsonDocument document =
                    QJsonDocument::fromJson(reply.value().toUtf8());
                if (document.isObject()) {
                    const QJsonObject object = document.object();
                    QString message = object.value(QStringLiteral("error")).toString();
                    if (message.isEmpty())
                        message = object.value(QStringLiteral("note")).toString();
                    const QString outcome =
                        object.value(QStringLiteral("outcome")).toString();
                    if (!operation.isEmpty())
                        emit writeFinished(operation,
                                           outcome.isEmpty()
                                               ? QStringLiteral("applied")
                                               : outcome,
                                           message);
                }
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
    const QString mode = qEnvironmentVariable("DF_VPN_FIXTURE");
    if (!mode.isEmpty() && mode != QLatin1String("1")
        && mode != QLatin1String("live"))
        setFixture(mode);
}

void MockVpnClient::setFixture(const QString &mode)
{
    resetForTest();
    // T288: the capture/absence fixture for a host with no VPN service. The
    // view carries no state, so the pane draws its structural absence.
    if (mode == QLatin1String("absent")) {
        m_absent = true;
        rebuild();
        return;
    }
    if (mode == QLatin1String("no-plugin") || mode == QLatin1String("no-add"))
        m_plugins.clear();
    if (mode == QLatin1String("no-editor") || mode == QLatin1String("no-add")
        || mode == QLatin1String("no-editor-no-wireguard")) {
        m_editorAvailable = false;
        m_editor.clear();
    }
    if (mode == QLatin1String("no-wireguard") || mode == QLatin1String("no-add")
        || mode == QLatin1String("no-editor-no-wireguard"))
        m_wireguardNative = false;
    if (mode == QLatin1String("empty"))
        m_connections.clear();
    if (mode == QLatin1String("incomplete")) {
        for (Connection &connection : m_connections) {
            connection.needsSetup = true;
            connection.connected = false;
        }
    }
    if (mode == QLatin1String("read-only")) {
        m_readOnly = true;
        m_readOnlyNote = QLatin1String("polkit denied VPN changes");
    }
    rebuild();
}

void MockVpnClient::resetForTest()
{
    m_failOperation.clear();
    m_failMessage.clear();
    m_readOnly = false;
    m_readOnlyNote.clear();
    m_wireguardNative = true;
    m_editorAvailable = true;
    m_editor = QStringLiteral("nm-connection-editor");
    m_absent = false;
    m_connections = {
        { QStringLiteral("Work VPN"), QStringLiteral("11111111-1111-1111-1111-111111111111"),
          QStringLiteral("openvpn"), QStringLiteral("OpenVPN"), true, true, false },
        { QStringLiteral("Home"), QStringLiteral("22222222-2222-2222-2222-222222222222"),
          QStringLiteral("wireguard"), QStringLiteral("WireGuard"), false, false, false },
    };
    // Two installed plugins; native WireGuard is a capability, not a plugin.
    m_plugins = {
        { QStringLiteral("org.freedesktop.NetworkManager.openvpn"),
          QStringLiteral("OpenVPN") },
        { QStringLiteral("org.freedesktop.NetworkManager.openconnect"),
          QStringLiteral("OpenConnect") },
    };
    rebuild();
}

void MockVpnClient::finishWrite(const QString &operation,
                                const QString &outcome,
                                const QString &message)
{
    emit writeFinished(operation, outcome, message);
}

bool MockVpnClient::failWrite(const QString &operation)
{
    if (m_failOperation != operation)
        return false;
    const QString message = m_failMessage;
    m_failOperation.clear();
    m_failMessage.clear();
    finishWrite(operation, QStringLiteral("failed"), message);
    return true;
}

void MockVpnClient::failNextWrite(const QString &operation,
                                  const QString &message)
{
    m_failOperation = operation;
    m_failMessage = message;
}

void MockVpnClient::addVpn(const QString &plugin, const QString &name,
                           const QString &)
{
    if (failWrite(QStringLiteral("add")))
        return;
    if (plugin.isEmpty() || name.isEmpty())
        return;
    const QString kindLabel = plugin.endsWith(QLatin1String("openvpn"))
        ? QStringLiteral("OpenVPN")
        : (plugin.endsWith(QLatin1String("openconnect"))
               ? QStringLiteral("OpenConnect") : QStringLiteral("VPN"));
    // A plugin profile is created without credentials and stays incomplete
    // until the editor finishes it. Native WireGuard never comes through here.
    m_connections.append({
        name,
        QStringLiteral("mock-vpn-%1").arg(m_connections.size() + 1),
        QStringLiteral("vpn"),
        kindLabel,
        false,
        true,
        true,
    });
    rebuild();
    finishWrite(QStringLiteral("add"), QStringLiteral("applied"));
}

void MockVpnClient::addWireguard(const QString &name, const QString &privateKey,
                                 const QString &address,
                                 const QString &peerPublicKey,
                                 const QString &endpoint, const QString &, int)
{
    if (failWrite(QStringLiteral("add-wireguard")))
        return;
    if (name.isEmpty() || privateKey.isEmpty() || address.isEmpty()
        || peerPublicKey.isEmpty() || endpoint.isEmpty())
        return;
    m_connections.append({
        name,
        QStringLiteral("mock-wireguard-%1").arg(m_connections.size() + 1),
        QStringLiteral("wireguard"),
        QStringLiteral("WireGuard"),
        false,
        true,
        false,
    });
    rebuild();
    finishWrite(QStringLiteral("add-wireguard"), QStringLiteral("applied"));
}

void MockVpnClient::editVpn(const QString &uuid)
{
    if (failWrite(QStringLiteral("edit")))
        return;
    for (Connection &connection : m_connections) {
        if (connection.uuid == uuid) {
            connection.needsSetup = false;
            rebuild();
            finishWrite(QStringLiteral("edit"), QStringLiteral("applied"));
            return;
        }
    }
}

void MockVpnClient::setAutoconnect(const QString &uuid, bool enabled)
{
    if (failWrite(QStringLiteral("autoconnect")))
        return;
    for (Connection &connection : m_connections) {
        if (connection.uuid == uuid) {
            connection.autoconnect = enabled;
            break;
        }
    }
    rebuild();
    finishWrite(QStringLiteral("autoconnect"), QStringLiteral("applied"));
}

void MockVpnClient::refresh()
{
    rebuild();
}

void MockVpnClient::connectVpn(const QString &uuid)
{
    if (failWrite(QStringLiteral("connect")))
        return;
    for (Connection &connection : m_connections) {
        if (connection.uuid != uuid)
            continue;
        // An incomplete profile cannot be presented as ready to connect.
        if (connection.needsSetup) {
            finishWrite(QStringLiteral("connect"), QStringLiteral("failed"),
                        QStringLiteral("Complete the VPN setup first."));
            return;
        }
        // Only one tunnel is up in the fixture: connecting one disconnects the
        // other, the way a single active VPN reads on a workstation.
        for (Connection &other : m_connections)
            other.connected = (&other == &connection);
        rebuild();
        finishWrite(QStringLiteral("connect"), QStringLiteral("applied"));
        return;
    }
}

void MockVpnClient::disconnectVpn(const QString &uuid)
{
    if (failWrite(QStringLiteral("disconnect")))
        return;
    for (Connection &connection : m_connections) {
        if (connection.uuid == uuid) {
            connection.connected = false;
            rebuild();
            finishWrite(QStringLiteral("disconnect"), QStringLiteral("applied"));
            return;
        }
    }
}

void MockVpnClient::rebuild()
{
    if (m_absent) {
        QVariantMap view;
        view.insert(QStringLiteral("kind"), QStringLiteral("vpn"));
        m_view = view;
        emit changed(m_view);
        return;
    }
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
        const QString stateLabel = connection.needsSetup
            ? QStringLiteral("Needs setup")
            : (connection.connected ? QStringLiteral("Connected")
                                    : QStringLiteral("Disconnected"));
        const QString label = connection.needsSetup
            ? connection.kindLabel + QStringLiteral(" \u00b7 Needs setup")
            : connection.kindLabel + QStringLiteral(" \u00b7 ")
                  + (connection.connected ? QStringLiteral("Connected")
                                          : QStringLiteral("Disconnected"));
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), connection.id);
        entry.insert(QStringLiteral("uuid"), connection.uuid);
        entry.insert(QStringLiteral("kind"), connection.kind);
        entry.insert(QStringLiteral("kindLabel"), connection.kindLabel);
        entry.insert(QStringLiteral("state"), state);
        entry.insert(QStringLiteral("stateLabel"), stateLabel);
        entry.insert(QStringLiteral("connected"), connection.connected);
        entry.insert(QStringLiteral("autoconnect"), connection.autoconnect);
        entry.insert(QStringLiteral("needsSetup"), connection.needsSetup);
        entry.insert(QStringLiteral("label"), label);
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
    view.insert(QStringLiteral("readOnly"), m_readOnly);
    view.insert(QStringLiteral("note"), m_readOnlyNote);
    view.insert(QStringLiteral("connections"), connections.toVariantList());

    QVariantList plugins;
    for (const Plugin &plugin : m_plugins) {
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), plugin.serviceType);
        entry.insert(QStringLiteral("serviceType"), plugin.serviceType);
        entry.insert(QStringLiteral("label"), plugin.label);
        entry.insert(QStringLiteral("builtin"), false);
        plugins.append(entry);
    }
    view.insert(QStringLiteral("plugins"), plugins);
    view.insert(QStringLiteral("pluginCount"), plugins.size());
    view.insert(QStringLiteral("pluginsPresent"), !plugins.isEmpty());
    view.insert(QStringLiteral("wireguardNative"), m_wireguardNative);
    view.insert(QStringLiteral("editorAvailable"), m_editorAvailable);
    view.insert(QStringLiteral("editor"), m_editor);
    const bool canAdd = !plugins.isEmpty() || m_wireguardNative;
    view.insert(QStringLiteral("canAdd"), canAdd);
    view.insert(QStringLiteral("addReason"), canAdd
                    ? QString()
                    : QStringLiteral("No NetworkManager VPN plugin is installed and "
                                     "native WireGuard is not available."));

    m_view = view;
    emit changed(m_view);
}
