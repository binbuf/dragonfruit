# SPDX-License-Identifier: MIT
# The desktop's system font (Inter, SIL OFL 1.1). One static library owns the
# bundled faces and the process-wide application-font installation, so the
# shell, Settings, Files, and the gallery cannot disagree about the desktop's
# type the way they would if each entry point registered fonts itself.
#
# The TTFs are tracked project assets under fonts/Inter (see README there) and
# are compiled in as Qt resources: no system install is required to develop or
# test, and packaging can install the same faces system-wide later for
# third-party apps.

set(_inter_dir "${CMAKE_SOURCE_DIR}/fonts/Inter")
set(_inter_roman "${_inter_dir}/Inter-VariableFont_opsz,wght.ttf")
set(_inter_italic "${_inter_dir}/Inter-Italic-VariableFont_opsz,wght.ttf")
if(NOT EXISTS "${_inter_roman}" OR NOT EXISTS "${_inter_italic}")
    message(FATAL_ERROR
        "dragonfruit-system-font: fonts/Inter is missing (expected "
        "${_inter_roman} and ${_inter_italic})")
endif()

qt_add_library(dragonfruit-system-font STATIC
    systemfont.cpp
    systemfont.h
)
qt_add_resources(dragonfruit-system-font "inter"
    PREFIX "/dragonfruit/fonts"
    BASE "${_inter_dir}"
    FILES "${_inter_roman}" "${_inter_italic}"
)
target_link_libraries(dragonfruit-system-font PUBLIC Qt6::Gui)
target_include_directories(dragonfruit-system-font PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")

add_subdirectory(tests)