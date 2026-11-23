// SPDX-License-Identifier: MIT
// The launch-time Xwayland `DISPLAY` hand-off (T309).
//
// Xwayland starts after the session entry has exported the startup
// environment, so `DISPLAY` is not a value the session can hand out once and
// forget. The compositor writes `$XDG_RUNTIME_DIR/<socket>.x11-display` when
// Xwayland is ready and removes it at teardown; a launcher reads it at the
// moment it starts a child, never once at startup, so a late Xwayland start
// still reaches the next launch.
//
// The read is deliberately cheap and uncached: it is one small file open per
// launched process. A stale `DISPLAY` inherited from the parent must not leak
// into a Wayland-only launch, so an absent or empty file removes the variable
// instead of leaving the inherited value in place.
#pragma once

#include <QDir>
#include <QFile>
#include <QProcessEnvironment>
#include <QString>

namespace Dragonfruit {

// The Xwayland display named by the hand-off file, or an empty string when
// the file is absent, unreadable, or empty. `waylandDisplay` names the
// compositor socket the caller actually connected to; when it is empty the
// environment's `WAYLAND_DISPLAY` is used. The file's contents are trimmed, so
// the compositor's trailing newline is not part of the value.
inline QString x11DisplayFromHandoff(const QProcessEnvironment &environment,
                                     const QString &waylandDisplay = QString())
{
    QString socket = waylandDisplay;
    if (socket.isEmpty())
        socket = environment.value(QStringLiteral("WAYLAND_DISPLAY"));
    const QString runtimeDir = environment.value(QStringLiteral("XDG_RUNTIME_DIR"));
    if (socket.isEmpty() || runtimeDir.isEmpty())
        return QString();

    QFile file(QDir(runtimeDir).filePath(socket + QStringLiteral(".x11-display")));
    if (!file.open(QIODevice::ReadOnly))
        return QString();
    return QString::fromUtf8(file.readAll()).trimmed();
}

// Apply the hand-off to a child environment: set `DISPLAY` when the file
// names one, remove an inherited value otherwise. Call this immediately before
// starting the child so the read is as fresh as possible.
inline void applyX11DisplayHandoff(QProcessEnvironment &environment,
                                   const QString &waylandDisplay = QString())
{
    const QString display = x11DisplayFromHandoff(environment, waylandDisplay);
    if (display.isEmpty())
        environment.remove(QStringLiteral("DISPLAY"));
    else
        environment.insert(QStringLiteral("DISPLAY"), display);
}

} // namespace Dragonfruit
