// SPDX-License-Identifier: MIT
#include "settingsclient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>

namespace {

const QString kService = QStringLiteral("org.dragonfruit.Settings1");
const QString kPath = QStringLiteral("/org/dragonfruit/Settings1");
const QString kInterface = QStringLiteral("org.dragonfruit.Settings1");

// Unwrap the D-Bus variant wrapper Qt adds to `v` and `a{sv}` payloads.
QVariant unwrapVariant(const QVariant &value)
{
    if (value.userType() == qMetaTypeId<QDBusVariant>())
        return value.value<QDBusVariant>().variant();
    return value;
}

} // namespace

SettingsClient::SettingsClient(QObject *parent)
    : QObject(parent)
{
    // The schema defaults make the shell self-sufficient when no daemon is on
    // the bus (the dev tool does not start settingsd). A `GetAll` snapshot
    // overrides them once the daemon answers.
    m_values = settingsSchemaDefaults();
}

SettingsClient::~SettingsClient() = default;

QVariant SettingsClient::value(const QString &key, const QVariant &fallback) const
{
    const auto it = m_values.constFind(key);
    return it == m_values.constEnd() ? fallback : it.value();
}

bool SettingsClient::boolean(const QString &key, bool fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<bool>() ? v.toBool() : fallback;
}

double SettingsClient::real(const QString &key, double fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<double>() ? v.toDouble() : fallback;
}

qlonglong SettingsClient::integer(const QString &key, qlonglong fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<qlonglong>() ? v.toLongLong() : fallback;
}

QString SettingsClient::string(const QString &key, const QString &fallback) const
{
    const QVariant v = value(key, fallback);
    return v.canConvert<QString>() ? v.toString() : fallback;
}

QStringList SettingsClient::stringList(const QString &key) const
{
    return value(key).toStringList();
}

void SettingsClient::applyValue(const QString &key, const QVariant &value)
{
    const auto it = m_values.constFind(key);
    if (it != m_values.constEnd() && it.value() == value)
        return;
    m_values.insert(key, value);
    emit changed(key, value);
}

void SettingsClient::applyValues(const QVariantMap &values)
{
    for (auto it = values.constBegin(); it != values.constEnd(); ++it)
        applyValue(it.key(), it.value());
}

void SettingsClient::setAvailable(bool available)
{
    if (m_available == available)
        return;
    m_available = available;
    emit availableChanged(available);
}

QVariantMap settingsSchemaDefaults()
{
    // Mirrors services/settingsd/src/schema.rs (SCHEMA_VERSION 1). Values are
    // typed exactly as the schema declares: d, x, b, s, as.
    QVariantMap values;
    values.insert(QStringLiteral("dock.size"), 0.5);
    values.insert(QStringLiteral("dock.magnification"), 0.5);
    values.insert(QStringLiteral("dock.position"), QStringLiteral("bottom"));
    values.insert(QStringLiteral("dock.autohide"), false);
    values.insert(QStringLiteral("dock.animateOpening"), true);
    values.insert(QStringLiteral("dock.showIndicators"), true);
    values.insert(QStringLiteral("dock.minimizeIntoTileIcon"), false);
    values.insert(QStringLiteral("dock.minimizedAnimation"), QStringLiteral("scale"));
    values.insert(QStringLiteral("dock.titlebarDoubleClick"), QStringLiteral("zoom"));
    values.insert(QStringLiteral("dock.showRecentApps"), false);
    values.insert(QStringLiteral("dock.pinned"), QStringList());
    values.insert(QStringLiteral("workspaces.count"), qlonglong(3));
    values.insert(QStringLiteral("gestures.enabled"), true);
    values.insert(QStringLiteral("gestures.spaceSwitch"), true);
    values.insert(QStringLiteral("gestures.missionControl"), true);
    values.insert(QStringLiteral("appearance.colorScheme"), QStringLiteral("auto"));
    values.insert(QStringLiteral("appearance.accent"), QString());
    values.insert(QStringLiteral("accessibility.reduceMotion"), false);
    values.insert(QStringLiteral("input.repeatDelay"), qlonglong(200));
    values.insert(QStringLiteral("input.repeatRate"), qlonglong(25));
    return values;
}

// --- DbusSettingsClient ----------------------------------------------------

