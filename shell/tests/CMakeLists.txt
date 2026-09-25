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
