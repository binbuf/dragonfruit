// SPDX-License-Identifier: MIT
// The shell's one asynchronous filesystem owner (T-14.7k).
//
// Directory listings and folder moves run on bounded worker pools here and
// post their results back to the caller's thread, so the shell thread that
// dispatches Dock and menu input never blocks on a slow folder, a network
// mount, or a cross-filesystem copy. `FolderStacks` uses it for stack
// listings; the controller uses it for a drop onto a folder stack.
//
// Ownership is explicit: listings are read-only and may run concurrently
// (bounded by `kMaxListingWorkers`); mutations are serialized on one worker
// so two moves can never race the same destination. Every mutation carries a
// monotonic operation id and a per-item outcome, and a bounded history keeps
// completed results so a repeated completion updates the UI once and a retry
// can inspect what failed.
#pragma once

#include <QHash>
#include <QList>
#include <QMetaType>
#include <QObject>
#include <QString>
#include <QStringList>
#include <QVariantList>

class QThreadPool;

// One item's outcome from a move operation.
struct FileOperationItem
{
    QString path;
    bool ok = false;
    QString error;
};

// Move policy and deterministic test seams. Production callers leave both at
// their defaults: a same-filesystem move is one atomic rename and the
// cross-filesystem fallback is selected only when `rename(2)` reports EXDEV.
struct FileMoveOptions
{
    // Exercise the copy-to-temp path even when both paths are on one
    // filesystem. Tests use this instead of depending on a second mount.
    bool forceCrossFilesystem = false;
    // Stop after this many copied entries and fail the move. Negative means
    // no limit. The partial temp is removed and the source is left intact.
    int failAfterEntries = -1;
};

// Move each of `sources` into `target` on the calling thread, de-duplicating
// names and using copy-to-temp-sibling plus rename across filesystems.
// Returns the number moved; `error` receives the last failure. Refuses
// `target` itself, `/`, and a directory's own subtree. This is the
// synchronous core the asynchronous owner runs on a worker; the unit tests
// exercise it directly.
int moveFilesIntoFolder(const QString &target, const QStringList &sources,
                        QString *error = nullptr,
                        const FileMoveOptions &options = FileMoveOptions());

// The per-item form used by the asynchronous owner: one outcome per
// non-empty source, in input order. `error` receives the last failure.
QList<FileOperationItem> moveFilesIntoFolderDetailed(const QString &target,
                                                    const QStringList &sources,
                                                    QString *error = nullptr,
                                                    const FileMoveOptions &options
                                                        = FileMoveOptions());

class FileOperations : public QObject
{
    Q_OBJECT

public:
    enum class Kind {
        Move,
    };
    Q_ENUM(Kind)

    struct Result
    {
        quint64 id = 0;
        Kind kind = Kind::Move;
        QString target;
        bool ok = false;
        QString error;
        QList<FileOperationItem> items;
        int succeeded = 0;
        int failed = 0;
    };

    explicit FileOperations(QObject *parent = nullptr);
    ~FileOperations() override;

    // List `path` through files-core on a bounded worker and post the ordered
    // snapshot back to this object's thread. `requestId` is echoed with the
    // result so a caller can discard a result for a folder that is no longer
    // current. Returns false for an empty path.
    bool listFolder(const QString &path, quint64 requestId);

    // Move each of `sources` into `target` on the mutation worker. Returns the
    // operation id (> 0), or 0 when there is nothing to move or the target is
    // empty. The result arrives through `operationFinished` exactly once.
    // `options` carries the deterministic test seams; production callers use
    // the defaults.
    quint64 moveIntoFolder(const QString &target, const QStringList &sources,
                           const FileMoveOptions &options = FileMoveOptions());

    // In-flight counts, for tests and diagnostics.
    int pendingListings() const { return m_listingPending; }
    int pendingOperations() const { return m_operationPending; }

    // The retained result for a completed operation, or a default Result when
    // the id is unknown or has aged out of the bounded history.
    Result completedOperation(quint64 id) const;
    bool hasCompletedOperation(quint64 id) const;

signals:
    // The worker listed `path`; `rows` is `{ name, path, isDir }` order.
    void folderListed(const QString &path, quint64 requestId,
                      const QVariantList &rows);
    // The listing failed (missing folder, permission, timeout). The caller
    // decides whether to keep its previous snapshot or degrade the entry.
    void folderListFailed(const QString &path, quint64 requestId,
                          const QString &error);
    // One mutation finished on this object's thread, exactly once per id.
    void operationFinished(const FileOperations::Result &result);

private:
    void finishOperation(const Result &result);

    QThreadPool *m_listingPool = nullptr;
    QThreadPool *m_mutationPool = nullptr;
    quint64 m_nextOperationId = 1;
    int m_listingPending = 0;
    int m_operationPending = 0;
    // Bounded, oldest-first history so a repeated completion can be detected
    // and a retry can inspect the outcome it is replacing.
    QHash<quint64, Result> m_completed;
    QList<quint64> m_completedOrder;
};

Q_DECLARE_METATYPE(FileOperations::Result)
