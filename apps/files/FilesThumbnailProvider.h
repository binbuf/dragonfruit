// SPDX-License-Identifier: MIT
// The Files image-thumbnail provider (T-10.4b).
//
// QML asks for `image://files-thumbnail/<base64url(file URI)>`; the provider
// serves a cached 128px preview for local files. Images decode through Qt's
// image readers; PDFs and videos use optional local helpers (`pdftoppm`,
// `ffmpegthumbnailer`, or `ffmpeg`) when present. Thumbnails are written to
// the freedesktop cache (`$XDG_CACHE_HOME/thumbnails/normal`) with the
// standard md5-of-URI name and `Thumb::URI`/`Thumb::MTime` metadata, so Files
// shares its cache with every other spec-conforming file manager.
//
// `QQuickImageProvider` requests run on Qt Quick's pixmap-reader thread, and
// `ForceAsynchronousImageLoading` keeps even the first request off the UI
// thread: a large image or a PDF/video helper can never stall the listing or
// the window.
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
    // The same source URL only when a fresh thumbnail already exists in the
    // freedesktop cache. Used for non-image files: PDFs/videos/archives get a
    // preview when another spec-conforming thumbnailer has populated the
    // cache, without asking the provider to decode the file itself.
    static QString cachedSourceFor(const QString &uri);
    // Whether this build/host can generate a preview for `uri`: images always
    // use Qt's image readers; PDFs and videos need an optional local helper
    // (`pdftoppm`, `ffmpegthumbnailer`, or `ffmpeg`).
    static bool canGenerate(const QString &uri);

private:
    static QString cacheFileFor(const QString &uri);
};
