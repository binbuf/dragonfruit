# SPDX-License-Identifier: MIT
# The first-party lock screen (T-12.3a): a pure QML view the shell controller
# renders into the compositor's `ext-session-lock-v1` surfaces.
qt_add_library(dragonfruit-shell-lock STATIC)
qt_add_qml_module(dragonfruit-shell-lock
    URI Dragonfruit.Lock
    VERSION 1.0
    QML_FILES
        LockScreen.qml
)
target_link_libraries(dragonfruit-shell-lock PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-lock LockScreen.qml)