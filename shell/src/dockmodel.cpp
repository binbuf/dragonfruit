// SPDX-License-Identifier: MIT
#include "dockmodel.h"

#include "desktopentry.h"
#include "dockprojection.h"

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

QString dockLaunchAppId(const DesktopEntry &entry)
{
    // The compositor keys the tile by the Wayland `app_id` a client announces.
    // `StartupWMClass` is exactly that identity when set (and matches an
    // Xwayland WM_CLASS); otherwise the desktop id without its suffix is the
    // best available guess. An empty id yields no key.
    const QString wmClass = entry.startupWmClass.trimmed();
    if (!wmClass.isEmpty())
        return wmClass;
    return withoutDesktopSuffix(entry.id.trimmed());
}

void DockTileRects::record(const QString &id, const QRect &rect)
{
    if (id.isEmpty() || rect.width() <= 0 || rect.height() <= 0)
        return;
    if (!m_rects.contains(id) && m_rects.size() >= kCapacity) {
        while (!m_order.isEmpty()) {
            const QString oldest = m_order.takeFirst();
            if (m_rects.remove(oldest) > 0)
                break;
        }
    }
    if (!m_rects.contains(id))
        m_order.append(id);
    m_rects.insert(id, rect);
}

QRect DockTileRects::rectFor(const QString &id) const
{
    return m_rects.value(id);
}

void DockTileRects::clear()
{
    m_rects.clear();
    m_order.clear();
}

QRect clampDockTileRect(const QRect &rect, const QRect &outputBounds)
{
    if (!outputBounds.isValid())
        return rect;
    const QRect clamped = rect.intersected(outputBounds);
    return clamped.isEmpty() ? QRect() : clamped;
}

DockConfig dockConfigFromValues(const QVariantMap &values)
{
    DockConfig config;
    const auto read = [&values](const char *key, const QVariant &fallback) {
        const auto it = values.constFind(QLatin1String(key));
        return it == values.constEnd() ? fallback : it.value();
    };
    config.size = read("dock.size", 0.5).toDouble();
    config.magnification = read("dock.magnification", 0.5).toDouble();
    config.position = read("dock.position", QStringLiteral("bottom")).toString();
    config.autohide = read("dock.autohide", false).toBool();
    config.animateOpening = read("dock.animateOpening", true).toBool();
    config.showIndicators = read("dock.showIndicators", true).toBool();
    config.minimizeIntoTileIcon = read("dock.minimizeIntoTileIcon", false).toBool();
    config.minimizedAnimation =
        read("dock.minimizedAnimation", QStringLiteral("scale")).toString();
    config.titlebarDoubleClick =
        read("dock.titlebarDoubleClick", QStringLiteral("zoom")).toString();
    config.showRecentApps = read("dock.showRecentApps", false).toBool();
    config.chooserOnHover = read("dock.chooserOnHover", false).toBool();
    config.minimizeReaction = read("dock.minimizeReaction", false).toBool();
    config.reduceMotion = read("accessibility.reduceMotion", false).toBool();
    config.pinned = read("dock.pinned", QStringList()).toStringList();
    config.pinnedFolders = read("dock.pinnedFolders", QStringList()).toStringList();
    return config;
}

int dockIconSize(double size, int iconMin, int iconMax)
{
    return qRound(iconMin + qBound(0.0, size, 1.0) * (iconMax - iconMin));
}

QStringList movePinnedEntry(const QStringList &pinnedIds, int index, int delta)
{
    const int size = pinnedIds.size();
    if (index < 0 || index >= size || delta == 0)
        return pinnedIds;
    const int target = index + delta;
    // A move at the pinned region boundary is a no-op, not a wrap.
    if (target < 0 || target >= size)
        return pinnedIds;
    QStringList out = pinnedIds;
    out.move(index, target);
    return out;
}

