# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-osd STATIC)
qt_add_qml_module(dragonfruit-shell-osd
    URI Dragonfruit.Osd
    VERSION 1.0
    QML_FILES
        Osd.qml
)
target_link_libraries(dragonfruit-shell-osd PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-osd Osd.qml)