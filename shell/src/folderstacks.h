// SPDX-License-Identifier: GPL-3.0-only
// Folder-stack state for the Dock (T-14.7k).
//
// The Dock renders any pinned folder as a **stack**: a folder silhouette that
// opens a popover listing the folder's contents and opens the folder in Files
// (ADR 0092). `FolderStacks` is the one model behind every folder stack: it
// holds the ordered set of absolute paths, lists each through the one
// `dragonfruit-files-core` implementation (the same C ABI the file chooser and
// Files use), watches each folder for changes, and tracks a per-folder
// new-items badge cleared when the stack is opened. The shell never opens or
// lists a directory itself.
//
// Listings run through `FileOperations` on a worker, so a slow folder never
// blocks Dock or menu input. A watcher burst is coalesced to one follow-up
// listing per folder, the last listing stays visible while a refresh runs, and
// a result is applied only to the request that is still current for that
// entry.
//
// A path that disappears degrades to a `missing` entry (empty listing) rather
// than a crash; the Dock dims it and names it in the hover label.
#pragma once

#include <QList>
#include <QObject>
#include <QSet>
#include <QString>
#include <QStringList>
#include <QVariantList>

class QFileSystemWatcher;
class FileOperations;

// A folder's display name (its basename). Root/empty falls back to
// "Downloads" so a stack always has a name.
QString folderDisplayName(const QString &path);

class FolderStacks : public QObject
{
    Q_OBJECT

public:
    // Uses a private `FileOperations` child. Tests and callers that do not
    // share the shell's owner use this form.
    explicit FolderStacks(QObject *parent = nullptr);
    // Shares the shell's one filesystem owner. `operations` must outlive this
    // object.
    FolderStacks(FileOperations *operations, QObject *parent);
    ~FolderStacks() override;

    // Replace the ordered folder set with `paths` (absolute, de-duplicated,
    // cleaned). Missing paths stay in the set as `missing` entries. Emits
    // `changed()` immediately when the set differs, then again as each
    // asynchronous listing lands.
    void setFolders(const QStringList &paths);
    QStringList folders() const;

    // The ordered stack maps the Dock consumes: each is
    // `{ path, name, items, count, badge, missing }` where `items` is
    // `{ name, path, isDir }` newest-listed order.
    QVariantList stacks() const;

    bool contains(const QString &path) const;

    // Clear one folder's new-items badge (its stack popover opened).
    void markSeen(const QString &path);

    // Re-list every folder now; emits `changed()` on any difference.
    void refresh();

signals:
    void changed();

private slots:
    void onDirectoryChanged(const QString &path);
    void onFolderListed(const QString &path, quint64 requestId,
                        const QVariantList &rows);
    void onFolderListFailed(const QString &path, quint64 requestId,
                            const QString &error);

private:
    struct Entry {
        QString path;
        QString name;
        QVariantList items;
        int count = 0;
        int badge = 0;
        bool missing = false;
        bool seenOnce = false;
        QSet<QString> previous;
        // The current in-flight listing. A result whose id no longer matches
        // belongs to a removed or superseded entry and is discarded.
        quint64 requestId = 0;
        bool listing = false;
        // A watcher event (or explicit refresh) arrived while `listing` was
        // true. One follow-up listing runs when the current one finishes,
        // coalescing a burst into at most two reads.
        bool refreshQueued = false;
    };

    void init(FileOperations *operations);
    Entry *entryForRequest(quint64 requestId, const QString &path);
    // Apply one listing to `entry`; returns true when the model changed.
    bool applyListing(Entry &entry, const QVariantList &items);
    // Submit a listing unless one is already in flight; coalesce otherwise.
    void relist(Entry &entry);
    void relistPath(const QString &path);
    void syncWatches();

    QList<Entry> m_entries;
    QFileSystemWatcher *m_watcher = nullptr;
    FileOperations *m_operations = nullptr;
    quint64 m_nextRequestId = 1;
};
