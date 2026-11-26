// SPDX-License-Identifier: GPL-3.0-or-later
#include "chooserbridge.h"

#include "files_core_list.h"
#include "portalpresenter.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QStandardPaths>
#include <QTimer>
#include <QUrl>
#include <QVariantMap>

#include <algorithm>

#if defined(QT_DBUS_LIB)
#include <QDBusArgument>
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusPendingCallWatcher>
#endif

namespace {

// The portal backend's diagnostic presenter surface (T-13.2a, ADR 0076).
const QString kService = QStringLiteral("org.freedesktop.impl.portal.desktop.dragonfruit");
const QString kPath = QStringLiteral("/org/freedesktop/portal/desktop");
const QString kInterface = QStringLiteral("org.dragonfruit.Portal1");

// A compact human-readable size for a file row.
QString formatSize(qint64 bytes)
{
    if (bytes < 0)
        return QString();
    if (bytes < 1024)
        return ChooserBridge::tr("%1 B").arg(bytes);
    if (bytes < 1024 * 1024)
        return ChooserBridge::tr("%1 KB").arg(QString::number(bytes / 1024.0, 'f', 1));
    if (bytes < 1024LL * 1024 * 1024)
        return ChooserBridge::tr("%1 MB").arg(QString::number(bytes / (1024.0 * 1024.0), 'f', 1));
    return ChooserBridge::tr("%1 GB").arg(QString::number(bytes / (1024.0 * 1024.0 * 1024.0), 'f',
                                                          1));
}

// One listed node as the picker's row map.
QVariantMap rowFromNode(const df_files_node &node)
{
    const bool directory = node.kind == DF_NODE_DIRECTORY;
    const qint64 size = node.has_size ? static_cast<qint64>(node.size) : -1;
    QVariantMap row;
    row.insert(QStringLiteral("name"),
               node.name ? QString::fromUtf8(node.name) : QString());
    row.insert(QStringLiteral("uri"), node.uri ? QString::fromUtf8(node.uri) : QString());
    row.insert(QStringLiteral("directory"), directory);
    row.insert(QStringLiteral("size"), size);
    row.insert(QStringLiteral("detail"), directory ? QString() : formatSize(size));
    return row;
}

// A local `file://` URI for a path.
QString fileUri(const QString &path)
{
    return QUrl::fromLocalFile(QDir::cleanPath(path)).toString();
}

// Decode a portal byte-array option, trimming the spec's trailing NUL.
QString decodePathBytes(QVariant value)
{
    QByteArray bytes = value.toByteArray();
#if defined(QT_DBUS_LIB)
    if (value.metaType() == QMetaType::fromType<QDBusArgument>()) {
        const QDBusArgument argument = value.value<QDBusArgument>();
        argument >> bytes;
    }
#endif
    while (bytes.endsWith('\0'))
        bytes.chop(1);
    if (bytes.isEmpty())
        return QString();
    return QFile::decodeName(bytes);
}

// Decode the portal's `aay` file-name list. Qt hands a nested array inside an
// `a{sv}` back as a QDBusArgument, so it must be walked by hand; a plain
// QVariantList is accepted too (the headless fixture and older Qt paths).
QStringList decodeNameList(const QVariant &value)
{
    QStringList names;
    const auto append = [&names](const QByteArray &raw) {
        QByteArray bytes = raw;
        while (bytes.endsWith('\0'))
            bytes.chop(1);
        const QString name = QFileInfo(QFile::decodeName(bytes)).fileName();
        if (!name.isEmpty())
            names.append(name);
    };
#if defined(QT_DBUS_LIB)
    if (value.metaType() == QMetaType::fromType<QDBusArgument>()) {
        const QDBusArgument argument = value.value<QDBusArgument>();
        argument.beginArray();
        while (!argument.atEnd()) {
            QByteArray bytes;
            if (argument.currentSignature() == QLatin1String("ay")) {
                argument >> bytes;
            } else {
                QVariant element;
                argument >> element;
                bytes = element.toByteArray();
            }
            append(bytes);
        }
        argument.endArray();
        return names;
    }
#endif
    const QVariantList values = value.toList();
    for (const QVariant &element : values)
        append(element.toByteArray());
    return names;
}

} // namespace

