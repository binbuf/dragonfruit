// SPDX-License-Identifier: MIT
#include "launchfailure.h"

#include "notificationclient.h"

void raiseDockLaunchFailure(NotificationClient *client, const QString &appName,
                            const QString &reason)
{
    if (!client)
        return;
    const QString name = appName.isEmpty() ? QStringLiteral("app") : appName;
    client->notify(QStringLiteral("Dock"),
                   QStringLiteral("Could not launch %1").arg(name),
                   reason.isEmpty() ? QStringLiteral("The app did not start.")
                                    : reason,
                   QStringLiteral("normal"), {}, {});
}