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

void DbusNotificationsClient::setFocusProfile(const QString &id)
{
    if (!id.trimmed().isEmpty())
        call(QStringLiteral("SetFocusProfile"), { id.trimmed() });
}

void DbusNotificationsClient::setFocusAllowedPeople(const QStringList &people)
{
    call(QStringLiteral("SetFocusAllowedPeople"), { people });
}

void DbusNotificationsClient::setAppPreference(const QString &app,
                                               const QString &style, bool sound,
                                               bool badge, bool allowed)
{
    if (app.trimmed().isEmpty() || style.isEmpty())
        return;
    call(QStringLiteral("SetAppPreference"),
         { app.trimmed(), style, sound, badge, allowed });
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
            || mode == QStringLiteral("dnd")) {
        // A mode change keeps a named profile whose mode matches, exactly like
        // the service; otherwise it selects the synthetic custom profile.
        const bool matches =
            (mode == QStringLiteral("focus")
             && (m_profile == QStringLiteral("personal")
                 || m_profile == QStringLiteral("work")
                 || m_profile == QStringLiteral("custom")))
            || (mode == QStringLiteral("dnd")
                && (m_profile == QStringLiteral("dnd")
                    || m_profile == QStringLiteral("sleep")
                    || m_profile == QStringLiteral("custom")));
        m_mode = mode;
        if (mode == QStringLiteral("off"))
            m_profile.clear();
        else if (!matches)
            m_profile = QStringLiteral("custom");
    }
    rebuild();
}

void MockNotificationsClient::setFocusAllowList(const QStringList &apps)
{
    m_allowList = apps;
    rebuild();
}

void MockNotificationsClient::setFocusProfile(const QString &id)
{
    const QString profile = id.trimmed().toLower();
    if (profile == QStringLiteral("personal") || profile == QStringLiteral("work")) {
        m_profile = profile;
        m_mode = QStringLiteral("focus");
    } else if (profile == QStringLiteral("sleep")
               || profile == QStringLiteral("dnd")) {
        m_profile = profile;
        m_mode = QStringLiteral("dnd");
    } else if (profile == QStringLiteral("custom")) {
        m_profile = profile;
        if (m_mode == QStringLiteral("off"))
            m_mode = QStringLiteral("focus");
    } else {
        return;
    }
    // A named profile installs its own lists; the fixture gives Personal the
    // seeded Pager/Ada entries so the pane tests have a known allow list.
    // Custom keeps the current lists.
    if (profile == QStringLiteral("personal")) {
        m_allowList = QStringList{ QStringLiteral("Pager") };
        m_allowedPeople = QStringList{ QStringLiteral("Ada") };
    } else if (profile != QStringLiteral("custom")) {
        m_allowList.clear();
        m_allowedPeople.clear();
    }
    rebuild();
}

void MockNotificationsClient::setFocusAllowedPeople(const QStringList &people)
{
    m_allowedPeople = people;
    rebuild();
}

