// SPDX-License-Identifier: MIT
#include "wallpaperpolicy.h"

#include <QImageReader>

namespace {

QVariant readValue(const QVariantMap &values, const QString &key, const QVariant &fallback)
{
    const QVariant value = values.value(key);
    return value.isValid() ? value : fallback;
}

} // namespace

QString WallpaperSettings::effectiveSource() const
{
    return effectiveWallpaperSource(source, builtinDefault, providerSource);
}

WallpaperSettings wallpaperSettingsFromValues(const QVariantMap &values)
{
    WallpaperSettings settings;
    settings.source =
        readValue(values, QStringLiteral("wallpaper.source"), settings.source).toString();
    settings.builtinDefault =
        readValue(values, QStringLiteral("wallpaper.builtinDefault"), settings.builtinDefault)
            .toString();
    settings.providerSource =
        readValue(values, QStringLiteral("wallpaper.providerSource"), settings.providerSource)
            .toString();
    settings.fit = readValue(values, QStringLiteral("wallpaper.fit"), settings.fit).toString();
    settings.showOnAllSpaces =
        readValue(values, QStringLiteral("wallpaper.showOnAllSpaces"),
                  settings.showOnAllSpaces)
            .toBool();
    return settings;
}

QString effectiveWallpaperSource(const QString &user, const QString &builtin,
                                 const QString &provider)
{
    // A user choice is sticky; the provider never overwrites it.
    if (!user.isEmpty())
        return user;
    // The shipped default wins over the fetched fallback so the out-of-box
    // background never depends on the network (ADR 0094).
    if (!builtin.isEmpty())
        return builtin;
    // The fetched Featured default, or empty for the Space's solid color.
    return provider;
}

uint32_t wallpaperFitFromName(const QString &name)
{
    // df_workspace.wallpaper_fit: fill=0, fit=1, stretch=2, center=3.
    if (name == QLatin1String("fit"))
        return 1;
    if (name == QLatin1String("stretch"))
        return 2;
    if (name == QLatin1String("center"))
        return 3;
    return 0;
}

double wallpaperTopBandLuminance(const QImage &image, const QColor &fallback)
{
    if (!image.isNull() && image.width() > 0 && image.height() > 0) {
        const QImage rgb = image.format() == QImage::Format_RGB32
                ? image
                : image.convertToFormat(QImage::Format_RGB32);
        const int rows = qMax(1, rgb.height() / 10);
        quint64 sum = 0;
        quint64 count = 0;
        for (int y = 0; y < rows; ++y) {
            const auto *line = reinterpret_cast<const QRgb *>(rgb.constScanLine(y));
            for (int x = 0; x < rgb.width(); ++x) {
                const QRgb pixel = line[x];
                sum += 299 * qRed(pixel) + 587 * qGreen(pixel) + 114 * qBlue(pixel);
                ++count;
            }
        }
        if (count > 0)
            return static_cast<double>(sum) / static_cast<double>(count) / 1000.0 / 255.0;
    }
    const QColor color = fallback.isValid() ? fallback : QColor(128, 128, 128);
    return 0.299 * color.redF() + 0.587 * color.greenF() + 0.114 * color.blueF();
}

double wallpaperTopBandLuminance(const QString &source, const QColor &fallback)
{
    if (!source.isEmpty()) {
        QImageReader reader(source);
        reader.setAutoTransform(true);
        const QImage image = reader.read();
        if (!image.isNull())
            return wallpaperTopBandLuminance(image, fallback);
    }
    return wallpaperTopBandLuminance(QImage(), fallback);
}
