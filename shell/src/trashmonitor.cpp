// SPDX-License-Identifier: MIT
#include "trashmonitor.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFileSystemWatcher>
#include <QSet>
#include <QStandardPaths>

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
    if (count == m_count)
        return;
    m_count = count;
    if (emitSignal)
        emit changed();
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