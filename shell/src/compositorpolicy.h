// SPDX-License-Identifier: GPL-3.0-only
// The compositor's motion/input policy view of the settings schema (T-08.2c).
// Pure: it maps the `org.dragonfruit.Settings1` values onto the exact keys the
// shell forwards over `df_toplevel_manager.set_motion_policy` /
// `set_input_policy`. No Wayland, no QML, no bus — unit-testable in isolation.
#pragma once

#include <QString>
#include <QVariantMap>

// T320: one pointer family's libinput snapshot. The Settings Mouse and
// Trackpad panes write separate `input.mouse.*` / `input.trackpad.*` keys; the
// compositor applies each snapshot only to devices of that capability kind.
// The fields without a libinput setter (secondary click, scroll speed,
// double-click, haptics) stay on the shared pointer policy and are not here.
struct PointerFamilyPolicy {
    double pointerSpeed = 0.0;
    bool naturalScroll = true;
    bool tapToClick = true;
    bool leftHanded = false;
    QString scrollMethod = QStringLiteral("two-finger");
    QString accelProfile = QStringLiteral("adaptive");

    bool operator==(const PointerFamilyPolicy &) const = default;
};

// The typed compositor policy. The defaults mirror the settings schema (the
// same values the SettingsClient seeds when settingsd is absent), so a shell
// with no daemon still applies a coherent policy.
struct CompositorPolicy {
    bool reducedMotion = false;
    // The *resolved* spelling ("light" | "dark"): the compositor has no host
    // style hints, so `auto` is resolved in the shell.
    QString colorScheme = QStringLiteral("dark");
    // T196: the attached-menu slider (0..1, measured midpoint 0.5255) and the
    // two accessibility overrides. They travel together with the resolved
    // scheme in the v9 `set_appearance_policy` snapshot.
    double transparency = 0.5255;
    bool reduceTransparency = false;
    bool increaseContrast = false;
    QString titlebarDoubleClick = QStringLiteral("zoom");
    QString minimizedAnimation = QStringLiteral("scale");
    int repeatDelayMs = 200;
    int repeatRateHz = 25;
    bool gesturesEnabled = true;
    bool gestureSpaceSwitch = true;
    bool gestureMissionControl = true;
    // T229: the pointer/trackpad half and the active keyboard layout.
    double pointerSpeed = 0.0;
    bool naturalScroll = true;
    bool tapToClick = true;
    bool leftHanded = false;
    QString scrollMethod = QStringLiteral("two-finger");
    QString secondaryClick = QStringLiteral("right");
    double scrollSpeed = 0.0;
    int doubleClickMs = 400;
    bool haptics = true;
    // T320: the per-family snapshots forwarded over
    // `df_toplevel_manager.set_pointer_family_policy` (v15). Their defaults
    // follow the schema, including a mouse's `button` scroll method.
    PointerFamilyPolicy mousePointer;
    PointerFamilyPolicy trackpadPointer;
    QString keyboardLayout;
    QString keyboardVariant;
    // T275: the compositor-owned accessibility accommodations. The shell
    // forwards the whole snapshot over v12 `set_accessibility_policy`.
    bool magnifierEnabled = false;
    double magnifierZoom = 2.0;
    QString magnifierMode = QStringLiteral("follow-focus");
    QString accessibilityShortcut;
    // T276: the Overview/hot-corner snapshot. Action ids are the schema's
    // stable spellings; modifiers gate the corner dwell; the workspace count
    // resizes the compositor's normal Space list.
    QString hotCornerTopLeft = QStringLiteral("mission-control");
    QString hotCornerTopRight = QStringLiteral("notification-center");
    QString hotCornerBottomLeft = QStringLiteral("desktop-reveal");
    QString hotCornerBottomRight = QStringLiteral("lock-screen");
    QString hotCornerTopLeftModifier = QStringLiteral("none");
    QString hotCornerTopRightModifier = QStringLiteral("none");
    QString hotCornerBottomLeftModifier = QStringLiteral("none");
    QString hotCornerBottomRightModifier = QStringLiteral("none");
    int workspaceCount = 3;
    bool switchToAppSpace = true;

    bool operator==(const CompositorPolicy &) const = default;
};

// Resolve `appearance.colorScheme` (`light` | `dark` | `auto`) against the
// host's current scheme. Pure so the `auto` rule is testable.
QString resolveCompositorColorScheme(const QString &scheme, bool hostDark);

// Read the motion/input keys out of a settings value map. `hostDark` is the
// host scheme `auto` follows (the caller owns watching it live). Missing keys
// fall back to the schema defaults.
CompositorPolicy compositorPolicyFromValues(const QVariantMap &values, bool hostDark);