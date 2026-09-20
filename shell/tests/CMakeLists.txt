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
    COMMAND tst_menubar -input ${CMAKE_CURRENT_SOURCE_DIR}
)
set_tests_properties(tst_menubar PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
