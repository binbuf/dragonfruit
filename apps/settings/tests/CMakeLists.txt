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