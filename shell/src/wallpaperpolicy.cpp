// SPDX-License-Identifier: MIT
#include "wallpaperpolicy.h"

namespace {

QVariant readValue(const QVariantMap &values, const QString &key, const QVariant &fallback)
{
    const QVariant value = values.value(key);
    return value.isValid() ? value : fallback;
}

} // namespace

WallpaperSettings wallpaperSettingsFromValues(const QVariantMap &values)
{
    WallpaperSettings settings;
    settings.source =
        readValue(values, QStringLiteral("wallpaper.source"), settings.source).toString();
    settings.fit = readValue(values, QStringLiteral("wallpaper.fit"), settings.fit).toString();
    settings.showOnAllSpaces =
        readValue(values, QStringLiteral("wallpaper.showOnAllSpaces"),
                  settings.showOnAllSpaces)
            .toBool();
    return settings;
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