// SPDX-License-Identifier: MIT
#include "NetworkClient.h"

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
const QString kNetworkInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Network");
const QString kFirewallInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Firewall");

} // namespace

// --- DbusNetworkClient -----------------------------------------------------

DbusNetworkClient::DbusNetworkClient(QObject *parent)
    : NetworkClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_networkInterface(kNetworkInterface)
    , m_firewallInterface(kFirewallInterface)
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

DbusNetworkClient::~DbusNetworkClient() = default;

bool DbusNetworkClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusNetworkClient::refresh()
{
    call(m_networkInterface, QStringLiteral("State"), {}, true);
}

void DbusNetworkClient::activateService(const QString &uuid)
{
    call(m_networkInterface, QStringLiteral("ActivateService"), {uuid}, false);
}

void DbusNetworkClient::deactivateService(const QString &uuid)
{
    call(m_networkInterface, QStringLiteral("DeactivateService"), {uuid}, false);
}

void DbusNetworkClient::moveService(const QString &uuid, int delta)
{
    call(m_networkInterface, QStringLiteral("MoveService"), {uuid, delta}, false);
}

void DbusNetworkClient::addService(const QString &name, const QString &kind,
                                   const QString &plugin, const QString &gateway)
{
    call(m_networkInterface, QStringLiteral("AddService"),
         {name, kind, plugin, gateway}, false);
}

void DbusNetworkClient::setAutoconnect(const QString &uuid, bool enabled)
{
    call(m_networkInterface, QStringLiteral("SetAutoconnect"), {uuid, enabled}, false);
}

void DbusNetworkClient::setFirewallEnabled(bool enabled)
{
    call(m_firewallInterface, QStringLiteral("SetEnabled"), {enabled}, false);
}

void DbusNetworkClient::openPort(int port, const QString &protocol)
{
    call(m_firewallInterface, QStringLiteral("OpenPort"), {port, protocol}, false);
}

void DbusNetworkClient::closePort(int port, const QString &protocol)
{
    call(m_firewallInterface, QStringLiteral("ClosePort"), {port, protocol}, false);
}

void DbusNetworkClient::openService(const QString &service)
{
    call(m_firewallInterface, QStringLiteral("OpenService"), {service}, false);
}

void DbusNetworkClient::closeService(const QString &service)
{
    call(m_firewallInterface, QStringLiteral("CloseService"), {service}, false);
}

