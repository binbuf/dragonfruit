// SPDX-License-Identifier: GPL-3.0-only
#include "FilesDirectoryModel.h"

#include "FilesThumbnailProvider.h"
#include "ffi/files_core.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QFileInfo>
#include <QLocale>
#include <QUrl>

#include <algorithm>
#include <utility>

namespace {

// The polling cadence while a listing is in flight. The Rust worker does the
// directory I/O on its own thread; this only drains ready events and stops as
// soon as the listing completes, so an idle window never polls.
//
// T332: a settled *watched* window keeps this timer running. Each tick is one
// blocking delta request in the Rust session (it wakes on the folder's change
// or after the timeout), so the change paints without a manual refresh, bursts
// arrive already coalesced by files-core, and an unwatched location (the
// Trash) still stops the timer when it settles.
constexpr int kPollIntervalMs = 16;

// How long an undo/redo waits for its worker outcome before letting the poll
// timer deliver it (T331). One rename is a single `rename(2)`.
constexpr int kReplayWaitMs = 2000;

bool isImage(const QString &suffix)
{
    static const QStringList kSuffixes = {
        QStringLiteral("png"),  QStringLiteral("jpg"),  QStringLiteral("jpeg"),
        QStringLiteral("jpe"),  QStringLiteral("jfif"), QStringLiteral("pjpeg"),
        QStringLiteral("pjp"),  QStringLiteral("gif"),  QStringLiteral("apng"),
        QStringLiteral("webp"), QStringLiteral("bmp"),  QStringLiteral("svg"),
        QStringLiteral("svgz"), QStringLiteral("tiff"), QStringLiteral("tif"),
        QStringLiteral("ico"),  QStringLiteral("avif"), QStringLiteral("heic"),
        QStringLiteral("heif"), QStringLiteral("jxl"),  QStringLiteral("dds"),
        QStringLiteral("exr"),  QStringLiteral("hdr"),  QStringLiteral("pcx"),
        QStringLiteral("tga"),  QStringLiteral("xcf"),  QStringLiteral("kra"),
        QStringLiteral("psd"),  QStringLiteral("raw"),
    };
    return kSuffixes.contains(suffix);
}

bool isAudio(const QString &suffix)
{
    static const QStringList kSuffixes = {
        QStringLiteral("mp3"), QStringLiteral("flac"), QStringLiteral("ogg"),
        QStringLiteral("wav"), QStringLiteral("m4a"), QStringLiteral("opus"),
    };
    return kSuffixes.contains(suffix);
}

bool isVideo(const QString &suffix)
{
    static const QStringList kSuffixes = {
        QStringLiteral("mp4"),  QStringLiteral("mkv"),  QStringLiteral("mov"),
        QStringLiteral("webm"), QStringLiteral("avi"),  QStringLiteral("m4v"),
        QStringLiteral("mpg"),  QStringLiteral("mpeg"), QStringLiteral("wmv"),
        QStringLiteral("flv"),  QStringLiteral("3gp"),  QStringLiteral("ogv"),
        QStringLiteral("ts"),   QStringLiteral("m2ts"),
    };
    return kSuffixes.contains(suffix);
}

bool isDocument(const QString &suffix)
{
    static const QStringList kSuffixes = {
        QStringLiteral("pdf"),  QStringLiteral("txt"), QStringLiteral("md"),
        QStringLiteral("doc"),  QStringLiteral("docx"), QStringLiteral("odt"),
        QStringLiteral("rtf"),
    };
    return kSuffixes.contains(suffix);
}

} // namespace

FilesDirectoryModel::FilesDirectoryModel(QObject *parent)
    : QAbstractListModel(parent)
{
    m_timer.setInterval(kPollIntervalMs);
    connect(&m_timer, &QTimer::timeout, this, &FilesDirectoryModel::poll);
}

FilesDirectoryModel::~FilesDirectoryModel()
{
    stop();
}

int FilesDirectoryModel::rowCount(const QModelIndex &parent) const
{
    return parent.isValid() ? 0 : int(m_nodes.size());
}

