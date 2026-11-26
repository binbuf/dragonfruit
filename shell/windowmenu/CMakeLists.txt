# SPDX-License-Identifier: GPL-3.0-or-later
# The SSD titlebar window menu's readable scene (T329). The compositor owns
# the menu model and input; this module is a pure QML presentation rendered
# into the shell's transient `windowmenu` overlay surface.
qt_add_library(dragonfruit-shell-windowmenu STATIC)
qt_add_qml_module(dragonfruit-shell-windowmenu
    URI Dragonfruit.WindowMenu
    VERSION 1.0
    QML_FILES
        WindowMenu.qml
)
target_link_libraries(dragonfruit-shell-windowmenu PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-windowmenu WindowMenu.qml)
