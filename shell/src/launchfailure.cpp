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

void raiseDockNotice(NotificationClient *client, const QString &summary,
                     const QString &body)
{
    if (!client)
        return;
    client->notify(QStringLiteral("Dock"), summary,
                   body.isEmpty() ? QStringLiteral("The operation did not complete.")
                                  : body,
                   QStringLiteral("normal"), {}, {});
}