// SPDX-License-Identifier: MIT
#include "BluetoothClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Bluetooth");

} // namespace

// --- DbusBluetoothClient ---------------------------------------------------

DbusBluetoothClient::DbusBluetoothClient(QObject *parent)
    : BluetoothClient(parent)
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

DbusBluetoothClient::~DbusBluetoothClient() = default;

bool DbusBluetoothClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusBluetoothClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusBluetoothClient::setPowered(bool powered)
{
    call(QStringLiteral("SetPowered"), {powered});
}

void DbusBluetoothClient::setDiscovering(bool discovering)
{
    call(QStringLiteral("SetDiscovering"), {discovering});
}

void DbusBluetoothClient::pair(const QString &address)
{
    call(QStringLiteral("Pair"), {address});
}

void DbusBluetoothClient::setConnected(const QString &address, bool connected)
{
    call(QStringLiteral("SetConnected"), {address, connected});
}

void DbusBluetoothClient::call(const QString &method, const QVariantList &arguments)
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

void DbusBluetoothClient::applyReply(const QByteArray &json)
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

// --- MockBluetoothClient ---------------------------------------------------

MockBluetoothClient::MockBluetoothClient(QObject *parent)
    : BluetoothClient(parent)
{
    rebuild();
}

void MockBluetoothClient::refresh()
{
    rebuild();
}

void MockBluetoothClient::setPowered(bool powered)
{
    m_powered = powered;
    if (!powered)
        m_discovering = false;
    rebuild();
}

void MockBluetoothClient::setDiscovering(bool discovering)
{
    m_discovering = discovering && m_powered;
    rebuild();
}

void MockBluetoothClient::pair(const QString &)
{
    rebuild();
}

void MockBluetoothClient::setConnected(const QString &address, bool connected)
{
    if (address == QStringLiteral("AA:BB:CC:DD:EE:FF"))
        m_headsetConnected = connected;
    rebuild();
}

void MockBluetoothClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("bluetooth"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("powered"), m_powered);
    view.insert(QStringLiteral("discovering"), m_discovering);
    view.insert(QStringLiteral("discoverable"), true);
    view.insert(QStringLiteral("pairable"), true);
    view.insert(QStringLiteral("adapterName"), QStringLiteral("workstation"));
    view.insert(QStringLiteral("glyph"),
                m_powered ? QStringLiteral("bluetooth")
                          : QStringLiteral("bluetooth-disabled"));
    view.insert(QStringLiteral("label"),
                m_discovering ? QStringLiteral("Bluetooth discovering")
                              : (m_powered ? QStringLiteral("Bluetooth on")
                                           : QStringLiteral("Bluetooth off")));

    const auto device = [](const QString &address, const QString &name, bool paired,
                           bool connected, int signal) {
        QVariantMap entry;
        entry.insert(QStringLiteral("address"), address);
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("paired"), paired);
        entry.insert(QStringLiteral("connected"), connected);
        entry.insert(QStringLiteral("signal"), signal);
        return entry;
    };

    QVariantList known;
    known.append(device(QStringLiteral("AA:BB:CC:DD:EE:FF"), QStringLiteral("WF-1000XM6"),
                        true, m_headsetConnected, 64));
    known.append(device(QStringLiteral("11:22:33:44:55:66"), QStringLiteral("WH-1000XM6"),
                        true, false, 48));
    view.insert(QStringLiteral("knownDevices"), known);

    QVariantList nearby;
    if (m_discovering)
        nearby.append(device(QStringLiteral("22:33:44:55:66:77"),
                             QStringLiteral("Nearby Speaker"), false, false, 30));
    view.insert(QStringLiteral("nearbyDevices"), nearby);

    view.insert(QStringLiteral("connectedCount"),
                m_headsetConnected ? 1 : 0);
    m_view = view;
    emit changed(m_view);
}