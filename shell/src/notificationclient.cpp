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

void DbusNotificationClient::clearAll()
{
#if defined(QT_DBUS_LIB)
    // The service closes every banner and drops the history, then emits
    // `Changed`; the shell re-reads both views.
    QDBusInterface iface(m_service, m_path, m_interface, QDBusConnection::sessionBus());
    if (iface.isValid())
        iface.asyncCall(QStringLiteral("ClearAll"));
#else
#endif
}

void DbusNotificationClient::clearHistory()
{
#if defined(QT_DBUS_LIB)
    QDBusInterface iface(m_service, m_path, m_interface, QDBusConnection::sessionBus());
    if (iface.isValid())
        iface.asyncCall(QStringLiteral("ClearHistory"));
#endif
}

// --- MockNotificationClient ------------------------------------------------

MockNotificationClient::MockNotificationClient(QObject *parent, bool seedFixture)
    : NotificationClient(parent)
{
    m_expiry.setSingleShot(true);
    connect(&m_expiry, &QTimer::timeout, this, [this]() { expire(m_bannerId); });
    if (!seedFixture) {
        m_bannerJson = QByteArrayLiteral("[]");
        return;
    }
    // T205: the capture/demo fixture can select the Center's empty, populated,
    // concurrent, scrollable and banner-only states. The default (`1`) keeps
    // the historical single banner.
    const QString fixture = qEnvironmentVariable("DF_NOTIFY_FIXTURE");
    if (fixture == QLatin1String("empty") || fixture == QLatin1String("populated")
        || fixture == QLatin1String("concurrent") || fixture == QLatin1String("scrollable")
        || fixture == QLatin1String("banner-empty")) {
        seedCaptureFixture(fixture);
    } else {
        seedBanner();
    }
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

void MockNotificationClient::seedCaptureFixture(const QString &mode)
{
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    int count = 0;
    bool banner = false;
    if (mode == QLatin1String("populated")) {
        count = 4;
    } else if (mode == QLatin1String("concurrent")) {
        count = 4;
        banner = true;
    } else if (mode == QLatin1String("scrollable")) {
        count = 18;
    } else if (mode == QLatin1String("banner-empty")) {
        banner = true;
    }

    QJsonArray history;
    QJsonObject activeBanner;
    quint32 activeId = 0;
    for (int i = 0; i < count; ++i) {
        const quint32 id = m_nextId++;
        QJsonArray actions;
        if (banner && i == 0) {
            actions.append(QJsonObject{
                { QStringLiteral("key"), QStringLiteral("reply") },
                { QStringLiteral("label"), QStringLiteral("Reply") } });
            actions.append(QJsonObject{
                { QStringLiteral("key"), QStringLiteral("default") },
                { QStringLiteral("label"), QStringLiteral("Open") } });
        }
        QJsonObject entry{
            { QStringLiteral("id"), static_cast<double>(id) },
            { QStringLiteral("appName"),
              i % 2 == 0 ? QStringLiteral("Mail") : QStringLiteral("Chat") },
            { QStringLiteral("appIcon"), QString() },
            { QStringLiteral("summary"), QStringLiteral("Message %1").arg(i + 1) },
            { QStringLiteral("body"), QStringLiteral("Fixture notification body %1").arg(i + 1) },
            { QStringLiteral("urgency"),
              i == 0 ? QStringLiteral("critical") : QStringLiteral("normal") },
            { QStringLiteral("createdAt"), static_cast<double>(now - i * 60000) },
            { QStringLiteral("deadline"), static_cast<double>(now + 600000) },
            { QStringLiteral("actions"), actions },
            { QStringLiteral("suppressed"), false },
        };
        if (banner && i == 0) {
            entry.insert(QStringLiteral("closedAt"), QJsonValue::Null);
            entry.insert(QStringLiteral("reason"), QJsonValue::Null);
            activeBanner = entry;
            activeId = id;
        } else {
            entry.insert(QStringLiteral("closedAt"),
                         static_cast<double>(now - (i + 1) * 60000));
            entry.insert(QStringLiteral("reason"),
                         i % 3 == 0 ? QStringLiteral("expired")
                                    : i % 3 == 1 ? QStringLiteral("dismissed")
                                                 : QStringLiteral("closed"));
        }
        history.append(entry);
    }
    // `banner-empty` shows an active banner over an empty Center history so a
    // capture can distinguish all three materials with the empty state.
    if (banner && count == 0) {
        const quint32 id = m_nextId++;
        QJsonArray actions;
        actions.append(QJsonObject{
            { QStringLiteral("key"), QStringLiteral("default") },
            { QStringLiteral("label"), QStringLiteral("Open") } });
        activeBanner = QJsonObject{
            { QStringLiteral("id"), static_cast<double>(id) },
            { QStringLiteral("appName"), QStringLiteral("Mail") },
            { QStringLiteral("appIcon"), QString() },
            { QStringLiteral("summary"), QStringLiteral("New message") },
            { QStringLiteral("body"), QStringLiteral("Fixture banner with actions") },
            { QStringLiteral("urgency"), QStringLiteral("normal") },
            { QStringLiteral("createdAt"), static_cast<double>(now) },
            { QStringLiteral("deadline"), static_cast<double>(now + 600000) },
            { QStringLiteral("actions"), actions },
            { QStringLiteral("closedAt"), QJsonValue::Null },
            { QStringLiteral("reason"), QJsonValue::Null },
            { QStringLiteral("suppressed"), false },
        };
        activeId = id;
    }

    m_historyJson = QJsonDocument(history).toJson(QJsonDocument::Compact);
    if (banner) {
        m_bannerJson = QJsonDocument(QJsonArray{ activeBanner })
                           .toJson(QJsonDocument::Compact);
        m_bannerId = activeId;
        m_expiry.stop();
    } else {
        m_bannerJson = QByteArrayLiteral("[]");
        m_bannerId = 0;
        m_expiry.stop();
    }
}

void MockNotificationClient::refresh()
{
    emit bannersChanged(m_bannerJson);
    emit historyChanged(m_historyJson);
    emit focusPolicyChanged(policyJson());
}

QByteArray MockNotificationClient::policyJson() const
{
    // T234: the shell mock mirrors the service's named-profile presentation.
    // The mode stays the one admission rule; the label is what the menu bar
    // and Control Center show.
    const QString profileLabel =
        m_focusMode == QLatin1String("dnd") ? QStringLiteral("Do Not Disturb")
        : m_focusMode == QLatin1String("focus") ? QStringLiteral("Focus")
                                                : QString();
    QJsonArray profiles;
    const auto addProfile = [&profiles](const char *id, const char *name,
                                        const char *mode) {
        profiles.append(QJsonObject{
            { QStringLiteral("id"), QString::fromLatin1(id) },
            { QStringLiteral("name"), QString::fromLatin1(name) },
            { QStringLiteral("mode"), QString::fromLatin1(mode) },
            { QStringLiteral("apps"), QJsonArray{} },
            { QStringLiteral("people"), QJsonArray{} },
        });
    };
    addProfile("dnd", "Do Not Disturb", "dnd");
    addProfile("personal", "Personal", "focus");
    addProfile("work", "Work", "focus");
    addProfile("sleep", "Sleep", "dnd");
    addProfile("custom", "Custom", "focus");
    return QJsonDocument(
               QJsonObject{
                   { QStringLiteral("mode"), m_focusMode },
                   { QStringLiteral("allowList"), QJsonArray{} },
                   { QStringLiteral("allowedPeople"), QJsonArray{} },
                   { QStringLiteral("activeProfile"),
                     m_focusMode == QLatin1String("off")
                         ? QJsonValue(QJsonValue::Null)
                         : QJsonValue(m_focusMode == QLatin1String("dnd")
                                          ? QStringLiteral("dnd")
                                          : QStringLiteral("custom")) },
                   { QStringLiteral("profileLabel"),
                     profileLabel.isEmpty() ? QJsonValue(QJsonValue::Null)
                                            : QJsonValue(profileLabel) },
                   { QStringLiteral("profiles"), profiles },
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
    const QString effectiveUrgency =
        urgency.isEmpty() ? QStringLiteral("normal") : urgency;
    const QJsonObject banner{
        { QStringLiteral("id"), static_cast<double>(id) },
        { QStringLiteral("appName"), appName },
        { QStringLiteral("appIcon"), QString() },
        { QStringLiteral("summary"), summary },
        { QStringLiteral("body"), body },
        { QStringLiteral("urgency"), effectiveUrgency },
        { QStringLiteral("createdAt"), static_cast<double>(now) },
        { QStringLiteral("deadline"), static_cast<double>(now + 5000) },
        { QStringLiteral("actions"), actions },
    };

    // Mirror the service's Focus/DND admission rule (T-11.2a): off admits
    // everything, focus admits critical urgency, dnd admits nothing without an
    // allow list. A suppressed notification is still recorded in history and
    // batched, but never replaces the active banner.
    const bool admitted = m_focusMode == QLatin1String("off")
        || (m_focusMode == QLatin1String("focus")
            && effectiveUrgency == QLatin1String("critical"));

    // The history records the open entry immediately, exactly like the
    // service (reason/closedAt null until it leaves the queue).
    QJsonObject entry = banner;
    entry.insert(QStringLiteral("closedAt"), QJsonValue::Null);
    entry.insert(QStringLiteral("reason"), QJsonValue::Null);
    entry.insert(QStringLiteral("suppressed"), !admitted);
    QJsonArray history = QJsonDocument::fromJson(m_historyJson).array();
    history.prepend(entry);
    m_historyJson = QJsonDocument(history).toJson(QJsonDocument::Compact);

    if (admitted) {
        m_bannerId = id;
        m_bannerJson = QJsonDocument(QJsonArray{ banner }).toJson(QJsonDocument::Compact);
        m_expiry.stop();
        m_expiry.setInterval(5000);
        m_expiry.start();
    } else {
        ++m_focusBatched;
    }
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

void MockNotificationClient::clearAll()
{
    m_historyJson = QByteArrayLiteral("[]");
    m_bannerJson = QByteArrayLiteral("[]");
    m_bannerId = 0;
    m_expiry.stop();
    refresh();
}

void MockNotificationClient::clearHistory()
{
    m_historyJson = QByteArrayLiteral("[]");
    refresh();
}
