// SPDX-License-Identifier: GPL-3.0-or-later
// The media-key step policy (T321).
//
// The compositor routes the XF86 volume/mute/brightness keys as ordinary
// input actions (`volume-up`, `volume-down`, `mute`, `brightness-up`,
// `brightness-down`). This pure policy turns one action into the single write
// the shell performs and the value the existing OSD presents. It holds no
// audio or brightness state: the current level comes from the status model and
// settingsd, and the result goes back through the same paths the Quick
// Settings sliders use.
#pragma once

#include <QString>

namespace MediaKeys {

// One step per key press. The compositor's shortcut filter ignores
// auto-repeat of an intercepted key, so holding a key does not ramp; every
// press is exactly one step (ADR 0331).
inline constexpr double kVolumeStep = 0.05;
inline constexpr double kBrightnessStep = 0.05;

enum class Write {
    // Not a media-key action, or the path it needs is absent.
    None,
    // Write `value` through the audio client (0..1).
    SetVolume,
    // Write `muted` through the audio client; `value` is the level to show.
    SetMute,
    // Write `value` to settingsd's `display.brightness` (0..1).
    SetBrightness,
};

struct Plan {
    Write write = Write::None;
    // The resulting level in [0, 1] (volume or brightness).
    double value = 0.0;
    // The resulting mute state (volume only).
    bool muted = false;
};

// Whether `action` is one of the five media-key actions this policy owns.
bool handles(const QString &action);

// Plan one action. `audioAvailable` is false when the status host or its
// audio view is absent; `brightnessAvailable` is false when no panel write
// path exists. An absent path yields `Write::None` so the caller neither
// writes nor shows the OSD.
Plan plan(const QString &action, bool audioAvailable, double volume, bool muted,
          bool brightnessAvailable, double brightness);

} // namespace MediaKeys
