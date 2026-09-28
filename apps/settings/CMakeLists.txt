# SPDX-License-Identifier: MIT
# Settings (T-09.1a): the app shell is a reusable QML module so the app and
# its headless QML test render the exact same surfaces — mirroring the
# design-system gallery/module split. The executable only loads the window.

set_source_files_properties(SettingsPanes.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
set_source_files_properties(SettingsMenu.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

qt_add_library(dragonfruit-settings-ui STATIC)
qt_add_qml_module(dragonfruit-settings-ui
    URI Dragonfruit.Settings
    VERSION 1.0
    SOURCES
        SettingsBridge.cpp
        SettingsBridge.h
        BluetoothClient.cpp
        BluetoothClient.h
        StorageClient.cpp
        StorageClient.h
        SoundClient.cpp
        SoundClient.h
        InputClient.cpp
        InputClient.h
        BatteryClient.cpp
        BatteryClient.h
        NotificationsClient.cpp
        NotificationsClient.h
        UpdatesClient.cpp
        UpdatesClient.h
        AccountsClient.cpp
        AccountsClient.h
        PrintersClient.cpp
        PrintersClient.h
        PrivacyClient.cpp
        PrivacyClient.h
        AccessibilityClient.cpp
        AccessibilityClient.h
        VpnClient.cpp
        VpnClient.h
    QML_FILES
        SettingsPanes.qml
        SettingsMenu.qml
        PaneHeader.qml
        AppearancePane.qml
        WallpaperPane.qml
        DesktopDockPane.qml
        MissionControlPane.qml
        BatteryPane.qml
        DisplaysPane.qml
        BluetoothPane.qml
        StoragePane.qml
        SoundPane.qml
        InputPane.qml
        KeyboardPane.qml
        MousePane.qml
        TrackpadPane.qml
        NotificationsPane.qml
        FocusPane.qml
        LockScreenPane.qml
        MenuBarPane.qml
        GeneralPane.qml
        UsersGroupsPane.qml
        PrintersScannersPane.qml
        PrivacyPane.qml
        AccessibilityPane.qml
        NetworkPane.qml
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
    dragonfruit-system-font
    dragonfruit-i18n
    dragonfruit-settings-uiplugin
)
qt_import_qml_plugins(dragonfruit-settings)

# The app identity (T-09 follow-up): install the `.desktop` entry so the Dock's
# default pins resolve and the shell can launch Settings from the system menu /
# Dock through the same `.desktop` path as Files.
include(GNUInstallDirs)
install(FILES org.dragonfruit.Settings.desktop
    DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)

df_qml_lint(app-settings
    SettingsPanes.qml
    SettingsMenu.qml
    PaneHeader.qml
    AppearancePane.qml
    WallpaperPane.qml
    DesktopDockPane.qml
    MissionControlPane.qml
    BatteryPane.qml
    DisplaysPane.qml
    BluetoothPane.qml
    StoragePane.qml
    SoundPane.qml
    InputPane.qml
    KeyboardPane.qml
    MousePane.qml
    TrackpadPane.qml
    NotificationsPane.qml
    FocusPane.qml
    LockScreenPane.qml
    MenuBarPane.qml
    GeneralPane.qml
    UsersGroupsPane.qml
    PrintersScannersPane.qml
    PrivacyPane.qml
    AccessibilityPane.qml
    NetworkPane.qml
    SettingsShell.qml
    SettingsWindow.qml
)

add_subdirectory(tests)