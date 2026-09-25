// SPDX-License-Identifier: MIT
// The compositor output view of the settings schema (T-09.5). Pure: it maps
// the `org.dragonfruit.Settings1` display keys onto what the shell forwards
// over `df_output.set_scale` / `df_output.set_transform`. No Wayland, no QML,
// no bus — unit-testable in isolation.
#pragma once

#include <cstdint>
#include <QString>
#include <QVariantMap>

// The typed display selection. Defaults mirror the settings schema (the same
// values the SettingsClient seeds when settingsd is absent).
struct DisplaySettings {
    // Output scale / scaled-resolution factor; 1.0 is the native mode.
    double scale = 1.0;
    // One of "normal" | "90" | "180" | "270".
    QString rotation = QStringLiteral("normal");

    bool operator==(const DisplaySettings &) const = default;
};

// Read the display keys out of a settings value map. Missing keys fall back
// to the schema defaults.
DisplaySettings displaySettingsFromValues(const QVariantMap &values);

// The `df_output.transform` wire value for a schema spelling (normal=0, 90=1,
// 180=2, 270=3). An unknown spelling is `normal`.
uint32_t outputTransformFromName(const QString &name);