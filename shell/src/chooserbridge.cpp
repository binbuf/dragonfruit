// SPDX-License-Identifier: MIT
#include "chooserbridge.h"

#include "files_core_list.h"

#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QUrl>
#include <QVariantMap>

#if defined(QT_DBUS_LIB)
#include <QDBusConnection>
#include <QDBusConnectionInterface>
#include <QDBusInterface>
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

// Turn one files-core listing event into the picker's row maps.
QVariantList rowsFromEvent(const df_files_event *event)
{
    QVariantList rows;
    if (!event || !event->nodes)
        return rows;
    rows.reserve(static_cast<int>(event->count));
    for (uint32_t i = 0; i < event->count; ++i) {
        const df_files_node &node = event->nodes[i];
        const bool directory = node.kind == DF_NODE_DIRECTORY;
        QVariantMap row;
        row.insert(QStringLiteral("name"),
                   node.name ? QString::fromUtf8(node.name) : QString());
        row.insert(QStringLiteral("uri"), node.uri ? QString::fromUtf8(node.uri) : QString());
        row.insert(QStringLiteral("directory"), directory);
        const qint64 size = node.has_size ? static_cast<qint64>(node.size) : -1;
        row.insert(QStringLiteral("size"), size);
        row.insert(QStringLiteral("detail"), directory ? QString() : formatSize(size));
        rows.append(row);
    }
    return rows;
}

} // namespace

ChooserBridge::ChooserBridge(QObject *parent)
    : QObject(parent)
{
}

ChooserBridge::~ChooserBridge() = default;