ChooserBridge::ChooserBridge(QObject *parent)
    : QObject(parent)
{
    m_listTimer = new QTimer(this);
    m_listTimer->setInterval(8);
    m_listTimer->setSingleShot(false);
    connect(m_listTimer, &QTimer::timeout, this, &ChooserBridge::onListTick);

    // The sidebar's favorites mirror Files's: Home first, then the XDG
    // user directories that really exist, then Computer. Recents is omitted
    // because it needs the GVfs `recent://` backend.
    const QString home = QDir::homePath();
    m_favorites.append(QVariantMap{{QStringLiteral("label"), tr("Home")},
                                   {QStringLiteral("icon"), QStringLiteral("home")},
                                   {QStringLiteral("uri"), fileUri(home)}});
    const struct {
        QStandardPaths::StandardLocation location;
        const char *icon;
    } kFavorites[] = {
        { QStandardPaths::DesktopLocation, "device-desktop" },
        { QStandardPaths::DocumentsLocation, "file-text" },
        { QStandardPaths::DownloadLocation, "download" },
        { QStandardPaths::PicturesLocation, "photo" },
        { QStandardPaths::MusicLocation, "music" },
        { QStandardPaths::MoviesLocation, "movie" },
    };
    for (const auto &favorite : kFavorites) {
        const QString path =
            QDir::cleanPath(QStandardPaths::writableLocation(favorite.location));
        if (path.isEmpty() || !QFileInfo(path).isDir())
            continue;
        const QString label = QFileInfo(path).fileName();
        if (label.isEmpty())
            continue;
        m_favorites.append(QVariantMap{
            { QStringLiteral("label"), label },
            { QStringLiteral("icon"), QString::fromLatin1(favorite.icon) },
            { QStringLiteral("uri"), fileUri(path) },
        });
    }
    m_favorites.append(QVariantMap{{QStringLiteral("label"), tr("Computer")},
                                   {QStringLiteral("icon"), QStringLiteral("device-desktop")},
                                   {QStringLiteral("uri"), QStringLiteral("file:///")}});
}

ChooserBridge::~ChooserBridge()
{
    stopListing();
}

QString ChooserBridge::acceptLabel() const
{
    if (!m_acceptLabel.isEmpty())
        return m_acceptLabel;
    if (saveMode())
        return tr("Save");
    if (saveFilesMode())
        return tr("Save Here");
    if (m_directory)
        return tr("Choose");
    return tr("Open");
}

QString ChooserBridge::currentLabel() const
{
    if (m_currentUri.isEmpty())
        return QString();
    const QString local = QUrl(m_currentUri).toLocalFile();
    return local.isEmpty() ? m_currentUri : local;
}

QVariantList ChooserBridge::breadcrumbs() const
{
    QVariantList crumbs;
    const QString path = currentDirectoryPath();
    if (path.isEmpty()) {
        if (!m_currentUri.isEmpty()) {
            crumbs.append(QVariantMap{{QStringLiteral("label"), m_currentUri},
                                      {QStringLiteral("uri"), m_currentUri}});
        }
        return crumbs;
    }
    crumbs.append(QVariantMap{{QStringLiteral("label"), tr("Computer")},
                              {QStringLiteral("uri"), QStringLiteral("file:///")}});
    QString accumulated;
    const QStringList parts = QDir::fromNativeSeparators(path).split(
        QLatin1Char('/'), Qt::SkipEmptyParts);
    for (const QString &part : parts) {
        accumulated += QLatin1Char('/');
        accumulated += part;
        crumbs.append(QVariantMap{{QStringLiteral("label"), part},
                                  {QStringLiteral("uri"), fileUri(accumulated)}});
    }
    return crumbs;
}

