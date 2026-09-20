# SPDX-License-Identifier: MIT
qt_add_qml_module(dragonfruit-shell-dock
    URI Dragonfruit.Dock
    VERSION 1.0
    QML_FILES
        Dock.qml
)

df_qml_lint(shell-dock Dock.qml)
