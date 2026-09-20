// SPDX-License-Identifier: MIT
#include "dockmodel.h"

#include "desktopentry.h"

#include <QSet>
#include <QVariantMap>

namespace {

QString withoutDesktopSuffix(QString id)
{
    if (id.endsWith(QLatin1String(".desktop")))
        id.chop(8);
    return id;
}

} // namespace

QString displayNameForIdentity(const QString &identity)
{
    if (identity.isEmpty())
        return QStringLiteral("Unknown");
    const QString id = withoutDesktopSuffix(identity);
    const qsizetype dot = id.lastIndexOf(QLatin1Char('.'));
    return dot >= 0 ? id.mid(dot + 1) : id;
}

double dockLaunchBouncePhase(qint64 elapsedMs)
{
    if (elapsedMs < 0 || elapsedMs >= kLaunchBounceMs)
        return -1.0;
    const qint64 hop = kLaunchBounceMs / 3;
    return static_cast<double>(elapsedMs % hop) / static_cast<double>(hop);
}

double dockAttentionBouncePhase(qint64 elapsedMs)
{
    if (elapsedMs < 0)
        return 0.0;
    return static_cast<double>(elapsedMs % kAttentionBounceHopMs)
           / static_cast<double>(kAttentionBounceHopMs);
}

QVariantList buildDockEntries(const QStringList &pinnedIds, const DesktopEntryIndex &index,
                              const QVariantList &running,
                              const QHash<QString, QString> &launchStates)
{
    QList<QVariantMap> minimizedEntries;
    QList<QVariantMap> runningApps;
    for (const QVariant &value : running) {
        const QVariantMap map = value.toMap();
        if (map.value(QStringLiteral("kind")).toString() == QLatin1String("minimized"))
            minimizedEntries.append(map);
        else
            runningApps.append(map);
    }

    QSet<int> usedRunning;
    QVariantList entries;

    for (const QString &pinId : pinnedIds) {
        const DesktopEntry entry = index.byId(pinId);
        const QString entryId = entry.valid ? entry.id : pinId;

        QVariantMap merged;
        int windows = 0;
        bool anyMinimized = false;
        QString matchAppId;
        bool matched = false;
        for (int i = 0; i < runningApps.size(); ++i) {
            if (usedRunning.contains(i))
                continue;
            const QVariantMap &candidate = runningApps.at(i);
            const QString appId = candidate.value(QStringLiteral("appId")).toString();
            if (appId.isEmpty())
                continue;
            const DesktopEntry resolved = index.resolve(appId);
            const bool sameApp = appId == pinId || appId == entryId
                                 || (resolved.valid && resolved.id == entryId);
            if (!sameApp)
                continue;
            usedRunning.insert(i);
            matched = true;
            if (matchAppId.isEmpty())
                matchAppId = appId;
            windows += candidate.value(QStringLiteral("windows")).toInt();
            if (candidate.value(QStringLiteral("minimized")).toBool())
                anyMinimized = true;
        }

        merged.insert(QStringLiteral("id"), pinId);
        merged.insert(QStringLiteral("appId"), matched ? matchAppId : entryId);
        merged.insert(QStringLiteral("desktopId"), entryId);
        merged.insert(QStringLiteral("name"),
                      entry.valid ? entry.name : displayNameForIdentity(pinId));
        merged.insert(QStringLiteral("icon"), entry.icon);
        merged.insert(QStringLiteral("kind"), QStringLiteral("pinned"));
        merged.insert(QStringLiteral("pinned"), true);
        merged.insert(QStringLiteral("missing"), !entry.valid);
        merged.insert(QStringLiteral("running"), matched);
        merged.insert(QStringLiteral("windows"), windows);
        merged.insert(QStringLiteral("minimized"), matched && anyMinimized);
        const QString launch = launchStates.value(pinId);
        if (!launch.isEmpty())
            merged.insert(QStringLiteral("launch"), launch);
        entries.append(merged);
    }

    for (int i = 0; i < runningApps.size(); ++i) {
        if (usedRunning.contains(i))
            continue;
        QVariantMap entry = runningApps.at(i);
        entry.insert(QStringLiteral("kind"), QStringLiteral("temporary"));
        entry.insert(QStringLiteral("pinned"), false);
        entry.insert(QStringLiteral("missing"), false);
        entries.append(entry);
    }

    for (const QVariantMap &minimized : minimizedEntries)
        entries.append(minimized);

    return entries;
}
