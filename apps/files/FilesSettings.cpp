// SPDX-License-Identifier: MIT
#include "FilesSettings.h"

#include "settingsclient.h"

#include <QImageReader>
#include <QString>
#include <QtMath>

namespace {

// The luma map is sampled on a fixed grid so a 6K photo costs the same as a
// small one. 64x64 is enough for a per-tile polarity decision and keeps the
// bilinear sample cheap for every desktop icon.
constexpr int kLumaGrid = 64;

qreal pixelLuminance(QRgb pixel)
{
    return (299 * qRed(pixel) + 587 * qGreen(pixel) + 114 * qBlue(pixel))
           / 1000.0 / 255.0;
}

qreal clamp01(qreal value)
{
    return qBound<qreal>(0.0, value, 1.0);
}

} // namespace

FilesSettings::FilesSettings(QObject *parent)
    : QObject(parent)
{
    // The same seam as Settings: `DF_SETTINGS_FIXTURE` selects the
    // deterministic in-process client for headless QML tests; otherwise the
    // live session-bus client serves the schema defaults when the daemon is
    // absent.
    if (qEnvironmentVariableIsSet("DF_SETTINGS_FIXTURE"))
        m_client = new MockSettingsClient(this);
    else
        m_client = new DbusSettingsClient(this);

    connect(m_client, &SettingsClient::changed, this,
            [this](const QString &, const QVariant &) {
                emit valuesChanged();
                emit wallpaperLuminanceChanged();
            });
    connect(m_client, &SettingsClient::availableChanged, this,
            [this](bool) { emit availableChanged(); });
}

QVariantMap FilesSettings::values() const
{
    return m_client->values();
}

bool FilesSettings::available() const
{
    return m_client->isAvailable();
}

QString FilesSettings::glassRollout() const
{
    // Only an explicit override selects a rail; an empty result lets the QML
    // shell read `Theme.glass.rollout.defaultMode` from the shared tokens.
    const QString requested =
        qEnvironmentVariable("DRAGONFRUIT_GLASS_ROLLOUT").trimmed().toLower();
    return (requested == QLatin1String("legacy")
            || requested == QLatin1String("replacement"))
        ? requested : QString();
}

bool FilesSettings::appChromeSupported() const
{
    return !qEnvironmentVariableIsSet("DF_APP_CHROME_SOLID");
}

QString FilesSettings::effectiveWallpaperSource() const
{
    const QVariantMap map = m_client->values();
    const QString user = map.value(QStringLiteral("wallpaper.source")).toString();
    if (!user.isEmpty())
        return user;
    const QString builtin =
        map.value(QStringLiteral("wallpaper.builtinDefault")).toString();
    if (!builtin.isEmpty())
        return builtin;
    return map.value(QStringLiteral("wallpaper.providerSource")).toString();
}

void FilesSettings::ensureLumaGrid() const
{
    const QString source = effectiveWallpaperSource();
    if (source == m_lumaGridKey)
        return;
    m_lumaGridKey = source;
    m_lumaGrid.clear();
    m_lumaImageAspect = 1.0;
    if (source.isEmpty())
        return;
    QImageReader reader(source);
    reader.setAutoTransform(true);
    const QImage image = reader.read();
    if (image.isNull() || image.width() <= 0 || image.height() <= 0)
        return;
    m_lumaImageAspect = static_cast<qreal>(image.width())
                        / static_cast<qreal>(image.height());
    const QImage sample = image.scaled(kLumaGrid, kLumaGrid, Qt::IgnoreAspectRatio,
                                       Qt::SmoothTransformation)
                                  .convertToFormat(QImage::Format_RGB32);
    m_lumaGrid.resize(kLumaGrid * kLumaGrid);
    for (int y = 0; y < kLumaGrid; ++y) {
        const auto *line = reinterpret_cast<const QRgb *>(sample.constScanLine(y));
        for (int x = 0; x < kLumaGrid; ++x)
            m_lumaGrid[y * kLumaGrid + x] = pixelLuminance(line[x]);
    }
}

qreal FilesSettings::wallpaperLuminance() const
{
    ensureLumaGrid();
    if (m_lumaGrid.isEmpty())
        return 0.5;
    qreal sum = 0;
    for (const qreal value : m_lumaGrid)
        sum += value;
    return sum / static_cast<qreal>(m_lumaGrid.size());
}

qreal FilesSettings::wallpaperLuminanceAt(qreal x, qreal y) const
{
    ensureLumaGrid();
    if (m_lumaGrid.isEmpty())
        return 0.5;

    const qreal nx = clamp01(x);
    const qreal ny = clamp01(y);
    const QString fit =
        m_client->values().value(QStringLiteral("wallpaper.fit")).toString();
    const qreal outW = m_desktopWidth > 0 ? m_desktopWidth : 16.0;
    const qreal outH = m_desktopHeight > 0 ? m_desktopHeight : 10.0;

    // Map output-normalized coordinates into image-normalized coordinates.
    // The image is expressed in "height units": (aspect, 1).
    qreal ix = nx;
    qreal iy = ny;
    if (fit == QLatin1String("fill") || fit == QLatin1String("fit")) {
        const qreal imageW = m_lumaImageAspect;
        const qreal scale = fit == QLatin1String("fill")
                ? qMax(outW / imageW, outH)
                : qMin(outW / imageW, outH);
        const qreal dw = imageW * scale;
        const qreal dh = scale;
        ix = (nx * outW - (outW - dw) / 2.0) / dw;
        iy = (ny * outH - (outH - dh) / 2.0) / dh;
        if (fit == QLatin1String("fit")
                && (ix < 0.0 || ix > 1.0 || iy < 0.0 || iy > 1.0))
            return wallpaperLuminance();
    } else if (fit == QLatin1String("center")) {
        // The native pixel size is not cached, so a centre crop is ambiguous;
        // the average is the safe polarity.
        return wallpaperLuminance();
    }
    // "stretch" (and the no-fit fallback) maps one-to-one.

    const qreal gx = clamp01(ix) * (kLumaGrid - 1);
    const qreal gy = clamp01(iy) * (kLumaGrid - 1);
    const int x0 = qBound(0, qFloor(gx), kLumaGrid - 1);
    const int y0 = qBound(0, qFloor(gy), kLumaGrid - 1);
    const int x1 = qMin(x0 + 1, kLumaGrid - 1);
    const int y1 = qMin(y0 + 1, kLumaGrid - 1);
    const qreal fx = gx - x0;
    const qreal fy = gy - y0;
    const qreal top = m_lumaGrid[y0 * kLumaGrid + x0] * (1.0 - fx)
                      + m_lumaGrid[y0 * kLumaGrid + x1] * fx;
    const qreal bottom = m_lumaGrid[y1 * kLumaGrid + x0] * (1.0 - fx)
                         + m_lumaGrid[y1 * kLumaGrid + x1] * fx;
    return top * (1.0 - fy) + bottom * fy;
}

void FilesSettings::setDesktopSize(qreal width, qreal height)
{
    m_desktopWidth = width > 0 ? width : 0;
    m_desktopHeight = height > 0 ? height : 0;
}

void FilesSettings::set(const QString &key, const QVariant &value)
{
    m_client->set(key, value);
}
