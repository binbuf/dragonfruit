// SPDX-License-Identifier: MIT
// The Files image-thumbnail provider (T-10.4b).
//
// QML asks for `image://files-thumbnail/<base64url(file URI)>`; the provider
// serves a cached 128px preview for local image files. Thumbnails are written
// to the freedesktop cache (`$XDG_CACHE_HOME/thumbnails/normal`) with the
// standard md5-of-URI name and `Thumb::URI`/`Thumb::MTime` metadata, so Files
// shares its cache with every other spec-conforming file manager.
//
// `QQuickImageProvider` requests run on Qt Quick's pixmap-reader thread, and
// `ForceAsynchronousImageLoading` keeps even the first request off the UI
// thread: a large image can never stall the listing or the window.
#pragma once

#include <QImage>
#include <QQuickImageProvider>
#include <QSize>
#include <QString>

class FilesThumbnailProvider : public QQuickImageProvider
{
public:
    FilesThumbnailProvider();

    QImage requestImage(const QString &id, QSize *size,
                        const QSize &requestedSize) override;

    // The QML `source` URL for a local file URI, or empty when the item is not
    // a local file. The id is base64url-encoded so no URI character (spaces,
    // `#`, `%`, non-ASCII) can be mistaken for a URL separator or decoded by
    // the QML engine before the provider sees it.
    static QString sourceFor(const QString &uri);

private:
    static QString cacheFileFor(const QString &uri);
};
