// SPDX-License-Identifier: MIT
// Interim home-trash state for the Dock's Trash entry (T-10, section 16).
//
// The design names GVfs `trash://` through a `GFileMonitor` as the single
// source of truth, with GIO `g_file_trash()` / the GVfs empty operation for
// drops and Empty Trash. This host has the libgio runtime but no GIO dev
// headers (`pkg-config gio-2.0` is absent), so this is the sanctioned
// filesystem fallback: it watches `$XDG_DATA_HOME/Trash/{info,files}` directly
// and only ever touches the home trash. DELETE THIS CLASS and switch to the
// GIO backend once the headers are available; the public behavior (a full
// flag, a count, and an empty operation) does not change.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>

class QFileSystemWatcher;

class TrashMonitor : public QObject
{
    Q_OBJECT

public:
    explicit TrashMonitor(const QString &root = defaultRoot(), QObject *parent = nullptr);

    // `$XDG_DATA_HOME/Trash` (default `~/.local/share/Trash`).
    static QString defaultRoot();

    QString root() const { return m_root; }
    bool isFull() const { return m_count > 0; }
    int itemCount() const { return m_count; }
    QString lastError() const { return m_error; }

    // Start/stop watching `info` and `files`. Idle contributes zero wakeups;
    // a filesystem event triggers a re-scan and emits changed().
    void start();
    void stop();
    bool isWatching() const { return m_watching; }

    // Re-scan now; emits changed() when the full/count state differs.
    void refresh();

    // Remove every home-trash item and re-scan. Returns the number of entries
    // removed, or -1 on error (lastError()). This is the fallback for the GVfs
    // empty operation; it never follows symlinks and never leaves the trash
    // root.
    int empty();

    // Move `paths` into the home trash (freedesktop Trash spec) and re-scan.
    // Returns the number of items trashed; per-item failures are skipped and
    // recorded in `lastError()`. Refuses the trash root and `/`. This is the
    // fallback for GIO `g_file_trash()`; it never leaves the trash root and
    // never follows symlinks out of it.
    int trash(const QStringList &paths);

signals:
    void changed();

private slots:
    void onDirectoryChanged(const QString &path);

private:
    void rescan(bool emitSignal);
    void syncWatches();

    QString m_root;
    QString m_error;
    int m_count = 0;
    bool m_watching = false;
    QFileSystemWatcher *m_watcher = nullptr;
};