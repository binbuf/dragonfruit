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
const QString kStorageInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Storage");
const QString kInputInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Input");
const QString kUpdatesInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Updates");
const QString kAccountsInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Accounts");
const QString kPrintersInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Printers");
const QString kPrivacyInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Privacy");
const QString kAccessibilityInterface =
    QStringLiteral("org.dragonfruit.SystemStatus1.Accessibility");
const QString kVpnInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Vpn");

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

void DbusSystemStatusClient::refreshStorage()
{
    call(kStorageInterface, QStringLiteral("State"), {},
         &SystemStatusClient::storageState);
}

void DbusSystemStatusClient::mountStorage(const QString &volumePath)
{
    call(kStorageInterface, QStringLiteral("Mount"), {volumePath},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::unmountStorage(const QString &volumePath)
{
    call(kStorageInterface, QStringLiteral("Unmount"), {volumePath},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::ejectStorage(const QString &drivePath)
{
    call(kStorageInterface, QStringLiteral("Eject"), {drivePath},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::refreshInput()
{
    call(kInputInterface, QStringLiteral("State"), {},
         &SystemStatusClient::inputState);
}

void DbusSystemStatusClient::refreshUpdates()
{
    call(kUpdatesInterface, QStringLiteral("State"), {},
         &SystemStatusClient::updatesState);
}

void DbusSystemStatusClient::checkUpdates()
{
    call(kUpdatesInterface, QStringLiteral("Check"), {},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::installUpdates()
{
    call(kUpdatesInterface, QStringLiteral("Install"), {},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::rebootUpdates()
{
    call(kUpdatesInterface, QStringLiteral("Reboot"), {},
         &SystemStatusClient::writeReport);
}

void DbusSystemStatusClient::refreshAccounts()
{
    call(kAccountsInterface, QStringLiteral("State"), {},
         &SystemStatusClient::accountsState);
}

void DbusSystemStatusClient::refreshPrinters()
{
    call(kPrintersInterface, QStringLiteral("State"), {},
         &SystemStatusClient::printersState);
}

void DbusSystemStatusClient::refreshPrivacy()
{
    call(kPrivacyInterface, QStringLiteral("State"), {},
         &SystemStatusClient::privacyState);
}

void DbusSystemStatusClient::refreshAccessibility()
{
    call(kAccessibilityInterface, QStringLiteral("State"), {},
         &SystemStatusClient::accessibilityState);
}

void DbusSystemStatusClient::refreshVpn()
{
    call(kVpnInterface, QStringLiteral("State"), {},
         &SystemStatusClient::vpnState);
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
    refreshStorage();
    refreshInput();
    refreshUpdates();
    refreshAccounts();
    refreshPrinters();
    refreshPrivacy();
    refreshAccessibility();
    refreshVpn();
}

void MockSystemStatusClient::refreshInput()
{
    // A deterministic inventory: one keyboard, one mouse, one trackpad. The
    // adapter is read-only, so the fixture never mutates.
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("input"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("glyph"), QStringLiteral("keyboard"));
    view.insert(QStringLiteral("label"),
                QStringLiteral("1 keyboards, 2 pointing devices"));
    view.insert(QStringLiteral("deviceCount"), 3);
    view.insert(QStringLiteral("keyboardCount"), 1);
    view.insert(QStringLiteral("pointerCount"), 2);
    view.insert(QStringLiteral("mouseCount"), 1);
    view.insert(QStringLiteral("touchpadCount"), 1);

    const auto device = [](const QString &name, const QString &kernel,
                           const QString &kind, const QString &kindLabel) {
        QJsonObject entry;
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("kernel"), kernel);
        entry.insert(QStringLiteral("kind"), kind);
        entry.insert(QStringLiteral("kindLabel"), kindLabel);
        return entry;
    };
    QJsonArray devices;
    devices.append(device(QStringLiteral("AT Translated Set 2 keyboard"),
                          QStringLiteral("/dev/input/event3"),
                          QStringLiteral("keyboard"), QStringLiteral("Keyboard")));
    devices.append(device(QStringLiteral("Logitech USB Mouse"),
                          QStringLiteral("/dev/input/event9"),
                          QStringLiteral("mouse"), QStringLiteral("Mouse")));
    devices.append(device(QStringLiteral("Synaptics TouchPad"),
                          QStringLiteral("/dev/input/event7"),
                          QStringLiteral("touchpad"), QStringLiteral("Trackpad")));
    view.insert(QStringLiteral("devices"), devices);
    emit inputState(compact(view));
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

void MockSystemStatusClient::refreshStorage()
{
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("storage"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), m_storageDrivePresent);
    view.insert(QStringLiteral("glyph"), QStringLiteral("storage"));
    view.insert(QStringLiteral("label"),
                !m_storageDrivePresent ? QStringLiteral("Storage unavailable")
                : m_storageVolumeMounted ? QStringLiteral("Storage 1 mounted")
                                         : QStringLiteral("Storage"));

    const auto volume = [this](const QString &path, const QString &name,
                               const QString &filesystem, bool mounted,
                               const QString &mountPoint) {
        QJsonObject entry;
        entry.insert(QStringLiteral("path"), path);
        entry.insert(QStringLiteral("drivePath"),
                     QStringLiteral("/org/freedesktop/UDisks2/drives/usb"));
        entry.insert(QStringLiteral("device"), QStringLiteral("/dev/sdb1"));
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("label"), name);
        entry.insert(QStringLiteral("filesystem"), filesystem);
        entry.insert(QStringLiteral("mounted"), mounted);
        entry.insert(QStringLiteral("mountPoint"),
                     mounted ? QJsonValue(mountPoint) : QJsonValue(QJsonValue::Null));
        entry.insert(QStringLiteral("removable"), true);
        entry.insert(QStringLiteral("system"), false);
        entry.insert(QStringLiteral("ejectable"), true);
        entry.insert(QStringLiteral("readOnly"), false);
        entry.insert(QStringLiteral("glyph"), QStringLiteral("drive-removable-media"));
        return entry;
    };

    QJsonArray volumes;
    QJsonArray drives;
    if (m_storageDrivePresent) {
        QJsonObject drive;
        drive.insert(QStringLiteral("path"),
                     QStringLiteral("/org/freedesktop/UDisks2/drives/usb"));
        drive.insert(QStringLiteral("name"), QStringLiteral("Flash Drive"));
        drive.insert(QStringLiteral("removable"), true);
        drive.insert(QStringLiteral("ejectable"), true);
        drive.insert(QStringLiteral("glyph"), QStringLiteral("drive-removable-media"));
        drives.append(drive);
        volumes.append(volume(QStringLiteral("/org/freedesktop/UDisks2/block_devices/sdb1"),
                              QStringLiteral("Photos"), QStringLiteral("vfat"),
                              m_storageVolumeMounted,
                              QStringLiteral("/run/media/user/Photos")));
    }
    view.insert(QStringLiteral("drives"), drives);
    view.insert(QStringLiteral("volumes"), volumes);
    view.insert(QStringLiteral("mountedCount"), m_storageVolumeMounted ? 1 : 0);
    view.insert(QStringLiteral("volumeCount"), volumes.size());
    view.insert(QStringLiteral("removableCount"), volumes.size());
    emit storageState(compact(view));
}

void MockSystemStatusClient::mountStorage(const QString &)
{
    m_storageVolumeMounted = true;
    refreshStorage();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::unmountStorage(const QString &)
{
    m_storageVolumeMounted = false;
    refreshStorage();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::ejectStorage(const QString &)
{
    m_storageVolumeMounted = false;
    m_storageDrivePresent = false;
    refreshStorage();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("accepted"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::refreshUpdates()
{
    // The General/About/Updates fixture (T-15.10b): a Dragonfruit host identity
    // with one security update on offer. Writes mutate the phase and re-emit.
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("updates"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("hostName"), QStringLiteral("dragon"));
    view.insert(QStringLiteral("deviceName"), QStringLiteral("Dragonfruit Book"));
    view.insert(QStringLiteral("osLabel"), QStringLiteral("Dragonfruit Linux 44"));
    view.insert(QStringLiteral("kernel"), QStringLiteral("6.12.0"));
    view.insert(QStringLiteral("architecture"), QStringLiteral("x86_64"));
    view.insert(QStringLiteral("processor"), QStringLiteral("Example CPU"));
    view.insert(QStringLiteral("memoryLabel"), QStringLiteral("16 GB"));
    view.insert(QStringLiteral("serial"), QStringLiteral("SERIAL-1"));
    view.insert(QStringLiteral("hasSerial"), true);
    view.insert(QStringLiteral("updatesAvailable"), true);
    view.insert(QStringLiteral("glyph"), QStringLiteral("software-update"));
    view.insert(QStringLiteral("phase"), m_updatesPhase);
    view.insert(QStringLiteral("busy"), false);
    view.insert(QStringLiteral("rebootRequired"),
                m_updatesPhase == QStringLiteral("reboot-required"));
    view.insert(QStringLiteral("updateCount"), m_updatesCount);
    view.insert(QStringLiteral("securityCount"),
                m_updatesPhase == QStringLiteral("available") ? 1 : 0);
    view.insert(QStringLiteral("lastCheckedMs"), 1000);
    const QString label = m_updatesPhase == QStringLiteral("reboot-required")
        ? QStringLiteral("Restart Required")
        : m_updatesCount > 0 ? QStringLiteral("1 Update Available")
                             : QStringLiteral("Up to Date");
    view.insert(QStringLiteral("label"), label);

    QJsonArray updates;
    if (m_updatesCount > 0) {
        QJsonObject update;
        update.insert(QStringLiteral("id"), QStringLiteral("glibc"));
        update.insert(QStringLiteral("name"), QStringLiteral("glibc"));
        update.insert(QStringLiteral("summary"), QStringLiteral("C library"));
        update.insert(QStringLiteral("currentVersion"), QStringLiteral("2.40"));
        update.insert(QStringLiteral("availableVersion"), QStringLiteral("2.41"));
        update.insert(QStringLiteral("severity"), QStringLiteral("security"));
        update.insert(QStringLiteral("severityLabel"),
                      QStringLiteral("Security Update"));
        updates.append(update);
    }
    view.insert(QStringLiteral("updates"), updates);
    emit updatesState(compact(view));
}

void MockSystemStatusClient::checkUpdates()
{
    m_updatesPhase = m_updatesCount > 0 ? QStringLiteral("available")
                                        : QStringLiteral("up-to-date");
    refreshUpdates();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("applied"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::installUpdates()
{
    if (m_updatesCount > 0) {
        m_updatesCount = 0;
        m_updatesPhase = QStringLiteral("reboot-required");
    } else {
        m_updatesPhase = QStringLiteral("up-to-date");
    }
    refreshUpdates();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("applied"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::rebootUpdates()
{
    m_updatesPhase = QStringLiteral("up-to-date");
    refreshUpdates();
    QJsonObject report;
    report.insert(QStringLiteral("outcome"), QStringLiteral("applied"));
    emit writeReport(compact(report));
}

void MockSystemStatusClient::refreshAccounts()
{
    // The Users and Groups fixture (T-15.11b): a workstation with one admin
    // (Dan, the automatic login user) and one locked standard user (Sam), plus
    // one user group. The tile is a read-only summary, so the fixture never
    // mutates.
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("accounts"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("users"));
    view.insert(QStringLiteral("label"), QStringLiteral("2 Users"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("humanCount"), 2);
    view.insert(QStringLiteral("adminCount"), 1);
    view.insert(QStringLiteral("lockedCount"), 1);
    view.insert(QStringLiteral("groupsAvailable"), true);
    view.insert(QStringLiteral("groupCount"), 1);
    view.insert(QStringLiteral("automaticLogin"), QStringLiteral("Dan Doe"));
    view.insert(QStringLiteral("automaticLoginUser"), QStringLiteral("dan"));
    view.insert(QStringLiteral("automaticLoginUid"), 1000);

    const auto user = [](int uid, const QString &userName, const QString &realName,
                         const QString &accountType, const QString &accountTypeLabel,
                         const QString &initial, bool locked, bool automaticLogin,
                         bool system) {
        QJsonObject entry;
        entry.insert(QStringLiteral("uid"), uid);
        entry.insert(QStringLiteral("userName"), userName);
        entry.insert(QStringLiteral("realName"), realName);
        entry.insert(QStringLiteral("displayName"), realName);
        entry.insert(QStringLiteral("initial"), initial);
        entry.insert(QStringLiteral("accountType"), accountType);
        entry.insert(QStringLiteral("accountTypeLabel"), accountTypeLabel);
        entry.insert(QStringLiteral("locked"), locked);
        entry.insert(QStringLiteral("automaticLogin"), automaticLogin);
        entry.insert(QStringLiteral("system"), system);
        return entry;
    };
    QJsonArray users;
    users.append(user(1000, QStringLiteral("dan"), QStringLiteral("Dan Doe"),
                      QStringLiteral("administrator"), QStringLiteral("Admin"),
                      QStringLiteral("D"), false, true, false));
    users.append(user(1001, QStringLiteral("sam"), QStringLiteral("Sam Smith"),
                      QStringLiteral("standard"), QStringLiteral("Standard"),
                      QStringLiteral("S"), true, false, false));
    view.insert(QStringLiteral("users"), users);
    emit accountsState(compact(view));
}

void MockSystemStatusClient::refreshPrinters()
{
    // The Printers and Scanners fixture (T-15.12b): a workstation with one
    // idle default queue and one processing queue with a job, plus a scanner.
    // The tile is a read-only summary, so the fixture never mutates.
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("printers"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("printer"));
    view.insert(QStringLiteral("label"), QStringLiteral("2 Printers, 1 Scanner"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("printersAvailable"), true);
    view.insert(QStringLiteral("scannersAvailable"), true);
    view.insert(QStringLiteral("printerCount"), 2);
    view.insert(QStringLiteral("scannerCount"), 1);
    view.insert(QStringLiteral("queuedJobCount"), 2);
    view.insert(QStringLiteral("defaultPrinter"), QStringLiteral("Canon_MF230"));

    const auto printer = [](const QString &name, const QString &displayName,
                            const QString &state, const QString &stateLabel,
                            const QString &stateMessage, bool isDefault, int jobCount) {
        QJsonObject entry;
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("displayName"), displayName);
        entry.insert(QStringLiteral("makeAndModel"), QString());
        entry.insert(QStringLiteral("location"), QString());
        entry.insert(QStringLiteral("uri"), QString());
        entry.insert(QStringLiteral("state"), state);
        entry.insert(QStringLiteral("stateLabel"), stateLabel);
        entry.insert(QStringLiteral("stateMessage"), stateMessage);
        entry.insert(QStringLiteral("acceptingJobs"), true);
        entry.insert(QStringLiteral("enabled"), true);
        entry.insert(QStringLiteral("isDefault"), isDefault);
        entry.insert(QStringLiteral("jobCount"), jobCount);
        entry.insert(QStringLiteral("jobs"), QJsonArray());
        return entry;
    };
    QJsonArray printers;
    printers.append(printer(QStringLiteral("Canon_MF230"), QStringLiteral("Canon MF230"),
                            QStringLiteral("idle"), QStringLiteral("Idle"),
                            QStringLiteral("Idle, Last Used"), true, 0));
    printers.append(printer(QStringLiteral("HP_LaserJet"), QStringLiteral("HP LaserJet"),
                            QStringLiteral("processing"), QStringLiteral("Printing"),
                            QStringLiteral("Printing"), false, 1));
    view.insert(QStringLiteral("printers"), printers);

    QJsonObject scanner;
    scanner.insert(QStringLiteral("device"), QStringLiteral("epson2:net:192.168.0.7"));
    scanner.insert(QStringLiteral("description"),
                   QStringLiteral("Epson GT-1500 flatbed scanner"));
    scanner.insert(QStringLiteral("displayName"),
                   QStringLiteral("Epson GT-1500 flatbed scanner"));
    scanner.insert(QStringLiteral("kind"), QStringLiteral("flatbed"));
    scanner.insert(QStringLiteral("kindLabel"), QStringLiteral("Flatbed"));
    QJsonArray scanners;
    scanners.append(scanner);
    view.insert(QStringLiteral("scanners"), scanners);
    emit printersState(compact(view));
}

void MockSystemStatusClient::refreshPrivacy()
{
    // The Privacy and Security fixture (T-15.13b): a portal PermissionStore
    // with three application permissions across camera and location. The tile
    // is a read-only summary of the host stack, so the fixture never mutates.
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("privacy"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("privacy"));
    view.insert(QStringLiteral("label"), QStringLiteral("3 Apps"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("appCount"), 3);
    view.insert(QStringLiteral("grantedCount"), 1);
    view.insert(QStringLiteral("deniedCount"), 1);
    view.insert(QStringLiteral("categoryCount"), 14);

    const auto app = [](const QString &id, const QString &state, const QString &stateLabel,
                        const QStringList &permissions) {
        QJsonObject entry;
        entry.insert(QStringLiteral("app"), id);
        entry.insert(QStringLiteral("state"), state);
        entry.insert(QStringLiteral("stateLabel"), stateLabel);
        entry.insert(QStringLiteral("permissions"),
                     QJsonArray::fromStringList(permissions));
        return entry;
    };
    const auto category = [&app](const QString &id, const QString &label,
                                 const QString &summary, int appCount, int grantedCount,
                                 const QJsonArray &resources) {
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), id);
        entry.insert(QStringLiteral("label"), label);
        entry.insert(QStringLiteral("summary"), summary);
        entry.insert(QStringLiteral("appCount"), appCount);
        entry.insert(QStringLiteral("grantedCount"), grantedCount);
        entry.insert(QStringLiteral("resources"), resources);
        return entry;
    };

    QJsonObject cameraResource;
    cameraResource.insert(QStringLiteral("id"), QStringLiteral("camera"));
    cameraResource.insert(QStringLiteral("appCount"), 2);
    cameraResource.insert(QStringLiteral("apps"), QJsonArray {
        app(QStringLiteral("org.example.Snapshot"), QStringLiteral("denied"),
            QStringLiteral("Denied"), { QStringLiteral("no") }),
        app(QStringLiteral("org.mozilla.firefox"), QStringLiteral("allowed"),
            QStringLiteral("Allowed"), { QStringLiteral("yes") }),
    });
    QJsonObject locationResource;
    locationResource.insert(QStringLiteral("id"), QStringLiteral("location"));
    locationResource.insert(QStringLiteral("appCount"), 1);
    locationResource.insert(QStringLiteral("apps"), QJsonArray {
        app(QStringLiteral("org.example.Maps"), QStringLiteral("ask"),
            QStringLiteral("Ask"), { QStringLiteral("ask") }),
    });

    QJsonArray categories;
    categories.append(category(QStringLiteral("devices"), QStringLiteral("Camera"),
                               QStringLiteral("2 apps"), 2, 1,
                               QJsonArray { cameraResource }));
    categories.append(category(QStringLiteral("location"), QStringLiteral("Location Services"),
                               QStringLiteral("1 app"), 1, 0,
                               QJsonArray { locationResource }));
    categories.append(category(QStringLiteral("notifications"), QStringLiteral("Notifications"),
                               QStringLiteral("None"), 0, 0, QJsonArray {}));
    categories.append(category(QStringLiteral("screencast"), QStringLiteral("Screen Recording"),
                               QStringLiteral("None"), 0, 0, QJsonArray {}));
    view.insert(QStringLiteral("categories"), categories);
    emit privacyState(compact(view));
}

void MockSystemStatusClient::refreshAccessibility()
{
    // The Accessibility fixture (T-15.14b): a deterministic AT-SPI bus with the
    // toolkit bridge and a screen reader on. The adapter is read-only, so the
    // fixture never mutates; the tile is a live summary.
    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("accessibility"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("accessibility"));
    view.insert(QStringLiteral("label"), QStringLiteral("Screen Reader On"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("enabled"), true);
    view.insert(QStringLiteral("enabledLabel"), QStringLiteral("On"));
    view.insert(QStringLiteral("screenReader"), true);
    view.insert(QStringLiteral("screenReaderLabel"), QStringLiteral("On"));
    emit accessibilityState(compact(view));
}

void MockSystemStatusClient::refreshVpn()
{
    // The Network advanced (VPN) fixture (T-15.15b): a deterministic
    // NetworkManager with one OpenVPN connection up and one WireGuard idle. The
    // tile is a read-only summary; the connect/deactivate writes live in the
    // Settings pane, so the fixture never mutates.
    QJsonObject work;
    work.insert(QStringLiteral("id"), QStringLiteral("Work VPN"));
    work.insert(QStringLiteral("uuid"), QStringLiteral("11111111-1111-1111-1111-111111111111"));
    work.insert(QStringLiteral("kind"), QStringLiteral("openvpn"));
    work.insert(QStringLiteral("kindLabel"), QStringLiteral("OpenVPN"));
    work.insert(QStringLiteral("state"), QStringLiteral("connected"));
    work.insert(QStringLiteral("stateLabel"), QStringLiteral("Connected"));
    work.insert(QStringLiteral("connected"), true);
    work.insert(QStringLiteral("autoconnect"), true);
    work.insert(QStringLiteral("label"), QStringLiteral("OpenVPN \u00b7 Connected"));

    QJsonObject home;
    home.insert(QStringLiteral("id"), QStringLiteral("Home"));
    home.insert(QStringLiteral("uuid"), QStringLiteral("22222222-2222-2222-2222-222222222222"));
    home.insert(QStringLiteral("kind"), QStringLiteral("wireguard"));
    home.insert(QStringLiteral("kindLabel"), QStringLiteral("WireGuard"));
    home.insert(QStringLiteral("state"), QStringLiteral("disconnected"));
    home.insert(QStringLiteral("stateLabel"), QStringLiteral("Disconnected"));
    home.insert(QStringLiteral("connected"), false);
    home.insert(QStringLiteral("autoconnect"), false);
    home.insert(QStringLiteral("label"), QStringLiteral("WireGuard \u00b7 Disconnected"));

    QJsonObject view;
    view.insert(QStringLiteral("kind"), QStringLiteral("vpn"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("vpn"));
    view.insert(QStringLiteral("label"), QStringLiteral("Work VPN"));
    view.insert(QStringLiteral("present"), true);
    view.insert(QStringLiteral("connectionCount"), 2);
    view.insert(QStringLiteral("connectedCount"), 1);
    view.insert(QStringLiteral("activeUuid"),
                QStringLiteral("11111111-1111-1111-1111-111111111111"));
    view.insert(QStringLiteral("activeName"), QStringLiteral("Work VPN"));
    view.insert(QStringLiteral("readOnly"), false);
    view.insert(QStringLiteral("note"), QString());
    view.insert(QStringLiteral("connections"),
                QJsonArray { work, home });
    emit vpnState(compact(view));
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
    view.insert(QStringLiteral("defaultSink"), m_defaultSinkName);
    view.insert(QStringLiteral("defaultSource"), m_defaultSourceName);
    view.insert(QStringLiteral("sinkCount"), 2);
    view.insert(QStringLiteral("sourceCount"), 2);

    const auto device = [](int id, const QString &name, const QString &description,
                           double volume, bool muted, bool isDefault) {
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), id);
        entry.insert(QStringLiteral("name"), name);
        entry.insert(QStringLiteral("description"), description);
        entry.insert(QStringLiteral("volume"), volume);
        entry.insert(QStringLiteral("percent"), qRound(volume * 100.0));
        entry.insert(QStringLiteral("muted"), muted);
        entry.insert(QStringLiteral("default"), isDefault);
        return entry;
    };
    QJsonArray sinks;
    sinks.append(device(7, QStringLiteral("speakers"), QStringLiteral("Built-in Speakers"),
                        m_volume, m_muted,
                        m_defaultSinkName == QStringLiteral("speakers")));
    sinks.append(device(9, QStringLiteral("headphones"), QStringLiteral("Headphones"),
                        m_volume, m_muted,
                        m_defaultSinkName == QStringLiteral("headphones")));
    view.insert(QStringLiteral("sinks"), sinks);

    QJsonArray sources;
    sources.append(device(20, QStringLiteral("microphone"),
                          QStringLiteral("Built-in Microphone"), 0.5, false,
                          m_defaultSourceName == QStringLiteral("microphone")));
    sources.append(device(21, QStringLiteral("usb-mic"), QStringLiteral("USB Microphone"),
                          0.5, false,
                          m_defaultSourceName == QStringLiteral("usb-mic")));
    view.insert(QStringLiteral("sources"), sources);
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
    view.insert(QStringLiteral("chargeState"), QStringLiteral("charging"));
    view.insert(QStringLiteral("health"), QStringLiteral("normal"));
    view.insert(QStringLiteral("healthLabel"), QStringLiteral("Normal"));
    view.insert(QStringLiteral("capacity"), 96);
    view.insert(QStringLiteral("chargeCycles"), 112);
    view.insert(QStringLiteral("timeToEmpty"), QJsonValue::Null);
    view.insert(QStringLiteral("timeToFull"), 5400);
    // T-15.6b: the power-profiles half, so the Control Center tile renders the
    // active profile without a power-profiles-daemon on the host.
    view.insert(QStringLiteral("profilesAvailable"), true);
    view.insert(QStringLiteral("activeProfile"), QStringLiteral("balanced"));
    view.insert(QStringLiteral("profileLabel"), QStringLiteral("Balanced"));
    QJsonArray profiles;
    const auto profile = [](const QString &id, const QString &label, const QString &glyph,
                            bool active) {
        QJsonObject entry;
        entry.insert(QStringLiteral("id"), id);
        entry.insert(QStringLiteral("label"), label);
        entry.insert(QStringLiteral("glyph"), glyph);
        entry.insert(QStringLiteral("active"), active);
        return entry;
    };
    profiles.append(profile(QStringLiteral("power-saver"), QStringLiteral("Power Saver"),
                            QStringLiteral("power-saver"), false));
    profiles.append(profile(QStringLiteral("balanced"), QStringLiteral("Balanced"),
                            QStringLiteral("power-balanced"), true));
    profiles.append(profile(QStringLiteral("performance"), QStringLiteral("Performance"),
                            QStringLiteral("power-performance"), false));
    view.insert(QStringLiteral("profiles"), profiles);
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
