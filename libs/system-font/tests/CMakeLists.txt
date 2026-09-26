# SPDX-License-Identifier: MIT
# System-font tests: the bundled Inter faces register, the application
# default is the Inter family, weight/italic variants resolve to it instead of
# a fallback, and repeat installation is a no-op.

qt_add_executable(tst_systemfont
    tst_systemfont.cpp
)
target_link_libraries(tst_systemfont PRIVATE
    Qt6::Gui Qt6::Qml Qt6::Quick Qt6::Test
    dragonfruit-system-font
)

add_test(
    NAME tst_systemfont
    COMMAND tst_systemfont
)
set_tests_properties(tst_systemfont PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen;QT_QUICK_BACKEND=software"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)