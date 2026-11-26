# SPDX-License-Identifier: GPL-3.0-only
# The session power confirmation dialog: a pure QML view the shell controller
# renders into its own centered `power-dialog` overlay surface. The controller
# owns the logind call; this scene only asks, counts down, and reports the
# user's choice.
qt_add_library(dragonfruit-shell-power STATIC)
qt_add_qml_module(dragonfruit-shell-power
    URI Dragonfruit.Power
    VERSION 1.0
    QML_FILES
        PowerDialog.qml
)
target_link_libraries(dragonfruit-shell-power PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-power PowerDialog.qml)
