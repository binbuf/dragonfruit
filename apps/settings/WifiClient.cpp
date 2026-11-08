// SPDX-License-Identifier: MIT
#include "WifiClient.h"

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
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Wifi");

QString uuidForSsid(const QString &ssid)
{
    return QStringLiteral("mock-wifi-") + ssid;
}

} // namespace

// --- DbusWifiClient --------------------------------------------------------

DbusWifiClient::DbusWifiClient(QObject *parent)
    : WifiClient(parent)
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

DbusWifiClient::~DbusWifiClient() = default;

bool DbusWifiClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusWifiClient::refresh()
{
    // `Refresh` re-reads NetworkManager before returning the view; `State`
    // would only replay the host's startup snapshot. The pane calls this on
    // open, and every write re-reads through it.
    call(QStringLiteral("Refresh"), {});
}

void DbusWifiClient::setEnabled(bool enabled)
{
    call(QStringLiteral("SetWirelessEnabled"), {enabled});
}

void DbusWifiClient::join(const QString &ssid, const QString &secret)
{
    call(QStringLiteral("Join"), {ssid, secret});
}

void DbusWifiClient::joinHidden(const QString &ssid, const QString &secret)
{
    call(QStringLiteral("JoinHidden"), {ssid, secret});
}

void DbusWifiClient::forget(const QString &uuid)
{
    call(QStringLiteral("Forget"), {uuid});
}

void DbusWifiClient::setAutoconnect(const QString &uuid, bool enabled)
{
    call(QStringLiteral("SetAutoconnect"), {uuid, enabled});
}

void DbusWifiClient::call(const QString &method, const QVariantList &arguments)
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

void DbusWifiClient::applyReply(const QByteArray &json)
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

// --- MockWifiClient --------------------------------------------------------

MockWifiClient::MockWifiClient(QObject *parent)
    : WifiClient(parent)
{
    const QString fixture = qEnvironmentVariable("DF_WIFI_FIXTURE");
    if (fixture == QLatin1String("absent")) {
        m_available = false;
        m_view.clear();
        return;
    }
    resetForTest();
    if (fixture == QLatin1String("off"))
        setEnabled(false);
}

void MockWifiClient::setFixture(const QString &mode)
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
    if (mode == QLatin1String("off"))
        setEnabled(false);
    emit availableChanged(true);
}

void MockWifiClient::resetForTest()
{
    m_available = true;
    m_enabled = true;
    m_networks = {
        { QStringLiteral("dragonfruit"), 82, QStringLiteral("WPA2"), true, true, 5180 },
        { QStringLiteral("cafe-guest"), 58, QStringLiteral("Open"), false, false, 2412 },
        { QStringLiteral("home-office"), 44, QStringLiteral("WPA3"), true, false, 5745 },
        { QStringLiteral("neighbor-secure"), 36, QStringLiteral("WPA2"), true, false, 2412 },
        { QStringLiteral("printer-setup"), 22, QStringLiteral("Open"), false, false, 2412 },
    };
    m_known = {
        { QStringLiteral("dragonfruit"), uuidForSsid(QStringLiteral("dragonfruit")),
          true, true, 1762000000, QStringLiteral("WPA2"), true },
        { QStringLiteral("home-office"), uuidForSsid(QStringLiteral("home-office")),
          true, false, 1761000000, QStringLiteral("WPA3"), true },
        { QStringLiteral("old-office"), uuidForSsid(QStringLiteral("old-office")),
          false, false, 1759000000, QString(), false },
    };
    m_activeSsid = QStringLiteral("dragonfruit");
    rebuild();
}

void MockWifiClient::refresh()
{
    rebuild();
}

void MockWifiClient::setEnabled(bool enabled)
{
    m_enabled = enabled;
    if (!enabled)
        m_activeSsid.clear();
    else if (m_activeSsid.isEmpty())
        m_activeSsid = QStringLiteral("dragonfruit");
    for (Network &network : m_networks)
        network.active = m_enabled && network.ssid == m_activeSsid;
    for (Known &known : m_known)
        known.connected = m_enabled && known.ssid == m_activeSsid;
    rebuild();
}

