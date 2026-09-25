# SPDX-License-Identifier: MIT
# Files (T-10.4a): the app shell is a reusable QML module so the app and its
# headless QML test render the exact same surfaces — mirroring the Settings
# shell/gallery split. The executable only loads the window.

qt_add_library(dragonfruit-files-ui STATIC)
qt_add_qml_module(dragonfruit-files-ui
    URI Dragonfruit.Files
    VERSION 1.0
    SOURCES
        FilesBridge.cpp
        FilesBridge.h
    QML_FILES
        FilesBrowser.qml
        PathBar.qml
        FilesShell.qml
        FilesWindow.qml
)
target_link_libraries(dragonfruit-files-ui PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

qt_add_executable(dragonfruit-files
    main.cpp
)
target_link_libraries(dragonfruit-files PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-files-uiplugin
)
qt_import_qml_plugins(dragonfruit-files)

df_qml_lint(app-files
    FilesBrowser.qml
    PathBar.qml
    FilesShell.qml
    FilesWindow.qml
)

add_subdirectory(tests)