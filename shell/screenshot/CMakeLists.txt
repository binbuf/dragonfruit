# SPDX-License-Identifier: MIT
qt_add_qml_module(dragonfruit-shell-screenshot
    URI Dragonfruit.Screenshot
    VERSION 1.0
    QML_FILES
        ScreenshotOverlay.qml
)

df_qml_lint(shell-screenshot ScreenshotOverlay.qml)
