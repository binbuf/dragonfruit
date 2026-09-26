// SPDX-License-Identifier: MIT
// Interim `.desktop` resolver and launcher for the Dock (T-10, section 8).
//
// This is explicitly a stand-in for app-index (T-23): it scans the XDG
// application directories, parses the fields the Dock needs (Name, Icon,
// Exec, StartupWMClass), and launches with QProcess::startDetached. It has no
// D-Bus, no compositor coupling, and no state beyond the scanned index.
//
// DELETE THIS FILE when T-23's `org.dragonfruit.AppIndex1` lands: the Dock
// switches to the app-index resolve/launch API and the public behavior is
// unchanged (T-10 section 8, "Interim note").
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

    // Scan `applicationDirs` (default: the XDG application directories, user
    // dirs first) and build the lookup tables. Later duplicates of the same
    // id do not override the first one (desktop-file spec precedence).
    void scan(const QStringList &applicationDirs = defaultApplicationDirs());
    static QStringList defaultApplicationDirs();

    // Populate from `org.dragonfruit.AppIndex1` (T-14.1a): the service is the
    // single owner of identity and themed icons. Clears any previous content
    // and inserts every enumerated record. Callers fall back to `scan()` only
    // when the service is absent (deleted in T-14.7).
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

    // Parse the contents of one desktop file (exposed for tests). Only the
    // `[Desktop Entry]` group is read; localized keys (`Name[xx]`) are
    // ignored in favor of the unlocalized key.
    static DesktopEntry parse(const QString &id, const QString &contents);

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
// is preserved. Pure and unit-tested.
QProcessEnvironment appLaunchEnvironment(const QProcessEnvironment &base);
