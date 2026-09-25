# SPDX-License-Identifier: MIT
# The shared C++ client for `org.dragonfruit.Settings1` (T-08/T-09.1b).
#
# One implementation, two kinds of consumer: the shell's Dock/Theme/
# compositor-policy controllers and the Settings app's QML `Settings`
# singleton. Keeping it in a Wayland-free, QML-free static library means both
# link the exact same read/write/subscribe logic (no second D-Bus dialect, no
# second schema-default table). See ADR 0036.

add_library(dragonfruit-settings-client STATIC
    settingsclient.cpp
    settingsclient.h
)
target_link_libraries(dragonfruit-settings-client PUBLIC Qt6::Core Qt6::DBus)
target_include_directories(dragonfruit-settings-client PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")