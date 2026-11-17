// SPDX-License-Identifier: MIT
#include "fileoperations.h"

#include "files_core_list.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QMetaObject>
#include <QThreadPool>
#include <QUrl>
#include <QVariantMap>

namespace {

// Read-only listings may overlap; three workers keep a burst of folder
// watcher events from starving the Dock without opening unbounded threads.
constexpr int kMaxListingWorkers = 3;
// Mutations are serialized on one worker: two moves must never race the same
// destination, and the per-target ordering matches the order the user dropped
// the items.
constexpr int kMaxMutationWorkers = 1;
// How many completed results stay queryable. Enough for a burst of drops and
// their retries; old entries age out.
constexpr int kRetainedResults = 32;

// Turn one files-core listing event into the stack's row maps.
QVariantList rowsFromEvent(const df_files_event *event)
{
    QVariantList rows;
    if (!event || !event->nodes)
        return rows;
    rows.reserve(static_cast<int>(event->count));
    for (uint32_t i = 0; i < event->count; ++i) {
        const df_files_node &node = event->nodes[i];
        const QString uri = node.uri ? QString::fromUtf8(node.uri) : QString();
        const QString local = QUrl(uri).toLocalFile();
        QVariantMap row;
        row.insert(QStringLiteral("name"), node.name ? QString::fromUtf8(node.name) : QString());
        row.insert(QStringLiteral("path"), local.isEmpty() ? uri : local);
        row.insert(QStringLiteral("isDir"), node.kind == DF_NODE_DIRECTORY);
        rows.append(row);
    }
    return rows;
}

// Blocking listing used only on a worker thread. The 64-poll guard matches the
// previous in-controller implementation; a folder that never answers is
// reported as a failure so the caller keeps its last snapshot.
QVariantList listFolderOnWorker(const QString &path, QString *error)
{
    const QByteArray encoded = QUrl::fromLocalFile(path).toString().toUtf8();
    void *session = df_files_begin(encoded.constData());
    if (!session) {
        *error = QStringLiteral("cannot list %1").arg(path);
        return {};
    }

    QVariantList rows;
    bool done = false;
    bool failed = false;
    for (int guard = 0; guard < 64; ++guard) {
        df_files_event *event = df_files_poll(session, 500);
        if (!event)
            break;
        const int status = event->status;
        if (status == DF_FILES_STATUS_BATCH) {
            rows = rowsFromEvent(event);
            df_files_event_free(event);
            continue;
        }
        if (status == DF_FILES_STATUS_ERROR) {
            if (event->error && *event->error)
                *error = QString::fromUtf8(event->error);
            else
                *error = QStringLiteral("cannot list %1").arg(path);
            df_files_event_free(event);
            failed = true;
            break;
        }
        const bool finished = status == DF_FILES_STATUS_DONE;
        df_files_event_free(event);
        if (finished) {
            done = true;
            break;
        }
        // TIMEOUT: keep waiting for the worker.
    }
    df_files_free(session);
    if (failed)
        return {};
    if (!done) {
        if (error->isEmpty())
            *error = QStringLiteral("timed out listing %1").arg(path);
        return {};
    }
    return rows;
}

// Move a file or directory, falling back to copy+remove across filesystems.
bool movePath(const QString &src, const QString &dst)
{
    if (QFile::rename(src, dst))
        return true;
    const QFileInfo info(src);
    if (info.isDir() && !info.isSymLink()) {
        if (!QDir().mkpath(dst))
            return false;
        const QDir dir(src);
        const QFileInfoList entries =
            dir.entryInfoList(QDir::AllEntries | QDir::NoDotAndDotDot | QDir::Hidden);
        for (const QFileInfo &entry : entries) {
            if (!movePath(entry.absoluteFilePath(), dst + QLatin1Char('/') + entry.fileName()))
                return false;
        }
        return QDir(src).removeRecursively();
    }
    if (QFile::copy(src, dst))
        return QFile::remove(src);
    return false;
}

// A free name inside `dir` for `name`: `name`, else `name.N`.
QString uniqueName(const QString &dir, const QString &name)
{
    if (!QFileInfo::exists(dir + QLatin1Char('/') + name))
        return name;
    QString stem = name;
    QString extension;
    const qsizetype dot = name.lastIndexOf(QLatin1Char('.'));
    if (dot > 0) {
        stem = name.left(dot);
        extension = name.mid(dot);
    }
    for (int i = 1;; ++i) {
        const QString candidate =
            stem + QLatin1Char('.') + QString::number(i) + extension;
        if (!QFileInfo::exists(dir + QLatin1Char('/') + candidate))
            return candidate;
    }
}

} // namespace

int moveFilesIntoFolder(const QString &target, const QStringList &sources, QString *error)
{
    QString lastError;
    const QList<FileOperationItem> outcomes =
        moveFilesIntoFolderDetailed(target, sources, &lastError);
    int moved = 0;
    for (const FileOperationItem &item : outcomes) {
        if (item.ok)
            ++moved;
    }
    if (error)
        *error = lastError;
    return moved;
}

