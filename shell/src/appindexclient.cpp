// SPDX-License-Identifier: MIT
#include "appindexclient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusMessage>
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
} // namespace

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

QList<DesktopEntry> AppIndexClient::enumerate() const
{
    QList<DesktopEntry> entries;
    const QString json = call(QStringLiteral("Enumerate"), {});
    if (json.isEmpty())
        return entries;
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isArray())
        return entries;
    const QJsonArray array = document.array();
    entries.reserve(array.size());
    for (const QJsonValue &value : array) {
        if (!value.isObject())
            continue;
        const DesktopEntry entry = entryFromObject(value.toObject());
        if (entry.valid)
            entries.append(entry);
    }
    return entries;
}

QString AppIndexClient::iconPath(const QString &name, int size) const
{
    return call(QStringLiteral("IconPath"), {name, size});
}