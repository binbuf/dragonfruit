// SPDX-License-Identifier: MIT
#include "FilesSettings.h"

#include "settingsclient.h"

#include <QImageReader>
#include <QString>

namespace {

// Average sRGB luma of a decoded wallpaper. The image is sampled on a fixed
// grid so a 6K photo costs the same as a small one; the desktop only needs a
// polarity decision, not a per-pixel map.
qreal averageLuminance(const QImage &image)
{
    if (image.isNull() || image.width() <= 0 || image.height() <= 0)
        return 0.5;
    const QImage sample = image.scaled(64, 64, Qt::IgnoreAspectRatio,
                                       Qt::SmoothTransformation)
                                  .convertToFormat(QImage::Format_RGB32);
    quint64 sum = 0;
    quint64 count = 0;
    for (int y = 0; y < sample.height(); ++y) {
        const auto *line = reinterpret_cast<const QRgb *>(sample.constScanLine(y));
        for (int x = 0; x < sample.width(); ++x) {
            const QRgb pixel = line[x];
            sum += 299 * qRed(pixel) + 587 * qGreen(pixel) + 114 * qBlue(pixel);
            ++count;
        }
    }
    return count > 0 ? static_cast<qreal>(sum) / static_cast<qreal>(count)
                             / 1000.0 / 255.0
                     : 0.5;
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

qreal FilesSettings::wallpaperLuminance() const
{
    const QString source = effectiveWallpaperSource();
    const QString key = source.isEmpty() ? QStringLiteral("<solid>") : source;
    if (key == m_wallpaperLumaKey)
        return m_wallpaperLuma;
    m_wallpaperLumaKey = key;
    m_wallpaperLuma = 0.5;
    if (!source.isEmpty()) {
        QImageReader reader(source);
        reader.setAutoTransform(true);
        const QImage image = reader.read();
        if (!image.isNull())
            m_wallpaperLuma = averageLuminance(image);
    }
    return m_wallpaperLuma;
}

void FilesSettings::set(const QString &key, const QVariant &value)
{
    m_client->set(key, value);
}
