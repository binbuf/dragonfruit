// SPDX-License-Identifier: GPL-3.0-or-later
#include "FilesBridge.h"

#include "FilesArguments.h"
#include "OpenHandler.h"

#include <QDir>
#include <QFileInfo>
#include <QLocale>
#include <QMimeDatabase>
#include <QProcess>
#include <QStandardPaths>
#include <QStorageInfo>
#include <QFile>
#include <QUrl>
#include <QCoreApplication>
#include <QDateTime>
#include <QJSValue>

namespace {

QVariantMap location(const QString &id, const QString &label, const QString &icon,
                     const QString &uri)
{
    QVariantMap map;
    map.insert(QStringLiteral("id"), id);
    map.insert(QStringLiteral("label"), label);
    map.insert(QStringLiteral("icon"), icon);
    map.insert(QStringLiteral("uri"), uri);
    return map;
}

QVariantMap crumb(const QString &label, const QString &uri)
{
    QVariantMap map;
    map.insert(QStringLiteral("label"), label);
    map.insert(QStringLiteral("uri"), uri);
    return map;
}

QString fileUri(const QString &path)
{
    return QUrl::fromLocalFile(path).toString();
}

// A block-device mount that belongs to the operating system, not to the user.
// The root volume is the Computer location and the rest are host plumbing
// (`/boot`, `/home`, ...), so the sidebar does not advertise them as volumes.
bool isSystemMount(const QString &root)
{
    static const QStringList kExact = {
        QStringLiteral("/"),       QStringLiteral("/boot"), QStringLiteral("/home"),
        QStringLiteral("/var"),    QStringLiteral("/usr"),  QStringLiteral("/etc"),
        QStringLiteral("/opt"),    QStringLiteral("/srv"),  QStringLiteral("/tmp"),
        QStringLiteral("/root"),
    };
    if (kExact.contains(root))
        return true;
    static const QStringList kPrefixes = {
        QStringLiteral("/boot/"), QStringLiteral("/home/"), QStringLiteral("/var/"),
        QStringLiteral("/usr/"),  QStringLiteral("/etc/"),  QStringLiteral("/opt/"),
        QStringLiteral("/srv/"),  QStringLiteral("/tmp/"),
    };
    for (const QString &prefix : kPrefixes) {
        if (root.startsWith(prefix))
            return true;
    }
    return false;
}

} // namespace