QVariantList ChooserBridge::selectedIndices() const
{
    QVariantList indices;
    for (int row : selectedRows())
        indices.append(row);
    return indices;
}

bool ChooserBridge::saveMode() const
{
    return m_kind == QLatin1String("save");
}

bool ChooserBridge::saveFilesMode() const
{
    return m_kind == QLatin1String("save-files");
}

bool ChooserBridge::canGoUp() const
{
    if (m_currentUri.isEmpty())
        return false;
    const QString path = QUrl(m_currentUri).toLocalFile();
    if (path.isEmpty())
        return false;
    const QString clean = QDir::cleanPath(path);
    return !clean.isEmpty() && clean != QLatin1String("/");
}

void ChooserBridge::setSaveName(const QString &name)
{
    if (name == m_saveName)
        return;
    m_saveName = name;
    // Editing the name invalidates a pending overwrite confirmation.
    m_confirmReplace = false;
    m_replaceName.clear();
    m_replacePath.clear();
    emit changed();
}

void ChooserBridge::connectService()
{
#if defined(QT_DBUS_LIB)
    if (m_connected)
        return;
    m_connected = true;
    QDBusConnection bus = QDBusConnection::sessionBus();
    bus.connect(kService, kPath, kInterface, QStringLiteral("FileChooserOpened"), this,
                SLOT(onFileChooserOpened(QString, QString, QString, QString, QString,
                                         QVariantMap)));
    if (QDBusConnectionInterface *iface = bus.interface()) {
        const auto update = [this](const QString &name) {
            if (name == kService)
                checkService();
        };
        connect(iface, &QDBusConnectionInterface::serviceRegistered, this, update);
        connect(iface, &QDBusConnectionInterface::serviceUnregistered, this, update);
    }
    checkService();
#endif
}

void ChooserBridge::checkService()
{
#if defined(QT_DBUS_LIB)
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *iface = bus.interface();
    const bool wasAvailable = m_serviceAvailable;
    m_serviceAvailable = iface && iface->isServiceRegistered(kService);
    // A restarted backend can hold chooser requests this shell never saw.
    if (m_serviceAvailable && !wasAvailable)
        queryPending();
#else
    m_serviceAvailable = false;
#endif
}

void ChooserBridge::onFileChooserOpened(const QString &handle, const QString &kind,
                                        const QString &appId, const QString &parentWindow,
                                        const QString &title, const QVariantMap &options)
{
    Q_UNUSED(parentWindow);
    Request request;
    request.handle = handle;
    request.kind = kind.isEmpty() ? QStringLiteral("open") : kind;
    request.appId = appId;
    request.title = title;
    request.multiple = options.value(QStringLiteral("multiple")).toBool();
    request.directory = options.value(QStringLiteral("directory")).toBool();
    request.startUri = suggestedUri(options);
    request.suggestedName = suggestedName(options);
    request.saveFiles = saveFilesFrom(options);
    request.fromPortal = true;
    m_queue.append(request);
    startNext();
}

void ChooserBridge::begin(const QString &handle, const QString &kind, const QString &title,
                          const QString &acceptLabel, bool multiple, bool directory,
                          const QString &startUri)
{
    Request request;
    request.handle = handle;
    request.kind = kind.isEmpty() ? QStringLiteral("open") : kind;
    request.title = title;
    request.acceptLabel = acceptLabel;
    request.multiple = multiple;
    request.directory = directory;
    request.startUri = startUri;
    m_queue.append(request);
    startNext();
}

