// SPDX-License-Identifier: MIT
// Pure Dock entry model (T-10, sections 4/7): merges the persisted pinned set
// with the shell's running-window projection into the ordered entry list the
// Dock renders. No Wayland, no QML, no state — unit-testable in isolation.
#pragma once

#include <QHash>
#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QVariantMap>

class DesktopEntryIndex;

// The Dock's slice of the settings schema (T-08.2a). The values are read from
// the `org.dragonfruit.Settings1` client; this struct is the typed view the
// controller and the layout use. The defaults mirror the schema and are the
// same ones the client seeds when settingsd is absent.
struct DockConfig {
    double size = 0.5;
    double magnification = 0.5;
    QString position = QStringLiteral("bottom");
    bool autohide = false;
    bool animateOpening = true;
    bool showIndicators = true;
    bool minimizeIntoTileIcon = false;
    QString minimizedAnimation = QStringLiteral("scale");
    QString titlebarDoubleClick = QStringLiteral("zoom");
    bool showRecentApps = false;
    bool reduceMotion = false;
    QStringList pinned;

    bool operator==(const DockConfig &) const = default;
};

// Read the Dock keys out of a settings value map (missing keys fall back to
// the schema defaults). Pure and unit-testable.
DockConfig dockConfigFromValues(const QVariantMap &values);

// Map `dock.size` (0..1) onto the icon-size token range. Pure: the QML token
// bounds are passed in by the caller.
int dockIconSize(double size, int iconMin, int iconMax);

// The default pinned set (T-10 section 19): Files, Settings, Terminal,
// Browser, resolved against the installed `.desktop` corpus by trying a small
// candidate list per slot and taking the first that resolves. Unresolved slots
// are skipped so the Dock never opens with broken default tiles.
QStringList resolveDefaultDockPins(const DesktopEntryIndex &index);

// Build the Dock's ordered entries. `running` is the shell projection
// (kind "temporary" with `windows`, plus per-window "minimized" entries).
// `launchStates` maps a pinned desktop id to "launching" | "failed" (absent =
// idle). `recentIds` is the recency-ordered app list for the suggested
// entries (T-10 section 17; empty when `dock.showRecentApps` is off). Pinned
// entries come first in `pinnedIds` order, then unmatched temporary running
// apps, then suggested recents, then minimized windows (the Dock inserts the
// divider between app entries and minimized entries).
QVariantList buildDockEntries(const QStringList &pinnedIds, const DesktopEntryIndex &index,
                              const QVariantList &running,
                              const QHash<QString, QString> &launchStates,
                              const QStringList &recentIds = {});

// Human-readable fallback name for an unresolved identity: the last
// reverse-DNS segment, with any `.desktop` suffix removed.
QString displayNameForIdentity(const QString &identity);

// Recent/suggested apps (T-10 section 17): up to `limit` entries for the
// recency-ordered `recentIds` that are not pinned and not already running.
// Gated by `dock.showRecentApps` at the shell; this only builds the entries.
// The result is appended to the app region (before the divider), each with
// `kind: "recent"` and `running: false`, so a click launches it.
QVariantList buildRecentEntries(const QStringList &recentIds, const QStringList &pinnedIds,
                                const QVariantList &running, const DesktopEntryIndex &index,
                                int limit = 3);

// The T-10 section 5.1 overflow clamp. A Dock wider than its output is an
// error state: `dock.size` is clamped so the content fits, overflow
// temporary/recent entries are hidden (recents first, then temporaries;
// pinned and minimized entries are never dropped), and a warning is logged
// once per session by the caller. `fixedCount` is the number of permanent
// non-app entries besides the divider (the Downloads stack and the Trash);
// the divider is always counted. `minimizedVisible` is false when
// `dock.minimizeIntoTileIcon` hides the minimized entries.
struct DockOverflowResult {
    QVariantList entries; // `entries` with overflow temporary/recent removed
    int iconSize = 0;     // effective icon size in px (never above requested)
    int hiddenTemporary = 0;
    int hiddenRecent = 0;
    bool clamped = false;    // the size was reduced or entries were hidden
    bool overflowed = false; // content still exceeds the axis at the minimum
};

DockOverflowResult applyDockOverflow(const QVariantList &entries, int availableLength,
                                     int requestedIconSize, int iconMin, int iconMax,
                                     int gap, int dividerWidth, int fixedCount = 2,
                                     bool minimizedVisible = true);

// T-10 section 8.1 bounce clocks. A launch bounce is three hops over
// `kLaunchBounceMs`; an attention bounce repeats one hop every
// `kAttentionBounceHopMs` until the controller's deadline. Both return the
// current hop phase in [0,1], where the caller maps it to an offset with
// `sin(pi * phase)`; `dockLaunchBouncePhase` returns -1 once the launch
// bounce has finished. Pure and unit-tested (tst_dockcore).
constexpr qint64 kLaunchBounceMs = 600;
constexpr qint64 kAttentionBounceHopMs = 400;
constexpr qint64 kAttentionBounceMs = 2000;
double dockLaunchBouncePhase(qint64 elapsedMs);
double dockAttentionBouncePhase(qint64 elapsedMs);
