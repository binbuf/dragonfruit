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

# EGL is used by the T-21.1d capability probe and (T186b) the dmabuf export.
# The Qt toolchain ships the headers and linker name on hosts without the
# system -devel package.
find_path(EGL_INCLUDE_DIR EGL/egl.h
    HINTS "${DF_QT_PREFIX}/include"
    PATHS /usr/include /usr/local/include)
find_library(EGL_LIBRARY NAMES EGL
    HINTS "${DF_QT_PREFIX}/lib64" "${DF_QT_PREFIX}/lib"
    PATHS /usr/lib64 /usr/lib /usr/lib/x86_64-linux-gnu)

if(NOT WAYLAND_CLIENT_FOUND OR NOT WAYLAND_SCANNER)
    message(WARNING
        "dragonfruit-shell: libwayland-client or wayland-scanner not found; "
        "the shell process target is skipped")
    return()
endif()

if(NOT EGL_INCLUDE_DIR OR NOT EGL_LIBRARY)
    message(FATAL_ERROR
        "dragonfruit-shell: EGL headers/library not found; the T-21.1d GPU "
        "probe and dmabuf export need EGL")
endif()

# Pure, Wayland-free Dock core (T-10): the app-index entry cache and launcher,
# `dock.pinned` persistence, and the entry merge model. Kept in a small static
# library so `tst_dockcore` can unit-test it without the compositor or the
# Wayland stack. The interim `.desktop` resolver was retired in T-14.7.
add_library(dragonfruit-shell-dockcore STATIC
    appindexclient.cpp
    apppicker.cpp
    appsdrawer.cpp
    chooserbridge.cpp
    chromebufferpool.cpp
    chromebufferpool.h
    chromedmabuf.cpp
    chromedmabuf.h
    chromescale.cpp
    chromescale.h
    chromestats.cpp
    chromestats.h
    gpuchromecapabilities.cpp
    gpuchromecapabilities.h
    clipboardhistory.cpp
    compositorpolicy.cpp
    controlcenterpolicy.cpp
    desktopentry.cpp
    displayspolicy.cpp
    dockdrops.cpp
    dockmodel.cpp
    dockprojection.cpp
    filestarget.cpp
    folderstacks.cpp
    focusstatus.cpp
    focusschedule.cpp
    launchfailure.cpp
    lockauth.cpp
    lockinput.cpp
    materialpolicy.cpp
    menubrokerclient.cpp
    menubrokerpolicy.cpp
    notificationcenter.cpp
    notificationcenter.h
    notificationclient.cpp
    notificationmodel.cpp
    osdmodel.cpp
    widgetmodel.cpp
    widgetmodel.h
    polkitagent.cpp
    polkitsession.cpp
    screencastbridge.cpp
    screenshotbridge.cpp
    screenshotwriter.cpp
    shellmenus.cpp
    systemstatusmodel.cpp
    trayclient.cpp
    trashbridge.cpp
    wallpaperpolicy.cpp
)
# The settingsd client is shared with the Settings app (T-09.1b); it lives in
# its own library so both processes link the same implementation (ADR 0036).
# `dragonfruit-files-core` is the Rust staticlib: the Dock's Trash state comes
# from files-core's one freedesktop Trash store (T-10.6a).
target_link_libraries(dragonfruit-shell-dockcore PUBLIC
    Qt6::Core Qt6::Gui Qt6::DBus Qt6::Network
    dragonfruit-chromepointer
    dragonfruit-settings-client
    dragonfruit-files-core
)
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

