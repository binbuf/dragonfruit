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
    // The Empty Trash operation's lifecycle (T-14.7r): one operation at a time.
    // `Idle` before any request, `Emptying` while the worker runs, then a
    // terminal `Succeeded`/`Failed`. The Dock renders one of these; the result
    // is a one-shot `emptyFinished`, never persistent state.
    enum class EmptyState {
        Idle,
        Emptying,
        Succeeded,
        Failed,
    };
    Q_ENUM(EmptyState)

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
    // -1 on error (lastError()). Synchronous; kept for the drop path and the
    // unit tests. The Dock's Empty Trash uses `emptyAsync` instead so the UI
    // thread never blocks (T-14.7r).
    int empty();

    // Start an asynchronous Empty Trash on a one-shot worker thread and return
    // immediately. Returns true when the operation started; false when an
    // empty is already in flight (the second request is ignored) or the
    // backend is unreachable. `emptyFinished` is emitted on the UI thread when
    // the worker reports, and `changed` re-reads the store (the monitor's
    // event-driven watch also reports it). Never blocks the caller.
    bool emptyAsync();

    // The current Empty Trash lifecycle and its last result. `emptyRemoved` is
    // valid after a `Succeeded`; `emptyError` after a `Failed`.
    EmptyState emptyState() const { return m_emptyState; }
    bool isEmptying() const { return m_emptyState == EmptyState::Emptying; }
    int emptyRemoved() const { return m_emptyRemoved; }
    QString emptyError() const { return m_emptyError; }

    // Return the operation state to Idle and clear the last result. The Dock
    // calls this when it dismisses the progress/result popover.
    void resetEmptyState();

    // Move `paths` into the home trash and re-read. Returns the number moved;
    // per-item failures are recorded in lastError().
    int trash(const QStringList &paths);

signals:
    void changed();
    // The Empty Trash worker started / finished. `ok` is true on success, with
    // `removed` the item count; on failure `removed` is -1 and `error` carries
    // the message. One-shot per operation.
    void emptyStarted();
    void emptyFinished(bool ok, int removed, const QString &error);

private:
    void applyState(bool force = false);
    void takeError();
    void finishEmpty(int removed);
    void joinEmptyWorker();

    void *m_monitor = nullptr;
    std::thread m_worker;
    std::atomic<bool> m_stop{false};
    bool m_watching = false;
    int m_count = 0;
    bool m_available = true;
    QString m_error;

    EmptyState m_emptyState = EmptyState::Idle;
    int m_emptyRemoved = 0;
    QString m_emptyError;
    std::thread m_emptyWorker;
};