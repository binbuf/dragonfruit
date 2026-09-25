# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-control-center STATIC)
qt_add_qml_module(dragonfruit-shell-control-center
    URI Dragonfruit.ControlCenter
    VERSION 1.0
    QML_FILES
        ControlCenter.qml
)
target_link_libraries(dragonfruit-shell-control-center PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-control-center ControlCenter.qml)