QList<FileOperationItem> moveFilesIntoFolderDetailed(const QString &target,
                                                     const QStringList &sources,
                                                     QString *error)
{
    QList<FileOperationItem> outcomes;
    outcomes.reserve(sources.size());

    const QString root = QDir(target).absolutePath();
    if (root.isEmpty() || root == QLatin1String("/")) {
        const QString message = QStringLiteral("refusing to move into an unsafe root");
        for (const QString &path : sources) {
            if (path.isEmpty())
                continue;
            FileOperationItem item;
            item.path = path;
            item.error = message;
            outcomes.append(item);
        }
        if (error)
            *error = message;
        return outcomes;
    }

    if (error)
        error->clear();

    const bool targetReady = QDir().mkpath(root);
    QString targetError;
    if (!targetReady)
        targetError = QStringLiteral("cannot create %1").arg(root);

    for (const QString &path : sources) {
        if (path.isEmpty())
            continue;
        FileOperationItem item;
        item.path = path;
        if (!targetReady) {
            item.error = targetError;
            outcomes.append(item);
            continue;
        }

        const QFileInfo info(path);
        const QString absolute = info.absoluteFilePath();
        if (absolute == root || absolute.startsWith(root + QLatin1Char('/'))) {
            item.error = QStringLiteral("refusing to move %1 into itself").arg(absolute);
        } else if (!info.exists() && !info.isSymLink()) {
            item.error = QStringLiteral("no such file: %1").arg(absolute);
        } else {
            const QString name = uniqueName(root, info.fileName());
            if (movePath(absolute, root + QLatin1Char('/') + name))
                item.ok = true;
            else
                item.error = QStringLiteral("cannot move %1 into %2").arg(absolute, root);
        }
        if (!item.ok && error)
            *error = item.error;
        outcomes.append(item);
    }
    return outcomes;
}

FileOperations::FileOperations(QObject *parent)
    : QObject(parent)
{
    qRegisterMetaType<FileOperations::Result>("FileOperations::Result");
    m_listingPool = new QThreadPool(this);
    m_listingPool->setMaxThreadCount(kMaxListingWorkers);
    m_mutationPool = new QThreadPool(this);
    m_mutationPool->setMaxThreadCount(kMaxMutationWorkers);
}

FileOperations::~FileOperations()
{
    // No task may post a result after this object starts dying. Qt removes
    // already-queued events for the object when the base destructor runs.
    m_listingPool->waitForDone();
    m_mutationPool->waitForDone();
}

bool FileOperations::listFolder(const QString &path, quint64 requestId)
{
    if (path.isEmpty())
        return false;
    ++m_listingPending;
    m_listingPool->start([this, path, requestId]() {
        QString error;
        const QVariantList rows = listFolderOnWorker(path, &error);
        QMetaObject::invokeMethod(
            this,
            [this, path, requestId, rows, error]() {
                if (m_listingPending > 0)
                    --m_listingPending;
                if (error.isEmpty())
                    emit folderListed(path, requestId, rows);
                else
                    emit folderListFailed(path, requestId, error);
            },
            Qt::QueuedConnection);
    });
    return true;
}

quint64 FileOperations::moveIntoFolder(const QString &target, const QStringList &sources)
{
    QStringList items;
    items.reserve(sources.size());
    for (const QString &path : sources) {
        if (!path.isEmpty())
            items.append(path);
    }
    if (items.isEmpty() || target.trimmed().isEmpty())
        return 0;

    const quint64 id = m_nextOperationId++;
    ++m_operationPending;
    const QString root = QDir(target).absolutePath();
    m_mutationPool->start([this, id, root, items]() {
        FileOperations::Result result;
        result.id = id;
        result.kind = Kind::Move;
        result.target = root;
        QString error;
        result.items = moveFilesIntoFolderDetailed(root, items, &error);
        for (const FileOperationItem &item : result.items) {
            if (item.ok)
                ++result.succeeded;
            else
                ++result.failed;
        }
        result.ok = result.failed == 0;
        result.error = error;
        QMetaObject::invokeMethod(
            this, [this, result]() { finishOperation(result); }, Qt::QueuedConnection);
    });
    return id;
}

void FileOperations::finishOperation(const Result &result)
{
    // A repeated completion for an id that already finished is ignored, so a
    // retried or duplicated worker report updates the UI exactly once.
    if (m_completed.contains(result.id))
        return;
    m_completed.insert(result.id, result);
    m_completedOrder.append(result.id);
    while (m_completedOrder.size() > kRetainedResults)
        m_completed.remove(m_completedOrder.takeFirst());
    if (m_operationPending > 0)
        --m_operationPending;
    emit operationFinished(result);
}

FileOperations::Result FileOperations::completedOperation(quint64 id) const
{
    return m_completed.value(id);
}

bool FileOperations::hasCompletedOperation(quint64 id) const
{
    return m_completed.contains(id);
}