QVariant FilesDirectoryModel::data(const QModelIndex &index, int role) const
{
    if (index.row() < 0 || index.row() >= int(m_nodes.size()))
        return {};
    const Node &node = m_nodes[size_t(index.row())];
    switch (role) {
    case NodeIdRole:
        return node.id;
    case NameRole:
        return node.name;
    case UriRole:
        return node.uri;
    case IsDirRole:
        return node.isDir;
    case KindRole:
        return node.kind;
    case KindTextRole:
        return node.kindText;
    case IconRole:
        return node.icon;
    case SizeRole:
        return node.size;
    case HasSizeRole:
        return node.hasSize;
    case SizeTextRole:
        return node.sizeText;
    case ModifiedRole:
        return node.modifiedMs;
    case HasModifiedRole:
        return node.hasModified;
    case ModifiedTextRole:
        return node.modifiedText;
    case SymlinkTargetRole:
        return node.symlinkTarget;
    case ThumbnailRole:
        return node.thumbnail;
    default:
        return {};
    }
}

QHash<int, QByteArray> FilesDirectoryModel::roleNames() const
{
    return {
        { NodeIdRole, "nodeId" },
        { NameRole, "name" },
        { UriRole, "uri" },
        { IsDirRole, "isDir" },
        { KindRole, "kind" },
        { KindTextRole, "kindText" },
        { IconRole, "icon" },
        { SizeRole, "size" },
        { HasSizeRole, "hasSize" },
        { SizeTextRole, "sizeText" },
        { ModifiedRole, "modified" },
        { HasModifiedRole, "hasModified" },
        { ModifiedTextRole, "modifiedText" },
        { SymlinkTargetRole, "symlinkTarget" },
        { ThumbnailRole, "thumbnail" },
    };
}

void FilesDirectoryModel::setLocation(const QString &uri)
{
    if (m_location == uri)
        return;
    m_location = uri;
    emit locationChanged();
    start();
}

void FilesDirectoryModel::reload()
{
    start();
}

bool FilesDirectoryModel::rename(quint64 nodeId, const QString &newName)
{
    if (!m_session || newName.isEmpty())
        return false;
    const quint64 op = df_files_begin_rename(m_session, nodeId,
                                             newName.toUtf8().constData());
    if (op == 0)
        return false;
    // Paint the optimistic edit now, whatever the timer is doing.
    repaint();
    setPendingOps(int(df_files_pending_ops(m_session)));
    ensurePolling();
    return true;
}

bool FilesDirectoryModel::newFolder(const QString &parentUri)
{
    if (!m_session)
        return false;
    const QString parent = parentUri.isEmpty() ? m_location : parentUri;
    if (parent.isEmpty())
        return false;
    const quint64 op = df_files_begin_new_folder(m_session,
                                                 parent.toUtf8().constData());
    if (op == 0)
        return false;
    repaint();
    setPendingOps(int(df_files_pending_ops(m_session)));
    ensurePolling();
    return true;
}

bool FilesDirectoryModel::trash(quint64 nodeId)
{
    if (!m_session)
        return false;
    const quint64 op = df_files_begin_trash(m_session, nodeId);
    if (op == 0)
        return false;
    repaint();
    setPendingOps(int(df_files_pending_ops(m_session)));
    ensurePolling();
    return true;
}

bool FilesDirectoryModel::emptyTrash()
{
    if (!m_session)
        return false;
    const quint64 op = df_files_begin_empty_trash(m_session);
    if (op == 0)
        return false;
    repaint();
    setPendingOps(int(df_files_pending_ops(m_session)));
    ensurePolling();
    return true;
}

bool FilesDirectoryModel::pasteItem(const QString &targetUri, const QString &sourceUri,
                                    const QString &name, bool isDir,
                                    const QString &mode, const QString &policy)
{
    if (!m_session || targetUri.isEmpty() || sourceUri.isEmpty() || name.isEmpty())
        return false;
    const int modeValue = mode == QStringLiteral("cut") ? DF_FILES_CLIPBOARD_CUT
                                                        : DF_FILES_CLIPBOARD_COPY;
    int policyValue = DF_FILES_CONFLICT_KEEP_BOTH;
    if (policy == QStringLiteral("replace"))
        policyValue = DF_FILES_CONFLICT_REPLACE;
    else if (policy == QStringLiteral("skip"))
        policyValue = DF_FILES_CONFLICT_SKIP;
    const int kind = isDir ? DF_NODE_DIRECTORY : DF_NODE_FILE;
    const quint64 op = df_files_begin_paste(
            m_session, targetUri.toUtf8().constData(),
            sourceUri.toUtf8().constData(), name.toUtf8().constData(), kind,
            modeValue, policyValue);
    if (op == 0)
        return false;
    // A new batch starts its own progress report.
    m_pasteReported = false;
    repaint();
    setPendingOps(int(df_files_pending_ops(m_session)));
    updatePasteProgress();
    ensurePolling();
    return true;
}

