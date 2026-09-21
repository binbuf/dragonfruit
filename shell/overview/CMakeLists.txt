# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-overview STATIC)
qt_add_qml_module(dragonfruit-shell-overview
    URI Dragonfruit.Overview
    VERSION 1.0
    QML_FILES
        Overview.qml
)
target_link_libraries(dragonfruit-shell-overview PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-overview Overview.qml)
