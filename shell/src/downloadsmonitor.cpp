// SPDX-License-Identifier: MIT
#include "downloadsmonitor.h"

#include "dockdrops.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFileSystemWatcher>
#include <QSet>
#include <QVariantMap>

namespace {

// Move a file or directory, falling back to copy+remove across filesystems.
bool movePath(const QString &src, const QString &dst)
{
    if (QFile::rename(src, dst))
        return true;
    const QFileInfo info(src);
    if (info.isDir() && !info.isSymLink()) {
        if (!QDir().mkpath(dst))
            return false;
        const QDir dir(src);
        const QFileInfoList entries =
            dir.entryInfoList(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden);
        for (const QFileInfo &entry : entries) {
            if (!movePath(entry.absoluteFilePath(), dst + QLatin1Char('/') + entry.fileName()))
                return false;
        }
        return QDir(src).removeRecursively();
    }
    if (QFile::copy(src, dst))
        return QFile::remove(src);
    return false;
}

// A free name inside `dir` for `name`: `name`, else `name.N`.
QString uniqueName(const QString &dir, const QString &name)
{
    if (!QFileInfo::exists(dir + QLatin1Char('/') + name))
        return name;
    QString stem = name;
    QString extension;
    const qsizetype dot = name.lastIndexOf(QLatin1Char('.'));
    if (dot > 0) {
        stem = name.left(dot);
        extension = name.mid(dot);
    }
    for (int i = 1;; ++i) {
        const QString candidate =
            stem + QLatin1Char('.') + QString::number(i) + extension;
        if (!QFileInfo::exists(dir + QLatin1Char('/') + candidate))
            return candidate;
    }
}

} // namespace

DownloadsMonitor::DownloadsMonitor(const QString &directory, QObject *parent)
    : QObject(parent)
    , m_directory(QDir::cleanPath(directory))
{
    m_watcher = new QFileSystemWatcher(this);
    connect(m_watcher, &QFileSystemWatcher::directoryChanged, this,
            &DownloadsMonitor::onDirectoryChanged);
}

QString DownloadsMonitor::defaultDirectory()
{
    return downloadsDirectory();
}

void DownloadsMonitor::start()
{
    if (m_watching)
        return;
    m_watching = true;
    rescan(false);
    syncWatches();
}

void DownloadsMonitor::stop()
{
    if (!m_watching)
        return;
    m_watching = false;
    if (!m_watcher->directories().isEmpty())
        m_watcher->removePaths(m_watcher->directories());
}

void DownloadsMonitor::refresh()
{
    rescan(true);
    if (m_watching)
        syncWatches();
}

void DownloadsMonitor::syncWatches()
{
    if (!QFileInfo::exists(m_directory))
        return;
    if (!m_watcher->directories().contains(m_directory) && !m_watcher->files().contains(m_directory))
        m_watcher->addPath(m_directory);
}

void DownloadsMonitor::onDirectoryChanged(const QString &)
{
    rescan(true);
    syncWatches();
}

void DownloadsMonitor::rescan(bool emitSignal)
{
    QFileInfoList entries;
    const QDir dir(m_directory);
    if (dir.exists()) {
        entries = dir.entryInfoList(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden,
                                    QDir::Time | QDir::DirsLast);
    }

    QVariantList items;
    items.reserve(entries.size());
    QSet<QString> names;
    for (const QFileInfo &entry : entries) {
        names.insert(entry.fileName());
        QVariantMap item;
        item.insert(QStringLiteral("name"), entry.fileName());
        item.insert(QStringLiteral("path"), entry.absoluteFilePath());
        item.insert(QStringLiteral("isDir"), entry.isDir() && !entry.isSymLink());
        items.append(item);
    }

    // Count items that appeared since the previous scan. The first scan only
    // establishes the baseline, so a pre-existing folder has no badge.
    int added = 0;
    if (m_seenOnce) {
        for (const QString &name : names) {
            if (!m_previous.contains(name))
                ++added;
        }
    }
    m_previous = names;
    m_seenOnce = true;

    const bool listingChanged = items != m_items;
    const bool badgeChanged = added > 0;
    m_items = items;
    if (badgeChanged)
        m_newCount += added;
    if (emitSignal && (listingChanged || badgeChanged))
        emit changed();
}

void DownloadsMonitor::markSeen()
{
    if (m_newCount == 0)
        return;
    m_newCount = 0;
    emit changed();
}

int DownloadsMonitor::moveIn(const QStringList &paths)
{
    m_error.clear();
    const QString root = QDir(m_directory).absolutePath();
    if (root.isEmpty() || root == QLatin1String("/")) {
        m_error = QStringLiteral("refusing to move into an unsafe downloads root");
        return 0;
    }
    QDir().mkpath(root);

    int moved = 0;
    for (const QString &path : paths) {
        if (path.isEmpty())
            continue;
        const QFileInfo info(path);
        const QString absolute = info.absoluteFilePath();
        if (absolute == root || absolute.startsWith(root + QLatin1Char('/'))) {
            m_error = QStringLiteral("refusing to move %1 into itself").arg(absolute);
            continue;
        }
        if (!info.exists() && !info.isSymLink()) {
            m_error = QStringLiteral("no such file: %1").arg(absolute);
            continue;
        }
        const QString name = uniqueName(root, info.fileName());
        if (!movePath(absolute, root + QLatin1Char('/') + name)) {
            m_error = QStringLiteral("cannot move %1 to Downloads").arg(absolute);
            continue;
        }
        ++moved;
    }

    refresh();
    return moved;
}
