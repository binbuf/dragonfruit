// SPDX-License-Identifier: GPL-3.0-or-later
#include "NetworkClient.h"

#include <algorithm>

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
    // `Refresh` re-reads NetworkManager and the firewall before returning the
    // combined view. A firewall write reuses this too, because the pane's
    // `network` view carries both halves.
    call(m_networkInterface, QStringLiteral("Refresh"), {}, true);
}

void DbusNetworkClient::activateService(const QString &uuid)
{
    call(m_networkInterface, QStringLiteral("ActivateService"), {uuid}, false,
         QStringLiteral("activate"));
}

void DbusNetworkClient::deactivateService(const QString &uuid)
{
    call(m_networkInterface, QStringLiteral("DeactivateService"), {uuid}, false,
         QStringLiteral("deactivate"));
}

void DbusNetworkClient::moveService(const QString &uuid, int delta)
{
    call(m_networkInterface, QStringLiteral("MoveService"), {uuid, delta}, false,
         QStringLiteral("move"));
}

void DbusNetworkClient::addService(const QString &name, const QString &kind,
                                   const QString &plugin, const QString &gateway,
                                   const QString &interfaceName)
{
    call(m_networkInterface, QStringLiteral("AddService"),
         {name, kind, plugin, gateway, interfaceName}, false,
         QStringLiteral("add"));
}

void DbusNetworkClient::forgetService(const QString &uuid)
{
    call(m_networkInterface, QStringLiteral("Forget"), {uuid}, false,
         QStringLiteral("forget"));
}

void DbusNetworkClient::setAutoconnect(const QString &uuid, bool enabled)
{
    call(m_networkInterface, QStringLiteral("SetAutoconnect"), {uuid, enabled}, false,
         QStringLiteral("autoconnect"));
}

void DbusNetworkClient::setFirewallEnabled(bool enabled)
{
    call(m_firewallInterface, QStringLiteral("SetEnabled"), {enabled}, false,
         QStringLiteral("firewall"));
}

void DbusNetworkClient::openPort(int port, const QString &protocol)
{
    call(m_firewallInterface, QStringLiteral("OpenPort"), {port, protocol}, false,
         QStringLiteral("port"));
}

void DbusNetworkClient::closePort(int port, const QString &protocol)
{
    call(m_firewallInterface, QStringLiteral("ClosePort"), {port, protocol}, false,
         QStringLiteral("port"));
}

void DbusNetworkClient::openService(const QString &service)
{
    call(m_firewallInterface, QStringLiteral("OpenService"), {service}, false,
         QStringLiteral("service"));
}

void DbusNetworkClient::closeService(const QString &service)
{
    call(m_firewallInterface, QStringLiteral("CloseService"), {service}, false,
         QStringLiteral("service"));
}

void DbusNetworkClient::call(const QString &interface, const QString &method,
                             const QVariantList &arguments, bool isState,
                             const QString &operation)
{
    auto *iface = new QDBusInterface(m_service, m_path, interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        if (!isState && !operation.isEmpty())
            emit writeFinished(operation, QStringLiteral("absent"), QString());
        return;
    }
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, operation, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid()) {
                    if (!isState && !operation.isEmpty())
                        emit writeFinished(operation, QStringLiteral("failed"),
                                           QString());
                    return;
                }
                if (isState) {
                    applyReply(reply.value().toUtf8());
                    return;
                }
                // The write reply is the outcome report; the new state is
                // the host's re-read, never invented here.
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
    applyFixture(fixture);
    rebuild();
}

void MockNetworkClient::setFixture(const QString &mode)
{
    if (mode == QLatin1String("absent")) {
        m_available = false;
        m_view.clear();
        emit availableChanged(false);
        emit changed(m_view);
        return;
    }
    m_available = true;
    resetForTest();
    applyFixture(mode);
    rebuild();
    emit availableChanged(true);
}