QStringList FilesDirectoryModel::names() const
{
    QStringList out;
    out.reserve(int(m_nodes.size()));
    for (const Node &node : m_nodes)
        out.append(node.name);
    return out;
}

QString FilesDirectoryModel::pasteProgressText() const
{
    if (m_pasteActive > 0) {
        const QString verb = m_pasteMode == QStringLiteral("cut")
                ? tr("Moving") : tr("Copying");
        const int position = qMin(m_pasteDone + 1, m_pasteTotal);
        return tr("%1 %2 of %3…").arg(verb).arg(position).arg(m_pasteTotal);
    }
    if (m_pasteTotal > 0 && m_pasteDone >= m_pasteTotal) {
        if (m_pasteFailed > 0)
            return tr("%1 failed").arg(m_pasteFailed);
        if (m_pasteSkipped > 0)
            return tr("%1 skipped").arg(m_pasteSkipped);
        const QString verb = m_pasteMode == QStringLiteral("cut")
                ? tr("Moved") : tr("Copied");
        return tr("%1 %2").arg(verb).arg(m_pasteCopied);
    }
    return {};
}

int FilesDirectoryModel::rowForNodeId(quint64 nodeId) const
{
    for (int row = 0; row < int(m_nodes.size()); ++row) {
        if (m_nodes[size_t(row)].id == nodeId)
            return row;
    }
    return -1;
}

quint64 FilesDirectoryModel::nodeIdAt(int row) const
{
    if (row < 0 || row >= int(m_nodes.size()))
        return 0;
    return m_nodes[size_t(row)].id;
}

QString FilesDirectoryModel::uriAt(int row) const
{
    if (row < 0 || row >= int(m_nodes.size()))
        return {};
    return m_nodes[size_t(row)].uri;
}

bool FilesDirectoryModel::isDirAt(int row) const
{
    if (row < 0 || row >= int(m_nodes.size()))
        return false;
    return m_nodes[size_t(row)].isDir;
}

QVariantList FilesDirectoryModel::allNodeIds() const
{
    QVariantList ids;
    ids.reserve(int(m_nodes.size()));
    for (const Node &node : m_nodes)
        ids.append(node.id);
    return ids;
}

QVariantMap FilesDirectoryModel::nodeInfo(quint64 nodeId) const
{
    const int row = rowForNodeId(nodeId);
    if (row < 0)
        return {};
    const Node &node = m_nodes[size_t(row)];
    const QUrl url(node.uri);
    const QFileInfo fileInfo(url.isLocalFile() ? url.toLocalFile() : QString());

    QVariantMap info;
    info.insert(QStringLiteral("id"), node.id);
    info.insert(QStringLiteral("name"), node.name);
    info.insert(QStringLiteral("uri"), node.uri);
    info.insert(QStringLiteral("isDir"), node.isDir);
    info.insert(QStringLiteral("kindText"), node.kindText);
    info.insert(QStringLiteral("icon"), node.icon);
    info.insert(QStringLiteral("size"), node.size);
    info.insert(QStringLiteral("hasSize"), node.hasSize);
    info.insert(QStringLiteral("sizeText"), node.sizeText);
    info.insert(QStringLiteral("modified"), node.modifiedMs);
    info.insert(QStringLiteral("hasModified"), node.hasModified);
    info.insert(QStringLiteral("modifiedText"), node.modifiedText);
    info.insert(QStringLiteral("symlinkTarget"), node.symlinkTarget);
    info.insert(QStringLiteral("thumbnail"), node.thumbnail);
    info.insert(QStringLiteral("extension"), fileInfo.suffix());
    info.insert(QStringLiteral("whereText"),
                url.isLocalFile() ? fileInfo.absolutePath() : QString());
    return info;
}

