// SPDX-License-Identifier: MIT
#include "menubrokerclient.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
#include <QDBusMessage>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QVariantMap>

namespace {
constexpr auto kService = "org.dragonfruit.MenuBroker1";
constexpr auto kPath = "/org/dragonfruit/MenuBroker1";
constexpr auto kInterface = "org.dragonfruit.MenuBroker1";
constexpr int kCallTimeoutMs = 2000;

QVariantList arrayToList(const QJsonValue &value)
{
    QVariantList list;
    const QJsonArray array = value.toArray();
    list.reserve(array.size());
    for (const QJsonValue &row : array)
        list.append(row.toVariant());
    return list;
}
} // namespace

bool MenuBrokerClient::available() const
{
    const QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return false;
    QDBusConnectionInterface *iface = bus.interface();
    return iface != nullptr && iface->isServiceRegistered(QString::fromLatin1(kService));
}

QString MenuBrokerClient::call(const QString &method, const QList<QVariant> &arguments) const
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

bool MenuBrokerClient::callBool(const QString &method, const QList<QVariant> &arguments) const
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

bool MenuBrokerClient::setFocusedApp(const QString &appId) const
{
    return callBool(QStringLiteral("SetFocusedApp"), {appId});
}

bool MenuBrokerClient::setWindowStates(const QString &json) const
{
    return callBool(QStringLiteral("SetWindowStates"), {json});
}

ResolvedMenu MenuBrokerClient::resolveFocused() const
{
    return parseResolved(call(QStringLiteral("ResolveFocused"), {}));
}

ResolvedMenu MenuBrokerClient::parseResolved(const QString &json)
{
    static const ResolvedMenu invalid;
    if (json.isEmpty())
        return invalid;
    QJsonParseError error{};
    const QJsonDocument document = QJsonDocument::fromJson(json.toUtf8(), &error);
    if (error.error != QJsonParseError::NoError || !document.isObject())
        return invalid;
    const QJsonObject object = document.object();
    ResolvedMenu resolved;
    resolved.appId = object.value(QStringLiteral("appId")).toString();
    resolved.appName = object.value(QStringLiteral("appName")).toString();
    resolved.tier = object.value(QStringLiteral("tier")).toString();
    resolved.applicationMenuItems =
        arrayToList(object.value(QStringLiteral("applicationMenuItems")));
    resolved.menus = arrayToList(object.value(QStringLiteral("menus")));
    resolved.valid = true;
    return resolved;
}