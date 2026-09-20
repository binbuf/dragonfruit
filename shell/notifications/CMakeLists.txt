# SPDX-License-Identifier: MIT
qt_add_qml_module(dragonfruit-shell-notifications
    URI Dragonfruit.Notifications
    VERSION 1.0
    QML_FILES
        NotificationCenter.qml
)

df_qml_lint(shell-notifications NotificationCenter.qml)
