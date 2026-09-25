# SPDX-License-Identifier: MIT
# Settings (T-09.1a): the app shell is a reusable QML module so the app and
# its headless QML test render the exact same surfaces — mirroring the
# design-system gallery/module split. The executable only loads the window.

set_source_files_properties(SettingsPanes.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

qt_add_library(dragonfruit-settings-ui STATIC)
qt_add_qml_module(dragonfruit-settings-ui
    URI Dragonfruit.Settings
    VERSION 1.0
    SOURCES
        SettingsBridge.cpp
        SettingsBridge.h
    QML_FILES
        SettingsPanes.qml
        PaneHeader.qml
        AppearancePane.qml
        WallpaperPane.qml
        DesktopDockPane.qml
        DisplaysPane.qml
        SettingsShell.qml
        SettingsWindow.qml
)
target_link_libraries(dragonfruit-settings-ui PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
    dragonfruit-settings-client
)

qt_add_executable(dragonfruit-settings
    main.cpp
)
target_link_libraries(dragonfruit-settings PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(dragonfruit-settings)

df_qml_lint(app-settings
    SettingsPanes.qml
    PaneHeader.qml
    AppearancePane.qml
    WallpaperPane.qml
    DesktopDockPane.qml
    DisplaysPane.qml
    SettingsShell.qml
    SettingsWindow.qml
)

add_subdirectory(tests)