// SPDX-License-Identifier: MIT
// The compositor wallaper view of the settings schema (T-09.3). Pure: it maps
// the `org.dragonfruit.Settings1` wallpaper keys onto what the shell forwards
// over `df_workspace.set_wallpaper`. No Wayland, no QML, no bus — unit-testable
// in isolation.
#pragma once

#include <cstdint>
#include <QString>
#include <QVariantMap>

// The typed wallpaper selection. Defaults mirror the settings schema (the same
// values the SettingsClient seeds when settingsd is absent).
struct WallpaperSettings {
    // An image path, or empty to keep the Space's solid color.
    QString source;
    // One of "fill" | "fit" | "stretch" | "center".
    QString fit = QStringLiteral("fill");
    // Apply to every Space, or only the active Space.
    bool showOnAllSpaces = true;

    bool operator==(const WallpaperSettings &) const = default;
};

// Read the wallpaper keys out of a settings value map. Missing keys fall back
// to the schema defaults.
WallpaperSettings wallpaperSettingsFromValues(const QVariantMap &values);

// The `df_workspace.wallpaper_fit` wire value for a schema spelling. An unknown
// spelling is `fill` (the schema default).
uint32_t wallpaperFitFromName(const QString &name);