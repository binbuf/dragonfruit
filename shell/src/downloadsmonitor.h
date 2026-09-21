// SPDX-License-Identifier: MIT
// Downloads-stack state for the Dock (T-10 section 17).
//
// The design names Files-core (T-17) folder monitoring as the eventual source
// for the Downloads stack. Until Files exists this is the shell's own
// zero-polling watch of `$XDG_DOWNLOAD_DIR` (default `~/Downloads`): a
// `QFileSystemWatcher` on the directory, a newest-first listing for the stack
// popover, and a "new items" badge cleared when the stack is opened. The
// public behavior (a listing, a count, a badge, and a move-in for drops) does
// not change when T-17 replaces the backend.
#pragma once

#include <QObject>
#include <QSet>
#include <QString>
#include <QVariantList>

class QFileSystemWatcher;

class DownloadsMonitor : public QObject
{
    Q_OBJECT

public:
    explicit DownloadsMonitor(const QString &directory = defaultDirectory(),
                              QObject *parent = nullptr);

    // `$XDG_DOWNLOAD_DIR` when set, else `~/Downloads` (shared with the drop
    // resolver, `dockdrops.cpp`).
    static QString defaultDirectory();

    QString directory() const { return m_directory; }

    // Newest first: `{ name, path, isDir }`. Directories sort after files at
    // equal timestamps so the common case (a fresh download) leads.
    QVariantList items() const { return m_items; }
    int itemCount() const { return m_items.size(); }

    // Items added since the stack was last opened (`markSeen()`); zero on the
    // first scan, so a pre-existing folder is not a badge on login.
    int newCount() const { return m_newCount; }

    // Move `paths` into the Downloads folder (the drop action, T-10 section
    // 17). Returns the number moved; per-item failures are skipped and
    // recorded in `lastError()`. Refuses the folder itself and `/`.
    int moveIn(const QStringList &paths);

    QString lastError() const { return m_error; }

    // Start/stop watching the directory. Idle contributes zero wakeups; a
    // filesystem event triggers a re-scan and emits changed().
    void start();
    void stop();
    bool isWatching() const { return m_watching; }

    // Re-scan now; emits changed() when the listing or badge differs.
    void refresh();

    // Clear the new-items badge (the stack popover opened) and emit changed().
    void markSeen();

signals:
    void changed();

private slots:
    void onDirectoryChanged(const QString &path);

private:
    void rescan(bool emitSignal);
    void syncWatches();

    QString m_directory;
    QString m_error;
    QVariantList m_items;
    int m_newCount = 0;
    bool m_watching = false;
    bool m_seenOnce = false;
    QSet<QString> m_previous;
    QFileSystemWatcher *m_watcher = nullptr;
};
