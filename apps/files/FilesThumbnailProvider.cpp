// SPDX-License-Identifier: MIT
#include "FilesThumbnailProvider.h"

#include <QCryptographicHash>
#include <QDateTime>
#include <QDir>
#include <QFileInfo>
#include <QImageReader>
#include <QSaveFile>
#include <QStandardPaths>
#include <QUrl>

namespace {

// The freedesktop thumbnail spec's "normal" size. The icon view asks for a
// 48px tile (96px at 2x) and the list view for a 16px row, so one cached
// normal thumbnail serves both without a second cache class.
constexpr int kThumbnailSize = 128;

QByteArray decodeId(const QString &id)
{
    return QByteArray::fromBase64(id.toLatin1(),
                                  QByteArray::Base64UrlEncoding
                                          | QByteArray::OmitTrailingEquals);
}

QString canonicalUri(const QString &uri)
{
    const QUrl url(uri);
    if (!url.isLocalFile())
        return {};
    const QFileInfo info(url.toLocalFile());
    if (!info.isFile())
        return {};
    // The spec keys the cache by the file URI. Canonicalizing makes
    // `file:///a/../b` and `file:///b` share one entry, and FullyEncoded
    // matches the byte form other file managers hash.
    return QUrl::fromLocalFile(info.canonicalFilePath())
            .toString(QUrl::FullyEncoded);
}

QString cacheDirectory()
{
    QString base =
            QStandardPaths::writableLocation(QStandardPaths::GenericCacheLocation);
    if (base.isEmpty())
        base = QDir::homePath() + QStringLiteral("/.cache");
    return base + QStringLiteral("/thumbnails/normal");
}

} // namespace

FilesThumbnailProvider::FilesThumbnailProvider()
    : QQuickImageProvider(QQmlImageProviderBase::Image,
                          QQmlImageProviderBase::ForceAsynchronousImageLoading)
{
}

QString FilesThumbnailProvider::sourceFor(const QString &uri)
{
    const QUrl url(uri);
    if (!url.isLocalFile())
        return {};
    const QString encoded =
            QString::fromLatin1(url.toString(QUrl::FullyEncoded).toUtf8().toBase64(
                    QByteArray::Base64UrlEncoding | QByteArray::OmitTrailingEquals));
    return QStringLiteral("image://files-thumbnail/") + encoded;
}

QString FilesThumbnailProvider::cacheFileFor(const QString &uri)
{
    const QString canonical = canonicalUri(uri);
    if (canonical.isEmpty())
        return {};
    const QByteArray digest =
            QCryptographicHash::hash(canonical.toUtf8(), QCryptographicHash::Md5)
                    .toHex();
    return cacheDirectory() + QLatin1Char('/') + QString::fromLatin1(digest)
            + QStringLiteral(".png");
}

QImage FilesThumbnailProvider::requestImage(const QString &id, QSize *size,
                                            const QSize &requestedSize)
{
    Q_UNUSED(requestedSize)

    const QString uri = QString::fromUtf8(decodeId(id));
    const QString canonical = canonicalUri(uri);
    if (canonical.isEmpty())
        return {};

    const QFileInfo source(QUrl(canonical).toLocalFile());
    const QString cachePath = cacheFileFor(uri);
    const QFileInfo cached(cachePath);
    // A cache entry is valid while it is at least as new as the source. The
    // spec stores the source mtime in the PNG, but comparing files avoids
    // decoding a stale thumbnail just to decide it is stale.
    if (cached.isFile() && cached.lastModified() >= source.lastModified()) {
        QImage image(cachePath);
        if (!image.isNull()) {
            if (size)
                *size = image.size();
            return image;
        }
    }

    QImageReader reader(source.canonicalFilePath());
    reader.setAutoTransform(true);
    const QSize original = reader.size();
    if (!original.isEmpty()) {
        QSize target = original;
        if (target.width() > kThumbnailSize || target.height() > kThumbnailSize) {
            target = original.scaled(QSize(kThumbnailSize, kThumbnailSize),
                                     Qt::KeepAspectRatio);
            reader.setScaledSize(target);
        }
    }
    QImage image = reader.read();
    if (image.isNull())
        return {};
    if (image.width() > kThumbnailSize || image.height() > kThumbnailSize) {
        image = image.scaled(QSize(kThumbnailSize, kThumbnailSize),
                             Qt::KeepAspectRatio, Qt::SmoothTransformation);
    }

    // Best-effort cache write: a read-only or absent cache directory never
    // stops the preview from painting.
    image.setText(QStringLiteral("Thumb::URI"), canonical);
    image.setText(QStringLiteral("Thumb::MTime"),
                  QString::number(source.lastModified().toSecsSinceEpoch()));
    if (QDir().mkpath(cacheDirectory())) {
        QSaveFile file(cachePath);
        if (file.open(QIODevice::WriteOnly)) {
            image.save(&file, "PNG");
            file.commit();
        }
    }

    if (size)
        *size = image.size();
    return image;
}
