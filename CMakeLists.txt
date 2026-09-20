# SPDX-License-Identifier: MIT
# Dragonfruit Qt/CMake side: design system, shell components, apps.
#
# The Rust side (Cargo workspace) and this build ship as one lockstep set;
# `make build` drives both (see the top-level Makefile).

cmake_minimum_required(VERSION 3.24)
project(dragonfruit VERSION 0.1.0 LANGUAGES C CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)

find_package(Qt6 6.11 REQUIRED COMPONENTS Core Gui Quick QuickTest)

qt_standard_project_setup()

enable_testing()

# One shared QML import root for every module (Dragonfruit, Dragonfruit.*).
# This is what lets a plain, runtime-compiled QML file (the tests) resolve
# `import Dragonfruit` from the filesystem instead of only through the
# compiled-in resource of an app.
set(QT_QML_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/qml)

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

add_subdirectory(design-system)
add_subdirectory(shell)
add_subdirectory(apps)
