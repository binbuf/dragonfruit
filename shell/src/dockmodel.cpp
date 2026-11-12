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
    // T199: live settings may arrive from an older/partial writer. Clamp the
    // continuous ranges and the placement enum here so the QML geometry is
    // always finite and the anchored edge is well defined.
    config.size = qBound(0.0, read("dock.size", 0.5).toDouble(), 1.0);
    config.magnification =
        qBound(0.0, read("dock.magnification", 0.5).toDouble(), 1.0);
    const QString position =
        read("dock.position", QStringLiteral("bottom")).toString();
    config.position = (position == QLatin1String("left")
                       || position == QLatin1String("right"))
                          ? position
                          : QStringLiteral("bottom");
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

QStringList dockRecentIds(const DockConfig &config, const QStringList &recentIds)
{
    // `dock.showRecentApps` is the single policy switch for the suggested
    // entries; the recency list is still kept while off so toggling it on is
    // immediate (T-10 section 17, T236).
    return config.showRecentApps ? recentIds : QStringList();
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
    // One measured lift: 0 -> 1 -> 0 over the token duration. The caller's
    // sin(pi * phase) turns it into a single 15 pt hop.
    if (elapsedMs < 0 || elapsedMs >= kLaunchBounceMs)
        return -1.0;
    return static_cast<double>(elapsedMs) / static_cast<double>(kLaunchBounceMs);
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

DockActivationFallback dockActivationFallback(bool found, bool launchable)
{
    if (found)
        return DockActivationFallback::None;
    return launchable ? DockActivationFallback::Launch : DockActivationFallback::Notice;
}

QVariantList filterDockQuittingEntries(const QVariantList &running,
                                       const QStringList &quittingAppIds)
{
    if (quittingAppIds.isEmpty())
        return running;
    const QSet<QString> quitting(quittingAppIds.begin(), quittingAppIds.end());
    QVariantList kept;
    kept.reserve(running.size());
    for (const QVariant &value : running) {
        const QVariantMap map = value.toMap();
        const QString kind = map.value(QStringLiteral("kind")).toString();
        if (kind != QLatin1String("temporary")
            && kind != QLatin1String("minimized")) {
            kept.append(value);
            continue;
        }
        if (!quitting.contains(map.value(QStringLiteral("appId")).toString()))
            kept.append(value);
    }
    return kept;
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
        bool anyActive = false;
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
            const QVariantList candidateWindows =
                candidate.value(QStringLiteral("windowList")).toList();
            windowList += candidateWindows;
            if (candidate.value(QStringLiteral("minimized")).toBool())
                anyMinimized = true;
            for (const QVariant &windowValue : candidateWindows) {
                if (windowValue.toMap().value(QStringLiteral("focused")).toBool()) {
                    anyActive = true;
                    break;
                }
            }
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
        // A pinned running app keeps the projection's active flag through the
        // pin merge, so its running dot stays full-strength while it owns the
        // focused window (T-14.7ac).
        merged.insert(QStringLiteral("active"), anyActive);
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

    // Publish each entry's command capabilities with the entry so
    // the menu disables unavailable rows (and explains why) from the same
    // data dispatch revalidates.
    for (QVariant &value : entries) {
        QVariantMap entry = value.toMap();
        entry.insert(QStringLiteral("capabilities"),
                     dockActionCapabilities(entry).toVariantMap());
        value = entry;
    }

    return entries;
}

QVariantMap DockActionCapabilities::toVariantMap() const
{
    return QVariantMap{
        {QStringLiteral("assignThisDesktop"), assignThisDesktop},
        {QStringLiteral("assignAllDesktops"), assignAllDesktops},
        {QStringLiteral("assignNone"), assignNone},
        {QStringLiteral("openAtLogin"), openAtLogin},
        {QStringLiteral("keepInDock"), keepInDock},
        {QStringLiteral("showAppWindows"), showAppWindows},
        {QStringLiteral("showOverview"), showOverview},
    };
}

DockActionCapabilities dockActionCapabilities(const QVariantMap &entry)
{
    DockActionCapabilities caps;
    const bool running = entry.value(QStringLiteral("running")).toBool();
    const QString desktopId = entry.value(QStringLiteral("desktopId")).toString();
    const QString appId = entry.value(QStringLiteral("appId")).toString();

    // "Assign to This Desktop" moves the app's live windows to the active
    // Space, so it needs a running app identity. The other assignment choices
    // have no compositor model yet (T-04/T-05); login items are T-24.
    caps.assignThisDesktop = running && !appId.isEmpty();
    caps.assignAllDesktops = false;
    caps.assignNone = false;
    caps.openAtLogin = false;

    // `desktopId` is the only persistable pin identity. An unresolved running
    // app has none, so Keep in Dock must not promise to remember it.
    caps.keepInDock = !desktopId.isEmpty();

    // The current compositor exposes only the unfiltered overview request.
    caps.showAppWindows = false;
    caps.showOverview = true;
    return caps;
}

bool dockActionAvailable(const QVariantMap &entry, const QString &action,
                         const QString &target)
{
    const DockActionCapabilities caps = dockActionCapabilities(entry);
    if (action == QLatin1String("assign_to")) {
        if (target == QLatin1String("this"))
            return caps.assignThisDesktop;
        if (target == QLatin1String("all"))
            return caps.assignAllDesktops;
        if (target == QLatin1String("none"))
            return caps.assignNone;
        return false;
    }
    if (action == QLatin1String("open_at_login"))
        return caps.openAtLogin;
    if (action == QLatin1String("keep_in_dock"))
        return caps.keepInDock;
    if (action == QLatin1String("show_app_windows"))
        return caps.showAppWindows;
    if (action == QLatin1String("show_overview")
        || action == QLatin1String("show_all_windows"))
        return caps.showOverview;
    // Every other action keeps its existing routing.
    return true;
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

qint64 DockLayoutMetrics::lengthFor(int iconSize, const DockLayoutSpacing &spacing) const
{
    if (itemCount() <= 0)
        return 0;
    return qint64(iconCount) * iconSize
         + qint64(dividerCount) * spacing.dividerWidth
         + qint64(iconGapCount) * spacing.gap
         + qint64(dividerGapCount) * spacing.dividerGap
         + qint64(paddingCount) * spacing.paddingAlong;
}

DockLayoutMetrics planDockLayout(const QVariantList &entries, int fixedCount,
                                 bool minimizedVisible)
{
    DockLayoutMetrics metrics;
    metrics.regions = planDockRegions(entries, fixedCount, minimizedVisible);
    metrics.iconCount = metrics.regions.pinned + metrics.regions.tail
                      + metrics.regions.minimized + metrics.regions.fixed;
    metrics.dividerCount = metrics.regions.dividerCount();
    const int items = metrics.itemCount();
    if (items <= 0)
        return metrics;

    // The plate's along-axis padding is present only when there is content.
    metrics.paddingCount = 2;
    const int gaps = items - 1;
    // A divider is always internal (the fixed tail follows the last app
    // region), so both of its adjacent gaps use `dividerGap`. Collapsed
    // regions remove the divider and its two gaps together.
    metrics.dividerGapCount = qMin(gaps, 2 * metrics.dividerCount);
    metrics.iconGapCount = gaps - metrics.dividerGapCount;
    return metrics;
}

DockOverflowResult applyDockOverflow(const QVariantList &entries, int availableLength,
                                     int requestedIconSize, int iconMin, int iconMax,
                                     int gap, int dividerWidth, int fixedCount,
                                     bool minimizedVisible)
{
    return applyDockOverflow(entries, availableLength, requestedIconSize, iconMin,
                             iconMax, DockLayoutSpacing{gap, gap, dividerWidth, 0},
                             fixedCount, minimizedVisible);
}

DockOverflowResult applyDockOverflow(const QVariantList &entries, int availableLength,
                                     int requestedIconSize, int iconMin, int iconMax,
                                     const DockLayoutSpacing &spacing, int fixedCount,
                                     bool minimizedVisible)
{
    DockOverflowResult result;
    result.entries = entries;
    result.iconSize = qBound(iconMin, requestedIconSize, iconMax);
    const int requested = result.iconSize;
    if (availableLength <= 0 || result.iconSize <= 0)
        return result;

    const auto lengthAt = [&spacing](const DockLayoutMetrics &layout, int icon) {
        return layout.lengthFor(icon, spacing);
    };
    const auto metricsFor = [fixedCount, minimizedVisible](const QVariantList &list) {
        return planDockLayout(list, fixedCount, minimizedVisible);
    };

    // The full content fits at the requested size: nothing is hidden.
    const DockLayoutMetrics full = metricsFor(entries);
    if (full.itemCount() <= 0)
        return result;
    if (lengthAt(full, requested) <= availableLength) {
        result.iconSize = requested;
        return result;
    }

    // The largest icon in the token range whose whole content fits. The
    // content length is linear in the icon size, so one division is exact.
    int icon = iconMin;
    if (full.iconCount > 0) {
        const qint64 fit = (availableLength - lengthAt(full, 0)) / full.iconCount;
        icon = int(qBound(qint64(iconMin), fit, qint64(requested)));
    }
    if (lengthAt(full, icon) <= availableLength) {
        result.iconSize = icon;
        result.clamped = icon < requested;
        return result;
    }

    // Even at the minimum the full set overflows. Count only the entries the
    // renderer actually shows: pinned, running/recent/overflow, minimized
    // (when visible), and the fixed tail. Unknown kinds are not rendered.
    int temporary = 0;
    int recent = 0;
    for (const QVariant &value : entries) {
        const QString kind = value.toMap().value(QStringLiteral("kind")).toString();
        if (kind == QLatin1String("temporary"))
            ++temporary;
        else if (kind == QLatin1String("recent"))
            ++recent;
    }

    result.iconSize = iconMin;
    result.clamped = true;

    // Build a candidate visible set and evaluate its *own* region boundaries.
    // A region that disappears frees its divider and both of its gaps; keeping
    // the full-set boundary count would make the fit disagree with the plate.
    const auto candidateFor = [&](int keptTemporary, int keptRecent,
                                  bool showOverflow) {
        DockOverflowResult candidate;
        candidate.entries = entries;
        candidate.iconSize = iconMin;
        candidate.clamped = true;
        candidate = buildOverflowResult(entries, keptTemporary, keptRecent,
                                        showOverflow, candidate);
        candidate.hiddenTemporary = temporary - keptTemporary;
        candidate.hiddenRecent = recent - keptRecent;
        candidate.overflowShown = showOverflow ? candidate.hiddenTemporary : 0;
        return candidate;
    };
    const auto fits = [&](const DockOverflowResult &candidate) {
        return lengthAt(metricsFor(candidate.entries), iconMin) <= availableLength;
    };

    // Recents are suggestions and the least important: keep every running
    // group visible and silently drop as many recents as needed. No overflow
    // affordance is required because no live group is folded away.
    for (int keptRecent = recent; keptRecent >= 0; --keptRecent) {
        const DockOverflowResult candidate = candidateFor(temporary, keptRecent, false);
        if (fits(candidate)) {
            result = candidate;
            return result;
        }
    }

    // Some running groups must fold. Reserve one terminal overflow cell and
    // keep as many groups visible as fit; recents are not kept while groups
    // are hidden. The first fitting candidate keeps the most groups.
    if (temporary > 0) {
        for (int keptTemporary = temporary - 1; keptTemporary >= 0; --keptTemporary) {
            const DockOverflowResult candidate = candidateFor(keptTemporary, 0, true);
            if (fits(candidate)) {
                result = candidate;
                return result;
            }
        }
        // Even the cell does not fit beside the non-droppable content. Keep it
        // anyway so hidden groups remain reachable through the app region's
        // scroll path, and report the error instead of dropping them.
        result = candidateFor(0, 0, true);
        result.overflowed = true;
        return result;
    }

    // No running groups to fold: drop only as many recents as needed.
    for (int keptRecent = recent - 1; keptRecent >= 0; --keptRecent) {
        const DockOverflowResult candidate = candidateFor(0, keptRecent, false);
        if (fits(candidate)) {
            result = candidate;
            return result;
        }
    }

    // Non-droppable content still exceeds the axis at the minimum size. Keep
    // every entry and report the error; the caller warns and the Dock keeps
    // its reachable overflow affordance for a later scroll path.
    result.overflowed = true;
    return result;
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
