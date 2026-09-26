# SPDX-License-Identifier: MIT
# Shared, toolkit-facing libraries used by more than one first-party process
# (shell and apps): the settingsd client (ADR 0036) and the system font
# (Inter, installed process-wide by every first-party entry point).

add_subdirectory(settings-client)
add_subdirectory(system-font)