// SPDX-License-Identifier: MIT
// The Dock's Trash state, backed by files-core's freedesktop Trash store
// (T-10.6a). The design names GVfs `trash://` as the one source of truth shared
// with Files; GIO/GVfs is not linked on this host, so the shell reads the same
// `FreedesktopTrash` store through the files-core C ABI. The interim
// `QFileSystemWatcher`-based `TrashMonitor` is deleted in the same change.
//
// A worker thread blocks on `df_files_trash_monitor_wait` and posts changes
// back to the UI thread; an event-driven watch means an idle Trash does no
// polling and no directory scans.
#pragma once

#include <QObject>
#include <QString>
#include <QStringList>

#include <atomic>
#include <thread>

class TrashBridge : public QObject
{
    Q_OBJECT

public:
    explicit TrashBridge(QObject *parent = nullptr);
    ~TrashBridge() override;

    bool isFull() const { return m_count > 0; }
    int itemCount() const { return m_count; }
    bool isAvailable() const { return m_available; }
    QString lastError() const { return m_error; }

    // Start the monitor and its watch worker. Idempotent.
    void start();
    // Stop the watch worker. Does not free until destruction.
    void stop();
    bool isWatching() const { return m_watching; }

    // Re-read now; emits changed() when the reading differs.
    void refresh();

    // Remove every home-trash item and re-read. Returns the number removed, or
    // -1 on error (lastError()).
    int empty();

    // Move `paths` into the home trash and re-read. Returns the number moved;
    // per-item failures are recorded in lastError().
    int trash(const QStringList &paths);

signals:
    void changed();

private:
    void applyState(bool force = false);
    void takeError();

    void *m_monitor = nullptr;
    std::thread m_worker;
    std::atomic<bool> m_stop{false};
    bool m_watching = false;
    int m_count = 0;
    bool m_available = true;
    QString m_error;
};