QVariantMap FilesDirectoryModel::selectionInfo(const QVariantList &nodeIds) const
{
    int folderCount = 0;
    int fileCount = 0;
    quint64 totalSize = 0;
    bool hasTotalSize = false;
    for (const QVariant &value : nodeIds) {
        const int row = rowForNodeId(value.toULongLong());
        if (row < 0)
            continue;
        const Node &node = m_nodes[size_t(row)];
        if (node.isDir)
            ++folderCount;
        else
            ++fileCount;
        if (node.hasSize) {
            totalSize += node.size;
            hasTotalSize = true;
        }
    }

    QVariantMap info;
    info.insert(QStringLiteral("count"), folderCount + fileCount);
    info.insert(QStringLiteral("folderCount"), folderCount);
    info.insert(QStringLiteral("fileCount"), fileCount);
    info.insert(QStringLiteral("totalSize"), QVariant::fromValue(totalSize));
    info.insert(QStringLiteral("hasTotalSize"), hasTotalSize);
    info.insert(QStringLiteral("totalSizeText"),
                hasTotalSize ? formatSize(totalSize) : QString());
    return info;
}

void FilesDirectoryModel::sortBy(const QString &key)
{
    if (key != QStringLiteral("name") && key != QStringLiteral("kind")
        && key != QStringLiteral("size") && key != QStringLiteral("modified"))
        return;
    if (m_sortKey == key)
        m_sortAscending = !m_sortAscending;
    else {
        m_sortKey = key;
        m_sortAscending = true;
    }
    applySort();
    emit sortChanged();
}

void FilesDirectoryModel::start()
{
    stop();
    beginResetModel();
    m_nodes.clear();
    endResetModel();
    emit countChanged();

    if (m_location.isEmpty()) {
        setState(QStringLiteral("idle"));
        return;
    }
    const QByteArray uri = m_location.toUtf8();
    // The T-10.5 perf fixture: a synthetic 100k listing behind the same ABI,
    // selected only for a `/synthetic` location so it never shadows a real
    // folder. `DF_FILES_SYNTHETIC_COUNT`/`_BATCH` are set by the perf runner.
    const int syntheticCount = qEnvironmentVariableIntValue("DF_FILES_SYNTHETIC_COUNT");
    if (syntheticCount > 0 && m_location.endsWith(QStringLiteral("/synthetic"))) {
        const int batch = qEnvironmentVariableIntValue("DF_FILES_SYNTHETIC_BATCH");
        m_session = df_files_begin_synthetic(uri.constData(), uint32_t(syntheticCount),
                                             uint32_t(batch));
    } else {
        m_session = df_files_begin(uri.constData());
    }
    if (!m_session) {
        setState(QStringLiteral("error"), QStringLiteral("Not a browsable location"));
        return;
    }
    const bool watching = df_files_watching(m_session) != 0;
    if (watching != m_watching) {
        m_watching = watching;
        m_watchUpdates = 0;
        emit watchChanged();
    }
    setState(QStringLiteral("streaming"));
    m_timer.start();
    poll();
}

void FilesDirectoryModel::stop()
{
    m_timer.stop();
    if (m_session) {
        df_files_free(m_session);
        m_session = nullptr;
    }
    if (m_watching) {
        m_watching = false;
        m_watchUpdates = 0;
        emit watchChanged();
    }
    setPendingOps(0);
    // A new session has no history: the undo stacks are folder-scoped.
    updateUndoState();
    // A new session starts with no paste history. The clipboard itself lives
    // in the shell and survives navigation.
    if (m_pasteTotal != 0 || m_pasteDone != 0 || m_pasteActive != 0
        || m_pasteCopied != 0 || m_pasteSkipped != 0 || m_pasteFailed != 0
        || !m_pasteMode.isEmpty()) {
        m_pasteTotal = 0;
        m_pasteDone = 0;
        m_pasteActive = 0;
        m_pasteCopied = 0;
        m_pasteSkipped = 0;
        m_pasteFailed = 0;
        m_pasteMode.clear();
        emit pasteProgressChanged();
    }
    m_pasteReported = false;
}

