// SPDX-License-Identifier: MIT
#include "systemstatusclient.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>

#if defined(QT_DBUS_LIB)
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#endif

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kWifiInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Wifi");
const QString kAudioInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Audio");
const QString kBatteryInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Battery");
const QString kBluetoothInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Bluetooth");

// Serialize a QJsonObject to the compact byte form the host uses.
QByteArray compact(const QJsonObject &object)
{
    return QJsonDocument(object).toJson(QJsonDocument::Compact);
}

} // namespace

// --- DbusSystemStatusClient ------------------------------------------------

DbusSystemStatusClient::DbusSystemStatusClient(QObject *parent)
    : SystemStatusClient(parent)
    , m_service(kService)
    , m_path(kPath)
{
#if defined(QT_DBUS_LIB)
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (QDBusConnectionInterface *iface = bus.interface()) {
        const auto update = [this](const QString &name) {
            if (name != m_service)
                return;
            const bool available = isAvailable();
            if (available != m_available) {
                m_available = available;
                emit availableChanged(m_available);
            }
        };
        connect(iface, &QDBusConnectionInterface::serviceRegistered, this, update);
        connect(iface, &QDBusConnectionInterface::serviceUnregistered, this, update);
    }
    m_available = isAvailable();
#endif
}

bool DbusSystemStatusClient::isAvailable() const
{
#if defined(QT_DBUS_LIB)
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
#else
    return false;
#endif
}

void DbusSystemStatusClient::call(const QString &interface, const QString &method,
                                  const QVariantList &arguments, ReplySignal replySignal)
{
#if defined(QT_DBUS_LIB)
    auto *call = new QDBusInterface(m_service, m_path, interface,
                                    QDBusConnection::sessionBus(), this);
    if (!call->isValid()) {
        call->deleteLater();
        m_available = false;
        emit availableChanged(false);
        return;
    }
    QDBusPendingCallWatcher *watcher = new QDBusPendingCallWatcher(
        call->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, replySignal, call]() {
                // The host returns the JSON payload as a D-Bus string.
                const QDBusPendingReply<QString> reply = *watcher;
                if (reply.isValid()) {
                    (this->*replySignal)(reply.value().toUtf8());
                } else {
                    qWarning() << "dragonfruit-shell: system-status call failed:"
                               << watcher->reply().errorMessage();
                }
                call->deleteLater();
                watcher->deleteLater();
            });
#else
    Q_UNUSED(interface);
    Q_UNUSED(method);
    Q_UNUSED(arguments);
    Q_UNUSED(replySignal);
#endif
}

void DbusSystemStatusClient::refreshWifi()
{
    call(kWifiInterface, QStringLiteral("State"), {}, &SystemStatusClient::wifiState);
}

void DbusSystemStatusClient::refreshAudio()
{
    call(kAudioInterface, QStringLiteral("State"), {}, &SystemStatusClient::audioState);
}

void DbusSystemStatusClient::refreshBattery()
{
    call(kBatteryInterface, QStringLiteral("State"), {}, &SystemStatusClient::batteryState);
}

void DbusSystemStatusClient::refreshBluetooth()
{
    call(kBluetoothInterface, QStringLiteral("State"), {},
         &SystemStatusClient::bluetoothState);
}

