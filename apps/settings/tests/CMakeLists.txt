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