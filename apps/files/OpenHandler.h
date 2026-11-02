// SPDX-License-Identifier: MIT
// Opening a file with the user's default application.
//
// The browser process can ask the platform, but the Files-owned desktop is an
// offscreen-platform process (it speaks the private compositor protocol
// itself). `QDesktopServices::openUrl` there has no `QPlatformServices` and
// opens nothing, and `xdg-open` can silently fail when `mimeapps.list` names
// an application that is no longer installed. Both callers therefore use
// `filesOpen()`: it resolves the MIME's default desktop entry, falls back to
// the first installed registered handler, and only then uses the generic
// `xdg-open`/`gio open` path. The launched child never inherits the desktop's
// forced offscreen platform.
#pragma once

#include <QDir>
#include <QFileInfo>
#include <QMimeDatabase>
#include <QProcess>
#include <QProcessEnvironment>
#include <QRegularExpression>
#include <QSettings>
#include <QStandardPaths>
#include <QString>
#include <QStringList>
#include <QUrl>

// The environment for a Files browser or default handler the offscreen
// desktop launches. The desktop's forced `QT_QPA_PLATFORM=offscreen` must not
// leak into the child: a browser would paint an invisible offscreen window,
// and the handler would inherit the same missing `QPlatformServices`.
inline QProcessEnvironment filesDesktopChildEnvironment()
{
    QProcessEnvironment environment = QProcessEnvironment::systemEnvironment();
    environment.remove(QStringLiteral("QT_QPA_PLATFORM"));
    return environment;
}

// The argv for the generic freedesktop opener, or an empty list when neither
// opener is installed. The executable is resolved now so the child does not
// depend on its own lookup.
inline QStringList filesOpenHandlerCommand(const QString &uri)
{
    if (uri.isEmpty())
        return {};

    const QString xdgOpen =
            QStandardPaths::findExecutable(QStringLiteral("xdg-open"));
    if (!xdgOpen.isEmpty())
        return {xdgOpen, uri};

    const QString gio = QStandardPaths::findExecutable(QStringLiteral("gio"));
    if (!gio.isEmpty())
        return {gio, QStringLiteral("open"), uri};

    return {};
}

// Run a small desktop utility (`xdg-mime`, `gio mime`) with a stable C locale
// so its output is parseable. These commands are local metadata lookups and
// finish in milliseconds; the generous timeout is only a safety net.
inline QString filesToolOutput(const QString &program, const QStringList &arguments)
{
    const QString executable = QStandardPaths::findExecutable(program);
    if (executable.isEmpty())
        return {};

    QProcess process;
    QProcessEnvironment environment = filesDesktopChildEnvironment();
    environment.insert(QStringLiteral("LC_ALL"), QStringLiteral("C"));
    process.setProcessEnvironment(environment);
    process.setProgram(executable);
    process.setArguments(arguments);
    process.start();
    if (!process.waitForStarted(1000))
        return {};
    if (!process.waitForFinished(2000)) {
        process.kill();
        process.waitForFinished(200);
        return {};
    }
    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0)
        return {};
    return QString::fromUtf8(process.readAllStandardOutput()).trimmed();
}

inline QString filesDesktopFileForId(const QString &id)
{
    if (id.isEmpty())
        return {};

    const QFileInfo direct(id);
    if (direct.isAbsolute() && direct.isFile())
        return direct.absoluteFilePath();

    QStringList directories =
            QStandardPaths::standardLocations(QStandardPaths::ApplicationsLocation);
    // Qt follows XDG_DATA_DIRS, which normally already contains the Flatpak
    // and Snap export directories. Keep the common locations as a fallback
    // for sessions that launch the app with a trimmed environment.
    directories << QDir::homePath()
                           + QStringLiteral("/.local/share/flatpak/exports/share/applications")
                << QStringLiteral("/var/lib/flatpak/exports/share/applications")
                << QStringLiteral("/var/lib/snapd/desktop/applications");

    for (const QString &directory : directories) {
        const QString path = directory + QLatin1Char('/') + id;
        if (QFileInfo(path).isFile())
            return path;
    }
    return {};
}

// Whether a desktop entry can actually be launched here: it is an
// application (not a hidden link), its TryExec/Exec resolve, and it does not
// require a terminal we do not own.
inline bool filesDesktopFileCanLaunch(const QString &path)
{
    QSettings desktop(path, QSettings::IniFormat);
    desktop.beginGroup(QStringLiteral("Desktop Entry"));

    if (desktop.value(QStringLiteral("Hidden"), false).toBool())
        return false;
    const QString type = desktop.value(QStringLiteral("Type")).toString();
    if (!type.isEmpty() && type != QStringLiteral("Application"))
        return false;
    if (desktop.value(QStringLiteral("Terminal"), false).toBool())
        return false;

    const QString tryExec = desktop.value(QStringLiteral("TryExec")).toString();
    if (!tryExec.isEmpty()) {
        const QFileInfo direct(tryExec);
        if (!(direct.isAbsolute() && direct.isExecutable())
            && QStandardPaths::findExecutable(tryExec).isEmpty()) {
            return false;
        }
    }

    const QString exec = desktop.value(QStringLiteral("Exec")).toString();
    if (exec.isEmpty())
        return false;
    QStringList tokens = QProcess::splitCommand(exec);
    int index = 0;
    // `Exec=env FOO=bar program ...` is common; validate the real program.
    if (index < tokens.size() && tokens[index] == QStringLiteral("env")) {
        ++index;
        while (index < tokens.size() && tokens[index].contains(QLatin1Char('=')))
            ++index;
    }
    if (index >= tokens.size())
        return false;
    const QString program = tokens[index];
    if (program.contains(QLatin1Char('%')))
        return false;
    const QFileInfo direct(program);
    if (direct.isAbsolute())
        return direct.isExecutable();
    return !QStandardPaths::findExecutable(program).isEmpty();
}

