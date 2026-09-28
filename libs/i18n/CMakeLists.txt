# SPDX-License-Identifier: MIT
# Localization (T-16.7). One library owns locale resolution, the runtime
# `.ts` catalog parser, the application-install helper, and the default
# `QLocale` setup, so the shell, Settings, and Files cannot disagree about the
# desktop's language.

qt_add_library(dragonfruit-i18n STATIC
    i18n.cpp
    i18n.h
)
target_link_libraries(dragonfruit-i18n PUBLIC Qt6::Core)
target_include_directories(dragonfruit-i18n PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
# The in-tree catalogs, resolved at configure time; the last-resort fallback
# that keeps `make demo`/`make dev` translated straight from a checkout
# (mirrors the settings client's in-tree wallpaper default).
target_compile_definitions(dragonfruit-i18n PRIVATE
    DF_TRANSLATIONS_DIR="${CMAKE_SOURCE_DIR}/translations")

# Ship the catalogs so the installed shell/apps find them through
# `catalogSearchPaths()`. The distro packages (T-16.9/T-16.10) install the
# same tree under their prefixes.
include(GNUInstallDirs)
install(DIRECTORY "${CMAKE_SOURCE_DIR}/translations/"
    DESTINATION "${CMAKE_INSTALL_DATADIR}/dragonfruit/translations"
    FILES_MATCHING PATTERN "*.ts")

add_subdirectory(tests)