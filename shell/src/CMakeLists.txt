# SPDX-License-Identifier: MIT
# The shell process (T-09): a Wayland client of the compositor over the
# private protocol, rendering the menu bar into a chrome surface.
#
# libwayland-client is found via pkg-config (the normal system/CI path) or,
# on hosts without the -devel package, via the local Qt toolchain prefix
# (which ships the headers and linker name). The shell target is skipped
# with a warning when neither is available, so the rest of the build never
# breaks.

find_package(PkgConfig QUIET)
if(PkgConfig_FOUND)
    pkg_check_modules(WAYLAND_CLIENT IMPORTED_TARGET wayland-client)
endif()

get_filename_component(DF_QT_PREFIX "${Qt6_DIR}/../../.." ABSOLUTE)
if(NOT WAYLAND_CLIENT_FOUND)
    find_path(WAYLAND_CLIENT_INCLUDE_DIR wayland-client.h
        HINTS "${DF_QT_PREFIX}/include")
    # Some hosts ship only the runtime soname (no -devel linker name), so
    # accept `libwayland-client.so.0` as a fallback.
    find_library(WAYLAND_CLIENT_LIBRARY NAMES wayland-client libwayland-client.so.0
        HINTS "${DF_QT_PREFIX}/lib64" "${DF_QT_PREFIX}/lib"
        PATHS /usr/lib64 /usr/lib /usr/lib/x86_64-linux-gnu)
    if(WAYLAND_CLIENT_INCLUDE_DIR AND WAYLAND_CLIENT_LIBRARY)
        set(WAYLAND_CLIENT_FOUND TRUE)
    endif()
endif()

find_program(WAYLAND_SCANNER wayland-scanner
    HINTS "${DF_QT_PREFIX}/bin" "$ENV{HOME}/.local/df-toolchain/usr/bin")

if(NOT WAYLAND_CLIENT_FOUND OR NOT WAYLAND_SCANNER)
    message(WARNING
        "dragonfruit-shell: libwayland-client or wayland-scanner not found; "
        "the shell process target is skipped")
    return()
endif()

# Pure, Wayland-free Dock core (T-10): the interim `.desktop` resolver and
# launcher, `dock.pinned` persistence, and the entry merge model. Kept in a
# small static library so `tst_dockcore` can unit-test it without the
# compositor or the Wayland stack. The resolver is deleted when T-23 lands.
add_library(dragonfruit-shell-dockcore STATIC
    desktopentry.cpp
    dockdrops.cpp
    dockpins.cpp
    docksettings.cpp
    dockmodel.cpp
    dockprojection.cpp
    downloadsmonitor.cpp
    framecommitgate.h
    trashmonitor.cpp
)
target_link_libraries(dragonfruit-shell-dockcore PUBLIC Qt6::Core)
target_include_directories(dragonfruit-shell-dockcore PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")

# The lockstep version is a single contract; read it from df-ipc rather than
# duplicating it in C++.
file(READ "${CMAKE_SOURCE_DIR}/protocols/df-ipc/src/lib.rs" DF_IPC_LIB)
string(REGEX MATCH "LOCKSTEP_VERSION: u32 = ([0-9]+)" _df_version_match "${DF_IPC_LIB}")
set(DF_LOCKSTEP_VERSION "${CMAKE_MATCH_1}")

# Generate the plain C client bindings for the private protocols. We always
# use wayland-scanner (not qtwaylandscanner) so the generated API is stable
# across Qt installations.
set(DF_PROTOCOL_DIR "${CMAKE_BINARY_DIR}/protocols")
set(DF_PROTOCOL_SOURCES)
foreach(proto dragonfruit-core dragonfruit-shell dragonfruit-toplevel)
    set(_xml "${CMAKE_SOURCE_DIR}/protocols/${proto}.xml")
    set(_header "${DF_PROTOCOL_DIR}/${proto}-client-protocol.h")
    set(_source "${DF_PROTOCOL_DIR}/${proto}-protocol.c")
    add_custom_command(
        OUTPUT "${_header}" "${_source}"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${DF_PROTOCOL_DIR}"
        COMMAND "${WAYLAND_SCANNER}" client-header "${_xml}" "${_header}"
        COMMAND "${WAYLAND_SCANNER}" private-code "${_xml}" "${_source}"
        DEPENDS "${_xml}"
        COMMENT "wayland-scanner ${proto}"
        VERBATIM)
    list(APPEND DF_PROTOCOL_SOURCES "${_source}")
endforeach()

qt_add_executable(dragonfruit-shell
    main.cpp
    shellcontroller.cpp
    shellprotocol.cpp
)
target_sources(dragonfruit-shell PRIVATE ${DF_PROTOCOL_SOURCES})
target_include_directories(dragonfruit-shell PRIVATE "${DF_PROTOCOL_DIR}")
target_compile_definitions(dragonfruit-shell PRIVATE
    DF_LOCKSTEP_VERSION=${DF_LOCKSTEP_VERSION}
    DF_QML_IMPORT_DIR="${CMAKE_BINARY_DIR}/qml")
target_link_libraries(dragonfruit-shell PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-shell-dockcore
    dragonfruit-design-systemplugin
    dragonfruit-shell-menubarplugin
    dragonfruit-shell-dockplugin
    dragonfruit-shell-overviewplugin
    dragonfruit-shell-switcherplugin
)
if(TARGET PkgConfig::WAYLAND_CLIENT)
    target_link_libraries(dragonfruit-shell PRIVATE PkgConfig::WAYLAND_CLIENT)
else()
    target_include_directories(dragonfruit-shell PRIVATE "${WAYLAND_CLIENT_INCLUDE_DIR}")
    target_link_libraries(dragonfruit-shell PRIVATE "${WAYLAND_CLIENT_LIBRARY}")
endif()
qt_import_qml_plugins(dragonfruit-shell)
