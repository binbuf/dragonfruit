// SPDX-License-Identifier: MIT
// The thin Qt model facade over `files-core` (T-10.4b/T-10.4c, ADR 0049/0050).
//
// `FilesDirectoryModel` is a `QAbstractListModel` that owns no filesystem
// logic. It starts a `files-core` listing over the C ABI (`ffi/files_core.h`)
// for its `location`, polls that session from a timer so no directory I/O ever
// runs on the UI thread, and repaints the model's ordered snapshot. Sorting,
// collation, stable node ids, and raw-byte names all stay in Rust; the QML
// views and the C++ roles are the only thing here.
//
// Since T-10.4c the session also carries optimistic operations: `rename`,
// `trash`, and `newFolder` edit the in-memory model and repaint before they
// return, then a Rust worker's outcome confirms or reverts on a later poll
// (`pendingOps`, `lastError`). The session outlives the completed listing so
// sort and operations keep working; only polling stops.
//
// Since T332 a local folder is watched by files-core itself: an external
// create, delete, rename or move reaches the model as one coalesced delta per
// burst, and a settled window keeps a blocking poll armed so the change paints
// without a manual refresh. The poll has always run on a worker thread, so a
// burst never blocks the UI; the model only applies its delta on the UI
// thread.
//
// The C++ side is deliberately dump: every poll returns the latest ordered
// delta (a reset, an insertion set, an in-place change set, or a removal
// request, ADR 0051/0339), which the model mirrors and forwards.
#pragma once

#include <QAbstractListModel>
#include <QString>
#include <QStringList>
#include <QTimer>
#include <QtQml/qqmlregistration.h>

#include <vector>

class FilesDirectoryModel : public QAbstractListModel
{
    Q_OBJECT
    QML_ELEMENT

    // The `file://` (or other scheme) location to list. Setting it restarts
    // the listing; an empty or unsupported location clears the model.
    Q_PROPERTY(QString location READ location WRITE setLocation NOTIFY locationChanged)
    // The number of rows currently painted.
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    // `idle` | `streaming` | `complete` | `error`.
    Q_PROPERTY(QString state READ state NOTIFY stateChanged)
    // The failure message when `state` is `error`, else empty.
    Q_PROPERTY(QString errorMessage READ errorMessage NOTIFY stateChanged)
    // The active sort key (`name` | `kind` | `size` | `modified`).
    Q_PROPERTY(QString sortKey READ sortKey NOTIFY sortChanged)
    // Whether the active sort runs A→Z / smallest→largest.
    Q_PROPERTY(bool sortAscending READ sortAscending NOTIFY sortChanged)
    // Whether directories are grouped ahead of files (the Files toggle).
    Q_PROPERTY(bool foldersFirst READ foldersFirst NOTIFY sortChanged)
    // How many optimistic operations are still awaiting their real outcome.
    // The model keeps polling while this is non-zero, even after the listing
    // completes (T-10.4c).
    Q_PROPERTY(int pendingOps READ pendingOps NOTIFY pendingOpsChanged)
    // The most recent operation that snapped back, for an inline notice; empty
    // when there is none (T-10.4c).
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)
    // T330: per-item paste progress. `pasteTotal`/`pasteDone` describe the
    // current batch, `pasteActive` counts items still on the worker, and
    // `pasteProgressText` is the status-bar label ("Copying 1 of 3…" while
    // active, then the last summary). `pasteFinished` fires once when the
    // batch settles.
    Q_PROPERTY(int pasteTotal READ pasteTotal NOTIFY pasteProgressChanged)
    Q_PROPERTY(int pasteDone READ pasteDone NOTIFY pasteProgressChanged)
    Q_PROPERTY(int pasteActive READ pasteActive NOTIFY pasteProgressChanged)
    Q_PROPERTY(int pasteCopied READ pasteCopied NOTIFY pasteProgressChanged)
    Q_PROPERTY(int pasteSkipped READ pasteSkipped NOTIFY pasteProgressChanged)
    Q_PROPERTY(int pasteFailed READ pasteFailed NOTIFY pasteProgressChanged)
    Q_PROPERTY(QString pasteMode READ pasteMode NOTIFY pasteProgressChanged)
    Q_PROPERTY(QString pasteProgressText READ pasteProgressText NOTIFY pasteProgressChanged)
    // T331: the undo/redo stacks. `undoLabel`/`redoLabel` name the next step
    // ("Rename “a.txt”") or are empty; `canUndo`/`canRedo` drive the Edit menu
    // rows and the keyboard path. The stacks clear on a location change.
    Q_PROPERTY(int undoCount READ undoCount NOTIFY undoStateChanged)
    Q_PROPERTY(int redoCount READ redoCount NOTIFY undoStateChanged)
    Q_PROPERTY(QString undoLabel READ undoLabel NOTIFY undoStateChanged)
    Q_PROPERTY(QString redoLabel READ redoLabel NOTIFY undoStateChanged)
    Q_PROPERTY(bool canUndo READ canUndo NOTIFY undoStateChanged)
    Q_PROPERTY(bool canRedo READ canRedo NOTIFY undoStateChanged)
    // The last undo/redo sentence ("Undid “Rename a.txt”") or refusal reason;
    // empty before the first replay.
    Q_PROPERTY(QString undoMessage READ undoMessage NOTIFY undoStateChanged)
    // T332: whether files-core watches this folder for external changes, and
    // how many bursts arrived and were folded into the painted rows. A
    // watched folder repaints when another process creates, deletes,
    // renames, or moves an entry; the selection follows its node id.
    Q_PROPERTY(bool watching READ watching NOTIFY watchChanged)
    Q_PROPERTY(int watchUpdates READ watchUpdates NOTIFY watchChanged)

