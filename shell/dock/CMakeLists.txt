# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-dock STATIC)
qt_add_qml_module(dragonfruit-shell-dock
    URI Dragonfruit.Dock
    VERSION 1.0
    QML_FILES
        Dock.qml
        DockEntry.qml
        DockGlyph.qml
)
target_link_libraries(dragonfruit-shell-dock PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-dock Dock.qml DockEntry.qml DockGlyph.qml)
