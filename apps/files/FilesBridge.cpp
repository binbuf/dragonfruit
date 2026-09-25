// SPDX-License-Identifier: MIT
#include "FilesBridge.h"

#include <QDir>
#include <QFileInfo>
#include <QStandardPaths>
#include <QStorageInfo>
#include <QUrl>

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
                                  QStringLiteral("Data"), QStringLiteral("volume"),
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
                                  QStringLiteral("volume"), fileUri(root)));
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

bool FilesBridge::isBrowsable(const QString &uri) const
{
    const QUrl url(uri);
    return url.scheme() == QStringLiteral("file")
            || url.scheme() == QStringLiteral("trash");
}