// SPDX-License-Identifier: MIT
// The thin Qt model facade over `files-core` (T-10.4b, ADR 0049).
//
// `FilesDirectoryModel` is a `QAbstractListModel` that owns no filesystem
// logic. It starts a `files-core` listing over the C ABI (`ffi/files_core.h`)
// for its `location`, polls that session from a timer so no directory I/O ever
// runs on the UI thread, and repaints the model's ordered snapshot. Sorting,
// collation, stable node ids, and raw-byte names all stay in Rust; the QML
// views and the C++ roles are the only thing here.
//
// The C++ side is deliberately dump: every poll returns the whole ordered
// snapshot, which the model mirrors and forwards. T-10.5 replaces the
// whole-snapshot delivery with an incremental/windowed one (ADR 0049).
#pragma once

#include <QAbstractListModel>
#include <QString>
#include <QTimer>
#include <QVector>
#include <QtQml/qqmlregistration.h>

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

    int count() const { return m_nodes.size(); }
    QString state() const { return m_state; }
    QString errorMessage() const { return m_errorMessage; }
    QString sortKey() const { return m_sortKey; }
    bool sortAscending() const { return m_sortAscending; }
    bool foldersFirst() const { return m_foldersFirst; }

    // Sort by `key`, flipping direction when it is already active.
    Q_INVOKABLE void sortBy(const QString &key);
    // Force a fresh listing of the current location.
    Q_INVOKABLE void reload();

signals:
    void locationChanged();
    void countChanged();
    void stateChanged();
    void sortChanged();

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
    void applySnapshot(const struct df_files_event *event);
    void setState(const QString &state, const QString &error = QString());
    void applySort();
    static QString iconFor(int kind, const QString &name);
    static QString kindTextFor(int kind);
    static QString formatSize(quint64 bytes);
    static QString formatModified(qint64 unixMillis);

    void *m_session = nullptr;
    QTimer m_timer;
    QVector<Node> m_nodes;
    QString m_location;
    QString m_state = QStringLiteral("idle");
    QString m_errorMessage;
    QString m_sortKey = QStringLiteral("name");
    bool m_sortAscending = true;
    bool m_foldersFirst = true;
};