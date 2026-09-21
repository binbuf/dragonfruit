// SPDX-License-Identifier: MIT
#include "trashmonitor.h"

#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFileSystemWatcher>
#include <QSet>
#include <QStandardPaths>
#include <QUrl>

namespace {

const QString kInfoSuffix = QStringLiteral(".trashinfo");

QString infoPath(const QString &root)
{
    return root + QStringLiteral("/info");
}

QString filesPath(const QString &root)
{
    return root + QStringLiteral("/files");
}

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

// A free name inside `filesDir` for `name`: `name`, else `name.N` (T-10
// section 16, freedesktop Trash spec).
QString uniqueName(const QString &filesDir, const QString &name)
{
    if (!QFileInfo::exists(filesDir + QLatin1Char('/') + name))
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
        if (!QFileInfo::exists(filesDir + QLatin1Char('/') + candidate))
            return candidate;
    }
}

} // namespace

TrashMonitor::TrashMonitor(const QString &root, QObject *parent)
    : QObject(parent)
    , m_root(QDir::cleanPath(root))
{
    m_watcher = new QFileSystemWatcher(this);
    connect(m_watcher, &QFileSystemWatcher::directoryChanged, this,
            &TrashMonitor::onDirectoryChanged);
}

QString TrashMonitor::defaultRoot()
{
    const QString data = QStandardPaths::writableLocation(QStandardPaths::GenericDataLocation);
    if (data.isEmpty())
        return QDir::homePath() + QStringLiteral("/.local/share/Trash");
    return data + QStringLiteral("/Trash");
}

void TrashMonitor::start()
{
    if (m_watching)
        return;
    m_watching = true;
    rescan(false);
    syncWatches();
}

void TrashMonitor::stop()
{
    if (!m_watching)
        return;
    m_watching = false;
    if (!m_watcher->directories().isEmpty())
        m_watcher->removePaths(m_watcher->directories());
}

void TrashMonitor::refresh()
{
    rescan(true);
    if (m_watching)
        syncWatches();
}

void TrashMonitor::syncWatches()
{
    // Watch the three directories that can appear/disappear. A path that does
    // not exist cannot be watched; the parent watch re-syncs when it does.
    const QStringList wanted{infoPath(m_root), filesPath(m_root), m_root};
    for (const QString &path : wanted) {
        if (QFileInfo::exists(path) && !m_watcher->directories().contains(path)
                && !m_watcher->files().contains(path)) {
            m_watcher->addPath(path);
        }
    }
}

void TrashMonitor::onDirectoryChanged(const QString &)
{
    // A new/removed directory drops or adds watches; re-sync after the scan.
    rescan(true);
    syncWatches();
}

void TrashMonitor::rescan(bool emitSignal)
{
    // Count the union of `info/<name>.trashinfo` and `files/<name>` base names
    // so a partially-written pair still reads as non-empty.
    QSet<QString> names;
    const QDir infoDir(infoPath(m_root));
    const QFileInfoList infoFiles =
        infoDir.entryInfoList(QStringList() << QStringLiteral("*") + kInfoSuffix,
                              QDir::Files | QDir::NoDotAndDotDot | QDir::Hidden);
    for (const QFileInfo &file : infoFiles) {
        QString name = file.fileName();
        name.chop(kInfoSuffix.size());
        names.insert(name);
    }
    const QDir filesDir(filesPath(m_root));
    const QFileInfoList fileEntries =
        filesDir.entryInfoList(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden);
    for (const QFileInfo &entry : fileEntries)
        names.insert(entry.fileName());

    const int count = names.size();
    const bool available = computeAvailable();
    if (count == m_count && available == m_available)
        return;
    m_count = count;
    m_available = available;
    if (emitSignal)
        emit changed();
}

