// SPDX-License-Identifier: MIT
#include "notificationclient.h"

#include <QDateTime>
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

// The standard freedesktop name/path; the shell-facing interface lives at the
// same object path (see services/notifications/src/dbus.rs).
const QString kService = QStringLiteral("org.freedesktop.Notifications");
const QString kPath = QStringLiteral("/org/freedesktop/Notifications");
const QString kInterface = QStringLiteral("org.dragonfruit.Notifications1");

} // namespace

// --- DbusNotificationClient ------------------------------------------------

DbusNotificationClient::DbusNotificationClient(QObject *parent)
    : NotificationClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
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
            if (available)
                refresh();
        };
        connect(iface, &QDBusConnectionInterface::serviceRegistered, this, update);
        connect(iface, &QDBusConnectionInterface::serviceUnregistered, this, update);
    }
    // The service's only notification; a re-read is the reaction, never a poll.
    bus.connect(m_service, m_path, m_interface, QStringLiteral("Changed"), this,
                SLOT(refresh()));
    m_available = isAvailable();
#endif
}

bool DbusNotificationClient::isAvailable() const
{
#if defined(QT_DBUS_LIB)
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
#else
    return false;
#endif
}

void DbusNotificationClient::call(const QString &method, const QVariantList &arguments,
                                  ReplySignal replySignal)
{
#if defined(QT_DBUS_LIB)
    auto *call = new QDBusInterface(m_service, m_path, m_interface,
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
                const QDBusPendingReply<QString> reply = *watcher;
                if (reply.isValid()) {
                    (this->*replySignal)(reply.value().toUtf8());
                } else {
                    qWarning() << "dragonfruit-shell: notification call failed:"
                               << watcher->reply().errorMessage();
                }
                call->deleteLater();
                watcher->deleteLater();
            });
#else
    Q_UNUSED(method);
    Q_UNUSED(arguments);
    Q_UNUSED(replySignal);
#endif
}

void DbusNotificationClient::refresh()
{
    call(QStringLiteral("Banners"), {}, &NotificationClient::bannersChanged);
    call(QStringLiteral("History"), {}, &NotificationClient::historyChanged);
}

void DbusNotificationClient::dismiss(quint32 id)
{
#if defined(QT_DBUS_LIB)
    // The service emits `Changed` after the close, which re-reads the views.
    QDBusInterface iface(m_service, m_path, m_interface, QDBusConnection::sessionBus());
    if (iface.isValid())
        iface.asyncCall(QStringLiteral("Dismiss"), id);
#else
    Q_UNUSED(id);
#endif
}

void DbusNotificationClient::expire(quint32 id)
{
#if defined(QT_DBUS_LIB)
    QDBusInterface iface(m_service, m_path, m_interface, QDBusConnection::sessionBus());
    if (iface.isValid())
        iface.asyncCall(QStringLiteral("Expire"), id);
#else
    Q_UNUSED(id);
#endif
}

void DbusNotificationClient::setDoNotDisturb(bool enabled)
{
#if defined(QT_DBUS_LIB)
    QDBusInterface iface(m_service, m_path, m_interface, QDBusConnection::sessionBus());
    if (iface.isValid())
        iface.asyncCall(QStringLiteral("SetDoNotDisturb"), enabled);
#else
    Q_UNUSED(enabled);
#endif
}

// --- MockNotificationClient ------------------------------------------------

MockNotificationClient::MockNotificationClient(QObject *parent)
    : NotificationClient(parent)
{
    // One representative banner with a long deadline so a capture is
    // deterministic; it still expires through the same path the live service
    // uses.
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    const quint32 id = m_nextId++;
    m_bannerId = id;
    m_bannerJson = QJsonDocument(QJsonArray{ QJsonObject{
                        { QStringLiteral("id"), static_cast<double>(id) },
                        { QStringLiteral("appName"), QStringLiteral("Mail") },
                        { QStringLiteral("appIcon"), QString() },
                        { QStringLiteral("summary"), QStringLiteral("New message") },
                        { QStringLiteral("body"),
                          QStringLiteral("Ada Lovelace — Notes on the Analytical Engine") },
                        { QStringLiteral("urgency"), QStringLiteral("normal") },
                        { QStringLiteral("createdAt"), static_cast<double>(now) },
                        { QStringLiteral("deadline"), static_cast<double>(now + 60000) },
                        { QStringLiteral("actions"), QJsonArray{} },
                    } })
                        .toJson(QJsonDocument::Compact);
    publish();

    m_expiry.setSingleShot(true);
    m_expiry.setInterval(60000);
    connect(&m_expiry, &QTimer::timeout, this, [this]() { expire(m_bannerId); });
    m_expiry.start();
}

void MockNotificationClient::refresh()
{
    emit bannersChanged(m_bannerJson);
    emit historyChanged(m_historyJson);
}

void MockNotificationClient::remove(quint32 id, const QString &reason)
{
    if (id != m_bannerId)
        return;
    // Re-encode the banner as a closed history entry (reason set).
    QJsonArray banners = QJsonDocument::fromJson(m_bannerJson).array();
    if (banners.isEmpty())
        return;
    QJsonObject entry = banners.first().toObject();
    entry[QStringLiteral("closedAt")] = static_cast<double>(QDateTime::currentMSecsSinceEpoch());
    entry[QStringLiteral("reason")] = reason;
    QJsonArray history = QJsonDocument::fromJson(m_historyJson).array();
    history.prepend(entry);
    m_historyJson = QJsonDocument(history).toJson(QJsonDocument::Compact);
    m_bannerJson = QJsonDocument(QJsonArray{}).toJson(QJsonDocument::Compact);
    m_bannerId = 0;
    m_expiry.stop();
    refresh();
}

void MockNotificationClient::publish()
{
    const QJsonArray banners = QJsonDocument::fromJson(m_bannerJson).array();
    QJsonArray history = QJsonDocument::fromJson(m_historyJson).array();
    if (history.isEmpty() && !banners.isEmpty()) {
        history.prepend(banners.first().toObject());
        m_historyJson = QJsonDocument(history).toJson(QJsonDocument::Compact);
    }
}

void MockNotificationClient::dismiss(quint32 id)
{
    remove(id, QStringLiteral("dismissed"));
}

void MockNotificationClient::expire(quint32 id)
{
    remove(id, QStringLiteral("expired"));
}

void MockNotificationClient::setDoNotDisturb(bool)
{
    // The fixture always shows its one banner; DND is T-11.2a.
}