# SPDX-License-Identifier: MIT
# Files (T-10.4a/T-10.4b): the app shell is a reusable QML module so the app
# and its headless QML test render the exact same surfaces — mirroring the
# Settings shell/gallery split. The executable only loads the window.

# The files-core bridge (T-10.4b, ADR 0049): the Rust crate is built as a
# static library exposing a small C ABI (`ffi/files_core.h`), which
# `FilesDirectoryModel` links. The imported target is defined once at the top
# level so the shell's Dock Trash bridge (T-10.6a) shares it.

qt_add_library(dragonfruit-files-ui STATIC)
qt_add_qml_module(dragonfruit-files-ui
    URI Dragonfruit.Files
    VERSION 1.0
    SOURCES
        FilesBridge.cpp
        FilesBridge.h
        FilesDirectoryModel.cpp
        FilesDirectoryModel.h
    QML_FILES
        FilesBrowser.qml
        FilesIconView.qml
        FilesListView.qml
        PathBar.qml
        FilesShell.qml
        FilesWindow.qml
)
target_link_libraries(dragonfruit-files-ui PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
    dragonfruit-files-core
)

qt_add_executable(dragonfruit-files
    main.cpp
)
target_link_libraries(dragonfruit-files PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick Qt6::DBus
    dragonfruit-system-font
    dragonfruit-files-uiplugin
)
qt_import_qml_plugins(dragonfruit-files)

# The app identity (T-10.6c): install the `.desktop` entry so the Dock's
# default pins resolve and the folder MIME type routes to Files, and the
# D-Bus activation service for `org.dragonfruit.Files1`.
include(GNUInstallDirs)
install(FILES org.dragonfruit.Files.desktop
    DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
install(FILES org.dragonfruit.Files1.service
    DESTINATION ${CMAKE_INSTALL_DATADIR}/dbus-1/services)

df_qml_lint(app-files
    FilesBrowser.qml
    FilesIconView.qml
    FilesListView.qml
    PathBar.qml
    FilesShell.qml
    FilesWindow.qml
)

add_subdirectory(tests)