void ChooserBridge::startNext()
{
    if (m_active || m_queue.isEmpty())
        return;
    const Request request = m_queue.takeFirst();
    m_active = true;
    m_fromPortal = request.fromPortal;
    m_handle = request.handle;
    m_kind = request.kind;
    m_appId = request.appId;
    m_title = request.title;
    m_acceptLabel = request.acceptLabel;
    m_multiple = request.multiple;
    m_directory = request.directory;
    m_suggestedName = request.suggestedName;
    m_saveFiles = request.saveFiles;
    m_saveName = request.suggestedName;
    m_error.clear();
    m_confirmReplace = false;
    m_replaceName.clear();
    m_replacePath.clear();
    m_selected.clear();
    m_current = -1;
    m_anchor = -1;

    // A request can suggest a folder that has gone away (or a non-local
    // scheme); fall back to Home rather than opening an empty, error-only
    // dialog with no way out.
    QString start = request.startUri;
    const QString startPath = QUrl(start).toLocalFile();
    if (start.isEmpty() || startPath.isEmpty() || !QFileInfo(startPath).isDir())
        start = fileUri(QDir::homePath());
    list(start);
    emit started();
}

void ChooserBridge::reset()
{
    stopListing();
    m_active = false;
    m_fromPortal = false;
    m_handle.clear();
    m_kind.clear();
    m_appId.clear();
    m_title.clear();
    m_acceptLabel.clear();
    m_multiple = false;
    m_directory = false;
    m_currentUri.clear();
    m_entries.clear();
    ++m_entriesRevision;
    m_loading = false;
    m_selected.clear();
    m_current = -1;
    m_anchor = -1;
    m_saveName.clear();
    m_suggestedName.clear();
    m_saveFiles.clear();
    m_error.clear();
    m_confirmReplace = false;
    m_replaceName.clear();
    m_replacePath.clear();
    emit changed();
}

void ChooserBridge::finish(bool completed)
{
    reset();
    emit finished(completed);
    startNext();
}

void ChooserBridge::refresh()
{
    if (m_currentUri.isEmpty())
        return;
    list(m_currentUri);
}

void ChooserBridge::browse(const QString &uri)
{
    if (uri.isEmpty() || uri == m_currentUri)
        return;
    const QString path = QUrl(uri).toLocalFile();
    if (path.isEmpty() || !QFileInfo(path).isDir()) {
        m_error = tr("Cannot open %1.").arg(uri);
        emit changed();
        return;
    }
    list(uri);
}

void ChooserBridge::stopListing()
{
    if (m_listTimer)
        m_listTimer->stop();
    if (m_session) {
        df_files_free(m_session);
        m_session = nullptr;
    }
    m_listingComplete = false;
}

void ChooserBridge::list(const QString &uri)
{
    stopListing();
    m_currentUri = uri;
    m_entries.clear();
    ++m_entriesRevision;
    m_selected.clear();
    m_current = -1;
    m_anchor = -1;
    m_error.clear();
    m_confirmReplace = false;
    m_replaceName.clear();
    m_replacePath.clear();
    m_loading = true;

    const QByteArray encoded = uri.toUtf8();
    m_session = df_files_begin(encoded.constData());
    if (!m_session) {
        m_error = tr("Cannot open %1.").arg(uri);
        m_loading = false;
        m_listingComplete = true;
        emit changed();
        return;
    }
    m_listTimer->start();
    emit changed();
}