void FilesDirectoryModel::poll()
{
    if (!m_session)
        return;
    df_files_delta *delta = df_files_poll_delta(m_session, kPollIntervalMs * 2);
    if (!delta) {
        stop();
        setState(QStringLiteral("error"), QStringLiteral("The listing session was lost"));
        return;
    }
    bool painted = false;
    switch (delta->status) {
    case DF_FILES_STATUS_BATCH:
        painted = applyDelta(delta);
        break;
    case DF_FILES_STATUS_DONE:
        // The listing finished, but the session stays alive: an optimistic
        // operation may still be in flight, and sorting/operating on a loaded
        // folder must keep working.
        setState(QStringLiteral("complete"));
        break;
    case DF_FILES_STATUS_ERROR:
        m_timer.stop();
        setState(QStringLiteral("error"),
                 delta->error ? QString::fromUtf8(delta->error)
                              : QStringLiteral("The folder could not be listed"));
        break;
    case DF_FILES_STATUS_TIMEOUT:
    default:
        break;
    }
    const int kind = delta->kind;
    const int status = delta->status;
    df_files_delta_free(delta);

    takeError();
    const int pending = m_session ? int(df_files_pending_ops(m_session)) : 0;
    setPendingOps(pending);
    updatePasteProgress();
    updateUndoState();
    // T332: a change folded from the folder watcher (an insertion, an
    // in-place rename, a removal) is a watch update; the counter lets the
    // shell and the tests see that a burst painted without a reload.
    if (painted && status == DF_FILES_STATUS_BATCH
        && (kind == DF_FILES_DELTA_INSERT || kind == DF_FILES_DELTA_CHANGED
            || kind == DF_FILES_DELTA_REMOVED)
        && m_state == QStringLiteral("complete")) {
        ++m_watchUpdates;
        emit watchChanged();
    }
    // Stop the timer (never free the session) once the listing is settled,
    // no operation is pending, and no watcher is armed. A watched folder
    // keeps the timer: each tick is one blocking delta request that wakes on
    // the folder's next change, so an external edit paints by itself.
    if (pending == 0 && m_state != QStringLiteral("streaming") && !m_watching)
        m_timer.stop();
}

void FilesDirectoryModel::updatePasteProgress()
{
    if (!m_session)
        return;
    const df_files_paste_progress progress = df_files_paste_state(m_session);
    const int total = int(progress.total);
    const int done = int(progress.done);
    const int active = int(progress.active);
    const int copied = int(progress.copied);
    const int skipped = int(progress.skipped);
    const int failed = int(progress.failed);
    const QString mode = progress.mode == DF_FILES_CLIPBOARD_CUT
            ? QStringLiteral("cut")
            : progress.mode == DF_FILES_CLIPBOARD_COPY
                  ? QStringLiteral("copy") : QString();
    if (m_pasteTotal != total || m_pasteDone != done || m_pasteActive != active
        || m_pasteCopied != copied || m_pasteSkipped != skipped
        || m_pasteFailed != failed || m_pasteMode != mode) {
        m_pasteTotal = total;
        m_pasteDone = done;
        m_pasteActive = active;
        m_pasteCopied = copied;
        m_pasteSkipped = skipped;
        m_pasteFailed = failed;
        m_pasteMode = mode;
        emit pasteProgressChanged();
    }
    if (active == 0 && total > 0 && done >= total && !m_pasteReported) {
        m_pasteReported = true;
        emit pasteFinished(copied, skipped, failed);
    }
}

void FilesDirectoryModel::updateUndoState()
{
    int undoCount = 0;
    int redoCount = 0;
    QString undoLabel;
    QString redoLabel;
    QString message;
    df_files_undo_state state = {};
    if (m_session) {
        state = df_files_undo_status(m_session);
        // The C strings belong to the session; copy before the next call.
        undoCount = int(state.undo_len);
        redoCount = int(state.redo_len);
        undoLabel = state.undo_label ? QString::fromUtf8(state.undo_label) : QString();
        redoLabel = state.redo_label ? QString::fromUtf8(state.redo_label) : QString();
        message = state.message ? QString::fromUtf8(state.message) : QString();
        df_files_undo_state_free(state);
    }
    if (undoCount == m_undoCount && redoCount == m_redoCount
        && undoLabel == m_undoLabel && redoLabel == m_redoLabel
        && message == m_undoMessage) {
        return;
    }
    m_undoCount = undoCount;
    m_redoCount = redoCount;
    m_undoLabel = undoLabel;
    m_redoLabel = redoLabel;
    m_undoMessage = message;
    emit undoStateChanged();
}