FilesBridge::FilesBridge(QObject *parent)
    : QObject(parent)
{
    m_fixture = qEnvironmentVariableIsSet("DF_FILES_FIXTURE");
    m_startUri = qEnvironmentVariable("DF_FILES_START_URI");
    m_revealUri = qEnvironmentVariable("DF_FILES_START_REVEAL");
    if (!m_revealUri.isEmpty() && QUrl(m_revealUri).scheme().isEmpty())
        m_revealUri = fileUri(m_revealUri);
    // A start argument that names a file opens its parent and reveals it
    // (T-10.6c), whether that argument came from the Dock's positional path or
    // from `DF_FILES_START_URI` directly.
    if (!m_startUri.isEmpty()) {
        const FilesOpenTarget target = filesOpenTarget(m_startUri);
        m_startUri = target.location;
        if (m_revealUri.isEmpty())
            m_revealUri = target.revealUri;
    }
    if (!m_revealUri.isEmpty()) {
        const QUrl revealUrl(m_revealUri);
        if (m_startUri.isEmpty() && revealUrl.scheme() == QStringLiteral("file")) {
            const QString path = QDir::cleanPath(revealUrl.toLocalFile());
            m_startUri = fileUri(QFileInfo(path).absolutePath());
        }
    }
    m_viewFixtureUri = qEnvironmentVariable("DF_FILES_VIEW_FIXTURE");
    if (!m_viewFixtureUri.isEmpty())
        m_viewFixtureUri = fileUri(m_viewFixtureUri);
    m_mutationFixtureUri = qEnvironmentVariable("DF_FILES_MUTATION_FIXTURE");
    if (!m_mutationFixtureUri.isEmpty())
        m_mutationFixtureUri = fileUri(m_mutationFixtureUri);
    m_startView = qEnvironmentVariable("DF_FILES_START_VIEW");
    if (m_startView != QStringLiteral("list"))
        m_startView = m_startView == QStringLiteral("icon") ? m_startView
                                                            : QString();
    m_captureDir = qEnvironmentVariable("DF_FILES_CAPTURE_DIR");
    m_startMenu = qEnvironmentVariable("DF_FILES_START_MENU");
    // `edit` opens the T331 Edit menu (a capture seam for the Undo rows).
    if (m_startMenu != QStringLiteral("item")
        && m_startMenu != QStringLiteral("background")
        && m_startMenu != QStringLiteral("edit")
        && m_startMenu != QStringLiteral("openWith"))
        m_startMenu.clear();
    m_startRename = qEnvironmentVariableIsSet("DF_FILES_START_RENAME");
    m_startSelect = qEnvironmentVariableIntValue("DF_FILES_START_SELECT");
    m_startTabs = qEnvironmentVariableIntValue("DF_FILES_CAPTURE_TABS");
    m_startOpenWithSelect = qEnvironmentVariable("DF_FILES_OPEN_WITH_SELECT");
    m_startEmptyTrash = qEnvironmentVariableIsSet("DF_FILES_START_EMPTY_TRASH");
    if (m_fixture) {
        // Deterministic locations for the headless QML tests; independent of
        // the host's home directory and mounts.
        m_homePath = QStringLiteral("/home/tester");
        m_userName = QStringLiteral("tester");
    } else {
        m_homePath = QStandardPaths::writableLocation(QStandardPaths::HomeLocation);
        if (m_homePath.isEmpty())
            m_homePath = QDir::homePath();
        m_homePath = QDir::cleanPath(m_homePath);
        m_userName = QFileInfo(m_homePath).fileName();
        if (m_userName.isEmpty())
            m_userName = QStringLiteral("Home");
    }
    m_homeUri = fileUri(m_homePath);
    buildLocations();
}

void FilesBridge::buildLocations()
{
    m_favorites.clear();
    // Favorites are the user's folders (Home is a Location, per
    // 09-files.md#sidebar). Recents is omitted: `recent://` needs the GVfs
    // backend, which this fallback does not have.
    if (m_fixture) {
        const struct {
            const char *id;
            const char *label;
            const char *icon;
            const char *suffix;
        } kFixture[] = {
            { "desktop", "Desktop", "displays", "/Desktop" },
            { "documents", "Documents", "documents", "/Documents" },
            { "downloads", "Downloads", "downloads", "/Downloads" },
        };
        for (const auto &fav : kFixture) {
            m_favorites.append(location(QString::fromLatin1(fav.id),
                                        QString::fromLatin1(fav.label),
                                        QString::fromLatin1(fav.icon),
                                        fileUri(m_homePath
                                                + QString::fromLatin1(fav.suffix))));
        }
    } else {
        const struct {
            const char *id;
            const char *icon;
            QStandardPaths::StandardLocation pathLocation;
        } kFavorites[] = {
            { "desktop", "displays", QStandardPaths::DesktopLocation },
            { "documents", "documents", QStandardPaths::DocumentsLocation },
            { "downloads", "downloads", QStandardPaths::DownloadLocation },
            { "pictures", "wallpaper", QStandardPaths::PicturesLocation },
            { "music", "music", QStandardPaths::MusicLocation },
            { "videos", "movies", QStandardPaths::MoviesLocation },
        };
        for (const auto &fav : kFavorites) {
            const QString path =
                    QDir::cleanPath(QStandardPaths::writableLocation(fav.pathLocation));
            // No dead rows: only a directory that is really there is listed.
            if (path.isEmpty() || !QFileInfo(path).isDir())
                continue;
            m_favorites.append(location(QString::fromLatin1(fav.id),
                                        QFileInfo(path).fileName(),
                                        QString::fromLatin1(fav.icon), fileUri(path)));
        }
    }

    m_volumes.clear();
    if (m_fixture) {
        m_volumes.append(location(QStringLiteral("volume-/media/Data"),
                                  QStringLiteral("Data"), QStringLiteral("drive-harddisk"),
                                  QStringLiteral("file:///media/Data")));
        return;
    }
    const QList<QStorageInfo> mounted = QStorageInfo::mountedVolumes();
    for (const QStorageInfo &volume : mounted) {
        if (!volume.isValid() || !volume.isReady())
            continue;
        const QString root = QDir::cleanPath(volume.rootPath());
        if (root.isEmpty() || root == QStringLiteral("/") || isSystemMount(root))
            continue;
        if (!QString::fromLocal8Bit(volume.device())
                     .startsWith(QStringLiteral("/dev/")))
            continue;
        QString label = volume.displayName();
        if (label.isEmpty())
            label = QFileInfo(root).fileName();
        if (label.isEmpty())
            label = root;
        m_volumes.append(location(QStringLiteral("volume-") + root, label,
                                  QStringLiteral("drive-harddisk"), fileUri(root)));
    }
}

