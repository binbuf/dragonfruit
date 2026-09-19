# SPDX-License-Identifier: GPL-3.0-or-later
qt_add_qml_module(dragonfruit-shell-control-center
    URI Dragonfruit.ControlCenter
    VERSION 1.0
    QML_FILES
        ControlCenter.qml
)

df_qml_lint(shell-control-center ControlCenter.qml)
