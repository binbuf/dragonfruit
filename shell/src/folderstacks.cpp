// SPDX-License-Identifier: MIT
#include "folderstacks.h"

#include "files_core_list.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFileSystemWatcher>
#include <QSet>
#include <QUrl>
#include <QVariantMap>

namespace {

// Turn one files-core listing event into the stack's row maps.
QVariantList rowsFromEvent(const df_files_event *event)
{
    QVariantList rows;
    if (!event || !event->nodes)
        return rows;
    rows.reserve(static_cast<int>(event->count));
    for (uint32_t i = 0; i < event->count; ++i) {
        const df_files_node &node = event->nodes[i];
        const QString uri = node.uri ? QString::fromUtf8(node.uri) : QString();
        const QString local = QUrl(uri).toLocalFile();
        QVariantMap row;
        row.insert(QStringLiteral("name"), node.name ? QString::fromUtf8(node.name) : QString());
        row.insert(QStringLiteral("path"), local.isEmpty() ? uri : local);
        row.insert(QStringLiteral("isDir"), node.kind == DF_NODE_DIRECTORY);
        rows.append(row);
    }
    return rows;
}

// List `path` through the one files-core implementation, blocking until the
// ordered snapshot is done (the same poll the chooser bridge uses).
QVariantList listFolder(const QString &path)
{
    const QByteArray encoded = QUrl::fromLocalFile(path).toString().toUtf8();
    void *session = df_files_begin(encoded.constData());
    if (!session)
        return {};
    QVariantList rows;
    for (int guard = 0; guard < 64; ++guard) {
        df_files_event *event = df_files_poll(session, 500);
        if (!event)
            break;
        const int status = event->status;
        if (status == DF_FILES_STATUS_BATCH) {
            rows = rowsFromEvent(event);
            df_files_event_free(event);
            continue;
        }
        df_files_event_free(event);
        if (status == DF_FILES_STATUS_ERROR || status == DF_FILES_STATUS_DONE)
            break;
        // TIMEOUT: keep waiting for the worker.
    }
    df_files_free(session);
    return rows;
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

int moveFilesIntoFolder(const QString &target, const QStringList &sources, QString *error)
{
    const QString root = QDir(target).absolutePath();
    if (root.isEmpty() || root == QLatin1String("/")) {
        if (error)
            *error = QStringLiteral("refusing to move into an unsafe root");
        return 0;
    }
    QDir().mkpath(root);
    if (error)
        error->clear();

    int moved = 0;
    for (const QString &path : sources) {
        if (path.isEmpty())
            continue;
        const QFileInfo info(path);
        const QString absolute = info.absoluteFilePath();
        if (absolute == root || absolute.startsWith(root + QLatin1Char('/'))) {
            if (error)
                *error = QStringLiteral("refusing to move %1 into itself").arg(absolute);
            continue;
        }
        if (!info.exists() && !info.isSymLink()) {
            if (error)
                *error = QStringLiteral("no such file: %1").arg(absolute);
            continue;
        }
        const QString name = uniqueName(root, info.fileName());
        if (!movePath(absolute, root + QLatin1Char('/') + name)) {
            if (error)
                *error = QStringLiteral("cannot move %1 into %2").arg(absolute, root);
            continue;
        }
        ++moved;
    }
    return moved;
}

QString folderDisplayName(const QString &path)
{
    const QString base = QFileInfo(QDir::cleanPath(path)).fileName();
    return base.isEmpty() ? QStringLiteral("Downloads") : base;
}

FolderStacks::FolderStacks(QObject *parent)
    : QObject(parent)
{
    m_watcher = new QFileSystemWatcher(this);
    connect(m_watcher, &QFileSystemWatcher::directoryChanged, this,
            &FolderStacks::onDirectoryChanged);
}

FolderStacks::~FolderStacks() = default;

QStringList FolderStacks::folders() const
{
    QStringList out;
    out.reserve(m_entries.size());
    for (const Entry &entry : m_entries)
        out.append(entry.path);
    return out;
}

bool FolderStacks::contains(const QString &path) const
{
    const QString clean = QDir::cleanPath(path);
    for (const Entry &entry : m_entries) {
        if (entry.path == clean)
            return true;
    }
    return false;
}

QVariantList FolderStacks::stacks() const
{
    QVariantList out;
    out.reserve(m_entries.size());
    for (const Entry &entry : m_entries) {
        QVariantMap map;
        map.insert(QStringLiteral("path"), entry.path);
        map.insert(QStringLiteral("name"), entry.name);
        map.insert(QStringLiteral("items"), entry.items);
        map.insert(QStringLiteral("count"), entry.count);
        map.insert(QStringLiteral("badge"), entry.badge);
        map.insert(QStringLiteral("missing"), entry.missing);
        out.append(map);
    }
    return out;
}

void FolderStacks::setFolders(const QStringList &paths)
{
    bool changed = false;
    QList<Entry> next;
    QSet<QString> seen;
    for (const QString &raw : paths) {
        if (raw.isEmpty())
            continue;
        const QString clean = QDir::cleanPath(raw);
        if (seen.contains(clean))
            continue;
        seen.insert(clean);

        // Preserve an existing entry's badge/baseline when the path is kept.
        bool reused = false;
        for (const Entry &entry : m_entries) {
            if (entry.path == clean) {
                next.append(entry);
                next.last().missing = !QFileInfo(clean).isDir();
                reused = true;
                break;
            }
        }
        if (reused)
            continue;

        Entry entry;
        entry.path = clean;
        entry.name = folderDisplayName(clean);
        next.append(entry);
        changed = true;
    }
    if (next.size() != m_entries.size())
        changed = true;

    m_entries = next;
    for (Entry &entry : m_entries)
        relist(entry, false);
    syncWatches();
    if (changed)
        emit this->changed();
}

void FolderStacks::markSeen(const QString &path)
{
    const QString clean = QDir::cleanPath(path);
    for (Entry &entry : m_entries) {
        if (entry.path != clean || entry.badge == 0)
            continue;
        entry.badge = 0;
        emit changed();
        return;
    }
}

void FolderStacks::refresh()
{
    for (Entry &entry : m_entries)
        relist(entry, true);
    syncWatches();
}

void FolderStacks::onDirectoryChanged(const QString &path)
{
    const QString clean = QDir::cleanPath(path);
    for (Entry &entry : m_entries) {
        if (entry.path == clean)
            relist(entry, true);
    }
    syncWatches();
}

void FolderStacks::relist(Entry &entry, bool emitSignal)
{
    const QFileInfo info(entry.path);
    if (!info.isDir()) {
        const bool changed = !entry.missing || !entry.items.isEmpty();
        entry.missing = true;
        entry.items.clear();
        entry.count = 0;
        if (changed && emitSignal)
            emit this->changed();
        return;
    }

    const QVariantList items = listFolder(entry.path);
    QSet<QString> names;
    for (const QVariant &value : items) {
        const QVariantMap row = value.toMap();
        names.insert(row.value(QStringLiteral("name")).toString());
    }

    int added = 0;
    if (entry.seenOnce) {
        for (const QString &name : names) {
            if (!entry.previous.contains(name))
                ++added;
        }
    }
    entry.previous = names;
    entry.seenOnce = true;

    const bool listingChanged = items != entry.items || entry.missing;
    entry.missing = false;
    entry.items = items;
    entry.count = items.size();
    if (added > 0)
        entry.badge += added;
    if (emitSignal && (listingChanged || added > 0))
        emit changed();
}

void FolderStacks::syncWatches()
{
    const QStringList current = m_watcher->directories();
    QStringList wanted;
    for (const Entry &entry : m_entries) {
        if (!entry.missing && QFileInfo(entry.path).isDir())
            wanted.append(entry.path);
    }
    for (const QString &path : current) {
        if (!wanted.contains(path))
            m_watcher->removePath(path);
    }
    for (const QString &path : wanted) {
        if (!current.contains(path))
            m_watcher->addPath(path);
    }
}