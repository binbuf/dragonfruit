# SPDX-License-Identifier: MIT
qt_add_executable(dragonfruit-files
    main.cpp
)
qt_add_qml_module(dragonfruit-files
    URI Dragonfruit.Files
    VERSION 1.0
    QML_FILES
        FilesWindow.qml
)
target_link_libraries(dragonfruit-files PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

df_qml_lint(app-files FilesWindow.qml)
