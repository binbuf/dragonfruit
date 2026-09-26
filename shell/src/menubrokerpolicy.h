// SPDX-License-Identifier: MIT
// The fixed application menu's live state (T-14.2a).
//
// The menu-broker owns global-menu resolution and the fixed application menu
// (Hide/Hide Others/Show All) with live state. The shell is the window-state
// authority, so this is the pure mapping from the shell's own running-window
// projection onto the three verbs' enabled flags. It mirrors the broker's rule
// exactly (services/menu-broker/src/model.rs) and is unit-testable without
// QML, Wayland, or D-Bus.
#pragma once

#include <QString>
#include <QVariantList>

// The live enabled state of the fixed application menu's hide verbs.
struct AppMenuLiveState {
    // Hide <App>: the focused app has a visible (non-minimized) window.
    bool hide = false;
    // Hide Others: another app still has a visible window.
    bool hideOthers = false;
    // Show All: at least one app is fully hidden (every window minimized).
    bool showAll = false;
};

// Derive the hide-verb state from the focused app id and the Dock's running
// projection (`ShellProtocol::dockStateChanged` entries). App-level entries
// carry `kind == "temporary"`, an `appId`, and a `minimized` flag that is true
// only when all the app's windows are minimized.
AppMenuLiveState appMenuLiveState(const QString &focusedAppId, const QVariantList &entries);

// The standard fixed application menu (About/Settings/Hide/Hide Others/Show
// All/Quit) with the hide verbs' live flags. This is the shell's fallback when
// the menu-broker service is absent; the labels and actions match the broker's
// synthesized menu.
QVariantList fixedApplicationMenu(const QString &appName, const AppMenuLiveState &state);

// Rewrite the hide verbs' `enabled` flags on a published application menu (the
// broker's own rewrite), matching rows by action and falling back to the
// label. Used when the shell consumes a broker-resolved published menu.
QVariantList applyAppMenuLiveState(const QVariantList &published,
                                   const AppMenuLiveState &state);