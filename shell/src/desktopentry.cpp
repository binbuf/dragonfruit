// SPDX-License-Identifier: MIT
#include "desktopentry.h"

#include "launch_env.h"

#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>

namespace {

// Tokenize a desktop-file Exec line. Arguments are separated by whitespace;
// double quotes group; a backslash escapes the next character inside quotes
// (desktop-file spec). Returns the raw tokens (field codes unexpanded).
QStringList tokenizeExec(const QString &line)
{
    QStringList tokens;
    QString current;
    bool inQuotes = false;
    bool haveToken = false;

    for (qsizetype i = 0; i < line.size(); ++i) {
        const QChar c = line.at(i);
        if (inQuotes) {
            if (c == QLatin1Char('\\') && i + 1 < line.size()) {
                const QChar next = line.at(++i);
                if (next == QLatin1Char('"') || next == QLatin1Char('`')
                    || next == QLatin1Char('$') || next == QLatin1Char('\\')) {
                    current += next;
                } else {
                    current += QLatin1Char('\\');
                    current += next;
                }
            } else if (c == QLatin1Char('"')) {
                inQuotes = false;
            } else {
                current += c;
            }
        } else if (c == QLatin1Char(' ') || c == QLatin1Char('\t')) {
            if (haveToken) {
                tokens << current;
                current.clear();
                haveToken = false;
            }
        } else if (c == QLatin1Char('"')) {
            inQuotes = true;
            haveToken = true;
        } else {
            current += c;
            haveToken = true;
        }
    }
    if (haveToken)
        tokens << current;
    return tokens;
}

// Expand one token's field codes into zero or more argv entries, appending to
// `out`. A token with no `%` is passed through unchanged.
void expandToken(const QString &token, const DesktopEntry &entry, const QStringList &files,
                 QStringList &out)
{
    if (!token.contains(QLatin1Char('%'))) {
        out << token;
        return;
    }
    QString current;
    bool pendingArg = false;
    auto flush = [&]() {
        if (pendingArg) {
            out << current;
            current.clear();
            pendingArg = false;
        }
    };
    for (qsizetype i = 0; i < token.size(); ++i) {
        const QChar c = token.at(i);
        if (c != QLatin1Char('%') || i + 1 >= token.size()) {
            current += c;
            pendingArg = true;
            continue;
        }
        const QChar code = token.at(++i);
        switch (code.unicode()) {
        case 'f':
        case 'u':
            // A single file argument (the first), or nothing.
            flush();
            if (!files.isEmpty())
                out << files.first();
            break;
        case 'F':
        case 'U':
            flush();
            out += files;
            break;
        case 'i':
            flush();
            if (!entry.icon.isEmpty()) {
                out << QStringLiteral("--icon") << entry.icon;
            }
            break;
        case 'c':
            current += entry.name;
            pendingArg = true;
            break;
        case 'k':
            current += entry.id;
            pendingArg = true;
            break;
        case '%':
            current += QLatin1Char('%');
            pendingArg = true;
            break;
        case 'd':
        case 'D':
        case 'n':
        case 'N':
        case 'v':
        case 'm':
            // Deprecated / no-op field codes.
            break;
        default:
            // Unknown code: drop the percent and keep the character.
            current += code;
            pendingArg = true;
            break;
        }
    }
    flush();
}

bool executableOnPath(const QString &program, const QStringList &searchPaths)
{
    if (program.isEmpty())
        return false;
    if (program.contains(QLatin1Char('/'))) {
        const QFileInfo info(program);
        return info.isFile() && info.isExecutable();
    }
    return !QStandardPaths::findExecutable(program, searchPaths).isEmpty();
}

TerminalLaunch makeTerminal(const QString &program, const QStringList &arguments,
                            const QString &source)
{
    TerminalLaunch terminal;
    terminal.program = program;
    terminal.arguments = arguments;
    terminal.source = source;
    return terminal;
}

} // namespace

