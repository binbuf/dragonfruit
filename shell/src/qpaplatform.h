// SPDX-License-Identifier: GPL-3.0-only
// Qt platform-plugin (QPA) lookup for the shell (T342).
//
// The shell normally asks Qt for the Wayland QPA so its GPU chrome rail can
// create a Qt-side GL context (ADR 0195). The Wayland QPA lives in a separate
// distro package on several distributions, so a source build can start on a
// host where Qt is present but that plugin is not. `main.cpp` uses these
// helpers to detect the missing plugin before `QGuiApplication` aborts, then
// falls back to the offscreen QPA with a clear warning.
//
// The functions are pure so `tst_qpaplatform` can exercise the lookup without
// installing, removing, or loading a platform plugin.
#pragma once

#include <QProcessEnvironment>
#include <QString>
#include <QStringList>

// Candidate file names for the Qt 6 Wayland platform plugin. Qt 6.5+ ships
// the generic plugin as `libqwayland.so`; older packaging split it into the
// generic and EGL variants.
QStringList waylandQpaPluginNames();

// Platform-plugin directories for one Qt installation, in lookup order:
// `QT_QPA_PLATFORM_PLUGIN_PATH` entries (already platform directories), then
// `<qtPluginsPath>/platforms`, then each `QT_PLUGIN_PATH` root + `/platforms`.
// Empty entries are skipped and duplicates are removed.
QStringList qpaPluginDirectories(const QString &qtPluginsPath,
                                 const QProcessEnvironment &environment);

// True when any candidate Wayland plugin file exists in `directories`.
bool waylandQpaPluginPresent(const QStringList &directories);

// True when `platform` (a `QT_QPA_PLATFORM` value) selects a Wayland key.
// The value may be a `;`-separated preference list (`wayland;xcb`); an empty
// value means Qt's default and returns false.
bool qpaPlatformIsWayland(const QString &platform);
