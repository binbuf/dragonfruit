# SPDX-License-Identifier: MIT
# The portal pickers and capture UI (T-13). The FileChooser picker
# (T-13.2b) is the first; the screenshot selection overlay follows in T-13.3a.
qt_add_library(dragonfruit-shell-screenshot STATIC)
qt_add_qml_module(dragonfruit-shell-screenshot
    URI Dragonfruit.Screenshot
    VERSION 1.0
    QML_FILES
        ScreenshotOverlay.qml
        FileChooser.qml
)
target_link_libraries(dragonfruit-shell-screenshot PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-screenshot ScreenshotOverlay.qml FileChooser.qml)