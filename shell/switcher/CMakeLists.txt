# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-switcher STATIC)
qt_add_qml_module(dragonfruit-shell-switcher
    URI Dragonfruit.Switcher
    VERSION 1.0
    QML_FILES
        AppSwitcher.qml
)
target_link_libraries(dragonfruit-shell-switcher PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-switcher AppSwitcher.qml)