inline QStringList filesRegisteredDesktopIds(const QString &mime)
{
    const QString output = filesToolOutput(QStringLiteral("gio"),
                                           {QStringLiteral("mime"), mime});
    QStringList ids;
    // Registered rows are indented desktop ids; the "Default application
    // for …" line has prose before the id and does not match.
    static const QRegularExpression row(
            QStringLiteral("^\\s*([A-Za-z0-9_.+-]+\\.desktop)\\s*$"));
    const QStringList lines = output.split(QLatin1Char('\n'));
    for (const QString &line : lines) {
        const QRegularExpressionMatch match = row.match(line);
        if (!match.hasMatch())
            continue;
        const QString id = match.captured(1);
        if (!ids.contains(id))
            ids.append(id);
    }
    return ids;
}

inline QString filesDefaultDesktopId(const QString &mime)
{
    // A stale `mimeapps.list` can name a desktop file that is no longer
    // installed (common after switching app bundles). Prefer the configured
    // default only when it really exists and can launch; otherwise fall back
    // to the first installed registered handler.
    QString chosen;
    const QString configured =
            filesToolOutput(QStringLiteral("xdg-mime"),
                            {QStringLiteral("query"), QStringLiteral("default"), mime});
    if (!configured.isEmpty()) {
        const QString path = filesDesktopFileForId(configured);
        if (!path.isEmpty() && filesDesktopFileCanLaunch(path))
            chosen = configured;
    }
    if (chosen.isEmpty()) {
        const QStringList candidates = filesRegisteredDesktopIds(mime);
        for (const QString &id : candidates) {
            if (id == QStringLiteral("org.dragonfruit.Files.desktop"))
                continue;
            const QString path = filesDesktopFileForId(id);
            if (!path.isEmpty() && filesDesktopFileCanLaunch(path)) {
                chosen = id;
                break;
            }
        }
    }
    return chosen;
}

inline bool filesStartDetached(const QStringList &command)
{
    if (command.isEmpty())
        return false;
    QProcess process;
    process.setProcessEnvironment(filesDesktopChildEnvironment());
    process.setProgram(command.first());
    process.setArguments(command.mid(1));
    return process.startDetached();
}

inline bool filesStartDesktopFile(const QString &desktopPath, const QString &filePath)
{
    // `gio launch` takes the full desktop-file path, so it also works for
    // Flatpak/Snap exports outside the standard application directories.
    // `gtk-launch` is the fallback when gio is absent.
    const QString gio = QStandardPaths::findExecutable(QStringLiteral("gio"));
    if (!gio.isEmpty()) {
        QProcess process;
        process.setProcessEnvironment(filesDesktopChildEnvironment());
        process.setProgram(gio);
        process.setArguments({QStringLiteral("launch"), desktopPath, filePath});
        if (process.startDetached())
            return true;
    }
    const QString gtkLaunch =
            QStandardPaths::findExecutable(QStringLiteral("gtk-launch"));
    if (!gtkLaunch.isEmpty()) {
        QProcess process;
        process.setProcessEnvironment(filesDesktopChildEnvironment());
        process.setProgram(gtkLaunch);
        process.setArguments({QFileInfo(desktopPath).fileName(), filePath});
        if (process.startDetached())
            return true;
    }
    return false;
}

// Open `uri` with the default application. Local files resolve through the
// MIME desktop database first; everything else uses the generic opener.
inline bool filesOpen(const QString &uri)
{
    if (uri.isEmpty())
        return false;

    const QUrl url(uri);
    if (!url.isLocalFile())
        return filesStartDetached(filesOpenHandlerCommand(uri));

    const QFileInfo info(url.toLocalFile());
    if (!info.isFile())
        return filesStartDetached(filesOpenHandlerCommand(uri));

    const QMimeType mime = QMimeDatabase().mimeTypeForFile(info);
    if (mime.isValid()) {
        const QString desktopId = filesDefaultDesktopId(mime.name());
        if (!desktopId.isEmpty()) {
            const QString desktopPath = filesDesktopFileForId(desktopId);
            if (!desktopPath.isEmpty()
                && filesStartDesktopFile(desktopPath, info.absoluteFilePath())) {
                return true;
            }
        }
    }

    // The MIME database or the desktop files may be incomplete; the generic
    // opener is still a useful last resort.
    return filesStartDetached(filesOpenHandlerCommand(uri));
}
