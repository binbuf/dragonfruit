# SPDX-License-Identifier: MIT
# Dragonfruit design system — the single place where the desktop's visual
# identity lives (T-08). Tokens are generated from design-system/tokens/
# tokens.json into Theme.qml (QML singleton) and compositor/src/
# design_tokens.rs (Rust); `make check-tokens` fails if either is stale.
#
# Static QML module: first-party apps and shell chrome link it directly,
# which is what makes "every first-party application consumes these
# components" enforceable at build time (docs/design/10-design-system.md).

# `pragma Singleton` is not auto-detected by this Qt build system: the source
# property is what makes the generated qmldir register `Theme` as a singleton.
set_source_files_properties(Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
# The generated Phosphor path registry is a singleton too (T-19.1a).
set_source_files_properties(PhosphorGlyphs.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
# The generated Tabler path registry is a singleton too (ADR 0185).
set_source_files_properties(TablerGlyphs.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
# The popup-family material fill/fallback helper (T-21.2c) is a singleton.
set_source_files_properties(Material.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
# The generated material-preset resolver (T195) is a singleton.
set_source_files_properties(Glass.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

qt_add_library(dragonfruit-design-system STATIC)
qt_add_qml_module(dragonfruit-design-system
    URI Dragonfruit
    VERSION 1.0
    QML_FILES
        Theme.qml
        Material.qml
        Glass.qml
        PhosphorGlyphs.qml
        TablerGlyphs.qml
        components/AppWindow.qml
        components/TitleBar.qml
        components/TrafficLights.qml
        components/DragonfruitLogo.qml
        components/MenuBarMenu.qml
        components/Toggle.qml
        components/Popup.qml
        components/Icon.qml
        components/TablerIcon.qml
        components/PhosphorIcon.qml
        components/SettingsCategoryIcon.qml
        components/FocusRing.qml
        components/Shadow.qml
        components/ChromeSheet.qml
        components/Button.qml
        components/Sidebar.qml
        components/Toolbar.qml
        components/SplitView.qml
        components/SettingsRow.qml
        components/SettingsGroup.qml
        components/SegmentedControl.qml
        components/Slider.qml
        components/Select.qml
        components/ContextMenu.qml
        components/MenuRow.qml
        components/ShortcutHint.qml
        components/SearchField.qml
        components/SourceList.qml
        components/Dialog.qml
        components/Sheet.qml
        components/Popover.qml
        components/Tooltip.qml
        components/Skeleton.qml
        components/Spinner.qml
        components/ScrollView.qml
        components/WindowResizeGrips.qml
)
target_link_libraries(dragonfruit-design-system PUBLIC Qt6::Core Qt6::Gui Qt6::Quick)

# The vendored Phosphor SVGs (T-19.1a) are exposed unchanged at the stable
# prefix `qrc:/icons/phosphor/<name>.svg`; the generated registry above is the
# software-tintable form `PhosphorIcon` renders. Adding a glyph is a file drop
# plus `scripts/gen-phosphor-glyphs.py` (see assets/icons/phosphor/README.md).
file(GLOB _phosphor_svgs CONFIGURE_DEPENDS
    "${CMAKE_SOURCE_DIR}/assets/icons/phosphor/*.svg")
if(NOT _phosphor_svgs)
    message(FATAL_ERROR "dragonfruit-design-system: assets/icons/phosphor is empty")
endif()
qt_add_resources(dragonfruit-design-system "phosphor"
    PREFIX "/icons/phosphor"
    BASE "${CMAKE_SOURCE_DIR}/assets/icons/phosphor"
    FILES ${_phosphor_svgs}
)

# The vendored Tabler SVGs (ADR 0185) are exposed unchanged at the stable
# prefix `qrc:/icons/tabler/<name>.svg` (`<name>-filled.svg` for the solid
# weight); the generated registry above is the software-tintable form
# `TablerIcon` renders. Adding a glyph is a file drop plus
# `scripts/gen-tabler-glyphs.py` (see assets/icons/tabler/README.md).
file(GLOB _tabler_svgs CONFIGURE_DEPENDS
    "${CMAKE_SOURCE_DIR}/assets/icons/tabler/*.svg")
if(NOT _tabler_svgs)
    message(FATAL_ERROR "dragonfruit-design-system: assets/icons/tabler is empty")
endif()
qt_add_resources(dragonfruit-design-system "tabler"
    PREFIX "/icons/tabler"
    BASE "${CMAKE_SOURCE_DIR}/assets/icons/tabler"
    FILES ${_tabler_svgs}
)

# The first-party app icons (T-19.1d) are exposed unchanged at the stable
# prefix `qrc:/icons/apps/<name>.svg`. The Dock renders them for the
# first-party desktop ids before an install; the same files are installed into
# the icon theme (`make install`), so the Dock, menus, and the portal file
# chooser resolve one artwork source. Generated from Tabler by
# `scripts/gen-app-icons.py` (`make check-tabler` fails if stale).
file(GLOB _app_icons CONFIGURE_DEPENDS
    "${CMAKE_SOURCE_DIR}/assets/icons/apps/*.svg")
if(NOT _app_icons)
    message(FATAL_ERROR "dragonfruit-design-system: assets/icons/apps is empty")
endif()
qt_add_resources(dragonfruit-design-system "app-icons"
    PREFIX "/icons/apps"
    BASE "${CMAKE_SOURCE_DIR}/assets/icons/apps"
    FILES ${_app_icons}
)

df_qml_lint(design-system
    Theme.qml
    Material.qml
    Glass.qml
    PhosphorGlyphs.qml
    TablerGlyphs.qml
    components/AppWindow.qml
    components/TitleBar.qml
    components/TrafficLights.qml
    components/DragonfruitLogo.qml
    components/MenuBarMenu.qml
    components/Toggle.qml
    components/Popup.qml
    components/Icon.qml
    components/TablerIcon.qml
    components/PhosphorIcon.qml
    components/SettingsCategoryIcon.qml
    components/FocusRing.qml
    components/Shadow.qml
    components/ChromeSheet.qml
    components/Button.qml
    components/Sidebar.qml
    components/Toolbar.qml
    components/SplitView.qml
    components/SettingsRow.qml
    components/SettingsGroup.qml
    components/SegmentedControl.qml
    components/Slider.qml
    components/Select.qml
    components/ContextMenu.qml
    components/MenuRow.qml
    components/ShortcutHint.qml
    components/SearchField.qml
    components/SourceList.qml
    components/Dialog.qml
    components/Sheet.qml
    components/Popover.qml
    components/Tooltip.qml
    components/Skeleton.qml
    components/Spinner.qml
    components/ScrollView.qml
    components/WindowResizeGrips.qml
)

add_subdirectory(gallery)
add_subdirectory(tests)
