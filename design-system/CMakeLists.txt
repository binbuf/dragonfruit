# SPDX-License-Identifier: LGPL-3.0-or-later
# Dragonfruit design system — the single place where the desktop's visual
# identity lives (T-08 fills in the real component library and token
# architecture; this scaffold wires the build and the `Dragonfruit` module).
#
# Static QML module: first-party apps and shell chrome link it directly,
# which is what makes "every first-party application consumes these
# components" enforceable at build time (.docs/design/10-design-system.md).

qt_add_library(dragonfruit-design-system STATIC)
qt_add_qml_module(dragonfruit-design-system
    URI Dragonfruit
    VERSION 1.0
    QML_FILES
        Theme.qml
)
target_link_libraries(dragonfruit-design-system PUBLIC Qt6::Core Qt6::Gui Qt6::Quick)

df_qml_lint(design-system Theme.qml)
