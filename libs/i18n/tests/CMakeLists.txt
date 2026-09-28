# SPDX-License-Identifier: MIT
# Localization tests: the `.ts` parser, locale resolution, catalog search, the
# application install path, the default-`QLocale` number/date formatting, and
# a real QML `qsTr()` lookup through an installed catalog.

qt_add_executable(tst_i18n
    tst_i18n.cpp
)
target_link_libraries(tst_i18n PRIVATE
    Qt6::Core Qt6::Qml Qt6::Test
    dragonfruit-i18n
)
target_compile_definitions(tst_i18n PRIVATE
    DF_TRANSLATIONS_DIR="${CMAKE_SOURCE_DIR}/translations")

add_test(
    NAME tst_i18n
    COMMAND tst_i18n
)
set_tests_properties(tst_i18n PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen"
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
)