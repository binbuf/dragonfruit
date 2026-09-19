# SPDX-License-Identifier: GPL-3.0-or-later
qt_add_qml_module(dragonfruit-shell-menubar
    URI Dragonfruit.MenuBar
    VERSION 1.0
    QML_FILES
        MenuBar.qml
)

df_qml_lint(shell-menubar MenuBar.qml)
