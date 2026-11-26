// SPDX-License-Identifier: GPL-3.0-or-later
// T338: the menu bar's wallpaper luma is a contrast input, not a reason to
// decode a 4K image on the GUI thread. A small top-band thumbnail is decoded
// on a worker thread and cached by source + mtime + fallback colour.
#pragma once

#include <QColor>
#include <QHash>
#include <QImage>
#include <QObject>
#include <QSet>
#include <QSize>
#include <QString>

#include <functional>
#include <optional>

class QThread;

// Decode `source` into at most `maxSize`, preserving the aspect ratio. The
// reader is asked for a scaled decode first (JPEG and friends scale in the
// codec); a plugin that ignores the request is scaled afterwards. The result
// is small enough that the top-band luma pass is cheap.
QImage wallpaperLumaThumbnail(const QString &source,
                              const QSize &maxSize = QSize(320, 320));

class WallpaperLuma : public QObject
{
    Q_OBJECT

public:
    using Decoder = std::function<QImage(const QString &source)>;

    // `decoder` is a test seam; the production default is
    // `wallpaperLumaThumbnail`.
    explicit WallpaperLuma(QObject *parent = nullptr, Decoder decoder = {});
    ~WallpaperLuma() override;

    // Normalised cache key: source path + mtime + size + fallback colour. A
    // changed image (new mtime or size) is decoded again; a Space switch on
    // the same file is a cache hit.
    static QString keyFor(const QString &source, const QColor &fallback);

    // True when `key` is already decoded; writes the luma.
    bool cached(const QString &key, double *luma) const;

    // Return the luma immediately for a cache hit or an empty source (the
    // solid fallback needs no decode); otherwise queue the decode on the
    // worker thread and return nullopt. `lumaReady` follows. A repeat request
    // while the same key is in flight is coalesced.
    std::optional<double> request(const QString &key, const QString &source,
                                  const QColor &fallback);

signals:
    void lumaReady(const QString &key, double luma);

private:
    void onDecoded(const QString &key, double luma);
    void insertCache(const QString &key, double luma);

    QThread *m_thread = nullptr;
    QObject *m_worker = nullptr;
    Decoder m_decoder;
    QHash<QString, double> m_cache;
    QSet<QString> m_inFlight;
};
