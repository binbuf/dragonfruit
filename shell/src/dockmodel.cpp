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

QVariantList buildRecentEntries(const QStringList &recentIds, const QStringList &pinnedIds,
                                const QVariantList &running, const DesktopEntryIndex &index,
                                int limit)
{
    // Identities already represented on the Dock: pinned pins (resolved and
    // raw) and the running projection (resolved and raw app id).
    QSet<QString> represented;
    for (const QString &pinId : pinnedIds) {
        represented.insert(pinId);
        const DesktopEntry resolved = index.resolve(pinId);
        if (resolved.valid)
            represented.insert(resolved.id);
    }
    for (const QVariant &value : running) {
        const QVariantMap map = value.toMap();
        if (map.value(QStringLiteral("kind")).toString() == QLatin1String("minimized"))
            continue;
        const QString appId = map.value(QStringLiteral("appId")).toString();
        if (appId.isEmpty())
            continue;
        represented.insert(appId);
        const DesktopEntry resolved = index.resolve(appId);
        if (resolved.valid)
            represented.insert(resolved.id);
    }

    QVariantList out;
    QSet<QString> used;
    for (const QString &recentId : recentIds) {
        if (out.size() >= limit)
            break;
        if (recentId.isEmpty() || represented.contains(recentId))
            continue;
        const DesktopEntry entry = index.resolve(recentId);
        // A recent that no longer resolves is not suggested; it is not a
        // pinned identity, so it must never render as "not found".
        if (!entry.valid)
            continue;
        if (represented.contains(entry.id) || used.contains(entry.id))
            continue;
        used.insert(entry.id);

        QVariantMap merged;
        merged.insert(QStringLiteral("id"), QStringLiteral("recent:") + entry.id);
        merged.insert(QStringLiteral("appId"), entry.id);
        merged.insert(QStringLiteral("desktopId"), entry.id);
        merged.insert(QStringLiteral("name"), entry.name);
        merged.insert(QStringLiteral("icon"), entry.icon);
        merged.insert(QStringLiteral("kind"), QStringLiteral("recent"));
        merged.insert(QStringLiteral("pinned"), false);
        merged.insert(QStringLiteral("missing"), false);
        merged.insert(QStringLiteral("running"), false);
        merged.insert(QStringLiteral("windows"), 0);
        out.append(merged);
    }
    return out;
}

QVariantList buildDockEntries(const QStringList &pinnedIds, const DesktopEntryIndex &index,
                              const QVariantList &running,
                              const QHash<QString, QString> &launchStates,
                              const QStringList &recentIds)
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
        QVariantList windowList;
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
            windowList += candidate.value(QStringLiteral("windowList")).toList();
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
        merged.insert(QStringLiteral("windowList"), windowList);
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
        // A temporary running app that resolves to an installed `.desktop`
        // entry can be promoted ("Keep in Dock") by dragging it into the
        // pinned region (T-10 section 12); carry the desktop id so the Dock
        // can build the new pin order.
        const DesktopEntry resolved =
            index.resolve(entry.value(QStringLiteral("appId")).toString());
        if (resolved.valid) {
            entry.insert(QStringLiteral("desktopId"), resolved.id);
            entry.insert(QStringLiteral("icon"), resolved.icon);
            if (entry.value(QStringLiteral("name")).toString().isEmpty())
                entry.insert(QStringLiteral("name"), resolved.name);
        }
        entries.append(entry);
    }

    const QVariantList recents =
        buildRecentEntries(recentIds, pinnedIds, running, index);
    for (const QVariant &recent : recents)
        entries.append(recent);

    for (const QVariantMap &minimized : minimizedEntries)
        entries.append(minimized);

    return entries;
}
