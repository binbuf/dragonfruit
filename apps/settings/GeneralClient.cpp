// SPDX-License-Identifier: GPL-3.0-only
#include "GeneralClient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QDateTime>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.SystemStatus1");
const QString kPath = QStringLiteral("/org/dragonfruit/SystemStatus1");
const QString kInterface = QStringLiteral("org.dragonfruit.SystemStatus1.General");

QVariantMap subMap(const QVariantMap &view, const QString &key)
{
    return view.value(key).toMap();
}

} // namespace

// --- DbusGeneralClient -----------------------------------------------------

DbusGeneralClient::DbusGeneralClient(QObject *parent)
    : GeneralClient(parent)
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

DbusGeneralClient::~DbusGeneralClient() = default;

bool DbusGeneralClient::available() const
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusGeneralClient::refresh()
{
    call(QStringLiteral("Refresh"), {}, QString(), true);
}

void DbusGeneralClient::setNtp(bool enabled)
{
    call(QStringLiteral("SetNtp"), {enabled}, QStringLiteral("ntp"));
}

void DbusGeneralClient::setTimezone(const QString &timezone)
{
    call(QStringLiteral("SetTimezone"), {timezone}, QStringLiteral("timezone"));
}

void DbusGeneralClient::setTime(const QString &value)
{
    call(QStringLiteral("SetTime"), {value}, QStringLiteral("time"));
}

void DbusGeneralClient::setLocale(const QString &locale)
{
    call(QStringLiteral("SetLocale"), {locale}, QStringLiteral("locale"));
}

void DbusGeneralClient::setAutostartEnabled(const QString &id, bool enabled)
{
    call(QStringLiteral("SetAutostartEnabled"), {id, enabled}, QStringLiteral("login"));
}

void DbusGeneralClient::setUserUnitEnabled(const QString &unit, bool enabled)
{
    call(QStringLiteral("SetUserUnitEnabled"), {unit, enabled}, QStringLiteral("login"));
}

void DbusGeneralClient::setSharingEnabled(const QString &id, bool enabled)
{
    call(QStringLiteral("SetSharingEnabled"), {id, enabled}, QStringLiteral("sharing"));
}

void DbusGeneralClient::runBackup(const QString &destination)
{
    call(QStringLiteral("RunBackup"), {destination}, QStringLiteral("backup"));
}

void DbusGeneralClient::openRestore()
{
    call(QStringLiteral("OpenRestore"), {}, QStringLiteral("backup"));
}

void DbusGeneralClient::openPasswordManager(const QString &id)
{
    if (!id.isEmpty())
        call(QStringLiteral("OpenPasswordManager"), {id}, QStringLiteral("passwords"));
}

void DbusGeneralClient::addAutostart(const QString &appId)
{
    if (!appId.trimmed().isEmpty())
        call(QStringLiteral("AddAutostart"), {appId.trimmed()}, QStringLiteral("login"));
}

void DbusGeneralClient::removeAutostart(const QString &id)
{
    if (!id.trimmed().isEmpty())
        call(QStringLiteral("RemoveAutostart"), {id.trimmed()}, QStringLiteral("login"));
}

void DbusGeneralClient::setHostname(const QString &hostname)
{
    call(QStringLiteral("SetHostname"), {hostname.trimmed()}, QStringLiteral("hostname"));
}

void DbusGeneralClient::restartSystem()
{
    call(QStringLiteral("RestartSystem"), {}, QStringLiteral("restart"));
}

void DbusGeneralClient::call(const QString &method, const QVariantList &arguments,
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
        return;
    }
    auto *watcher = new QDBusPendingCallWatcher(
        iface->asyncCallWithArgumentList(method, arguments), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, isState, operation, iface, watcher]() {
                const QDBusPendingReply<QString> reply = *watcher;
                iface->deleteLater();
                watcher->deleteLater();
                if (!reply.isValid())
                    return;
                if (isState) {
                    applyReply(reply.value().toUtf8());
                    return;
                }
                // The write reply is the outcome report (`applied`, `absent`,
                // `denied`, or `failed` with a message). The new state is the
                // host's re-read, never invented here.
                const QJsonDocument report =
                    QJsonDocument::fromJson(reply.value().toUtf8());
                if (report.isObject()) {
                    const QJsonObject object = report.object();
                    emit writeFinished(operation, object.value(QStringLiteral("outcome")).toString(),
                                       object.value(QStringLiteral("message")).toString());
                }
                refresh();
            });
}

