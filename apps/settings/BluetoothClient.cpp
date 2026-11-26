// SPDX-License-Identifier: GPL-3.0-only
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
    call(QStringLiteral("Refresh"), {}, QString(), true);
}

void DbusBluetoothClient::setPowered(bool powered)
{
    call(QStringLiteral("SetPowered"), {powered}, QStringLiteral("power"));
}

void DbusBluetoothClient::setDiscovering(bool discovering)
{
    call(QStringLiteral("SetDiscovering"), {discovering},
         QStringLiteral("discovery"));
}

void DbusBluetoothClient::pair(const QString &address)
{
    call(QStringLiteral("Pair"), {address}, QStringLiteral("pair"));
}

void DbusBluetoothClient::setConnected(const QString &address, bool connected)
{
    call(QStringLiteral("SetConnected"), {address, connected},
         QStringLiteral("connect"));
}

void DbusBluetoothClient::forget(const QString &address)
{
    call(QStringLiteral("Forget"), {address}, QStringLiteral("forget"));
}

void DbusBluetoothClient::call(const QString &method, const QVariantList &arguments,
                               const QString &operation, bool isState)
{
    auto *iface = new QDBusInterface(m_service, m_path, m_interface,
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
    if (qEnvironmentVariable("DF_BLUETOOTH_FIXTURE") == QLatin1String("absent")) {
        m_absent = true;
        return;
    }
    resetForTest();
    if (qEnvironmentVariable("DF_BLUETOOTH_FIXTURE") == QLatin1String("off"))
        setPowered(false);
}

void MockBluetoothClient::refresh()
{
    rebuild();
}

void MockBluetoothClient::finishWrite(const QString &operation,
                                      const QString &outcome,
                                      const QString &message)
{
    emit writeFinished(operation, outcome, message);
}

bool MockBluetoothClient::failWrite(const QString &operation)
{
    if (m_failOperation != operation)
        return false;
    const QString message = m_failMessage;
    m_failOperation.clear();
    m_failMessage.clear();
    finishWrite(operation, QStringLiteral("failed"), message);
    return true;
}

void MockBluetoothClient::failNextWrite(const QString &operation,
                                        const QString &message)
{
    m_failOperation = operation;
    m_failMessage = message;
}

void MockBluetoothClient::resetForTest()
{
    m_absent = false;
    m_error = false;
    m_powered = true;
    m_discovering = false;
    m_headsetConnected = false;
    m_forgotten.clear();
    m_failOperation.clear();
    m_failMessage.clear();
    rebuild();
}

void MockBluetoothClient::setFixture(const QString &mode)
{
    if (mode == QLatin1String("absent")) {
        m_absent = true;
        m_view.clear();
        emit availableChanged(false);
        emit changed(m_view);
        return;
    }
    resetForTest();
    if (mode == QLatin1String("off"))
        setPowered(false);
    else if (mode == QLatin1String("empty"))
        m_forgotten = { QStringLiteral("AA:BB:CC:DD:EE:FF"),
                        QStringLiteral("11:22:33:44:55:66") };
    else if (mode == QLatin1String("error"))
        m_error = true;
    rebuild();
    emit availableChanged(true);
}

void MockBluetoothClient::setPowered(bool powered)
{
    if (failWrite(QStringLiteral("power")))
        return;
    m_powered = powered;
    if (!powered)
        m_discovering = false;
    rebuild();
    finishWrite(QStringLiteral("power"), QStringLiteral("applied"));
}

void MockBluetoothClient::setDiscovering(bool discovering)
{
    if (failWrite(QStringLiteral("discovery")))
        return;
    m_discovering = discovering && m_powered;
    rebuild();
    finishWrite(QStringLiteral("discovery"), QStringLiteral("applied"));
}

void MockBluetoothClient::pair(const QString &address)
{
    if (failWrite(QStringLiteral("pair")))
        return;
    m_forgotten.removeAll(address);
    rebuild();
    finishWrite(QStringLiteral("pair"), QStringLiteral("applied"));
}

void MockBluetoothClient::setConnected(const QString &address, bool connected)
{
    if (failWrite(QStringLiteral("connect")))
        return;
    if (address == QStringLiteral("AA:BB:CC:DD:EE:FF"))
        m_headsetConnected = connected;
    rebuild();
    finishWrite(QStringLiteral("connect"), QStringLiteral("applied"));
}

void MockBluetoothClient::forget(const QString &address)
{
    if (failWrite(QStringLiteral("forget")))
        return;
    if (!m_forgotten.contains(address))
        m_forgotten.append(address);
    if (address == QStringLiteral("AA:BB:CC:DD:EE:FF"))
        m_headsetConnected = false;
    rebuild();
    finishWrite(QStringLiteral("forget"), QStringLiteral("applied"));
}

void MockBluetoothClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("bluetooth"));
    if (m_absent) {
        m_view = view;
        emit changed(m_view);
        return;
    }
    view.insert(QStringLiteral("state"),
                m_error ? QStringLiteral("error") : QStringLiteral("available"));
    if (m_error)
        view.insert(QStringLiteral("error"), QStringLiteral("BlueZ: the read failed."));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("powered"), m_powered);
    view.insert(QStringLiteral("discovering"), m_discovering);
    view.insert(QStringLiteral("discoverable"), true);
    view.insert(QStringLiteral("pairable"), true);
    view.insert(QStringLiteral("adapterName"), QStringLiteral("workstation"));
    view.insert(QStringLiteral("adapterAddress"),
                QStringLiteral("11:22:33:44:55:66"));
    view.insert(QStringLiteral("computerName"), QStringLiteral("workstation"));
    view.insert(QStringLiteral("glyph"),
                m_powered ? QStringLiteral("bluetooth")
                          : QStringLiteral("bluetooth-disabled"));
    view.insert(QStringLiteral("label"),
                m_discovering ? QStringLiteral("Bluetooth discovering")
                              : (m_powered ? QStringLiteral("Bluetooth on")
                                           : QStringLiteral("Bluetooth off")));

    const auto device = [](const QString &address, const QString &name,
                           const QString &type, bool paired, bool connected,
                           int signal, int battery) {
        QVariantMap entry;
        entry.insert(QStringLiteral("address"), address);
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("type"), type);
        entry.insert(QStringLiteral("paired"), paired);
        entry.insert(QStringLiteral("connected"), connected);
        entry.insert(QStringLiteral("signal"), signal);
        entry.insert(QStringLiteral("battery"), battery);
        return entry;
    };

    QVariantList known;
    if (!m_forgotten.contains(QStringLiteral("AA:BB:CC:DD:EE:FF")))
        known.append(device(QStringLiteral("AA:BB:CC:DD:EE:FF"),
                            QStringLiteral("WF-1000XM6"), QStringLiteral("Audio"),
                            true, m_headsetConnected, 64, 82));
    if (!m_forgotten.contains(QStringLiteral("11:22:33:44:55:66")))
        known.append(device(QStringLiteral("11:22:33:44:55:66"),
                            QStringLiteral("WH-1000XM6"), QStringLiteral("Audio"),
                            true, false, 48, 61));
    view.insert(QStringLiteral("knownDevices"), known);

    QVariantList nearby;
    if (m_discovering && !m_forgotten.contains(QStringLiteral("22:33:44:55:66:77")))
        nearby.append(device(QStringLiteral("22:33:44:55:66:77"),
                             QStringLiteral("Nearby Speaker"),
                             QStringLiteral("Audio"), false, false, 30, -1));
    view.insert(QStringLiteral("nearbyDevices"), nearby);

    view.insert(QStringLiteral("connectedCount"),
                m_headsetConnected ? 1 : 0);
    m_view = view;
    emit changed(m_view);
}
