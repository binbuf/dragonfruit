# SPDX-License-Identifier: MIT
# Dragonfruit Qt/CMake side: design system, shell components, apps.
#
# The Rust side (Cargo workspace) and this build ship as one lockstep set;
# `make build` drives both (see the top-level Makefile).

cmake_minimum_required(VERSION 3.24)
project(dragonfruit VERSION 0.3.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

# 6.8 is the real floor, not a preference: QChronoTimer (shellcontroller.cpp)
# arrived in Qt 6.8 and QtQuick.VectorImage (shell/dock/DockGlyph.qml) in 6.6.
# Keeping the floor at the oldest supported distribution lets Debian 13,
# Ubuntu 25.04+, Fedora 43+, and Arch build the same source. Raise it only with
# a packaging decision (see docs/design/12-packaging.md).
find_package(Qt6 6.8 REQUIRED COMPONENTS Core Gui Quick QuickTest Test DBus)

# `REQUIRES` also opts in to every Qt policy introduced up to that version
# (QTP0001, QTP0004, ...). Without it the policies stay unset and every
# `qt_add_qml_module` emits a developer warning. The version matches the
# `find_package(Qt6 6.8 ...)` above.
qt_standard_project_setup(REQUIRES 6.8)

option(DF_BUILD_TESTS "Build the Qt/QML test targets" ON)
enable_testing()

# The files-core bridge (T-10.4b, ADR 0049; T-10.6a): the Rust crate is built
# as a static library exposing a small C ABI (`staticlib`). The Files app and
# the shell's Dock Trash bridge both link it, so the imported target and its
# one cargo invocation are defined here rather than in either consumer.
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

# One shared QML import root for every module (Dragonfruit, Dragonfruit.*).
# This is what lets a plain, runtime-compiled QML file (the tests) resolve
# `import Dragonfruit` from the filesystem instead of only through the
# compiled-in resource of an app.
set(QT_QML_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/qml)

include(GNUInstallDirs)

# A packaged/install-tree session must resolve the first-party QML modules
# (`Dragonfruit.Settings`, `Dragonfruit.Files`, the shell panels) without the
# build tree on QML_IMPORT_PATH. Install every generated module under one
# stable root; the session entry adds it to QML_IMPORT_PATH.
install(DIRECTORY "${CMAKE_BINARY_DIR}/qml/"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/dragonfruit/qml"
    FILES_MATCHING
    PATTERN "*.qml"
    PATTERN "*.js"
    PATTERN "*.mjs"
    PATTERN "*.qmltypes"
    PATTERN "qmldir"
)

# The shipped system cursor theme. The compositor also has the in-repo copy as
# a development fallback; an installed session finds it in the icon theme dir.
install(DIRECTORY "${CMAKE_SOURCE_DIR}/assets/cursors/capitaine-cursors"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/icons"
)

# QML lint for every module, as a ctest so CI runs it headlessly.
get_target_property(QMLLINT_EXECUTABLE Qt6::qmllint LOCATION)
if(NOT QMLLINT_EXECUTABLE)
    find_program(QMLLINT_EXECUTABLE qmllint REQUIRED)
endif()

function(df_qml_lint target)
    set(_files ${ARGN})
    add_test(
        NAME qmllint_${target}
        COMMAND ${QMLLINT_EXECUTABLE} -I ${CMAKE_BINARY_DIR} -I ${CMAKE_BINARY_DIR}/qml ${_files}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    )
    add_custom_target(lint_${target}
        COMMAND ${QMLLINT_EXECUTABLE} -I ${CMAKE_BINARY_DIR} -I ${CMAKE_BINARY_DIR}/qml ${_files}
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        COMMENT "qmllint ${target}"
        VERBATIM
    )
endfunction()

# Shared libraries come first: the design-system gallery app links the system
# font, and the shell/apps link it through their own targets.
add_subdirectory(libs)
add_subdirectory(design-system)
add_subdirectory(shell)
add_subdirectory(apps)
