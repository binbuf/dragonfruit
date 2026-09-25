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
// The C++ side is deliberately dump: every poll returns the whole ordered
// snapshot, which the model mirrors and forwards. T-10.5 replaces the
// whole-snapshot delivery with an incremental/windowed one (ADR 0049).
#pragma once

#include <QAbstractListModel>
#include <QString>
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
    // Whether directories are grouped ahead of files (the Finder toggle).
    Q_PROPERTY(bool foldersFirst READ foldersFirst NOTIFY sortChanged)
    // How many optimistic operations are still awaiting their real outcome.
    // The model keeps polling while this is non-zero, even after the listing
    // completes (T-10.4c).
    Q_PROPERTY(int pendingOps READ pendingOps NOTIFY pendingOpsChanged)
    // The most recent operation that snapped back, for an inline notice; empty
    // when there is none (T-10.4c).
    Q_PROPERTY(QString lastError READ lastError NOTIFY lastErrorChanged)

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

signals:
    void locationChanged();
    void countChanged();
    void stateChanged();
    void sortChanged();
    void pendingOpsChanged();
    void lastErrorChanged();

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
    };

    void start();
    void stop();
    // Apply one incremental delta: a reset rebuilds the whole model, an
    // insertion delta merges only the new rows (T-10.5).
    void applyDelta(const struct df_files_delta *delta);
    void applyReset(const struct df_files_delta *delta);
    void applyInsertions(const struct df_files_delta *delta);
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
    static QString iconFor(int kind, const QString &name);
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
};