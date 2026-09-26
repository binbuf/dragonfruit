// SPDX-License-Identifier: MIT
#include "trayclient.h"

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

TrayItem itemFromObject(const QJsonObject &object)
{
    TrayItem item;
    item.name = object.value(QStringLiteral("name")).toString();
    item.id = object.value(QStringLiteral("id")).toString();
    item.title = object.value(QStringLiteral("title")).toString();
    item.iconName = object.value(QStringLiteral("iconName")).toString();
    item.iconPath = object.value(QStringLiteral("iconPath")).toString();
    item.tooltip = object.value(QStringLiteral("tooltip")).toString();
    item.menuPath = object.value(QStringLiteral("menuPath")).toString();
    item.itemIsMenu = object.value(QStringLiteral("itemIsMenu")).toBool();
    item.needsAttention = object.value(QStringLiteral("needsAttention")).toBool();
    item.hasPixmap = object.value(QStringLiteral("hasPixmap")).toBool();
    item.valid = !item.name.isEmpty();
    return item;
}
} // namespace

bool TrayClient::available() const
{
    const QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return false;
    QDBusConnectionInterface *iface = bus.interface();
    return iface != nullptr && iface->isServiceRegistered(QString::fromLatin1(kService));
}

QString TrayClient::call(const QString &method, const QList<QVariant> &arguments) const
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

bool TrayClient::callBool(const QString &method, const QList<QVariant> &arguments) const
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

QList<TrayItem> TrayClient::parseItems(const QString &json)
{
    QList<TrayItem> items;
    if (json.isEmpty())
        return items;
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isArray())
        return items;
    const QJsonArray array = document.array();
    items.reserve(array.size());
    for (const QJsonValue &value : array) {
        if (!value.isObject())
            continue;
        const TrayItem item = itemFromObject(value.toObject());
        if (item.valid)
            items.append(item);
    }
    return items;
}

QVariantList TrayClient::parseMenu(const QString &json)
{
    QVariantList rows;
    if (json.isEmpty())
        return rows;
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isArray())
        return rows;
    return document.toVariant().toList();
}

QList<TrayItem> TrayClient::items() const
{
    return parseItems(call(QStringLiteral("TrayItems"), {}));
}

QVariantList TrayClient::menu(const QString &name) const
{
    return parseMenu(call(QStringLiteral("TrayMenu"), {name}));
}

bool TrayClient::menuEvent(const QString &name, int id) const
{
    return callBool(QStringLiteral("TrayMenuEvent"), {name, id});
}

bool TrayClient::activate(const QString &name, const QString &kind) const
{
    return callBool(QStringLiteral("TrayActivate"), {name, kind});
}