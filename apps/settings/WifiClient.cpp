// SPDX-License-Identifier: GPL-3.0-only
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
    call(QStringLiteral("Refresh"), {}, QString(), true);
}

void DbusWifiClient::setEnabled(bool enabled)
{
    call(QStringLiteral("SetWirelessEnabled"), {enabled}, QStringLiteral("power"));
}

void DbusWifiClient::join(const QString &ssid, const QString &secret)
{
    call(QStringLiteral("Join"), {ssid, secret}, QStringLiteral("join"));
}

void DbusWifiClient::joinHidden(const QString &ssid, const QString &secret)
{
    call(QStringLiteral("JoinHidden"), {ssid, secret}, QStringLiteral("joinHidden"));
}

void DbusWifiClient::forget(const QString &uuid)
{
    call(QStringLiteral("Forget"), {uuid}, QStringLiteral("forget"));
}

void DbusWifiClient::setAutoconnect(const QString &uuid, bool enabled)
{
    call(QStringLiteral("SetAutoconnect"), {uuid, enabled},
         QStringLiteral("autoconnect"));
}

void DbusWifiClient::scan()
{
    call(QStringLiteral("Scan"), {}, QStringLiteral("scan"));
}

void DbusWifiClient::disconnect(const QString &uuid)
{
    call(QStringLiteral("Disconnect"), {uuid}, QStringLiteral("disconnect"));
}

void DbusWifiClient::startHotspot(const QString &ssid, const QString &secret)
{
    call(QStringLiteral("StartHotspot"), {ssid, secret}, QStringLiteral("hotspot"));
}

void DbusWifiClient::stopHotspot(const QString &uuid)
{
    call(QStringLiteral("StopHotspot"), {uuid}, QStringLiteral("hotspot"));
}

void DbusWifiClient::call(const QString &method, const QVariantList &arguments,
                          const QString &operation, bool isRead)
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
                                     QDBusConnection::sessionBus(), this);
    if (!iface->isValid()) {
        iface->deleteLater();
        if (m_available) {
            m_available = false;
            emit availableChanged(false);
        }
        if (!isRead && !operation.isEmpty())
            emit writeFinished(operation, QStringLiteral("absent"), QString());
        return;
    }
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isRead, operation, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid()) {
                    if (!isRead && !operation.isEmpty())
                        emit writeFinished(operation, QStringLiteral("failed"), QString());
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
    else if (mode == QLatin1String("scanning"))
        m_scanning = true;
    else if (mode == QLatin1String("empty"))
        m_networks.clear();
    else if (mode == QLatin1String("error"))
        m_error = true;
    rebuild();
    emit availableChanged(true);
}

void MockWifiClient::resetForTest()
{
    m_available = true;
    m_enabled = true;
    m_scanning = false;
    m_error = false;
    m_hotspotSsid.clear();
    m_hotspotUuid.clear();
    m_hotspotActive = false;
    m_failOperation.clear();
    m_failMessage.clear();
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
    // A read answers the scan, exactly like the host's next NM read.
    m_scanning = false;
    rebuild();
}

void MockWifiClient::finishWrite(const QString &operation, const QString &outcome,
                                 const QString &message)
{
    emit writeFinished(operation, outcome, message);
}

bool MockWifiClient::failWrite(const QString &operation)
{
    if (m_failOperation != operation)
        return false;
    const QString message = m_failMessage;
    m_failOperation.clear();
    m_failMessage.clear();
    finishWrite(operation, QStringLiteral("failed"), message);
    return true;
}

void MockWifiClient::failNextWrite(const QString &operation, const QString &message)
{
    m_failOperation = operation;
    m_failMessage = message;
}

void MockWifiClient::setEnabled(bool enabled)
{
    if (failWrite(QStringLiteral("power")))
        return;
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
    finishWrite(QStringLiteral("power"), QStringLiteral("applied"));
}

void MockWifiClient::join(const QString &ssid, const QString &)
{
    if (failWrite(QStringLiteral("join")))
        return;
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
    bool known = false;
    for (const Known &entry : m_known) {
        if (entry.ssid == ssid)
            known = true;
    }
    if (!known) {
        m_known.append({ ssid, uuidForSsid(ssid), true, true, 1762000001,
                         QStringLiteral("WPA2"), true });
    }
    rebuild();
    finishWrite(QStringLiteral("join"), QStringLiteral("applied"));
}

