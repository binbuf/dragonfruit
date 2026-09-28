// SPDX-License-Identifier: MIT
#include "NotificationsClient.h"

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
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.Notifications");

} // namespace

// --- DbusNotificationsClient ----------------------------------------------

DbusNotificationsClient::DbusNotificationsClient(QObject *parent)
    : NotificationsClient(parent)
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

DbusNotificationsClient::~DbusNotificationsClient() = default;

bool DbusNotificationsClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusNotificationsClient::refresh()
{
    call(QStringLiteral("State"), {});
}

void DbusNotificationsClient::setFocusMode(const QString &mode)
{
    if (!mode.isEmpty())
        call(QStringLiteral("SetFocusMode"), { mode });
}

void DbusNotificationsClient::setFocusAllowList(const QStringList &apps)
{
    call(QStringLiteral("SetFocusAllowList"), { apps });
}

void DbusNotificationsClient::call(const QString &method, const QVariantList &arguments)
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

void DbusNotificationsClient::applyReply(const QByteArray &json)
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

// --- MockNotificationsClient ----------------------------------------------

MockNotificationsClient::MockNotificationsClient(QObject *parent)
    : NotificationsClient(parent)
{
    rebuild();
}

void MockNotificationsClient::refresh()
{
    rebuild();
}

void MockNotificationsClient::setFocusMode(const QString &mode)
{
    if (mode == QStringLiteral("off") || mode == QStringLiteral("focus")
            || mode == QStringLiteral("dnd"))
        m_mode = mode;
    rebuild();
}

void MockNotificationsClient::setFocusAllowList(const QStringList &apps)
{
    m_allowList = apps;
    rebuild();
}

void MockNotificationsClient::rebuild()
{
    const auto modeLabel = [this]() {
        if (m_mode == QStringLiteral("dnd"))
            return QStringLiteral("Do Not Disturb");
        if (m_mode == QStringLiteral("focus"))
            return QStringLiteral("Focus");
        return QStringLiteral("Off");
    };
    const bool suppressing = m_mode != QStringLiteral("off");

    // The observed apps, case-insensitively sorted like the adapter model
    // (chat, Mail, Pager).
    const struct {
        const char *name;
        int notifications;
    } source[] = {
        { "chat", 0 },
        { "Mail", 1 },
        { "Pager", 1 },
    };
    QVariantList apps;
    for (const auto &entry : source) {
        const QString name = QString::fromLatin1(entry.name);
        const bool allowed = m_allowList.contains(name, Qt::CaseInsensitive);
        QVariantMap app;
        app.insert(QStringLiteral("name"), name);
        app.insert(QStringLiteral("allowed"), allowed);
        app.insert(QStringLiteral("notifications"), entry.notifications);
        app.insert(QStringLiteral("status"),
                   allowed ? QStringLiteral("Allowed") : QStringLiteral("Default"));
        apps.append(app);
    }

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("notifications"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("bell"));
    view.insert(QStringLiteral("label"), modeLabel());
    view.insert(QStringLiteral("mode"), m_mode);
    view.insert(QStringLiteral("modeLabel"), modeLabel());
    view.insert(QStringLiteral("suppressing"), suppressing);
    view.insert(QStringLiteral("dnd"), m_mode == QStringLiteral("dnd"));
    view.insert(QStringLiteral("batched"), suppressing ? 2 : 0);
    view.insert(QStringLiteral("allowList"), m_allowList);
    view.insert(QStringLiteral("activeCount"), 1);
    view.insert(QStringLiteral("historyCount"), 2);
    view.insert(QStringLiteral("appCount"), apps.length());
    view.insert(QStringLiteral("apps"), apps);
    m_view = view;
    emit changed(m_view);
}