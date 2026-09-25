# SPDX-License-Identifier: MIT
# Settings shell tests (T-09.1a): the shell opens, local search filters the
# pane list, keyboard navigation and history work, and the AT-SPI roles are
# present. Headless offscreen + software scene graph, like every QML test.

qt_add_executable(tst_settings_shell
    tst_settings_shell.cpp
)
target_link_libraries(tst_settings_shell PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(tst_settings_shell)

add_test(
    NAME tst_settings_shell
    COMMAND tst_settings_shell -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_shell.qml
)
set_tests_properties(tst_settings_shell PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Appearance-pane tests (T-09.2): every control applies live to the settings
# key and the app-local Theme. `DF_SETTINGS_FIXTURE` forces the deterministic
# in-process mock, so the cases never touch the host session bus.
qt_add_executable(tst_settings_appearance
    tst_settings_appearance.cpp
)
target_link_libraries(tst_settings_appearance PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(tst_settings_appearance)

add_test(
    NAME tst_settings_appearance
    COMMAND tst_settings_appearance
        -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_appearance.qml
)
set_tests_properties(tst_settings_appearance PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;DF_SETTINGS_FIXTURE=1;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Wallpaper-pane tests (T-09.3): the built-in artwork selection, "Show on all
# Spaces", and fit apply live to the settings key, and an external change
# converges into the preview and selection. `DF_SETTINGS_FIXTURE` forces the
# deterministic in-process mock, so the cases never touch the host bus.
qt_add_executable(tst_settings_wallpaper
    tst_settings_wallpaper.cpp
)
target_link_libraries(tst_settings_wallpaper PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(tst_settings_wallpaper)

add_test(
    NAME tst_settings_wallpaper
    COMMAND tst_settings_wallpaper
        -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_wallpaper.qml
)
set_tests_properties(tst_settings_wallpaper PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;DF_SETTINGS_FIXTURE=1;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Desktop & Dock-pane tests (T-09.4): every `dock.*` control applies live to its
# settings key and an external change converges. `DF_SETTINGS_FIXTURE` forces
# the deterministic in-process mock, so the cases never touch the host bus.
qt_add_executable(tst_settings_desktop_dock
    tst_settings_desktop_dock.cpp
)
target_link_libraries(tst_settings_desktop_dock PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(tst_settings_desktop_dock)

add_test(
    NAME tst_settings_desktop_dock
    COMMAND tst_settings_desktop_dock
        -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_desktop_dock.qml
)
set_tests_properties(tst_settings_desktop_dock PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;DF_SETTINGS_FIXTURE=1;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Displays-pane tests (T-09.5): the resolution tiles and rotation select apply
# live to their `display.*` key and an external change converges. The
# `DF_SETTINGS_FIXTURE` mock keeps the cases off the host bus.
qt_add_executable(tst_settings_displays
    tst_settings_displays.cpp
)
target_link_libraries(tst_settings_displays PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(tst_settings_displays)

add_test(
    NAME tst_settings_displays
    COMMAND tst_settings_displays
        -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_displays.qml
)
set_tests_properties(tst_settings_displays PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;DF_SETTINGS_FIXTURE=1;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Settings menu-model publication tests (T-09.6a): the app's published menu
# model is asserted in shape and injected into the shell's `MenuBar` (the
# `Dragonfruit.MenuBar` module is linked so the round-trip is real). Headless
# offscreen + software scene graph; no bus and no daemon.
qt_add_executable(tst_settings_menu
    tst_settings_menu.cpp
)
target_link_libraries(tst_settings_menu PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
    dragonfruit-shell-menubarplugin
)
qt_import_qml_plugins(tst_settings_menu)

add_test(
    NAME tst_settings_menu
    COMMAND tst_settings_menu -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_menu.qml
)
set_tests_properties(tst_settings_menu PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Settings live-apply tests (T-09.1b): a representative control is bound to a
# settingsd key through the QML `Settings` singleton and round-trips it. The
# bus-backed cases run under a private session bus; the real-daemon case uses
# the built `dragonfruit-settingsd` when CMake found it (else it skips).
qt_add_executable(tst_settings_live
    tst_settings_live.cpp
)
target_compile_definitions(tst_settings_live PRIVATE
    DF_QML_IMPORT_DIR="${CMAKE_BINARY_DIR}/qml"
)
target_link_libraries(tst_settings_live PRIVATE
    Qt6::QuickTest Qt6::Quick Qt6::Qml Qt6::DBus
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
    dragonfruit-settings-client
)
qt_import_qml_plugins(tst_settings_live)

find_program(DF_SETTINGSD_BIN NAMES dragonfruit-settingsd
    HINTS ${CMAKE_SOURCE_DIR}/target/debug ${CMAKE_SOURCE_DIR}/target/release)

find_program(DBUS_RUN_SESSION dbus-run-session)
if(DBUS_RUN_SESSION)
    add_test(
        NAME tst_settings_live
        COMMAND ${DBUS_RUN_SESSION} -- $<TARGET_FILE:tst_settings_live>
    )
else()
    add_test(NAME tst_settings_live COMMAND tst_settings_live)
endif()
set(_settings_live_env
    "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml")
if(DF_SETTINGSD_BIN)
    list(APPEND _settings_live_env "DF_SETTINGSD_BIN=${DF_SETTINGSD_BIN}")
endif()
set_tests_properties(tst_settings_live PROPERTIES
    ENVIRONMENT "${_settings_live_env}"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)

# Absent-provider matrix tests (T-09.6b): with no settingsd and no
# xdg-desktop-portal, every shipped pane still renders and every control stays
# functional on the schema defaults, and only a pane with a real body is
# advertised (the no-half-panes rule). `dbus-run-session` gives a private bus
# with neither provider; `DF_SETTINGS_FIXTURE` is deliberately **not** set, so
# the `Settings` singleton is the live `DbusSettingsClient` and reports
# `available == false`.
qt_add_executable(tst_settings_absence
    tst_settings_absence.cpp
)
target_link_libraries(tst_settings_absence PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(tst_settings_absence)

if(DBUS_RUN_SESSION)
    add_test(
        NAME tst_settings_absence
        COMMAND ${DBUS_RUN_SESSION} -- $<TARGET_FILE:tst_settings_absence>
            -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_absence.qml
    )
else()
    add_test(
        NAME tst_settings_absence
        COMMAND tst_settings_absence
            -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_settings_absence.qml
    )
endif()
set_tests_properties(tst_settings_absence PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)