# The standard ext-session-lock-v1 XML is vendored (MIT) so the shell can be
# the first-party lock UI without depending on a system wayland-protocols
# package (T-12.3a, ADR 0067).
set(_lock_xml "${CMAKE_SOURCE_DIR}/protocols/wayland-protocols/ext-session-lock-v1.xml")
set(_lock_header "${DF_PROTOCOL_DIR}/ext-session-lock-v1-client-protocol.h")
set(_lock_source "${DF_PROTOCOL_DIR}/ext-session-lock-v1-protocol.c")
add_custom_command(
    OUTPUT "${_lock_header}" "${_lock_source}"
    COMMAND ${CMAKE_COMMAND} -E make_directory "${DF_PROTOCOL_DIR}"
    COMMAND "${WAYLAND_SCANNER}" client-header "${_lock_xml}" "${_lock_header}"
    COMMAND "${WAYLAND_SCANNER}" private-code "${_lock_xml}" "${_lock_source}"
    DEPENDS "${_lock_xml}"
    COMMENT "wayland-scanner ext-session-lock-v1"
    VERBATIM)
list(APPEND DF_PROTOCOL_SOURCES "${_lock_source}")

# The standard wlr-data-control-unstable-v1 XML is vendored (MIT) so the
# shell's clipboard history can observe and forward the selection without
# becoming a second owner (T-13.5b, ADR 0082/0083).
set(_data_control_xml
    "${CMAKE_SOURCE_DIR}/protocols/wayland-protocols/wlr-data-control-unstable-v1.xml")
set(_data_control_header
    "${DF_PROTOCOL_DIR}/wlr-data-control-unstable-v1-client-protocol.h")
set(_data_control_source "${DF_PROTOCOL_DIR}/wlr-data-control-unstable-v1-protocol.c")
add_custom_command(
    OUTPUT "${_data_control_header}" "${_data_control_source}"
    COMMAND ${CMAKE_COMMAND} -E make_directory "${DF_PROTOCOL_DIR}"
    COMMAND "${WAYLAND_SCANNER}" client-header "${_data_control_xml}" "${_data_control_header}"
    COMMAND "${WAYLAND_SCANNER}" private-code "${_data_control_xml}" "${_data_control_source}"
    DEPENDS "${_data_control_xml}"
    COMMENT "wayland-scanner wlr-data-control-unstable-v1"
    VERBATIM)
list(APPEND DF_PROTOCOL_SOURCES "${_data_control_source}")

# The standard fractional-scale-v1 and viewporter XML is vendored (MIT) so the
# chrome surfaces can commit a physical-resolution buffer and map it onto the
# logical surface at 1.5x (T-21.1b). The compositor advertises both globals.
foreach(_scale_proto fractional-scale-v1 viewporter)
    set(_scale_xml "${CMAKE_SOURCE_DIR}/protocols/wayland-protocols/${_scale_proto}.xml")
    set(_scale_header "${DF_PROTOCOL_DIR}/${_scale_proto}-client-protocol.h")
    set(_scale_source "${DF_PROTOCOL_DIR}/${_scale_proto}-protocol.c")
    add_custom_command(
        OUTPUT "${_scale_header}" "${_scale_source}"
        COMMAND ${CMAKE_COMMAND} -E make_directory "${DF_PROTOCOL_DIR}"
        COMMAND "${WAYLAND_SCANNER}" client-header "${_scale_xml}" "${_scale_header}"
        COMMAND "${WAYLAND_SCANNER}" private-code "${_scale_xml}" "${_scale_source}"
        DEPENDS "${_scale_xml}"
        COMMENT "wayland-scanner ${_scale_proto}"
        VERBATIM)
    list(APPEND DF_PROTOCOL_SOURCES "${_scale_source}")
endforeach()

# linux-dmabuf is the compositor's GPU-chrome import path (T-21.1d). The
# shell's probe binds it read-only to learn the format/modifier feedback; no
# buffer is created in T186a. The v5 XML is vendored (MIT) so the probe does
# not depend on a system wayland-protocols package.
set(_dmabuf_xml
    "${CMAKE_SOURCE_DIR}/protocols/wayland-protocols/linux-dmabuf-unstable-v1.xml")
set(_dmabuf_header
    "${DF_PROTOCOL_DIR}/linux-dmabuf-unstable-v1-client-protocol.h")
