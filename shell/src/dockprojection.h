// SPDX-License-Identifier: MIT
// Pure Dock running-app projection (T-10, sections 4.1/7/22): groups the
// compositor's toplevel list into the per-app entries the Dock renders, with
// each app's window list and the separate minimized-window entries. No
// Wayland, no QML, no shell state — unit-testable in isolation (tst_dockcore).
#pragma once

#include <QList>
#include <QString>
#include <QVariantList>

// One window from the compositor, already resolved by the shell: the private
// protocol's toplevel id, app id, title, minimize state, focus, and its Space
// (`workspaceIndex` < 0 when the workspace is not yet known).
struct DockWindow {
    quintptr windowId = 0;
    QString appId;
    QString title;
    bool minimized = false;
    bool focused = false;
    int workspaceIndex = -1;
    QString workspaceName;
};

// Build the Dock's running projection. `windows` is in announcement order
// (oldest first); within each app the most recent window leads, and the
// focused window leads its app's list. One `temporary` app entry is produced
// per distinct non-empty app id; an empty app id groups under a generic
// `__unknown__` entry the Dock renders with a fallback label. Every app entry
// carries its full `windowList` (most-recent-first) and a `minimized` flag
// that is true only when *all* of the app's windows are minimized. One
// `minimized` entry per minimized window follows, each carrying its app's
// full window list so the owning app's menu works from the minimized entry.
// App entries are name-sorted (stable for equal names).
//
// This is the section 22 lifecycle seam: an app_id change, a window opening
// on another Space, a rapid open/close, or the last window closing all
// resolve here, so the Dock can never hold a stale per-app row.
QVariantList buildDockProjection(const QList<DockWindow> &windows);

// Human-readable fallback name for an unresolved app id (last reverse-DNS
// segment). The real name and icon come from app-index (T-23).
QString displayNameForAppId(const QString &appId);