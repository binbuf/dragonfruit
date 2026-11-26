# SPDX-License-Identifier: GPL-3.0-only
# Shared, toolkit-facing libraries used by more than one first-party process
# (shell and apps): the settingsd client (ADR 0036), the system font
# (Inter, installed process-wide by every first-party entry point), the
# localization helper (T-16.7), and the launch-time display hand-off (T309).

add_subdirectory(settings-client)
add_subdirectory(system-font)
add_subdirectory(i18n)
add_subdirectory(chromepointer)
add_subdirectory(launch-env)