QVariantList FilesBridge::breadcrumb(const QString &uri) const
{
    QVariantList out;
    const QUrl url(uri);
    if (url.scheme() == QStringLiteral("file")) {
        const QString path = QDir::cleanPath(url.toLocalFile());
        out.append(crumb(QStringLiteral("Computer"), computerUri()));
        if (path.isEmpty() || path == QStringLiteral("/"))
            return out;
        if (path == m_homePath || path.startsWith(m_homePath + QLatin1Char('/'))) {
            out.append(crumb(m_userName, m_homeUri));
            QString accumulated = m_homePath;
            const QStringList parts =
                    path.mid(m_homePath.length()).split(QLatin1Char('/'), Qt::SkipEmptyParts);
            for (const QString &part : parts) {
                accumulated += QLatin1Char('/') + part;
                out.append(crumb(part, fileUri(accumulated)));
            }
        } else {
            QString accumulated;
            const QStringList parts = path.split(QLatin1Char('/'), Qt::SkipEmptyParts);
            for (const QString &part : parts) {
                accumulated += QLatin1Char('/') + part;
                out.append(crumb(part, fileUri(accumulated)));
            }
        }
    } else if (url.scheme() == QStringLiteral("trash")) {
        out.append(crumb(QStringLiteral("Trash"), trashUri()));
    } else {
        out.append(crumb(displayName(uri), uri));
    }
    return out;
}

QString FilesBridge::displayName(const QString &uri) const
{
    const QUrl url(uri);
    if (url.scheme() == QStringLiteral("file")) {
        const QString path = QDir::cleanPath(url.toLocalFile());
        if (path.isEmpty() || path == QStringLiteral("/"))
            return QStringLiteral("Computer");
        if (path == m_homePath)
            return m_userName;
        const QString name = QFileInfo(path).fileName();
        return name.isEmpty() ? path : name;
    }
    if (url.scheme() == QStringLiteral("trash"))
        return QStringLiteral("Trash");
    return uri;
}

bool FilesBridge::writeFixture(const QString &uri, const QString &contents) const
{
    // Only inside the test's mutation tree, never the user's files.
    if (m_mutationFixtureUri.isEmpty()
        || !uri.startsWith(m_mutationFixtureUri + QLatin1Char('/'))) {
        return false;
    }
    const QUrl url(uri);
    if (!url.isLocalFile())
        return false;
    QFile file(url.toLocalFile());
    if (!file.open(QIODevice::WriteOnly))
        return false;
    return file.write(QByteArray(contents.toUtf8())) >= 0;
}

