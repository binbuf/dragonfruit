# SPDX-License-Identifier: MIT
qt_add_library(dragonfruit-shell-control-center STATIC)
# `pragma Singleton` is not auto-detected by this Qt build system.
set_source_files_properties(ControlCenterRegistry.qml
    PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
qt_add_qml_module(dragonfruit-shell-control-center
    URI Dragonfruit.ControlCenter
    VERSION 1.0
    QML_FILES
        ControlCenter.qml
        ControlCenterModule.qml
        EditControls.qml
        ControlCenterRegistry.qml
        # T293b: the Settings app's Menu Bar Controls sub-pane (the former
        # Quick Settings surface). It lives in this module so `MenuBarPane`
        # can mount it without a Settings <-> Control Center dependency cycle:
        # `ControlCenterRegistry` is its one data source and the body writes
        # the panel's own `quickSettings.*` keys.
        MenuBarControlsPane.qml
)
target_link_libraries(dragonfruit-shell-control-center PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(shell-control-center
    ControlCenter.qml ControlCenterModule.qml EditControls.qml
    ControlCenterRegistry.qml MenuBarControlsPane.qml)
