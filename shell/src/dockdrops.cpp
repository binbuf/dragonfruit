// SPDX-License-Identifier: MIT
#include "dockdrops.h"

#include <QDir>
#include <QFileInfo>
#include <QList>
#include <QUrl>

QStringList parseUriList(const QByteArray &data)
{
    QStringList paths;
    const QList<QByteArray> lines = data.split('\n');
    for (QByteArray line : lines) {
        if (line.endsWith('\r'))
            line.chop(1);
        const QByteArray trimmed = line.trimmed();
        if (trimmed.isEmpty() || trimmed.startsWith('#'))
            continue;
        const QUrl url = QUrl::fromEncoded(trimmed);
        if (!url.isValid() || url.scheme().compare(QLatin1String("file"), Qt::CaseInsensitive) != 0)
            continue;
        const QString local = url.toLocalFile();
        if (!local.isEmpty())
            paths.append(local);
    }
    return paths;
}

DockDropPayloadData parseDockDropPayload(const QString &mime, const QByteArray &data)
{
    DockDropPayloadData out;
    if (mime == QLatin1String("application/x-dragonfruit-app")) {
        const QString id = QString::fromUtf8(data).trimmed();
        if (id.isEmpty())
            return out;
        out.kind = DockDropPayload::Application;
        out.desktopId = id;
        out.valid = true;
        return out;
    }
    if (mime == QLatin1String("text/uri-list")) {
        out.paths = parseUriList(data);
        if (uriListIsApplication(out.paths)) {
            out.kind = DockDropPayload::Application;
            out.desktopId = desktopIdForFile(out.paths.first());
            out.paths.clear();
        } else if (uriListIsFolder(out.paths)) {
            // A single directory pins as a folder stack (T-14.7k); the path
            // stays in `paths` for the PinFolder action.
            out.kind = DockDropPayload::Folder;
        } else {
            out.kind = DockDropPayload::Files;
        }
        out.valid = true;
        return out;
    }
    return out;
}

bool uriListIsApplication(const QStringList &paths)
{
    return paths.size() == 1
           && paths.first().endsWith(QLatin1String(".desktop"), Qt::CaseInsensitive);
}

bool uriListIsFolder(const QStringList &paths)
{
    if (paths.size() != 1)
        return false;
    // A folder pin only ever holds an absolute, existing local directory
    // (ADR 0092 security rule); a `files` payload that happens to be one
    // directory is still promoted.
    const QFileInfo info(paths.first());
    return info.isAbsolute() && info.isDir();
}

QString desktopIdForFile(const QString &path)
{
    if (!path.endsWith(QLatin1String(".desktop"), Qt::CaseInsensitive))
        return QString();
    return QFileInfo(path).fileName();
}

DockDropAction dockDropActionFor(const QString &targetKind, DockDropPayload payload)
{
    if (payload == DockDropPayload::Application) {
        // An application alias pins the app wherever it lands on the Dock,
        // except on a target that cannot hold a pin (divider, Trash, stack).
        if (targetKind.isEmpty() || targetKind == QLatin1String("pinned")
            || targetKind == QLatin1String("temporary")
            || targetKind == QLatin1String("recent")) {
            return DockDropAction::PinApp;
        }
        return DockDropAction::None;
    }

    if (payload == DockDropPayload::Folder) {
        // A folder pins on the app region; on a stack it moves in like any
        // other item, and on the Trash it trashes.
        if (targetKind == QLatin1String("trash"))
            return DockDropAction::TrashFiles;
        if (targetKind == QLatin1String("stack"))
            return DockDropAction::MoveToFolder;
        if (targetKind.isEmpty() || targetKind == QLatin1String("pinned")
            || targetKind == QLatin1String("temporary")
            || targetKind == QLatin1String("recent")) {
            return DockDropAction::PinFolder;
        }
        return DockDropAction::None;
    }

    // Files: the target decides.
    if (targetKind == QLatin1String("trash"))
        return DockDropAction::TrashFiles;
    if (targetKind == QLatin1String("stack"))
        return DockDropAction::MoveToFolder;
    if (targetKind == QLatin1String("pinned") || targetKind == QLatin1String("temporary")
        || targetKind == QLatin1String("recent")) {
        return DockDropAction::OpenWithApp;
    }
    return DockDropAction::None;
}

QString dockDropAffordance(const QString &targetKind, DockDropPayload payload,
                           const QString &targetName, bool trashAvailable)
{
    if (payload == DockDropPayload::Application) {
        // A pin never lands on Trash, a folder stack, or the divider.
        if (targetKind == QLatin1String("trash") || targetKind == QLatin1String("stack")
            || targetKind == QLatin1String("divider"))
            return QString();
        return QStringLiteral("Add to Dock");
    }
    if (payload == DockDropPayload::Folder) {
        if (targetKind == QLatin1String("trash"))
            return trashAvailable ? QStringLiteral("Move to Trash")
                                  : QStringLiteral("Trash unavailable");
        if (targetKind == QLatin1String("stack"))
            return targetName.isEmpty() ? QStringLiteral("Move to Folder")
                                        : QStringLiteral("Move to %1").arg(targetName);
        if (targetKind == QLatin1String("divider"))
            return QString();
        return QStringLiteral("Pin Folder");
    }
    if (targetKind == QLatin1String("trash"))
        return trashAvailable ? QStringLiteral("Move to Trash")
                              : QStringLiteral("Trash unavailable");
    if (targetKind == QLatin1String("stack"))
        return targetName.isEmpty() ? QStringLiteral("Move to Folder")
                                    : QStringLiteral("Move to %1").arg(targetName);
    if (targetKind == QLatin1String("pinned") || targetKind == QLatin1String("temporary")
        || targetKind == QLatin1String("recent")) {
        return targetName.isEmpty()
            ? QStringLiteral("Open with")
            : QStringLiteral("Open with %1").arg(targetName);
    }
    return QString();
}

bool dockPinnedContains(const QStringList &pinned, const QString &id,
                        const QString &resolvedId)
{
    if (id.isEmpty())
        return false;
    if (pinned.contains(id))
        return true;
    return !resolvedId.isEmpty() && resolvedId != id && pinned.contains(resolvedId);
}

QString downloadsDirectory()
{
    const QByteArray configured = qgetenv("XDG_DOWNLOAD_DIR");
    if (!configured.isEmpty()) {
        QString dir = QString::fromLocal8Bit(configured);
        dir.replace(QLatin1String("$HOME"), QDir::homePath());
        return dir;
    }
    return QDir::homePath() + QStringLiteral("/Downloads");
}
