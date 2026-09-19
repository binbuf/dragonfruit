# SPDX-License-Identifier: LGPL-3.0-or-later
# Design-system tests (T-08): token pipeline, per-component keyboard/AT-SPI
# coverage, reduced-motion variants, and the FR-3 screenshot diff between the
# app TitleBar and the compositor SSD reference.

qt_add_executable(tst_design_system
    tst_design_system.cpp
)
target_link_libraries(tst_design_system PRIVATE
    Qt6::QuickTest
    dragonfruit-design-systemplugin
    dragonfruit-galleryplugin
)
qt_import_qml_plugins(tst_design_system)

add_test(
    NAME tst_design_system
    COMMAND tst_design_system -input ${CMAKE_CURRENT_SOURCE_DIR}
)
set_tests_properties(tst_design_system PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software;QML2_IMPORT_PATH=${CMAKE_BINARY_DIR}/qml"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)
