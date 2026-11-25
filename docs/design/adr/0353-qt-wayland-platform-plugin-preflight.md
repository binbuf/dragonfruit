# ADR 0353 — Qt Wayland platform plugin preflight and offscreen fallback

- **Status:** accepted
- **Date:** 2026-11-24
- **Task:** T342
- **Supersedes:** none

## Context

The shell asks Qt for the Wayland QPA when the GPU chrome rail is requested,
so its Qt-side GL context can exist without a shown `QQuickWindow` (ADR 0195).
The packaged RPM already requires `qt6-qtwayland`, but the source-build path
did not: a host could have Qt Base and Qt Declarative development files and
still abort at `QGuiApplication` with "no Qt platform plugin could be
initialized". The plugin's package name is also not uniform across
distributions, and a preflight that silently installs packages would be
surprising on a developer host.

## Decision

1. The source dependency lists name the plugin: `qt6-qtwayland` on Fedora and
   Alpine, `qt6-wayland` on Debian/Ubuntu, Arch, and openSUSE. The README and
   CONTRIBUTING quick start list the Fedora package.
2. `scripts/check-native-deps.sh` is the one read-only check. It detects the
   distribution and package manager, searches the Qt platform-plugin
   directories (`QT_QPA_PLATFORM_PLUGIN_PATH`, `QT_PLUGIN_PATH`,
   `qtpaths`/`qmake`, `pkg-config`, the dev toolchain, and system roots), and
   reports either the plugin path or the missing package with the exact
   install command. It never invokes root or installs anything. `--plugin-dir`
   restricts the search for deterministic negative testing.
3. `scripts/native-session.sh` runs the check in its login preflight and as a
   missing prerequisite (so the package list can install it), before it arms a
   login. `--skip-preflight` remains the explicit override.
4. `shell/src/main.cpp` checks the same plugin lookup before creating
   `QGuiApplication`. If a Wayland QPA was requested (by the shell's default or
   by `QT_QPA_PLATFORM`) and the plugin is absent, it logs a clear warning and
   sets `QT_QPA_PLATFORM=offscreen` instead of aborting. The offscreen rail is
   a diagnostic fallback, not a supported long-term mode.
5. The lookup is a pure helper (`shell/src/qpaplatform.*`) with unit coverage
   in `tst_qpaplatform`; no plugin is loaded by the test.

## Consequences

- A fresh Fedora/openSUSE/Alpine host that follows the README gets the plugin,
  and a host without it is caught before logout rather than at a black login.
- The fallback keeps a missing plugin debuggable in a nested session, but the
  preflight still fails the native-session arm so the degraded mode cannot
  become the silent default.
- Packaging metadata already declares the plugin for RPM/DEB/Arch; the
  portable tar remains unmanaged and relies on the README and the check.