public:
    enum Roles {
        NodeIdRole = Qt::UserRole + 1,
        NameRole,
        UriRole,
        IsDirRole,
        KindRole,
        KindTextRole,
        IconRole,
        SizeRole,
        HasSizeRole,
        SizeTextRole,
        ModifiedRole,
        HasModifiedRole,
        ModifiedTextRole,
        SymlinkTargetRole,
        ThumbnailRole,
    };
    Q_ENUM(Roles)

    explicit FilesDirectoryModel(QObject *parent = nullptr);
    ~FilesDirectoryModel() override;

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role) const override;
    QHash<int, QByteArray> roleNames() const override;

    QString location() const { return m_location; }
    void setLocation(const QString &uri);

    int count() const { return int(m_nodes.size()); }
    QString state() const { return m_state; }
    QString errorMessage() const { return m_errorMessage; }
    QString sortKey() const { return m_sortKey; }
    bool sortAscending() const { return m_sortAscending; }
    bool foldersFirst() const { return m_foldersFirst; }
    int pendingOps() const { return m_pendingOps; }
    QString lastError() const { return m_lastError; }
    int pasteTotal() const { return m_pasteTotal; }
    int pasteDone() const { return m_pasteDone; }
    int pasteActive() const { return m_pasteActive; }
    int pasteCopied() const { return m_pasteCopied; }
    int pasteSkipped() const { return m_pasteSkipped; }
    int pasteFailed() const { return m_pasteFailed; }
    QString pasteMode() const { return m_pasteMode; }
    QString pasteProgressText() const;
    int undoCount() const { return m_undoCount; }
    int redoCount() const { return m_redoCount; }
    QString undoLabel() const { return m_undoLabel; }
    QString redoLabel() const { return m_redoLabel; }
    bool canUndo() const { return m_undoCount > 0; }
    bool canRedo() const { return m_redoCount > 0; }
    QString undoMessage() const { return m_undoMessage; }
    bool watching() const { return m_watching; }
    int watchUpdates() const { return m_watchUpdates; }

    // Sort by `key`, flipping direction when it is already active.
    Q_INVOKABLE void sortBy(const QString &key);
    // Force a fresh listing of the current location.
    Q_INVOKABLE void reload();
    // Optimistically rename `nodeId` to `newName`: the row repaints before
    // this returns, then confirms or snaps back when the worker finishes.
    Q_INVOKABLE bool rename(quint64 nodeId, const QString &newName);
    // Optimistically create the next `untitled folder` under `parentUri`
    // (the current location when empty).
    Q_INVOKABLE bool newFolder(const QString &parentUri = QString());
    // Optimistically move `nodeId` to Trash.
    Q_INVOKABLE bool trash(quint64 nodeId);
    // Optimistically clear the whole Trash (`trash://` listings only). The
    // rows disappear before this returns, then confirm or snap back.
    Q_INVOKABLE bool emptyTrash();
    // Paste one clipboard item (T330). `mode` is `copy` or `cut`; `policy` is
    // `replace`, `skip`, or `keep-both`. The row paints before this returns
    // for a paste into this folder; the worker confirms or reverts it.
    Q_INVOKABLE bool pasteItem(const QString &targetUri, const QString &sourceUri,
                               const QString &name, bool isDir,
                               const QString &mode, const QString &policy);
    // The names currently painted, for the shell's pre-flight conflict check
    // (the clipboard model lives in QML and survives navigation).
    Q_INVOKABLE QStringList names() const;
    // Undo / redo the last operation (T331). Returns false when there is
    // nothing to replay or a replay is still running; the outcome lands in
    // `undoMessage` (a refusal leaves the filesystem untouched and says why).
    Q_INVOKABLE bool undo();
    Q_INVOKABLE bool redo();
    // Put Back the item the shell trashed through another path (the Dock's
    // Trash bridge) and that this session recorded. Returns false with
    // `lastError` set when the id is unknown or the store refused.
    Q_INVOKABLE bool putBack(quint64 nodeId);
    // Record a successful trash the window's Trash bridge performed (T331):
    // the files-core session then offers it from `canUndo` and `putBack`.
    // `oldUri` is where the item was, `trashPath` its store path. Returns
    // false when the store could not confirm the entry.
    Q_INVOKABLE bool noteTrash(quint64 nodeId, const QString &oldUri,
                               const QString &trashPath);
    // The row currently painting `nodeId`, or -1. The shell uses it to
    // translate an anchor + a clicked id into a contiguous range.
    Q_INVOKABLE int rowForNodeId(quint64 nodeId) const;
    // The node id painting at `row`, or 0.
    Q_INVOKABLE quint64 nodeIdAt(int row) const;
    // The URI painting at `row`, or empty.
    Q_INVOKABLE QString uriAt(int row) const;
    // Whether the node painting at `row` is a directory.
    Q_INVOKABLE bool isDirAt(int row) const;
    // Every node id in the current order (Select All).
    Q_INVOKABLE QVariantList allNodeIds() const;
    // The inspector view of one row: name, kind, size, modified, symlink
    // target, thumbnail, parent folder and extension. Empty when the id is
    // not in the current listing (T230d).
    Q_INVOKABLE QVariantMap nodeInfo(quint64 nodeId) const;
    // The aggregate inspector view of a multi-selection: found count, folder
    // and file counts, and the summed file size (T230d).
    Q_INVOKABLE QVariantMap selectionInfo(const QVariantList &nodeIds) const;

