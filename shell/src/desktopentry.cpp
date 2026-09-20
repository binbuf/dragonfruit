// SPDX-License-Identifier: MIT
#include "desktopentry.h"

#include <QDir>
#include <QFile>
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

QString desktopIdFromPath(const QString &path)
{
    return QFileInfo(path).fileName();
}

} // namespace

QStringList DesktopEntryIndex::defaultApplicationDirs()
{
    QStringList dirs;
    const QString dataHome = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    if (!dataHome.isEmpty())
        dirs << dataHome + QStringLiteral("/applications");

    const QByteArray raw = qgetenv("XDG_DATA_DIRS");
    const QString dataDirs = raw.isEmpty() ? QStringLiteral("/usr/local/share:/usr/share")
                                           : QString::fromLocal8Bit(raw);
    const QStringList parts = dataDirs.split(QLatin1Char(':'), Qt::SkipEmptyParts);
    for (const QString &part : parts)
        dirs << part + QStringLiteral("/applications");

    // Flatpak exports are not always on XDG_DATA_DIRS.
    dirs << QStringLiteral("/var/lib/flatpak/exports/share/applications");
    if (!dataHome.isEmpty())
        dirs << dataHome + QStringLiteral("/flatpak/exports/share/applications");
    return dirs;
}

void DesktopEntryIndex::scan(const QStringList &applicationDirs)
{
    m_byId.clear();
    m_byWmClass.clear();
    m_entries.clear();

    for (const QString &dirPath : applicationDirs) {
        const QDir dir(dirPath);
        if (!dir.exists())
            continue;
        const QStringList files =
            dir.entryList(QStringList{QStringLiteral("*.desktop")}, QDir::Files, QDir::Name);
        for (const QString &file : files) {
            const QString id = desktopIdFromPath(file);
            if (m_byId.contains(id))
                continue; // first directory wins
            QFile handle(dir.filePath(file));
            if (!handle.open(QIODevice::ReadOnly | QIODevice::Text))
                continue;
            const DesktopEntry entry = parse(id, QString::fromUtf8(handle.readAll()));
            if (entry.valid)
                insert(entry);
        }
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
                                                  const QStringList &files)
{
    if (!isLaunchable(entry))
        return QStringList();

    QStringList argv;
    const QStringList tokens = tokenizeExec(entry.exec);
    for (const QString &token : tokens)
        expandToken(token, entry, files, argv);

    // Terminal applications need an emulator wrapping the command. The
    // interim policy: honor $TERMINAL, else the distro-agnostic
    // `x-terminal-emulator` alternative; if neither is available the command
    // runs unwrapped. T-23's app-index owns terminal selection properly.
    if (entry.terminal) {
        QStringList term;
        const QByteArray configured = qgetenv("TERMINAL");
        if (!configured.isEmpty()) {
            term = tokenizeExec(QString::fromLocal8Bit(configured));
        } else if (!QStandardPaths::findExecutable(QStringLiteral("x-terminal-emulator"))
                        .isEmpty()) {
            term << QStringLiteral("x-terminal-emulator");
        }
        if (!term.isEmpty()) {
            term << QStringLiteral("-e");
            argv = term + argv;
        }
    }
    return argv;
}

DesktopEntry DesktopEntryIndex::parse(const QString &id, const QString &contents)
{
    DesktopEntry entry;
    entry.id = id;

    bool inDesktopEntry = false;
    const QStringList lines = contents.split(QLatin1Char('\n'));
    for (const QString &rawLine : lines) {
        const QString line = rawLine.trimmed();
        if (line.isEmpty() || line.startsWith(QLatin1Char('#')))
            continue;
        if (line.startsWith(QLatin1Char('[')) && line.endsWith(QLatin1Char(']'))) {
            inDesktopEntry = (line == QLatin1String("[Desktop Entry]"));
            continue;
        }
        if (!inDesktopEntry)
            continue;

        const qsizetype eq = line.indexOf(QLatin1Char('='));
        if (eq <= 0)
            continue;
        const QString key = line.left(eq);
        const QString value = line.mid(eq + 1).trimmed();
        // Localized keys carry a `[locale]` suffix; keep the unlocalized value.
        if (key.contains(QLatin1Char('[')))
            continue;

        if (key == QLatin1String("Name")) {
            entry.name = value;
        } else if (key == QLatin1String("Icon")) {
            entry.icon = value;
        } else if (key == QLatin1String("Exec")) {
            entry.exec = value;
        } else if (key == QLatin1String("StartupWMClass")) {
            entry.startupWmClass = value;
        } else if (key == QLatin1String("Categories")) {
            entry.categories =
                value.split(QLatin1Char(';'), Qt::SkipEmptyParts);
        } else if (key == QLatin1String("Terminal")) {
            entry.terminal = (value.compare(QLatin1String("true"), Qt::CaseInsensitive) == 0);
        } else if (key == QLatin1String("NoDisplay") || key == QLatin1String("Hidden")) {
            if (value.compare(QLatin1String("true"), Qt::CaseInsensitive) == 0)
                entry.noDisplay = true;
        }
    }

    entry.valid = true;
    if (entry.name.isEmpty())
        entry.name = id.endsWith(QLatin1String(".desktop")) ? id.chopped(8) : id;
    return entry;
}