void MockWifiClient::join(const QString &ssid, const QString &)
{
    if (!m_enabled)
        return;
    m_activeSsid = ssid;
    bool found = false;
    for (Network &network : m_networks) {
        network.active = network.ssid == ssid;
        if (network.active)
            found = true;
    }
    if (!found) {
        m_networks.append({ ssid, 70, QStringLiteral("WPA2"), true, true, 2412 });
    }
    for (Known &known : m_known) {
        known.connected = known.ssid == ssid;
        if (known.connected)
            known.autoconnect = true;
    }
    for (const Known &known : m_known) {
        if (known.ssid == ssid)
            return rebuild();
    }
    m_known.append({ ssid, uuidForSsid(ssid), true, true, 1762000001,
                     QStringLiteral("WPA2"), true });
    rebuild();
}

void MockWifiClient::joinHidden(const QString &ssid, const QString &)
{
    if (!m_enabled || ssid.isEmpty())
        return;
    bool found = false;
    for (Network &network : m_networks) {
        if (network.ssid == ssid) {
            network.active = true;
            found = true;
        } else {
            network.active = false;
        }
    }
    if (!found) {
        m_networks.append({ ssid, 100, QStringLiteral("WPA2"), true, true, 2412 });
    }
    m_activeSsid = ssid;
    bool known = false;
    for (Known &entry : m_known) {
        entry.connected = entry.ssid == ssid;
        if (entry.ssid == ssid)
            known = true;
    }
    if (!known) {
        m_known.append({ ssid, uuidForSsid(ssid), true, true, 1762000002,
                         QStringLiteral("WPA2"), true });
    }
    rebuild();
}

void MockWifiClient::forget(const QString &uuid)
{
    QString forgottenSsid;
    for (int i = 0; i < m_known.size(); ++i) {
        if (m_known[i].uuid != uuid)
            continue;
        forgottenSsid = m_known[i].ssid;
        m_known.removeAt(i);
        break;
    }
    if (!forgottenSsid.isEmpty() && forgottenSsid == m_activeSsid) {
        m_activeSsid.clear();
        for (Network &network : m_networks)
            network.active = false;
    }
    rebuild();
}

void MockWifiClient::setAutoconnect(const QString &uuid, bool enabled)
{
    for (Known &known : m_known) {
        if (known.uuid == uuid) {
            known.autoconnect = enabled;
            break;
        }
    }
    rebuild();
}

void MockWifiClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("wifi"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("radioEnabled"), m_enabled);
    view.insert(QStringLiteral("wifiState"),
                m_enabled ? (m_activeSsid.isEmpty() ? QStringLiteral("disconnected")
                                                    : QStringLiteral("connected"))
                          : QStringLiteral("disconnected"));
    view.insert(QStringLiteral("connectivity"),
                m_activeSsid.isEmpty() ? QStringLiteral("No Internet")
                                       : QStringLiteral("Connected"));
    view.insert(QStringLiteral("activeSsid"), m_activeSsid);
    view.insert(QStringLiteral("activeAddress"),
                m_activeSsid.isEmpty() ? QString() : QStringLiteral("192.168.1.42"));
    view.insert(QStringLiteral("glyph"),
                !m_enabled ? QStringLiteral("wifi-disabled")
                           : (m_activeSsid.isEmpty() ? QStringLiteral("wifi-off")
                                                     : QStringLiteral("wifi-secure")));
    view.insert(QStringLiteral("label"),
                !m_enabled ? QStringLiteral("Wi-Fi Off")
                           : (m_activeSsid.isEmpty() ? QStringLiteral("Not Connected")
                                                     : m_activeSsid + QStringLiteral(" · 82%")));
    view.insert(QStringLiteral("readOnly"), false);
    view.insert(QStringLiteral("note"), QString());

    QVariantList networks;
    for (const Network &network : m_networks) {
        if (!m_enabled)
            break;
        QVariantMap entry;
        entry.insert(QStringLiteral("ssid"), network.ssid);
        entry.insert(QStringLiteral("strength"), network.strength);
        entry.insert(QStringLiteral("security"), network.security);
        entry.insert(QStringLiteral("secured"), network.secured);
        entry.insert(QStringLiteral("active"), network.active);
        entry.insert(QStringLiteral("band"),
                     network.frequency >= 5925 ? QStringLiteral("6 GHz")
                                               : (network.frequency >= 5000
                                                      ? QStringLiteral("5 GHz")
                                                      : QStringLiteral("2.4 GHz")));
        entry.insert(QStringLiteral("frequency"), network.frequency);
        networks.append(entry);
    }
    view.insert(QStringLiteral("networks"), networks);
    view.insert(QStringLiteral("networkCount"), networks.size());

    QVariantList knownNetworks;
    for (const Known &known : m_known) {
        QVariantMap entry;
        entry.insert(QStringLiteral("ssid"), known.ssid);
        entry.insert(QStringLiteral("uuid"), known.uuid);
        entry.insert(QStringLiteral("autoconnect"), known.autoconnect);
        entry.insert(QStringLiteral("connected"), known.connected);
        entry.insert(QStringLiteral("lastUsed"), known.lastUsed);
        entry.insert(QStringLiteral("security"), known.security);
        entry.insert(QStringLiteral("secured"), known.secured);
        knownNetworks.append(entry);
    }
    view.insert(QStringLiteral("knownNetworks"), knownNetworks);
    view.insert(QStringLiteral("knownNetworkCount"), knownNetworks.size());

    QVariantList services;
    services.append(QVariantMap{
        { QStringLiteral("id"), QStringLiteral("Home Wi-Fi") },
        { QStringLiteral("uuid"), uuidForSsid(QStringLiteral("dragonfruit")) },
        { QStringLiteral("kind"), QStringLiteral("wifi") },
        { QStringLiteral("kindLabel"), QStringLiteral("Wi-Fi") },
        { QStringLiteral("state"), m_activeSsid.isEmpty() ? QStringLiteral("disconnected")
                                                          : QStringLiteral("connected") },
        { QStringLiteral("stateLabel"), m_activeSsid.isEmpty() ? QStringLiteral("Not connected")
                                                               : QStringLiteral("Connected") },
        { QStringLiteral("connected"), !m_activeSsid.isEmpty() },
        { QStringLiteral("autoconnect"), true },
        { QStringLiteral("order"), 0 },
        { QStringLiteral("interface"), QStringLiteral("wlan0") },
        { QStringLiteral("hardware"), QStringLiteral("aa:bb:cc:dd:ee:01") },
        { QStringLiteral("address"), m_activeSsid.isEmpty() ? QString()
                                                            : QStringLiteral("192.168.1.42") },
    });
    services.append(QVariantMap{
        { QStringLiteral("id"), QStringLiteral("Wired") },
        { QStringLiteral("uuid"), QStringLiteral("mock-wired") },
        { QStringLiteral("kind"), QStringLiteral("ethernet") },
        { QStringLiteral("kindLabel"), QStringLiteral("Ethernet") },
        { QStringLiteral("state"), QStringLiteral("connected") },
        { QStringLiteral("stateLabel"), QStringLiteral("Connected") },
        { QStringLiteral("connected"), true },
        { QStringLiteral("autoconnect"), true },
        { QStringLiteral("order"), 1 },
        { QStringLiteral("interface"), QStringLiteral("enp1s0") },
        { QStringLiteral("hardware"), QStringLiteral("aa:bb:cc:dd:ee:02") },
        { QStringLiteral("address"), QStringLiteral("192.168.1.20") },
    });
    view.insert(QStringLiteral("services"), services);
    view.insert(QStringLiteral("serviceCount"), services.size());

    m_view = view;
    emit changed(m_view);
}
