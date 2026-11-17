// SPDX-License-Identifier: MIT
#include "MenuPublisher.h"

#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusServiceWatcher>
#include <QDebug>

namespace {
constexpr auto kService = "org.dragonfruit.MenuBroker1";
constexpr auto kPath = "/org/dragonfruit/MenuBroker1";
constexpr auto kInterface = "org.dragonfruit.MenuBroker1";
constexpr auto kPublisherPath = "/org/dragonfruit/MenuPublisher1";
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

MenuPublisher::~MenuPublisher()
{
    if (!m_appId.isEmpty() && m_published)
        call(QStringLiteral("Withdraw"), {m_appId});
    if (m_registered) {
        QDBusConnection::sessionBus().unregisterObject(
            QString::fromLatin1(kPublisherPath));
    }
}

void MenuPublisher::ensureRegistered()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return;
    if (!m_registered) {
        m_registered = bus.registerObject(QString::fromLatin1(kPublisherPath), this,
                                          QDBusConnection::ExportAllSlots);
        if (!m_registered) {
            qWarning() << "dragonfruit-settings: could not export the menu "
                          "activation endpoint at"
                       << kPublisherPath;
        }
    }
    if (!m_brokerWatcher) {
        // A restarted broker starts with an empty publication table. Watch its
        // owner and re-publish the cached model as soon as the new owner is
        // ready; the shell also re-resolves on the same transition.
        m_brokerWatcher = new QDBusServiceWatcher(
            QString::fromLatin1(kService), bus,
            QDBusServiceWatcher::WatchForRegistration
                | QDBusServiceWatcher::WatchForUnregistration,
            this);
        connect(m_brokerWatcher, &QDBusServiceWatcher::serviceRegistered, this,
                [this](const QString &) { republish(); });
        connect(m_brokerWatcher, &QDBusServiceWatcher::serviceUnregistered, this,
                [this](const QString &) { m_published = false; });
    }
}

bool MenuPublisher::publish(const QString &appId, const QString &modelJson)
{
    if (appId.isEmpty() || modelJson.isEmpty())
        return false;
    ensureRegistered();
    m_appId = appId;
    m_modelJson = modelJson;
    m_published = call(QStringLiteral("Publish"), {appId, modelJson});
    return m_published;
}

bool MenuPublisher::withdraw(const QString &appId)
{
    if (appId.isEmpty())
        return false;
    if (appId == m_appId) {
        m_modelJson.clear();
        m_published = false;
    }
    return call(QStringLiteral("Withdraw"), {appId});
}

void MenuPublisher::republish()
{
    if (m_appId.isEmpty() || m_modelJson.isEmpty())
        return;
    m_published = call(QStringLiteral("Publish"), {m_appId, m_modelJson});
}

bool MenuPublisher::Activate(const QString &appId, const QString &action,
                             const QString &itemJson, qulonglong generation,
                             uint windowId)
{
    // The broker addresses the unique owner that published this model. The
    // app id check rejects a request that outlived a re-key or a replacement
    // publisher on the same connection.
    if (appId != m_appId || action.isEmpty())
        return false;
    m_activationPending = true;
    m_activationHandled = false;
    emit activationRequested(action, itemJson, generation, windowId);
    m_activationPending = false;
    return m_activationHandled;
}

void MenuPublisher::completeActivation(bool handled)
{
    if (m_activationPending)
        m_activationHandled = handled;
}
