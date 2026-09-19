# SPDX-License-Identifier: GPL-3.0-or-later
qt_add_qml_module(dragonfruit-shell-notifications
    URI Dragonfruit.Notifications
    VERSION 1.0
    QML_FILES
        NotificationCenter.qml
)

df_qml_lint(shell-notifications NotificationCenter.qml)
