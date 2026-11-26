// SPDX-License-Identifier: GPL-3.0-or-later
// Panel-backlight discovery and level mapping (T319).
//
// `/sys/class/backlight/*` is the kernel's backlight inventory. The shell
// resolves one device there and asks logind to write it; the Settings app uses
// the same resolver only to say whether the panel has a backlight at all. The
// helpers are pure filesystem/arithmetic so both consumers and the unit tests
// share one device choice and one 0..1 -> device-range mapping.
#pragma once

#include <QString>

// One usable `/sys/class/backlight` device.
struct BacklightDevice {
    // The directory name logind expects as the `name` argument.
    QString name;
    // The sysfs directory (`/sys/class/backlight/<name>`).
    QString path;
    // The device's `max_brightness`, always > 0 for a valid device.
    int maxBrightness = 0;
    // `raw`, `platform`, or `firmware` from the device's `type` file.
    QString type;

    bool isValid() const { return !name.isEmpty() && maxBrightness > 0; }
};

// Resolve the preferred device under `root` (default `/sys/class/backlight`).
// A `raw` device wins over `platform`, which wins over `firmware`; ties break
// by name. A directory without a readable positive `max_brightness` is skipped.
// Returns an invalid device when none is usable.
BacklightDevice resolveBacklightDevice(
    const QString &root = QStringLiteral("/sys/class/backlight"));

// Map a 0..1 brightness level onto the device's 0..max range, rounding to the
// nearest device step. A non-positive maximum maps to 0.
int mapBrightnessLevel(double level, int maxBrightness);