void DbusSystemStatusClient::setBluetoothPowered(bool powered)
{
    call(kBluetoothInterface, QStringLiteral("SetPowered"), {powered},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::setBluetoothDiscovering(bool discovering)
{
    call(kBluetoothInterface, QStringLiteral("SetDiscovering"), {discovering},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::pairBluetooth(const QString &address)
{
    call(kBluetoothInterface, QStringLiteral("Pair"), {address},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::setBluetoothConnected(const QString &address, bool connected)
{
    call(kBluetoothInterface, QStringLiteral("SetConnected"), {address, connected},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::join(const QString &ssid, const QString &secret)
{
    call(kWifiInterface, QStringLiteral("Join"), {ssid, secret},
         &SystemStatusClient::joinReport);
}

void DbusSystemStatusClient::setVolume(double volume)
{
    call(kAudioInterface, QStringLiteral("SetVolume"), {volume},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::setMute(bool muted)
{
    call(kAudioInterface, QStringLiteral("SetMute"), {muted},
         &SystemStatusClient::writeReport);
}

// --- MockSystemStatusClient ------------------------------------------------

MockSystemStatusClient::MockSystemStatusClient(QObject *parent)
    : SystemStatusClient(parent)
{
    refreshWifi();
    refreshAudio();
    refreshBattery();
    refreshBluetooth();
}

void MockSystemStatusClient::refreshBluetooth()
{
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("bluetooth"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("glyph"),
                m_btPowered ? QStringLiteral("bluetooth")
                            : QStringLiteral("bluetooth-disabled"));
    view.insert(QStringLiteral("label"),
                m_btPowered ? (m_btDiscovering ? QStringLiteral("Bluetooth discovering")
                                               : QStringLiteral("Bluetooth on"))
                            : QStringLiteral("Bluetooth off"));
    view.insert(QStringLiteral("powered"), m_btPowered);
    view.insert(QStringLiteral("discovering"), m_btDiscovering);
    view.insert(QStringLiteral("discoverable"), true);
    view.insert(QStringLiteral("pairable"), true);
    view.insert(QStringLiteral("adapterName"), QStringLiteral("workstation"));
    view.insert(QStringLiteral("connectedCount"), m_btDeviceConnected ? 1 : 0);
    view.insert(QStringLiteral("knownCount"), 2);
    view.insert(QStringLiteral("nearbyCount"), m_btDiscovering ? 1 : 0);

    const auto device = [](const QString &address, const QString &name, bool paired,
                           bool connected, int signal) {
        QJsonObject entry;
        entry.insert(QStringLiteral("address"), address);
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("paired"), paired);
        entry.insert(QStringLiteral("connected"), connected);
        entry.insert(QStringLiteral("trusted"), paired);
        entry.insert(QStringLiteral("blocked"), false);
        entry.insert(QStringLiteral("rssi"), signal);
        entry.insert(QStringLiteral("signal"), signal);
        entry.insert(QStringLiteral("icon"), QStringLiteral("audio-headset"));
        return entry;
    };

    QJsonArray known;
    known.append(device(QStringLiteral("AA:BB:CC:DD:EE:FF"), QStringLiteral("WF-1000XM6"),
                        true, m_btDeviceConnected, 64));
    known.append(device(QStringLiteral("11:22:33:44:55:66"), QStringLiteral("WH-1000XM6"),
                        true, false, 48));
    view.insert(QStringLiteral("knownDevices"), known);

    QJsonArray nearby;
    if (m_btDiscovering) {
        nearby.append(device(QStringLiteral("22:33:44:55:66:77"), QStringLiteral("Nearby Speaker"),
                             false, false, 30));
    }
    view.insert(QStringLiteral("nearbyDevices"), nearby);
    emit bluetoothState(compact(view));
}

void MockSystemStatusClient::setBluetoothPowered(bool powered)
{
    m_btPowered = powered;
    if (!powered)
        m_btDiscovering = false;
    refreshBluetooth();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::setBluetoothDiscovering(bool discovering)
{
    m_btDiscovering = discovering && m_btPowered;
    refreshBluetooth();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::pairBluetooth(const QString &)
{
    refreshBluetooth();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::setBluetoothConnected(const QString &address, bool connected)
{
    if (address == QStringLiteral("AA:BB:CC:DD:EE:FF"))
        m_btDeviceConnected = connected;
    refreshBluetooth();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::refreshWifi()
{
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("wifi"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("wifi-secure"));
    view.insert(QStringLiteral("label"),
                m_activeSsid + QStringLiteral(" \u00b7 82%"));
    view.insert(QStringLiteral("radioEnabled"), true);
    view.insert(QStringLiteral("wifiState"), QStringLiteral("connected"));
    view.insert(QStringLiteral("connectivity"), QStringLiteral("Connected"));
    view.insert(QStringLiteral("activeSsid"), m_activeSsid);
    view.insert(QStringLiteral("readOnly"), false);
    view.insert(QStringLiteral("note"), QString());
    view.insert(QStringLiteral("networkCount"), 3);

    const auto network = [this](const QString &ssid, int strength, const QString &security,
                                bool secured, bool active, const QString &band) {
        QJsonObject entry;
        entry.insert(QStringLiteral("ssid"), ssid);
        entry.insert(QStringLiteral("strength"), strength);
        entry.insert(QStringLiteral("security"), security);
        entry.insert(QStringLiteral("secured"), secured);
        entry.insert(QStringLiteral("active"), active);
        entry.insert(QStringLiteral("band"), band);
        return entry;
    };
    QJsonArray networks;
    networks.append(network(m_activeSsid, 82, QStringLiteral("WPA2"), true, true,
                            QStringLiteral("5 GHz")));
    networks.append(network(QStringLiteral("dragonfruit-guest"), 61, QStringLiteral("Open"),
                            false, false, QStringLiteral("2.4 GHz")));
    networks.append(network(QStringLiteral("NeighbourNet"), 34, QStringLiteral("WPA3"), true,
                            false, QStringLiteral("5 GHz")));
    view.insert(QStringLiteral("networks"), networks);
    emit wifiState(compact(view));
}

void MockSystemStatusClient::refreshAudio()
{
    const int percent = m_muted ? 0 : qRound(m_volume * 100.0);
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("audio"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"),
                (m_muted || m_volume <= 0.0) ? QStringLiteral("volume-muted")
                                             : QStringLiteral("volume"));
    view.insert(QStringLiteral("label"),
                m_muted ? QStringLiteral("Muted")
                        : QStringLiteral("%1%").arg(percent));
    view.insert(QStringLiteral("volume"), m_volume);
    view.insert(QStringLiteral("percent"), percent);
    view.insert(QStringLiteral("muted"), m_muted);
    view.insert(QStringLiteral("defaultSink"), QStringLiteral("speakers"));
    view.insert(QStringLiteral("sinkCount"), 2);

    const auto sink = [](const QString &name, const QString &description, double volume,
                         bool muted, bool isDefault) {
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), name == QStringLiteral("speakers") ? 7 : 9);
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("description"), description);
        entry.insert(QStringLiteral("volume"), volume);
        entry.insert(QStringLiteral("percent"), qRound(volume * 100.0));
        entry.insert(QStringLiteral("muted"), muted);
        entry.insert(QStringLiteral("default"), isDefault);
        return entry;
    };
    QJsonArray sinks;
    sinks.append(sink(QStringLiteral("speakers"), QStringLiteral("Built-in Speakers"), m_volume,
                      m_muted, true));
    sinks.append(sink(QStringLiteral("headphones"), QStringLiteral("Headphones"),
                      m_volume, m_muted, false));
    view.insert(QStringLiteral("sinks"), sinks);
    emit audioState(compact(view));
}

void MockSystemStatusClient::refreshBattery()
{
    // A present, 82%-charged battery on line power: enough to render every
    // part of the read-only popover (level fill, label, charge state).
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("battery"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("glyph"), QStringLiteral("battery"));
    view.insert(QStringLiteral("label"), QStringLiteral("82% charging"));
    view.insert(QStringLiteral("percent"), 82);
    view.insert(QStringLiteral("level"), 0.82);
    view.insert(QStringLiteral("charging"), true);
    view.insert(QStringLiteral("plugged"), true);
    view.insert(QStringLiteral("onBattery"), false);
    view.insert(QStringLiteral("timeToEmpty"), QJsonValue::Null);
    view.insert(QStringLiteral("timeToFull"), 5400);
    emit batteryState(compact(view));
}

void MockSystemStatusClient::join(const QString &ssid, const QString &)
{
    if (!ssid.isEmpty()) {
        m_activeSsid = ssid;
        refreshWifi();
    }
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit joinReport(compact(report));
}

void MockSystemStatusClient::setVolume(double volume)
{
    m_volume = qBound(0.0, volume, 1.0);
    if (m_volume > 0.0)
        m_muted = false;
    refreshAudio();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("applied"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::setMute(bool muted)
{
    m_muted = muted;
    refreshAudio();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("applied"));
    emit writeReport(compact(report));
}