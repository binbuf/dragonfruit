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
// idle). Pinned entries come first in `pinnedIds` order, then unmatched
// temporary running apps, then minimized windows (the Dock inserts the
// divider between app entries and minimized entries).
QVariantList buildDockEntries(const QStringList &pinnedIds, const DesktopEntryIndex &index,
                              const QVariantList &running,
                              const QHash<QString, QString> &launchStates);

// Human-readable fallback name for an unresolved identity: the last
// reverse-DNS segment, with any `.desktop` suffix removed.
QString displayNameForIdentity(const QString &identity);

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
