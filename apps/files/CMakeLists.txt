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
        DesktopSurface.qml
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
    dragonfruit-i18n
    dragonfruit-files-uiplugin
)
qt_import_qml_plugins(dragonfruit-files)

# The Files-owned desktop process (T-19.3) is a trusted private-protocol
# client, exactly like the shell: it speaks df_core/df_shell directly instead
# of using the Qt Wayland platform plugin. libwayland-client is found the same
# way the shell finds it (pkg-config, else the local Qt toolchain prefix); when
# it is absent the browser still builds and `--desktop` reports the missing
# client instead of linking.
find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
    pkg_check_modules(FILES_WAYLAND_CLIENT IMPORTED_TARGET wayland-client)
endif()

get_filename_component(DF_QT_PREFIX "${Qt6_DIR}/../../.." ABSOLUTE)
if(NOT FILES_WAYLAND_CLIENT_FOUND)
    find_path(FILES_WAYLAND_INCLUDE_DIR wayland-client.h HINTS "${DF_QT_PREFIX}/include")
    find_library(FILES_WAYLAND_LIBRARY NAMES wayland-client libwayland-client.so.0
        HINTS "${DF_QT_PREFIX}/lib64" "${DF_QT_PREFIX}/lib"
        PATHS /usr/lib64 /usr/lib /usr/lib/x86_64-linux-gnu)
    if(FILES_WAYLAND_INCLUDE_DIR AND FILES_WAYLAND_LIBRARY)
        set(FILES_WAYLAND_CLIENT_FOUND TRUE)
    endif()
endif()
find_program(FILES_WAYLAND_SCANNER wayland-scanner
    HINTS "${DF_QT_PREFIX}/bin" "$ENV{HOME}/.local/df-toolchain/usr/bin")

if(FILES_WAYLAND_CLIENT_FOUND AND FILES_WAYLAND_SCANNER)
    # Generate the private protocol client bindings into a Files-local dir, so
    # the shell and Files never race on one custom-command output.
    set(DF_FILES_PROTOCOL_DIR "${CMAKE_BINARY_DIR}/files-protocols")
    set(DF_FILES_PROTOCOL_SOURCES)
    foreach(proto dragonfruit-core dragonfruit-shell)
        set(_xml "${CMAKE_SOURCE_DIR}/protocols/${proto}.xml")
        set(_header "${DF_FILES_PROTOCOL_DIR}/${proto}-client-protocol.h")
        set(_source "${DF_FILES_PROTOCOL_DIR}/${proto}-protocol.c")
        add_custom_command(
            OUTPUT "${_header}" "${_source}"
            COMMAND ${CMAKE_COMMAND} -E make_directory "${DF_FILES_PROTOCOL_DIR}"
            COMMAND "${FILES_WAYLAND_SCANNER}" client-header "${_xml}" "${_header}"
            COMMAND "${FILES_WAYLAND_SCANNER}" private-code "${_xml}" "${_source}"
            DEPENDS "${_xml}"
            COMMENT "wayland-scanner ${proto} (files desktop)"
            VERBATIM)
        list(APPEND DF_FILES_PROTOCOL_SOURCES "${_source}")
    endforeach()

    # The lockstep version is a single contract; read it from df-ipc.
    file(READ "${CMAKE_SOURCE_DIR}/protocols/df-ipc/src/lib.rs" DF_IPC_LIB)
    string(REGEX MATCH "LOCKSTEP_VERSION: u32 = ([0-9]+)" _df_version_match "${DF_IPC_LIB}")
    set(DF_FILES_LOCKSTEP_VERSION "${CMAKE_MATCH_1}")

    target_sources(dragonfruit-files PRIVATE
        DesktopProtocol.cpp
        DesktopProtocol.h
        DesktopInput.h
        FilesDesktop.cpp
        FilesDesktop.h
        ${DF_FILES_PROTOCOL_SOURCES}
    )
    target_include_directories(dragonfruit-files PRIVATE "${DF_FILES_PROTOCOL_DIR}")
    target_compile_definitions(dragonfruit-files PRIVATE
        DF_FILES_DESKTOP_CLIENT=1
        DF_LOCKSTEP_VERSION=${DF_FILES_LOCKSTEP_VERSION}
    )
    target_link_libraries(dragonfruit-files PRIVATE dragonfruit-chromepointer)
    if(TARGET PkgConfig::FILES_WAYLAND_CLIENT)
        target_link_libraries(dragonfruit-files PRIVATE PkgConfig::FILES_WAYLAND_CLIENT)
    else()
        target_include_directories(dragonfruit-files PRIVATE "${FILES_WAYLAND_INCLUDE_DIR}")
        target_link_libraries(dragonfruit-files PRIVATE "${FILES_WAYLAND_LIBRARY}")
    endif()
else()
    message(WARNING
        "dragonfruit-files: libwayland-client or wayland-scanner not found; "
        "the --desktop process client is not built")
endif()

# The app identity (T-10.6c): install the `.desktop` entry so the Dock's
# default pins resolve and the folder MIME type routes to Files, and the
# D-Bus activation service for `org.dragonfruit.Files1`.
include(GNUInstallDirs)
install(FILES org.dragonfruit.Files.desktop
    DESTINATION ${CMAKE_INSTALL_DATADIR}/applications)
install(FILES org.dragonfruit.Files1.service
    DESTINATION ${CMAKE_INSTALL_DATADIR}/dbus-1/services)

# The first-party app icon (T-19.1d), generated from Phosphor by
# `scripts/gen-app-icons.py`, installs into hicolor so the `.desktop` `Icon=`
# resolves for launchers, menus, and the portal file chooser — the same
# artwork the Dock bundles at `qrc:/icons/apps/`.
install(FILES "${CMAKE_SOURCE_DIR}/assets/icons/apps/org.dragonfruit.Files.svg"
    DESTINATION ${CMAKE_INSTALL_DATADIR}/icons/hicolor/scalable/apps)

df_qml_lint(app-files
    FilesBrowser.qml
    FilesIconView.qml
    FilesListView.qml
    PathBar.qml
    FilesShell.qml
    FilesWindow.qml
    DesktopSurface.qml
)

add_subdirectory(tests)