bool TrashMonitor::computeAvailable() const
{
    if (m_root.isEmpty() || m_root == QLatin1String("/"))
        return false;
    const QFileInfo rootInfo(m_root);
    if (rootInfo.exists())
        return rootInfo.isReadable();
    // A missing trash root is normal (an empty trash is created on demand), so
    // walk up to the nearest existing ancestor: if that is writable we could
    // create the trash; a read-only or unmounted ancestor means unavailable.
    QString path = rootInfo.absolutePath();
    while (!path.isEmpty()) {
        const QFileInfo info(path);
        if (info.exists())
            return info.isWritable();
        const QString parent = info.absolutePath();
        if (parent == path)
            break;
        path = parent;
    }
    return false;
}

int TrashMonitor::empty()
{
    m_error.clear();
    if (m_root.isEmpty() || m_root == QLatin1String("/")) {
        m_error = QStringLiteral("refusing to empty an unsafe trash root");
        return -1;
    }

    int removed = 0;
    // Remove the info records first so a crash mid-empty leaves no dangling
    // metadata; then remove the payloads. Never follow symlinks out of root.
    const QDir infoDir(infoPath(m_root));
    const QFileInfoList infoFiles =
        infoDir.entryInfoList(QStringList() << QStringLiteral("*") + kInfoSuffix,
                              QDir::Files | QDir::NoDotAndDotDot | QDir::Hidden);
    for (const QFileInfo &file : infoFiles) {
        if (QFile::remove(file.absoluteFilePath()))
            ++removed;
        else
            m_error = QStringLiteral("cannot remove %1").arg(file.absoluteFilePath());
    }

    const QDir filesDir(filesPath(m_root));
    const QFileInfoList fileEntries =
        filesDir.entryInfoList(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden);
    for (const QFileInfo &entry : fileEntries) {
        bool ok = false;
        if (entry.isSymLink() || entry.isFile())
            ok = QFile::remove(entry.absoluteFilePath());
        else if (entry.isDir())
            ok = QDir(entry.absoluteFilePath()).removeRecursively();
        if (ok)
            ++removed;
        else if (m_error.isEmpty())
            m_error = QStringLiteral("cannot remove %1").arg(entry.absoluteFilePath());
    }

    refresh();
    return removed;
}

int TrashMonitor::trash(const QStringList &paths)
{
    m_error.clear();
    if (m_root.isEmpty() || m_root == QLatin1String("/")) {
        m_error = QStringLiteral("refusing to trash into an unsafe root");
        return 0;
    }

    QDir().mkpath(infoPath(m_root));
    QDir().mkpath(filesPath(m_root));

    const QString root = QDir(m_root).absolutePath();
    int trashed = 0;
    for (const QString &path : paths) {
        if (path.isEmpty())
            continue;
        const QFileInfo info(path);
        const QString absolute = info.absoluteFilePath();
        // Never trash the trash itself or the filesystem root.
        if (absolute == root || absolute == QLatin1String("/")
            || absolute.startsWith(root + QLatin1Char('/'))) {
            m_error = QStringLiteral("refusing to trash %1").arg(absolute);
            continue;
        }
        if (!info.exists() && !info.isSymLink()) {
            m_error = QStringLiteral("no such file: %1").arg(absolute);
            continue;
        }

        const QString name = uniqueName(filesPath(m_root), info.fileName());
        const QString destination = filesPath(m_root) + QLatin1Char('/') + name;
        if (!movePath(absolute, destination)) {
            m_error = QStringLiteral("cannot move %1 to the trash").arg(absolute);
            continue;
        }

        QFile record(infoPath(m_root) + QLatin1Char('/') + name + kInfoSuffix);
        if (!record.open(QIODevice::WriteOnly | QIODevice::Truncate)) {
            m_error = QStringLiteral("cannot write %1").arg(record.fileName());
            // Roll the payload back so the trash stays internally consistent.
            movePath(destination, absolute);
            continue;
        }
        const QString encoded = QString::fromLatin1(QUrl::toPercentEncoding(absolute));
        const QString when =
            QDateTime::currentDateTime().toString(QStringLiteral("yyyy-MM-ddThh:mm:ss"));
        record.write("[Trash Info]\nPath=");
        record.write(encoded.toUtf8());
        record.write("\nDeletionDate=");
        record.write(when.toUtf8());
        record.write("\n");
        record.close();
        ++trashed;
    }

    refresh();
    return trashed;
}