void ChooserBridge::onListTick()
{
    if (!m_session) {
        m_listTimer->stop();
        return;
    }
    bool changedNow = false;
    for (int guard = 0; guard < 64; ++guard) {
        df_files_delta *delta = df_files_poll_delta(m_session, 0);
        if (!delta) {
            m_listTimer->stop();
            break;
        }
        const int status = delta->status;
        if (status == DF_FILES_STATUS_ERROR) {
            m_error = delta->error ? QString::fromUtf8(delta->error)
                                   : tr("Cannot open %1.").arg(m_currentUri);
            m_loading = false;
            m_listingComplete = true;
            df_files_delta_free(delta);
            m_listTimer->stop();
            changedNow = true;
            break;
        }
        if (status == DF_FILES_STATUS_TIMEOUT) {
            df_files_delta_free(delta);
            // A picker is short-lived and modal: while it is up, keep the
            // blocking poll armed so a folder change repaints it (T332). The
            // call sleeps on the watcher's channel, and it runs on this
            // bridge's own thread, so the UI never blocks on directory I/O.
            if (m_listingComplete && df_files_pending_ops(m_session) == 0
                && !df_files_watching(m_session))
                m_listTimer->stop();
            break;
        }
        applyDelta(delta);
        const bool done = status == DF_FILES_STATUS_DONE;
        df_files_delta_free(delta);
        changedNow = true;
        if (char *failure = df_files_take_error(m_session)) {
            m_error = QString::fromUtf8(failure);
            df_files_string_free(failure);
        }
        if (done) {
            m_loading = false;
            m_listingComplete = true;
            if (df_files_pending_ops(m_session) == 0 && !df_files_watching(m_session))
                m_listTimer->stop();
            break;
        }
    }
    if (changedNow)
        emit changed();
}

void ChooserBridge::applyDelta(const df_files_delta *delta)
{
    if (!delta)
        return;

    // Keep the selection across an insertion by remembering the selected URIs
    // and finding their new ranks, so a streaming listing does not clear a
    // choice the user already made.
    QStringList selectedUris;
    for (int row : m_selected) {
        const QVariantMap entry = entryAt(row);
        if (!entry.isEmpty())
            selectedUris.append(entry.value(QStringLiteral("uri")).toString());
    }
    QString currentUri;
    if (m_current >= 0) {
        const QVariantMap entry = entryAt(m_current);
        if (!entry.isEmpty())
            currentUri = entry.value(QStringLiteral("uri")).toString();
    }

    // T332: a changed delta replaces held rows in place; a removal asks for
    // a reset (the picker keeps its own selected URIs, so a removed row
    // simply drops out). A `REMOVED` delta without rows is answered by the
    // next poll's reset, because the picker always keeps its blocking poll
    // armed while it is visible.
    if (delta->kind == DF_FILES_DELTA_REMOVED && delta->row_count == 0) {
        // The gap is described by the next reset; this tick just repaints.
        return;
    }
    // A reset and an insert both carry whole rows, each at its final rank
    // (`total` counts them): rebuild from them rather than inserting into the
    // rows the picker already painted, which would duplicate a resident row.
    // Until T333 the insert path only appended, so a picker whose first delta
    // was `INSERT` (the watcher's incremental C ABI) never showed a row.
    if (delta->kind == DF_FILES_DELTA_RESET || delta->kind == DF_FILES_DELTA_INSERT) {
        m_entries.clear();
        for (uint32_t i = 0; i < delta->row_count; ++i)
            m_entries.append(rowFromNode(delta->rows[i].node));
    } else if (delta->kind == DF_FILES_DELTA_CHANGED) {
        for (uint32_t i = 0; i < delta->change_count; ++i) {
            const df_files_change &change = delta->changes[i];
            const int rank = int(change.rank);
            if (rank >= 0 && rank < m_entries.size())
                m_entries[rank] = rowFromNode(change.node);
        }
    } else {
        for (uint32_t i = 0; i < delta->row_count; ++i) {
            const df_files_row &row = delta->rows[i];
            const int rank = qBound(0, static_cast<int>(row.rank), m_entries.size());
            m_entries.insert(rank, rowFromNode(row.node));
        }
    }
    ++m_entriesRevision;

    if (!selectedUris.isEmpty() || !currentUri.isEmpty()) {
        m_selected.clear();
        m_current = -1;
        m_anchor = -1;
        for (int row = 0; row < m_entries.size(); ++row) {
            const QString uri =
                m_entries.at(row).toMap().value(QStringLiteral("uri")).toString();
            if (selectedUris.contains(uri))
                m_selected.append(row);
            if (!currentUri.isEmpty() && uri == currentUri) {
                m_current = row;
                m_anchor = row;
            }
        }
    }
}

