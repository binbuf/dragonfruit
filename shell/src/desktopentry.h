// SPDX-License-Identifier: GPL-3.0-or-later
// The Dock's application-entry cache and launcher.
//
// Identity is owned by `org.dragonfruit.AppIndex1` (T-14.1a, ADR 0086): the
// cache is loaded from the application catalog and never scans or parses
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
    bool operator==(const DesktopEntry &) const = default;
};

// The terminal emulator that wraps a `Terminal=true` entry (T340). `program`
// is empty when none was found. `arguments` are the emulator's own prefix
// arguments (for example `-e` or `--`) placed before the app command.
struct TerminalLaunch {
    QString program;
    QStringList arguments;
    QString source; // "TERMINAL", "xdg-terminal-exec", ...
    bool found() const { return !program.isEmpty(); }
};

// Resolve a terminal for `Terminal=true` entries, in order: `$TERMINAL`,
// `xdg-terminal-exec`, `x-terminal-emulator`, then a common list: ptyxis,
// gnome-terminal, konsole, alacritty, kitty, foot, xterm.  // df-allow-desktop-name
// `searchPaths` is the PATH to search; empty uses the process PATH. A
// configured `$TERMINAL` that is not executable falls through. Pure and
// unit-tested (tst_desktopentry).
TerminalLaunch resolveTerminal(const QProcessEnvironment &environment,
                               const QStringList &searchPaths = QStringList());

// The visible reason a `Terminal=true` entry cannot be launched because no
// terminal exists. Empty when `terminal` was found. Never run the command
// unwrapped.
QString terminalLaunchError(const TerminalLaunch &terminal);

class DesktopEntryIndex
{
public:
    DesktopEntryIndex() = default;

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
    // codes (%d/%D/%n/%N/%v/%m) are dropped. A `Terminal=true` entry is
    // wrapped in `terminal`; when that is absent the result is empty (never
    // the unwrapped command). Empty when not launchable. The default terminal
    // is resolved from the process environment; tests pass one explicitly.
    static QStringList buildLaunchCommand(
        const DesktopEntry &entry, const QStringList &files = QStringList(),
        const TerminalLaunch &terminal =
            resolveTerminal(QProcessEnvironment::systemEnvironment()));

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
// shell needed it to reach the socket). The Xwayland `DISPLAY` is read from
// `$XDG_RUNTIME_DIR/<socket>.x11-display` at launch time (T309): the helper
// sets it when the file is present and removes an inherited value when it is
// absent. Pure and unit-tested (T-14.7g, T309).
QProcessEnvironment appLaunchEnvironment(const QProcessEnvironment &base,
                                         const QString &waylandDisplay = QString());

// A `systemd-run --user` invocation that starts `argv` in its own transient
// service/scope. Keeping the command pure makes the isolation path testable
// without a running user manager.
struct AppScopeCommand {
    QString program;
    QStringList arguments;
    QString unit;
};

// Build the `systemd-run` command for a transient user service. The service
// inherits `environment` via explicit `--setenv` entries (the user manager's
// environment is not the shell's), and the unit name carries the app identity
// so Force Quit can stop the whole cgroup.
AppScopeCommand appScopeCommand(const QStringList &argv,
                                const QProcessEnvironment &environment,
                                const QString &unit);
