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
    call(QStringLiteral("FocusPolicy"), {}, &NotificationClient::focusPolicyChanged);
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

void DbusNotificationClient::notify(const QString &appName, const QString &summary,
                                    const QString &body, const QString &urgency,
                                    const QStringList &actionKeys,
                                    const QStringList &actionLabels)
{
#if defined(QT_DBUS_LIB)
    // Raise the notification as an app would, through the standard
    // freedesktop interface at the same path.
    QDBusInterface iface(kService, kPath, kService, QDBusConnection::sessionBus());
    if (!iface.isValid()) {
        m_available = false;
        emit availableChanged(false);
        return;
    }
    QStringList flatActions;
    for (int i = 0; i < actionKeys.size(); ++i) {
        flatActions << actionKeys.at(i)
                    << (i < actionLabels.size() ? actionLabels.at(i) : actionKeys.at(i));
    }
    QVariantMap hints;
    if (urgency == QLatin1String("critical"))
        hints.insert(QStringLiteral("urgency"), QVariant::fromValue(2u));
    else if (urgency == QLatin1String("low"))
        hints.insert(QStringLiteral("urgency"), QVariant::fromValue(0u));
    QDBusPendingCallWatcher *watcher = new QDBusPendingCallWatcher(
        iface.asyncCall(QStringLiteral("Notify"), appName, 0u, QString(), summary, body,
                        flatActions, hints, -1),
        this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this, [this, watcher]() {
        const QDBusPendingReply<quint32> reply = *watcher;
        if (reply.isValid())
            emit notified(reply.value());
        else
            qWarning() << "dragonfruit-shell: notification Notify failed:"
                       << watcher->reply().errorMessage();
        watcher->deleteLater();
    });
#else
    Q_UNUSED(appName);
    Q_UNUSED(summary);
    Q_UNUSED(body);
    Q_UNUSED(urgency);
    Q_UNUSED(actionKeys);
    Q_UNUSED(actionLabels);
#endif
}

void DbusNotificationClient::invoke(quint32 id, const QString &actionKey)
{
#if defined(QT_DBUS_LIB)
    // The service emits `ActionInvoked` to the originating app and dismisses
    // the banner; `Changed` then re-reads the views.
    QDBusInterface iface(m_service, m_path, m_interface, QDBusConnection::sessionBus());
    if (iface.isValid())
        iface.asyncCall(QStringLiteral("Invoke"), id, actionKey);
#else
    Q_UNUSED(id);
    Q_UNUSED(actionKey);
#endif
}

void DbusNotificationClient::setFocusMode(const QString &mode)
{
#if defined(QT_DBUS_LIB)
    // `SetFocusMode` rejects unknown names; the service emits `Changed` on a
    // change, which re-reads `FocusPolicy()`.
    QDBusInterface iface(m_service, m_path, m_interface, QDBusConnection::sessionBus());
    if (iface.isValid())
        iface.asyncCall(QStringLiteral("SetFocusMode"), mode);
#else
    Q_UNUSED(mode);
#endif
}

// --- MockNotificationClient ------------------------------------------------

MockNotificationClient::MockNotificationClient(QObject *parent, bool seedFixture)
    : NotificationClient(parent)
{
    m_expiry.setSingleShot(true);
    connect(&m_expiry, &QTimer::timeout, this, [this]() { expire(m_bannerId); });
    if (seedFixture)
        seedBanner();
    else
        m_bannerJson = QByteArrayLiteral("[]");
}

void MockNotificationClient::seedBanner()
{
    // One representative banner with a long deadline so a capture is
    // deterministic; it still expires through the same path the live service
    // uses.
    notify(QStringLiteral("Mail"), QStringLiteral("New message"),
           QStringLiteral("Ada Lovelace — Notes on the Analytical Engine"),
           QStringLiteral("normal"), {}, {});
    m_expiry.stop();
    m_expiry.setInterval(60000);
    if (m_bannerId != 0)
        m_expiry.start();
}

void MockNotificationClient::refresh()
{
    emit bannersChanged(m_bannerJson);
    emit historyChanged(m_historyJson);
    emit focusPolicyChanged(policyJson());
}

QByteArray MockNotificationClient::policyJson() const
{
    return QJsonDocument(
               QJsonObject{
                   { QStringLiteral("mode"), m_focusMode },
                   { QStringLiteral("allowList"), QJsonArray{} },
                   { QStringLiteral("batchedCount"), m_focusBatched },
               })
        .toJson(QJsonDocument::Compact);
}

