// SPDX-License-Identifier: GPL-3.0-or-later
#include "folderstacks.h"

#include "fileoperations.h"

#include <QDir>
#include <QFileInfo>
#include <QFileSystemWatcher>
#include <QVariantMap>

FolderStacks::FolderStacks(QObject *parent)
    : QObject(parent)
{
    init(new FileOperations(this));
}

FolderStacks::FolderStacks(FileOperations *operations, QObject *parent)
    : QObject(parent)
{
    init(operations ? operations : new FileOperations(this));
}

FolderStacks::~FolderStacks() = default;

void FolderStacks::init(FileOperations *operations)
{
    m_operations = operations;
    m_watcher = new QFileSystemWatcher(this);
    connect(m_watcher, &QFileSystemWatcher::directoryChanged, this,
            &FolderStacks::onDirectoryChanged);
    connect(m_operations, &FileOperations::folderListed, this,
            &FolderStacks::onFolderListed);
    connect(m_operations, &FileOperations::folderListFailed, this,
            &FolderStacks::onFolderListFailed);
}

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
    QStringList previousPaths;
    previousPaths.reserve(m_entries.size());
    for (const Entry &entry : m_entries)
        previousPaths.append(entry.path);

    QList<Entry> next;
    QSet<QString> seen;
    for (const QString &raw : paths) {
        if (raw.isEmpty())
            continue;
        const QString clean = QDir::cleanPath(raw);
        if (seen.contains(clean))
            continue;
        seen.insert(clean);

        // Preserve an existing entry's badge, baseline, last listing, and any
        // in-flight request when the path is kept.
        bool reused = false;
        for (const Entry &entry : m_entries) {
            if (entry.path == clean) {
                next.append(entry);
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
    }

    m_entries = next;
    for (Entry &entry : m_entries)
        relist(entry);
    syncWatches();

    QStringList nextPaths;
    nextPaths.reserve(m_entries.size());
    for (const Entry &entry : m_entries)
        nextPaths.append(entry.path);
    if (nextPaths != previousPaths)
        emit changed();
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
        relist(entry);
    syncWatches();
}

void FolderStacks::onDirectoryChanged(const QString &path)
{
    relistPath(path);
    syncWatches();
}

void FolderStacks::relistPath(const QString &path)
{
    const QString clean = QDir::cleanPath(path);
    for (Entry &entry : m_entries) {
        if (entry.path == clean)
            relist(entry);
    }
}

void FolderStacks::relist(Entry &entry)
{
    if (entry.listing) {
        // Coalesce a watcher burst into one follow-up read.
        entry.refreshQueued = true;
        return;
    }
    const quint64 requestId = m_nextRequestId++;
    entry.requestId = requestId;
    entry.listing = true;
    entry.refreshQueued = false;
    if (!m_operations->listFolder(entry.path, requestId)) {
        entry.listing = false;
        entry.requestId = 0;
    }
}

FolderStacks::Entry *FolderStacks::entryForRequest(quint64 requestId,
                                                   const QString &path)
{
    if (requestId == 0)
        return nullptr;
    for (Entry &entry : m_entries) {
        if (entry.requestId == requestId && entry.path == path)
            return &entry;
    }
    // A result for a folder that was removed from the set (or superseded by a
    // newer request) must never be applied to another entry.
    return nullptr;
}

bool FolderStacks::applyListing(Entry &entry, const QVariantList &items)
{
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
    return listingChanged || added > 0;
}

void FolderStacks::onFolderListed(const QString &path, quint64 requestId,
                                  const QVariantList &rows)
{
    Entry *entry = entryForRequest(requestId, path);
    if (!entry)
        return;
    entry->listing = false;
    entry->requestId = 0;
    const bool listingChanged = applyListing(*entry, rows);
    const bool followUp = entry->refreshQueued;
    entry->refreshQueued = false;
    if (followUp)
        relist(*entry);
    syncWatches();
    if (listingChanged)
        emit changed();
}

void FolderStacks::onFolderListFailed(const QString &path, quint64 requestId,
                                      const QString &error)
{
    Entry *entry = entryForRequest(requestId, path);
    if (!entry)
        return;
    entry->listing = false;
    entry->requestId = 0;
    const bool wasMissing = entry->missing && entry->items.isEmpty();
    entry->missing = true;
    entry->items.clear();
    entry->count = 0;
    const bool followUp = entry->refreshQueued;
    entry->refreshQueued = false;
    if (followUp)
        relist(*entry);
    syncWatches();
    qWarning() << "shell: cannot list folder" << path << ":" << error;
    if (!wasMissing)
        emit changed();
}

void FolderStacks::syncWatches()
{
    const QStringList current = m_watcher->directories();
    QStringList wanted;
    // Watch every entry, including one whose last read failed: a present but
    // transiently unreadable folder recovers on its next change, and
    // `addPath` on a vanished path is a harmless no-op that a later refresh
    // retries.
    wanted.reserve(m_entries.size());
    for (const Entry &entry : m_entries)
        wanted.append(entry.path);
    for (const QString &path : current) {
        if (!wanted.contains(path))
            m_watcher->removePath(path);
    }
    for (const QString &path : wanted) {
        if (!current.contains(path))
            m_watcher->addPath(path);
    }
}

QString folderDisplayName(const QString &path)
{
    const QString base = QFileInfo(QDir::cleanPath(path)).fileName();
    return base.isEmpty() ? QStringLiteral("Downloads") : base;
}