set(_dmabuf_source "${DF_PROTOCOL_DIR}/linux-dmabuf-unstable-v1-protocol.c")
add_custom_command(
    OUTPUT "${_dmabuf_header}" "${_dmabuf_source}"
    COMMAND ${CMAKE_COMMAND} -E make_directory "${DF_PROTOCOL_DIR}"
    COMMAND "${WAYLAND_SCANNER}" client-header "${_dmabuf_xml}" "${_dmabuf_header}"
    COMMAND "${WAYLAND_SCANNER}" private-code "${_dmabuf_xml}" "${_dmabuf_source}"
    DEPENDS "${_dmabuf_xml}"
    COMMENT "wayland-scanner linux-dmabuf-unstable-v1"
    VERBATIM)

# The GPU producer, runtime probe, and the T185 producer share one small
# library so the shell and the nested GL smoke rail exercise the same code.
# The smoke target is not part of `make e2e`; it needs a host Wayland/GL rail.
add_library(dragonfruit-shell-chromegpu STATIC
    chromedmabufexport.cpp
    chromedmabufexport.h
    chromeframeproducer.cpp
    chromeframeproducer.h
    gpuchromeframeproducer.cpp
    gpuchromeframeproducer.h
    gpuchromeprobe.cpp
    gpuchromeprobe.h
)
target_sources(dragonfruit-shell-chromegpu PRIVATE "${_dmabuf_source}")
target_include_directories(dragonfruit-shell-chromegpu PUBLIC
    "${CMAKE_CURRENT_SOURCE_DIR}"
    "${DF_PROTOCOL_DIR}"
)
target_include_directories(dragonfruit-shell-chromegpu PUBLIC "${EGL_INCLUDE_DIR}")
target_link_libraries(dragonfruit-shell-chromegpu PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-shell-dockcore
    "${EGL_LIBRARY}"
)
if(TARGET PkgConfig::WAYLAND_CLIENT)
    target_link_libraries(dragonfruit-shell-chromegpu PUBLIC PkgConfig::WAYLAND_CLIENT)
else()
    target_include_directories(dragonfruit-shell-chromegpu PUBLIC "${WAYLAND_CLIENT_INCLUDE_DIR}")
    target_link_libraries(dragonfruit-shell-chromegpu PUBLIC "${WAYLAND_CLIENT_LIBRARY}")
endif()

qt_add_executable(dragonfruit-shell
    main.cpp
    shellcontroller.cpp
    shellprotocol.cpp
    systemstatusclient.cpp
    themebinding.cpp
)
target_sources(dragonfruit-shell PRIVATE ${DF_PROTOCOL_SOURCES})
target_include_directories(dragonfruit-shell PRIVATE "${DF_PROTOCOL_DIR}")
target_compile_definitions(dragonfruit-shell PRIVATE
    DF_LOCKSTEP_VERSION=${DF_LOCKSTEP_VERSION}
    DF_QML_IMPORT_DIR="${CMAKE_BINARY_DIR}/qml")
target_link_libraries(dragonfruit-shell PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick Qt6::DBus
    dragonfruit-shell-chromegpu
    dragonfruit-shell-dockcore
    dragonfruit-system-font
    dragonfruit-i18n
    dragonfruit-design-systemplugin
    dragonfruit-shell-menubarplugin
    dragonfruit-shell-dockplugin
    dragonfruit-shell-overviewplugin
    dragonfruit-shell-switcherplugin
    dragonfruit-shell-apps-drawerplugin
    dragonfruit-shell-notificationsplugin
    dragonfruit-shell-control-centerplugin
    dragonfruit-shell-osdplugin
    dragonfruit-shell-lockplugin
    dragonfruit-shell-screenshotplugin
)
if(TARGET PkgConfig::WAYLAND_CLIENT)
    target_link_libraries(dragonfruit-shell PRIVATE PkgConfig::WAYLAND_CLIENT)
else()
    target_include_directories(dragonfruit-shell PRIVATE "${WAYLAND_CLIENT_INCLUDE_DIR}")
    target_link_libraries(dragonfruit-shell PRIVATE "${WAYLAND_CLIENT_LIBRARY}")
endif()
qt_import_qml_plugins(dragonfruit-shell)
