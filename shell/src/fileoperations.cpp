// SPDX-License-Identifier: GPL-3.0-or-later
#include "fileoperations.h"

#include "files_core_list.h"

#include <QDir>
#include <QFileInfo>
#include <QMetaObject>
#include <QThreadPool>
#include <QUrl>
#include <QVariantMap>

#include <atomic>
#include <filesystem>
#include <system_error>
#include <unistd.h>

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

namespace fs = std::filesystem;

// One cross-filesystem copy in progress. `failAfterEntries` is the test seam
// that stops the copy mid-tree; production leaves it negative.
struct MoveCopyState
{
    int copiedEntries = 0;
    int failAfterEntries = -1;
    QString error;
};

fs::path nativePath(const QString &path)
{
    return fs::path(path.toStdString());
}

QString displayPath(const fs::path &path)
{
    return QString::fromStdString(path.string());
}

// `QFileInfo::exists` follows symlinks, so a broken link would look absent.
bool pathExists(const QString &path)
{
    const QFileInfo info(path);
    return info.exists() || info.isSymLink();
}

QString copyFailure(const fs::path &path, const std::error_code &error)
{
    return QStringLiteral("cannot copy %1: %2")
        .arg(displayPath(path), QString::fromStdString(error.message()));
}

// Copy permissions and modification time where the filesystem supports them.
// Metadata is best-effort: losing it must not turn a completed move into a
// failure, and a filesystem without POSIX modes simply rejects the call.
void preserveMetadata(const fs::path &src, const fs::path &dst)
{
    std::error_code error;
    const fs::file_status status = fs::status(src, error);
    if (!error)
        fs::permissions(dst, status.permissions(), fs::perm_options::replace, error);

    error.clear();
    const fs::file_time_type mtime = fs::last_write_time(src, error);
    if (!error)
        fs::last_write_time(dst, mtime, error);
}

// Make a copied tree removable even when the source had read-only directory
// modes. Cleanup must not fail because a metadata-preserving copy already
// applied the source's permissions to the partial temp.
void makeTreeWritable(const fs::path &path)
{
    std::error_code error;
    const fs::file_status status = fs::symlink_status(path, error);
    if (error || fs::is_symlink(status) || !fs::is_directory(status))
        return;
    fs::permissions(path, fs::perms::owner_write | fs::perms::owner_exec,
                    fs::perm_options::add, error);

    error.clear();
    fs::directory_iterator it(path, error);
    if (error)
        return;
    for (const fs::directory_entry &entry : it)
        makeTreeWritable(entry.path());
}

void removeTree(const QString &path)
{
    makeTreeWritable(nativePath(path));
    std::error_code error;
    fs::remove_all(nativePath(path), error);
    if (error) {
        qWarning() << "fileoperations: could not remove temporary" << path
                   << QString::fromStdString(error.message());
    }
}

// Copy one entry (file, directory, or symlink) and, for directories, its whole
// subtree. A failure leaves the partial destination for the caller to remove;
// the source is never touched here.
bool copyEntry(const fs::path &src, const fs::path &dst, MoveCopyState *state)
{
    if (state->failAfterEntries >= 0
        && state->copiedEntries >= state->failAfterEntries) {
        state->error = QStringLiteral("simulated copy failure after %1 entries")
                           .arg(state->copiedEntries);
        return false;
    }

    std::error_code error;
    const fs::file_status status = fs::symlink_status(src, error);
    if (error) {
        state->error = QStringLiteral("cannot inspect %1: %2")
                           .arg(displayPath(src),
                                QString::fromStdString(error.message()));
        return false;
    }

    if (fs::is_symlink(status)) {
        const fs::path target = fs::read_symlink(src, error);
        if (error) {
            state->error = copyFailure(src, error);
            return false;
        }
        fs::create_symlink(target, dst, error);
        if (error) {
            state->error = copyFailure(src, error);
            return false;
        }
    } else if (fs::is_directory(status)) {
        fs::create_directory(dst, error);
        if (error) {
            state->error = copyFailure(src, error);
            return false;
        }
        fs::directory_iterator it(src, error);
        if (error) {
            state->error = copyFailure(src, error);
            return false;
        }
        while (it != fs::directory_iterator()) {
            const fs::path child = it->path();
            if (!copyEntry(child, dst / child.filename(), state))
                return false;
            it.increment(error);
            if (error) {
                state->error = copyFailure(src, error);
                return false;
            }
        }
        // After the children, so creating them does not rewrite the mtime.
        preserveMetadata(src, dst);
    } else if (fs::is_regular_file(status)) {
        fs::copy_file(src, dst, fs::copy_options::none, error);
        if (error) {
            state->error = copyFailure(src, error);
            return false;
        }
        preserveMetadata(src, dst);
    } else {
        state->error = QStringLiteral("cannot move %1: unsupported file type")
                           .arg(displayPath(src));
        return false;
    }

    ++state->copiedEntries;
    return true;
}

