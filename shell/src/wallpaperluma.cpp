// SPDX-License-Identifier: GPL-3.0-or-later
#include "wallpaperluma.h"

#include "wallpaperpolicy.h"

#include <QFileInfo>
#include <QImageReader>
#include <QMetaObject>
#include <QThread>

namespace {

constexpr int kMaxCacheEntries = 64;

class WallpaperLumaWorker : public QObject
{
    Q_OBJECT

public:
    explicit WallpaperLumaWorker(WallpaperLuma::Decoder decoder)
        : m_decoder(std::move(decoder))
    {
    }

public slots:
    void decode(const QString &key, const QString &source, const QColor &fallback)
    {
        const QImage thumbnail =
            m_decoder ? m_decoder(source) : wallpaperLumaThumbnail(source);
        emit decoded(key, wallpaperTopBandLuminance(thumbnail, fallback));
    }

signals:
    void decoded(const QString &key, double luma);

private:
    WallpaperLuma::Decoder m_decoder;
};

} // namespace

QImage wallpaperLumaThumbnail(const QString &source, const QSize &maxSize)
{
    if (source.isEmpty())
        return QImage();
    QImageReader reader(source);
    reader.setAutoTransform(true);
    const QSize original = reader.size();
    if (original.isValid() && maxSize.isValid()) {
        const QSize scaled = original.scaled(maxSize, Qt::KeepAspectRatio)
                                 .expandedTo(QSize(1, 1));
        reader.setScaledSize(scaled);
    }
    QImage image = reader.read();
    // Some image plugins ignore setScaledSize; still never hand a full 4K
    // buffer to the luma pass.
    if (!image.isNull() && maxSize.isValid()
        && (image.width() > maxSize.width()
            || image.height() > maxSize.height())) {
        image = image.scaled(maxSize, Qt::KeepAspectRatio,
                             Qt::SmoothTransformation);
    }
    return image;
}

WallpaperLuma::WallpaperLuma(QObject *parent, Decoder decoder)
    : QObject(parent)
    , m_thread(new QThread(this))
    , m_decoder(std::move(decoder))
{
    auto *worker = new WallpaperLumaWorker(m_decoder);
    worker->moveToThread(m_thread);
    m_worker = worker;
    connect(m_thread, &QThread::finished, worker, &QObject::deleteLater);
    connect(worker, &WallpaperLumaWorker::decoded, this,
            &WallpaperLuma::onDecoded, Qt::QueuedConnection);
    m_thread->start();
}

WallpaperLuma::~WallpaperLuma()
{
    // A decode in flight finishes before shutdown; the GUI thread is never
    // involved either way.
    m_thread->quit();
    m_thread->wait();
}

QString WallpaperLuma::keyFor(const QString &source, const QColor &fallback)
{
    const QFileInfo info(source);
    const qint64 mtime =
        info.exists() ? info.lastModified().toMSecsSinceEpoch() : 0;
    const qint64 size = info.exists() ? info.size() : 0;
    return source + QLatin1Char('|') + QString::number(mtime) + QLatin1Char('|')
        + QString::number(size) + QLatin1Char('|')
        + (fallback.isValid() ? fallback.name(QColor::HexArgb) : QString());
}

bool WallpaperLuma::cached(const QString &key, double *luma) const
{
    const auto it = m_cache.constFind(key);
    if (it == m_cache.constEnd())
        return false;
    if (luma)
        *luma = it.value();
    return true;
}

std::optional<double> WallpaperLuma::request(const QString &key,
                                             const QString &source,
                                             const QColor &fallback)
{
    if (const auto it = m_cache.constFind(key); it != m_cache.constEnd())
        return it.value();
    if (source.isEmpty()) {
        // No image: the menu bar uses the Space's solid colour immediately.
        const double luma = wallpaperTopBandLuminance(QImage(), fallback);
        insertCache(key, luma);
        return luma;
    }
    if (m_inFlight.contains(key))
        return std::nullopt;
    m_inFlight.insert(key);
    QMetaObject::invokeMethod(m_worker, "decode", Qt::QueuedConnection,
                              Q_ARG(QString, key), Q_ARG(QString, source),
                              Q_ARG(QColor, fallback));
    return std::nullopt;
}

void WallpaperLuma::onDecoded(const QString &key, double luma)
{
    m_inFlight.remove(key);
    insertCache(key, luma);
    emit lumaReady(key, luma);
}

void WallpaperLuma::insertCache(const QString &key, double luma)
{
    if (m_cache.size() >= kMaxCacheEntries)
        m_cache.clear();
    m_cache.insert(key, luma);
}

#include "wallpaperluma.moc"
