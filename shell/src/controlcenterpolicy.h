// SPDX-License-Identifier: MIT
// The Control Center tile -> owner mapping (T-11.3b).
//
// The panel is a pure view: its Focus and dark-mode toggles raise booleans and
// the shell turns those into the owning service's vocabulary. Keeping the
// mapping here (pure, no QML or bus) makes the "the toggle reaches the right
// owner" contract unit-testable headlessly.
#pragma once

#include <QString>

// Map the Control Center Focus toggle to the notification service's Focus/DND
// mode (`services/notifications`, ADR 0058). On writes `dnd`, off writes
// `off`; the toggle is Do Not Disturb, so it never selects the middle `focus`
// mode, which the menu bar still reflects when set elsewhere.
QString focusModeForToggle(bool enabled);

// Map the Control Center dark-mode toggle to settingsd's
// `appearance.colorScheme`. On writes `dark`, off writes `light`; the toggle
// always writes an absolute scheme, so it accompanies the Settings app's
// Light/Dark/Auto control rather than overriding `auto` silently.
QString colorSchemeForDarkToggle(bool dark);