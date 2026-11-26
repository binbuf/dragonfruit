// SPDX-License-Identifier: GPL-3.0-or-later
// The location a Files invocation should open (T-10.6b).
//
// The Dock launches Files with the target URI as a positional argument
// (`dragonfruit-files trash://`, T-10.6a), so the app must not depend on the
// capture-only `DF_FILES_START_URI` environment seam. This header is the one
// place the positional argument is interpreted; it is header-only so the unit
// test can exercise it without pulling in the QML plugin.
#pragma once

#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>
#include <QString>
#include <QStringList>
#include <QUrl>

// The location argument (`trash://`, `file:///…`, or an absolute path), or an
// empty string when there is none.
//
// Only a URI with a scheme or an absolute path is accepted, so Qt's own
// options and their separate values (`-platform offscreen`) are never mistaken
// for the location. An empty result means "open Home", which the QML shell
// already does when `DF_FILES_START_URI` is unset.
// The location a launch names is either one of the options T333 added
// (`--location=`, `--activation-handoff=`, `--new-window`) or a positional URI;
// an option's value is never a location even when it parses as a URI.
inline bool filesLocationOption(const QString &argument)
{
    return argument.startsWith(QStringLiteral("--location="))
            || argument.startsWith(QStringLiteral("--activation-handoff="))
            || argument == QStringLiteral("--new-window");
}

inline QString filesLocationArgument(const QStringList &arguments)
{
    for (int i = 1; i < arguments.size(); ++i) {
        const QString &argument = arguments.at(i);
        if (filesLocationOption(argument))
            continue;
        const QUrl url(argument);
        if (url.isValid() && !url.scheme().isEmpty())
            return argument;
        if (argument.startsWith(QLatin1Char('/')))
            return argument;
    }
    return QString();
}

// Whether this invocation opens a Files window *because* a component asked
// for a location (`--activation-handoff=<token>` plus the location), which the
// entry point may hand to an already-running instance instead of opening a
// window in this process (T333's single-instance rule). A bare invocation and
// the Dock's plain positional `trash://` are never handed off: a Dock click
// always opens a window. The location is read by `filesLocationArgument`
// (the first non-option argument) or from a `--location=<uri>` option.
inline bool filesActivationRequested(const QStringList &arguments)
{
    for (int i = 1; i < arguments.size(); ++i) {
        if (arguments.at(i).startsWith(QStringLiteral("--activation-handoff=")))
            return true;
    }
    return false;
}

// The location a hand-off request names. An empty result means "show the
// current window" (the holder raises its windows).
inline QString filesHandoffLocation(const QStringList &arguments)
{
    for (int i = 1; i < arguments.size(); ++i) {
        const QString &argument = arguments.at(i);
        if (argument.startsWith(QStringLiteral("--activation-handoff=")))
            continue;
        if (argument.startsWith(QStringLiteral("--location="))) {
            const QString value = argument.mid(QStringLiteral("--location=").size());
            if (value.isEmpty())
                return QString();
            const QUrl url(value);
            if (url.isValid() && !url.scheme().isEmpty())
                return value;
            return QUrl::fromLocalFile(QFileInfo(value).absoluteFilePath()).toString();
        }
        if (filesLocationOption(argument))
            continue;
        const QString location = filesLocationArgument({QString(), argument});
        if (!location.isEmpty())
            return location;
    }
    return QString();
}

// Whether this invocation is the Files-owned desktop (T-19.3):
// `dragonfruit-files --desktop`. The desktop process renders `~/Desktop` on a
// compositor background-layer surface and is independent of the browser
// process (its own launch token, its own crash domain).
inline bool filesDesktopRequested(const QStringList &arguments)
{
    return arguments.contains(QStringLiteral("--desktop"));
}

// The directory the Files desktop browses: the user's Desktop, resolved
// through `xdg-user-dirs` (Qt's standard location), falling back to
// `$HOME/Desktop` when the location is unset. Returned as a `file://` URI for
// the files-core model, matching the browser's location form.
inline QString filesDesktopDirectory()
{
    QString path = QStandardPaths::writableLocation(QStandardPaths::DesktopLocation);
    if (path.isEmpty())
        path = QDir::homePath() + QStringLiteral("/Desktop");
    return QUrl::fromLocalFile(path).toString();
}

// What a Files invocation should open: `location` is the directory to browse,
// `revealUri` the item inside it to select first (empty for none). T-10.6c:
// the Dock passes an app's executable or a Downloads-stack file, so a `file://`
// path that names a file opens its parent folder and reveals the file; a
// directory browses itself; a non-file scheme (`trash://`) browses itself.
struct FilesOpenTarget {
    QString location;
    QString revealUri;
};

inline FilesOpenTarget filesOpenTarget(const QString &argument)
{
    FilesOpenTarget target;
    if (argument.isEmpty())
        return target;

    const QUrl url(argument);
    const bool local = url.scheme() == QStringLiteral("file") || !url.isValid()
                       || url.scheme().isEmpty();
    if (local) {
        const QString path = url.scheme() == QStringLiteral("file")
                                 ? url.toLocalFile()
                                 : argument;
        const QFileInfo info(path);
        if (info.isDir()) {
            target.location = QUrl::fromLocalFile(info.absoluteFilePath()).toString();
        } else if (info.isFile()) {
            target.location = QUrl::fromLocalFile(info.absolutePath()).toString();
            target.revealUri = QUrl::fromLocalFile(info.absoluteFilePath()).toString();
        } else {
            // A path that does not exist: browse it as given so the shell can
            // show its own error state rather than silently opening Home.
            target.location = argument;
        }
        return target;
    }

    target.location = argument;
    return target;
}
// T333's launch forms for the first-party window menu and other components:
//   Files > New Window   -> `--new-window [--location=<folder>]` (a second
//                           process)
//   Files > New Tab      -> `--activation-handoff=<token> [--location=<folder>]`
//                           (handed to the running instance)
// The location a launch names, in any form.
inline QString filesLaunchLocation(const QStringList &arguments)
{
    const QString handoff = filesHandoffLocation(arguments);
    return handoff.isEmpty() ? filesLocationArgument(arguments) : handoff;
}

// Whether the launch must open a window even when a component hand-off
// advertises an offer (`--new-window`, or a file opened into a new window).
inline bool filesNewWindowRequested(const QStringList &arguments)
{
    return arguments.contains(QStringLiteral("--new-window"));
}
