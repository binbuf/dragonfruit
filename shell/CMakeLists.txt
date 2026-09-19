# SPDX-License-Identifier: GPL-3.0-or-later
# Dragonfruit shell components (menu bar, Dock, Control Center,
# notifications, screenshot UI). Each becomes part of the shell process in
# T-09/T-10 and friends; this scaffold wires the modules into the build.

add_subdirectory(menubar)
add_subdirectory(dock)
add_subdirectory(control-center)
add_subdirectory(notifications)
add_subdirectory(screenshot)