signals:
    void locationChanged();
    void countChanged();
    void stateChanged();
    void sortChanged();
    void pendingOpsChanged();
    void lastErrorChanged();
    void pasteProgressChanged();
    // Emitted once when a paste batch settles; the shell clears a cut
    // clipboard and shows the summary.
    void pasteFinished(int copied, int skipped, int failed);
    // The undo/redo availability, labels, or last message changed (T331).
    void undoStateChanged();
    // The folder's watcher state or its folded-burst count changed (T332).
    void watchChanged();

private slots:
    void poll();

private:
    struct Node {
        quint64 id = 0;
        QString name;
        QString uri;
        int kind = 0;
        bool isDir = false;
        QString kindText;
        QString icon;
        quint64 size = 0;
        bool hasSize = false;
        QString sizeText;
        qint64 modifiedMs = 0;
        bool hasModified = false;
        QString modifiedText;
        QString symlinkTarget;
        // An `image://files-thumbnail/…` source for local image files, empty
        // for everything else (the views fall back to the kind glyph).
        QString thumbnail;
    };

    void start();
    void stop();
    // Apply one incremental delta: a reset rebuilds the whole model, an
    // insertion merges only the new rows, and a change/removal delta touches
    // only the named rows (T-10.5/T332). Returns whether rows were painted.
    bool applyDelta(const struct df_files_delta *delta);
    void applyReset(const struct df_files_delta *delta);
    void applyInsertions(const struct df_files_delta *delta);
    void applyChanges(const struct df_files_delta *delta);
    // Repaint `count` rows starting at `row` (a coalesced change set).
    void dataChangedRows(int row, int count);
    static Node nodeFromFfi(const struct df_files_node &source);
    void setState(const QString &state, const QString &error = QString());
    void applySort();
    // Repaint from the session's current snapshot without waiting.
    void repaint();
    // Keep the poll timer alive while an operation is pending.
    void ensurePolling();
    // Drain a snapped-back operation message into `lastError`.
    void takeError();
    void setPendingOps(int pending);
    // Fold the Rust session's paste counters into the properties and emit
    // `pasteFinished` once when a batch settles.
    void updatePasteProgress();
    // Fold the Rust session's undo stacks into the properties (T331).
    void updateUndoState();
    // Queue an undo/redo; `redo` picks the stack (T331).
    bool replay(bool redo);
    static QString iconFor(int kind, const QString &name);
    static QString thumbnailFor(int kind, const QString &uri, const QString &name);
    static QString kindTextFor(int kind);
    static QString formatSize(quint64 bytes);
    static QString formatModified(qint64 unixMillis);

    void *m_session = nullptr;
    QTimer m_timer;
    // The rows in the model's ordered projection (T-10.5). An insertion delta
    // merges the new nodes into this vector in one pass; `data()` reads it
    // directly, so the view's delegate virtualization keeps memory flat.
    std::vector<Node> m_nodes;
    QString m_location;
    QString m_state = QStringLiteral("idle");
    QString m_errorMessage;
    QString m_sortKey = QStringLiteral("name");
    bool m_sortAscending = true;
    bool m_foldersFirst = true;
    int m_pendingOps = 0;
    QString m_lastError;
    int m_pasteTotal = 0;
    int m_pasteDone = 0;
    int m_pasteActive = 0;
    int m_pasteCopied = 0;
    int m_pasteSkipped = 0;
    int m_pasteFailed = 0;
    QString m_pasteMode;
    bool m_pasteReported = false;
    int m_undoCount = 0;
    int m_redoCount = 0;
    QString m_undoLabel;
    QString m_redoLabel;
    QString m_undoMessage;
    bool m_watching = false;
    int m_watchUpdates = 0;
};