bool FilesBridge::externalChange(const QString &change, const QString &uri,
                                 const QString &toUri, const QString &contents) const
{
    // Only inside the test's mutation tree, never the user's files.
    if (m_mutationFixtureUri.isEmpty()
        || !uri.startsWith(m_mutationFixtureUri + QLatin1Char('/'))) {
        return false;
    }
    const QUrl url(uri);
    if (!url.isLocalFile())
        return false;
    const QString path = url.toLocalFile();
    if (change == QStringLiteral("create")) {
        QFile file(path);
        if (!file.open(QIODevice::WriteOnly))
            return false;
        return file.write(contents.toUtf8()) >= 0;
    }
    if (change == QStringLiteral("delete"))
        return QFile::remove(path);
    if (change == QStringLiteral("rename")) {
        if (!toUri.startsWith(m_mutationFixtureUri + QLatin1Char('/')))
            return false;
        const QUrl target(toUri);
        return target.isLocalFile() && QFile::rename(path, target.toLocalFile());
    }
    return false;
}

bool FilesBridge::isBrowsable(const QString &uri) const
{
    const QUrl url(uri);
    return url.scheme() == QStringLiteral("file")
            || url.scheme() == QStringLiteral("trash");
}

QVariantMap FilesBridge::resolveOpenTarget(const QString &argument) const
{
    const FilesOpenTarget target = filesOpenTarget(argument);
    QVariantMap out;
    out.insert(QStringLiteral("location"), target.location);
    out.insert(QStringLiteral("revealUri"), target.revealUri);
    return out;
}

bool FilesBridge::open(const QString &uri) const
{
    return filesOpen(uri);
}

QString FilesBridge::mimeType(const QString &uri) const
{
    const QUrl url(uri);
    if (!url.isLocalFile())
        return {};
    const QFileInfo info(url.toLocalFile());
    if (!info.isFile())
        return {};
    if (m_fixture) {
        if (info.suffix().compare(QStringLiteral("txt"), Qt::CaseInsensitive) == 0)
            return QStringLiteral("text/plain");
        if (info.suffix().compare(QStringLiteral("png"), Qt::CaseInsensitive) == 0)
            return QStringLiteral("image/png");
        return QStringLiteral("application/octet-stream");
    }
    const QMimeType mime = QMimeDatabase().mimeTypeForFile(info);
    return mime.isValid() ? mime.name() : QString();
}

QVariantList FilesBridge::handlers(const QString &uri) const
{    const QUrl url(uri);
    if (!url.isLocalFile())
        return {};
    const QFileInfo info(url.toLocalFile());
    if (!info.isFile())
        return {};

    if (m_handlerFixtureSet)
        return m_handlerFixture;

    const QString key = QDir::cleanPath(info.absoluteFilePath());
    const auto cached = m_handlerCache.constFind(key);
    if (cached != m_handlerCache.constEnd())
        return *cached;

    QVariantList rows;
    if (m_fixture) {
        // Deterministic rows for the headless picker tests/captures. A
        // `.unknown` file exercises the absent-handler state.
        if (!info.fileName().endsWith(QStringLiteral(".unknown"))) {
            const bool text = info.suffix().compare(QStringLiteral("txt"),
                                                    Qt::CaseInsensitive) == 0;
            const struct {
                const char *id;
                const char *name;
            } kFixture[] = {
                { "df-test-text.desktop", "Text Editor" },
                { "df-test-code.desktop", "Code Editor" },
            };
            const struct {
                const char *id;
                const char *name;
            } kImage = { "df-test-image.desktop", "Image Viewer" };
            if (text) {
                for (int i = 0; i < 2; ++i) {
                    QVariantMap row;
                    row.insert(QStringLiteral("id"),
                               QString::fromLatin1(kFixture[i].id));
                    row.insert(QStringLiteral("name"),
                               QString::fromLatin1(kFixture[i].name));
                    row.insert(QStringLiteral("path"), QString());
                    row.insert(QStringLiteral("isDefault"),
                               m_fixtureDefaultId.isEmpty()
                                   ? i == 0
                                   : m_fixtureDefaultId
                                             == QString::fromLatin1(kFixture[i].id));
                    rows.append(row);
                }
            } else if (info.suffix().compare(QStringLiteral("png"),
                                            Qt::CaseInsensitive) == 0) {
                QVariantMap row;
                row.insert(QStringLiteral("id"), QString::fromLatin1(kImage.id));
                row.insert(QStringLiteral("name"),
                           QString::fromLatin1(kImage.name));
                row.insert(QStringLiteral("path"), QString());
                row.insert(QStringLiteral("isDefault"), true);
                rows.append(row);
            }
        }
        m_handlerCache.insert(key, rows);
        return rows;
    }

    const QMimeType mime = QMimeDatabase().mimeTypeForFile(info);
    if (mime.isValid()) {
        const QList<FilesHandlerEntry> entries = filesHandlersForMime(mime.name());
        for (const FilesHandlerEntry &entry : entries) {
            QVariantMap row;
            row.insert(QStringLiteral("id"), entry.id);
            row.insert(QStringLiteral("name"), entry.name);
            row.insert(QStringLiteral("path"), entry.path);
            row.insert(QStringLiteral("isDefault"), entry.isDefault);
            rows.append(row);
        }
    }
    m_handlerCache.insert(key, rows);
    return rows;
}

