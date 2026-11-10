// SPDX-License-Identifier: MIT
#include "SettingsDeepLink.h"

#include <QDBusConnection>

SettingsDeepLink::SettingsDeepLink(QObject *parent)
    : QObject(parent)
{
}

bool SettingsDeepLink::registerService()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    if (!bus.isConnected())
        return false;
    if (!bus.registerService(QStringLiteral("org.dragonfruit.SettingsApp1")))
        return false;
    return bus.registerObject(QStringLiteral("/org/dragonfruit/SettingsApp1"),
                              this, QDBusConnection::ExportAllSlots);
}

void SettingsDeepLink::Open(const QString &url)
{
    if (!url.isEmpty())
        emit requested(url);
}
