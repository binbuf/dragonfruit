// SPDX-License-Identifier: MIT
#include "FilesThumbnailProvider.h"

#include <QCryptographicHash>
#include <QDateTime>
#include <QDir>
#include <QFileInfo>
#include <QImageReader>
#include <QProcess>
#include <QSaveFile>
#include <QStandardPaths>
#include <QTemporaryDir>
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

bool isImageSuffix(const QString &suffix)
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

bool isVideoSuffix(const QString &suffix)
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

QImage renderPdf(const QString &path)
{
    const QString pdftoppm =
            QStandardPaths::findExecutable(QStringLiteral("pdftoppm"));
    if (pdftoppm.isEmpty())
        return {};

    QTemporaryDir directory;
    if (!directory.isValid())
        return {};
    const QString prefix = directory.path() + QStringLiteral("/page");
    QProcess process;
    process.setProgram(pdftoppm);
    process.setArguments({QStringLiteral("-f"), QStringLiteral("1"),
                          QStringLiteral("-l"), QStringLiteral("1"),
                          QStringLiteral("-singlefile"),
                          QStringLiteral("-scale-to"), QString::number(kThumbnailSize),
                          QStringLiteral("-png"), path, prefix});
    process.start();
    if (!process.waitForFinished(15000)) {
        process.kill();
        process.waitForFinished(200);
        return {};
    }
    if (process.exitStatus() != QProcess::NormalExit || process.exitCode() != 0)
        return {};
    return QImage(prefix + QStringLiteral(".png"));
}

QImage renderVideo(const QString &path)
{
    QTemporaryDir directory;
    if (!directory.isValid())
        return {};
    const QString output = directory.path() + QStringLiteral("/thumb.png");

    QString program =
            QStandardPaths::findExecutable(QStringLiteral("ffmpegthumbnailer"));
    QStringList arguments;
    if (!program.isEmpty()) {
        arguments = {QStringLiteral("-i"), path, QStringLiteral("-o"), output,
                     QStringLiteral("-s"), QString::number(kThumbnailSize),
                     QStringLiteral("-f")};
    } else {
        program = QStandardPaths::findExecutable(QStringLiteral("ffmpeg"));
        if (program.isEmpty())
            return {};
        arguments = {QStringLiteral("-y"), QStringLiteral("-ss"), QStringLiteral("1"),
                     QStringLiteral("-i"), path, QStringLiteral("-frames:v"),
                     QStringLiteral("1"), QStringLiteral("-vf"),
                     QStringLiteral("scale=128:128:force_original_aspect_ratio=decrease"),
                     QStringLiteral("-f"), QStringLiteral("image2"), output};
    }

    auto run = [&](const QStringList &args) {
        QProcess process;
        process.setProgram(program);
        process.setArguments(args);
        process.start();
        if (!process.waitForFinished(15000)) {
            process.kill();
            process.waitForFinished(200);
            return false;
        }
        return process.exitStatus() == QProcess::NormalExit
                && process.exitCode() == 0;
    };

    if (!run(arguments)) {
        // A clip shorter than one second produces no frame at the 1s seek.
        if (program.endsWith(QStringLiteral("ffmpeg"))
            && arguments.size() > 2
            && arguments[2] == QStringLiteral("1")) {
            QStringList retry = arguments;
            retry[2] = QStringLiteral("0");
            if (!run(retry))
                return {};
        } else {
            return {};
        }
    }
    return QImage(output);
}

// The pinned Qt build ships only the core image plugins (PNG/JPEG/GIF/ICO/
// SVG). If a WebP/TIFF/AVIF/HEIC image fails to decode, use a local converter
// when one is installed instead of dropping straight to the type glyph.
QImage renderImageFallback(const QString &path)
{
    QTemporaryDir directory;
    if (!directory.isValid())
        return {};
    const QString output = directory.path() + QStringLiteral("/thumb.png");

    auto run = [&](const QString &program, const QStringList &arguments) {
        QProcess process;
        process.setProgram(program);
        process.setArguments(arguments);
        process.start();
        if (!process.waitForFinished(15000)) {
            process.kill();
            process.waitForFinished(200);
            return false;
        }
        return process.exitStatus() == QProcess::NormalExit
                && process.exitCode() == 0;
    };

    const QString ffmpeg = QStandardPaths::findExecutable(QStringLiteral("ffmpeg"));
    if (!ffmpeg.isEmpty()) {
        if (run(ffmpeg, {QStringLiteral("-y"), QStringLiteral("-i"), path,
                         QStringLiteral("-frames:v"), QStringLiteral("1"),
                         QStringLiteral("-vf"),
                         QStringLiteral("scale=128:128:force_original_aspect_ratio=decrease"),
                         QStringLiteral("-f"), QStringLiteral("image2"), output})) {
            const QImage image(output);
            if (!image.isNull())
                return image;
        }
    }

    QString magick = QStandardPaths::findExecutable(QStringLiteral("magick"));
    if (magick.isEmpty())
        magick = QStandardPaths::findExecutable(QStringLiteral("convert"));
    if (!magick.isEmpty()) {
        if (run(magick, {path + QStringLiteral("[0]"), QStringLiteral("-thumbnail"),
                         QStringLiteral("128x128"),
                         QStringLiteral("png:") + output})) {
            const QImage image(output);
            if (!image.isNull())
                return image;
        }
        // Some formats reject the frame selector; try the whole file once.
        if (run(magick, {path, QStringLiteral("-thumbnail"),
                         QStringLiteral("128x128"),
                         QStringLiteral("png:") + output})) {
            return QImage(output);
        }
    }
    return {};
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

QString FilesThumbnailProvider::cachedSourceFor(const QString &uri)
{
    const QString canonical = canonicalUri(uri);
    if (canonical.isEmpty())
        return {};
    const QFileInfo source(QUrl(canonical).toLocalFile());
    const QFileInfo cached(cacheFileFor(uri));
    if (!cached.isFile() || cached.lastModified() < source.lastModified())
        return {};
    return sourceFor(uri);
}

bool FilesThumbnailProvider::canGenerate(const QString &uri)
{
    const QUrl url(uri);
    if (!url.isLocalFile())
        return false;
    const QFileInfo info(url.toLocalFile());
    if (!info.isFile())
        return false;
    const QString suffix = info.suffix().toLower();
    if (isImageSuffix(suffix))
        return true;
    if (suffix == QStringLiteral("pdf"))
        return !QStandardPaths::findExecutable(QStringLiteral("pdftoppm")).isEmpty();
    if (isVideoSuffix(suffix)) {
        return !QStandardPaths::findExecutable(QStringLiteral("ffmpegthumbnailer"))
                        .isEmpty()
                || !QStandardPaths::findExecutable(QStringLiteral("ffmpeg")).isEmpty();
    }
    return false;
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

    const QString suffix = source.suffix().toLower();
    QImage image;
    if (isImageSuffix(suffix)) {
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
        image = reader.read();
        if (image.isNull())
            image = renderImageFallback(source.canonicalFilePath());
    } else if (suffix == QStringLiteral("pdf")) {
        image = renderPdf(source.canonicalFilePath());
    } else if (isVideoSuffix(suffix)) {
        image = renderVideo(source.canonicalFilePath());
    }
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
