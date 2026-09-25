// SPDX-License-Identifier: MIT
// The location a Files invocation should open (T-10.6b).
//
// The Dock launches Files with the target URI as a positional argument
// (`dragonfruit-files trash://`, T-10.6a), so the app must not depend on the
// capture-only `DF_FILES_START_URI` environment seam. This header is the one
// place the positional argument is interpreted; it is header-only so the unit
// test can exercise it without pulling in the QML plugin.
#pragma once

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