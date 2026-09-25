// SPDX-License-Identifier: MIT
#include "FilesDirectoryModel.h"

#include "ffi/files_core.h"

#include <QDateTime>
#include <QFileInfo>
#include <QLocale>

namespace {

// The polling cadence while a listing is in flight. The Rust worker does the
// directory I/O on its own thread; this only drains ready events and stops as
// soon as the listing completes, so an idle window never polls.
constexpr int kPollIntervalMs = 16;

bool isImage(const QString &suffix)
{
    static const QStringList kSuffixes = {
        QStringLiteral("png"),  QStringLiteral("jpg"),  QStringLiteral("jpeg"),
        QStringLiteral("gif"),  QStringLiteral("webp"), QStringLiteral("bmp"),
        QStringLiteral("svg"),  QStringLiteral("tiff"), QStringLiteral("tif"),
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
        QStringLiteral("mp4"),  QStringLiteral("mkv"), QStringLiteral("mov"),
        QStringLiteral("webm"), QStringLiteral("avi"),
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
    return parent.isValid() ? 0 : m_nodes.size();
}

QVariant FilesDirectoryModel::data(const QModelIndex &index, int role) const
{
    if (index.row() < 0 || index.row() >= m_nodes.size())
        return {};
    const Node &node = m_nodes.at(index.row());
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

int FilesDirectoryModel::rowForNodeId(quint64 nodeId) const
{
    for (int row = 0; row < m_nodes.size(); ++row) {
        if (m_nodes.at(row).id == nodeId)
            return row;
    }
    return -1;
}

quint64 FilesDirectoryModel::nodeIdAt(int row) const
{
    if (row < 0 || row >= m_nodes.size())
        return 0;
    return m_nodes.at(row).id;
}

QString FilesDirectoryModel::uriAt(int row) const
{
    if (row < 0 || row >= m_nodes.size())
        return {};
    return m_nodes.at(row).uri;
}

bool FilesDirectoryModel::isDirAt(int row) const
{
    if (row < 0 || row >= m_nodes.size())
        return false;
    return m_nodes.at(row).isDir;
}

QVariantList FilesDirectoryModel::allNodeIds() const
{
    QVariantList ids;
    ids.reserve(m_nodes.size());
    for (const Node &node : m_nodes)
        ids.append(node.id);
    return ids;
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
    m_session = df_files_begin(m_location.toUtf8().constData());
    if (!m_session) {
        setState(QStringLiteral("error"), QStringLiteral("Not a browsable location"));
        return;
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
    setPendingOps(0);
}

void FilesDirectoryModel::poll()
{
    if (!m_session)
        return;
    df_files_event *event = df_files_poll(m_session, 0);
    if (!event) {
        stop();
        setState(QStringLiteral("error"), QStringLiteral("The listing session was lost"));
        return;
    }
    switch (event->status) {
    case DF_FILES_STATUS_BATCH:
        applySnapshot(event);
        break;
    case DF_FILES_STATUS_DONE:
        // The listing finished, but the session stays alive: an optimistic
        // operation may still be in flight, and sorting/operating on a loaded
        // folder must keep working. Polling just stops until there is work.
        setState(QStringLiteral("complete"));
        break;
    case DF_FILES_STATUS_ERROR:
        m_timer.stop();
        setState(QStringLiteral("error"),
                 event->error ? QString::fromUtf8(event->error)
                              : QStringLiteral("The folder could not be listed"));
        break;
    case DF_FILES_STATUS_TIMEOUT:
    default:
        break;
    }
    df_files_event_free(event);

    takeError();
    const int pending = m_session ? int(df_files_pending_ops(m_session)) : 0;
    setPendingOps(pending);
    // Stop the timer (never free the session) once the listing is settled and
    // no operation is pending, so an idle window does zero polling.
    if (pending == 0 && m_state != QStringLiteral("streaming"))
        m_timer.stop();
}

void FilesDirectoryModel::repaint()
{
    if (!m_session)
        return;
    df_files_event *event = df_files_snapshot(m_session);
    if (event) {
        applySnapshot(event);
        df_files_event_free(event);
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

void FilesDirectoryModel::applySnapshot(const df_files_event *event)
{
    beginResetModel();
    m_nodes.clear();
    m_nodes.reserve(static_cast<int>(event->count));
    for (uint32_t i = 0; i < event->count; ++i) {
        const df_files_node &source = event->nodes[i];
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
        m_nodes.append(node);
    }
    endResetModel();
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
    df_files_event *event = df_files_snapshot(m_session);
    if (event) {
        applySnapshot(event);
        df_files_event_free(event);
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

QString FilesDirectoryModel::kindTextFor(int kind)
{
    switch (kind) {
    case DF_NODE_DIRECTORY:
        return QStringLiteral("Folder");
    case DF_NODE_FILE:
        return QStringLiteral("File");
    case DF_NODE_SYMLINK:
        return QStringLiteral("Alias");
    default:
        return QStringLiteral("Item");
    }
}

QString FilesDirectoryModel::formatSize(quint64 bytes)
{
    return QLocale::system().formattedDataSize(static_cast<qint64>(bytes),
                                              2, QLocale::DataSizeIecFormat);
}

QString FilesDirectoryModel::formatModified(qint64 unixMillis)
{
    const QDateTime moment = QDateTime::fromMSecsSinceEpoch(unixMillis);
    return QLocale::system().toString(moment, QLocale::ShortFormat);
}