QStringList resolveDefaultDockPins(const DesktopEntryIndex &index)
{
    QStringList resolved;
    auto add = [&resolved](const QString &id) {
        if (!id.isEmpty() && !resolved.contains(id))
            resolved.append(id);
    };

    // Our own first-party apps, by id (the only legal desktop name).
    const DesktopEntry files = index.byId(QStringLiteral("org.dragonfruit.Files.desktop"));
    if (files.valid)
        add(files.id);
    const DesktopEntry settings = index.byId(QStringLiteral("org.dragonfruit.Settings.desktop"));
    if (settings.valid)
        add(settings.id);

    // Terminal and browser are chosen by the freedesktop registered category
    // so no distribution-specific desktop id is hardcoded (T-01 FR-3).
    auto firstWithCategory = [&index](const QString &category) -> QString {
        for (const DesktopEntry &entry : index.entries()) {
            if (entry.noDisplay)
                continue;
            if (entry.categories.contains(category, Qt::CaseInsensitive))
                return entry.id;
        }
        return QString();
    };
    add(firstWithCategory(QStringLiteral("TerminalEmulator")));
    add(firstWithCategory(QStringLiteral("WebBrowser")));
    return resolved;
}

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

double dockMinimizeReactionPhase(qint64 elapsedMs)
{
    if (elapsedMs < 0 || elapsedMs >= kMinimizeReactionMs)
        return -1.0;
    return static_cast<double>(elapsedMs) / static_cast<double>(kMinimizeReactionMs);
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
        merged.insert(QStringLiteral("iconPath"), entry.iconPath);
        merged.insert(QStringLiteral("kind"), QStringLiteral("recent"));
        merged.insert(QStringLiteral("pinned"), false);
        merged.insert(QStringLiteral("missing"), false);
        merged.insert(QStringLiteral("running"), false);
        merged.insert(QStringLiteral("windows"), 0);
        merged.insert(QStringLiteral("windowCount"), 0);
        out.append(merged);
    }
    return out;
}

