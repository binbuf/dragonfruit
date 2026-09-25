// SPDX-License-Identifier: MIT
// The compositor's motion/input policy view of the settings schema (T-08.2c).
// Pure: it maps the `org.dragonfruit.Settings1` values onto the exact keys the
// shell forwards over `df_toplevel_manager.set_motion_policy` /
// `set_input_policy`. No Wayland, no QML, no bus — unit-testable in isolation.
#pragma once

#include <QString>
#include <QVariantMap>

// The typed compositor policy. The defaults mirror the settings schema (the
// same values the SettingsClient seeds when settingsd is absent), so a shell
// with no daemon still applies a coherent policy.
struct CompositorPolicy {
    bool reducedMotion = false;
    // The *resolved* spelling ("light" | "dark"): the compositor has no host
    // style hints, so `auto` is resolved in the shell.
    QString colorScheme = QStringLiteral("dark");
    QString titlebarDoubleClick = QStringLiteral("zoom");
    QString minimizedAnimation = QStringLiteral("scale");
    int repeatDelayMs = 200;
    int repeatRateHz = 25;
    bool gesturesEnabled = true;
    bool gestureSpaceSwitch = true;
    bool gestureMissionControl = true;

    bool operator==(const CompositorPolicy &) const = default;
};

// Resolve `appearance.colorScheme` (`light` | `dark` | `auto`) against the
// host's current scheme. Pure so the `auto` rule is testable.
QString resolveCompositorColorScheme(const QString &scheme, bool hostDark);

// Read the motion/input keys out of a settings value map. `hostDark` is the
// host scheme `auto` follows (the caller owns watching it live). Missing keys
// fall back to the schema defaults.
CompositorPolicy compositorPolicyFromValues(const QVariantMap &values, bool hostDark);