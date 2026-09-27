// SPDX-License-Identifier: MIT
// The Dock's application-entry cache and launcher.
//
// Identity is owned by `org.dragonfruit.AppIndex1` (T-14.1a, ADR 0086): the
// cache is loaded from `AppIndexClient::enumerate()` and never scans or parses
// `.desktop` files itself (the interim resolver was retired in T-14.7). What
// remains here is the record shape the Dock renders, the in-memory lookup the
// pure merge model uses, and the launcher (`buildLaunchCommand` /
// `appLaunchEnvironment`).
#pragma once

#include <QHash>
#include <QList>
#include <QProcessEnvironment>
#include <QString>
#include <QStringList>

// One parsed `[Desktop Entry]`. `valid` is false for a synthesized record
// (a pinned id that no installed `.desktop` file resolves).
struct DesktopEntry {
    QString id; // e.g. "org.dragonfruit.Files.desktop"
    QString name;
    QString icon;
    QString iconPath; // themed icon file, from app-index (T-14.1a)
    QString exec;
    QString startupWmClass;
    QStringList categories;
    bool terminal = false;
    bool noDisplay = false;
    bool valid = false;
};

class AppIndexClient;

class DesktopEntryIndex
{
public:
    DesktopEntryIndex() = default;

    // Populate from `org.dragonfruit.AppIndex1` (T-14.1a): the service is the
    // single owner of identity and themed icons. Clears any previous content
    // and inserts every enumerated record. When the service is absent the
    // call is a no-op and the cache stays empty (the Dock renders no entries).
    void loadFromAppIndex(const AppIndexClient &client);

    // Insert a set of records (from app-index) into the lookup tables. Pure;
    // exposed so the loader is unit-tested without a live bus.
    void loadFromRecords(const QList<DesktopEntry> &entries);

    // Resolve a compositor `app_id` / Xwayland `WM_CLASS` to an installed
    // entry. Matches, in order: the exact desktop id (with or without the
    // `.desktop` suffix), `StartupWMClass` (case-insensitive), and the
    // reverse-DNS basename stem. Returns an invalid entry on a miss.
    DesktopEntry resolve(const QString &appId) const;

    DesktopEntry byId(const QString &id) const;
    QList<DesktopEntry> entries() const { return m_entries; }
    bool isEmpty() const { return m_entries.isEmpty(); }

    // True when the entry has a usable single-line Exec.
    static bool isLaunchable(const DesktopEntry &entry);

    // The argv for launching `entry`. `files` are substituted for the file
    // field codes (%f/%F/%u/%U); `%i` becomes `--icon <icon>`, `%c` the name,
    // `%k` the desktop-file path, `%%` a literal percent; the deprecated
    // codes (%d/%D/%n/%N/%v/%m) are dropped. Empty when not launchable.
    static QStringList buildLaunchCommand(const DesktopEntry &entry,
                                          const QStringList &files = QStringList());

private:
    void insert(const DesktopEntry &entry);

    QHash<QString, DesktopEntry> m_byId;
    QHash<QString, QString> m_byWmClass; // lowercased WM_CLASS -> id
    QList<DesktopEntry> m_entries;
};

// The environment a launched app inherits. The shell forces the offscreen QPA
// for its own chrome (`main.cpp`), so that value must not reach a launched Qt
// app or it would render offscreen and never map a window. When
// `QT_QPA_PLATFORM` is the shell's `offscreen` value it is replaced with the
// session's `wayland` platform; any other value (a deliberate session setting)
// is preserved. `waylandDisplay` is the socket the shell connected the
// compositor on (`--socket-name`, or the resolved `$WAYLAND_DISPLAY`); when
// non-empty it is exported as `WAYLAND_DISPLAY` so a launched client finds the
// display even when the shell was started with `--socket-name` and the
// variable was never in the environment. `XDG_RUNTIME_DIR` is inherited (the
// shell needed it to reach the socket); the helper only strips the offscreen
// QPA. Pure and unit-tested (T-14.7g).
QProcessEnvironment appLaunchEnvironment(const QProcessEnvironment &base,
                                         const QString &waylandDisplay = QString());