AppOpenPlan planAppOpen(const DesktopEntryIndex &index, const QVariantList &running,
                        const QString &desktopId)
{
    AppOpenPlan plan;
    DesktopEntry entry = index.byId(desktopId);
    if (!entry.valid)
        entry = index.resolve(desktopId);
    if (!entry.valid)
        return plan;
    plan.resolved = true;
    plan.desktopId = entry.id;

    for (const QVariant &value : running) {
        const QVariantMap map = value.toMap();
        const QString appId = map.value(QStringLiteral("appId")).toString();
        if (appId.isEmpty())
            continue;
        const DesktopEntry resolved = index.resolve(appId);
        const bool same = appId == desktopId || appId == entry.id
                          || (resolved.valid && resolved.id == entry.id);
        if (same) {
            plan.running = true;
            plan.appId = appId;
            break;
        }
    }
    return plan;
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
        merged.insert(QStringLiteral("iconPath"), entry.iconPath);
        merged.insert(QStringLiteral("kind"), QStringLiteral("pinned"));
        merged.insert(QStringLiteral("pinned"), true);
        merged.insert(QStringLiteral("missing"), !entry.valid);
        merged.insert(QStringLiteral("running"), matched);
        merged.insert(QStringLiteral("windows"), windows);
        merged.insert(QStringLiteral("windowList"), windowList);
        merged.insert(QStringLiteral("windowCount"), windowList.size());
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
        entry.insert(QStringLiteral("windowCount"), dockWindowCount(entry));
        // A temporary running app that resolves to an installed `.desktop`
        // entry can be promoted ("Keep in Dock") by dragging it into the
        // pinned region (T-10 section 12); carry the desktop id so the Dock
        // can build the new pin order.
        const DesktopEntry resolved =
            index.resolve(entry.value(QStringLiteral("appId")).toString());
        if (resolved.valid) {
            entry.insert(QStringLiteral("desktopId"), resolved.id);
            entry.insert(QStringLiteral("icon"), resolved.icon);
            entry.insert(QStringLiteral("iconPath"), resolved.iconPath);
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

int DockRegionPlan::dividerCount() const
{
    // One rule between each adjacent pair of non-empty regions, in order
    // (pinned | tail | minimized | fixed). Empty regions collapse away, so a
    // pinned-only or tail-only Dock still separates cleanly from the fixed
    // stacks/Trash tail, and a Dock with no app region at all draws nothing.
    int count = 0;
    bool haveRegion = false;
    const int regions[4] = {pinned, tail, minimized, fixed};
    for (const int region : regions) {
        if (region <= 0)
            continue;
        if (haveRegion)
            ++count;
        haveRegion = true;
    }
    return count;
}

DockRegionPlan planDockRegions(const QVariantList &entries, int fixedCount,
                               bool minimizedVisible)
{
    DockRegionPlan plan;
    for (const QVariant &value : entries) {
        const QString kind = value.toMap().value(QStringLiteral("kind")).toString();
        if (kind == QLatin1String("pinned")) {
            ++plan.pinned;
        } else if (kind == QLatin1String("temporary") || kind == QLatin1String("recent")
                   || kind == QLatin1String("overflow")) {
            ++plan.tail;
        } else if (kind == QLatin1String("minimized")) {
            if (minimizedVisible)
                ++plan.minimized;
        }
    }
    plan.fixed = qMax(0, fixedCount);
    return plan;
}

static DockOverflowResult buildOverflowResult(const QVariantList &entries, int keptTemporary,
                                              int keptRecent, bool showOverflow,
                                              DockOverflowResult result);

DockOverflowResult applyDockOverflow(const QVariantList &entries, int availableLength,
                                     int requestedIconSize, int iconMin, int iconMax,
                                     int gap, int dividerWidth, int fixedCount,
                                     bool minimizedVisible)
{
    DockOverflowResult result;
    result.entries = entries;
    result.iconSize = qBound(iconMin, requestedIconSize, iconMax);

    int pinned = 0;
    int temporary = 0;
    int recent = 0;
    int minimized = 0;
    int other = 0;
    for (int i = 0; i < entries.size(); ++i) {
        const QString kind = entries.at(i).toMap().value(QStringLiteral("kind")).toString();
        if (kind == QLatin1String("pinned"))
            ++pinned;
        else if (kind == QLatin1String("temporary"))
            ++temporary;
        else if (kind == QLatin1String("recent"))
            ++recent;
        else if (kind == QLatin1String("minimized"))
            ++minimized;
        else
            ++other;
    }
    if (!minimizedVisible)
        minimized = 0;

    // Every region boundary contributes a divider item (T-14.7v): the pinned |
    // temporary/recent rule, the app | minimized rule, and the minimized |
    // stacks/Trash rule. The fixed tail is the two fixed entries.
    const int dividerCount =
        planDockRegions(entries, fixedCount, minimizedVisible).dividerCount();

    // The dividers plus the two fixed entries are never droppable; pinned and
    // minimized are never dropped either.
    const int nonDroppable = pinned + fixedCount + minimized + other + dividerCount;
    const int droppable = temporary + recent;
    const int count = nonDroppable + droppable;
    if (availableLength <= 0 || count <= 1 || result.iconSize <= 0)
        return result;

    // Icons are `icon` wide and dividers `dividerWidth`; the gaps between
    // adjacent items are `gap`. Every divider is internal (the fixed tail
    // always follows it) so it contributes two of the gaps at most; a Dock
    // whose regions collapse may have fewer. `n - 1` gaps are always charged.
    const auto totalFor = [&](int icon, int n) -> qint64 {
        if (n <= 0)
            return 0;
        if (n <= dividerCount)
            return qint64(n) * dividerWidth;
        return qint64(n - dividerCount) * icon + qint64(dividerCount) * dividerWidth
               + qint64(n - 1) * gap;
    };

    int icon = result.iconSize;
    if (totalFor(icon, count) > availableLength) {
        // The largest icon in the token range whose whole content fits.
        const int fit = dividerCount >= count
            ? iconMin
            : int((availableLength - qint64(dividerCount) * dividerWidth
                   - qint64(count - 1) * gap) / (count - dividerCount));
        icon = qBound(iconMin, fit, result.iconSize);
    }
    // The whole content fits at the clamped size: nothing is hidden.
    if (totalFor(icon, count) <= availableLength) {
        result.iconSize = icon;
        result.clamped = icon < qBound(iconMin, requestedIconSize, iconMax);
        return result;
    }

    // Even at the minimum the full set overflows. The largest number of
    // entries that still fits becomes the budget for the visible set; the rest
    // of the running groups fold into one terminal overflow cell. Pinned,
    // minimized, fixed, and the divider are never dropped, so the budget below
    // them is the error state the caller warns about.
    icon = iconMin;
    int maxVisible = count;
    while (maxVisible > 0 && totalFor(iconMin, maxVisible) > availableLength)
        --maxVisible;

    result.iconSize = icon;

    if (maxVisible < nonDroppable) {
        // Not even the non-droppable content fits: fall back to the legacy
        // clamp, hiding every droppable entry and reporting the error.
        result.hiddenTemporary = temporary;
        result.hiddenRecent = recent;
        result.overflowed = true;
        result.clamped = true;
        return buildOverflowResult(entries, 0, 0, false, result);
    }

    const int capacity = maxVisible - nonDroppable;
    // Recents are the least important: keep as many temporary running groups
    // as fit first, then fill any remaining capacity with recents.
    int keptTemporary = qMin(temporary, capacity);
    int keptRecent = qMin(recent, capacity - keptTemporary);
    int hiddenTemporary = temporary - keptTemporary;
    int hiddenRecent = recent - keptRecent;

    // A hidden running group becomes reachable through one overflow cell. The
    // cell consumes one visible slot, so when groups are hidden and there is
    // room for a cell, fold one more visible group in to make room. With no
    // room left for even the cell, fall back to dropping silently (below).
    bool showOverflow = false;
    if (hiddenTemporary >= 1 && capacity >= 1) {
        // `keptRecent` is necessarily 0 here (a hidden temporary means the
        // capacity was exhausted by temporaries), so this always frees a
        // temporary slot rather than a recent one.
        if (keptTemporary > 0) {
            --keptTemporary;
            ++hiddenTemporary;
        }
        showOverflow = hiddenTemporary >= 1;
    }

    result.hiddenTemporary = hiddenTemporary;
    result.hiddenRecent = hiddenRecent;
    result.overflowShown = showOverflow ? hiddenTemporary : 0;
    result.overflowed = false;
    result.clamped = true;
    return buildOverflowResult(entries, keptTemporary, keptRecent, showOverflow, result);
}

// Assemble the visible entry list from the kept temporary/recent counts: drop
// the tail of each droppable region and, when asked, append the terminal
// overflow entry carrying every hidden running group in original order. The
// overflow entry is the last app-region item, immediately before the first
// minimized entry (the divider follows it in the Dock's rendering).
static DockOverflowResult buildOverflowResult(const QVariantList &entries, int keptTemporary,
                                              int keptRecent, bool showOverflow,
                                              DockOverflowResult result)
{
    QVariantList filtered;
    QVariantList hiddenGroups;
    int keptTemp = 0;
    int keptRec = 0;
    int lastAppRegion = -1;
    for (const QVariant &value : entries) {
        const QVariantMap map = value.toMap();
        const QString kind = map.value(QStringLiteral("kind")).toString();
        bool keep = true;
        if (kind == QLatin1String("temporary")) {
            keep = keptTemp < keptTemporary;
            ++keptTemp;
            if (!keep)
                hiddenGroups.append(value);
        } else if (kind == QLatin1String("recent")) {
            keep = keptRec < keptRecent;
            ++keptRec;
        }
        if (!keep)
            continue;
        filtered.append(value);
        if (kind == QLatin1String("pinned") || kind == QLatin1String("temporary")
                || kind == QLatin1String("recent"))
            lastAppRegion = filtered.size() - 1;
    }

    if (showOverflow && hiddenGroups.size() > 0) {
        QVariantList groups;
        groups.reserve(hiddenGroups.size());
        for (const QVariant &group : hiddenGroups) {
            QVariantMap g = group.toMap();
            g.insert(QStringLiteral("overflowGroup"), true);
            groups.append(g);
        }
        QVariantMap overflow;
        overflow.insert(QStringLiteral("id"), QStringLiteral("__overflow__"));
        overflow.insert(QStringLiteral("kind"), QStringLiteral("overflow"));
        overflow.insert(QStringLiteral("name"), QString());
        overflow.insert(QStringLiteral("appId"), QString());
        overflow.insert(QStringLiteral("running"), false);
        overflow.insert(QStringLiteral("pinned"), false);
        overflow.insert(QStringLiteral("hiddenCount"), groups.size());
        overflow.insert(QStringLiteral("windowCount"), groups.size());
        overflow.insert(QStringLiteral("groups"), groups);
        filtered.insert(lastAppRegion + 1, overflow);
    }

    result.entries = filtered;
    return result;
}