bool FilesBridge::openWith(const QString &uri, const QString &desktopId)
{
    if (uri.isEmpty() || desktopId.isEmpty())
        return false;
    if (m_openWithCallable.isCallable()) {
        m_openWithRequests.append(desktopId + QLatin1Char('\t') + uri);
        const QJSValue result =
                m_openWithCallable.call({QJSValue(uri), QJSValue(desktopId)});
        return !result.isBool() || result.toBool();
    }
    if (m_fixture)
        return true; // fixture rows have no real desktop file to launch
    return filesOpenWith(uri, desktopId);
}

bool FilesBridge::setDefaultHandler(const QString &uri, const QString &desktopId)
{
    const QUrl url(uri);
    if (!url.isLocalFile() || desktopId.isEmpty())
        return false;
    const QFileInfo info(url.toLocalFile());
    if (!info.isFile())
        return false;
    if (m_fixture) {
        m_defaultHandlerRequests.append(desktopId);
        m_fixtureDefaultId = desktopId;
        m_handlerCache.clear();
        return true;
    }
    const QMimeType mime = QMimeDatabase().mimeTypeForFile(info);
    if (!mime.isValid())
        return false;
    const bool ok = filesSetDefaultDesktopId(mime.name(), desktopId);
    if (ok)
        m_handlerCache.clear();
    return ok;
}

void FilesBridge::setHandlerFixture(const QVariantList &handlers)
{
    m_handlerFixture = handlers;
    m_handlerFixtureSet = true;
    m_handlerCache.clear();
}

void FilesBridge::clearHandlerFixture()
{
    m_handlerFixture.clear();
    m_handlerFixtureSet = false;
    m_fixtureDefaultId.clear();
    m_handlerCache.clear();
}

void FilesBridge::setOpenWithCallable(const QJSValue &callable)
{
    m_openWithCallable = callable.isCallable() ? callable : QJSValue();
    m_openWithRequests.clear();
}

QVariantMap FilesBridge::volumeInfo(const QString &uri) const
{
    QVariantMap info;
    info.insert(QStringLiteral("available"), false);
    info.insert(QStringLiteral("freeBytes"), QVariant::fromValue<qulonglong>(0));
    info.insert(QStringLiteral("freeText"), QString());

    const QUrl url(uri);
    QString path;
    if (url.isLocalFile())
        path = url.toLocalFile();
    else if (url.scheme() == QStringLiteral("trash"))
        path = m_homePath;
    else if (url.scheme() == QStringLiteral("file"))
        path = QStringLiteral("/");
    if (path.isEmpty())
        return info;

    if (m_fixture) {
        // Deterministic headless value; the live path reads QStorageInfo.
        info.insert(QStringLiteral("available"), true);
        info.insert(QStringLiteral("freeBytes"),
                    QVariant::fromValue<qulonglong>(274877906944ULL));
        info.insert(QStringLiteral("freeText"),
                    QStringLiteral("256 GB available"));
        return info;
    }

    const QStorageInfo storage(path);
    if (!storage.isValid() || !storage.isReady())
        return info;
    const qint64 free = storage.bytesAvailable();
    if (free < 0)
        return info;
    info.insert(QStringLiteral("available"), true);
    info.insert(QStringLiteral("freeBytes"),
                QVariant::fromValue<qulonglong>(quint64(free)));
    info.insert(QStringLiteral("freeText"),
                QLocale().formattedDataSize(free, 1,
                                            QLocale::DataSizeTraditionalFormat)
                    + QLatin1Char(' ') + tr("available"));
    return info;
}