void DbusNetworkClient::call(const QString &interface, const QString &method,
                             const QVariantList &arguments, bool isState)
{
    auto *iface = new QDBusInterface(m_service, m_path, interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        return;
    }
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

void DbusNetworkClient::applyReply(const QByteArray &json)
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

// --- MockNetworkClient -----------------------------------------------------

MockNetworkClient::MockNetworkClient(QObject *parent)
    : NetworkClient(parent)
{
    const QString fixture = qEnvironmentVariable("DF_NETWORK_FIXTURE");
    if (fixture == QLatin1String("absent")) {
        m_available = false;
        m_view.clear();
        return;
    }
    resetForTest();
    if (fixture == QLatin1String("nftables")) {
        m_firewallBackend = QStringLiteral("nftables");
        m_firewallStatusKnown = false;
        m_firewallEnabled = false;
        m_firewallNote = QStringLiteral(
            "nftables has no service status; enable/disable is managed by the host.");
        m_ports.clear();
        m_servicesList.clear();
        rebuild();
    }
}

void MockNetworkClient::resetForTest()
{
    m_available = true;
    m_firewallDetected = true;
    m_firewallStatusKnown = true;
    m_firewallEnabled = false;
    m_firewallBackend = QStringLiteral("firewalld");
    m_firewallNote.clear();
    m_ports = {
        QVariantMap{ { QStringLiteral("port"), 22 },
                     { QStringLiteral("protocol"), QStringLiteral("tcp") },
                     { QStringLiteral("id"), QStringLiteral("22/tcp") },
                     { QStringLiteral("label"), QStringLiteral("22/tcp") } },
        QVariantMap{ { QStringLiteral("port"), 443 },
                     { QStringLiteral("protocol"), QStringLiteral("tcp") },
                     { QStringLiteral("id"), QStringLiteral("443/tcp") },
                     { QStringLiteral("label"), QStringLiteral("443/tcp") } },
    };
    m_servicesList = { QStringLiteral("ssh"), QStringLiteral("https") };
    m_services = {
        { QStringLiteral("Wired"), QStringLiteral("mock-wired"),
          QStringLiteral("ethernet"), true, true, 10,
          QStringLiteral("enp1s0"), QStringLiteral("aa:bb:cc:dd:ee:02"),
          QStringLiteral("192.168.1.20") },
        { QStringLiteral("Home Wi-Fi"), QStringLiteral("mock-wifi-dragonfruit"),
          QStringLiteral("wifi"), true, true, 5,
          QStringLiteral("wlan0"), QStringLiteral("aa:bb:cc:dd:ee:01"),
          QStringLiteral("192.168.1.42") },
        { QStringLiteral("Thunderbolt Bridge"), QStringLiteral("mock-bridge"),
          QStringLiteral("bridge"), false, false, 0,
          QStringLiteral("bridge0"), QStringLiteral("aa:bb:cc:dd:ee:03"), QString() },
    };
    rebuild();
}

void MockNetworkClient::refresh()
{
    rebuild();
}

void MockNetworkClient::activateService(const QString &uuid)
{
    for (Service &service : m_services)
        service.connected = service.uuid == uuid;
    rebuild();
}

void MockNetworkClient::deactivateService(const QString &uuid)
{
    for (Service &service : m_services) {
        if (service.uuid == uuid)
            service.connected = false;
    }
    rebuild();
}

void MockNetworkClient::moveService(const QString &uuid, int delta)
{
    for (int i = 0; i < m_services.size(); ++i) {
        if (m_services[i].uuid != uuid)
            continue;
        const int target = qBound(0, i + delta, m_services.size() - 1);
        if (target == i)
            return;
        const int priority = m_services[target].priority;
        m_services[i].priority = delta < 0 ? priority + 1 : priority - 1;
        break;
    }
    rebuild();
}

void MockNetworkClient::addService(const QString &name, const QString &kind,
                                   const QString &plugin, const QString &gateway)
{
    Q_UNUSED(plugin);
    Q_UNUSED(gateway);
    Service service;
    service.id = name;
    service.uuid = QStringLiteral("mock-added-") + QString::number(m_services.size() + 1);
    service.kind = kind;
    service.autoconnect = true;
    service.priority = 0;
    m_services.append(service);
    rebuild();
}

void MockNetworkClient::setAutoconnect(const QString &uuid, bool enabled)
{
    for (Service &service : m_services) {
        if (service.uuid == uuid)
            service.autoconnect = enabled;
    }
    rebuild();
}

void MockNetworkClient::setFirewallEnabled(bool enabled)
{
    if (!m_firewallStatusKnown)
        return;
    m_firewallEnabled = enabled;
    rebuild();
}

void MockNetworkClient::openPort(int port, const QString &protocol)
{
    if (!m_firewallStatusKnown)
        return;
    const QString id = QStringLiteral("%1/%2").arg(port).arg(protocol);
    for (const QVariant &entry : m_ports) {
        if (entry.toMap().value(QStringLiteral("id")).toString() == id)
            return;
    }
    m_ports.append(QVariantMap{
        { QStringLiteral("port"), port },
        { QStringLiteral("protocol"), protocol },
        { QStringLiteral("id"), id },
        { QStringLiteral("label"), id },
    });
    rebuild();
}

void MockNetworkClient::closePort(int port, const QString &protocol)
{
    if (!m_firewallStatusKnown)
        return;
    const QString id = QStringLiteral("%1/%2").arg(port).arg(protocol);
    for (int i = 0; i < m_ports.size(); ++i) {
        if (m_ports[i].toMap().value(QStringLiteral("id")).toString() == id) {
            m_ports.removeAt(i);
            break;
        }
    }
    rebuild();
}

void MockNetworkClient::openService(const QString &service)
{
    if (!m_firewallStatusKnown)
        return;
    if (!m_servicesList.contains(service))
        m_servicesList.append(service);
    rebuild();
}

void MockNetworkClient::closeService(const QString &service)
{
    if (!m_firewallStatusKnown)
        return;
    m_servicesList.removeAll(service);
    rebuild();
}

void MockNetworkClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("network"));
    view.insert(QStringLiteral("state"),
                m_available ? QStringLiteral("available")
                            : QStringLiteral("unavailable"));
    view.insert(QStringLiteral("radioEnabled"), true);
    view.insert(QStringLiteral("readOnly"), false);
    view.insert(QStringLiteral("note"), QString());

    std::sort(m_services.begin(), m_services.end(),
              [](const Service &left, const Service &right) {
                  if (left.priority != right.priority)
                      return left.priority > right.priority;
                  return left.id < right.id;
              });
    QVariantList services;
    for (int i = 0; i < m_services.size(); ++i) {
        const Service &service = m_services[i];
        const QString kindLabel = service.kind == QLatin1String("ethernet")
            ? QStringLiteral("Ethernet")
            : (service.kind == QLatin1String("wifi") ? QStringLiteral("Wi-Fi")
                                                     : QStringLiteral("Bridge"));
        QVariantMap entry;
        entry.insert(QStringLiteral("id"), service.id);
        entry.insert(QStringLiteral("uuid"), service.uuid);
        entry.insert(QStringLiteral("kind"), service.kind);
        entry.insert(QStringLiteral("kindLabel"), kindLabel);
        entry.insert(QStringLiteral("state"),
                     service.connected ? QStringLiteral("connected")
                                       : QStringLiteral("disconnected"));
        entry.insert(QStringLiteral("stateLabel"),
                     service.connected ? QStringLiteral("Connected")
                                       : QStringLiteral("Not connected"));
        entry.insert(QStringLiteral("connected"), service.connected);
        entry.insert(QStringLiteral("autoconnect"), service.autoconnect);
        entry.insert(QStringLiteral("priority"), service.priority);
        entry.insert(QStringLiteral("order"), i);
        entry.insert(QStringLiteral("interface"), service.interface);
        entry.insert(QStringLiteral("hardware"), service.hardware);
        entry.insert(QStringLiteral("address"), service.address);
        services.append(entry);
    }
    view.insert(QStringLiteral("services"), services);
    view.insert(QStringLiteral("serviceCount"), services.size());

    QVariantMap firewall;
    firewall.insert(QStringLiteral("kind"), QStringLiteral("firewall"));
    firewall.insert(QStringLiteral("state"),
                    !m_firewallDetected
                        ? QStringLiteral("unavailable")
                        : (m_firewallStatusKnown ? QStringLiteral("available")
                                                 : QStringLiteral("unavailable")));
    firewall.insert(QStringLiteral("detected"), m_firewallDetected);
    firewall.insert(QStringLiteral("backend"), m_firewallBackend);
    firewall.insert(QStringLiteral("backendLabel"),
                    m_firewallBackend == QLatin1String("nftables")
                        ? QStringLiteral("nftables") : QStringLiteral("firewalld"));
    firewall.insert(QStringLiteral("statusKnown"), m_firewallStatusKnown);
    firewall.insert(QStringLiteral("enabled"),
                    m_firewallStatusKnown ? QVariant(m_firewallEnabled) : QVariant());
    firewall.insert(QStringLiteral("label"),
                    !m_firewallStatusKnown ? QStringLiteral("Unavailable")
                                           : (m_firewallEnabled ? QStringLiteral("On")
                                                                : QStringLiteral("Off")));
    firewall.insert(QStringLiteral("defaultZone"),
                    m_firewallStatusKnown ? QStringLiteral("public") : QString());
    firewall.insert(QStringLiteral("ports"), m_ports);
    firewall.insert(QStringLiteral("portCount"), m_ports.size());
    firewall.insert(QStringLiteral("services"), m_servicesList);
    firewall.insert(QStringLiteral("serviceCount"), m_servicesList.size());
    firewall.insert(QStringLiteral("note"), m_firewallNote);
    firewall.insert(QStringLiteral("readOnly"), false);
    firewall.insert(QStringLiteral("readOnlyNote"), QString());
    view.insert(QStringLiteral("firewall"), firewall);

    m_view = view;
    emit changed(m_view);
}
