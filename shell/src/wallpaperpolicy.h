// SPDX-License-Identifier: MIT
// The compositor wallpaper view of the settings schema (T-09.3, extended by
// T-18.1b). Pure: it maps the `org.dragonfruit.Settings1` wallpaper keys and
// the provider's resolved defaults onto what the shell forwards over
// `df_workspace.set_wallpaper`. No Wayland, no QML, no bus — unit-testable in
// isolation.
#pragma once

#include <cstdint>
#include <QString>
#include <QVariantMap>

// The typed wallpaper selection. Defaults mirror the settings schema (the same
// values the SettingsClient seeds when settingsd is absent).
struct WallpaperSettings {
    // The user's explicit choice (owner apps/settings); empty means "no
    // override", never overwritten by the provider.
    QString source;
    // The shipped original `Default.jpg` path, resolved (ADR 0094).
    QString builtinDefault;
    // The fetched Featured default/fallback path; empty until a catalogue
    // exists.
    QString providerSource;
    // One of "fill" | "fit" | "stretch" | "center".
    QString fit = QStringLiteral("fill");
    // Apply to every Space, or only the active Space.
    bool showOnAllSpaces = true;

    // The effective image path the shell forwards: the user choice, else the
    // shipped default, else the fetched fallback; empty keeps the solid color.
    QString effectiveSource() const;

    bool operator==(const WallpaperSettings &) const = default;
};

// Read the wallpaper keys out of a settings value map. Missing keys fall back
// to the schema defaults. The provider-derived `builtinDefault` /
// `providerSource` are empty when neither settingsd nor the caller supplies
// them; the shell fills them from the provider's D-Bus properties or its own
// shipped-asset resolution before choosing the effective source.
WallpaperSettings wallpaperSettingsFromValues(const QVariantMap &values);

// The effective-source precedence, pure so it is unit-testable without a bus:
// the user choice when non-empty, otherwise the shipped default, otherwise the
// fetched fallback. An empty result is the Space's solid color.
QString effectiveWallpaperSource(const QString &user, const QString &builtin,
                                 const QString &provider);

// The `df_workspace.wallpaper_fit` wire value for a schema spelling. An unknown
// spelling is `fill` (the schema default).
uint32_t wallpaperFitFromName(const QString &name);