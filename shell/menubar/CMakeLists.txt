# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-menubar STATIC)
qt_add_qml_module(dragonfruit-shell-menubar
    URI Dragonfruit.MenuBar
    VERSION 1.0
    QML_FILES
        MenuBar.qml
        StatusItem.qml
        StatusGlyph.qml
        MenuBarClock.qml
        WifiMenu.qml
        VolumeMenu.qml
)
target_link_libraries(dragonfruit-shell-menubar PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-menubar
    MenuBar.qml
    StatusItem.qml
    StatusGlyph.qml
    MenuBarClock.qml
    WifiMenu.qml
    VolumeMenu.qml
)
