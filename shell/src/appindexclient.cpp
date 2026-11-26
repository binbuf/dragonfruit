// SPDX-License-Identifier: GPL-3.0-or-later
#include "appindexclient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QDBusPendingCall>
#include <QDBusPendingCallWatcher>
#include <QDBusPendingReply>
#include <QDBusServiceWatcher>

#include <optional>
#include <utility>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>

namespace {
constexpr auto kService = "org.dragonfruit.AppIndex1";
constexpr auto kPath = "/org/dragonfruit/AppIndex1";
constexpr auto kInterface = "org.dragonfruit.AppIndex1";
constexpr int kCallTimeoutMs = 2000;

DesktopEntry entryFromObject(const QJsonObject &object)
{
    DesktopEntry entry;
    entry.id = object.value(QStringLiteral("desktopId")).toString();
    entry.name = object.value(QStringLiteral("name")).toString();
    entry.icon = object.value(QStringLiteral("icon")).toString();
    entry.iconPath = object.value(QStringLiteral("iconPath")).toString();
    entry.exec = object.value(QStringLiteral("exec")).toString();
    entry.terminal = object.value(QStringLiteral("terminal")).toBool();
    entry.noDisplay = object.value(QStringLiteral("noDisplay")).toBool();
    entry.startupWmClass = object.value(QStringLiteral("startupWmClass")).toString();
    const QJsonArray categories = object.value(QStringLiteral("categories")).toArray();
    for (const QJsonValue &value : categories)
        entry.categories << value.toString();
    entry.valid = object.value(QStringLiteral("valid")).toBool(false);
    return entry;
}

QString catalogCall(const QString &owner, const QString &method,
                    const QList<QVariant> &arguments = {})
{
    QDBusMessage request = QDBusMessage::createMethodCall(
        owner, QString::fromLatin1(kPath), QString::fromLatin1(kInterface), method);
    request.setArguments(arguments);
    const QDBusMessage reply =
        QDBusConnection::sessionBus().call(request, QDBus::Block, kCallTimeoutMs);
    if (reply.type() != QDBusMessage::ReplyMessage || reply.arguments().isEmpty())
        return {};
    return reply.arguments().first().toString();
}

std::optional<DesktopEntryIndex> readCatalog(const QString &owner)
{
    const QJsonDocument document =
        QJsonDocument::fromJson(catalogCall(owner, QStringLiteral("Enumerate")).toUtf8());
    if (!document.isArray())
        return std::nullopt;
    const QJsonArray array = document.array();
    QList<DesktopEntry> entries;
    entries.reserve(array.size());
    for (const QJsonValue &value : array) {
        if (!value.isObject())
            return std::nullopt;
        const DesktopEntry entry = entryFromObject(value.toObject());
        if (!entry.valid || entry.id.isEmpty())
            return std::nullopt;
        entries.append(entry);
    }
    DesktopEntryIndex index;
    index.loadFromRecords(entries);
    return index;
}
} // namespace

namespace {

QString stripDesktopSuffix(QString id)
{
    if (id.endsWith(QLatin1String(".desktop"), Qt::CaseInsensitive))
        id.chop(8);
    return id;
}

bool idsMatch(const QString &left, const QString &right)
{
    if (left.isEmpty() || right.isEmpty())
        return false;
    if (left.compare(right, Qt::CaseInsensitive) == 0)
        return true;
    return stripDesktopSuffix(left).compare(stripDesktopSuffix(right),
                                            Qt::CaseInsensitive) == 0;
}

} // namespace

QList<AppMenuWindow> parseAppMenuWindows(const QString &json)
{
    QList<AppMenuWindow> windows;
    if (json.isEmpty())
        return windows;
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isArray())
        return windows;
    for (const QJsonValue &value : document.array()) {
        if (!value.isObject())
            continue;
        const QJsonObject object = value.toObject();
        const double rawId = object.value(QStringLiteral("windowId")).toDouble(-1);
        if (rawId <= 0 || rawId > 4294967295.0)
            continue;
        AppMenuWindow window;
        window.windowId = static_cast<quint32>(rawId);
        window.appId = object.value(QStringLiteral("appId")).toString();
        window.owner = object.value(QStringLiteral("owner")).toString();
        window.menuPath = object.value(QStringLiteral("menuPath")).toString();
        windows.append(window);
    }
    return windows;
}