QString FilesBridge::defaultHandler(const QString &uri) const
{
    const QUrl url(uri);
    if (!url.isLocalFile())
        return {};
    const QFileInfo info(url.toLocalFile());
    if (info.isDir())
        return tr("Folder");
    if (!info.isFile())
        return {};

    const QVariantList rows = handlers(uri);
    for (const QVariant &value : rows) {
        const QVariantMap row = value.toMap();
        if (row.value(QStringLiteral("isDefault")).toBool()) {
            const QString name = row.value(QStringLiteral("name")).toString();
            if (!name.isEmpty())
                return name;
        }
    }
    if (!rows.isEmpty()) {
        const QString name = rows.first().toMap()
                                     .value(QStringLiteral("name"))
                                     .toString();
        if (!name.isEmpty())
            return name;
    }
    return tr("No default application");
}

bool FilesBridge::newWindow(const QString &uri)
{
    const QString location = uri;
    if (!location.isEmpty() && !isBrowsable(location))
        return false;
    if (m_newWindowCallable.isCallable()) {
        m_newWindowRequests.append(location);
        const QJSValue result = m_newWindowCallable.call({QJSValue(location)});
        return !result.isBool() || result.toBool();
    }

    // A second process, like the desktop's folder launch: the browser and the
    // new window never share a crash domain. The first instance keeps the
    // `org.dragonfruit.Files1` name; the second one may lose it, which is the
    // documented single-instance trade-off (T333, 09-files.md).
    QProcess window;
    window.setProcessEnvironment(filesDesktopChildEnvironment());
    window.setProgram(QCoreApplication::applicationFilePath());
    if (!location.isEmpty())
        window.setArguments({location});
    return window.startDetached();
}

void FilesBridge::setNewWindowCallable(const QJSValue &callable)
{
    m_newWindowCallable = callable.isCallable() ? callable : QJSValue();
    // Each install starts a fresh recording, so one test's request never
    // counts against the next.
    m_newWindowRequests.clear();
}

bool FilesBridge::hasNewWindowCallable() const
{
    return m_newWindowCallable.isCallable();
}

namespace {
// The token a `--activation-handoff` launch carries (one per request, like the
// desktop's launch token): time, pid and a counter, so two requests in the same
// millisecond still differ.
static QString filesLaunchToken()
{
    static quint64 counter = 0;
    return QStringLiteral("%1-%2-%3")
            .arg(QDateTime::currentMSecsSinceEpoch())
            .arg(QCoreApplication::applicationPid())
            .arg(++counter);
}

} // namespace

QVariantMap FilesBridge::menuLaunchCommands(const QString &location) const
{
    QVariantMap commands;
    QVariantList newWindow;
    newWindow << QStringLiteral("--new-window");
    if (!location.isEmpty()) {
        newWindow << QStringLiteral("--location=") + location;
    }
    commands.insert(QStringLiteral("newWindow"), newWindow);
    QVariantList newTab;
    newTab << QStringLiteral("--activation-handoff=") + filesLaunchToken();
    if (!location.isEmpty())
        newTab << QStringLiteral("--location=") + location;
    commands.insert(QStringLiteral("newTab"), newTab);
    return commands;
}
