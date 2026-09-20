# SPDX-License-Identifier: MIT
# The component gallery (T-08). GalleryContent is a reusable QML module so the
# visual-regression test and the human review app render the exact same
# surfaces; GalleryWindow is the review app with a headless snapshot driver.

qt_add_library(dragonfruit-gallery STATIC)
qt_add_qml_module(dragonfruit-gallery
    URI Dragonfruit.Gallery
    VERSION 1.0
    QML_FILES
        GalleryContent.qml
        SsdTitlebarReference.qml
)
target_link_libraries(dragonfruit-gallery PUBLIC
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
)

qt_add_executable(dragonfruit-gallery-app
    main.cpp
)
qt_add_qml_module(dragonfruit-gallery-app
    URI Dragonfruit.GalleryApp
    VERSION 1.0
    QML_FILES
        GalleryWindow.qml
)
target_link_libraries(dragonfruit-gallery-app PRIVATE
    Qt6::Core Qt6::Gui Qt6::Quick
    dragonfruit-design-systemplugin
    dragonfruit-galleryplugin
)
qt_import_qml_plugins(dragonfruit-gallery-app)
target_compile_definitions(dragonfruit-gallery-app PRIVATE
    DRAGONFRUIT_QML_IMPORT_DIR="${CMAKE_BINARY_DIR}/qml"
)

df_qml_lint(gallery
    GalleryContent.qml
    SsdTitlebarReference.qml
    GalleryWindow.qml
)