// One fixture switch for the constructor and `setFixture`. The firewall modes
// model the capability surface, not a second manager: `ufw` is readable as a
// normal user but its rules are not, `nftables` has no toggle, `iptables` is
// named and read-only, and `no-firewall` is the only "no manager" state.
void MockNetworkClient::applyFixture(const QString &mode)
{
    if (mode == QLatin1String("nm-absent"))
        m_networkPresent = false;
    if (mode == QLatin1String("nftables")) {
        m_firewallBackend = QStringLiteral("nftables");
        m_firewallStatusKnown = false;
        m_firewallEnabled = false;
        m_firewallRulesPresent = true;
        m_firewallManageWith = QStringLiteral("nft");
        m_firewallNote = QStringLiteral(
            "nftables rules are present; manage them with `sudo nft list ruleset`.");
        m_ports.clear();
        m_servicesList.clear();
    } else if (mode == QLatin1String("ufw")) {
        m_firewallBackend = QStringLiteral("ufw");
        m_firewallStatusKnown = true;
        m_firewallEnabled = true;
        m_firewallRulesPresent = false;
        m_firewallManageWith = QStringLiteral("ufw");
        m_firewallNote = QStringLiteral(
            "ufw rules need administrator rights; manage them with "
            "`sudo ufw status verbose`.");
        m_ports.clear();
        m_servicesList.clear();
    } else if (mode == QLatin1String("iptables")) {
        m_firewallBackend = QStringLiteral("iptables");
        m_firewallStatusKnown = false;
        m_firewallEnabled = false;
        m_firewallRulesPresent = true;
        m_firewallManageWith = QStringLiteral("iptables");
        m_firewallNote = QStringLiteral(
            "iptables rules present; no supported manager. Inspect with "
            "`sudo iptables -S`.");
        m_ports.clear();
        m_servicesList.clear();
    } else if (mode == QLatin1String("no-firewall")) {
        m_firewallDetected = false;
        m_firewallStatusKnown = false;
        m_firewallEnabled = false;
        m_firewallRulesPresent = false;
        m_firewallBackend = QStringLiteral("none");
        m_firewallManageWith.clear();
        m_firewallNote.clear();
        m_ports.clear();
        m_servicesList.clear();
    }
}

void MockNetworkClient::resetForTest()
{
    m_available = true;
    m_networkPresent = true;
    m_firewallDetected = true;
    m_firewallStatusKnown = true;
    m_firewallEnabled = false;
    m_firewallRulesPresent = true;
    m_firewallBackend = QStringLiteral("firewalld");
    m_firewallManageWith = QStringLiteral("firewall-cmd");
    m_firewallNote.clear();
    m_failOperation.clear();
    m_failMessage.clear();
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
          QStringLiteral("192.168.1.20"), 24, QStringLiteral("192.168.1.1"),
          { QStringLiteral("192.168.1.1"), QStringLiteral("1.1.1.1") },
          { QStringLiteral("lan") },
          { QStringLiteral("2001:db8::20") },
          QStringLiteral("auto"), QStringLiteral("none"),
          QStringLiteral("stable"), QStringLiteral(""),
          1 },
        { QStringLiteral("Home Wi-Fi"), QStringLiteral("mock-wifi-dragonfruit"),
          QStringLiteral("wifi"), true, true, 5,
          QStringLiteral("wlan0"), QStringLiteral("aa:bb:cc:dd:ee:01"),
          QStringLiteral("192.168.1.42"), 24, QStringLiteral("192.168.1.1"),
          { QStringLiteral("192.168.1.1") },
          { QStringLiteral("lan") },
          { QStringLiteral("2001:db8::42") },
          QStringLiteral("auto"), QStringLiteral("none"),
          QStringLiteral("random"), QStringLiteral("wpa-psk"),
          2 },
        { QStringLiteral("Thunderbolt Bridge"), QStringLiteral("mock-bridge"),
          QStringLiteral("bridge"), false, false, 0,
          QStringLiteral("bridge0"), QStringLiteral("aa:bb:cc:dd:ee:03"),
          QString(), 0, QString(), {}, {}, {}, QString(), QString(),
          QString(), QString(), 0 },
    };
    rebuild();
}

void MockNetworkClient::finishWrite(const QString &operation,
                                    const QString &outcome,
                                    const QString &message)
{
    emit writeFinished(operation, outcome, message);
}

bool MockNetworkClient::failWrite(const QString &operation)
{
    if (m_failOperation != operation)
        return false;
    const QString message = m_failMessage;
    m_failOperation.clear();
    m_failMessage.clear();
    finishWrite(operation, QStringLiteral("failed"), message);
    return true;
}

void MockNetworkClient::failNextWrite(const QString &operation,
                                      const QString &message)
{
    m_failOperation = operation;
    m_failMessage = message;
}

void MockNetworkClient::refresh()
{
    rebuild();
}

void MockNetworkClient::activateService(const QString &uuid)
{
    if (failWrite(QStringLiteral("activate")))
        return;
    for (Service &service : m_services)
        service.connected = service.uuid == uuid;
    rebuild();
    finishWrite(QStringLiteral("activate"), QStringLiteral("applied"));
}

