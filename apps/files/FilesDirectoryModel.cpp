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
        stop();
        setState(QStringLiteral("complete"));
        break;
    case DF_FILES_STATUS_ERROR:
        stop();
        setState(QStringLiteral("error"),
                 event->error ? QString::fromUtf8(event->error)
                              : QStringLiteral("The folder could not be listed"));
        break;
    case DF_FILES_STATUS_TIMEOUT:
    default:
        break;
    }
    df_files_event_free(event);
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