// SPDX-License-Identifier: MIT
#include "apppicker.h"

#include <QSet>

#include <algorithm>

QList<AppPickerRow> buildAppPickerList(const QList<DesktopEntry> &entries,
                                       const QStringList &pinnedIds, const QString &query)
{
    const QString needle = query.trimmed().toLower();

    QList<AppPickerRow> rows;
    rows.reserve(entries.size());
    QSet<QString> seen;
    for (const DesktopEntry &entry : entries) {
        if (!DesktopEntryIndex::isLaunchable(entry))
            continue;
        if (entry.id.isEmpty() || seen.contains(entry.id))
            continue;
        seen.insert(entry.id);
        if (!needle.isEmpty()
            && !entry.name.toLower().contains(needle)
            && !entry.id.toLower().contains(needle)) {
            continue;
        }
        AppPickerRow row;
        row.desktopId = entry.id;
        row.name = entry.name.isEmpty() ? entry.id : entry.name;
        row.iconPath = entry.iconPath;
        row.pinned = pinnedIds.contains(entry.id);
        rows.append(row);
    }

    std::sort(rows.begin(), rows.end(), [](const AppPickerRow &a, const AppPickerRow &b) {
        const int byName = QString::localeAwareCompare(a.name, b.name);
        if (byName != 0)
            return byName < 0;
        return a.desktopId < b.desktopId;
    });
    return rows;
}

QStringList toggleAppPickerPin(const QStringList &pinnedIds, const QString &desktopId, bool pinned)
{
    QStringList ids = pinnedIds;
    if (pinned) {
        if (!desktopId.isEmpty() && !ids.contains(desktopId))
            ids.append(desktopId);
    } else {
        ids.removeAll(desktopId);
    }
    return ids;
}