void DbusGeneralClient::applyReply(const QByteArray &json)
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

// --- MockGeneralClient -----------------------------------------------------

MockGeneralClient::MockGeneralClient(QObject *parent)
    : GeneralClient(parent)
{
    rebuild();
}

void MockGeneralClient::refresh()
{
    // The fixture keeps the writes applied in-process, like the host's
    // authoritative re-read after a write.
    emit changed(m_view);
}

void MockGeneralClient::setNtp(bool enabled)
{
    if (failWrite(QStringLiteral("ntp")))
        return;
    applyToMap(QStringLiteral("time"), QString(), QStringLiteral("ntpEnabled"), enabled);
    QVariantMap time = subMap(m_view, QStringLiteral("time"));
    time.insert(QStringLiteral("label"),
                QStringLiteral("%1 · %2").arg(
                    time.value(QStringLiteral("timezone")).toString(),
                    enabled ? QStringLiteral("Automatic") : QStringLiteral("Manual")));
    m_view.insert(QStringLiteral("time"), time);
    emit writeFinished(QStringLiteral("ntp"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::setTimezone(const QString &timezone)
{
    if (failWrite(QStringLiteral("timezone")))
        return;
    applyToMap(QStringLiteral("time"), QString(), QStringLiteral("timezone"), timezone);
    QVariantMap time = subMap(m_view, QStringLiteral("time"));
    time.insert(QStringLiteral("label"),
                QStringLiteral("%1 · %2").arg(
                    timezone,
                    time.value(QStringLiteral("ntpEnabled")).toBool()
                        ? QStringLiteral("Automatic")
                        : QStringLiteral("Manual")));
    m_view.insert(QStringLiteral("time"), time);
    emit writeFinished(QStringLiteral("timezone"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::setTime(const QString &value)
{
    if (failWrite(QStringLiteral("time")))
        return;
    m_lastTime = value;
    const QDateTime parsed = QDateTime::fromString(value, QStringLiteral("yyyy-MM-dd HH:mm:ss"));
    if (parsed.isValid()) {
        QVariantMap time = subMap(m_view, QStringLiteral("time"));
        time.insert(QStringLiteral("localTimeMs"), parsed.toMSecsSinceEpoch());
        m_view.insert(QStringLiteral("time"), time);
    }
    emit writeFinished(QStringLiteral("time"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::setLocale(const QString &locale)
{
    if (failWrite(QStringLiteral("locale")))
        return;
    applyToMap(QStringLiteral("locale"), QString(), QStringLiteral("locale"), locale);
    applyToMap(QStringLiteral("locale"), QString(), QStringLiteral("formatsLocale"), locale);
    QVariantMap map = subMap(m_view, QStringLiteral("locale"));
    map.insert(QStringLiteral("language"), locale.section(QLatin1Char('_'), 0, 0));
    map.insert(QStringLiteral("label"), locale);
    m_view.insert(QStringLiteral("locale"), map);
    emit writeFinished(QStringLiteral("locale"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::setAutostartEnabled(const QString &id, bool enabled)
{
    if (failWrite(QStringLiteral("login")))
        return;
    QVariantMap login = subMap(m_view, QStringLiteral("login"));
    QVariantList items = login.value(QStringLiteral("autostart")).toList();
    for (QVariant &entry : items) {
        QVariantMap map = entry.toMap();
        if (map.value(QStringLiteral("id")).toString() == id) {
            map.insert(QStringLiteral("enabled"), enabled);
            entry = map;
        }
    }
    login.insert(QStringLiteral("autostart"), items);
    m_view.insert(QStringLiteral("login"), login);
    emit writeFinished(QStringLiteral("login"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::setUserUnitEnabled(const QString &unit, bool enabled)
{
    if (failWrite(QStringLiteral("login")))
        return;
    QVariantMap login = subMap(m_view, QStringLiteral("login"));
    QVariantList units = login.value(QStringLiteral("userUnits")).toList();
    for (QVariant &entry : units) {
        QVariantMap map = entry.toMap();
        if (map.value(QStringLiteral("id")).toString() == unit) {
            map.insert(QStringLiteral("enabled"),
                       enabled ? QStringLiteral("enabled") : QStringLiteral("disabled"));
            entry = map;
        }
    }
    login.insert(QStringLiteral("userUnits"), units);
    m_view.insert(QStringLiteral("login"), login);
    emit writeFinished(QStringLiteral("login"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::setSharingEnabled(const QString &id, bool enabled)
{
    if (failWrite(QStringLiteral("sharing")))
        return;
    QVariantMap sharing = subMap(m_view, QStringLiteral("sharing"));
    QVariantList services = sharing.value(QStringLiteral("services")).toList();
    int active = 0;
    for (QVariant &entry : services) {
        QVariantMap map = entry.toMap();
        if (map.value(QStringLiteral("id")).toString() == id) {
            map.insert(QStringLiteral("enabled"), enabled);
            map.insert(QStringLiteral("active"), enabled);
            entry = map;
        }
        if (entry.toMap().value(QStringLiteral("active")).toBool())
            ++active;
    }
    sharing.insert(QStringLiteral("services"), services);
    sharing.insert(QStringLiteral("activeCount"), active);
    sharing.insert(QStringLiteral("label"),
                   QStringLiteral("%1 of %2 on")
                       .arg(active)
                       .arg(services.size()));
    m_view.insert(QStringLiteral("sharing"), sharing);
    emit writeFinished(QStringLiteral("sharing"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::runBackup(const QString &destination)
{
    if (failWrite(QStringLiteral("backup")))
        return;
    m_lastDestination = destination;
    QVariantMap backup = subMap(m_view, QStringLiteral("backup"));
    backup.insert(QStringLiteral("message"), QStringLiteral("Backup started"));
    backup.insert(QStringLiteral("running"), false);
    m_view.insert(QStringLiteral("backup"), backup);
    emit writeFinished(QStringLiteral("backup"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::openRestore()
{
    if (failWrite(QStringLiteral("backup")))
        return;
    ++m_restores;
    QVariantMap backup = subMap(m_view, QStringLiteral("backup"));
    backup.insert(QStringLiteral("message"), QStringLiteral("Restore opened"));
    m_view.insert(QStringLiteral("backup"), backup);
    emit writeFinished(QStringLiteral("backup"), QStringLiteral("applied"), QString());
    emit changed(m_view);
}

void MockGeneralClient::openPasswordManager(const QString &id)
{
    ++m_passwordOpens;
    m_lastPasswordManager = id;
}

void MockGeneralClient::addAutostart(const QString &appId)
{
    if (failWrite(QStringLiteral("login")))
        return;
    QVariantMap login = subMap(m_view, QStringLiteral("login"));
    QVariantList apps = login.value(QStringLiteral("availableApps")).toList();
    QVariantList autostart = login.value(QStringLiteral("autostart")).toList();
    QVariantMap chosen;
    for (int i = 0; i < apps.size(); ++i) {
        const QVariantMap app = apps.at(i).toMap();
        if (app.value(QStringLiteral("id")).toString() == appId) {
            chosen = app;
            apps.removeAt(i);
            break;
        }
    }
    if (chosen.isEmpty()) {
        emit writeFinished(QStringLiteral("login"), QStringLiteral("failed"),
                           QStringLiteral("The application is no longer installed."));
        return;
    }
    autostart.append(QVariantMap{
        {QStringLiteral("id"), chosen.value(QStringLiteral("id"))},
        {QStringLiteral("name"), chosen.value(QStringLiteral("name"))},
        {QStringLiteral("command"), chosen.value(QStringLiteral("command"))},
        {QStringLiteral("source"), QStringLiteral("user")},
        {QStringLiteral("path"), QStringLiteral("/home/user/.config/autostart/%1.desktop")
                                     .arg(appId)},
        {QStringLiteral("enabled"), true},
        {QStringLiteral("system"), false},
        {QStringLiteral("canToggle"), true},
        {QStringLiteral("broken"), false},
        {QStringLiteral("brokenReason"), QString()},
    });
    login.insert(QStringLiteral("autostart"), autostart);
    login.insert(QStringLiteral("availableApps"), apps);
    login.insert(QStringLiteral("count"),
                 autostart.size() + login.value(QStringLiteral("userUnits")).toList().size());
    login.insert(QStringLiteral("label"),
                 QStringLiteral("%1 items")
                     .arg(autostart.size()
                          + login.value(QStringLiteral("userUnits")).toList().size()));
    m_view.insert(QStringLiteral("login"), login);
    emit changed(m_view);
    emit writeFinished(QStringLiteral("login"), QStringLiteral("applied"), QString());
}

void MockGeneralClient::removeAutostart(const QString &id)
{
    if (failWrite(QStringLiteral("login")))
        return;
    QVariantMap login = subMap(m_view, QStringLiteral("login"));
    QVariantList autostart = login.value(QStringLiteral("autostart")).toList();
    for (int i = 0; i < autostart.size(); ++i) {
        if (autostart.at(i).toMap().value(QStringLiteral("id")).toString() == id) {
            autostart.removeAt(i);
            break;
        }
    }
    login.insert(QStringLiteral("autostart"), autostart);
    const int count = autostart.size()
        + login.value(QStringLiteral("userUnits")).toList().size();
    login.insert(QStringLiteral("count"), count);
    login.insert(QStringLiteral("label"),
                 count == 0 ? QStringLiteral("No login items")
                            : QStringLiteral("%1 items").arg(count));
    m_view.insert(QStringLiteral("login"), login);
    emit changed(m_view);
    emit writeFinished(QStringLiteral("login"), QStringLiteral("applied"), QString());
}

void MockGeneralClient::setHostname(const QString &hostname)
{
    if (failWrite(QStringLiteral("hostname")))
        return;
    const QString value = hostname.trimmed();
    QString error;
    if (value.isEmpty())
        error = QStringLiteral("The hostname cannot be empty.");
    else if (value.size() > 63)
        error = QStringLiteral("The hostname cannot be longer than 63 characters.");
    else if (value.startsWith(QLatin1Char('-')) || value.endsWith(QLatin1Char('-')))
        error = QStringLiteral("The hostname cannot start or end with a hyphen.");
    else {
        for (const QChar character : value) {
            const bool asciiAlphanumeric =
                (character >= QLatin1Char('a') && character <= QLatin1Char('z'))
                || (character >= QLatin1Char('A') && character <= QLatin1Char('Z'))
                || (character >= QLatin1Char('0') && character <= QLatin1Char('9'));
            if (!asciiAlphanumeric && character != QLatin1Char('-')) {
                error = QStringLiteral("The hostname can only contain letters, digits, and hyphens.");
                break;
            }
        }
    }
    if (!error.isEmpty()) {
        emit writeFinished(QStringLiteral("hostname"), QStringLiteral("failed"), error);
        return;
    }
    QVariantMap sharing = subMap(m_view, QStringLiteral("sharing"));
    sharing.insert(QStringLiteral("hostname"), value);
    m_view.insert(QStringLiteral("sharing"), sharing);
    emit changed(m_view);
    emit writeFinished(QStringLiteral("hostname"), QStringLiteral("applied"), QString());
}

void MockGeneralClient::restartSystem()
{
    if (failWrite(QStringLiteral("restart")))
        return;
    ++m_restarts;
    emit writeFinished(QStringLiteral("restart"), QStringLiteral("applied"), QString());
}

bool MockGeneralClient::failWrite(const QString &operation)
{
    if (m_failOperation.isEmpty() || m_failOperation != operation)
        return false;
    m_failOperation.clear();
    const QString message = m_failMessage;
    m_failMessage.clear();
    emit writeFinished(operation, QStringLiteral("denied"), message);
    return true;
}

void MockGeneralClient::failNextWrite(const QString &operation, const QString &message)
{
    m_failOperation = operation;
    m_failMessage = message;
}

void MockGeneralClient::resetForTest()
{
    m_absent = false;
    rebuild();
}

void MockGeneralClient::setAbsentFixture(bool absent)
{
    if (m_absent == absent)
        return;
    m_absent = absent;
    emit availableChanged(!m_absent);
    emit changed(view());
}

void MockGeneralClient::applyToMap(const QString &section, const QString &id,
                                   const QString &key, const QVariant &value)
{
    QVariantMap map = subMap(m_view, section);
    if (!id.isEmpty()) {
        QVariantList list = map.value(key).toList();
        for (QVariant &entry : list) {
            QVariantMap item = entry.toMap();
            if (item.value(QStringLiteral("id")).toString() == id) {
                item.insert(key, value);
                entry = item;
            }
        }
        map.insert(key, list);
    } else {
        map.insert(key, value);
    }
    m_view.insert(section, map);
}

void MockGeneralClient::rebuild()
{
    QVariantMap view;
    view.insert(QStringLiteral("kind"), QStringLiteral("general"));
    view.insert(QStringLiteral("state"), QStringLiteral("available"));

    QVariantMap time;
    time.insert(QStringLiteral("state"), QStringLiteral("available"));
    time.insert(QStringLiteral("timezone"), QStringLiteral("America/Los_Angeles"));
    time.insert(QStringLiteral("ntpEnabled"), true);
    time.insert(QStringLiteral("ntpSynchronized"), true);
    time.insert(QStringLiteral("canNtp"), true);
    time.insert(QStringLiteral("localTimeMs"), qlonglong(1760000000000));
    time.insert(QStringLiteral("label"), QStringLiteral("America/Los_Angeles · Automatic"));
    time.insert(QStringLiteral("timezones"),
                QVariantList{QStringLiteral("UTC"),
                             QStringLiteral("America/Los_Angeles"),
                             QStringLiteral("Europe/Berlin")});
    view.insert(QStringLiteral("time"), time);

    QVariantMap locale;
    locale.insert(QStringLiteral("state"), QStringLiteral("available"));
    locale.insert(QStringLiteral("locale"), QStringLiteral("en_US.UTF-8"));
    locale.insert(QStringLiteral("language"), QStringLiteral("en"));
    locale.insert(QStringLiteral("formatsLocale"), QStringLiteral("en_US.UTF-8"));
    locale.insert(QStringLiteral("keymap"), QStringLiteral("us"));
    locale.insert(QStringLiteral("label"), QStringLiteral("en_US.UTF-8"));
    locale.insert(QStringLiteral("availableLocales"),
                  QVariantList{QStringLiteral("en_US.UTF-8"),
                               QStringLiteral("de_DE.UTF-8"),
                               QStringLiteral("fr_FR.UTF-8")});
    view.insert(QStringLiteral("locale"), locale);

    QVariantMap login;
    login.insert(QStringLiteral("state"), QStringLiteral("available"));
    login.insert(QStringLiteral("extensionsAvailable"), false);
    login.insert(QStringLiteral("extensionsNote"),
                 QStringLiteral("Extensions are managed by the desktop portal and installed apps."));
    login.insert(QStringLiteral("count"), 4);
    login.insert(QStringLiteral("label"), QStringLiteral("4 items"));
    QVariantList autostart;
    autostart.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("org.dragonfruit.Backup")},
        {QStringLiteral("name"), QStringLiteral("Dragonfruit Backup")},
        {QStringLiteral("command"), QStringLiteral("/usr/bin/dragonfruit-backup")},
        {QStringLiteral("source"), QStringLiteral("user")},
        {QStringLiteral("path"), QStringLiteral("/home/user/.config/autostart/org.dragonfruit.Backup.desktop")},
        {QStringLiteral("enabled"), true},
        {QStringLiteral("system"), false},
        {QStringLiteral("canToggle"), true},
        {QStringLiteral("broken"), false},
        {QStringLiteral("brokenReason"), QString()},
    });
    autostart.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("org.example.Agent")},
        {QStringLiteral("name"), QStringLiteral("Example Agent")},
        {QStringLiteral("command"), QStringLiteral("/usr/bin/example-agent")},
        {QStringLiteral("source"), QStringLiteral("system")},
        {QStringLiteral("path"), QStringLiteral("/etc/xdg/autostart/org.example.Agent.desktop")},
        {QStringLiteral("enabled"), false},
        {QStringLiteral("system"), true},
        {QStringLiteral("canToggle"), true},
        {QStringLiteral("broken"), true},
        {QStringLiteral("brokenReason"),
         QStringLiteral("The command /usr/bin/example-agent is missing.")},
    });
    login.insert(QStringLiteral("autostart"), autostart);
    QVariantList units;
    units.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("backup.service")},
        {QStringLiteral("name"), QStringLiteral("Backup")},
        {QStringLiteral("enabled"), QStringLiteral("enabled")},
        {QStringLiteral("active"), QStringLiteral("active")},
        {QStringLiteral("canToggle"), true},
    });
    units.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("sync.service")},
        {QStringLiteral("name"), QStringLiteral("Sync")},
        {QStringLiteral("enabled"), QStringLiteral("disabled")},
        {QStringLiteral("active"), QStringLiteral("inactive")},
        {QStringLiteral("canToggle"), true},
    });
    login.insert(QStringLiteral("userUnits"), units);
    login.insert(QStringLiteral("availableApps"), QVariantList{
        QVariantMap{
            {QStringLiteral("id"), QStringLiteral("org.example.Notes")},
            {QStringLiteral("name"), QStringLiteral("Example Notes")},
            {QStringLiteral("command"), QStringLiteral("/usr/bin/example-notes")},
        },
        QVariantMap{
            {QStringLiteral("id"), QStringLiteral("org.example.Broken")},
            {QStringLiteral("name"), QStringLiteral("Example Broken Agent")},
            {QStringLiteral("command"), QStringLiteral("/usr/bin/example-missing")},
        },
    });
    login.insert(QStringLiteral("extensions"), QVariantList{
        QVariantMap{
            {QStringLiteral("id"), QStringLiteral("org.freedesktop.impl.portal.Screenshot")},
            {QStringLiteral("name"), QStringLiteral("Screenshot")},
            {QStringLiteral("category"), QStringLiteral("Portal backends")},
            {QStringLiteral("detail"),
             QStringLiteral("A desktop portal backend installed on this system.")},
        },
    });
    view.insert(QStringLiteral("login"), login);

    QVariantMap sharing;
    sharing.insert(QStringLiteral("state"), QStringLiteral("available"));
    sharing.insert(QStringLiteral("hostname"), QStringLiteral("dragon"));
    sharing.insert(QStringLiteral("hostnameEditable"), true);
    sharing.insert(QStringLiteral("hostnameReason"), QString());
    sharing.insert(QStringLiteral("installedCount"), 3);
    sharing.insert(QStringLiteral("activeCount"), 2);
    sharing.insert(QStringLiteral("label"), QStringLiteral("2 of 3 on"));
    QVariantList services;
    services.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("file")},
        {QStringLiteral("name"), QStringLiteral("File Sharing")},
        {QStringLiteral("description"), QStringLiteral("Share folders with other computers using SMB.")},
        {QStringLiteral("unit"), QStringLiteral("smb.service")},
        {QStringLiteral("installed"), true},
        {QStringLiteral("enabled"), true},
        {QStringLiteral("active"), false},
        {QStringLiteral("canToggle"), true},
        {QStringLiteral("needsAuth"), true},
        {QStringLiteral("details"), QVariantList{
             QVariantMap{{QStringLiteral("label"), QStringLiteral("Unit")},
                         {QStringLiteral("value"), QStringLiteral("smb.service")}},
             QVariantMap{{QStringLiteral("label"), QStringLiteral("Shares")},
                         {QStringLiteral("value"), QStringLiteral("2")}},
         }},
    });
    services.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("remoteLogin")},
        {QStringLiteral("name"), QStringLiteral("Remote Login")},
        {QStringLiteral("description"), QStringLiteral("Allow remote login with SSH.")},
        {QStringLiteral("unit"), QStringLiteral("sshd.service")},
        {QStringLiteral("installed"), true},
        {QStringLiteral("enabled"), true},
        {QStringLiteral("active"), true},
        {QStringLiteral("canToggle"), true},
        {QStringLiteral("needsAuth"), true},
        {QStringLiteral("details"), QVariantList{
             QVariantMap{{QStringLiteral("label"), QStringLiteral("Unit")},
                         {QStringLiteral("value"), QStringLiteral("sshd.service")}},
             QVariantMap{{QStringLiteral("label"), QStringLiteral("Port")},
                         {QStringLiteral("value"), QStringLiteral("22")}},
         }},
    });
    services.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("printer")},
        {QStringLiteral("name"), QStringLiteral("Printer Sharing")},
        {QStringLiteral("description"), QStringLiteral("Share printers with other computers.")},
        {QStringLiteral("unit"), QStringLiteral("cups.service")},
        {QStringLiteral("installed"), true},
        {QStringLiteral("enabled"), true},
        {QStringLiteral("active"), true},
        {QStringLiteral("canToggle"), true},
        {QStringLiteral("needsAuth"), true},
        {QStringLiteral("details"), QVariantList{
             QVariantMap{{QStringLiteral("label"), QStringLiteral("Queues")},
                         {QStringLiteral("value"), QStringLiteral("1")}},
         }},
    });
    sharing.insert(QStringLiteral("services"), services);
    view.insert(QStringLiteral("sharing"), sharing);

    QVariantMap backup;
    backup.insert(QStringLiteral("state"), QStringLiteral("available"));
    backup.insert(QStringLiteral("backend"), QStringLiteral("timeshift"));
    backup.insert(QStringLiteral("backendLabel"), QStringLiteral("Timeshift"));
    backup.insert(QStringLiteral("detectedDestination"), QStringLiteral("/dev/sdb1"));
    backup.insert(QStringLiteral("detectedSchedule"), QStringLiteral("daily"));
    backup.insert(QStringLiteral("lastRun"), QStringLiteral("2026-11-06 22:00"));
    backup.insert(QStringLiteral("running"), false);
    backup.insert(QStringLiteral("canRun"), true);
    backup.insert(QStringLiteral("canRestore"), true);
    backup.insert(QStringLiteral("message"), QString());
    backup.insert(QStringLiteral("label"), QStringLiteral("Timeshift · daily"));
    QVariantList candidates;
    candidates.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("restic")},
        {QStringLiteral("label"), QStringLiteral("restic")},
        {QStringLiteral("installed"), false},
        {QStringLiteral("canRun"), false},
        {QStringLiteral("canRestore"), false},
    });
    candidates.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("pika")},
        {QStringLiteral("label"), QStringLiteral("Pika Backup")},
        {QStringLiteral("installed"), false},
        {QStringLiteral("canRun"), false},
        {QStringLiteral("canRestore"), false},
    });
    candidates.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("timeshift")},
        {QStringLiteral("label"), QStringLiteral("Timeshift")},
        {QStringLiteral("installed"), true},
        {QStringLiteral("canRun"), true},
        {QStringLiteral("canRestore"), true},
    });
    backup.insert(QStringLiteral("candidates"), candidates);
    view.insert(QStringLiteral("backup"), backup);

    QVariantMap storage;
    storage.insert(QStringLiteral("state"), QStringLiteral("available"));
    storage.insert(QStringLiteral("totalBytes"), qlonglong(512000000000));
    storage.insert(QStringLiteral("usedBytes"), qlonglong(480000000000));
    storage.insert(QStringLiteral("freeBytes"), qlonglong(32000000000));
    storage.insert(QStringLiteral("label"), QStringLiteral("447 GiB used"));
    QVariantList categories;
    categories.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("system")},
        {QStringLiteral("label"), QStringLiteral("System")},
        {QStringLiteral("bytes"), qlonglong(120000000000)},
        {QStringLiteral("glyph"), QStringLiteral("hard-drives")},
    });
    categories.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("home")},
        {QStringLiteral("label"), QStringLiteral("Home")},
        {QStringLiteral("bytes"), qlonglong(340000000000)},
        {QStringLiteral("glyph"), QStringLiteral("house")},
    });
    categories.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("media")},
        {QStringLiteral("label"), QStringLiteral("Media")},
        {QStringLiteral("bytes"), qlonglong(20000000000)},
        {QStringLiteral("glyph"), QStringLiteral("image")},
    });
    storage.insert(QStringLiteral("categories"), categories);
    QVariantList volumes;
    volumes.append(QVariantMap{
        {QStringLiteral("device"), QStringLiteral("/dev/nvme0n1p3")},
        {QStringLiteral("mountPoint"), QStringLiteral("/")},
        {QStringLiteral("category"), QStringLiteral("system")},
        {QStringLiteral("totalBytes"), qlonglong(512000000000)},
        {QStringLiteral("usedBytes"), qlonglong(480000000000)},
        {QStringLiteral("availableBytes"), qlonglong(32000000000)},
        {QStringLiteral("removable"), false},
    });
    storage.insert(QStringLiteral("volumes"), volumes);
    view.insert(QStringLiteral("storage"), storage);

    QVariantMap startup;
    startup.insert(QStringLiteral("state"), QStringLiteral("available"));
    startup.insert(QStringLiteral("device"), QStringLiteral("/dev/nvme0n1p3"));
    startup.insert(QStringLiteral("disk"), QStringLiteral("/dev/nvme0n1"));
    startup.insert(QStringLiteral("filesystem"), QStringLiteral("btrfs"));
    startup.insert(QStringLiteral("mountPoint"), QStringLiteral("/"));
    startup.insert(QStringLiteral("totalBytes"), qlonglong(512000000000));
    startup.insert(QStringLiteral("bootMode"), QStringLiteral("UEFI"));
    startup.insert(QStringLiteral("secureBoot"), QStringLiteral("On"));
    startup.insert(QStringLiteral("label"), QStringLiteral("fedora"));
    startup.insert(QStringLiteral("disks"), QVariantList{
        QVariantMap{
            {QStringLiteral("device"), QStringLiteral("/dev/nvme0n1")},
            {QStringLiteral("label"), QStringLiteral("fedora")},
            {QStringLiteral("model"), QStringLiteral("Example NVMe SSD")},
            {QStringLiteral("sizeBytes"), qlonglong(512000000000)},
            {QStringLiteral("boot"), true},
            {QStringLiteral("mounted"), true},
        },
        QVariantMap{
            {QStringLiteral("device"), QStringLiteral("/dev/sdb")},
            {QStringLiteral("label"), QStringLiteral("Backup")},
            {QStringLiteral("model"), QStringLiteral("Example USB Disk")},
            {QStringLiteral("sizeBytes"), qlonglong(1000000000000)},
            {QStringLiteral("boot"), false},
            {QStringLiteral("mounted"), false},
        },
    });
    view.insert(QStringLiteral("startup"), startup);

    // T269: the Autofill & Passwords provider inventory. The Settings app
    // stores no secrets; this is a read-only projection of installed managers
    // and the Secret Service lock state.
    QVariantMap passwords;
    passwords.insert(QStringLiteral("state"), QStringLiteral("available"));
    passwords.insert(QStringLiteral("secretServiceAvailable"), true);
    passwords.insert(QStringLiteral("keyringLocked"), true);
    passwords.insert(QStringLiteral("keyringLabel"), QStringLiteral("Locked"));
    passwords.insert(QStringLiteral("installedCount"), 2);
    passwords.insert(QStringLiteral("message"), QString());
    QVariantList providers;
    providers.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("keyring")},
        {QStringLiteral("label"), QStringLiteral("Desktop Keyring")},
        {QStringLiteral("installed"), true},
        {QStringLiteral("active"), true},
        {QStringLiteral("command"), QStringLiteral("gnome-keyring-daemon")}, // df-allow-desktop-name: fixture executable
        {QStringLiteral("launch"), QStringLiteral("seahorse")},
        {QStringLiteral("description"),
         QStringLiteral("The desktop keyring and Secret Service provider.")},
    });
    providers.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("keepassxc")},
        {QStringLiteral("label"), QStringLiteral("KeePassXC")},
        {QStringLiteral("installed"), true},
        {QStringLiteral("active"), false},
        {QStringLiteral("command"), QStringLiteral("keepassxc")},
        {QStringLiteral("launch"), QStringLiteral("keepassxc")},
        {QStringLiteral("description"),
         QStringLiteral("A local password database with browser integration.")},
    });
    providers.append(QVariantMap{
        {QStringLiteral("id"), QStringLiteral("bitwarden")},
        {QStringLiteral("label"), QStringLiteral("Bitwarden")},
        {QStringLiteral("installed"), false},
        {QStringLiteral("active"), false},
        {QStringLiteral("command"), QStringLiteral("bitwarden")},
        {QStringLiteral("launch"), QStringLiteral("")},
        {QStringLiteral("description"),
         QStringLiteral("A hosted password manager with a desktop client.")},
    });
    passwords.insert(QStringLiteral("providers"), providers);
    passwords.insert(QStringLiteral("label"), QStringLiteral("2 providers"));
    view.insert(QStringLiteral("passwords"), passwords);

    view.insert(QStringLiteral("errors"), QVariantMap());
    m_view = view;
    emit changed(m_view);
}
