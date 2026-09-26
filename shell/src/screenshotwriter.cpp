// SPDX-License-Identifier: MIT
#include "screenshotwriter.h"

#include <QClipboard>
#include <QDir>
#include <QFileInfo>
#include <QGuiApplication>
#include <QMimeData>
#include <QStandardPaths>
#include <QUrl>

ScreenshotWriter::ScreenshotWriter(QObject *parent)
    : QObject(parent)
{
}

QString ScreenshotWriter::saveDirectory()
{
    // The test/capture override first: an offscreen run has no XDG user dirs.
    const QString override = qEnvironmentVariable("DF_SCREENSHOT_DIR");
    if (!override.isEmpty())
        return override;
    const QString pictures = QStandardPaths::writableLocation(QStandardPaths::PicturesLocation);
    if (pictures.isEmpty())
        return QString();
    return pictures + QStringLiteral("/Screenshots");
}

QString ScreenshotWriter::baseName(const QDateTime &now)
{
    return QStringLiteral("Screenshot_") + now.toString(QStringLiteral("yyyy-MM-dd_HH-mm-ss"));
}

QString ScreenshotWriter::savePath(const QDateTime &now)
{
    const QString directory = saveDirectory();
    if (directory.isEmpty())
        return QString();
    QDir dir(directory);
    const QString base = baseName(now);
    QString candidate = dir.filePath(base + QStringLiteral(".png"));
    for (int suffix = 1; QFileInfo::exists(candidate) && suffix < 10000; ++suffix) {
        candidate = dir.filePath(base + QStringLiteral("-") + QString::number(suffix)
                                 + QStringLiteral(".png"));
    }
    return candidate;
}

bool ScreenshotWriter::save(const QImage &image, const QString &path)
{
    if (image.isNull() || path.isEmpty())
        return false;
    const QFileInfo info(path);
    if (!info.dir().exists() && !QDir().mkpath(info.absolutePath()))
        return false;
    return image.save(path, "PNG");
}

bool ScreenshotWriter::copy(const QImage &image, const QString &filePath)
{
    if (image.isNull())
        return false;
    QClipboard *clipboard = QGuiApplication::clipboard();
    if (!clipboard)
        return false;
    auto *mime = new QMimeData;
    mime->setImageData(image);
    if (!filePath.isEmpty())
        mime->setUrls({QUrl::fromLocalFile(filePath)});
    clipboard->setMimeData(mime);
    return true;
}

ScreenshotWriter::Result ScreenshotWriter::deliver(const QImage &image) const
{
    Result result;
    result.path = savePath();
    result.saved = save(image, result.path);
    if (result.saved)
        result.copied = copy(image, result.path);
    return result;
}