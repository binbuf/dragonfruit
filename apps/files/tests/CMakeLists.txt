# SPDX-License-Identifier: MIT
# Files shell tests (T-10.4a): the window opens on Home, the sidebar changes
# the browsing model, toolbar history works, the view switch persists per
# location, and the AT-SPI roles are present. `DF_FILES_FIXTURE` forces a
# deterministic location set, so the cases do not depend on the host's home
# directory or mounts. Headless offscreen + software scene graph.
qt_add_executable(tst_files_shell
    tst_files_shell.cpp
)
target_link_libraries(tst_files_shell PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-files-uiplugin
)
qt_import_qml_plugins(tst_files_shell)

add_test(
    NAME tst_files_shell
    COMMAND tst_files_shell -input ${CMAKE_CURRENT_SOURCE_DIR}/tst_files_shell.qml
)
set_tests_properties(tst_files_shell PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;DF_FILES_FIXTURE=1;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)