void ChooserBridge::activate(int index)
{
    const QVariantMap entry = entryAt(index);
    if (entry.isEmpty())
        return;
    if (entry.value(QStringLiteral("directory")).toBool()) {
        browse(entry.value(QStringLiteral("uri")).toString());
        return;
    }
    if (saveFilesMode())
        return;
    setSelection(index, 0, true);
    if (saveMode()) {
        setSaveName(entry.value(QStringLiteral("name")).toString());
        accept();
        return;
    }
    if (m_directory) {
        m_error = tr("Select a folder.");
        emit changed();
        return;
    }
    accept();
}

void ChooserBridge::select(int index, int modifiers)
{
    setSelection(index, modifiers, false);
}

void ChooserBridge::selectAll()
{
    if (!m_active || !m_multiple || m_entries.isEmpty())
        return;
    m_selected.clear();
    for (int row = 0; row < m_entries.size(); ++row)
        m_selected.append(row);
    m_current = 0;
    m_anchor = 0;
    m_error.clear();
    emit changed();
}

void ChooserBridge::setSelection(int index, int modifiers, bool replaceAlways)
{
    if (index < 0 || index >= m_entries.size()) {
        if (m_selected.isEmpty() && m_current == -1)
            return;
        m_selected.clear();
        m_current = -1;
        m_anchor = -1;
        emit changed();
        return;
    }

    const bool toggle = !replaceAlways && m_multiple
                        && (modifiers & (Qt::ControlModifier | Qt::MetaModifier)) != 0;
    const bool extend = !replaceAlways && m_multiple
                        && (modifiers & Qt::ShiftModifier) != 0;

    if (toggle) {
        if (m_selected.contains(index))
            m_selected.removeAll(index);
        else
            m_selected.append(index);
        m_anchor = index;
    } else if (extend && m_anchor >= 0) {
        m_selected.clear();
        const int first = qMin(m_anchor, index);
        const int last = qMax(m_anchor, index);
        for (int row = first; row <= last; ++row)
            m_selected.append(row);
    } else {
        m_selected = { index };
        m_anchor = index;
    }
    m_current = index;
    m_error.clear();
    emit changed();
}

void ChooserBridge::goUp()
{
    const QString path = currentDirectoryPath();
    if (path.isEmpty())
        return;
    QDir dir(path);
    if (!dir.cdUp())
        return;
    list(fileUri(dir.absolutePath()));
}

void ChooserBridge::newFolder()
{
    if (!m_session || m_currentUri.isEmpty())
        return;
    const QByteArray parent = m_currentUri.toUtf8();
    if (df_files_begin_new_folder(m_session, parent.constData()) == 0) {
        m_error = tr("Cannot create a folder here.");
        emit changed();
        return;
    }
    m_error.clear();
    // The optimistic row is painted by the model; drain the reset delta so
    // the view shows it within a frame.
    m_listTimer->start();
}