DbusSettingsClient::DbusSettingsClient(QObject *parent)
    : SettingsClient(parent)
    , m_service(kService)
    , m_path(kPath)
    , m_interface(kInterface)
{
    // Watch the well-known name itself. `QDBusConnectionInterface`'s
    // serviceRegistered/serviceUnregistered only fire for a connection's
    // unique name, so a settingsd owned by another process never triggered
    // them: the shell resynced once via the constructor's GetAll and then
    // never after a restart (T-08.3 found this live). A QDBusServiceWatcher
    // follows the well-known name across owners.
    m_watcher = new QDBusServiceWatcher(m_service, QDBusConnection::sessionBus(),
                                        QDBusServiceWatcher::WatchForRegistration
                                            | QDBusServiceWatcher::WatchForUnregistration,
                                        this);
    connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
            &DbusSettingsClient::onServiceRegistered);
    connect(m_watcher, &QDBusServiceWatcher::serviceUnregistered, this,
            &DbusSettingsClient::onServiceUnregistered);
    subscribeToChanges();
    setAvailable(isAvailable());
    if (m_available)
        refresh();
}

bool DbusSettingsClient::isAvailable() const
{
    QDBusConnectionInterface *iface = QDBusConnection::sessionBus().interface();
    return iface && iface->isServiceRegistered(m_service);
}

void DbusSettingsClient::subscribeToChanges()
{
    // The `Changed` signal is the only notification (no polling). Registered
    // once; it follows the well-known name across daemon restarts.
    QDBusConnection::sessionBus().connect(m_service, m_path, m_interface,
                                          QStringLiteral("Changed"), this,
                                          SLOT(onRemoteChanged(QString,QDBusVariant)));
}

void DbusSettingsClient::onRemoteChanged(const QString &key, const QDBusVariant &value)
{
    applyValue(key, unwrapVariant(QVariant::fromValue(value)));
}

void DbusSettingsClient::onServiceRegistered(const QString &name)
{
    if (name != m_service)
        return;
    setAvailable(true);
    // Re-sync: the daemon may have a persisted value the shell has not seen.
    refresh();
}

void DbusSettingsClient::onServiceUnregistered(const QString &name)
{
    if (name != m_service)
        return;
    // The daemon restart resets its owner; keep the last known values so the
    // desktop does not lose a setting visually (T-08 restart behavior).
    setAvailable(false);
}

void DbusSettingsClient::refresh()
{
    if (!isAvailable()) {
        emit refreshed();
        return;
    }
    auto *call = new QDBusInterface(m_service, m_path, m_interface,
                                    QDBusConnection::sessionBus(), this);
    auto *watcher = new QDBusPendingCallWatcher(call->asyncCall(QStringLiteral("GetAll")), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, call]() {
                const QDBusPendingReply<QVariantMap> reply = *watcher;
                if (reply.isValid()) {
                    QVariantMap values;
                    const QVariantMap raw = reply.value();
                    for (auto it = raw.constBegin(); it != raw.constEnd(); ++it)
                        values.insert(it.key(), unwrapVariant(it.value()));
                    applyValues(values);
                } else {
                    qWarning() << "dragonfruit-shell: settings GetAll failed:"
                               << watcher->reply().errorMessage();
                }
                emit refreshed();
                call->deleteLater();
                watcher->deleteLater();
            });
}

void DbusSettingsClient::set(const QString &key, const QVariant &value)
{
    // Optimistic local apply so the Dock reacts on the same event-loop turn;
    // the daemon's `Changed` echo then de-duplicates. When no daemon is on the
    // bus this is the only effect (in-memory for the session).
    applyValue(key, value);
    if (!isAvailable())
        return;
    auto *call = new QDBusInterface(m_service, m_path, m_interface,
                                    QDBusConnection::sessionBus(), this);
    auto *watcher = new QDBusPendingCallWatcher(
        call->asyncCall(QStringLiteral("Set"), key,
                        QVariant::fromValue(QDBusVariant(value))),
        this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this, watcher, call]() {
                const QDBusMessage reply = watcher->reply();
                if (reply.type() == QDBusMessage::ErrorMessage)
                    qWarning() << "dragonfruit-shell: settings Set failed:"
                               << reply.errorMessage();
                call->deleteLater();
                watcher->deleteLater();
            });
}

// --- MockSettingsClient ----------------------------------------------------

MockSettingsClient::MockSettingsClient(QObject *parent)
    : SettingsClient(parent)
{
    m_available = true;
}

void MockSettingsClient::refresh()
{
    emit refreshed();
}

void MockSettingsClient::set(const QString &key, const QVariant &value)
{
    applyValue(key, value);
}