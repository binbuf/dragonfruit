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
#include <QString>
#include <QStringList>

// One parsed `[Desktop Entry]`. `valid` is false for a synthesized record
// (a pinned id that no installed `.desktop` file resolves).
struct DesktopEntry {
    QString id; // e.g. "org.dragonfruit.Files.desktop"
    QString name;
    QString icon;
    QString exec;
    QString startupWmClass;
    QStringList categories;
    bool terminal = false;
    bool noDisplay = false;
    bool valid = false;
};

class DesktopEntryIndex
{
public:
    DesktopEntryIndex() = default;

    // Scan `applicationDirs` (default: the XDG application directories, user
    // dirs first) and build the lookup tables. Later duplicates of the same
    // id do not override the first one (desktop-file spec precedence).
    void scan(const QStringList &applicationDirs = defaultApplicationDirs());
    static QStringList defaultApplicationDirs();

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