bool FilesDirectoryModel::undo()
{
    return replay(false);
}

bool FilesDirectoryModel::redo()
{
    return replay(true);
}

bool FilesDirectoryModel::replay(bool redo)
{
    if (!m_session || m_state == QStringLiteral("streaming"))
        return false;
    if (df_files_undo(m_session, redo ? 1 : 0) == 0)
        return false;
    // The Rust session folds the replay's outcome into the model on this
    // thread (the caller is the keyboard/menu handler) and hands back the
    // rebuilt snapshot, so the undone rows are painted before the key returns.
    df_files_await_ops(m_session, kReplayWaitMs);
    // Fold the outcome first (it updates the undo copies and the message),
    // then repaint from the rebuild: both land before the key returns.
    repaint();
    updateUndoState();
    setPendingOps(m_session ? int(df_files_pending_ops(m_session)) : 0);
    ensurePolling();
    return true;
}

bool FilesDirectoryModel::putBack(quint64 nodeId)
{
    if (!m_session || df_files_put_back(m_session, nodeId) == 0) {
        takeError();
        return false;
    }
    updateUndoState();
    repaint();
    ensurePolling();
    return true;
}

bool FilesDirectoryModel::noteTrash(quint64 nodeId, const QString &oldUri,
                                    const QString &trashPath)
{
    if (!m_session || oldUri.isEmpty() || trashPath.isEmpty())
        return false;
    const QByteArray uriBytes = oldUri.toUtf8();
    const QByteArray pathBytes = trashPath.toUtf8();
    if (df_files_note_trash(m_session, nodeId, uriBytes.constData(),
                            pathBytes.constData())
        == 0) {
        return false;
    }
    updateUndoState();
    return true;
}

void FilesDirectoryModel::repaint()
{
    if (!m_session)
        return;
    df_files_delta *delta = df_files_snapshot_delta(m_session);
    if (delta) {
        applyDelta(delta);
        df_files_delta_free(delta);
    }
}

void FilesDirectoryModel::ensurePolling()
{
    if (m_session && !m_timer.isActive())
        m_timer.start();
}

void FilesDirectoryModel::setPendingOps(int pending)
{
    if (m_pendingOps == pending)
        return;
    m_pendingOps = pending;
    emit pendingOpsChanged();
}

void FilesDirectoryModel::takeError()
{
    if (!m_session)
        return;
    char *message = df_files_take_error(m_session);
    if (!message)
        return;
    m_lastError = QString::fromUtf8(message);
    df_files_string_free(message);
    emit lastErrorChanged();
}

FilesDirectoryModel::Node FilesDirectoryModel::nodeFromFfi(const df_files_node &source)
{
    Node node;
    node.id = source.id;
    node.name = source.name ? QString::fromUtf8(source.name) : QString();
    node.uri = source.uri ? QString::fromUtf8(source.uri) : QString();
    node.kind = source.kind;
    node.isDir = source.kind == DF_NODE_DIRECTORY;
    node.kindText = kindTextFor(source.kind);
    node.icon = iconFor(source.kind, node.name);
    node.hasSize = source.has_size != 0;
    node.size = source.size;
    node.sizeText = node.hasSize ? formatSize(source.size) : QString();
    node.hasModified = source.has_modified != 0;
    node.modifiedMs = source.modified_ms;
    node.modifiedText =
            node.hasModified ? formatModified(source.modified_ms) : QString();
    node.symlinkTarget = source.symlink_target
            ? QString::fromUtf8(source.symlink_target)
            : QString();
    node.thumbnail = thumbnailFor(source.kind, node.uri, node.name);
    return node;
}

