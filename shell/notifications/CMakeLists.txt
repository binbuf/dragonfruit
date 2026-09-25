# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-notifications STATIC)
qt_add_qml_module(dragonfruit-shell-notifications
    URI Dragonfruit.Notifications
    VERSION 1.0
    QML_FILES
        NotificationBanner.qml
        NotificationCenter.qml
)
target_link_libraries(dragonfruit-shell-notifications PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-notifications NotificationBanner.qml NotificationCenter.qml)