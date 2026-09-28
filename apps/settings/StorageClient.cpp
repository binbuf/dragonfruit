// SPDX-License-Identifier: MIT
#include "StorageClient.h"

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
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Storage");

} // namespace

// --- DbusStorageClient -----------------------------------------------------

DbusStorageClient::DbusStorageClient(QObject *parent)
    : StorageClient(parent)
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

DbusStorageClient::~DbusStorageClient() = default;

bool DbusStorageClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusStorageClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusStorageClient::mount(const QString &volumePath)
{
    call(QStringLiteral("Mount"), {volumePath});
}

void DbusStorageClient::unmount(const QString &volumePath)
{
    call(QStringLiteral("Unmount"), {volumePath});
}

void DbusStorageClient::eject(const QString &drivePath)
{
    call(QStringLiteral("Eject"), {drivePath});
}

void DbusStorageClient::call(const QString &method, const QVariantList &arguments)
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

void DbusStorageClient::applyReply(const QByteArray &json)
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

// --- MockStorageClient -----------------------------------------------------

MockStorageClient::MockStorageClient(QObject *parent)
    : StorageClient(parent)
{
    rebuild();
}

void MockStorageClient::refresh()
{
    rebuild();
}

void MockStorageClient::mount(const QString &)
{
    m_volumeMounted = true;
    rebuild();
}

void MockStorageClient::unmount(const QString &)
{
    m_volumeMounted = false;
    rebuild();
}

void MockStorageClient::eject(const QString &)
{
    m_volumeMounted = false;
    m_drivePresent = false;
    rebuild();
}

void MockStorageClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("storage"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), m_drivePresent);
    view.insert(QStringLiteral("glyph"), QStringLiteral("storage"));
    view.insert(QStringLiteral("label"),
                !m_drivePresent ? QStringLiteral("Storage unavailable")
                : m_volumeMounted ? QStringLiteral("Storage 1 mounted")
                                  : QStringLiteral("Storage"));

    QVariantList drives;
    QVariantList volumes;
    if (m_drivePresent) {
        QVariantMap drive;
        drive.insert(QStringLiteral("path"),
                     QStringLiteral("/org/freedesktop/UDisks2/drives/usb"));
        drive.insert(QStringLiteral("name"), QStringLiteral("Flash Drive"));
        drive.insert(QStringLiteral("removable"), true);
        drive.insert(QStringLiteral("ejectable"), true);
        drives.append(drive);

        QVariantMap volume;
        volume.insert(QStringLiteral("path"),
                      QStringLiteral("/org/freedesktop/UDisks2/block_devices/sdb1"));
        volume.insert(QStringLiteral("drivePath"),
                      QStringLiteral("/org/freedesktop/UDisks2/drives/usb"));
        volume.insert(QStringLiteral("device"), QStringLiteral("/dev/sdb1"));
        volume.insert(QStringLiteral("name"), QStringLiteral("Photos"));
        volume.insert(QStringLiteral("label"), QStringLiteral("Photos"));
        volume.insert(QStringLiteral("filesystem"), QStringLiteral("vfat"));
        volume.insert(QStringLiteral("mounted"), m_volumeMounted);
        volume.insert(QStringLiteral("mountPoint"),
                      m_volumeMounted
                          ? QVariant(QStringLiteral("/run/media/user/Photos"))
                          : QVariant());
        volume.insert(QStringLiteral("removable"), true);
        volume.insert(QStringLiteral("system"), false);
        volume.insert(QStringLiteral("ejectable"), true);
        volume.insert(QStringLiteral("readOnly"), false);
        volumes.append(volume);
    }
    view.insert(QStringLiteral("drives"), drives);
    view.insert(QStringLiteral("volumes"), volumes);
    view.insert(QStringLiteral("mountedCount"), m_volumeMounted ? 1 : 0);
    view.insert(QStringLiteral("volumeCount"), volumes.size());
    view.insert(QStringLiteral("removableCount"), volumes.size());
    m_view = view;
    emit changed(m_view);
}