// SPDX-License-Identifier: MIT
#include "controlcenterpolicy.h"

QString focusModeForToggle(bool enabled)
{
    return enabled ? QStringLiteral("dnd") : QStringLiteral("off");
}

QString colorSchemeForDarkToggle(bool dark)
{
    return dark ? QStringLiteral("dark") : QStringLiteral("light");
}