void ChooserBridge::accept()
{
    if (!m_active)
        return;
    if (m_confirmReplace) {
        acceptReplace();
        return;
    }

    if (saveMode()) {
        const QString path = resolveSavePath();
        if (path.isEmpty())
            return;
        const QFileInfo info(path);
        if (info.exists() || info.isSymLink()) {
            if (info.isDir()) {
                m_error = tr("That name is already used by a folder.");
                emit changed();
                return;
            }
            requestReplace(path);
            return;
        }
        completeWith({QUrl::fromLocalFile(path).toString()});
        return;
    }

    if (saveFilesMode()) {
        // SaveFiles picks the destination folder; the portal appends the
        // caller's file names to the result.
        for (int row : selectedRows()) {
            const QVariantMap entry = entryAt(row);
            if (entry.value(QStringLiteral("directory")).toBool()) {
                completeWith({entry.value(QStringLiteral("uri")).toString()});
                return;
            }
        }
        if (!m_currentUri.isEmpty()) {
            completeWith({m_currentUri});
            return;
        }
        m_error = tr("Choose a folder.");
        emit changed();
        return;
    }

    QStringList files;
    QStringList folders;
    for (int row : selectedRows()) {
        const QVariantMap entry = entryAt(row);
        if (entry.isEmpty())
            continue;
        if (entry.value(QStringLiteral("directory")).toBool())
            folders.append(entry.value(QStringLiteral("uri")).toString());
        else
            files.append(entry.value(QStringLiteral("uri")).toString());
    }

    if (m_directory) {
        if (!folders.isEmpty()) {
            completeWith(folders);
            return;
        }
        if (!files.isEmpty()) {
            m_error = tr("Select a folder.");
            emit changed();
            return;
        }
        if (!m_currentUri.isEmpty()) {
            completeWith({m_currentUri});
            return;
        }
        m_error = tr("Choose a folder.");
        emit changed();
        return;
    }

    if (!files.isEmpty()) {
        completeWith(files);
        return;
    }
    // A selected folder is the Open button's "enter this folder", matching
    // the double-click gesture.
    if (!folders.isEmpty()) {
        browse(folders.first());
        return;
    }
    m_error = tr("Select a file.");
    emit changed();
}

void ChooserBridge::acceptReplace()
{
    if (!m_active || !m_confirmReplace)
        return;
    const QString path = m_replacePath;
    m_confirmReplace = false;
    m_replaceName.clear();
    m_replacePath.clear();
    completeWith({QUrl::fromLocalFile(path).toString()});
}

void ChooserBridge::dismissReplace()
{
    if (!m_confirmReplace)
        return;
    m_confirmReplace = false;
    m_replaceName.clear();
    m_replacePath.clear();
    emit changed();
}

void ChooserBridge::requestReplace(const QString &path)
{
    m_replacePath = path;
    m_replaceName = QFileInfo(path).fileName();
    m_confirmReplace = true;
    m_error.clear();
    emit changed();
}

void ChooserBridge::cancelCurrent()
{
    if (!m_active || !m_fromPortal || m_handle.isEmpty())
        return;
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    // Send even if the registration check is stale: a request only exists
    // because the portal is running, and dropping the answer would hang the
    // caller. The fixture (`DF_CHOOSER_FIXTURE`) has no portal request.
    QDBusMessage message = QDBusMessage::createMethodCall(
        kService, kPath, kInterface, QStringLiteral("CancelFileChooser"));
    message << handle;
    QDBusConnection::sessionBus().asyncCall(message);
#else
    Q_UNUSED(handle);
#endif
}

void ChooserBridge::cancel()
{
    if (!m_active)
        return;
    cancelCurrent();
    finish(false);
}

void ChooserBridge::queryPending()
{
#if defined(QT_DBUS_LIB)
    if (!m_serviceAvailable)
        return;
    QDBusMessage message = QDBusMessage::createMethodCall(
        kService, kPath, kInterface, QStringLiteral("PendingFileChoosers"));
    auto *watcher = new QDBusPendingCallWatcher(
        QDBusConnection::sessionBus().asyncCall(message), this);
    connect(watcher, &QDBusPendingCallWatcher::finished, this,
            &ChooserBridge::onPendingReply);
#endif
}

void ChooserBridge::onPendingReply(QDBusPendingCallWatcher *watcher)
{
#if defined(QT_DBUS_LIB)
    watcher->deleteLater();
    const QDBusMessage reply = watcher->reply();
    if (reply.type() == QDBusMessage::ErrorMessage || reply.arguments().isEmpty())
        return;
    const QStringList handles =
        Dragonfruit::PortalPresenter::pendingHandles(reply.arguments().first());
    for (const QString &handle : handles) {
        if (handle.isEmpty() || handle == m_handle)
            continue;
        QDBusMessage cancel = QDBusMessage::createMethodCall(
            kService, kPath, kInterface, QStringLiteral("CancelFileChooser"));
        cancel << handle;
        QDBusConnection::sessionBus().asyncCall(cancel);
    }
#else
    Q_UNUSED(watcher);
#endif
}

