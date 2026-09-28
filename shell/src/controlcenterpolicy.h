// SPDX-License-Identifier: MIT
// The Control Center tile -> owner mapping (T-11.3b).
//
// The panel is a pure view: its Focus and dark-mode toggles raise booleans and
// the shell turns those into the owning service's vocabulary. Keeping the
// mapping here (pure, no QML or bus) makes the "the toggle reaches the right
// owner" contract unit-testable headlessly.
#pragma once

#include <QString>
#include <QVariantMap>

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

// The Mission Control tile's view, projected from the settingsd values
// (T-15.5b). Mission Control and hot corners are compositor-native, so the
// shell derives the summary the tile shows from the keys the pane writes
// (`overview.hotCorner*` plus the `gestures.*` trio), mirroring the Rust
// `MissionControlSnapshot::label()`:
//
//   * gesture reachable, no corner          -> "Gesture"
//   * gesture reachable, n corners           -> "Gesture, n corner(s)"
//   * no gesture, no corner                  -> "No trigger"
//   * no gesture, n corners                  -> "n corner(s)"
//
// The returned map is `{ state, glyph, label, reachable, gesture, cornerCount }`
// with `state` always `available`: the compositor that owns the runtime is the
// shell's own peer. Pure, so the projection is unit-testable without a bus.
QVariantMap missionControlView(const QVariantMap &values);