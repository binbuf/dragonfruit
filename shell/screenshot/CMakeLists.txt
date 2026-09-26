# SPDX-License-Identifier: MIT
# The portal pickers and capture UI (T-13). The FileChooser picker
# (T-13.2b), the screenshot selection overlay (T-13.3a), and the ScreenCast
# source picker (T-13.4a) live here.
qt_add_library(dragonfruit-shell-screenshot STATIC)
qt_add_qml_module(dragonfruit-shell-screenshot
    URI Dragonfruit.Screenshot
    VERSION 1.0
    QML_FILES
        FileChooser.qml
        ScreenCastPicker.qml
        SelectionOverlay.qml
)
target_link_libraries(dragonfruit-shell-screenshot PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-screenshot FileChooser.qml ScreenCastPicker.qml SelectionOverlay.qml)