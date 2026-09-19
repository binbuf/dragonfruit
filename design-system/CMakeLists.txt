# SPDX-License-Identifier: LGPL-3.0-or-later
# Dragonfruit design system — the single place where the desktop's visual
# identity lives (T-08). Tokens are generated from design-system/tokens/
# tokens.json into Theme.qml (QML singleton) and compositor/src/
# design_tokens.rs (Rust); `make check-tokens` fails if either is stale.
#
# Static QML module: first-party apps and shell chrome link it directly,
# which is what makes "every first-party application consumes these
# components" enforceable at build time (.docs/design/10-design-system.md).

# `pragma Singleton` is not auto-detected by this Qt build system: the source
# property is what makes the generated qmldir register `Theme` as a singleton.
set_source_files_properties(Theme.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

qt_add_library(dragonfruit-design-system STATIC)
qt_add_qml_module(dragonfruit-design-system
    URI Dragonfruit
    VERSION 1.0
    QML_FILES
        Theme.qml
        components/AppWindow.qml
        components/TitleBar.qml
        components/TrafficLights.qml
        components/MenuBarMenu.qml
        components/Toggle.qml
        components/Popup.qml
        components/Icon.qml
        components/FocusRing.qml
        components/Shadow.qml
        components/Button.qml
        components/Sidebar.qml
        components/Toolbar.qml
        components/SplitView.qml
        components/SettingsRow.qml
        components/SettingsGroup.qml
        components/SegmentedControl.qml
        components/ContextMenu.qml
        components/SearchField.qml
        components/SourceList.qml
        components/Dialog.qml
        components/Sheet.qml
        components/Popover.qml
        components/ScrollView.qml
)
target_link_libraries(dragonfruit-design-system PUBLIC Qt6::Core Qt6::Gui Qt6::Quick)

df_qml_lint(design-system
    Theme.qml
    components/AppWindow.qml
    components/TitleBar.qml
    components/TrafficLights.qml
    components/MenuBarMenu.qml
    components/Toggle.qml
    components/Popup.qml
    components/Icon.qml
    components/FocusRing.qml
    components/Shadow.qml
    components/Button.qml
    components/Sidebar.qml
    components/Toolbar.qml
    components/SplitView.qml
    components/SettingsRow.qml
    components/SettingsGroup.qml
    components/SegmentedControl.qml
    components/ContextMenu.qml
    components/SearchField.qml
    components/SourceList.qml
    components/Dialog.qml
    components/Sheet.qml
    components/Popover.qml
    components/ScrollView.qml
)

add_subdirectory(gallery)
add_subdirectory(tests)
