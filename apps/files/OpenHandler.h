// SPDX-License-Identifier: MIT
// Opening a file from the Files-owned desktop (T-19.3).
//
// The desktop process forces `QT_QPA_PLATFORM=offscreen` before QGuiApplication
// because it speaks the private compositor protocol itself instead of creating
// a Qt Wayland toplevel. `QDesktopServices::openUrl` is implemented by the
// platform plugin, and the offscreen plugin has no `QPlatformServices`: it
// logs
//
//   This plugin does not support QPlatformServices::openDocument() for '…'
//
// and opens nothing. The desktop must therefore ask a separate, normally
// platformed process to open the file. `xdg-open` is the freedesktop entry
// point Qt itself uses on Unix; `gio open` is the fallback when it is absent.
#pragma once

#include <QProcessEnvironment>
#include <QStandardPaths>
#include <QString>
#include <QStringList>

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

// The argv that opens `uri` with the desktop's default handler, or an empty
// list when neither opener is installed. The executable is resolved now (on
// the desktop's normal PATH) so the child does not depend on its own lookup.
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