TerminalLaunch resolveTerminal(const QProcessEnvironment &environment,
                               const QStringList &searchPaths)
{
    // 1. The user's explicit choice. It may carry its own arguments (for
    //    example `alacritty --class Terminal`); the app command follows `-e`.
    //    A stale value falls through to the remaining candidates.
    const QString configured = environment.value(QStringLiteral("TERMINAL")).trimmed();
    if (!configured.isEmpty()) {
        const QStringList tokens = tokenizeExec(configured);
        if (!tokens.isEmpty() && executableOnPath(tokens.first(), searchPaths)) {
            QStringList arguments = tokens.mid(1);
            arguments << QStringLiteral("-e");
            return makeTerminal(tokens.first(), arguments, QStringLiteral("TERMINAL"));
        }
    }

    // 2. The freedesktop launcher: it owns the terminal choice and accepts the
    //    command directly after `--`, so no `-e` is added.
    if (executableOnPath(QStringLiteral("xdg-terminal-exec"), searchPaths))
        return makeTerminal(QStringLiteral("xdg-terminal-exec"),
                            {QStringLiteral("--")},
                            QStringLiteral("xdg-terminal-exec"));

    // 3. The Debian alternatives wrapper (and the traditional fallback).
    if (executableOnPath(QStringLiteral("x-terminal-emulator"), searchPaths))
        return makeTerminal(QStringLiteral("x-terminal-emulator"),
                            {QStringLiteral("-e")},
                            QStringLiteral("x-terminal-emulator"));

    // 4. A common terminal, in preference order. The prefix is the terminal's
    //    own "run this command" separator.
    struct Candidate {
        const char *name;
        const char *prefix;
    };
    // df-allow-desktop-name: freedesktop terminal program names, not desktop
    // identity.
    static constexpr Candidate candidates[] = {
        {"ptyxis", "--"},   {"gnome-terminal", "--"}, {"konsole", "-e"},  // df-allow-desktop-name
        {"alacritty", "-e"}, {"kitty", "--"},          {"foot", "--"},
        {"xterm", "-e"},
    };
    for (const Candidate &candidate : candidates) {
        const QString program = QString::fromLatin1(candidate.name);
        if (executableOnPath(program, searchPaths))
            return makeTerminal(program, {QString::fromLatin1(candidate.prefix)},
                                QStringLiteral("common-list"));
    }

    return TerminalLaunch{};
}

QString terminalLaunchError(const TerminalLaunch &terminal)
{
    if (terminal.found())
        return QString();
    return QStringLiteral(
        "No terminal emulator is installed. Install xdg-terminal-exec or a "
        "terminal such as ptyxis, gnome-terminal, konsole, alacritty, kitty, "  // df-allow-desktop-name
        "foot, or xterm.");
}

void DesktopEntryIndex::loadFromRecords(const QList<DesktopEntry> &entries)
{
    m_byId.clear();
    m_byWmClass.clear();
    m_entries.clear();
    for (const DesktopEntry &entry : entries) {
        if (entry.valid && !m_byId.contains(entry.id))
            insert(entry);
    }
}

void DesktopEntryIndex::insert(const DesktopEntry &entry)
{
    m_byId.insert(entry.id, entry);
    m_entries.append(entry);
    if (!entry.startupWmClass.isEmpty()) {
        const QString key = entry.startupWmClass.toLower();
        if (!m_byWmClass.contains(key))
            m_byWmClass.insert(key, entry.id);
    }
}

DesktopEntry DesktopEntryIndex::byId(const QString &id) const
{
    if (id.isEmpty())
        return DesktopEntry{};
    if (const auto it = m_byId.constFind(id); it != m_byId.constEnd())
        return it.value();
    if (!id.endsWith(QLatin1String(".desktop")))
        return m_byId.value(id + QStringLiteral(".desktop"), DesktopEntry{});
    return DesktopEntry{};
}

