# SPDX-License-Identifier: MIT
# T290a: the Settings module's QML is compiled into the module's resource (the
# generated qmldir carries `prefer :/qt/qml/Dragonfruit/Settings/`), but Qt
# also copies every `QML_FILES` entry into `build/qml/Dragonfruit/Settings/`.
# That copy rule is keyed on a timestamp file, not on the copied files
# themselves, so a copy that is externally stale (a reused build tree, an
# interrupted copy, a manual edit) survives `cmake --build` forever.
#
# This check compares every source QML file with its build copy and fails with
# the stale path. `make qml-test` refreshes the copy before the gate build, so
# a failure here means the build tree drifted outside the gate.

if(NOT DEFINED SOURCE_DIR)
    message(FATAL_ERROR "tst_settings_qml_copy_sync: SOURCE_DIR is required")
endif()
if(NOT DEFINED COPY_DIR)
    message(FATAL_ERROR "tst_settings_qml_copy_sync: COPY_DIR is required")
endif()
if(NOT DEFINED QML_FILES)
    message(FATAL_ERROR "tst_settings_qml_copy_sync: QML_FILES is required")
endif()

set(_checked 0)
set(_stale "")
set(_missing "")

foreach(_name IN LISTS QML_FILES)
    set(_src "${SOURCE_DIR}/${_name}")
    set(_dst "${COPY_DIR}/${_name}")

    if(NOT EXISTS "${_src}")
        message(FATAL_ERROR
            "tst_settings_qml_copy_sync: source QML file is missing: ${_src}")
    endif()

    if(NOT EXISTS "${_dst}")
        list(APPEND _missing "${_name}")
        continue()
    endif()

    file(SHA256 "${_src}" _src_hash)
    file(SHA256 "${_dst}" _dst_hash)
    if(NOT _src_hash STREQUAL _dst_hash)
        list(APPEND _stale "${_name}")
    endif()

    math(EXPR _checked "${_checked} + 1")
endforeach()

if(_missing)
    list(JOIN _missing "\n  " _missing_text)
    message(FATAL_ERROR
        "tst_settings_qml_copy_sync: Settings QML build copy is missing:\n"
        "  ${_missing_text}\n"
        "The copy lives at ${COPY_DIR}. Refresh it with `make qml-test` "
        "(or `make settings-qml-refresh` then `cmake --build build`).")
endif()

if(_stale)
    list(JOIN _stale "\n  " _stale_text)
    message(FATAL_ERROR
        "tst_settings_qml_copy_sync: stale Settings QML build copy:\n"
        "  ${_stale_text}\n"
        "The copy lives at ${COPY_DIR}. Refresh it with `make qml-test` "
        "(or `make settings-qml-refresh` then `cmake --build build`).")
endif()

message(STATUS
    "tst_settings_qml_copy_sync: ${_checked} Settings QML files match the build copy")
