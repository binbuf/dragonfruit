// SPDX-License-Identifier: MIT
#include "MenuPublisher.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>

namespace {
constexpr auto kService = "org.dragonfruit.MenuBroker1";
constexpr auto kPath = "/org/dragonfruit/MenuBroker1";
constexpr auto kInterface = "org.dragonfruit.MenuBroker1";
constexpr int kCallTimeoutMs = 2000;

bool brokerAvailable()
{
    const QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return false;
    QDBusConnectionInterface *iface = bus.interface();
    return iface != nullptr && iface->isServiceRegistered(QString::fromLatin1(kService));
}

bool call(const QString &method, const QList<QVariant> &arguments)
{
    if (!brokerAvailable())
        return false;
    QDBusMessage message = QDBusMessage::createMethodCall(
        QString::fromLatin1(kService), QString::fromLatin1(kPath),
        QString::fromLatin1(kInterface), method);
    message.setArguments(arguments);
    const QDBusMessage reply =
        QDBusConnection::sessionBus().call(message, QDBus::Block, kCallTimeoutMs);
    if (reply.type() != QDBusMessage::ReplyMessage || reply.arguments().isEmpty())
        return false;
    return reply.arguments().first().toBool();
}
} // namespace

MenuPublisher::MenuPublisher(QObject *parent)
    : QObject(parent)
{
}

bool MenuPublisher::publish(const QString &appId, const QString &modelJson)
{
    if (appId.isEmpty() || modelJson.isEmpty())
        return false;
    return call(QStringLiteral("Publish"), {appId, modelJson});
}

bool MenuPublisher::withdraw(const QString &appId)
{
    if (appId.isEmpty())
        return false;
    return call(QStringLiteral("Withdraw"), {appId});
}
