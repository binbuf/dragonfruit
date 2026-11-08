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

#include <QSet>
#include <QString>
#include <QStringList>
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

// The `org.dragonfruit.MenuBroker1.SetWindowStates` payload for the Dock's
// running projection: one row per app-level entry (`kind == "temporary"`) as
// `{appId, windows, minimized}`. The per-window "minimized" entries are not
// apps and are skipped, matching the broker's reading. Pure and unit-tested.
QString windowStatesJson(const QVariantList &entries);

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

// The synthesized dynamic Window menu (T230): Minimize, one row per open
// window of `appId` with the focused one checked, then Bring All to Front.
// Empty when `appId` is empty or has no app-level entry. Mirrors the broker's
// `synthesized_window_menu`.
QVariantList windowMenu(const QVariantList &entries, const QString &appId,
                        const QString &appName);

// Whether a top-level menu list already carries `title` (case-insensitive).
// Used to avoid appending a second Window menu.
bool hasMenuTitle(const QVariantList &menus, const QString &title);

// T230: the compositor window ids Hide <App> should minimize: every visible
// (non-minimized) window of `appId`, or every visible window of every *other*
// app when `others` is true. Empty when there is nothing to hide.
QStringList visibleWindowIds(const QVariantList &entries, const QString &appId,
                             bool others = false);

// Drop tracked ids whose windows no longer exist (closed while hidden), so
// Show All never addresses a stale compositor handle.
QStringList retainWindowIds(const QVariantList &entries, const QSet<QString> &windowIds);

// T230: every currently minimized window id in the projection. Show All falls
// back to this set when the shell has no tracked Hide/Hide Others ids, so the
// enabled verb is never inert.
QStringList minimizedWindowIds(const QVariantList &entries);

// A publishable application accelerator (T-14.2b): the opaque action the owner
// handles and the chord that triggers it. The chord keeps the published
// spelling (`Super+H`, `Ctrl+Super+F`), which the compositor's keymap parser
// accepts.
struct MenuAccelerator {
    QString action;
    QString chord;
};

// Flatten a menu row list into accelerators, recursing through submenus. Rows
// without both a `shortcut` and an `action` do not register. Mirrors the
// broker's `accelerators::extract` so the two can never disagree about which
// rows are actionable. Pure and unit-tested (tst_dockcore).
QList<MenuAccelerator> menuAccelerators(const QVariantList &rows);

// The full table for the focused app: the fixed application menu first, then
// the app's own top-level menus.
QList<MenuAccelerator> publishedAccelerators(const QVariantList &applicationMenuItems,
                                             const QVariantList &appMenuModel);

// Serialize accelerators into the `df_toplevel_manager.set_app_accelerators`
// wire form: one `action<TAB>chord` per line, empty for no accelerators.
QString acceleratorWireTable(const QList<MenuAccelerator> &accelerators);