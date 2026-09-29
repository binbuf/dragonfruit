# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-apps-drawer STATIC)
qt_add_qml_module(dragonfruit-shell-apps-drawer
    URI Dragonfruit.AppsDrawer
    VERSION 1.0
    QML_FILES
        AppsDrawer.qml
)
target_link_libraries(dragonfruit-shell-apps-drawer PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-apps-drawer AppsDrawer.qml)