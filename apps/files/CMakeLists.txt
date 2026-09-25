# SPDX-License-Identifier: MIT
# Files (T-10.4a/T-10.4b): the app shell is a reusable QML module so the app
# and its headless QML test render the exact same surfaces — mirroring the
# Settings shell/gallery split. The executable only loads the window.

# The files-core bridge (T-10.4b, ADR 0049): the Rust crate is built as a
# static library exposing a small C ABI (`ffi/files_core.h`), which
# `FilesDirectoryModel` links. Cargo is invoked from CMake so a standalone
# `make cmake-build` also produces it; the target always runs cargo, which is
# a no-op once the crate is up to date.
find_program(DF_CARGO_EXECUTABLE cargo REQUIRED)
set(DF_FILES_CORE_STATICLIB
    "${CMAKE_SOURCE_DIR}/target/debug/libdragonfruit_files_core.a")
add_custom_target(dragonfruit-files-core-ffi
    COMMAND ${DF_CARGO_EXECUTABLE} build -p dragonfruit-files-core
    WORKING_DIRECTORY ${CMAKE_SOURCE_DIR}
    COMMENT "Building dragonfruit-files-core staticlib (T-10.4b bridge)"
    VERBATIM
)
add_library(dragonfruit-files-core STATIC IMPORTED GLOBAL)
set_target_properties(dragonfruit-files-core PROPERTIES
    IMPORTED_LOCATION "${DF_FILES_CORE_STATICLIB}"
    INTERFACE_LINK_LIBRARIES "pthread;dl;m"
)
add_dependencies(dragonfruit-files-core dragonfruit-files-core-ffi)

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
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-files-uiplugin
)
qt_import_qml_plugins(dragonfruit-files)

df_qml_lint(app-files
    FilesBrowser.qml
    FilesIconView.qml
    FilesListView.qml
    PathBar.qml
    FilesShell.qml
    FilesWindow.qml
)

add_subdirectory(tests)