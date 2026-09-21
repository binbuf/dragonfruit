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

bool uriListIsApplication(const QStringList &paths)
{
    return paths.size() == 1
           && paths.first().endsWith(QLatin1String(".desktop"), Qt::CaseInsensitive);
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

    // Files: the target decides.
    if (targetKind == QLatin1String("trash"))
        return DockDropAction::TrashFiles;
    if (targetKind == QLatin1String("stack"))
        return DockDropAction::MoveToDownloads;
    if (targetKind == QLatin1String("pinned") || targetKind == QLatin1String("temporary")
        || targetKind == QLatin1String("recent")) {
        return DockDropAction::OpenWithApp;
    }
    return DockDropAction::None;
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
