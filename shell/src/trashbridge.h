// SPDX-License-Identifier: GPL-3.0-only
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

#include <QHash>
#include <QList>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>

#include <atomic>
#include <mutex>
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
    // immediately. Returns true when the request was accepted (an unreachable
    // backend reports its failure immediately); false when an empty is already
    // in flight (the second request is ignored). `requestId` identifies this
    // request and is echoed with the result so a late result from a superseded
    // operation can be ignored; zero asks the bridge to allocate one.
    // `emptyFinished` is emitted on the UI thread when the worker reports, and
    // `changed` re-reads the store (the monitor's event-driven watch also
    // reports it). Never blocks the caller.
    bool emptyAsync(quint64 requestId = 0);

    // The current Empty Trash lifecycle and its last result. `emptyRemoved` is
    // valid after a `Succeeded`; `emptyError` after a `Failed`.
    EmptyState emptyState() const { return m_emptyState; }
    bool isEmptying() const { return m_emptyState == EmptyState::Emptying; }
    int emptyRemoved() const { return m_emptyRemoved; }
    QString emptyError() const { return m_emptyError; }
    quint64 emptyRequestId() const { return m_emptyRequestId; }

    // Return the operation state to Idle and clear the last result. The Dock
    // no longer calls this when it closes the status card: the operation and
    // its result outlive the card (reliability §7). It remains the explicit
    // reset seam.
    void resetEmptyState();

    // Move `paths` into the home trash and re-read. Returns the number moved;
    // per-item failures are recorded in lastError(). Synchronous; kept for the
    // unit tests. The Dock's drop path uses `trashAsync` so the shell thread
    // never blocks.
    int trash(const QStringList &paths);

    // Start an asynchronous Trash request and return immediately. Requests are
    // serialized behind one mutation worker (and behind Empty Trash), so a
    // second drop queues rather than racing the first. Returns false when
    // `paths` has no non-empty entry. `requestId` identifies the request and is
    // echoed with the result; zero asks the bridge to allocate one. An
    // unreachable backend reports its failure immediately instead of hanging.
    bool trashAsync(const QStringList &paths, quint64 requestId = 0);

    bool isTrashing() const { return m_trashRunning; }
    quint64 trashRequestId() const { return m_trashRequestId; }

    // The per-item outcomes of a completed request (`{ path, ok, error }`),
    // retained in a bounded history so a retry can inspect what failed. An
    // unknown id returns an empty list.
    QVariantList trashOutcomes(quint64 requestId) const;
    quint64 lastTrashRequestId() const { return m_lastTrashRequestId; }

signals:
    void changed();
    // The Empty Trash worker started / finished. `ok` is true on success, with
    // `removed` the item count; on failure `removed` is -1 and `error` carries
    // the message. `requestId` matches the request passed to `emptyAsync`, so
    // a late result from a superseded operation can be ignored. One-shot per
    // operation.
    void emptyStarted();
    void emptyFinished(bool ok, int removed, const QString &error,
                       quint64 requestId);
    // The async Trash request started / finished. `outcomes` carries one
    // `{ path, ok, error }` map per requested item in input order, so a
    // partial failure names exactly the affected items. Emitted once per
    // request id.
    void trashStarted(quint64 requestId);
    void trashFinished(bool ok, int trashed, int failed, const QString &error,
                       const QVariantList &outcomes, quint64 requestId);

private:
    struct PendingTrash {
        quint64 requestId = 0;
        QStringList paths;
    };

    void applyState(bool force = false);
    void takeError();
    QString takeErrorText();
    void finishEmpty(int removed, quint64 requestId);
    void joinEmptyWorker();
    void pumpTrash();
    void finishTrash(quint64 requestId, const QVariantList &outcomes,
                     int trashed, int failed, const QString &error);
    void recordTrashOutcomes(quint64 requestId, const QVariantList &outcomes);
    void trimTrashHistory();
    void joinTrashWorker();

    void *m_monitor = nullptr;
    std::thread m_worker;
    std::atomic<bool> m_stop{false};
    bool m_watching = false;
    // Trash and Empty Trash both mutate the one store; one mutex serializes
    // the backend calls while the event-driven watch stays a read-only
    // observer. Workers never touch UI-thread members directly.
    std::mutex m_mutationMutex;
    int m_count = 0;
    bool m_available = true;
    QString m_error;

    EmptyState m_emptyState = EmptyState::Idle;
    int m_emptyRemoved = 0;
    QString m_emptyError;
    quint64 m_emptyRequestId = 0;
    quint64 m_nextEmptyRequestId = 1;
    std::thread m_emptyWorker;

    // The async Trash queue: one request runs, later ones wait. The running
    // request's id is the only result the bridge will accept.
    QList<PendingTrash> m_trashQueue;
    std::thread m_trashWorker;
    bool m_trashRunning = false;
    quint64 m_trashRequestId = 0;
    quint64 m_nextTrashRequestId = 1;
    quint64 m_lastTrashRequestId = 0;
    // Bounded history of completed outcomes, oldest first.
    QHash<quint64, QVariantList> m_trashHistory;
    QList<quint64> m_trashHistoryOrder;
};