void MockWifiClient::joinHidden(const QString &ssid, const QString &)
{
    if (failWrite(QStringLiteral("joinHidden")))
        return;
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
    finishWrite(QStringLiteral("joinHidden"), QStringLiteral("applied"));
}

void MockWifiClient::forget(const QString &uuid)
{
    if (failWrite(QStringLiteral("forget")))
        return;
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
    finishWrite(QStringLiteral("forget"), QStringLiteral("applied"));
}

void MockWifiClient::setAutoconnect(const QString &uuid, bool enabled)
{
    if (failWrite(QStringLiteral("autoconnect")))
        return;
    for (Known &known : m_known) {
        if (known.uuid == uuid) {
            known.autoconnect = enabled;
            break;
        }
    }
    rebuild();
    finishWrite(QStringLiteral("autoconnect"), QStringLiteral("applied"));
}

void MockWifiClient::scan()
{
    if (failWrite(QStringLiteral("scan")))
        return;
    m_scanning = true;
    rebuild();
    finishWrite(QStringLiteral("scan"), QStringLiteral("applied"));
}

void MockWifiClient::disconnect(const QString &uuid)
{
    if (failWrite(QStringLiteral("disconnect")))
        return;
    if (uuid.isEmpty() || m_activeSsid.isEmpty())
        return;
    const Known known = [&]() {
        for (const Known &entry : m_known)
            if (entry.uuid == uuid)
                return entry;
        return Known{};
    }();
    if (known.ssid != m_activeSsid)
        return;
    m_activeSsid.clear();
    for (Network &network : m_networks)
        network.active = false;
    for (Known &entry : m_known)
        entry.connected = false;
    rebuild();
    finishWrite(QStringLiteral("disconnect"), QStringLiteral("applied"));
}

void MockWifiClient::startHotspot(const QString &ssid, const QString &)
{
    if (failWrite(QStringLiteral("hotspot")))
        return;
    if (!m_enabled || ssid.isEmpty())
        return;
    m_hotspotSsid = ssid;
    m_hotspotUuid = uuidForSsid(ssid);
    m_hotspotActive = true;
    rebuild();
    finishWrite(QStringLiteral("hotspot"), QStringLiteral("applied"));
}

void MockWifiClient::stopHotspot(const QString &uuid)
{
    if (failWrite(QStringLiteral("hotspot")))
        return;
    if (uuid.isEmpty() || uuid != m_hotspotUuid)
        return;
    m_hotspotActive = false;
    rebuild();
    finishWrite(QStringLiteral("hotspot"), QStringLiteral("applied"));
}

void MockWifiClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("wifi"));
    view.insert(QStringLiteral("state"),
                m_error ? QStringLiteral("error") : QStringLiteral("available"));
    if (m_error)
        view.insert(QStringLiteral("error"),
                    QStringLiteral("NetworkManager: the read failed."));
    view.insert(QStringLiteral("scanning"), m_scanning);
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
        // T271 Linux-only rows with a real NetworkManager owner.
        { QStringLiteral("wirelessMode"), QStringLiteral("infrastructure") },
        { QStringLiteral("metered"), 2 },
        { QStringLiteral("ipv6"), QStringLiteral("auto") },
        { QStringLiteral("dns"), QVariantList{ QStringLiteral("192.168.1.1"),
                                               QStringLiteral("1.1.1.1") } },
        { QStringLiteral("proxy"), QStringLiteral("none") },
        { QStringLiteral("clonedMac"), QStringLiteral("preserve") },
        { QStringLiteral("keyMgmt"), QStringLiteral("wpa-psk") },
        { QStringLiteral("hotspot"), false },
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
    view.insert(QStringLiteral("hotspotSupported"), true);
    view.insert(QStringLiteral("hotspotActive"), m_hotspotActive);
    view.insert(QStringLiteral("hotspotSsid"), m_hotspotSsid);
    view.insert(QStringLiteral("hotspotUuid"), m_hotspotUuid);

    m_view = view;
    emit changed(m_view);
}