void MockNetworkClient::deactivateService(const QString &uuid)
{
    if (failWrite(QStringLiteral("deactivate")))
        return;
    for (Service &service : m_services) {
        if (service.uuid == uuid)
            service.connected = false;
    }
    rebuild();
    finishWrite(QStringLiteral("deactivate"), QStringLiteral("applied"));
}

void MockNetworkClient::moveService(const QString &uuid, int delta)
{
    if (failWrite(QStringLiteral("move")))
        return;
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
    finishWrite(QStringLiteral("move"), QStringLiteral("applied"));
}

void MockNetworkClient::addService(const QString &name, const QString &kind,
                                   const QString &plugin, const QString &gateway,
                                   const QString &interfaceName)
{
    if (failWrite(QStringLiteral("add")))
        return;
    if (name.trimmed().isEmpty()) {
        finishWrite(QStringLiteral("add"), QStringLiteral("failed"),
                    QStringLiteral("A service name is required."));
        return;
    }
    if (kind == QLatin1String("vpn") && plugin.isEmpty()) {
        finishWrite(QStringLiteral("add"), QStringLiteral("failed"),
                    QStringLiteral("Choose a VPN plugin."));
        return;
    }
    Service service;
    service.id = name;
    service.uuid = QStringLiteral("mock-added-") + QString::number(m_services.size() + 1);
    service.kind = kind;
    service.autoconnect = true;
    service.priority = 0;
    service.interface = interfaceName;
    service.gateway = gateway;
    service.keyMgmt = plugin;
    m_services.append(service);
    rebuild();
    finishWrite(QStringLiteral("add"), QStringLiteral("applied"));
}

void MockNetworkClient::forgetService(const QString &uuid)
{
    if (failWrite(QStringLiteral("forget")))
        return;
    const int before = m_services.size();
    for (int i = 0; i < m_services.size(); ++i) {
        if (m_services[i].uuid == uuid) {
            m_services.removeAt(i);
            break;
        }
    }
    if (m_services.size() == before) {
        finishWrite(QStringLiteral("forget"), QStringLiteral("failed"),
                    QStringLiteral("The service no longer exists."));
        return;
    }
    rebuild();
    finishWrite(QStringLiteral("forget"), QStringLiteral("applied"));
}

void MockNetworkClient::setAutoconnect(const QString &uuid, bool enabled)
{
    if (failWrite(QStringLiteral("autoconnect")))
        return;
    for (Service &service : m_services) {
        if (service.uuid == uuid)
            service.autoconnect = enabled;
    }
    rebuild();
    finishWrite(QStringLiteral("autoconnect"), QStringLiteral("applied"));
}

void MockNetworkClient::setFirewallEnabled(bool enabled)
{
    if (failWrite(QStringLiteral("firewall")))
        return;
    if (!m_firewallStatusKnown)
        return;
    m_firewallEnabled = enabled;
    rebuild();
    finishWrite(QStringLiteral("firewall"), QStringLiteral("applied"));
}

void MockNetworkClient::openPort(int port, const QString &protocol)
{
    if (failWrite(QStringLiteral("port")))
        return;
    if (!m_firewallStatusKnown || !m_firewallRulesPresent)
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
    finishWrite(QStringLiteral("port"), QStringLiteral("applied"));
}

void MockNetworkClient::closePort(int port, const QString &protocol)
{
    if (failWrite(QStringLiteral("port")))
        return;
    if (!m_firewallStatusKnown || !m_firewallRulesPresent)
        return;
    const QString id = QStringLiteral("%1/%2").arg(port).arg(protocol);
    for (int i = 0; i < m_ports.size(); ++i) {
        if (m_ports[i].toMap().value(QStringLiteral("id")).toString() == id) {
            m_ports.removeAt(i);
            break;
        }
    }
    rebuild();
    finishWrite(QStringLiteral("port"), QStringLiteral("applied"));
}

void MockNetworkClient::openService(const QString &service)
{
    if (failWrite(QStringLiteral("service")))
        return;
    if (!m_firewallStatusKnown || !m_firewallRulesPresent)
        return;
    if (!m_servicesList.contains(service))
        m_servicesList.append(service);
    rebuild();
    finishWrite(QStringLiteral("service"), QStringLiteral("applied"));
}

void MockNetworkClient::closeService(const QString &service)
{
    if (failWrite(QStringLiteral("service")))
        return;
    if (!m_firewallStatusKnown || !m_firewallRulesPresent)
        return;
    m_servicesList.removeAll(service);
    rebuild();
    finishWrite(QStringLiteral("service"), QStringLiteral("applied"));
}

void MockNetworkClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("network"));
    view.insert(QStringLiteral("state"),
                (m_available && m_networkPresent)
                    ? QStringLiteral("available")
                    : QStringLiteral("unavailable"));
    view.insert(QStringLiteral("radioEnabled"), m_networkPresent);
    view.insert(QStringLiteral("readOnly"), false);
    view.insert(QStringLiteral("note"), m_networkPresent
                    ? QString()
                    : QStringLiteral("NetworkManager is not running."));
    QString activeSsid;
    QString activeAddress;
    if (m_networkPresent) {
        for (const Service &service : m_services) {
            if (service.connected && service.kind == QLatin1String("wifi")) {
                activeSsid = QStringLiteral("dragonfruit");
                activeAddress = service.address;
            }
        }
    }
    view.insert(QStringLiteral("activeSsid"), activeSsid);
    view.insert(QStringLiteral("activeAddress"), activeAddress);

    if (m_networkPresent) {
        std::sort(m_services.begin(), m_services.end(),
                  [](const Service &left, const Service &right) {
                      if (left.priority != right.priority)
                          return left.priority > right.priority;
                      return left.id < right.id;
                  });
    }
    QVariantList services;
    if (m_networkPresent) {
        for (int i = 0; i < m_services.size(); ++i) {
            const Service &service = m_services[i];
            QString kindLabel = QStringLiteral("Other");
            if (service.kind == QLatin1String("ethernet"))
                kindLabel = QStringLiteral("Ethernet");
            else if (service.kind == QLatin1String("wifi"))
                kindLabel = QStringLiteral("Wi-Fi");
            else if (service.kind == QLatin1String("bridge"))
                kindLabel = QStringLiteral("Bridge");
            else if (service.kind == QLatin1String("bond"))
                kindLabel = QStringLiteral("Bond");
            else if (service.kind == QLatin1String("vlan"))
                kindLabel = QStringLiteral("VLAN");
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
            entry.insert(QStringLiteral("status"),
                         service.connected ? QStringLiteral("green")
                                           : QStringLiteral("gray"));
            entry.insert(QStringLiteral("connected"), service.connected);
            entry.insert(QStringLiteral("autoconnect"), service.autoconnect);
            entry.insert(QStringLiteral("priority"), service.priority);
            entry.insert(QStringLiteral("order"), i);
            entry.insert(QStringLiteral("interface"), service.interface);
            entry.insert(QStringLiteral("hardware"), service.hardware);
            entry.insert(QStringLiteral("address"), service.address);
            entry.insert(QStringLiteral("prefix"), service.prefix);
            entry.insert(QStringLiteral("gateway"), service.gateway);
            entry.insert(QStringLiteral("dns"), service.dns);
            entry.insert(QStringLiteral("searchDomains"), service.searchDomains);
            entry.insert(QStringLiteral("ipv6Addresses"), service.ipv6Addresses);
            entry.insert(QStringLiteral("ipv6"), service.ipv6Method);
            entry.insert(QStringLiteral("proxy"), service.proxy);
            entry.insert(QStringLiteral("clonedMac"), service.clonedMac);
            entry.insert(QStringLiteral("keyMgmt"), service.keyMgmt);
            entry.insert(QStringLiteral("metered"), service.metered);
            entry.insert(QStringLiteral("wirelessMode"),
                         service.kind == QLatin1String("wifi")
                             ? QStringLiteral("infrastructure") : QString());
            entry.insert(QStringLiteral("hotspot"), false);
            services.append(entry);
        }
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
                        ? QStringLiteral("nftables")
                        : (m_firewallBackend == QLatin1String("ufw")
                               ? QStringLiteral("ufw")
                               : (m_firewallBackend == QLatin1String("iptables")
                                      ? QStringLiteral("iptables")
                                      : (m_firewallBackend == QLatin1String("none")
                                             ? QStringLiteral("No firewall detected")
                                             : QStringLiteral("firewalld")))));
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
    firewall.insert(QStringLiteral("rulesPresent"), m_firewallRulesPresent);
    firewall.insert(QStringLiteral("manageWith"), m_firewallManageWith);
    firewall.insert(QStringLiteral("authNote"),
                    QStringLiteral("Each change may ask for administrator "
                                   "authentication."));
    firewall.insert(QStringLiteral("readOnly"), false);
    firewall.insert(QStringLiteral("readOnlyNote"), QString());
    view.insert(QStringLiteral("firewall"), firewall);

    m_view = view;
    emit changed(m_view);
}