const AppMenuWindow *findAppMenuWindow(const QList<AppMenuWindow> &windows,
                                       const QString &rawAppId,
                                       const QString &desktopId)
{
    if (rawAppId.isEmpty() && desktopId.isEmpty())
        return nullptr;
    for (const AppMenuWindow &window : windows) {
        if (idsMatch(window.appId, rawAppId))
            return &window;
    }
    for (const AppMenuWindow &window : windows) {
        if (idsMatch(window.appId, desktopId))
            return &window;
    }
    return nullptr;
}

bool AppIndexClient::available() const
{
    const QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return false;
    QDBusConnectionInterface *iface = bus.interface();
    return iface != nullptr && iface->isServiceRegistered(QString::fromLatin1(kService));
}

QString AppIndexClient::call(const QString &method, const QList<QVariant> &arguments) const
{
    if (!available())
        return QString();
    QDBusInterface iface(QString::fromLatin1(kService), QString::fromLatin1(kPath),
                         QString::fromLatin1(kInterface), QDBusConnection::sessionBus());
    iface.setTimeout(kCallTimeoutMs);
    const QDBusMessage reply = iface.callWithArgumentList(QDBus::Block, method, arguments);
    if (reply.type() != QDBusMessage::ReplyMessage || reply.arguments().isEmpty())
        return QString();
    return reply.arguments().first().toString();
}

bool AppIndexClient::callBool(const QString &method, const QList<QVariant> &arguments) const
{
    if (!available())
        return false;
    QDBusInterface iface(QString::fromLatin1(kService), QString::fromLatin1(kPath),
                         QString::fromLatin1(kInterface), QDBusConnection::sessionBus());
    iface.setTimeout(kCallTimeoutMs);
    const QDBusMessage reply = iface.callWithArgumentList(QDBus::Block, method, arguments);
    if (reply.type() != QDBusMessage::ReplyMessage || reply.arguments().isEmpty())
        return false;
    return reply.arguments().first().toBool();
}

DesktopEntry AppIndexClient::parseRecord(const QString &json)
{
    static const DesktopEntry invalid;
    if (json.isEmpty())
        return invalid;
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isObject())
        return invalid;
    return entryFromObject(document.object());
}

DesktopEntry AppIndexClient::resolve(const QString &identity) const
{
    return parseRecord(call(QStringLiteral("Resolve"), {identity}));
}

DesktopEntry AppIndexClient::resolveWindow(const QString &appId, const QString &instance,
                                           const QString &wmClass) const
{
    return parseRecord(call(QStringLiteral("ResolveWindow"), {appId, instance, wmClass}));
}

QString AppIndexClient::iconPath(const QString &name, int size) const
{
    return call(QStringLiteral("IconPath"), {name, size});
}

bool AppIndexClient::focusWindow(const QString &windowId, const QString &appId) const
{
    bool ok = false;
    const uint id = windowId.toUInt(&ok);
    if (!ok)
        return false;
    return callBool(QStringLiteral("FocusWindow"), {id, appId});
}

QList<AppMenuWindow> AppIndexClient::appMenuWindows() const
{
    return parseAppMenuWindows(call(QStringLiteral("AppMenuWindows"), {}));
}

QVariantList AppIndexClient::parseMenus(const QString &json)
{
    QVariantList menus;
    if (json.isEmpty())
        return menus;
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isArray())
        return menus;
    const QJsonArray array = document.array();
    menus.reserve(array.size());
    for (const QJsonValue &value : array)
        menus.append(value.toVariant());
    return menus;
}

QVariantList AppIndexClient::windowMenu(const QString &windowId) const
{
    bool ok = false;
    const uint id = windowId.toUInt(&ok);
    if (!ok)
        return {};
    return parseMenus(call(QStringLiteral("WindowMenu"), {id}));
}

