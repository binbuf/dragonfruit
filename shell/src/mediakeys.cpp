// SPDX-License-Identifier: MIT
#include "mediakeys.h"

#include <QtGlobal>

#include <cmath>

namespace MediaKeys {
namespace {

// Snap to hundredths so repeated 5% presses do not accumulate float noise
// (0.5 + 0.05 must present as exactly 55%, not 55.00000000000001%).
double stepped(double value, double delta)
{
    return qBound(0.0, std::round((value + delta) * 100.0) / 100.0, 1.0);
}

} // namespace

bool handles(const QString &action)
{
    return action == QLatin1String("volume-up")
        || action == QLatin1String("volume-down")
        || action == QLatin1String("mute")
        || action == QLatin1String("brightness-up")
        || action == QLatin1String("brightness-down");
}

Plan plan(const QString &action, bool audioAvailable, double volume, bool muted,
          bool brightnessAvailable, double brightness)
{
    Plan result;
    if (action == QLatin1String("volume-up")
        || action == QLatin1String("volume-down")) {
        if (!audioAvailable)
            return result;
        const double delta =
            action == QLatin1String("volume-up") ? kVolumeStep : -kVolumeStep;
        result.write = Write::SetVolume;
        result.value = stepped(volume, delta);
        result.muted = muted;
        return result;
    }
    if (action == QLatin1String("mute")) {
        if (!audioAvailable)
            return result;
        result.write = Write::SetMute;
        result.value = qBound(0.0, volume, 1.0);
        result.muted = !muted;
        return result;
    }
    if (action == QLatin1String("brightness-up")
        || action == QLatin1String("brightness-down")) {
        if (!brightnessAvailable)
            return result;
        const double delta =
            action == QLatin1String("brightness-up") ? kBrightnessStep
                                                     : -kBrightnessStep;
        result.write = Write::SetBrightness;
        result.value = stepped(brightness, delta);
        return result;
    }
    return result;
}

} // namespace MediaKeys