bool FilesDirectoryModel::applyDelta(const df_files_delta *delta)
{
    if (!delta)
        return false;
    switch (delta->kind) {
    case DF_FILES_DELTA_RESET:
        applyReset(delta);
        return true;
    case DF_FILES_DELTA_INSERT:
        applyInsertions(delta);
        return true;
    case DF_FILES_DELTA_CHANGED:
        applyChanges(delta);
        return true;
    case DF_FILES_DELTA_REMOVED:
        // A removal shifts every later rank and the delta carries no full
        // order: re-read the session's ordered snapshot (a reset) so the rows
        // stay truthful rather than guessing a gap (T332).
        if (m_session) {
            df_files_delta *full = df_files_snapshot_delta(m_session);
            if (full) {
                applyReset(full);
                df_files_delta_free(full);
                return true;
            }
        }
        return false;
    default:
        return false;
    }
}

void FilesDirectoryModel::dataChangedRows(int row, int count)
{
    if (count <= 0 || row < 0 || row >= int(m_nodes.size()))
        return;
    const int last = qMin(row + count - 1, int(m_nodes.size()) - 1);
    emit dataChanged(index(row), index(last));
}

void FilesDirectoryModel::applyChanges(const df_files_delta *delta)
{
    // T332: the held rows were replaced in place — a rename kept its node id,
    // a size changed. Only the named rows repaint; the row set and the
    // receiver's selection (keyed by node id) stay exactly as they are.
    if (!delta->changes || delta->change_count == 0)
        return;
    std::vector<quint32> changedIds;
    changedIds.reserve(size_t(delta->change_count));
    for (uint32_t i = 0; i < delta->change_count; ++i) {
        const df_files_change &change = delta->changes[i];
        const int row = int(change.rank);
        if (row < 0 || row >= int(m_nodes.size()))
            continue;
        const Node incoming = nodeFromFfi(change.node);
        Node &node = m_nodes[size_t(row)];
        if (node.id != incoming.id)
            continue;
        node = incoming;
        if (!changedIds.empty()) {
            int lo = 0;
            int hi = int(changedIds.size());
            while (lo < hi) {
                const int mid = (lo + hi) / 2;
                if (changedIds[size_t(mid)] < change.rank)
                    lo = mid + 1;
                else
                    hi = mid;
            }
            if (lo >= int(changedIds.size()) || changedIds[size_t(lo)] != change.rank)
                changedIds.insert(changedIds.begin() + lo, change.rank);
        } else {
            changedIds.push_back(change.rank);
        }
    }
    if (changedIds.empty())
        return;
    // One contiguous span covers a watch burst's changed rows; a sparse set
    // reports each run separately.
    size_t start = 0;
    while (start < changedIds.size()) {
        size_t end = start;
        while (end + 1 < changedIds.size()
               && changedIds[end + 1] == changedIds[end] + 1) {
            ++end;
        }
        emit dataChanged(index(int(changedIds[start])), index(int(changedIds[end])));
        start = end + 1;
    }
}

void FilesDirectoryModel::applyReset(const df_files_delta *delta)
{
    beginResetModel();
    m_nodes.clear();
    m_nodes.reserve(delta->total);
    for (uint32_t i = 0; i < delta->row_count; ++i)
        m_nodes.push_back(nodeFromFfi(delta->rows[i].node));
    endResetModel();
    emit countChanged();
}

