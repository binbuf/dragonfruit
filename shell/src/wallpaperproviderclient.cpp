// SPDX-License-Identifier: GPL-3.0-only
#include "wallpaperproviderclient.h"

#include <QDBusConnection>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>
#include <QDBusVariant>
#include <QDebug>

namespace {

QString unwrapText(const QVariant &raw)
{
    QVariant value = raw;
    if (value.userType() == qMetaTypeId<QDBusVariant>())
        value = value.value<QDBusVariant>().variant();
    return value.toString();
}

} // namespace

WallpaperProviderClient::WallpaperProviderClient(const QString &service,
                                                 const QString &path,
                                                 const QString &interface,
                                                 int readTimeoutMs, QObject *parent)
    : QObject(parent)
    , m_service(service)
    , m_path(path)
    , m_interface(interface)
    , m_readTimeoutMs(readTimeoutMs)
{
}

WallpaperProviderClient::~WallpaperProviderClient() = default;

void WallpaperProviderClient::start()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    if (!m_watcher) {
        m_watcher = new QDBusServiceWatcher(
            m_service, bus,
            QDBusServiceWatcher::WatchForRegistration
                | QDBusServiceWatcher::WatchForUnregistration,
            this);
        // A provider (re)start re-reads its resolved defaults; an
        // unregistration keeps the last known values so a restart leaves the
        // desktop unchanged.
        connect(m_watcher, &QDBusServiceWatcher::serviceRegistered, this,
                [this](const QString &) { refresh(); });
    }
    if (!m_subscribed) {
        // The standard property notification covers `DefaultSource` and
        // `BuiltinDefaultSource`; the contract signal is a belt-and-braces
        // refresh for a first fill.
        bus.connect(m_service, m_path,
                    QStringLiteral("org.freedesktop.DBus.Properties"),
                    QStringLiteral("PropertiesChanged"), this,
                    SLOT(onPropertiesChanged(QString,QVariantMap,QStringList)));
        bus.connect(m_service, m_path, m_interface, QStringLiteral("ItemsChanged"),
                    this, SLOT(onItemsChanged()));
        m_subscribed = true;
    }
    refresh();
}

void WallpaperProviderClient::refresh()
{
    // T338: one asynchronous `Properties.GetAll` with a short bound instead of
    // two blocking `Get` calls. A burst of signals coalesces into one
    // follow-up read; the provider's reply is cached in the two members.
    if (m_refreshPending) {
        m_refreshQueued = true;
        return;
    }
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    m_refreshPending = true;
    QDBusMessage call = QDBusMessage::createMethodCall(
        m_service, m_path, QStringLiteral("org.freedesktop.DBus.Properties"),
        QStringLiteral("GetAll"));
    call << m_interface;
    auto *watcher = new QDBusPendingCallWatcher(
        bus.asyncCall(call, m_readTimeoutMs), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            [this](QDBusPendingCallWatcher *pending) {
                pending->deleteLater();
                m_refreshPending = false;
                const QDBusPendingReply<QVariantMap> reply = *pending;
                if (reply.isValid())
                    applyProperties(reply.value());
                // A timeout or an absent provider is a normal state: the
                // reply is an error and both properties keep their last
                // known value.
                if (m_refreshQueued) {
                    m_refreshQueued = false;
                    refresh();
                }
            });
}

void WallpaperProviderClient::onItemsChanged()
{
    // A first fill or a refreshed catalogue may have moved DefaultSource; the
    // read is coalesced with any other pending request.
    refresh();
}

void WallpaperProviderClient::onPropertiesChanged(const QString &interface,
                                                  const QVariantMap &values,
                                                  const QStringList &invalidated)
{
    Q_UNUSED(interface);
    bool moved = false;
    if (values.contains(QStringLiteral("DefaultSource")))
        applyProperty(QStringLiteral("DefaultSource"),
                      values.value(QStringLiteral("DefaultSource")), &moved);
    if (values.contains(QStringLiteral("BuiltinDefaultSource")))
        applyProperty(QStringLiteral("BuiltinDefaultSource"),
                      values.value(QStringLiteral("BuiltinDefaultSource")), &moved);
    // An invalidated property has no value in the signal; re-read it.
    if (invalidated.contains(QStringLiteral("DefaultSource"))
        || invalidated.contains(QStringLiteral("BuiltinDefaultSource"))) {
        refresh();
    }
    if (moved)
        emit changed();
}

void WallpaperProviderClient::applyProperties(const QVariantMap &properties)
{
    bool moved = false;
    if (properties.contains(QStringLiteral("DefaultSource")))
        applyProperty(QStringLiteral("DefaultSource"),
                      properties.value(QStringLiteral("DefaultSource")), &moved);
    if (properties.contains(QStringLiteral("BuiltinDefaultSource")))
        applyProperty(QStringLiteral("BuiltinDefaultSource"),
                      properties.value(QStringLiteral("BuiltinDefaultSource")), &moved);
    if (moved)
        emit changed();
}

void WallpaperProviderClient::applyProperty(const QString &name,
                                            const QVariant &value, bool *changed)
{
    const QString text = unwrapText(value);
    QString *target = nullptr;
    if (name == QLatin1String("DefaultSource"))
        target = &m_defaultSource;
    else if (name == QLatin1String("BuiltinDefaultSource"))
        target = &m_builtinDefaultSource;
    if (!target || *target == text)
        return;
    *target = text;
    if (changed)
        *changed = true;
}