QString ChooserBridge::acceptLabel() const
{
    if (!m_acceptLabel.isEmpty())
        return m_acceptLabel;
    if (saveMode())
        return tr("Save");
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

bool ChooserBridge::saveMode() const
{
    return m_kind == QLatin1String("save") || m_kind == QLatin1String("save-files");
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

void ChooserBridge::setSelectedIndex(int index)
{
    if (index < -1 || index >= m_entries.size())
        index = -1;
    if (index == m_selectedIndex)
        return;
    m_selectedIndex = index;
    emit changed();
}

void ChooserBridge::setSaveName(const QString &name)
{
    if (name == m_saveName)
        return;
    m_saveName = name;
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
    m_serviceAvailable = iface && iface->isServiceRegistered(kService);
#else
    m_serviceAvailable = false;
#endif
}

void ChooserBridge::onFileChooserOpened(const QString &handle, const QString &kind,
                                        const QString &appId, const QString &parentWindow,
                                        const QString &title, const QVariantMap &options)
{
    Q_UNUSED(parentWindow);
    const bool multiple = options.value(QStringLiteral("multiple")).toBool();
    const bool directory = options.value(QStringLiteral("directory")).toBool();
    QString name = QString::fromLocal8Bit(options.value(QStringLiteral("current_name")).toByteArray());
    begin(handle, kind, title, QString(), multiple, directory, suggestedUri(options));
    m_appId = appId;
    if (!name.isEmpty())
        setSaveName(name);
}

void ChooserBridge::begin(const QString &handle, const QString &kind, const QString &title,
                          const QString &acceptLabel, bool multiple, bool directory,
                          const QString &startUri)
{
    reset();
    m_active = true;
    m_handle = handle;
    m_kind = kind.isEmpty() ? QStringLiteral("open") : kind;
    m_title = title;
    m_acceptLabel = acceptLabel;
    m_multiple = multiple;
    m_directory = directory;
    QString start = startUri;
    if (start.isEmpty())
        start = QUrl::fromLocalFile(QDir::homePath()).toString();
    emit started();
    list(start);
    emit changed();
}

void ChooserBridge::reset()
{
    if (!m_active && m_handle.isEmpty())
        return;
    m_active = false;
    m_handle.clear();
    m_kind.clear();
    m_title.clear();
    m_acceptLabel.clear();
    m_appId.clear();
    m_multiple = false;
    m_directory = false;
    m_currentUri.clear();
    m_entries.clear();
    m_selectedIndex = -1;
    m_saveName.clear();
    m_error.clear();
    emit changed();
}

void ChooserBridge::refresh()
{
    if (m_currentUri.isEmpty())
        return;
    list(m_currentUri);
}

void ChooserBridge::browse(const QString &uri)
{
    if (uri.isEmpty())
        return;
    list(uri);
}

void ChooserBridge::list(const QString &uri)
{
    m_error.clear();
    const QByteArray encoded = uri.toUtf8();
    void *session = df_files_begin(encoded.constData());
    if (!session) {
        m_error = tr("Cannot open %1.").arg(uri);
        m_entries.clear();
        m_selectedIndex = -1;
        emit changed();
        return;
    }

    QVariantList rows;
    // Poll until the listing is done; a BATCH carries the model's current
    // ordered snapshot, so the last one before DONE is the final order. A
    // terminal DONE carries no nodes, so the last BATCH is kept.
    for (int guard = 0; guard < 64; ++guard) {
        df_files_event *event = df_files_poll(session, 500);
        if (!event)
            break;
        const int status = event->status;
        if (status == DF_FILES_STATUS_ERROR) {
            m_error = event->error ? QString::fromUtf8(event->error) : tr("Listing failed.");
            df_files_event_free(event);
            break;
        }
        if (status == DF_FILES_STATUS_BATCH) {
            rows = rowsFromEvent(event);
            df_files_event_free(event);
            continue;
        }
        df_files_event_free(event);
        if (status == DF_FILES_STATUS_DONE)
            break;
        // TIMEOUT: keep waiting for the worker.
    }
    df_files_free(session);

    m_currentUri = uri;
    m_entries = rows;
    m_selectedIndex = -1;
    emit changed();
}

void ChooserBridge::activate(int index)
{
    const QVariantMap entry = entryAt(index);
    if (entry.isEmpty())
        return;
    if (entry.value(QStringLiteral("directory")).toBool()) {
        list(entry.value(QStringLiteral("uri")).toString());
        return;
    }
    setSelectedIndex(index);
    if (saveMode()) {
        setSaveName(entry.value(QStringLiteral("name")).toString());
        return;
    }
    // A file activation in open mode is an accept.
    accept();
}

void ChooserBridge::select(int index)
{
    setSelectedIndex(index);
}

void ChooserBridge::goUp()
{
    const QString path = currentDirectoryPath();
    if (path.isEmpty())
        return;
    QDir dir(path);
    if (!dir.cdUp())
        return;
    list(QUrl::fromLocalFile(dir.absolutePath()).toString());
}

void ChooserBridge::accept()
{
    if (!m_active)
        return;
    if (saveMode()) {
        const QString name = m_saveName.trimmed();
        if (name.isEmpty()) {
            m_error = tr("Enter a file name.");
            emit changed();
            return;
        }
        const QString dir = currentDirectoryPath();
        if (dir.isEmpty()) {
            m_error = tr("Choose a folder.");
            emit changed();
            return;
        }
        const QString path = dir.endsWith(QLatin1Char('/')) ? dir + name : dir + QLatin1Char('/') + name;
        completeWith({path});
        return;
    }

    const QVariantMap entry = entryAt(m_selectedIndex);
    if (m_directory) {
        if (!entry.isEmpty() && entry.value(QStringLiteral("directory")).toBool()) {
            completeWith({entry.value(QStringLiteral("uri")).toString()});
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

    if (entry.isEmpty() || entry.value(QStringLiteral("directory")).toBool()) {
        m_error = tr("Select a file.");
        emit changed();
        return;
    }
    completeWith({entry.value(QStringLiteral("uri")).toString()});
}

void ChooserBridge::cancel()
{
    if (!m_active)
        return;
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    if (m_serviceAvailable) {
        QDBusInterface iface(kService, kPath, kInterface, QDBusConnection::sessionBus());
        if (iface.isValid())
            iface.asyncCall(QStringLiteral("CancelFileChooser"), handle);
    }
#endif
    reset();
    emit finished(false);
}

void ChooserBridge::completeWith(const QStringList &selections)
{
    const QString handle = m_handle;
#if defined(QT_DBUS_LIB)
    if (m_serviceAvailable) {
        QDBusInterface iface(kService, kPath, kInterface, QDBusConnection::sessionBus());
        if (iface.isValid())
            iface.asyncCall(QStringLiteral("CompleteFileChooser"), handle, selections);
    }
#else
    Q_UNUSED(selections);
#endif
    reset();
    emit finished(true);
}

QVariantMap ChooserBridge::entryAt(int index) const
{
    if (index < 0 || index >= m_entries.size())
        return QVariantMap();
    return m_entries.at(index).toMap();
}

QString ChooserBridge::suggestedUri(const QVariantMap &options) const
{
    const QByteArray fileBytes =
        options.value(QStringLiteral("current_file")).toByteArray();
    if (!fileBytes.isEmpty()) {
        const QString path = QFile::decodeName(fileBytes);
        return QUrl::fromLocalFile(QFileInfo(path).absolutePath()).toString();
    }
    const QByteArray folderBytes =
        options.value(QStringLiteral("current_folder")).toByteArray();
    if (!folderBytes.isEmpty())
        return QUrl::fromLocalFile(QFile::decodeName(folderBytes)).toString();
    return QString();
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