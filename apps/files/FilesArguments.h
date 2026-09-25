// SPDX-License-Identifier: MIT
// The location a Files invocation should open (T-10.6b).
//
// The Dock launches Files with the target URI as a positional argument
// (`dragonfruit-files trash://`, T-10.6a), so the app must not depend on the
// capture-only `DF_FILES_START_URI` environment seam. This header is the one
// place the positional argument is interpreted; it is header-only so the unit
// test can exercise it without pulling in the QML plugin.
#pragma once

#include <QFileInfo>
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
inline QString filesLocationArgument(const QStringList &arguments)
{
    for (int i = 1; i < arguments.size(); ++i) {
        const QString &argument = arguments.at(i);
        const QUrl url(argument);
        if (url.isValid() && !url.scheme().isEmpty())
            return argument;
        if (argument.startsWith(QLatin1Char('/')))
            return argument;
    }
    return QString();
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