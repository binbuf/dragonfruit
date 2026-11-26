// SPDX-License-Identifier: GPL-3.0-only
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
    call(QStringLiteral("Refresh"), {});
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
    const bool isState = method == QStringLiteral("State")
        || method == QStringLiteral("Refresh");
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, method, isState, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid()) {
                    if (!isState)
                        emit writeFinished(method, QStringLiteral("failed"),
                                           reply.error().message());
                    return;
                }
                if (isState) {
                    applyReply(reply.value().toUtf8());
                    return;
                }
                // T282: the write reply is the outcome report; the new state
                // is the host's re-read, never invented here.
                QString outcome = QStringLiteral("applied");
                QString message;
                const QJsonDocument document =
                    QJsonDocument::fromJson(reply.value().toUtf8());
                if (document.isObject()) {
                    const QJsonObject object = document.object();
                    outcome = object.value(QStringLiteral("outcome"))
                                  .toString(outcome);
                    message = object.value(QStringLiteral("error")).toString();
                    if (message.isEmpty())
                        message = object.value(QStringLiteral("note")).toString();
                }
                emit writeFinished(method, outcome, message);
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

bool MockStorageClient::available() const
{
    return m_mode != QStringLiteral("absent");
}

void MockStorageClient::refresh()
{
    rebuild();
}

void MockStorageClient::setFixture(const QString &mode)
{
    m_mode = mode == QStringLiteral("absent") || mode == QStringLiteral("empty")
        ? mode : QStringLiteral("live");
    m_volumeMounted = false;
    m_drivePresent = m_mode == QStringLiteral("live");
    rebuild();
}

void MockStorageClient::resetForTest()
{
    m_mode = QStringLiteral("live");
    m_drivePresent = true;
    m_volumeMounted = false;
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
    if (m_mode == QStringLiteral("absent")) {
        QVariantMap absent;
        absent.insert(QStringLiteral("kind"), QStringLiteral("storage"));
        absent.insert(QStringLiteral("state"), QStringLiteral("unavailable"));
        absent.insert(QStringLiteral("present"), false);
        absent.insert(QStringLiteral("volumes"), QVariantList());
        absent.insert(QStringLiteral("drives"), QVariantList());
        absent.insert(QStringLiteral("mountedCount"), 0);
        m_view = absent;
        emit changed(m_view);
        return;
    }
    const bool hasDrive = m_mode == QStringLiteral("live") && m_drivePresent;
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("storage"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("present"), m_mode == QStringLiteral("empty")
                                             ? true : m_drivePresent);
    view.insert(QStringLiteral("glyph"), QStringLiteral("storage"));
    view.insert(QStringLiteral("label"),
                !view.value(QStringLiteral("present")).toBool()
                    ? QStringLiteral("Storage unavailable")
                : m_volumeMounted ? QStringLiteral("Storage 1 mounted")
                                  : QStringLiteral("Storage"));

    QVariantList drives;
    QVariantList volumes;
    if (hasDrive) {
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