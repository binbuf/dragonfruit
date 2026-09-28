// SPDX-License-Identifier: MIT
#include "wallpaperpolicy.h"

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