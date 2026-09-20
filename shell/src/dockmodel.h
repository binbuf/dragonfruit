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
