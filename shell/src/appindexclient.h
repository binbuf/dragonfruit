// SPDX-License-Identifier: MIT
// Client for `org.dragonfruit.AppIndex1` (T-14.1a).
//
// The shell queries app-index for application identity and themed icons; it
// never scans `.desktop` directories itself. Every call is blocking and
// returns exactly what the service's flat JSON view carries. When the service
// is absent every call is a no-op miss, so a session without app-index
// degrades to the Dock's legacy local index rather than failing.
#pragma once

#include "desktopentry.h"

#include <QList>
#include <QString>

class AppIndexClient
{
public:
    AppIndexClient() = default;

    // True when `org.dragonfruit.AppIndex1` is on the session bus.
    bool available() const;

    // Resolve a Wayland `app_id` (or desktop id) to an installed record.
    // Returns an invalid entry on a miss.
    DesktopEntry resolve(const QString &identity) const;

    // Resolve a window's identifiers: the Wayland `appId` (may be empty) and
    // the X11 `WM_CLASS` instance/class (may be empty). Returns an invalid
    // entry on a miss.
    DesktopEntry resolveWindow(const QString &appId, const QString &instance,
                               const QString &wmClass) const;

    // Every installed record, or an empty list when the service is absent.
    QList<DesktopEntry> enumerate() const;

    // Resolve an icon name to a themed file path for `size` (default: the
    // service's record size). Empty when no theme provides it.
    QString iconPath(const QString &name, int size = 128) const;

private:
    QString call(const QString &method, const QList<QVariant> &arguments) const;
    static DesktopEntry parseRecord(const QString &json);
};