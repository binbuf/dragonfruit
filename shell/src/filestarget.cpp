// SPDX-License-Identifier: MIT
#include "filestarget.h"

#include <QFileInfo>
#include <QStandardPaths>

QString revealExecutable(const DesktopEntry &entry)
{
    const QStringList argv = DesktopEntryIndex::buildLaunchCommand(entry);
    if (argv.isEmpty())
        return QString();
    const QString program = argv.first();
    if (program.isEmpty())
        return QString();

    const QFileInfo info(program);
    if (info.isAbsolute())
        return info.absoluteFilePath();
    // `buildLaunchCommand` wraps a Terminal=true app in an emulator; the
    // wrapper is not the app, so there is no executable of the app's own to
    // reveal.
    if (entry.terminal)
        return QString();
    return QStandardPaths::findExecutable(program);
}