// A free hidden sibling in the destination folder. It is on the destination
// filesystem by construction, so the final rename cannot cross a mount.
QString makeTempSibling(const QString &parent)
{
    static std::atomic<quint64> counter{0};
    for (;;) {
        const QString candidate =
            parent + QStringLiteral("/.dragonfruit-move-%1-%2")
                         .arg(::getpid())
                         .arg(counter.fetch_add(1) + 1);
        if (!pathExists(candidate))
            return candidate;
    }
}

// Cross-filesystem move: copy the whole entry into a temp sibling, rename the
// temp into place, then remove the source. Any copy or rename failure removes
// the temp and leaves the source complete.
bool moveAcrossFilesystems(const QString &src, const QString &dst,
                           const FileMoveOptions &options, QString *error)
{
    const QString parent = QFileInfo(dst).absolutePath();
    if (parent.isEmpty()) {
        if (error)
            *error = QStringLiteral("cannot resolve the destination folder for %1").arg(dst);
        return false;
    }

    const QString temp = makeTempSibling(parent);
    MoveCopyState state;
    state.failAfterEntries = options.failAfterEntries;
    if (!copyEntry(nativePath(src), nativePath(temp), &state)) {
        removeTree(temp);
        if (error)
            *error = state.error;
        return false;
    }

    // `uniqueName` reserved this name before the copy. Re-check so a
    // concurrently created entry is never overwritten by the rename.
    if (pathExists(dst)) {
        removeTree(temp);
        if (error)
            *error = QStringLiteral("destination appeared during the move: %1").arg(dst);
        return false;
    }

    std::error_code renameError;
    fs::rename(nativePath(temp), nativePath(dst), renameError);
    if (renameError) {
        removeTree(temp);
        if (error) {
            *error = QStringLiteral("cannot rename %1 into place: %2")
                         .arg(temp, QString::fromStdString(renameError.message()));
        }
        return false;
    }

    // The destination is complete. Removing the source is cleanup; if it
    // fails, deleting a good destination would lose data, so the move stays
    // complete and the leftover is reported. Read-only source directories are
    // made writable so the normal case still finishes.
    makeTreeWritable(nativePath(src));
    std::error_code removeError;
    fs::remove_all(nativePath(src), removeError);
    if (removeError) {
        qWarning() << "fileoperations: moved" << src << "to" << dst
                   << "but could not remove the source:"
                   << QString::fromStdString(removeError.message());
    }
    return true;
}

// Move a file or directory. Same-filesystem moves are one atomic rename;
// EXDEV selects the copy-to-temp path.
bool movePath(const QString &src, const QString &dst,
              const FileMoveOptions &options, QString *error)
{
    if (!options.forceCrossFilesystem) {
        std::error_code renameError;
        fs::rename(nativePath(src), nativePath(dst), renameError);
        if (!renameError)
            return true;
        if (renameError != std::errc::cross_device_link) {
            if (error) {
                *error = QStringLiteral("cannot move %1 into %2: %3")
                             .arg(src, dst,
                                  QString::fromStdString(renameError.message()));
            }
            return false;
        }
    }
    return moveAcrossFilesystems(src, dst, options, error);
}

// A free name inside `dir` for `name`: `name`, else `name.N`. A broken
// symlink counts as occupied so the rename cannot replace it.
QString uniqueName(const QString &dir, const QString &name)
{
    if (!pathExists(dir + QLatin1Char('/') + name))
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
        if (!pathExists(dir + QLatin1Char('/') + candidate))
            return candidate;
    }
}

} // namespace

int moveFilesIntoFolder(const QString &target, const QStringList &sources,
                        QString *error, const FileMoveOptions &options)
{
    QString lastError;
    const QList<FileOperationItem> outcomes =
        moveFilesIntoFolderDetailed(target, sources, &lastError, options);
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
                                                     QString *error,
                                                     const FileMoveOptions &options)
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
        } else if (info.isDir() && !info.isSymLink()
                   && root.startsWith(absolute + QLatin1Char('/'))) {
            // Copying a directory into its own subtree would recurse forever;
            // the cross-filesystem temp would land inside the source.
            item.error = QStringLiteral("refusing to move %1 into its own subtree")
                             .arg(absolute);
        } else if (!info.exists() && !info.isSymLink()) {
            item.error = QStringLiteral("no such file: %1").arg(absolute);
        } else {
            const QString name = uniqueName(root, info.fileName());
            QString moveError;
            if (movePath(absolute, root + QLatin1Char('/') + name, options, &moveError))
                item.ok = true;
            else if (!moveError.isEmpty())
                item.error = moveError;
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

quint64 FileOperations::moveIntoFolder(const QString &target, const QStringList &sources,
                                       const FileMoveOptions &options)
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
    m_mutationPool->start([this, id, root, items, options]() {
        FileOperations::Result result;
        result.id = id;
        result.kind = Kind::Move;
        result.target = root;
        QString error;
        result.items = moveFilesIntoFolderDetailed(root, items, &error, options);
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
