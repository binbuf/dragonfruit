// SPDX-License-Identifier: MIT
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

// Move each of `sources` into the `target` folder, de-duplicating names and
// falling back to copy+remove across filesystems. Returns the number moved;
// `error` receives the last failure. Refuses `target` itself and `/`.
int moveFilesIntoFolder(const QString &target, const QStringList &sources,
                        QString *error = nullptr);

// A folder's display name (its basename). Root/empty falls back to
// "Downloads" so a stack always has a name.
QString folderDisplayName(const QString &path);

class FolderStacks : public QObject
{
    Q_OBJECT

public:
    explicit FolderStacks(QObject *parent = nullptr);
    ~FolderStacks() override;

    // Replace the ordered folder set with `paths` (absolute, de-duplicated,
    // cleaned). Missing paths stay in the set as `missing` entries. Emits
    // `changed()` when the set or any listing differs.
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
    };

    // Re-list one entry from files-core and update its badge.
    void relist(Entry &entry, bool emitSignal);
    void syncWatches();

    QList<Entry> m_entries;
    QFileSystemWatcher *m_watcher = nullptr;
};