void MockNotificationsClient::setAppPreference(const QString &app,
                                               const QString &style, bool sound,
                                               bool badge, bool allowed)
{
    const QString name = app.trimmed();
    if (name.isEmpty())
        return;
    const QString key = name.toLower();
    const QVariantMap preference{
        { QStringLiteral("name"), name },
        { QStringLiteral("style"), style },
        { QStringLiteral("sound"), sound },
        { QStringLiteral("badge"), badge },
        { QStringLiteral("allowed"), allowed },
    };
    const bool isDefault = style == QStringLiteral("banner") && sound && badge
                           && allowed;
    if (isDefault)
        m_appPreferences.remove(key);
    else
        m_appPreferences.insert(key, preference);
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
        const QVariantMap stored =
            m_appPreferences.value(name.toLower());
        const QString style = stored.value(QStringLiteral("style"),
                                           QStringLiteral("banner")).toString();
        const bool notificationsAllowed =
            stored.value(QStringLiteral("allowed"), true).toBool();
        const bool sound = stored.value(QStringLiteral("sound"), true).toBool();
        const bool badge = stored.value(QStringLiteral("badge"), true).toBool();
        const QString styleLabel = style == QStringLiteral("alert")
            ? QStringLiteral("Alerts")
            : (style == QStringLiteral("none") ? QStringLiteral("None")
                                               : QStringLiteral("Banners"));
        const QString status = !notificationsAllowed
            ? QStringLiteral("Off")
            : (allowed ? QStringLiteral("Allowed") : QStringLiteral("Default"));
        QVariantMap app;
        app.insert(QStringLiteral("name"), name);
        app.insert(QStringLiteral("allowed"), allowed);
        app.insert(QStringLiteral("notificationsAllowed"), notificationsAllowed);
        app.insert(QStringLiteral("notifications"), entry.notifications);
        app.insert(QStringLiteral("status"), status);
        app.insert(QStringLiteral("style"), style);
        app.insert(QStringLiteral("styleLabel"), styleLabel);
        app.insert(QStringLiteral("sound"), sound);
        app.insert(QStringLiteral("badge"), badge);
        apps.append(app);
    }

    const auto profileLabel = [this, &modeLabel]() -> QString {
        if (m_profile == QStringLiteral("dnd"))
            return QStringLiteral("Do Not Disturb");
        if (m_profile == QStringLiteral("personal"))
            return QStringLiteral("Personal");
        if (m_profile == QStringLiteral("work"))
            return QStringLiteral("Work");
        if (m_profile == QStringLiteral("sleep"))
            return QStringLiteral("Sleep");
        // `custom` is a marker, not a name the user picked; it reads as the
        // mode, exactly like the service's presentation label.
        return modeLabel();
    };
    QVariantList profiles{
        QVariantMap{ { QStringLiteral("id"), QStringLiteral("dnd") },
                     { QStringLiteral("name"), QStringLiteral("Do Not Disturb") },
                     { QStringLiteral("mode"), QStringLiteral("dnd") } },
        QVariantMap{ { QStringLiteral("id"), QStringLiteral("personal") },
                     { QStringLiteral("name"), QStringLiteral("Personal") },
                     { QStringLiteral("mode"), QStringLiteral("focus") } },
        QVariantMap{ { QStringLiteral("id"), QStringLiteral("work") },
                     { QStringLiteral("name"), QStringLiteral("Work") },
                     { QStringLiteral("mode"), QStringLiteral("focus") } },
        QVariantMap{ { QStringLiteral("id"), QStringLiteral("sleep") },
                     { QStringLiteral("name"), QStringLiteral("Sleep") },
                     { QStringLiteral("mode"), QStringLiteral("dnd") } },
        QVariantMap{ { QStringLiteral("id"), QStringLiteral("custom") },
                     { QStringLiteral("name"), QStringLiteral("Custom") },
                     { QStringLiteral("mode"), QStringLiteral("focus") } },
    };

    // The observed people come from the optional notification sender hint.
    const struct {
        const char *name;
        int notifications;
    } peopleSource[] = {
        { "Ada", 1 },
        { "Grace", 1 },
    };
    QVariantList people;
    for (const auto &entry : peopleSource) {
        const QString name = QString::fromLatin1(entry.name);
        QVariantMap person;
        person.insert(QStringLiteral("name"), name);
        person.insert(QStringLiteral("allowed"),
                      m_allowedPeople.contains(name, Qt::CaseInsensitive));
        person.insert(QStringLiteral("notifications"), entry.notifications);
        people.append(person);
    }

    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("notifications"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));
    view.insert(QStringLiteral("glyph"), QStringLiteral("bell"));
    view.insert(QStringLiteral("label"), profileLabel());
    view.insert(QStringLiteral("mode"), m_mode);
    view.insert(QStringLiteral("modeLabel"), modeLabel());
    view.insert(QStringLiteral("profile"),
                m_profile.isEmpty() ? QVariant() : QVariant(m_profile));
    view.insert(QStringLiteral("profileLabel"), profileLabel());
    view.insert(QStringLiteral("profiles"), profiles);
    view.insert(QStringLiteral("suppressing"), suppressing);
    view.insert(QStringLiteral("dnd"), m_mode == QStringLiteral("dnd"));
    view.insert(QStringLiteral("batched"), suppressing ? 2 : 0);
    view.insert(QStringLiteral("allowList"), m_allowList);
    view.insert(QStringLiteral("allowedPeople"), m_allowedPeople);
    view.insert(QStringLiteral("activeCount"), 1);
    view.insert(QStringLiteral("historyCount"), 2);
    view.insert(QStringLiteral("appCount"), apps.length());
    view.insert(QStringLiteral("apps"), apps);
    view.insert(QStringLiteral("peopleCount"), people.length());
    view.insert(QStringLiteral("people"), people);
    m_view = view;
    emit changed(m_view);
}