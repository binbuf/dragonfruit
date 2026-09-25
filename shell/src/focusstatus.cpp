// SPDX-License-Identifier: MIT
#include "focusstatus.h"

#include <QCoreApplication>

QVariantMap focusStatusItem(const QVariantMap &policy)
{
    const QString mode = policy.value(QStringLiteral("mode")).toString();
    const int batched = policy.value(QStringLiteral("batchedCount")).toInt();
    const bool active = mode == QLatin1String("focus") || mode == QLatin1String("dnd");
    const bool dnd = mode == QLatin1String("dnd");

    QString name = dnd ? QCoreApplication::translate("ShellController", "Do Not Disturb")
                       : QCoreApplication::translate("ShellController", "Focus");
    if (active && batched > 0) {
        name = QCoreApplication::translate("ShellController", "%1 (%2 notifications silenced)")
                   .arg(name)
                   .arg(batched);
    }

    QVariantMap item;
    item.insert(QStringLiteral("id"), QStringLiteral("focus"));
    item.insert(QStringLiteral("icon"), QStringLiteral("focus"));
    // The batch count is the one number the bar can surface; hide it at zero
    // so an active Focus with nothing silenced stays a clean crescent.
    item.insert(QStringLiteral("label"),
                (active && batched > 0) ? QString::number(batched) : QString());
    item.insert(QStringLiteral("accessibleName"), name);
    item.insert(QStringLiteral("available"), active);
    item.insert(QStringLiteral("enabled"), active);
    item.insert(QStringLiteral("selected"), dnd);
    item.insert(QStringLiteral("level"), 0.8);
    return item;
}
