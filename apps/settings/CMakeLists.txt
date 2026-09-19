# SPDX-License-Identifier: GPL-3.0-or-later
qt_add_executable(dragonfruit-settings
    main.cpp
)
qt_add_qml_module(dragonfruit-settings
    URI Dragonfruit.Settings
    VERSION 1.0
    QML_FILES
        SettingsWindow.qml
)
target_link_libraries(dragonfruit-settings PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(app-settings SettingsWindow.qml)