DesktopEntry DesktopEntryIndex::resolve(const QString &appId) const
{
    if (appId.isEmpty())
        return DesktopEntry{};
    // 1. Exact desktop id, with or without the suffix.
    DesktopEntry entry = byId(appId);
    if (entry.valid)
        return entry;
    // 2. StartupWMClass (case-insensitive).
    if (const auto it = m_byWmClass.constFind(appId.toLower()); it != m_byWmClass.constEnd())
        return m_byId.value(it.value());
    // 3. The reverse-DNS basename stem ("org.dragonfruit.Files" -> "Files").
    const qsizetype dot = appId.lastIndexOf(QLatin1Char('.'));
    if (dot >= 0) {
        const QString stem = appId.mid(dot + 1);
        for (const DesktopEntry &candidate : m_entries) {
            QString candidateStem = candidate.id;
            if (candidateStem.endsWith(QLatin1String(".desktop")))
                candidateStem.chop(8);
            const qsizetype candidateDot = candidateStem.lastIndexOf(QLatin1Char('.'));
            const QString candidateTail =
                candidateDot >= 0 ? candidateStem.mid(candidateDot + 1) : candidateStem;
            if (candidateTail.compare(stem, Qt::CaseInsensitive) == 0)
                return candidate;
        }
    }
    return DesktopEntry{};
}

bool DesktopEntryIndex::isLaunchable(const DesktopEntry &entry)
{
    return entry.valid && !entry.noDisplay && !entry.exec.trimmed().isEmpty()
           && !entry.exec.contains(QLatin1Char('\n'));
}

QStringList DesktopEntryIndex::buildLaunchCommand(const DesktopEntry &entry,
                                                  const QStringList &files,
                                                  const TerminalLaunch &terminal)
{
    if (!isLaunchable(entry))
        return QStringList();

    QStringList argv;
    const QStringList tokens = tokenizeExec(entry.exec);
    for (const QString &token : tokens)
        expandToken(token, entry, files, argv);

    // Terminal applications need an emulator wrapping the command. app-index
    // carries only the `Terminal` flag, so the launcher resolves one (T340).
    // Never run a `Terminal=true` command unwrapped: with no terminal the
    // empty result becomes a visible launch failure at the caller.
    if (entry.terminal) {
        if (!terminal.found())
            return QStringList();
        argv = QStringList{terminal.program} + terminal.arguments + argv;
    }
    return argv;
}

AppScopeCommand appScopeCommand(const QStringList &argv,
                                const QProcessEnvironment &environment,
                                const QString &unit)
{
    AppScopeCommand command;
    command.program = QStringLiteral("systemd-run");
    command.unit = unit;
    command.arguments << QStringLiteral("--user")
                      << QStringLiteral("--collect")
                      << QStringLiteral("--unit=%1").arg(unit)
                      << QStringLiteral("--working-directory=%1").arg(QDir::homePath());
    const QStringList values = environment.toStringList();
    for (const QString &entry : values)
        command.arguments << QStringLiteral("--setenv=%1").arg(entry);
    command.arguments << QStringLiteral("--") << argv;
    return command;
}

QProcessEnvironment appLaunchEnvironment(const QProcessEnvironment &base,
                                         const QString &waylandDisplay)
{
    QProcessEnvironment environment = base;
    if (environment.value(QStringLiteral("QT_QPA_PLATFORM"))
        == QLatin1String("offscreen")) {
        // The shell forced the offscreen QPA for its own chrome; a launched Qt
        // app must use the session's Wayland platform (not inherit offscreen,
        // and not fall back to X11 because the shell also carries DISPLAY).
        environment.insert(QStringLiteral("QT_QPA_PLATFORM"), QStringLiteral("wayland"));
    }
    if (!waylandDisplay.isEmpty()) {
        // The shell's own connection names the display the compositor serves;
        // a child must use the same socket. This fixes the `--socket-name`
        // case where `$WAYLAND_DISPLAY` was never set, so a launch otherwise
        // produced a client with no display that never maps a window
        // (T-14.7g).
        environment.insert(QStringLiteral("WAYLAND_DISPLAY"), waylandDisplay);
    }
    // Xwayland starts after the shell, so read its hand-off file now rather
    // than caching it. The helper also removes an inherited DISPLAY when the
    // file is absent or empty, so a Wayland-only launch cannot fall back to a
    // stale X server from the shell's parent environment (T309).
    Dragonfruit::applyX11DisplayHandoff(environment, waylandDisplay);
    return environment;
}
