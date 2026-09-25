# SPDX-License-Identifier: MIT
# Menu-bar tests (T-09): layout zones, adapter degradation, app-menu
# interaction (open/close, escape, click-away, drag-through, focus-switch
# tracking), clock formatting, and a pixel check that status glyphs render.

qt_add_executable(tst_menubar
    tst_menubar.cpp
)
target_link_libraries(tst_menubar PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-shell-menubarplugin
)
qt_import_qml_plugins(tst_menubar)

add_test(
    NAME tst_menubar
    COMMAND tst_menubar -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_menubar.qml
)
set_tests_properties(tst_menubar PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Dock tests (T-10): entry regions, running indicators, magnification,
# placement, auto-hide, activation, and accessibility.
qt_add_executable(tst_dock
    tst_dock.cpp
)
target_link_libraries(tst_dock PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-shell-dockplugin
)
qt_import_qml_plugins(tst_dock)

add_test(
    NAME tst_dock
    COMMAND tst_dock -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_dock.qml
)
set_tests_properties(tst_dock PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Dock core unit tests (T-10): interim `.desktop` resolver/launcher,
# `dock.pinned` persistence, and the pinned+running entry merge. No QML,
# Wayland, or compositor required.
qt_add_executable(tst_dockcore
    tst_dockcore.cpp
)
target_link_libraries(tst_dockcore PRIVATE
    Qt6::Core Qt6::Test
    dragonfruit-shell-dockcore
)

add_test(
    NAME tst_dockcore
    COMMAND tst_dockcore
)
set_tests_properties(tst_dockcore PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# System-status model tests (T-07.5a): the bridge host's JSON views decode to
# the Wi-Fi/volume menu models and each gesture raises its request signal. No
# Wayland, QML, or D-Bus.
qt_add_executable(tst_statusmodel
    tst_statusmodel.cpp
)
target_link_libraries(tst_statusmodel PRIVATE
    Qt6::Core Qt6::Test
    dragonfruit-shell-dockcore
)

add_test(
    NAME tst_statusmodel
    COMMAND tst_statusmodel
)
set_tests_properties(tst_statusmodel PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Compositor-policy tests (T-08.2c): the pure settingsd -> compositor
# motion/input mapping the shell forwards over the private protocol. No bus,
# Wayland, or QML.
qt_add_executable(tst_compositorpolicy
    tst_compositorpolicy.cpp
)
target_link_libraries(tst_compositorpolicy PRIVATE
    Qt6::Core Qt6::Test
    dragonfruit-shell-dockcore
)

add_test(
    NAME tst_compositorpolicy
    COMMAND tst_compositorpolicy
)
set_tests_properties(tst_compositorpolicy PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Wallpaper-policy tests (T-09.3): the pure settingsd -> compositor wallpaper
# mapping the shell forwards as `df_workspace.set_wallpaper`. No bus, Wayland,
# or QML.
qt_add_executable(tst_wallpaperpolicy
    tst_wallpaperpolicy.cpp
)
target_link_libraries(tst_wallpaperpolicy PRIVATE
    Qt6::Core Qt6::Test
    dragonfruit-shell-dockcore
)

add_test(
    NAME tst_wallpaperpolicy
    COMMAND tst_wallpaperpolicy
)
set_tests_properties(tst_wallpaperpolicy PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Display-policy tests (T-09.5): the pure settingsd -> compositor output
# mapping the shell forwards as `df_output.set_scale` / `set_transform`. No
# bus, Wayland, or QML.
qt_add_executable(tst_displayspolicy
    tst_displayspolicy.cpp
)
target_link_libraries(tst_displayspolicy PRIVATE
    Qt6::Core Qt6::Test
    dragonfruit-shell-dockcore
)

add_test(
    NAME tst_displayspolicy
    COMMAND tst_displayspolicy
)
set_tests_properties(tst_displayspolicy PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Settings-client tests (T-08.2a): the typed value store and the live
# `org.dragonfruit.Settings1` client against a fake service on a private
# session bus. The bus-dependent half skips when no bus is present.
qt_add_executable(tst_settingsclient
    tst_settingsclient.cpp
)
target_link_libraries(tst_settingsclient PRIVATE
    Qt6::Core Qt6::Test Qt6::DBus
    dragonfruit-shell-dockcore
)

find_program(DBUS_RUN_SESSION dbus-run-session)
if(DBUS_RUN_SESSION)
    add_test(
        NAME tst_settingsclient
        COMMAND ${DBUS_RUN_SESSION} -- $<TARGET_FILE:tst_settingsclient>
    )
else()
    add_test(NAME tst_settingsclient COMMAND tst_settingsclient)
endif()
set_tests_properties(tst_settingsclient PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Theme-binding tests (T-08.2b): a settingsd key change (through the same
# SettingsClient the Dock uses) flips the design-system Theme, and the gallery
# variant renders the new scheme. Needs a QML engine plus a real scene graph
# (offscreen + software), so it pulls the design-system and gallery plugins and
# compiles the shell's ThemeBinding source directly (it is not part of the
# wayland-free dockcore).
qt_add_executable(tst_themebinding
    tst_themebinding.cpp
    ${CMAKE_CURRENT_SOURCE_DIR}/../src/themebinding.cpp
)
target_include_directories(tst_themebinding PRIVATE ${CMAKE_CURRENT_SOURCE_DIR}/../src)
target_link_libraries(tst_themebinding PRIVATE
    Qt6::Test Qt6::Gui Qt6::Qml Qt6::Quick
    dragonfruit-shell-dockcore
    dragonfruit-design-systemplugin
    dragonfruit-galleryplugin
)
qt_import_qml_plugins(tst_themebinding)

add_test(
    NAME tst_themebinding
    COMMAND tst_themebinding
)
set_tests_properties(tst_themebinding PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Overview chrome tests (T-11): workspace strip (incl. fullscreen Spaces),
# minimized-window strip, selection signals, and the shared progress fade.
qt_add_executable(tst_overview
    tst_overview.cpp
)
target_link_libraries(tst_overview PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-shell-overviewplugin
)
qt_import_qml_plugins(tst_overview)

add_test(
    NAME tst_overview
    COMMAND tst_overview -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_overview.qml
)
set_tests_properties(tst_overview PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# App-switcher overlay tests (T-06.2a): one card per app in recency order, the
# selection highlight, accessible names, and the reduced-motion variant.
qt_add_executable(tst_switcher
    tst_switcher.cpp
)
target_link_libraries(tst_switcher PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-shell-switcherplugin
)
qt_import_qml_plugins(tst_switcher)

add_test(
    NAME tst_switcher
    COMMAND tst_switcher -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_switcher.qml
)
set_tests_properties(tst_switcher PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Notification-model tests (T-11.1a): the service's `Banners()`/`History()`
# JSON decodes to the shell's banner and history views. No bus, Wayland, or
# QML.
qt_add_executable(tst_notificationmodel
    tst_notificationmodel.cpp
)
target_link_libraries(tst_notificationmodel PRIVATE
    Qt6::Core Qt6::Test
    dragonfruit-shell-dockcore
)

add_test(
    NAME tst_notificationmodel
    COMMAND tst_notificationmodel
)
set_tests_properties(tst_notificationmodel PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Notification chrome tests (T-11.1a): the banner card and the notification
# center render the decoded service views.
qt_add_executable(tst_notifications
    tst_notifications.cpp
)
target_link_libraries(tst_notifications PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-shell-notificationsplugin
)
qt_import_qml_plugins(tst_notifications)

add_test(
    NAME tst_notifications
    COMMAND tst_notifications -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_notifications.qml
)
set_tests_properties(tst_notifications PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