QDBusPendingCall AppIndexClient::windowMenuEvent(quint32 windowId, int id) const
{
    QDBusMessage message = QDBusMessage::createMethodCall(
        QString::fromLatin1(kService), QString::fromLatin1(kPath),
        QString::fromLatin1(kInterface), QStringLiteral("WindowMenuEvent"));
    message.setArguments({windowId, id});
    return QDBusConnection::sessionBus().asyncCall(message, kCallTimeoutMs);
}

AppIndexCatalog::AppIndexCatalog(QObject *parent)
    : QObject(parent)
{
    m_worker.setMaxThreadCount(1);
}

AppIndexCatalog::~AppIndexCatalog()
{
    m_worker.waitForDone();
}

void AppIndexCatalog::start()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;

    const QString service = QString::fromLatin1(kService);
    bus.connect(service, QString::fromLatin1(kPath), QString::fromLatin1(kInterface),
                QStringLiteral("Changed"), this, SLOT(onChanged(QString)));
    auto *watcher = new QDBusServiceWatcher(
        service, bus, QDBusServiceWatcher::WatchForOwnerChange, this);
    connect(watcher, &QDBusServiceWatcher::serviceOwnerChanged, this,
            [this](const QString &, const QString &, const QString &owner) {
                setOwner(owner);
            });

    QDBusMessage request = QDBusMessage::createMethodCall(
        QStringLiteral("org.freedesktop.DBus"), QStringLiteral("/org/freedesktop/DBus"),
        QStringLiteral("org.freedesktop.DBus"), QStringLiteral("GetNameOwner"));
    request.setArguments({service});
    auto *initial = new QDBusPendingCallWatcher(bus.asyncCall(request, kCallTimeoutMs), this);
    connect(initial, &QDBusPendingCallWatcher::finished, this,
            [this, revision = m_ownerRevision](QDBusPendingCallWatcher *call) {
                const QDBusPendingReply<QString> reply = *call;
                if (revision == m_ownerRevision && reply.isValid())
                    setOwner(reply.value());
                call->deleteLater();
            });
}

void AppIndexCatalog::setOwner(const QString &owner)
{
    // Every owner notification supersedes the initial lookup, including
    // an unregister event received before its registration was observed.
    ++m_ownerRevision;
    m_owner = owner;
    m_subscribed = false;
    m_hasCurrentSnapshot = false;
    m_refreshPending = false;
    emit ownerChanged();
    refresh();
}

void AppIndexCatalog::refreshIfNeeded()
{
    if (!m_refreshInFlight && (!m_subscribed || !m_hasCurrentSnapshot))
        refresh();
}

void AppIndexCatalog::onChanged(const QString &interests)
{
    Q_UNUSED(interests);
    refresh();
}

void AppIndexCatalog::refresh()
{
    if (!available())
        return;
    if (m_refreshInFlight) {
        m_refreshPending = true;
        return;
    }
    m_refreshInFlight = true;
    m_worker.start([this, owner = m_owner, revision = m_ownerRevision,
                    subscribed = m_subscribed, previous = m_index.entries()]() {
        // Subscribe before enumerating, on the same bus connection. Calls
        // use the unique name so a slow old owner cannot modify the new one.
        const bool subscribedNow = subscribed
            || !catalogCall(owner, QStringLiteral("Subscribe"),
                            {QStringLiteral("identity")}).isEmpty();
        auto snapshot = readCatalog(owner);
        const bool valid = snapshot.has_value();
        if (snapshot && snapshot->entries() == previous)
            snapshot.reset();
        QMetaObject::invokeMethod(this,
            [this, revision, subscribedNow, valid, snapshot = std::move(snapshot)]() mutable {
                if (revision == m_ownerRevision) {
                    m_subscribed = subscribedNow;
                    m_hasCurrentSnapshot = valid;
                    if (snapshot) {
                        m_index = std::move(*snapshot);
                        emit catalogChanged();
                    }
                }
                m_refreshInFlight = false;
                if (std::exchange(m_refreshPending, false))
                    refresh();
            }, Qt::QueuedConnection);
    });
}
