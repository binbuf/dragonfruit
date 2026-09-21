// SPDX-License-Identifier: MIT
// Pure Dock entry model (T-10, sections 4/7): merges the persisted pinned set
// with the shell's running-window projection into the ordered entry list the
// Dock renders. No Wayland, no QML, no state — unit-testable in isolation.
#pragma once

#include <QHash>
#include <QString>
#include <QStringList>
#include <QVariantList>

class DesktopEntryIndex;

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