void ChooserBridge::completeWith(const QStringList &selections)
{
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    if (m_fromPortal) {
        QDBusMessage message = QDBusMessage::createMethodCall(
            kService, kPath, kInterface, QStringLiteral("CompleteFileChooser"));
        message << handle << selections;
        QDBusConnection::sessionBus().asyncCall(message);
    }
#else
    Q_UNUSED(selections);
#endif
    finish(true);
}

QVariantMap ChooserBridge::entryAt(int index) const
{
    if (index < 0 || index >= m_entries.size())
        return QVariantMap();
    return m_entries.at(index).toMap();
}

QList<int> ChooserBridge::selectedRows() const
{
    QList<int> rows = m_selected;
    std::sort(rows.begin(), rows.end());
    rows.erase(std::unique(rows.begin(), rows.end()), rows.end());
    return rows;
}

QString ChooserBridge::suggestedUri(const QVariantMap &options) const
{
    // `current_file`/`current_folder` cross the portal as D-Bus byte arrays
    // (`ay`) that carry a trailing NUL. Qt hands the shell that NUL verbatim,
    // so strip it before resolving the local path or the URI grows a `%00`
    // and the listing fails.
    const QString file = decodePathBytes(options.value(QStringLiteral("current_file")));
    if (!file.isEmpty()) {
        const QFileInfo info(file);
        return fileUri(info.isDir() ? info.absoluteFilePath() : info.absolutePath());
    }
    const QString folder = decodePathBytes(options.value(QStringLiteral("current_folder")));
    if (!folder.isEmpty())
        return fileUri(folder);
    return QString();
}

QString ChooserBridge::suggestedName(const QVariantMap &options) const
{
    const QString currentName = options.value(QStringLiteral("current_name")).toString();
    if (!currentName.isEmpty())
        return currentName;
    const QString file = decodePathBytes(options.value(QStringLiteral("current_file")));
    if (!file.isEmpty())
        return QFileInfo(file).fileName();
    return QString();
}

QStringList ChooserBridge::saveFilesFrom(const QVariantMap &options) const
{
    return decodeNameList(options.value(QStringLiteral("files")));
}

QString ChooserBridge::currentDirectoryPath() const
{
    if (m_currentUri.isEmpty())
        return QString();
    const QString local = QUrl(m_currentUri).toLocalFile();
    if (!local.isEmpty())
        return local;
    return m_currentUri;
}

QString ChooserBridge::resolveSavePath()
{
    QString name = m_saveName.trimmed();
    if (name.isEmpty()) {
        m_error = tr("Enter a file name.");
        emit changed();
        return QString();
    }
    if (name == QLatin1String(".") || name == QLatin1String("..")
        || name.endsWith(QLatin1Char('/'))) {
        m_error = tr("Enter a valid file name.");
        emit changed();
        return QString();
    }
    if (name.startsWith(QLatin1Char('~')))
        name = QDir::homePath() + name.mid(1);

    // A selected folder is the destination; otherwise the browsed folder is.
    QString directory = currentDirectoryPath();
    for (int row : selectedRows()) {
        const QVariantMap entry = entryAt(row);
        if (entry.value(QStringLiteral("directory")).toBool()) {
            directory = QUrl(entry.value(QStringLiteral("uri")).toString()).toLocalFile();
            break;
        }
    }

    QString path;
    if (QDir::isAbsolutePath(name))
        path = QDir::cleanPath(name);
    else
        path = QDir::cleanPath(directory + QLatin1Char('/') + name);
    if (path.isEmpty() || path == QLatin1String("/")) {
        m_error = tr("Enter a valid file name.");
        emit changed();
        return QString();
    }
    if (!QFileInfo(QFileInfo(path).absolutePath()).isDir()) {
        m_error = tr("The folder does not exist.");
        emit changed();
        return QString();
    }
    return path;
}