void MockNotificationClient::notify(const QString &appName, const QString &summary,
                                    const QString &body, const QString &urgency,
                                    const QStringList &actionKeys,
                                    const QStringList &actionLabels)
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    const quint32 id = m_nextId++;
    QJsonArray actions;
    for (int i = 0; i < actionKeys.size(); ++i) {
        actions.append(QJsonObject{
            { QStringLiteral("key"), actionKeys.at(i) },
            { QStringLiteral("label"),
              i < actionLabels.size() ? actionLabels.at(i) : actionKeys.at(i) } });
    }
    const QJsonObject banner{
        { QStringLiteral("id"), static_cast<double>(id) },
        { QStringLiteral("appName"), appName },
        { QStringLiteral("appIcon"), QString() },
        { QStringLiteral("summary"), summary },
        { QStringLiteral("body"), body },
        { QStringLiteral("urgency"),
          urgency.isEmpty() ? QStringLiteral("normal") : urgency },
        { QStringLiteral("createdAt"), static_cast<double>(now) },
        { QStringLiteral("deadline"), static_cast<double>(now + 5000) },
        { QStringLiteral("actions"), actions },
    };
    m_bannerId = id;
    m_bannerJson = QJsonDocument(QJsonArray{ banner }).toJson(QJsonDocument::Compact);

    // The history records the open entry immediately, exactly like the
    // service (reason/closedAt null until it leaves the queue).
    QJsonObject entry = banner;
    entry.insert(QStringLiteral("closedAt"), QJsonValue::Null);
    entry.insert(QStringLiteral("reason"), QJsonValue::Null);
    QJsonArray history = QJsonDocument::fromJson(m_historyJson).array();
    history.prepend(entry);
    m_historyJson = QJsonDocument(history).toJson(QJsonDocument::Compact);

    m_expiry.stop();
    m_expiry.setInterval(5000);
    m_expiry.start();
    refresh();
    emit notified(id);
}

void MockNotificationClient::remove(quint32 id, const QString &reason)
{
    if (id != m_bannerId)
        return;
    QJsonArray history = QJsonDocument::fromJson(m_historyJson).array();
    for (int i = 0; i < history.size(); ++i) {
        QJsonObject entry = history.at(i).toObject();
        if (static_cast<quint32>(entry.value(QStringLiteral("id")).toDouble()) != id)
            continue;
        entry.insert(QStringLiteral("closedAt"),
                     static_cast<double>(QDateTime::currentMSecsSinceEpoch()));
        entry.insert(QStringLiteral("reason"), reason);
        history.replace(i, entry);
        break;
    }
    m_historyJson = QJsonDocument(history).toJson(QJsonDocument::Compact);
    m_bannerJson = QByteArrayLiteral("[]");
    m_bannerId = 0;
    m_expiry.stop();
    refresh();
}

void MockNotificationClient::dismiss(quint32 id)
{
    remove(id, QStringLiteral("dismissed"));
}

void MockNotificationClient::expire(quint32 id)
{
    remove(id, QStringLiteral("expired"));
}

void MockNotificationClient::invoke(quint32 id, const QString &actionKey)
{
    Q_UNUSED(actionKey);
    // The live service emits `ActionInvoked` to the originating app and then
    // dismisses the banner; the mock has no app, so only the dismissal is
    // observable here.
    remove(id, QStringLiteral("dismissed"));
}

void MockNotificationClient::setFocusMode(const QString &mode)
{
    // Mirror the service's accepted names (`do-not-disturb` is its alias).
    const QString normalized =
        mode == QLatin1String("do-not-disturb") ? QStringLiteral("dnd") : mode;
    if (normalized != QLatin1String("off") && normalized != QLatin1String("focus")
        && normalized != QLatin1String("dnd"))
        return;
    if (normalized == m_focusMode)
        return;
    m_focusMode = normalized;
    // The batch clears when the mode returns to `off`, exactly like the
    // service (T-11.2a semantics).
    if (m_focusMode == QLatin1String("off"))
        m_focusBatched = 0;
    emit focusPolicyChanged(policyJson());
}

void MockNotificationClient::setDoNotDisturb(bool enabled)
{
    // The T-11.1a compat surface maps the bool onto the three-way policy.
    setFocusMode(enabled ? QStringLiteral("dnd") : QStringLiteral("off"));
}
