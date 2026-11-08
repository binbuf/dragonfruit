// SPDX-License-Identifier: MIT
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
    call(QStringLiteral("State"), {});
}

void DbusGeneralClient::setNtp(bool enabled)
{
    call(QStringLiteral("SetNtp"), {enabled});
}

void DbusGeneralClient::setTimezone(const QString &timezone)
{
    call(QStringLiteral("SetTimezone"), {timezone});
}

void DbusGeneralClient::setTime(const QString &value)
{
    call(QStringLiteral("SetTime"), {value});
}

void DbusGeneralClient::setLocale(const QString &locale)
{
    call(QStringLiteral("SetLocale"), {locale});
}

void DbusGeneralClient::setAutostartEnabled(const QString &id, bool enabled)
{
    call(QStringLiteral("SetAutostartEnabled"), {id, enabled});
}

void DbusGeneralClient::setUserUnitEnabled(const QString &unit, bool enabled)
{
    call(QStringLiteral("SetUserUnitEnabled"), {unit, enabled});
}

void DbusGeneralClient::setSharingEnabled(const QString &id, bool enabled)
{
    call(QStringLiteral("SetSharingEnabled"), {id, enabled});
}

void DbusGeneralClient::runBackup(const QString &destination)
{
    call(QStringLiteral("RunBackup"), {destination});
}

void DbusGeneralClient::openRestore()
{
    call(QStringLiteral("OpenRestore"), {});
}

void DbusGeneralClient::call(const QString &method, const QVariantList &arguments)
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
    applyToMap(QStringLiteral("time"), QString(), QStringLiteral("ntpEnabled"), enabled);
    QVariantMap time = subMap(m_view, QStringLiteral("time"));
    time.insert(QStringLiteral("label"),
                QStringLiteral("%1 · %2").arg(
                    time.value(QStringLiteral("timezone")).toString(),
                    enabled ? QStringLiteral("Automatic") : QStringLiteral("Manual")));
    m_view.insert(QStringLiteral("time"), time);
    emit changed(m_view);
}

void MockGeneralClient::setTimezone(const QString &timezone)
{
    applyToMap(QStringLiteral("time"), QString(), QStringLiteral("timezone"), timezone);
    QVariantMap time = subMap(m_view, QStringLiteral("time"));
    time.insert(QStringLiteral("label"),
                QStringLiteral("%1 · %2").arg(
                    timezone,
                    time.value(QStringLiteral("ntpEnabled")).toBool()
                        ? QStringLiteral("Automatic")
                        : QStringLiteral("Manual")));
    m_view.insert(QStringLiteral("time"), time);
    emit changed(m_view);
}

void MockGeneralClient::setTime(const QString &value)
{
    m_lastTime = value;
    const QDateTime parsed = QDateTime::fromString(value, QStringLiteral("yyyy-MM-dd HH:mm:ss"));
    if (parsed.isValid()) {
        QVariantMap time = subMap(m_view, QStringLiteral("time"));
        time.insert(QStringLiteral("localTimeMs"), parsed.toMSecsSinceEpoch());
        m_view.insert(QStringLiteral("time"), time);
    }
    emit changed(m_view);
}

void MockGeneralClient::setLocale(const QString &locale)
{
    applyToMap(QStringLiteral("locale"), QString(), QStringLiteral("locale"), locale);
    applyToMap(QStringLiteral("locale"), QString(), QStringLiteral("formatsLocale"), locale);
    QVariantMap map = subMap(m_view, QStringLiteral("locale"));
    map.insert(QStringLiteral("language"), locale.section(QLatin1Char('_'), 0, 0));
    map.insert(QStringLiteral("label"), locale);
    m_view.insert(QStringLiteral("locale"), map);
    emit changed(m_view);
}

void MockGeneralClient::setAutostartEnabled(const QString &id, bool enabled)
{
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
    emit changed(m_view);
}

void MockGeneralClient::setUserUnitEnabled(const QString &unit, bool enabled)
{
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
    emit changed(m_view);
}

void MockGeneralClient::setSharingEnabled(const QString &id, bool enabled)
{
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
    emit changed(m_view);
}

void MockGeneralClient::runBackup(const QString &destination)
{
    m_lastDestination = destination;
    QVariantMap backup = subMap(m_view, QStringLiteral("backup"));
    backup.insert(QStringLiteral("message"), QStringLiteral("Backup started"));
    backup.insert(QStringLiteral("running"), false);
    m_view.insert(QStringLiteral("backup"), backup);
    emit changed(m_view);
}

void MockGeneralClient::openRestore()
{
    ++m_restores;
    QVariantMap backup = subMap(m_view, QStringLiteral("backup"));
    backup.insert(QStringLiteral("message"), QStringLiteral("Restore opened"));
    m_view.insert(QStringLiteral("backup"), backup);
    emit changed(m_view);
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
    view.insert(QStringLiteral("login"), login);

    QVariantMap sharing;
    sharing.insert(QStringLiteral("state"), QStringLiteral("available"));
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
    storage.insert(QStringLiteral("label"), QStringLiteral("480 GB used"));
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
    view.insert(QStringLiteral("startup"), startup);

    view.insert(QStringLiteral("errors"), QVariantMap());
    m_view = view;
    emit changed(m_view);
}