void FilesDirectoryModel::applyInsertions(const df_files_delta *delta)
{
    const int incomingCount = int(delta->row_count);
    if (incomingCount <= 0)
        return;
    const int oldCount = int(m_nodes.size());
    const int newCount = oldCount + incomingCount;

    // Validate the delta: ranks strictly ascending and within the new length.
    // The Rust merge guarantees this; if it is ever violated, fall back to a
    // full reset rather than paint a wrong order.
    std::vector<Node> incoming;
    incoming.reserve(size_t(incomingCount));
    int previousRank = -1;
    bool valid = true;
    for (int i = 0; i < incomingCount; ++i) {
        const int rank = int(delta->rows[i].rank);
        if (rank <= previousRank || rank >= newCount) {
            valid = false;
            break;
        }
        previousRank = rank;
        incoming.push_back(nodeFromFfi(delta->rows[i].node));
    }
    if (!valid) {
        if (m_session) {
            df_files_delta *full = df_files_snapshot_delta(m_session);
            if (full) {
                applyReset(full);
                df_files_delta_free(full);
            }
        }
        return;
    }

    // Merge in one pass: the existing rows keep their relative order, so
    // walking the final ranks and pulling from the old rows or the incoming
    // ones reproduces the ordered projection exactly.
    std::vector<Node> merged;
    merged.reserve(size_t(newCount));
    int oldIndex = 0;
    int newIndex = 0;
    for (int position = 0; position < newCount; ++position) {
        if (newIndex < incomingCount && int(delta->rows[newIndex].rank) == position)
            merged.push_back(std::move(incoming[size_t(newIndex++)]));
        else
            merged.push_back(std::move(m_nodes[size_t(oldIndex++)]));
    }

    beginInsertRows(QModelIndex(), oldCount, newCount - 1);
    m_nodes = std::move(merged);
    endInsertRows();
    // The new rows shifted the existing ranks; refresh the visible delegates.
    if (oldCount > 0)
        emit dataChanged(index(0), index(oldCount - 1));
    emit countChanged();
}

void FilesDirectoryModel::setState(const QString &state, const QString &error)
{
    if (m_state == state && m_errorMessage == error)
        return;
    m_state = state;
    m_errorMessage = error;
    emit stateChanged();
}

void FilesDirectoryModel::applySort()
{
    if (!m_session)
        return;
    df_files_set_sort(m_session, m_sortKey.toUtf8().constData(),
                      m_sortAscending ? "ascending" : "descending",
                      m_foldersFirst ? 1 : 0);
    df_files_delta *delta = df_files_snapshot_delta(m_session);
    if (delta) {
        applyDelta(delta);
        df_files_delta_free(delta);
    }
}

QString FilesDirectoryModel::iconFor(int kind, const QString &name)
{
    if (kind == DF_NODE_DIRECTORY)
        return QStringLiteral("folder");
    if (kind != DF_NODE_FILE)
        return QStringLiteral("file");
    const QString suffix = QFileInfo(name).suffix().toLower();
    if (isImage(suffix))
        return QStringLiteral("wallpaper");
    if (isAudio(suffix))
        return QStringLiteral("music");
    if (isVideo(suffix))
        return QStringLiteral("movies");
    if (isDocument(suffix))
        return QStringLiteral("documents");
    return QStringLiteral("file");
}

QString FilesDirectoryModel::thumbnailFor(int kind, const QString &uri,
                                          const QString &name)
{
    Q_UNUSED(name)
    // Directories keep the folder glyph. Local images get a generated preview;
    // PDFs/videos do too when an optional helper is installed. Everything else
    // reuses a fresh freedesktop cache entry if another spec-conforming
    // thumbnailer made one.
    if (kind == DF_NODE_DIRECTORY)
        return {};
    if (FilesThumbnailProvider::canGenerate(uri))
        return FilesThumbnailProvider::sourceFor(uri);
    return FilesThumbnailProvider::cachedSourceFor(uri);
}

QString FilesDirectoryModel::kindTextFor(int kind)
{
    switch (kind) {
    case DF_NODE_DIRECTORY:
        return QCoreApplication::translate("FilesDirectoryModel", "Folder");
    case DF_NODE_FILE:
        return QCoreApplication::translate("FilesDirectoryModel", "File");
    case DF_NODE_SYMLINK:
        return QCoreApplication::translate("FilesDirectoryModel", "Alias");
    default:
        return QCoreApplication::translate("FilesDirectoryModel", "Item");
    }
}

QString FilesDirectoryModel::formatSize(quint64 bytes)
{
    // The default locale, not `QLocale::system()`: the localization helper
    // installs the effective locale at startup (T-16.7).
    return QLocale().formattedDataSize(static_cast<qint64>(bytes),
                                       2, QLocale::DataSizeIecFormat);
}

QString FilesDirectoryModel::formatModified(qint64 unixMillis)
{
    const QDateTime moment = QDateTime::fromMSecsSinceEpoch(unixMillis);
    return QLocale().toString(moment, QLocale::ShortFormat);
}